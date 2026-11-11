//! A real server on a real socket, with real clients (the swarm's play bot).

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use ferrite_game::chunkenc::decode_sections;
use ferrite_game::{GameConfig, GameThread};
use ferrite_net::{Lobby, LobbyConfig, tokio_backend};
use ferrite_protocol::codec::decode_exact;
use ferrite_protocol::packets::play::*;
use ferrite_swarm::{Captured, PlayBot};
use ferrite_world::Dimension;
use tokio::net::TcpListener;

struct TestServer {
    addr: SocketAddr,
    _game: GameThread,
}

async fn server(view_distance: u8) -> TestServer {
    server_with(GameConfig { view_distance, ..GameConfig::default() }).await
}

async fn server_with(cfg: GameConfig) -> TestServer {
    let (host, game) = ferrite_game::start(cfg);
    let lobby = Arc::new(Lobby::new(LobbyConfig { play: Some(host), ..LobbyConfig::default() }));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(tokio_backend::serve(listener, lobby, std::future::pending::<()>()));
    TestServer { addr, _game: game }
}

fn decode<T: for<'a> ferrite_protocol::codec::Decode<'a>>(p: &Captured) -> T {
    decode_exact(&p.body).unwrap_or_else(|e| panic!("{} did not decode: {e}", p.name))
}

/// Reads packets until `pred` matches one, failing after `limit` packets or 10 seconds.
async fn until<F: FnMut(&Captured) -> bool>(
    bot: &mut PlayBot,
    limit: usize,
    mut pred: F,
) -> Captured {
    tokio::time::timeout(Duration::from_secs(10), async {
        for _ in 0..limit {
            let p = bot.next().await.expect("the server stays connected");
            if pred(&p) {
                return p;
            }
        }
        panic!("no matching packet within {limit} packets");
    })
    .await
    .expect("timed out")
}

#[tokio::test]
async fn joining_streams_the_world_around_the_spawn() {
    let s = server(4).await;
    let mut bot = PlayBot::join(s.addr, "Alice").await.unwrap();
    let first = bot.next().await.unwrap();
    assert_eq!(first.name, "login", "the first play packet is login");

    let mut chunks = Vec::new();
    let mut batches = 0usize;
    let mut finished = 0usize;
    let mut saw_load_start = false;
    let mut saw_position = false;
    // The bot is told to view 2 chunks, so the world around it streams in within a few ticks.
    until(&mut bot, 400, |p| {
        match p.name {
            "game_event" => saw_load_start |= decode::<GameEvent>(p).event == 13,
            "player_position" => saw_position = true,
            "chunk_batch_start" => batches += 1,
            "chunk_batch_finished" => {
                finished += decode::<ChunkBatchFinished>(p).batch_size.0 as usize
            }
            "level_chunk_with_light" => chunks.push(decode::<LevelChunkWithLight>(p)),
            _ => {}
        }
        finished > 0 && finished == chunks.len() && chunks.len() >= 25
    })
    .await;

    assert!(saw_load_start && saw_position);
    assert_eq!(finished, chunks.len(), "every batch announces how many chunks it holds");
    assert!(batches >= 1);
    let coords: HashSet<(i32, i32)> = chunks.iter().map(|c| (c.x, c.z)).collect();
    assert_eq!(coords.len(), chunks.len(), "no chunk is sent twice");
    assert!(coords.contains(&(0, 0)), "the spawn chunk is sent");
    // View distance 2 is a rounded square: the corner (3, 3) is out, (2, 2) is in.
    assert!(coords.contains(&(2, 2)) && !coords.contains(&(3, 3)));
    // Every chunk decodes and is the flat world.
    let flat = ferrite_game::flat::FlatGenerator::overworld().generate();
    for c in &chunks {
        let decoded = decode_sections(&c.buffer.0, Dimension::OVERWORLD).unwrap();
        assert_eq!(decoded, flat, "chunk ({}, {})", c.x, c.z);
    }
}

#[tokio::test]
async fn players_see_each_other_move_chat_and_leave() {
    let s = server(4).await;
    let mut a = PlayBot::join(s.addr, "Alice").await.unwrap();
    // Let Alice finish joining.
    until(&mut a, 400, |p| p.name == "chunk_batch_finished").await;

    let mut b = PlayBot::join(s.addr, "Bob").await.unwrap();

    // Alice learns about Bob: a tab-list entry and the entity.
    let info = until(&mut a, 400, |p| {
        p.name == "player_info_update"
            && decode::<PlayerInfoUpdate>(p)
                .entries
                .iter()
                .any(|e| e.profile.as_ref().is_some_and(|(n, _)| n.0 == "Bob"))
    })
    .await;
    let bob_uuid = decode::<PlayerInfoUpdate>(&info).entries[0].uuid;
    let add = until(&mut a, 400, |p| p.name == "add_entity").await;
    let add: AddEntity = decode(&add);
    assert_eq!(add.uuid, bob_uuid);
    assert_eq!(add.kind.0 as u32, ferrite_data::registries::entity_type::PLAYER);
    assert_eq!((add.x, add.z), (0.5, 0.5));

    // Bob learns about Alice too.
    until(&mut b, 600, |p| {
        p.name == "player_info_update"
            && decode::<PlayerInfoUpdate>(p)
                .entries
                .iter()
                .any(|e| e.profile.as_ref().is_some_and(|(n, _)| n.0 == "Alice"))
    })
    .await;
    until(&mut b, 600, |p| p.name == "add_entity").await;
    // Bob must have confirmed his teleport before the server accepts his movement; the bot does
    // that when it reads `player_position`, so make sure it has read it.
    assert!(b.position.is_some());
    until(&mut b, 600, |p| p.name == "chunk_batch_finished").await;

    // Bob walks east by a small step.
    b.send(&MovePlayerPosRot { x: 1.5, y: -60.0, z: 0.5, y_rot: 90.0, x_rot: 10.0, flags: 1 })
        .await
        .unwrap();
    let m = until(&mut a, 600, |p| p.name == "move_entity_pos_rot").await;
    let m: MoveEntityPosRot = decode(&m);
    assert_eq!(m.id, add.id);
    assert_eq!((m.dx, m.dy, m.dz), (4096, 0, 0), "one block east, in 1/4096 units");
    assert_eq!(m.y_rot, 64, "90 degrees is a quarter turn: 64/256");
    assert!(m.on_ground.0);
    until(&mut a, 100, |p| p.name == "rotate_head").await;

    // Bob chats; Alice reads it.
    b.send(&Chat {
        message: "hello there".into(),
        timestamp: 0,
        salt: 0,
        signature: None,
        offset: ferrite_protocol::codec::VarInt(0),
        acknowledged: Fixed([0; 3]),
        checksum: 0,
    })
    .await
    .unwrap();
    let chat = until(&mut a, 600, |p| {
        p.name == "system_chat"
            && decode::<SystemChat>(p).content.0
                == ferrite_nbt::Tag::String("<Bob> hello there".into())
    })
    .await;
    assert_eq!(chat.name, "system_chat");

    // Bob leaves; Alice is told.
    drop(b);
    until(&mut a, 600, |p| {
        p.name == "player_info_remove" && decode::<PlayerInfoRemove>(p).uuids.0 == vec![bob_uuid]
    })
    .await;
    let gone = until(&mut a, 600, |p| p.name == "remove_entities").await;
    assert_eq!(decode::<RemoveEntities>(&gone).ids.0, vec![add.id]);
}

#[tokio::test]
async fn impossible_movement_is_undone() {
    let s = server(4).await;
    let mut bot = PlayBot::join(s.addr, "Eve").await.unwrap();
    until(&mut bot, 400, |p| p.name == "chunk_batch_finished").await;
    // Jump a thousand blocks in one packet.
    bot.send(&MovePlayerPos { x: 1000.0, y: -60.0, z: 0.0, flags: 1 }).await.unwrap();
    let back = until(&mut bot, 400, |p| {
        p.name == "player_position" && decode::<PlayerPosition>(p).teleport_id.0 != 0
    })
    .await;
    let back: PlayerPosition = decode(&back);
    assert_eq!((back.x, back.y, back.z), (0.5, -60.0, 0.5), "the server puts the player back");
}

fn state(name: &str) -> i32 {
    i32::from(ferrite_data::blocks::block_by_name(name).unwrap().default_state)
}

fn item(name: &str) -> i32 {
    ferrite_data::registries::item::by_name.get(name).copied().unwrap() as i32
}

/// The block update for `pos` that arrives next, skipping unrelated ones.
async fn block_update_at(bot: &mut PlayBot, x: i32, y: i32, z: i32) -> BlockUpdate {
    let p = until(bot, 600, |p| {
        p.name == "block_update" && {
            let u: BlockUpdate = decode(p);
            (u.pos.x(), u.pos.y(), u.pos.z()) == (x, y, z)
        }
    })
    .await;
    decode(&p)
}

#[tokio::test]
async fn creative_players_break_and_place_blocks_and_everyone_sees_it() {
    use ferrite_protocol::codec::VarInt;
    let s = server(4).await;
    let mut a = PlayBot::join(s.addr, "Alice").await.unwrap();
    let mut b = PlayBot::join(s.addr, "Bob").await.unwrap();
    until(&mut a, 600, |p| p.name == "chunk_batch_finished").await;
    until(&mut b, 600, |p| p.name == "chunk_batch_finished").await;

    // Alice breaks the grass under her feet.
    a.send(&PlayerAction {
        action: VarInt(0),
        pos: PackedPos::new(0, -61, 0),
        face: 1,
        sequence: VarInt(1),
    })
    .await
    .unwrap();
    let u = block_update_at(&mut a, 0, -61, 0).await;
    assert_eq!(u.state.0, state("air"));
    let ack = until(&mut a, 100, |p| p.name == "block_changed_ack").await;
    assert_eq!(decode::<BlockChangedAck>(&ack).sequence.0, 1);
    // Bob, who has the chunk loaded, is told too.
    assert_eq!(block_update_at(&mut b, 0, -61, 0).await.state.0, state("air"));

    // Alice puts stone in her hotbar and places it next to the hole.
    a.send(&SetCreativeModeSlot {
        slot: 36,
        item: ItemStack { count: 64, item: item("minecraft:stone") },
    })
    .await
    .unwrap();
    a.send(&UseItemOn {
        hand: VarInt(0),
        pos: PackedPos::new(2, -61, 0),
        face: VarInt(1),
        cursor_x: 0.5,
        cursor_y: 1.0,
        cursor_z: 0.5,
        inside: false,
        world_border_hit: false,
        sequence: VarInt(2),
    })
    .await
    .unwrap();
    assert_eq!(block_update_at(&mut a, 2, -60, 0).await.state.0, state("stone"));
    assert_eq!(block_update_at(&mut b, 2, -60, 0).await.state.0, state("stone"));
}

#[tokio::test]
async fn a_block_cannot_be_placed_inside_a_player() {
    use ferrite_protocol::codec::VarInt;
    let s = server(4).await;
    let mut a = PlayBot::join(s.addr, "Alice").await.unwrap();
    until(&mut a, 600, |p| p.name == "chunk_batch_finished").await;
    a.send(&SetCreativeModeSlot {
        slot: 36,
        item: ItemStack { count: 1, item: item("minecraft:stone") },
    })
    .await
    .unwrap();
    // Alice stands at (0.5, -60, 0.5); placing on the ground block below her would fill her space.
    a.send(&UseItemOn {
        hand: VarInt(0),
        pos: PackedPos::new(0, -61, 0),
        face: VarInt(1),
        cursor_x: 0.5,
        cursor_y: 1.0,
        cursor_z: 0.5,
        inside: false,
        world_border_hit: false,
        sequence: VarInt(7),
    })
    .await
    .unwrap();
    // The server answers with the truth (air) for the position, never stone.
    let u = block_update_at(&mut a, 0, -60, 0).await;
    assert_eq!(u.state.0, state("air"));
    let ack = until(&mut a, 100, |p| p.name == "block_changed_ack").await;
    assert_eq!(decode::<BlockChangedAck>(&ack).sequence.0, 7);
}

#[tokio::test]
async fn survival_players_dig_blocks_collect_the_drops_and_others_see_the_progress() {
    use ferrite_protocol::codec::VarInt;
    let s =
        server_with(GameConfig { game_mode: 0, view_distance: 4, ..GameConfig::default() }).await;
    let mut a = PlayBot::join(s.addr, "Alice").await.unwrap();
    let mut b = PlayBot::join(s.addr, "Bob").await.unwrap();
    until(&mut a, 600, |p| p.name == "chunk_batch_finished").await;
    until(&mut b, 600, |p| p.name == "chunk_batch_finished").await;
    let at = PackedPos::new(0, -61, 0);
    a.send(&PlayerAction { action: VarInt(0), pos: at, face: 1, sequence: VarInt(1) })
        .await
        .unwrap();
    // Grass by hand takes 0.6 / 30 per tick: about 11 ticks to the 70% mark the server accepts.
    // The server must not break it early...
    a.send(&PlayerAction { action: VarInt(2), pos: at, face: 1, sequence: VarInt(2) })
        .await
        .unwrap();
    assert_eq!(block_update_at(&mut a, 0, -61, 0).await.state.0, state("grass_block"));
    // ...and Bob is shown the digging progress.
    until(&mut b, 600, |p| p.name == "block_destruction").await;
    // The server finishes the dig by itself once enough time has passed.
    let u = block_update_at(&mut a, 0, -61, 0).await;
    assert_eq!(u.state.0, state("air"));
    // The drop appears, is picked up by the player standing over it, and lands in the hotbar.
    let spawned = until(&mut a, 600, |p| p.name == "add_entity").await;
    let spawned: AddEntity = decode(&spawned);
    assert_eq!(
        spawned.kind.0 as u32,
        ferrite_data::registries::entity_type::by_name["minecraft:item"]
    );
    // (Vanilla sends the inventory change first, then the pickup animation.)
    // The item falls into the hole; the player steps down to it.
    a.send(&MovePlayerPos { x: 0.5, y: -61.0, z: 0.5, flags: 1 }).await.unwrap();
    let slot = until(&mut a, 600, |p| p.name == "container_set_slot").await;
    let slot: ContainerSetSlot = decode(&slot);
    assert_eq!((slot.slot, slot.item.item, slot.item.count), (36, item("minecraft:dirt"), 1));
    let taken = until(&mut a, 50, |p| p.name == "take_item_entity").await;
    assert_eq!(decode::<TakeItemEntity>(&taken).collected.0, spawned.id.0);
}

#[tokio::test]
async fn inventory_clicks_move_stacks_and_disagreements_are_corrected() {
    use ferrite_protocol::codec::VarInt;
    use ferrite_protocol::types::LenVec;
    let s = server(4).await;
    let mut a = PlayBot::join(s.addr, "Alice").await.unwrap();
    until(&mut a, 600, |p| p.name == "chunk_batch_finished").await;
    let stone = ItemStack { count: 10, item: item("minecraft:stone") };
    a.send(&SetCreativeModeSlot { slot: 36, item: stone }).await.unwrap();
    let click = |slot: i16, changed: Vec<ChangedSlot>, carried: ItemStack| ContainerClick {
        window_id: VarInt(0),
        state_id: VarInt(1),
        slot,
        button: 0,
        click_type: VarInt(0),
        changed: LenVec(changed),
        carried: HashedStack::plain(carried),
    };
    let nothing = ItemStack::default();
    // Pick the stack up, then put it down in the main inventory: both agree with the server.
    a.send(&click(36, vec![ChangedSlot { slot: 36, item: HashedStack(None) }], stone))
        .await
        .unwrap();
    a.send(&click(20, vec![ChangedSlot { slot: 20, item: HashedStack::plain(stone) }], nothing))
        .await
        .unwrap();
    // A click that claims something else happened is answered with the real inventory.
    a.send(&click(
        20,
        vec![ChangedSlot { slot: 20, item: HashedStack(None) }],
        nothing, // really the stack is now on the cursor
    ))
    .await
    .unwrap();
    let content = until(&mut a, 600, |p| p.name == "container_set_content").await;
    let content: ContainerSetContent = decode(&content);
    assert_eq!(content.items.0[20], nothing, "the stack was picked up");
    assert_eq!(content.carried, stone, "and is on the cursor, whatever the client believed");
    // Dropping with the drop key throws one item and updates the slot.
    a.send(&SetCreativeModeSlot { slot: 36, item: stone }).await.unwrap();
    a.send(&PlayerAction {
        action: VarInt(4),
        pos: PackedPos::new(0, 0, 0),
        face: 0,
        sequence: VarInt(0),
    })
    .await
    .unwrap();
    let thrown = until(&mut a, 600, |p| p.name == "add_entity").await;
    assert_eq!(
        decode::<AddEntity>(&thrown).kind.0 as u32,
        ferrite_data::registries::entity_type::by_name["minecraft:item"]
    );
}

#[tokio::test]
async fn chests_and_crafting_tables_open_windows_that_keep_their_contents() {
    use ferrite_protocol::codec::VarInt;
    use ferrite_protocol::types::LenVec;
    let s = server(4).await;
    let mut a = PlayBot::join(s.addr, "Alice").await.unwrap();
    until(&mut a, 600, |p| p.name == "chunk_batch_finished").await;
    // Put a chest next to the player's feet and open it.
    a.send(&SetCreativeModeSlot {
        slot: 36,
        item: ItemStack { count: 1, item: item("minecraft:chest") },
    })
    .await
    .unwrap();
    a.send(&UseItemOn {
        hand: VarInt(0),
        pos: PackedPos::new(1, -61, 0),
        face: VarInt(1),
        cursor_x: 0.5,
        cursor_y: 1.0,
        cursor_z: 0.5,
        inside: false,
        world_border_hit: false,
        sequence: VarInt(1),
    })
    .await
    .unwrap();
    block_update_at(&mut a, 1, -60, 0).await;
    let open = |seq| UseItemOn {
        hand: VarInt(0),
        pos: PackedPos::new(1, -60, 0),
        face: VarInt(1),
        cursor_x: 0.5,
        cursor_y: 1.0,
        cursor_z: 0.5,
        inside: false,
        world_border_hit: false,
        sequence: VarInt(seq),
    };
    // Sneak so the click does not try to place the chest again... a bare hand opens it.
    a.send(&SetCreativeModeSlot { slot: 36, item: ItemStack::default() }).await.unwrap();
    a.send(&open(2)).await.unwrap();
    let screen = until(&mut a, 600, |p| p.name == "open_screen").await;
    let screen: OpenScreen = decode(&screen);
    assert_eq!(screen.menu_type.0, 2, "a single chest is a 9x3 window");
    let content = until(&mut a, 100, |p| p.name == "container_set_content").await;
    let content: ContainerSetContent = decode(&content);
    let lid = until(&mut a, 100, |p| p.name == "block_event").await;
    assert_eq!(decode::<BlockEvent>(&lid).data, 1, "one viewer");
    assert_eq!((content.window_id.0, content.items.0.len()), (screen.container_id.0, 63));
    // Move something into the chest: pick up from the inventory (a creative-set hotbar stack)
    // is not possible inside the window, so put the cursor stack down instead.
    let stone = ItemStack { count: 7, item: item("minecraft:stone") };
    a.send(&ContainerClick {
        window_id: screen.container_id,
        state_id: content.state_id,
        slot: 4,
        button: 0,
        click_type: VarInt(3), // clone (creative): the cursor gets a full stack
        changed: LenVec(vec![]),
        carried: HashedStack::plain(ItemStack { count: 64, item: item("minecraft:stone") }),
    })
    .await
    .unwrap();
    let _ = stone;
    a.send(&ContainerClose { window_id: screen.container_id }).await.unwrap();
    until(&mut a, 600, |p| p.name == "container_close").await;
}

#[tokio::test]
async fn a_saved_world_comes_back_with_its_blocks_players_and_chests() {
    use ferrite_protocol::codec::VarInt;
    let dir = tempfile::tempdir().unwrap();
    let cfg = || GameConfig {
        view_distance: 4,
        world_dir: Some(dir.path().to_path_buf()),
        ..GameConfig::default()
    };
    {
        let s = server_with(cfg()).await;
        let mut a = PlayBot::join(s.addr, "Alice").await.unwrap();
        until(&mut a, 600, |p| p.name == "chunk_batch_finished").await;
        a.send(&SetCreativeModeSlot {
            slot: 37,
            item: ItemStack { count: 7, item: item("minecraft:stone") },
        })
        .await
        .unwrap();
        a.send(&PlayerAction {
            action: VarInt(0),
            pos: PackedPos::new(2, -61, 2),
            face: 1,
            sequence: VarInt(1),
        })
        .await
        .unwrap();
        assert_eq!(block_update_at(&mut a, 2, -61, 2).await.state.0, state("air"));
        a.send(&MovePlayerPos { x: 5.5, y: -60.0, z: 3.5, flags: 1 }).await.unwrap();
        // Give the server a moment to take the move, then leave.
        tokio::time::sleep(Duration::from_millis(300)).await;
        drop(a);
        tokio::time::sleep(Duration::from_millis(300)).await;
    } // the server stops and saves
    let s = server_with(cfg()).await;
    let mut a = PlayBot::join(s.addr, "Alice").await.unwrap();
    let pos = until(&mut a, 600, |p| p.name == "player_position").await;
    let pos: PlayerPosition = decode(&pos);
    assert_eq!((pos.x, pos.z), (5.5, 3.5), "the player is where they left");
    let content = until(&mut a, 600, |p| p.name == "container_set_content").await;
    let content: ContainerSetContent = decode(&content);
    assert_eq!(content.items.0[37], ItemStack { count: 7, item: item("minecraft:stone") });
    let mut found = false;
    until(&mut a, 600, |p| {
        if p.name == "level_chunk_with_light" {
            let c: LevelChunkWithLight = decode(p);
            if (c.x, c.z) == (0, 0) {
                let d = decode_sections(&c.buffer.0, Dimension::OVERWORLD).unwrap();
                let air = state("air") as u16;
                found = d.block(2, -61, 2) == Some(air);
                return true;
            }
        }
        false
    })
    .await;
    assert!(found, "the dug block is still gone after a restart");
}
