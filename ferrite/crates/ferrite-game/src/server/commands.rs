//! Running commands: parsing a line with the player's command tree, reporting errors the way the
//! official server does, and the commands themselves.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use ferrite_commands::args::{Source, Value};
use ferrite_commands::dispatch::{self, Context, ParseResults};
use ferrite_commands::reader::{Arg as MsgArg, Message};
use ferrite_commands::tree::{NodeKind, Tree};
use ferrite_text::{Arg, ClickEvent, Color, Component, HoverEvent};

use super::*;

type Args = BTreeMap<String, dispatch::ParsedArgument>;

/// The raw `commands` packet body for a permission level.
pub(super) fn tree_bytes(level: u8) -> &'static [u8] {
    ferrite_data::generated::commands::COMMAND_TREES[usize::from(level.min(4))]
}

fn trees() -> &'static [Tree; 5] {
    static TREES: OnceLock<[Tree; 5]> = OnceLock::new();
    TREES.get_or_init(|| std::array::from_fn(Tree::for_level))
}

/// Why a command did not run (shown to the player in red).
pub(super) struct Failure(pub Box<Component>);

pub(super) type CommandResult = std::result::Result<i32, Failure>;

/// The ticks of a `time` argument.
fn time_arg(args: &Args, name: &str) -> i32 {
    match args.get(name).map(|a| &a.value) {
        Some(Value::Time(t)) => *t,
        _ => 0,
    }
}

fn failure(key: &str, args: Vec<Arg>) -> Failure {
    Failure(Box::new(Component::translatable(key, args)))
}

/// A parse error as a component.
fn message_component(m: &Message) -> Component {
    match m {
        Message::Literal(s) => Component::literal(s.clone()),
        Message::Translatable { key, args } => Component::translatable(
            key,
            args.iter()
                .map(|a| match a {
                    MsgArg::Str(s) => Arg::String(s.clone()),
                    MsgArg::Int(v) => Arg::Int(*v as i32),
                    MsgArg::Long(v) => Arg::Long(*v),
                    MsgArg::Float(v) => Arg::Float(*v),
                    MsgArg::Double(v) => Arg::Double(*v),
                    MsgArg::Msg(m) => message_component(m).into(),
                })
                .collect(),
        ),
    }
}

/// The display name of a player: their name, clickable to message them, with a hover card.
pub(super) fn player_display_name(name: &str, uuid: Uuid) -> Component {
    let bytes = uuid.as_u128();
    let ints = [(bytes >> 96) as i32, (bytes >> 64) as i32, (bytes >> 32) as i32, bytes as i32];
    Component::literal(name)
        .click(ClickEvent::SuggestCommand(format!("/tell {name} ")))
        .hover(HoverEvent::ShowEntity {
            id: "minecraft:player".into(),
            uuid: ints,
            name: Some(Box::new(Component::literal(name))),
        })
        .insertion(name)
}

/// A list of components separated by gray commas (`ComponentUtils.formatList`).
fn format_list(items: Vec<Component>) -> Component {
    let mut items = items;
    match items.len() {
        0 => Component::empty(),
        1 => items.remove(0),
        _ => {
            let mut out = Component::empty();
            for (i, c) in items.into_iter().enumerate() {
                if i > 0 {
                    out = out.append(Component::literal(", ").color(Color::Gray));
                }
                out = out.append(c);
            }
            out
        }
    }
}

const GAME_MODE_NAMES: [&str; 4] = ["survival", "creative", "adventure", "spectator"];
const DIFFICULTY_NAMES: [&str; 4] = ["peaceful", "easy", "normal", "hard"];

/// The effective command a context chain runs: the names of the nodes of its last context,
/// preceded by the command an alias redirects to.
fn command_key(tree: &Tree, ctx: &Context) -> String {
    let mut prefix: Vec<String> = Vec::new();
    let mut cur = ctx;
    loop {
        let names: Vec<&str> =
            cur.nodes.iter().map(|(n, _)| tree.nodes[*n].name.as_str()).collect();
        match &cur.child {
            Some(child) => {
                // After a redirect the child context continues at the target node; aliases
                // (like `tell` for `msg`) redirect to a command, `run` back to the root.
                let last = cur.nodes.last().map(|(n, _)| *n);
                let target = last.and_then(|n| tree.nodes[n].redirect);
                prefix.clear();
                if let Some(t) = target
                    && t != tree.root
                    && tree.nodes[t].kind == NodeKind::Literal
                {
                    prefix.push(tree.nodes[t].name.clone());
                }
                cur = child;
            }
            None => {
                let mut all = prefix;
                all.extend(names.into_iter().map(str::to_string));
                return all.join("/");
            }
        }
    }
}

impl Game {
    pub(super) fn permission_level_of(&self, name: &str) -> u8 {
        self.cfg
            .ops
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map_or(self.cfg.default_permission_level, |(_, l)| *l)
    }

    /// Sends a command's failure: red, the way `CommandSourceStack.sendFailure` does.
    fn send_failure(&self, id: PlayerId, c: Component) {
        if let Some(p) = self.players.get(&id) {
            let wrapped = Component::empty().color(Color::Red).append(c);
            send(&p.sink, &SystemChat { content: Text(wrapped.to_tag()), overlay: false });
        }
    }

    /// Sends a command's success message to the player who ran it and, when `admins` is set,
    /// tells the other operators what was done (`CommandSourceStack.sendSuccess`).
    pub(super) fn send_success(&self, id: PlayerId, c: Component, admins: bool) {
        let Some(p) = self.players.get(&id) else { return };
        send(&p.sink, &SystemChat { content: Text(c.to_tag()), overlay: false });
        if admins {
            let who = player_display_name(&p.name, p.uuid);
            self.tell_admins(Some(id), who, c);
        }
    }

    /// `chat.type.admin` to every operator except `except`.
    pub(super) fn tell_admins(&self, except: Option<PlayerId>, who: Component, c: Component) {
        let note = Component::translatable("chat.type.admin", vec![who.into(), c.into()])
            .color(Color::Gray)
            .italic(true);
        for (pid, other) in &self.players {
            if Some(*pid) != except && other.permission_level > 0 {
                send(&other.sink, &SystemChat { content: Text(note.to_tag()), overlay: false });
            }
        }
    }

    /// The two lines shown for a command that did not parse: the message, then the command with
    /// the place it went wrong marked.
    fn report_syntax_error(&self, id: PlayerId, message: &Message, cursor: i64, input: &str) {
        self.send_failure(id, message_component(message));
        if cursor < 0 {
            return;
        }
        let units: Vec<u16> = input.encode_utf16().collect();
        let i = (cursor as usize).min(units.len());
        let text = |from: usize, to: usize| String::from_utf16_lossy(&units[from..to]);
        let mut c = Component::empty()
            .color(Color::Gray)
            .click(ClickEvent::SuggestCommand(format!("/{input}")));
        if i > 10 {
            c = c.append(Component::literal("..."));
        }
        let before = text(i.saturating_sub(10), i);
        if !before.is_empty() {
            c = c.append(Component::literal(before));
        }
        if i < units.len() {
            c = c.append(
                Component::literal(text(i, units.len())).color(Color::Red).underlined(true),
            );
        }
        c = c.append(
            Component::translatable("command.context.here", vec![]).color(Color::Red).italic(true),
        );
        self.send_failure(id, c);
    }

    /// A player typed `/<line>`.
    pub(super) fn on_chat_command(&mut self, id: PlayerId, line: &str) {
        let Some(p) = self.players.get(&id) else { return };
        let level = p.permission_level;
        let tree = &trees()[usize::from(level.min(4))];
        let results: ParseResults = dispatch::parse(tree, line, Source { permission_level: level });
        if results.unsupported {
            self.send_failure(
                id,
                Component::literal("This command uses an argument type Ferrite cannot parse yet."),
            );
            return;
        }
        if let Some(e) = dispatch::parse_error(&results) {
            self.report_syntax_error(id, &e.message, e.cursor, line);
            return;
        }
        let key = command_key(tree, &results.context);
        let ctx = results.context.last();
        let outcome = self.run_command(id, &key, &ctx.arguments);
        match outcome {
            Ok(_) => {}
            Err(Failure(c)) => self.send_failure(id, *c),
        }
    }

    fn run_command(&mut self, id: PlayerId, key: &str, args: &Args) -> CommandResult {
        match key {
            "gamemode/gamemode" => self.cmd_gamemode(id, id, args),
            "gamemode/gamemode/target" => {
                let targets = self.players_from(id, args.get("target"))?;
                let mut last = Ok(0);
                for t in targets {
                    last = self.cmd_gamemode(id, t, args);
                }
                last
            }
            "difficulty" => self.cmd_difficulty_query(id),
            "difficulty/peaceful" => self.cmd_difficulty_set(id, 0),
            "difficulty/easy" => self.cmd_difficulty_set(id, 1),
            "difficulty/normal" => self.cmd_difficulty_set(id, 2),
            "difficulty/hard" => self.cmd_difficulty_set(id, 3),
            "list" => self.cmd_list(id, false),
            "list/uuids" => self.cmd_list(id, true),
            "seed" => self.cmd_seed(id),
            "say/message" => self.cmd_say(id, args, "minecraft:say_command", "message"),
            "me/action" => self.cmd_say(id, args, "minecraft:emote_command", "action"),
            "msg/targets/message" => self.cmd_msg(id, args),
            // (Teams do not exist yet, so nobody is ever on one.)
            "teammsg/message" => Err(failure("commands.teammsg.failed.noteam", vec![])),
            "tick/query" => self.cmd_tick_query(id),
            "tick/freeze" => self.cmd_tick_freeze(id, true),
            "tick/unfreeze" => self.cmd_tick_freeze(id, false),
            "tick/rate/rate" => self.cmd_tick_rate(id, args),
            "tick/step" => self.cmd_tick_step(id, 1),
            "tick/step/time" => {
                let n = time_arg(args, "time");
                self.cmd_tick_step(id, n)
            }
            "tick/step/stop" => self.cmd_tick_step_stop(id),
            "tick/sprint/time" => {
                let n = time_arg(args, "time");
                self.cmd_tick_sprint(id, n)
            }
            "tick/sprint/stop" => self.cmd_tick_sprint_stop(id),
            key if key.starts_with("time/") => self.cmd_time(id, key, args),
            other => Err(Failure(Box::new(Component::literal(format!(
                "The command /{} is not implemented in Ferrite yet.",
                other.replace('/', " ")
            ))))),
        }
    }

    /// The players a selector argument names (entities that are not players are not handled
    /// yet).
    fn players_from(
        &mut self,
        source: PlayerId,
        arg: Option<&dispatch::ParsedArgument>,
    ) -> std::result::Result<Vec<PlayerId>, Failure> {
        let Some(dispatch::ParsedArgument { value: Value::Selector(sel), .. }) = arg else {
            return Err(failure("argument.entity.invalid", vec![]));
        };
        let sel = sel.clone();
        let found = self.select_players(source, &sel);
        if found.is_empty() {
            return Err(failure("argument.entity.notfound.player", vec![]));
        }
        Ok(found)
    }

    fn cmd_gamemode(&mut self, source: PlayerId, target: PlayerId, args: &Args) -> CommandResult {
        let Some(dispatch::ParsedArgument { value: Value::GameMode(mode), .. }) =
            args.get("gamemode")
        else {
            return Err(failure("argument.gamemode.invalid", vec![]));
        };
        // Nothing happens, and nothing is said, when the player is in that mode already.
        if !self.set_game_mode(target, *mode) {
            return Ok(0);
        }
        let mode_name = Component::translatable(
            &format!("gameMode.{}", GAME_MODE_NAMES[usize::from(*mode)]),
            vec![],
        );
        if source == target {
            self.send_success(
                source,
                Component::translatable("commands.gamemode.success.self", vec![mode_name.into()]),
                true,
            );
        } else if let Some(t) = self.players.get(&target) {
            send(
                &t.sink,
                &SystemChat {
                    content: Text(
                        Component::translatable("gameMode.changed", vec![mode_name.clone().into()])
                            .to_tag(),
                    ),
                    overlay: false,
                },
            );
            let name = player_display_name(&t.name, t.uuid);
            self.send_success(
                source,
                Component::translatable(
                    "commands.gamemode.success.other",
                    vec![name.into(), mode_name.into()],
                ),
                true,
            );
        }
        Ok(1)
    }

    /// `ServerPlayer.setGameMode`: abilities, the tab list, and the client's own mode.
    pub(super) fn set_game_mode(&mut self, id: PlayerId, mode: u8) -> bool {
        let Some(p) = self.players.get_mut(&id) else { return false };
        if p.game_mode == mode {
            return false;
        }
        p.game_mode = mode;
        // Spectators fly; survival players cannot; creative players keep flying if they were.
        match mode {
            3 => p.flying = true,
            0 | 2 => p.flying = false,
            _ => {}
        }
        let uuid = p.uuid;
        let abilities = PlayerAbilities {
            flags: abilities_for(mode) | if p.flying { 2 } else { 0 },
            flying_speed: 0.05,
            walking_speed: 0.1,
        };
        send(&p.sink, &abilities);
        let entry = PlayerInfoEntry {
            uuid,
            game_mode: Some(VarInt(i32::from(mode))),
            ..Default::default()
        };
        let update = PlayerInfoUpdate { actions: 0x04, entries: vec![entry] };
        for other in self.players.values() {
            send(&other.sink, &update);
        }
        if let Some(p) = self.players.get(&id) {
            send(&p.sink, &GameEvent { event: 3, param: f32::from(mode) });
            send(&p.sink, &abilities);
        }
        true
    }

    fn cmd_difficulty_query(&mut self, id: PlayerId) -> CommandResult {
        let d = self.difficulty;
        self.send_success(
            id,
            Component::translatable(
                "commands.difficulty.query",
                vec![
                    Component::translatable(
                        &format!("options.difficulty.{}", DIFFICULTY_NAMES[usize::from(d)]),
                        vec![],
                    )
                    .into(),
                ],
            ),
            false,
        );
        Ok(i32::from(d))
    }

    fn cmd_difficulty_set(&mut self, id: PlayerId, d: u8) -> CommandResult {
        let name = Component::translatable(
            &format!("options.difficulty.{}", DIFFICULTY_NAMES[usize::from(d)]),
            vec![],
        );
        if self.difficulty == d {
            return Err(failure("commands.difficulty.failure", vec![name.into()]));
        }
        self.difficulty = d;
        for p in self.players.values() {
            send(&p.sink, &ChangeDifficulty { difficulty: d, locked: false });
        }
        self.send_success(
            id,
            Component::translatable("commands.difficulty.success", vec![name.into()]),
            true,
        );
        Ok(0)
    }

    fn cmd_list(&mut self, id: PlayerId, uuids: bool) -> CommandResult {
        let max = self.cfg.max_players as i32;
        let names: Vec<Component> = self
            .order
            .iter()
            .filter_map(|i| self.players.get(i))
            .map(|p| {
                if uuids {
                    Component::translatable(
                        "commands.list.nameAndId",
                        vec![p.name.clone().into(), p.uuid.hyphenated().to_string().into()],
                    )
                } else {
                    player_display_name(&p.name, p.uuid)
                }
            })
            .collect();
        let count = names.len() as i32;
        self.send_success(
            id,
            Component::translatable(
                "commands.list.players",
                vec![count.into(), max.into(), format_list(names).into()],
            ),
            false,
        );
        Ok(count)
    }

    fn cmd_seed(&mut self, id: PlayerId) -> CommandResult {
        let seed = self.cfg.seed.to_string();
        let text = Component::literal(seed.clone())
            .color(Color::Green)
            .click(ClickEvent::CopyToClipboard(seed.clone()))
            .hover(HoverEvent::ShowText(Box::new(Component::translatable(
                "chat.copy.click",
                vec![],
            ))))
            .insertion(&seed);
        let bracketed = Component::translatable("chat.square_brackets", vec![text.into()]);
        self.send_success(
            id,
            Component::translatable("commands.seed.success", vec![bracketed.into()]),
            false,
        );
        Ok(1)
    }

    /// Players a selector picks: names and UUIDs, `@a @p @r @s @e` with their options
    /// (entities that are not players are not handled yet).
    pub(super) fn select_players(
        &mut self,
        source: PlayerId,
        sel: &ferrite_commands::selector::Selector,
    ) -> Vec<PlayerId> {
        use ferrite_commands::selector::{Order, Target};
        let found: Vec<PlayerId> = match &sel.target {
            Target::Name(n) => self
                .order
                .iter()
                .copied()
                .filter(|i| self.players.get(i).is_some_and(|p| p.name.eq_ignore_ascii_case(n)))
                .collect(),
            Target::Uuid(u) => self
                .order
                .iter()
                .copied()
                .filter(|i| self.players.get(i).is_some_and(|p| p.uuid.as_u128() == *u))
                .collect(),
            Target::Selector('s') => {
                if self.players.contains_key(&source) {
                    vec![source]
                } else {
                    Vec::new()
                }
            }
            Target::Selector(_) => self.order.clone(),
        };
        let Some(src) = self.players.get(&source) else { return Vec::new() };
        let origin = [sel.x.unwrap_or(src.x), sel.y.unwrap_or(src.y), sel.z.unwrap_or(src.z)];
        let mut matches: Vec<(PlayerId, f64)> = Vec::new();
        for id in found {
            let Some(p) = self.players.get(&id) else { continue };
            let d2 =
                (p.x - origin[0]).powi(2) + (p.y - origin[1]).powi(2) + (p.z - origin[2]).powi(2);
            if let Some(r) = &sel.distance
                && !range_has(r.min.map(|m| m * m), r.max.map(|m| m * m), d2)
            {
                continue;
            }
            // A volume: the player's feet must lie in the box that starts at the origin.
            if sel.dx.is_some() || sel.dy.is_some() || sel.dz.is_some() {
                let inside = |v: f64, o: f64, d: Option<f64>| match d {
                    Some(d) => (v >= o.min(o + d)) && (v <= o.max(o + d)),
                    None => true,
                };
                if !(inside(p.x, origin[0], sel.dx)
                    && inside(p.y, origin[1], sel.dy)
                    && inside(p.z, origin[2], sel.dz))
                {
                    continue;
                }
            }
            if let Some(r) = &sel.level
                && !range_has(r.min.map(f64::from), r.max.map(f64::from), f64::from(p.xp_level))
            {
                continue;
            }
            if let Some(r) = &sel.x_rotation
                && !range_has(r.min, r.max, wrap_degrees(f64::from(p.pitch)))
            {
                continue;
            }
            if let Some(r) = &sel.y_rotation
                && !range_has(r.min, r.max, wrap_degrees(f64::from(p.yaw)))
            {
                continue;
            }
            if sel.game_modes.iter().any(|(negated, m)| (p.game_mode == *m) == *negated) {
                continue;
            }
            if sel.names.iter().any(|(negated, n)| (p.name == *n) == *negated) {
                continue;
            }
            // Players are not on teams or tagged yet, and have no scores.
            if sel.tags.iter().any(|(negated, t)| !t.is_empty() && !*negated)
                || sel.teams.iter().any(|(negated, _)| !*negated)
                || !sel.scores.is_empty()
            {
                continue;
            }
            let player_type = "minecraft:player";
            if sel.types.iter().any(|(negated, t)| match t {
                ferrite_commands::selector::TypeFilter::Type(n) => (n == player_type) == *negated,
                ferrite_commands::selector::TypeFilter::Tag(_) => !*negated,
            }) {
                continue;
            }
            matches.push((id, d2));
        }
        match sel.order {
            Order::Nearest => matches.sort_by(|a, b| a.1.total_cmp(&b.1)),
            Order::Furthest => matches.sort_by(|a, b| b.1.total_cmp(&a.1)),
            Order::Random => {
                // A shuffle from the game's own dice.
                let mut dice = self.mob_dice();
                for i in (1..matches.len()).rev() {
                    let j = dice.below(i as u32 + 1) as usize;
                    matches.swap(i, j);
                }
            }
            Order::Arbitrary => {}
        }
        matches.truncate(usize::try_from(sel.max_results).unwrap_or(usize::MAX));
        matches.into_iter().map(|(id, _)| id).collect()
    }
}

/// Whether `v` lies within the bounds (either may be missing).
fn range_has(min: Option<f64>, max: Option<f64>, v: f64) -> bool {
    min.is_none_or(|m| v >= m) && max.is_none_or(|m| v <= m)
}

fn wrap_degrees(d: f64) -> f64 {
    let mut r = d % 360.0;
    if r >= 180.0 {
        r -= 360.0;
    }
    if r < -180.0 {
        r += 360.0;
    }
    r
}

const CHAT_TYPES: &str = "minecraft:chat_type";

/// The wire id of a chat type: its index in the `chat_type` registry plus one.
fn chat_type_id(name: &str) -> i32 {
    ferrite_data::configuration::SYNCHRONIZED_REGISTRIES
        .iter()
        .find(|r| r.id == CHAT_TYPES)
        .and_then(|r| r.entries.iter().position(|e| *e == name))
        .map_or(0, |i| i as i32 + 1)
}

impl Game {
    /// A message argument as the text sent: selectors in it are replaced by the names of what
    /// they pick (`MessageArgument.Message.toComponent`).
    fn resolve_message(
        &mut self,
        source: PlayerId,
        text: &str,
        selectors: &[(usize, usize, ferrite_commands::selector::Selector)],
    ) -> Component {
        let permitted = self.players.get(&source).is_some_and(|p| p.permission_level >= 2);
        if selectors.is_empty() || !permitted {
            return Component::literal(text);
        }
        let units: Vec<u16> = text.encode_utf16().collect();
        let piece = |from: usize, to: usize| String::from_utf16_lossy(&units[from..to]);
        let mut out = Component::literal(piece(0, selectors[0].0));
        let mut i = selectors[0].0;
        for (start, end, sel) in selectors {
            let picked = self.select_players(source, sel);
            let names: Vec<Component> = picked
                .iter()
                .filter_map(|id| self.players.get(id))
                .map(|p| player_display_name(&p.name, p.uuid))
                .collect();
            if i < *start {
                out = out.append(Component::literal(piece(i, *start)));
            }
            if !names.is_empty() {
                out = out.append(format_list(names));
            }
            i = *end;
        }
        if i < units.len() {
            out = out.append(Component::literal(piece(i, units.len())));
        }
        out
    }

    fn send_disguised(
        &self,
        to: PlayerId,
        message: &Component,
        chat_type: &str,
        name: &Component,
        target: Option<&Component>,
    ) {
        if let Some(p) = self.players.get(&to) {
            send(
                &p.sink,
                &DisguisedChat {
                    message: Text(message.to_tag()),
                    chat_type: VarInt(chat_type_id(chat_type)),
                    name: Text(name.to_tag()),
                    target_name: target.map(|t| Text(t.to_tag())),
                },
            );
        }
    }

    fn message_arg(&mut self, id: PlayerId, args: &Args, key: &str) -> Component {
        match args.get(key).map(|a| &a.value) {
            Some(Value::Message { text, selectors }) => {
                let (text, selectors) = (text.clone(), selectors.clone());
                self.resolve_message(id, &text, &selectors)
            }
            _ => Component::empty(),
        }
    }

    fn cmd_say(&mut self, id: PlayerId, args: &Args, chat_type: &str, key: &str) -> CommandResult {
        let message = self.message_arg(id, args, key);
        let Some(src) = self.players.get(&id) else { return Ok(0) };
        let name = player_display_name(&src.name, src.uuid);
        for to in self.order.clone() {
            self.send_disguised(to, &message, chat_type, &name, None);
        }
        Ok(1)
    }

    fn cmd_msg(&mut self, id: PlayerId, args: &Args) -> CommandResult {
        let targets = self.players_from(id, args.get("targets"))?;
        let message = self.message_arg(id, args, "message");
        let Some(src) = self.players.get(&id) else { return Ok(0) };
        let name = player_display_name(&src.name, src.uuid);
        for t in &targets {
            let Some(p) = self.players.get(t) else { continue };
            let target_name = player_display_name(&p.name, p.uuid);
            self.send_disguised(
                id,
                &message,
                "minecraft:msg_command_outgoing",
                &name,
                Some(&target_name),
            );
            self.send_disguised(*t, &message, "minecraft:msg_command_incoming", &name, None);
        }
        Ok(targets.len() as i32)
    }
}

/// `String.format("%.1f", x)`.
fn one_decimal(x: f64) -> String {
    format!("{x:.1}")
}

impl Game {
    fn cmd_tick_query(&mut self, id: PlayerId) -> CommandResult {
        let t = &self.ticking;
        let status = if t.sprint.is_some() {
            "commands.tick.status.sprinting"
        } else if t.frozen {
            "commands.tick.status.frozen"
        } else {
            "commands.tick.status.running"
        };
        let mut sorted = t.durations.clone();
        sorted.sort_by(f64::total_cmp);
        let mean =
            if sorted.is_empty() { 0.0 } else { sorted.iter().sum::<f64>() / sorted.len() as f64 };
        let pct = |p: f64| -> f64 {
            if sorted.is_empty() {
                return 0.0;
            }
            sorted[((sorted.len() as f64 * p) as usize).min(sorted.len() - 1)]
        };
        let rate = f64::from(t.rate);
        let count = sorted.len() as i32;
        let lines = [
            Component::translatable(status, vec![]),
            Component::translatable(
                "commands.tick.query.rate.running",
                vec![
                    one_decimal(rate).into(),
                    one_decimal(mean).into(),
                    one_decimal(1000.0 / rate).into(),
                ],
            ),
            Component::translatable(
                "commands.tick.query.percentiles",
                vec![
                    one_decimal(pct(0.5)).into(),
                    one_decimal(pct(0.95)).into(),
                    one_decimal(pct(0.99)).into(),
                    count.into(),
                ],
            ),
        ];
        for l in lines {
            self.send_success(id, l, false);
        }
        Ok(rate as i32)
    }

    fn cmd_tick_freeze(&mut self, id: PlayerId, frozen: bool) -> CommandResult {
        self.ticking.frozen = frozen;
        if !frozen {
            self.ticking.steps = 0;
        }
        self.broadcast_ticking_state();
        let key =
            if frozen { "commands.tick.status.frozen" } else { "commands.tick.status.running" };
        self.send_success(id, Component::translatable(key, vec![]), true);
        Ok(1)
    }

    fn cmd_tick_rate(&mut self, id: PlayerId, args: &Args) -> CommandResult {
        let Some(dispatch::ParsedArgument { value: Value::Float(rate), .. }) = args.get("rate")
        else {
            return Err(failure("argument.float.invalid", vec![]));
        };
        self.ticking.rate = *rate;
        self.broadcast_ticking_state();
        self.send_success(
            id,
            Component::translatable(
                "commands.tick.rate.success",
                vec![one_decimal(f64::from(*rate)).into()],
            ),
            true,
        );
        Ok(*rate as i32)
    }

    fn cmd_tick_step(&mut self, id: PlayerId, n: i32) -> CommandResult {
        if !self.ticking.frozen {
            // Stepping needs a frozen game: freeze it first, as the game does.
            self.ticking.frozen = true;
            self.broadcast_ticking_state();
        }
        self.ticking.steps = n.max(0) as u32;
        for p in self.players.values() {
            send(&p.sink, &TickingStep { steps: VarInt(n) });
        }
        self.send_success(
            id,
            Component::translatable("commands.tick.step.success", vec![n.into()]),
            true,
        );
        Ok(1)
    }

    fn cmd_tick_step_stop(&mut self, id: PlayerId) -> CommandResult {
        if self.ticking.steps == 0 {
            return Err(failure("commands.tick.step.stop.fail", vec![]));
        }
        self.ticking.steps = 0;
        for p in self.players.values() {
            send(&p.sink, &TickingStep { steps: VarInt(0) });
        }
        self.send_success(
            id,
            Component::translatable("commands.tick.step.stop.success", vec![]),
            true,
        );
        Ok(1)
    }

    fn cmd_tick_sprint(&mut self, id: PlayerId, n: i32) -> CommandResult {
        self.ticking.sprint = Some((n.max(1) as u32, n.max(1) as u32, Instant::now()));
        self.broadcast_ticking_state();
        self.send_success(
            id,
            Component::translatable("commands.tick.status.sprinting", vec![]),
            true,
        );
        Ok(1)
    }

    fn cmd_tick_sprint_stop(&mut self, id: PlayerId) -> CommandResult {
        if self.ticking.sprint.is_none() {
            return Err(failure("commands.tick.sprint.stop.fail", vec![]));
        }
        self.ticking.sprint = None;
        self.broadcast_ticking_state();
        self.send_success(
            id,
            Component::translatable("commands.tick.sprint.stop.success", vec![]),
            true,
        );
        Ok(1)
    }

    /// Counts a sprint tick down and reports when it is over.
    pub(super) fn finish_sprint_if_done(&mut self) {
        let Some((left, total, started)) = self.ticking.sprint else { return };
        if left > 1 {
            self.ticking.sprint = Some((left - 1, total, started));
            return;
        }
        self.ticking.sprint = None;
        self.broadcast_ticking_state();
        let secs = started.elapsed().as_secs_f64().max(1e-6);
        let tps = (f64::from(total) / secs) as i32;
        let report = Component::translatable(
            "commands.tick.sprint.report",
            vec![tps.into(), format!("{:.2}", secs * 1000.0 / f64::from(total)).into()],
        );
        self.tell_admins(None, Component::literal("Server"), report);
    }

    // ---- /time ---------------------------------------------------------------------------

    fn clock_names() -> &'static [&'static str] {
        ferrite_data::configuration::SYNCHRONIZED_REGISTRIES
            .iter()
            .find(|r| r.id == "minecraft:world_clock")
            .map_or(&[], |r| r.entries)
    }

    fn cmd_time(&mut self, id: PlayerId, key: &str, args: &Args) -> CommandResult {
        // `time of <clock> ...` is `time ...` on that clock; the plain forms use the overworld's.
        let (clock_id, rest) = match key.strip_prefix("time/of/clock/") {
            Some(rest) => {
                let name = match args.get("clock").map(|a| &a.value) {
                    Some(Value::Resource(r)) => r.clone(),
                    _ => "minecraft:overworld".to_string(),
                };
                (name, rest.to_string())
            }
            None => {
                ("minecraft:overworld".to_string(), key.trim_start_matches("time/").to_string())
            }
        };
        let Some(index) = Self::clock_names().iter().position(|n| *n == clock_id) else {
            return Err(failure(
                "argument.resource.not_found",
                vec![clock_id.into(), "minecraft:world_clock".into()],
            ));
        };
        let clock = self.clocks[index];
        let clock_arg = || -> Arg { clock_id.clone().into() };
        match rest.as_str() {
            "set/time" => {
                let t = i64::from(time_arg(args, "time"));
                if clock.total_ticks == t {
                    return Err(failure(
                        "commands.time.set.already_at_time",
                        vec![clock_arg(), (t as i32).into()],
                    ));
                }
                self.clocks[index].total_ticks = t;
                self.clocks[index].partial_tick = 0.0;
                self.broadcast_clock(index);
                self.send_success(
                    id,
                    Component::translatable(
                        "commands.time.set.absolute",
                        vec![clock_arg(), (t as i32).into()],
                    ),
                    true,
                );
                Ok(t as i32)
            }
            "set/timemarker" => {
                let marker = match args.get("timemarker").map(|a| &a.value) {
                    Some(Value::Identifier(m)) => m.clone(),
                    _ => String::new(),
                };
                let found = ferrite_data::timelines::TIMELINES
                    .iter()
                    .filter(|t| t.clock == clock_id)
                    .find_map(|t| t.markers.iter().find(|m| m.name == marker).map(|m| (t, m)));
                let Some((timeline, m)) = found else {
                    // (The game tells everyone the clock first, even when it cannot do the job.)
                    self.broadcast_clock(index);
                    return Err(failure(
                        "commands.time.no_time_marker_found",
                        vec![marker.into(), clock_arg()],
                    ));
                };
                let period = timeline.period_ticks.unwrap_or(i64::MAX);
                let now = clock.total_ticks;
                if now.rem_euclid(period) == m.ticks.rem_euclid(period) {
                    self.broadcast_clock(index);
                    return Err(failure(
                        "commands.time.set.already_at_time_marker",
                        vec![clock_arg(), marker.into()],
                    ));
                }
                let base = now - now.rem_euclid(period) + m.ticks;
                let target = if base <= now { base + period } else { base };
                self.clocks[index].total_ticks = target;
                self.clocks[index].partial_tick = 0.0;
                self.broadcast_clock(index);
                self.send_success(
                    id,
                    Component::translatable(
                        "commands.time.set.time_marker",
                        vec![clock_arg(), marker.into()],
                    ),
                    true,
                );
                Ok(target as i32)
            }
            "add/time" => {
                let t = i64::from(time_arg(args, "time"));
                let total = clock.total_ticks + t;
                self.clocks[index].total_ticks = total;
                self.broadcast_clock(index);
                self.send_success(
                    id,
                    Component::translatable(
                        "commands.time.set.absolute",
                        vec![clock_arg(), total.into()],
                    ),
                    true,
                );
                Ok(total as i32)
            }
            "pause" => {
                if clock.paused {
                    return Err(failure("commands.time.pause.already_paused", vec![clock_arg()]));
                }
                self.clocks[index].paused = true;
                self.broadcast_clock(index);
                self.send_success(
                    id,
                    Component::translatable("commands.time.pause", vec![clock_arg()]),
                    true,
                );
                Ok(1)
            }
            "resume" => {
                if !clock.paused {
                    return Err(failure("commands.time.pause.already_running", vec![clock_arg()]));
                }
                self.clocks[index].paused = false;
                self.broadcast_clock(index);
                self.send_success(
                    id,
                    Component::translatable("commands.time.resume", vec![clock_arg()]),
                    true,
                );
                Ok(1)
            }
            "rate/rate" => {
                let Some(dispatch::ParsedArgument { value: Value::Float(r), .. }) =
                    args.get("rate")
                else {
                    return Err(failure("argument.float.invalid", vec![]));
                };
                self.clocks[index].rate = *r;
                self.broadcast_clock(index);
                self.send_success(
                    id,
                    Component::translatable("commands.time.rate", vec![clock_arg(), (*r).into()]),
                    true,
                );
                Ok(*r as i32)
            }
            "query/time" => {
                self.send_success(
                    id,
                    Component::translatable(
                        "commands.time.query.absolute",
                        vec![clock_arg(), clock.total_ticks.into()],
                    ),
                    false,
                );
                Ok(clock.total_ticks as i32)
            }
            "query/gametime" => {
                let t = self.world.time();
                self.send_success(
                    id,
                    Component::translatable("commands.time.query.gametime", vec![t.into()]),
                    false,
                );
                Ok(t as i32)
            }
            "query/timeline" | "query/timeline/repetition" => {
                let name = match args.get("timeline").map(|a| &a.value) {
                    Some(Value::Resource(r)) => r.clone(),
                    _ => String::new(),
                };
                let Some(tl) = ferrite_data::timelines::timeline(&name) else {
                    return Err(failure(
                        "argument.resource.not_found",
                        vec![name.into(), "minecraft:timeline".into()],
                    ));
                };
                let t = clock.total_ticks;
                let (value, key) = if rest.ends_with("repetition") {
                    (
                        tl.period_ticks.map_or(0, |p| t.div_euclid(p)),
                        "commands.time.query.timeline.repetitions",
                    )
                } else {
                    (tl.period_ticks.map_or(t, |p| t.rem_euclid(p)), "commands.time.query.timeline")
                };
                self.send_success(
                    id,
                    Component::translatable(key, vec![name.into(), value.into()]),
                    false,
                );
                Ok(value as i32)
            }
            other => Err(Failure(Box::new(Component::literal(format!(
                "The command /time {} is not implemented in Ferrite yet.",
                other.replace('/', " ")
            ))))),
        }
    }
}
