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

    /// Sends a command's success message to the player who ran it.
    pub(super) fn send_success(&self, id: PlayerId, c: Component) {
        if let Some(p) = self.players.get(&id) {
            send(&p.sink, &SystemChat { content: Text(c.to_tag()), overlay: false });
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
            other => Err(Failure(Box::new(Component::literal(format!(
                "The command /{} is not implemented in Ferrite yet.",
                other.replace('/', " ")
            ))))),
        }
    }

    /// The players a selector argument names (entities that are not players are not handled
    /// yet).
    fn players_from(
        &self,
        source: PlayerId,
        arg: Option<&dispatch::ParsedArgument>,
    ) -> std::result::Result<Vec<PlayerId>, Failure> {
        let Some(dispatch::ParsedArgument { value: Value::Selector(sel), .. }) = arg else {
            return Err(failure("argument.entity.invalid", vec![]));
        };
        let found = self.select_players(source, sel);
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
        );
        Ok(1)
    }

    /// Players a selector picks (names, UUIDs and the player-only selector types for now).
    pub(super) fn select_players(
        &self,
        source: PlayerId,
        sel: &ferrite_commands::selector::Selector,
    ) -> Vec<PlayerId> {
        use ferrite_commands::selector::Target;
        match &sel.target {
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
            Target::Selector('s') => vec![source],
            Target::Selector(_) => self.order.clone(),
        }
    }
}
