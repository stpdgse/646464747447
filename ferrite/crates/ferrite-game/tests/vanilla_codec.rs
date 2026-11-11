//! Differential tests against the official packet codecs (they need a JDK and the server jar and
//! are ignored by default; see `scripts/vanilla-codec.sh`).
//!
//! Ferrite encodes packets; vanilla's own `StreamCodec`s decode them. A packet that fails to decode
//! or leaves bytes unread is a layout bug that no amount of self-consistency testing would find.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::Command;

use ferrite_protocol::codec::{Encode, VarInt};
use ferrite_protocol::packet::Packet;
use ferrite_protocol::packets::play::*;
use ferrite_protocol::types::LenVec;

fn line<P: Packet + Encode>(p: &P) -> String {
    let mut body = Vec::new();
    p.encode(&mut body).unwrap();
    let mut hex = String::new();
    for b in &body {
        let _ = write!(hex, "{b:02x}");
    }
    format!("{} {} {}\n", P::ID, P::NAME, hex)
}

/// One representative of every serverbound play packet Ferrite understands.
fn serverbound_samples() -> String {
    let mut s = String::new();
    s += &line(&AcceptTeleportation {
        teleport_id: VarInt(7),
        x: 1.5,
        y: -60.0,
        z: -2.25,
        y_rot: 90.0,
        x_rot: -10.0,
    });
    s += &line(&ServerboundKeepAlive { id: 123_456_789 });
    s += &line(&ChunkBatchReceived { chunks_per_tick: 12.5 });
    s += &line(&PlayerLoaded {});
    s += &line(&ClientTickEnd {});
    s += &line(&MovePlayerPos { x: 1.0, y: 70.5, z: -3.0, flags: 1 });
    s += &line(&MovePlayerPosRot { x: 1.0, y: 70.5, z: -3.0, y_rot: 45.0, x_rot: 30.0, flags: 3 });
    s += &line(&MovePlayerRot { y_rot: -90.0, x_rot: 5.0, flags: 0 });
    s += &line(&MovePlayerStatusOnly { flags: 1 });
    s += &line(&PlayerAction {
        action: VarInt(2),
        pos: PackedPos::new(-5, 64, 300),
        face: 1,
        sequence: VarInt(9),
    });
    s += &line(&UseItemOn {
        hand: VarInt(0),
        pos: PackedPos::new(1, 2, 3),
        face: VarInt(1),
        cursor_x: 0.5,
        cursor_y: 1.0,
        cursor_z: 0.25,
        inside: false,
        world_border_hit: false,
        sequence: VarInt(4),
    });
    s += &line(&UseItem { hand: VarInt(1), sequence: VarInt(5), y_rot: 10.0, x_rot: 20.0 });
    s += &line(&SetCarriedItem { slot: 3 });
    s += &line(&SetCreativeModeSlot { slot: 36, item: ItemStack { count: 64, item: 1 } });
    s += &line(&SetCreativeModeSlot { slot: 37, item: ItemStack::default() });
    s += &line(&ContainerClick {
        window_id: VarInt(0),
        state_id: VarInt(4),
        slot: 36,
        button: 0,
        click_type: VarInt(0),
        changed: LenVec(vec![ChangedSlot { slot: 36, item: HashedStack(None) }]),
        carried: HashedStack::plain(ItemStack { count: 5, item: 1 }),
    });
    s += &line(&ContainerClick {
        window_id: VarInt(0),
        state_id: VarInt(5),
        slot: 9,
        button: 1,
        click_type: VarInt(1),
        changed: LenVec(vec![]),
        carried: HashedStack(None),
    });
    s += &line(&ContainerClose { window_id: VarInt(0) });
    s += &line(&Punch {});
    s += &line(&PlayerCommand { entity_id: VarInt(5), action: VarInt(1), data: VarInt(0) });
    s += &line(&PlayerInput { flags: 0b0100_0001 });
    s += &line(&ChatCommand { command: "gamemode creative".into() });
    s += &line(&Chat {
        message: "hello world".into(),
        timestamp: 1_700_000_000_000,
        salt: -42,
        signature: None,
        offset: VarInt(2),
        acknowledged: Fixed([0x03, 0x00, 0x00]),
        checksum: 17,
    });
    s += &line(&PlayClientInformation {
        locale: "en_US".into(),
        view_distance: 12,
        chat_mode: VarInt(0),
        chat_colors: true,
        displayed_skin_parts: 0x7f,
        main_hand: VarInt(1),
        enable_text_filtering: false,
        allow_server_listings: true,
        particle_status: VarInt(0),
    });
    s += &line(&ServerboundCustomPayload {
        channel: ferrite_protocol::types::Identifier::new("minecraft:brand").unwrap(),
        data: ferrite_protocol::types::Remaining(b"\x07vanilla".to_vec()),
    });
    s
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// Compiles and runs the Java harness; returns its `PACKET` lines.
fn run_harness(mode: &str, samples: &str) -> Vec<String> {
    let Some(classpath) = env("FERRITE_VANILLA_CLASSPATH") else {
        panic!(
            "set FERRITE_VANILLA_CLASSPATH to the server jar and its libraries (see scripts/vanilla-codec.sh)"
        );
    };
    let java = env("FERRITE_JAVA").unwrap_or_else(|| "java".into());
    let javac = env("FERRITE_JAVAC").unwrap_or_else(|| "javac".into());
    let dir = std::env::temp_dir().join(format!("ferrite-codec-{}-{mode}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let samples_path = dir.join("samples.txt");
    std::fs::write(&samples_path, samples).unwrap();
    let source: PathBuf =
        [env!("CARGO_MANIFEST_DIR"), "java", "DecodePackets.java"].iter().collect();
    let status = Command::new(&javac)
        .args(["-nowarn", "-cp", &classpath, "-d"])
        .arg(&dir)
        .arg(&source)
        .status()
        .expect("javac");
    assert!(status.success(), "javac failed");
    let out = Command::new(&java)
        .args(["-cp", &format!("{}:{classpath}", dir.display()), "DecodePackets", mode])
        .arg(&samples_path)
        .output()
        .expect("java");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<String> =
        stdout.lines().filter_map(|l| l.find("PACKET ").map(|i| l[i..].to_string())).collect();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        !lines.is_empty(),
        "the harness printed nothing:\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    lines
}

#[test]
#[ignore = "needs a JDK and the vanilla server jar (scripts/vanilla-codec.sh)"]
fn vanilla_decodes_every_serverbound_packet_ferrite_encodes() {
    let samples = serverbound_samples();
    let lines = run_harness("serverbound", &samples);
    let expected = samples.lines().count();
    for l in &lines {
        eprintln!("{l}");
    }
    let bad: Vec<_> =
        lines.iter().filter(|l| l.contains("ERROR") || l.contains("TRAILING")).collect();
    assert!(
        bad.is_empty(),
        "vanilla rejected:\n{}",
        bad.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
    );
    assert_eq!(lines.len(), expected, "every sample produced a line");
}

#[tokio::test]
#[ignore = "needs a JDK and the vanilla server jar (scripts/vanilla-codec.sh)"]
async fn vanilla_decodes_everything_the_ferrite_server_sends() {
    use std::sync::Arc;
    use std::time::Duration;

    use ferrite_game::GameConfig;
    use ferrite_net::{Lobby, LobbyConfig, tokio_backend};
    use ferrite_swarm::PlayBot;

    let (host, _game) =
        ferrite_game::start(GameConfig { view_distance: 6, ..GameConfig::default() });
    let lobby = Arc::new(Lobby::new(LobbyConfig { play: Some(host), ..LobbyConfig::default() }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(tokio_backend::serve(listener, lobby, std::future::pending::<()>()));

    // Alice records a session in which Bob joins, walks, turns, chats and leaves.
    let mut alice = PlayBot::join(addr, "Alice").await.unwrap();
    let mut captured = Vec::new();
    let mut bob = PlayBot::join(addr, "Bob").await.unwrap();
    // Bob reads until his world has arrived, then acts.
    for _ in 0..300 {
        if bob.next().await.unwrap().name == "chunk_batch_finished" {
            break;
        }
    }
    // One move per game tick, or the server (rightly) coalesces them.
    let tick = Duration::from_millis(120);
    bob.send(&MovePlayerPosRot { x: 2.0, y: -60.0, z: 0.5, y_rot: 33.0, x_rot: 12.0, flags: 1 })
        .await
        .unwrap();
    tokio::time::sleep(tick).await;
    bob.send(&MovePlayerRot { y_rot: 200.0, x_rot: -20.0, flags: 1 }).await.unwrap();
    tokio::time::sleep(tick).await;
    bob.send(&MovePlayerPos { x: 2.5, y: -60.0, z: 0.5, flags: 1 }).await.unwrap();
    tokio::time::sleep(tick).await;
    bob.send(&MovePlayerPos { x: 20.0, y: -60.0, z: 0.5, flags: 1 }).await.unwrap(); // more than 8 blocks: a teleport
    tokio::time::sleep(tick).await;
    bob.send(&Chat {
        message: "hi".into(),
        timestamp: 0,
        salt: 0,
        signature: None,
        offset: VarInt(0),
        acknowledged: Fixed([0; 3]),
        checksum: 0,
    })
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;
    drop(bob);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(300), alice.next()).await {
            Ok(Ok(p)) => captured.push(p),
            _ => break,
        }
    }
    let mut samples = String::new();
    for p in &captured {
        let mut hex = String::new();
        for b in &p.body {
            let _ = write!(hex, "{b:02x}");
        }
        let _ = writeln!(samples, "{} {} {}", p.id, p.name, hex);
    }
    let names: std::collections::BTreeSet<_> = captured.iter().map(|p| p.name).collect();
    for needed in [
        "login",
        "level_chunk_with_light",
        "player_info_update",
        "add_entity",
        "move_entity_pos_rot",
        "move_entity_rot",
        "move_entity_pos",
        "rotate_head",
        "teleport_entity",
        "system_chat",
        "player_info_remove",
        "remove_entities",
    ] {
        assert!(names.contains(needed), "the session never produced {needed}: {names:?}");
    }
    let lines =
        tokio::task::spawn_blocking(move || run_harness("clientbound", &samples)).await.unwrap();
    let bad: Vec<_> =
        lines.iter().filter(|l| l.contains("ERROR") || l.contains("TRAILING")).collect();
    assert!(
        bad.is_empty(),
        "vanilla rejected:\n{}",
        bad.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
    );
    eprintln!("vanilla decoded {} packets of {} kinds", lines.len(), names.len());
}
