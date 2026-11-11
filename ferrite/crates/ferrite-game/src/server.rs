//! The game thread: it owns the world and every player, and advances at 20 ticks per second.
//!
//! Connection tasks never touch game state. They send events over a channel and receive
//! packets through each player's [`PlayerSink`], so the game is single-writer and deterministic.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use bytes::Bytes;
use ferrite_blocks::{BlockPos, Chunks, Rules, World};
use ferrite_net::{PlayHost, PlayStart, PlayerHandle, PlayerSink};
use ferrite_protocol::codec::{Encode, VarInt, VarLong};
use ferrite_protocol::packet::Packet;
use ferrite_protocol::packets::play::*;
use ferrite_protocol::types::{Identifier, LenVec, Str, Text};
use ferrite_world::{Chunk, ChunkPos};
use uuid::Uuid;

mod survival;

use crate::chunkenc::{encode_sections, full_bright_light, heightmaps};
use crate::flat::FlatGenerator;
use crate::synchronized_id;

pub const TICKS_PER_SECOND: u64 = 20;
const TICK: Duration = Duration::from_millis(1000 / TICKS_PER_SECOND);
const KEEP_ALIVE_EVERY: u64 = 15 * TICKS_PER_SECOND;
const KEEP_ALIVE_TIMEOUT: u64 = 30 * TICKS_PER_SECOND;
/// Chunks sent to one player per tick.
const CHUNKS_PER_TICK: usize = 8;
/// The farthest one movement packet may move a player before it is treated as a teleport request
/// that was never granted.
const MAX_MOVE: f64 = 100.0;

pub type PlayerId = u64;

/// World and server settings.
#[derive(Clone, Debug)]
pub struct GameConfig {
    pub motd: String,
    pub max_players: u32,
    pub view_distance: u8,
    pub simulation_distance: u8,
    /// 0 survival, 1 creative, 2 adventure, 3 spectator.
    pub game_mode: u8,
    pub seed: i64,
    pub online_mode: bool,
    pub generator: FlatGenerator,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            motd: "A Ferrite server".into(),
            max_players: 20,
            view_distance: 10,
            simulation_distance: 10,
            game_mode: 1,
            seed: 0,
            online_mode: false,
            generator: FlatGenerator::overworld(),
        }
    }
}

enum Event {
    Join { id: PlayerId, start: PlayStart, sink: PlayerSink },
    Packet { id: PlayerId, packet_id: i32, body: Bytes },
    Leave { id: PlayerId },
}

/// The connection layer's way into the game.
#[derive(Debug)]
pub struct GameHost {
    tx: Mutex<Sender<Event>>,
    next_id: AtomicU64,
}

struct Handle {
    id: PlayerId,
    tx: Sender<Event>,
}

impl PlayerHandle for Handle {
    fn packet(&mut self, packet_id: i32, body: Bytes) {
        let _ = self.tx.send(Event::Packet { id: self.id, packet_id, body });
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        let _ = self.tx.send(Event::Leave { id: self.id });
    }
}

impl PlayHost for GameHost {
    fn join(&self, start: PlayStart, sink: PlayerSink) -> Option<Box<dyn PlayerHandle>> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let tx = self.tx.lock().ok()?.clone();
        tx.send(Event::Join { id, start, sink }).ok()?;
        Some(Box::new(Handle { id, tx }))
    }
}

/// A running game thread.
#[derive(Debug)]
pub struct GameThread {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl GameThread {
    /// Stops the game after the current tick and waits for it.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for GameThread {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Starts the game on its own thread.
pub fn start(config: GameConfig) -> (Arc<GameHost>, GameThread) {
    let (tx, rx) = mpsc::channel();
    let host = Arc::new(GameHost { tx: Mutex::new(tx), next_id: AtomicU64::new(1) });
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let thread = std::thread::Builder::new()
        .name("ferrite-game".into())
        .spawn(move || Game::new(config, rx).run(&stop2))
        .expect("spawn the game thread");
    (host, GameThread { stop, thread: Some(thread) })
}

// ---- state ------------------------------------------------------------------------------

/// What other players last saw of an entity, so movement is sent as small deltas that never
/// drift from the true position.
#[derive(Clone, Copy, Debug)]
struct SentState {
    /// Position in 1/4096 blocks.
    base: [i64; 3],
    y_rot: u8,
    x_rot: u8,
    head: u8,
}

fn quantize(v: f64) -> i64 {
    (v * 4096.0).floor() as i64
}

fn angle(degrees: f32) -> u8 {
    ((degrees * 256.0 / 360.0).floor() as i32 & 0xFF) as u8
}

struct Player {
    entity_id: i32,
    uuid: Uuid,
    name: String,
    sink: PlayerSink,
    x: f64,
    y: f64,
    z: f64,
    yaw: f32,
    pitch: f32,
    on_ground: bool,
    view_distance: i32,
    center: (i32, i32),
    loaded: HashSet<(i32, i32)>,
    queue: Vec<(i32, i32)>,
    next_teleport: i32,
    pending_teleport: Option<i32>,
    keep_alive: Option<(i64, u64)>,
    latency_ms: i32,
    tracked: HashSet<PlayerId>,
    sent: SentState,
    moved: bool,
    held_slot: u8,
    menu: crate::inventory::Menu,
    sneaking: bool,
    /// The latest block-change sequence number to acknowledge at the end of the tick.
    ack: Option<i32>,
    game_mode: u8,
    joined: u64,
    /// The block being dug (survival).
    dig: Option<survival::Dig>,
    /// The inventory window's state id, bumped on every change sent.
    inv_state: i32,
    /// Dropped items this player has been told about.
    tracked_items: HashSet<i32>,
}

/// The world's random source (splitmix64), for the few placements that roll dice.
struct SplitMix(u64);

impl SplitMix {
    fn below(&mut self, bound: u32) -> u32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        ((z >> 32) as u32) % bound.max(1)
    }
}

struct Game {
    cfg: GameConfig,
    /// State of the random source.
    rng: u64,
    rx: Receiver<Event>,
    world: World<Chunks>,
    /// Encoded chunk packets, dropped when a block in the chunk changes.
    chunk_cache: HashMap<(i32, i32), Arc<Vec<u8>>>,
    players: HashMap<PlayerId, Player>,
    /// Dropped items.
    items: std::collections::BTreeMap<i32, crate::items::ItemEntity>,
    /// Join order, for stable iteration.
    order: Vec<PlayerId>,
    next_entity: i32,
    tick: u64,
}

fn send<P: Packet>(sink: &PlayerSink, packet: &P) {
    let mut body = Vec::new();
    if packet.encode(&mut body).is_ok() {
        sink.send(P::ID, body);
    }
}

impl Game {
    fn new(cfg: GameConfig, rx: Receiver<Event>) -> Self {
        let mut world = World::new(Chunks::new(cfg.generator.dimension), Rules::default());
        world.track_changes(true);
        Game {
            cfg,
            rng: 0x2545_F491_4F6C_DD1D,
            rx,
            world,
            chunk_cache: HashMap::new(),
            players: HashMap::new(),
            items: std::collections::BTreeMap::new(),
            order: Vec::new(),
            next_entity: 1,
            tick: 0,
        }
    }

    fn run(mut self, stop: &AtomicBool) {
        let mut next = Instant::now();
        while !stop.load(Ordering::Relaxed) {
            while let Ok(event) = self.rx.try_recv() {
                self.on_event(event);
            }
            self.tick();
            next += TICK;
            let now = Instant::now();
            if next > now {
                std::thread::sleep(next - now);
            } else if now - next > Duration::from_secs(1) {
                // Far behind (a stall): skip the backlog instead of racing to catch up.
                next = now;
            }
        }
        for p in self.players.values() {
            send(&p.sink, &Disconnect { reason: Text::plain("Server closed") });
            p.sink.close();
        }
    }

    fn spawn_point(&self) -> (f64, f64, f64) {
        (0.5, f64::from(self.cfg.generator.surface_y() + 1), 0.5)
    }

    // ---- events ---------------------------------------------------------------------------

    fn on_event(&mut self, event: Event) {
        match event {
            Event::Join { id, start, sink } => self.on_join(id, start, sink),
            Event::Leave { id } => self.on_leave(id),
            Event::Packet { id, packet_id, body } => self.on_packet(id, packet_id, &body),
        }
    }

    fn on_join(&mut self, id: PlayerId, start: PlayStart, sink: PlayerSink) {
        let uuid = start.profile.uuid;
        // A second login with the same account replaces the first.
        if let Some(old) = self.players.iter().find(|(_, p)| p.uuid == uuid).map(|(k, _)| *k) {
            if let Some(p) = self.players.get(&old) {
                send(
                    &p.sink,
                    &Disconnect { reason: Text::plain("You logged in from another location") },
                );
                p.sink.close();
            }
            self.on_leave(old);
        }
        let name = start.profile.name.0.clone();
        let (x, y, z) = self.spawn_point();
        let view = start
            .client
            .as_ref()
            .map(|c| i32::from(c.view_distance))
            .filter(|v| *v > 0)
            .unwrap_or(i32::from(self.cfg.view_distance))
            .min(i32::from(self.cfg.view_distance));
        let entity_id = self.next_entity;
        self.next_entity += 1;
        let player = Player {
            entity_id,
            uuid,
            name: name.clone(),
            sink,
            x,
            y,
            z,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: true,
            view_distance: view.max(2),
            center: (x.floor() as i32 >> 4, z.floor() as i32 >> 4),
            loaded: HashSet::new(),
            queue: Vec::new(),
            next_teleport: 1,
            pending_teleport: None,
            keep_alive: None,
            latency_ms: 0,
            tracked: HashSet::new(),
            sent: SentState {
                base: [quantize(x), quantize(y), quantize(z)],
                y_rot: 0,
                x_rot: 0,
                head: 0,
            },
            moved: false,
            held_slot: 0,
            menu: crate::inventory::Menu::default(),
            sneaking: false,
            ack: None,
            game_mode: self.cfg.game_mode,
            joined: self.tick,
            dig: None,
            inv_state: 1,
            tracked_items: HashSet::new(),
        };
        self.players.insert(id, player);
        self.order.push(id);
        self.send_join_sequence(id);
        // Tell everyone (including the newcomer) about the tab list entry.
        let entry = self.tab_entry(id);
        let update = PlayerInfoUpdate { actions: 0xFF, entries: vec![entry] };
        for p in self.players.values() {
            send(&p.sink, &update);
        }
        // The newcomer also needs the entries of the players already here.
        let others: Vec<PlayerInfoEntry> =
            self.order.iter().filter(|o| **o != id).map(|o| self.tab_entry(*o)).collect();
        if !others.is_empty()
            && let Some(p) = self.players.get(&id)
        {
            send(&p.sink, &PlayerInfoUpdate { actions: 0xFF, entries: others });
        }
        self.broadcast_chat(&format!("{name} joined the game"));
        tracing::info!("{name} joined ({} online)", self.players.len());
    }

    fn tab_entry(&self, id: PlayerId) -> PlayerInfoEntry {
        let p = &self.players[&id];
        PlayerInfoEntry {
            uuid: p.uuid,
            profile: Some((Str::new(p.name.clone()), LenVec(Vec::new()))),
            game_mode: Some(VarInt(i32::from(p.game_mode))),
            listed: Some(true),
            latency: Some(VarInt(p.latency_ms)),
            display_name: Some(None),
            list_order: Some(VarInt(0)),
            show_hat: Some(true),
        }
    }

    fn on_leave(&mut self, id: PlayerId) {
        let Some(player) = self.players.remove(&id) else { return };
        self.order.retain(|o| *o != id);
        let gone = PlayerInfoRemove { uuids: LenVec(vec![player.uuid]) };
        let removed = RemoveEntities { ids: LenVec(vec![VarInt(player.entity_id)]) };
        for p in self.players.values_mut() {
            send(&p.sink, &gone);
            if p.tracked.remove(&id) {
                send(&p.sink, &removed);
            }
        }
        self.broadcast_chat(&format!("{} left the game", player.name));
        tracing::info!("{} left ({} online)", player.name, self.players.len());
    }

    fn broadcast_chat(&self, text: &str) {
        let msg = SystemChat { content: Text::plain(text), overlay: false };
        for p in self.players.values() {
            send(&p.sink, &msg);
        }
    }

    // ---- joining --------------------------------------------------------------------------

    fn send_join_sequence(&mut self, id: PlayerId) {
        let (x, y, z) = {
            let p = &self.players[&id];
            (p.x, p.y, p.z)
        };
        let cfg = &self.cfg;
        let dim = cfg.generator.dimension;
        let overworld = Identifier::new("minecraft:overworld").expect("valid identifier");
        let dimension_type =
            synchronized_id("minecraft:dimension_type", "minecraft:overworld").unwrap_or(0);
        let sea_level = dim.min_y + 1;
        // Vanilla reports the first free block above the ground as the spawn.
        let spawn_block = BlockPos::new(0, cfg.generator.surface_y() + 1, 0);
        let (gm, entity_id, view) = {
            let p = &self.players[&id];
            (p.game_mode, p.entity_id, p.view_distance)
        };
        let p = &self.players[&id];
        let s = &p.sink;
        send(
            s,
            &Login {
                entity_id,
                hardcore: false,
                levels: LenVec(
                    ["minecraft:overworld", "minecraft:the_nether", "minecraft:the_end"]
                        .iter()
                        .map(|n| Identifier::new(*n).expect("valid identifier"))
                        .collect(),
                ),
                max_players: VarInt(cfg.max_players as i32),
                view_distance: VarInt(view),
                simulation_distance: VarInt(i32::from(cfg.simulation_distance)),
                reduced_debug_info: false,
                show_death_screen: true,
                limited_crafting: false,
                spawn: SpawnInfo {
                    dimension_type: VarInt(dimension_type as i32),
                    dimension: overworld.clone(),
                    seed: cfg.seed,
                    game_mode: VarInt(i32::from(gm)),
                    previous_game_mode: None,
                    is_debug: false,
                    is_flat: true,
                    last_death: None,
                    portal_cooldown: VarInt(0),
                    sea_level: VarInt(sea_level),
                },
                online_mode: cfg.online_mode,
                enforces_secure_chat: false,
            },
        );
        send(s, &ChangeDifficulty { difficulty: 1, locked: false });
        send(
            s,
            &PlayerAbilities { flags: abilities_for(gm), flying_speed: 0.05, walking_speed: 0.1 },
        );
        send(s, &SetHeldSlot { slot: VarInt(0) });
        send(s, &Commands { nodes: LenVec(vec![CommandNode::default()]), root: VarInt(0) });
        send(
            s,
            &PlayerPosition {
                teleport_id: VarInt(0),
                x,
                y,
                z,
                dx: 0.0,
                dy: 0.0,
                dz: 0.0,
                y_rot: 0.0,
                x_rot: 0.0,
                relative: 0,
            },
        );
        send(s, &ServerData { motd: Text::plain(cfg.motd.clone()), icon: None });
        send(
            s,
            &InitializeBorder {
                center_x: 0.0,
                center_z: 0.0,
                old_size: 59_999_968.0,
                new_size: 59_999_968.0,
                lerp_time: VarLong(0),
                absolute_max_size: VarInt(29_999_984),
                warning_blocks: VarInt(5),
                warning_time: VarInt(15),
            },
        );
        send(s, &self.time_packet());
        send(
            s,
            &SetDefaultSpawnPosition {
                dimension: overworld,
                pos: PackedPos::new(spawn_block.x, spawn_block.y, spawn_block.z),
                yaw: 0.0,
                pitch: 0.0,
            },
        );
        send(s, &GameEvent { event: 13, param: 0.0 });
        send(s, &TickingState { tick_rate: TICKS_PER_SECOND as f32, frozen: false });
        send(s, &TickingStep { steps: VarInt(0) });
        send(s, &SetChunkCacheCenter { x: VarInt(p.center.0), z: VarInt(p.center.1) });
        send(s, &SetHealth { health: 20.0, food: VarInt(20), saturation: 5.0 });
        send(s, &SetExperience { progress: 0.0, total: VarInt(0), level: VarInt(0) });
        send(
            s,
            &ContainerSetContent {
                window_id: VarInt(0),
                state_id: VarInt(1),
                items: LenVec(vec![ItemStack::default(); 46]),
                carried: ItemStack::default(),
            },
        );
        self.players.get_mut(&id).expect("just inserted").pending_teleport = Some(0);
        self.plan_chunks(id);
    }

    fn time_packet(&self) -> SetTime {
        let t = self.world.time();
        SetTime {
            game_time: t,
            clock_updates: LenVec(
                [0, 1]
                    .iter()
                    .map(|c| ClockUpdate {
                        clock: VarInt(*c),
                        total_ticks: VarLong(t),
                        partial_tick: 0.0,
                        rate: 1.0,
                    })
                    .collect(),
            ),
        }
    }

    // ---- chunks ---------------------------------------------------------------------------

    /// Whether chunk `(x, z)` is within a player's view, using vanilla's rounded-square shape.
    fn in_view(center: (i32, i32), view: i32, x: i32, z: i32) -> bool {
        let dx = ((x - center.0).abs() - 1).max(0);
        let dz = ((z - center.1).abs() - 1).max(0);
        dx * dx + dz * dz <= view * view
    }

    /// Recomputes which chunks a player should have and queues the missing ones, nearest first.
    fn plan_chunks(&mut self, id: PlayerId) {
        let Some(p) = self.players.get_mut(&id) else { return };
        let (center, view) = (p.center, p.view_distance);
        // Forget what is out of view.
        let out: Vec<(i32, i32)> =
            p.loaded.iter().copied().filter(|c| !Self::in_view(center, view, c.0, c.1)).collect();
        for c in &out {
            p.loaded.remove(c);
            send(
                &p.sink,
                &ForgetLevelChunk { pos: (i64::from(c.1) << 32) | i64::from(c.0 as u32) },
            );
        }
        let mut wanted = Vec::new();
        for dz in -(view + 1)..=(view + 1) {
            for dx in -(view + 1)..=(view + 1) {
                let c = (center.0 + dx, center.1 + dz);
                if Self::in_view(center, view, c.0, c.1) && !p.loaded.contains(&c) {
                    wanted.push((dx * dx + dz * dz, c));
                }
            }
        }
        // The queue is consumed from the back, so sort farthest first.
        wanted.sort_by_key(|(d, c)| std::cmp::Reverse((*d, c.1, c.0)));
        p.queue = wanted.into_iter().map(|(_, c)| c).collect();
    }

    fn chunk_packet(&mut self, cx: i32, cz: i32) -> Arc<Vec<u8>> {
        if let Some(b) = self.chunk_cache.get(&(cx, cz)) {
            return b.clone();
        }
        let pos = ChunkPos::new(cx, cz);
        if !self.world.blocks.is_loaded(pos) {
            let chunk: Chunk = self.cfg.generator.generate();
            self.world.blocks.insert(pos, chunk);
        }
        let chunk = self.world.blocks.get(pos).expect("just ensured");
        let packet = LevelChunkWithLight {
            x: cx,
            z: cz,
            heightmaps: LenVec(heightmaps(chunk)),
            buffer: ferrite_protocol::types::ByteVec(encode_sections(chunk)),
            block_entities: LenVec(Vec::new()),
            light: full_bright_light(chunk.sections().len()),
        };
        let mut body = Vec::new();
        packet.encode(&mut body).expect("chunk packets encode");
        let body = Arc::new(body);
        self.chunk_cache.insert((cx, cz), body.clone());
        body
    }

    fn stream_chunks(&mut self, id: PlayerId) {
        let batch: Vec<(i32, i32)> = {
            let Some(p) = self.players.get_mut(&id) else { return };
            if p.queue.is_empty() {
                return;
            }
            let n = CHUNKS_PER_TICK.min(p.queue.len());
            let at = p.queue.len() - n;
            p.queue.split_off(at).into_iter().rev().collect()
        };
        let bodies: Vec<_> = batch.iter().map(|c| self.chunk_packet(c.0, c.1)).collect();
        let Some(p) = self.players.get_mut(&id) else { return };
        send(&p.sink, &ChunkBatchStart {});
        for (c, body) in batch.iter().zip(&bodies) {
            p.sink.send(LevelChunkWithLight::ID, Bytes::copy_from_slice(body));
            p.loaded.insert(*c);
        }
        send(&p.sink, &ChunkBatchFinished { batch_size: VarInt(batch.len() as i32) });
    }

    // ---- packets from players -------------------------------------------------------------

    fn on_packet(&mut self, id: PlayerId, packet_id: i32, body: &[u8]) {
        macro_rules! decode {
            ($ty:ty) => {
                match ferrite_protocol::codec::decode_exact::<$ty>(body) {
                    Ok(p) => p,
                    Err(_) => return self.kick(id, "Invalid packet"),
                }
            };
        }
        match packet_id {
            AcceptTeleportation::ID => {
                let p = decode!(AcceptTeleportation);
                if let Some(pl) = self.players.get_mut(&id)
                    && pl.pending_teleport == Some(p.teleport_id.0)
                {
                    pl.pending_teleport = None;
                }
            }
            ServerboundKeepAlive::ID => {
                let p = decode!(ServerboundKeepAlive);
                let tick = self.tick;
                if let Some(pl) = self.players.get_mut(&id)
                    && let Some((expected, sent)) = pl.keep_alive
                    && expected == p.id
                {
                    pl.latency_ms = ((tick - sent) * 50) as i32;
                    pl.keep_alive = None;
                }
            }
            MovePlayerPos::ID => {
                let p = decode!(MovePlayerPos);
                self.on_move(id, Some((p.x, p.y, p.z)), None, p.flags & 1 != 0);
            }
            MovePlayerPosRot::ID => {
                let p = decode!(MovePlayerPosRot);
                self.on_move(id, Some((p.x, p.y, p.z)), Some((p.y_rot, p.x_rot)), p.flags & 1 != 0);
            }
            MovePlayerRot::ID => {
                let p = decode!(MovePlayerRot);
                self.on_move(id, None, Some((p.y_rot, p.x_rot)), p.flags & 1 != 0);
            }
            MovePlayerStatusOnly::ID => {
                let p = decode!(MovePlayerStatusOnly);
                if let Some(pl) = self.players.get_mut(&id) {
                    pl.on_ground = p.flags & 1 != 0;
                }
            }
            PlayClientInformation::ID => {
                let p = decode!(PlayClientInformation);
                let server_max = i32::from(self.cfg.view_distance);
                let mut replan = false;
                if let Some(pl) = self.players.get_mut(&id) {
                    let v = i32::from(p.view_distance).clamp(2, server_max);
                    replan = v != pl.view_distance;
                    pl.view_distance = v;
                }
                if replan {
                    self.plan_chunks(id);
                }
            }
            Chat::ID => {
                let p = decode!(Chat);
                self.on_chat(id, &p.message.0);
            }
            SetCarriedItem::ID => {
                let p = decode!(SetCarriedItem);
                if let Some(pl) = self.players.get_mut(&id)
                    && (0..9).contains(&p.slot)
                {
                    pl.held_slot = p.slot as u8;
                }
            }
            ContainerClick::ID => {
                let p = decode!(ContainerClick);
                self.on_container_click(id, &p);
            }
            ContainerClose::ID => {
                let p = decode!(ContainerClose);
                if p.window_id.0 == 0 {
                    self.close_inventory(id);
                }
            }
            PlayerAction::ID => {
                let p = decode!(PlayerAction);
                self.on_player_action(id, &p);
            }
            UseItemOn::ID => {
                let p = decode!(UseItemOn);
                self.on_use_item_on(id, &p);
            }
            SetCreativeModeSlot::ID => {
                let p = decode!(SetCreativeModeSlot);
                if let Some(pl) = self.players.get_mut(&id)
                    && pl.game_mode == 1
                    && (0..46).contains(&p.slot)
                {
                    let mut item = p.item;
                    item.count = item.count.clamp(0, 99);
                    pl.menu.slots[p.slot as usize] = item;
                }
            }
            PlayerInput::ID => {
                let p = decode!(PlayerInput);
                if let Some(pl) = self.players.get_mut(&id) {
                    pl.sneaking = p.flags & 32 != 0;
                }
            }
            // Accepted and ignored for now.
            ChunkBatchReceived::ID
            | PlayerLoaded::ID
            | ClientTickEnd::ID
            | ServerboundCustomPayload::ID
            | PlayerCommand::ID
            | Punch::ID => {}
            _ => {}
        }
    }

    /// Whether the player may touch the block at `pos`: loaded, and within reach.
    fn in_reach(&self, id: PlayerId, pos: BlockPos) -> bool {
        let Some(p) = self.players.get(&id) else { return false };
        if !p.loaded.contains(&(pos.x >> 4, pos.z >> 4)) {
            return false;
        }
        let (dx, dy, dz) = (
            f64::from(pos.x) + 0.5 - p.x,
            f64::from(pos.y) + 0.5 - (p.y + 1.62),
            f64::from(pos.z) + 0.5 - p.z,
        );
        dx * dx + dy * dy + dz * dz <= 6.0 * 6.0
    }

    /// Tells one player what is really at `pos` (used to undo a change the server refused).
    fn resend_block(&self, id: PlayerId, pos: BlockPos) {
        if let (Some(p), Some(state)) = (self.players.get(&id), self.world.block(pos)) {
            send(
                &p.sink,
                &BlockUpdate {
                    pos: PackedPos::new(pos.x, pos.y, pos.z),
                    state: VarInt(i32::from(state)),
                },
            );
        }
    }

    /// Acknowledges a block-change sequence number. Like vanilla, the acknowledgement goes out at
    /// the end of the tick, after the block updates it refers to.
    fn ack(&mut self, id: PlayerId, sequence: VarInt) {
        if let Some(p) = self.players.get_mut(&id) {
            p.ack = Some(sequence.0);
        }
    }

    fn on_player_action(&mut self, id: PlayerId, a: &PlayerAction) {
        let pos = BlockPos::new(a.pos.x(), a.pos.y(), a.pos.z());
        let mode = self.players.get(&id).map_or(0, |p| p.game_mode);
        match a.action.0 {
            // Start digging: creative breaks the block at once, survival begins timed mining.
            0 => {
                if !self.in_reach(id, pos) || mode == 2 || mode == 3 {
                    self.resend_block(id, pos);
                } else if mode == 1 {
                    self.world.destroy_block(pos);
                } else {
                    self.start_dig(id, pos);
                }
            }
            1 => self.abort_dig(id),
            3 | 4 => self.drop_held(id, a.action.0 == 3),
            2 => {
                if mode == 0 && self.in_reach(id, pos) {
                    self.stop_dig(id, pos);
                } else {
                    self.resend_block(id, pos);
                }
            }
            _ => {}
        }
        if matches!(a.action.0, 0..=2) {
            self.ack(id, a.sequence);
        }
    }

    /// The block a held item places: items and blocks share names.
    fn block_for_item(item: i32) -> Option<&'static str> {
        let name = ferrite_data::registries::item::names.get(usize::try_from(item).ok()?)?;
        let name = name.trim_start_matches("minecraft:");
        // The crop item is not the seeds that plant the crop block.
        if matches!(name, "wheat" | "lily_pad" | "frogspawn") {
            return None;
        }
        ferrite_data::blocks::block_by_name(name).map(|b| b.name.trim_start_matches("minecraft:"))
    }

    fn on_use_item_on(&mut self, id: PlayerId, u: &UseItemOn) {
        let target = BlockPos::new(u.pos.x(), u.pos.y(), u.pos.z());
        let Some(face) = ferrite_blocks::Direction::ALL.get(u.face.0 as usize).copied() else {
            return self.kick(id, "Invalid packet");
        };
        let (sneaking, held) = match self.players.get(&id) {
            Some(p) => (p.sneaking, p.menu.slots[36 + usize::from(p.held_slot)]),
            None => return,
        };
        let mut place_pos = target.offset(face);
        if self.in_reach(id, target) {
            // A block that reacts to a click (levers, buttons) takes it unless the player sneaks
            // with something in hand.
            let uses_block = !(sneaking && held.count > 0);
            if uses_block && self.world.use_block(target) {
                self.ack(id, u.sequence);
                return;
            }
            // Clicking something replaceable (snow, grass, water) places into it.
            if self.world.block(target).is_some_and(|s| {
                ferrite_data::shapes::facts(s).has(ferrite_data::shapes::flag::REPLACEABLE)
            }) {
                place_pos = target;
            }
            let placed = self.try_place(id, u, target, face, sneaking, held.item, held.count > 0);
            if !placed {
                self.resend_block(id, place_pos);
            }
        } else {
            self.resend_block(id, target);
            self.resend_block(id, place_pos);
        }
        self.ack(id, u.sequence);
    }

    /// Places the held item's block the way a player's click does; whether anything was placed.
    #[allow(clippy::too_many_arguments)]
    fn try_place(
        &mut self,
        id: PlayerId,
        u: &UseItemOn,
        target: BlockPos,
        face: ferrite_blocks::Direction,
        sneaking: bool,
        item: i32,
        holding: bool,
    ) -> bool {
        if !holding {
            return false;
        }
        let Some(name) = Self::block_for_item(item) else { return false };
        let Some((yaw, pitch)) = self.players.get(&id).map(|p| (p.yaw, p.pitch)) else {
            return false;
        };
        let place_pos = if self.world.block(target).is_some_and(|s| {
            ferrite_data::shapes::facts(s).has(ferrite_data::shapes::flag::REPLACEABLE)
        }) {
            target
        } else {
            target.offset(face)
        };
        if !self.in_reach(id, place_pos) {
            return false;
        }
        let ctx = ferrite_blocks::PlaceContext {
            clicked: target,
            face,
            cursor: [u.cursor_x, u.cursor_y, u.cursor_z],
            yaw,
            pitch,
            sneaking,
            // There are no operators yet, so game-master blocks cannot be placed.
            operator: false,
        };
        let players = &self.players;
        let mut dice = SplitMix(self.rng);
        let placed = self
            .world
            .place_block(
                name,
                &ctx,
                |state, pos| Self::overlaps_player(players, state, pos),
                &mut |bound| dice.below(bound),
            )
            .is_some();
        self.rng = dice.0;
        placed
    }

    /// Whether a block in `state` at `pos` would overlap a player (which vanilla does not allow).
    fn overlaps_player(players: &HashMap<PlayerId, Player>, state: u16, pos: BlockPos) -> bool {
        let boxes = ferrite_data::shapes::facts(state).collision_shape();
        if boxes.is_empty() {
            return false;
        }
        players.values().any(|p| {
            let (px0, px1) = (p.x - 0.3, p.x + 0.3);
            let (py0, py1) = (p.y, p.y + 1.8);
            let (pz0, pz1) = (p.z - 0.3, p.z + 0.3);
            boxes.iter().any(|b| {
                let (x0, y0, z0) =
                    (f64::from(pos.x) + b[0], f64::from(pos.y) + b[1], f64::from(pos.z) + b[2]);
                let (x1, y1, z1) =
                    (f64::from(pos.x) + b[3], f64::from(pos.y) + b[4], f64::from(pos.z) + b[5]);
                px0 < x1 && px1 > x0 && py0 < y1 && py1 > y0 && pz0 < z1 && pz1 > z0
            })
        })
    }

    fn kick(&mut self, id: PlayerId, reason: &str) {
        if let Some(p) = self.players.get(&id) {
            send(&p.sink, &Disconnect { reason: Text::plain(reason) });
            p.sink.close();
        }
        self.on_leave(id);
    }

    fn on_chat(&mut self, id: PlayerId, message: &str) {
        let Some(p) = self.players.get(&id) else { return };
        let clean: String = message.chars().filter(|c| !c.is_control()).collect();
        if clean.is_empty() {
            return;
        }
        let line = format!("<{}> {}", p.name, clean);
        tracing::info!("{line}");
        self.broadcast_chat(&line);
    }

    fn on_move(
        &mut self,
        id: PlayerId,
        pos: Option<(f64, f64, f64)>,
        rot: Option<(f32, f32)>,
        on_ground: bool,
    ) {
        let Some(p) = self.players.get_mut(&id) else { return };
        // Until the client confirmed a teleport it may still be sending positions from before.
        if p.pending_teleport.is_some() {
            return;
        }
        let dim = self.cfg.generator.dimension;
        if let Some((x, y, z)) = pos {
            let finite = x.is_finite() && y.is_finite() && z.is_finite();
            let in_world = x.abs() < 3.0e7
                && z.abs() < 3.0e7
                && y > f64::from(dim.min_y) - 64.0
                && y < f64::from(dim.max_y()) + 64.0;
            let far = (x - p.x).abs().max((y - p.y).abs()).max((z - p.z).abs()) > MAX_MOVE;
            if !finite || !in_world || far {
                // Put the client back where we think it is.
                let tp = p.next_teleport + 1;
                p.next_teleport = tp;
                p.pending_teleport = Some(tp);
                let (px, py, pz, yaw, pitch) = (p.x, p.y, p.z, p.yaw, p.pitch);
                send(
                    &p.sink,
                    &PlayerPosition {
                        teleport_id: VarInt(tp),
                        x: px,
                        y: py,
                        z: pz,
                        dx: 0.0,
                        dy: 0.0,
                        dz: 0.0,
                        y_rot: yaw,
                        x_rot: pitch,
                        relative: 0,
                    },
                );
                return;
            }
            p.x = x;
            p.y = y;
            p.z = z;
        }
        if let Some((yaw, pitch)) = rot
            && yaw.is_finite()
            && pitch.is_finite()
        {
            p.yaw = yaw;
            p.pitch = pitch.clamp(-90.0, 90.0);
        }
        p.on_ground = on_ground;
        p.moved = true;
    }

    // ---- ticking --------------------------------------------------------------------------

    fn tick(&mut self) {
        self.tick += 1;
        self.world.tick();
        self.tick_digging();
        self.broadcast_block_changes();
        self.tick_items();
        let ids: Vec<PlayerId> = self.order.clone();
        for id in &ids {
            self.keep_alive(*id);
            self.update_center(*id);
            self.stream_chunks(*id);
        }
        self.update_tracking();
        self.broadcast_movement();
        for p in self.players.values_mut() {
            if let Some(sequence) = p.ack.take() {
                send(&p.sink, &BlockChangedAck { sequence: VarInt(sequence) });
            }
        }
        if self.tick.is_multiple_of(20) {
            let t = self.time_packet();
            for p in self.players.values() {
                send(&p.sink, &t);
            }
        }
    }

    fn keep_alive(&mut self, id: PlayerId) {
        let tick = self.tick;
        let Some(p) = self.players.get_mut(&id) else { return };
        match p.keep_alive {
            Some((_, sent)) if tick - sent > KEEP_ALIVE_TIMEOUT => self.kick(id, "Timed out"),
            None if (tick - p.joined).is_multiple_of(KEEP_ALIVE_EVERY) && tick > p.joined => {
                let kid = tick as i64;
                p.keep_alive = Some((kid, tick));
                send(&p.sink, &KeepAlive { id: kid });
            }
            _ => {}
        }
    }

    fn update_center(&mut self, id: PlayerId) {
        let Some(p) = self.players.get_mut(&id) else { return };
        let c = (p.x.floor() as i32 >> 4, p.z.floor() as i32 >> 4);
        if c != p.center && p.pending_teleport.is_none() {
            p.center = c;
            send(&p.sink, &SetChunkCacheCenter { x: VarInt(c.0), z: VarInt(c.1) });
            self.plan_chunks(id);
        }
    }

    fn broadcast_block_changes(&mut self) {
        let changed = self.world.take_changes();
        if changed.is_empty() {
            return;
        }
        for pos in changed {
            let Some(state) = self.world.block(pos) else { continue };
            let chunk = (pos.x >> 4, pos.z >> 4);
            self.chunk_cache.remove(&chunk);
            let update = BlockUpdate {
                pos: PackedPos::new(pos.x, pos.y, pos.z),
                state: VarInt(i32::from(state)),
            };
            for p in self.players.values() {
                if p.loaded.contains(&chunk) {
                    send(&p.sink, &update);
                }
            }
        }
    }

    // ---- entities -------------------------------------------------------------------------

    fn add_entity_packet(p: &Player) -> AddEntity {
        AddEntity {
            id: VarInt(p.entity_id),
            uuid: p.uuid,
            kind: VarInt(ferrite_data::registries::entity_type::PLAYER as i32),
            x: p.x,
            y: p.y,
            z: p.z,
            velocity: LpVec3::ZERO,
            x_rot: angle(p.pitch),
            y_rot: angle(p.yaw),
            y_head_rot: angle(p.yaw),
            data: VarInt(0),
        }
    }

    /// Spawns and despawns other players for each player according to distance.
    fn update_tracking(&mut self) {
        let ids = self.order.clone();
        for &a in &ids {
            let range = {
                let p = &self.players[&a];
                f64::from(p.view_distance.min(i32::from(self.cfg.simulation_distance)) * 16)
            };
            for &b in &ids {
                if a == b {
                    continue;
                }
                let (pa, pb) = (&self.players[&a], &self.players[&b]);
                let near = (pa.x - pb.x).abs() <= range && (pa.z - pb.z).abs() <= range;
                let tracked = pa.tracked.contains(&b);
                if near && !tracked {
                    send(&pa.sink, &Self::add_entity_packet(pb));
                    self.players.get_mut(&a).expect("exists").tracked.insert(b);
                } else if !near && tracked {
                    send(&pa.sink, &RemoveEntities { ids: LenVec(vec![VarInt(pb.entity_id)]) });
                    self.players.get_mut(&a).expect("exists").tracked.remove(&b);
                }
            }
        }
    }

    fn broadcast_movement(&mut self) {
        let ids = self.order.clone();
        for &id in &ids {
            let (packets, new_sent) = {
                let p = &self.players[&id];
                if !p.moved {
                    continue;
                }
                let now = [quantize(p.x), quantize(p.y), quantize(p.z)];
                let d = [now[0] - p.sent.base[0], now[1] - p.sent.base[1], now[2] - p.sent.base[2]];
                let (yr, xr, hr) = (angle(p.yaw), angle(p.pitch), angle(p.yaw));
                let moved = d != [0, 0, 0];
                let turned = yr != p.sent.y_rot || xr != p.sent.x_rot;
                let small = d.iter().all(|v| v.abs() <= 32767);
                let mut out: Vec<(i32, Vec<u8>)> = Vec::new();
                let mut push = |id: i32, body: Vec<u8>| out.push((id, body));
                let eid = VarInt(p.entity_id);
                let enc = |v: &dyn Fn(&mut Vec<u8>) -> ferrite_protocol::error::Result<()>| {
                    let mut b = Vec::new();
                    let _ = v(&mut b);
                    b
                };
                if moved && !small {
                    let t = TeleportEntity {
                        id: eid,
                        x: p.x,
                        y: p.y,
                        z: p.z,
                        dx: 0.0,
                        dy: 0.0,
                        dz: 0.0,
                        y_rot: p.yaw,
                        x_rot: p.pitch,
                        relative: 0,
                        on_ground: p.on_ground,
                    };
                    push(TeleportEntity::ID, enc(&|b| t.encode(b)));
                } else if moved && turned {
                    let m = MoveEntityPosRot {
                        id: eid,
                        on_ground: OnGround(p.on_ground),
                        dx: d[0] as i16,
                        dy: d[1] as i16,
                        dz: d[2] as i16,
                        y_rot: yr,
                        x_rot: xr,
                    };
                    push(MoveEntityPosRot::ID, enc(&|b| m.encode(b)));
                } else if moved {
                    let m = MoveEntityPos {
                        id: eid,
                        on_ground: OnGround(p.on_ground),
                        dx: d[0] as i16,
                        dy: d[1] as i16,
                        dz: d[2] as i16,
                    };
                    push(MoveEntityPos::ID, enc(&|b| m.encode(b)));
                } else if turned {
                    let m = MoveEntityRot { id: eid, y_rot: yr, x_rot: xr, on_ground: p.on_ground };
                    push(MoveEntityRot::ID, enc(&|b| m.encode(b)));
                }
                if hr != p.sent.head {
                    let m = RotateHead { id: eid, y_head_rot: hr };
                    push(RotateHead::ID, enc(&|b| m.encode(b)));
                }
                // The base only advances by what was actually sent as a delta, so rounding never
                // accumulates; a teleport resets it.
                let base = if moved { now } else { p.sent.base };
                (out, SentState { base, y_rot: yr, x_rot: xr, head: hr })
            };
            let p = self.players.get_mut(&id).expect("exists");
            p.sent = new_sent;
            p.moved = false;
            for (&other, o) in &self.players {
                if other != id && o.tracked.contains(&id) {
                    for (pid, body) in &packets {
                        o.sink.send(*pid, Bytes::copy_from_slice(body));
                    }
                }
            }
        }
    }
}

fn abilities_for(game_mode: u8) -> u8 {
    match game_mode {
        1 => 1 | 4 | 8, // invulnerable, may fly, instant build
        3 => 1 | 2 | 4,
        _ => 0,
    }
}
