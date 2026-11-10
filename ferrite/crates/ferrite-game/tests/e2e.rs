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
    let (host, game) = ferrite_game::start(GameConfig { view_distance, ..GameConfig::default() });
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
