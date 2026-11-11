//! Decodes packets captured from the official server (`ferrite-capture`) with Ferrite's types,
//! requiring every byte to be consumed and the re-encoding to be identical.

use ferrite_protocol::codec::{Encode, decode_exact};
use ferrite_protocol::packets::play::*;

fn captures() -> Vec<(String, Vec<u8>)> {
    let text = [include_str!("captures/join.txt"), include_str!("captures/items.txt")].join("\n");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let mut parts = l.splitn(3, ' ');
            let _id = parts.next().unwrap();
            let name = parts.next().unwrap().to_string();
            let hex = parts.next().unwrap_or("");
            let bytes = (0..hex.len() / 2)
                .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
                .collect();
            (name, bytes)
        })
        .collect()
}

fn round_trip<T>(name: &str, body: &[u8]) -> Result<(), String>
where
    T: for<'a> ferrite_protocol::codec::Decode<'a> + Encode,
{
    let value: T = decode_exact(body).map_err(|e| format!("{name}: decode failed: {e}"))?;
    let mut out = Vec::new();
    value.encode(&mut out).map_err(|e| format!("{name}: encode failed: {e}"))?;
    if out != body {
        return Err(format!("{name}: re-encoding differs ({} vs {} bytes)", out.len(), body.len()));
    }
    Ok(())
}

macro_rules! verify {
    ($caps:expr, $verified:expr, $($name:literal => $ty:ty),* $(,)?) => {{
        let mut errors = Vec::new();
        $(
            let mut seen = 0;
            for (n, body) in $caps.iter().filter(|(n, _)| n == $name) {
                seen += 1;
                if let Err(e) = round_trip::<$ty>(n, body) {
                    errors.push(e);
                }
            }
            assert!(seen > 0, "no capture of {}", $name);
            $verified.push($name);
        )*
        errors
    }};
}

#[test]
fn every_captured_packet_with_a_type_round_trips() {
    let caps = captures();
    assert!(caps.len() > 50, "captures missing");
    let mut verified: Vec<&str> = Vec::new();
    let errors = verify!(caps, verified,
        "login" => Login,
        "change_difficulty" => ChangeDifficulty,
        "player_abilities" => PlayerAbilities,
        "set_held_slot" => SetHeldSlot,
        "player_position" => PlayerPosition,
        "game_event" => GameEvent,
        "set_chunk_cache_center" => SetChunkCacheCenter,
        "chunk_batch_start" => ChunkBatchStart,
        "chunk_batch_finished" => ChunkBatchFinished,
        "bundle_delimiter" => BundleDelimiter,
        "set_default_spawn_position" => SetDefaultSpawnPosition,
        "set_time" => SetTime,
        "initialize_border" => InitializeBorder,
        "ticking_state" => TickingState,
        "ticking_step" => TickingStep,
        "server_data" => ServerData,
        "set_health" => SetHealth,
        "set_experience" => SetExperience,
        "post_effects" => PostEffects,
        "level_chunk_with_light" => LevelChunkWithLight,
        "add_entity" => AddEntity,
        "move_entity_pos" => MoveEntityPos,
        "player_info_update" => PlayerInfoUpdate,
        "set_entity_data" => SetEntityData,
        "container_set_content" => ContainerSetContent,
        "container_set_slot" => ContainerSetSlot,
        "take_item_entity" => TakeItemEntity,
        "set_entity_motion" => SetEntityMotion,
    );
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    eprintln!("verified against vanilla: {}", verified.join(", "));
}

#[test]
fn packed_positions_round_trip() {
    for (x, y, z) in [
        (0, 0, 0),
        (-1, -64, -1),
        (123, 319, -456),
        (33_554_431, 2047, -33_554_432),
        (-33_554_432, -2048, 33_554_431),
    ] {
        let p = PackedPos::new(x, y, z);
        assert_eq!((p.x(), p.y(), p.z()), (x, y, z));
    }
    // The spawn position captured from vanilla: (0, -60, 0).
    assert_eq!(PackedPos::new(0, -60, 0).0, 0x0000_0000_0000_0FC4);
}

#[test]
fn compact_velocities_survive_a_round_trip() {
    use ferrite_protocol::codec::Decode;
    for v in [
        LpVec3::ZERO,
        LpVec3 { x: 0.0, y: -0.08, z: 0.0 },
        LpVec3 { x: 0.5, y: 0.25, z: -0.75 },
        LpVec3 { x: 3.9, y: -2.0, z: 1.0 },
        LpVec3 { x: 100.0, y: -250.5, z: 7.0 },
        LpVec3 { x: -1.7e10, y: 0.0, z: 1.7e10 },
    ] {
        let mut bytes = Vec::new();
        v.encode(&mut bytes).unwrap();
        let mut slice = &bytes[..];
        let back = LpVec3::decode(&mut slice).unwrap();
        assert!(slice.is_empty());
        // Quantised to 15 bits per component relative to the scale.
        let scale = v.x.abs().max(v.y.abs()).max(v.z.abs()).ceil().max(1.0);
        for (a, b) in [(v.x, back.x), (v.y, back.y), (v.z, back.z)] {
            assert!((a - b).abs() <= scale / 16000.0 + 1e-9, "{v:?} -> {back:?}");
        }
        let mut again = Vec::new();
        back.encode(&mut again).unwrap();
        assert_eq!(again, bytes, "encoding is idempotent for {v:?}");
    }
}

#[test]
fn player_info_entries_need_the_fields_their_actions_ask_for() {
    use ferrite_protocol::packet::Packet;
    let _ = PlayerInfoUpdate::ID;
    let bad = PlayerInfoUpdate {
        actions: player_action::ADD_PLAYER,
        entries: vec![PlayerInfoEntry::default()],
    };
    assert!(bad.encode(&mut Vec::new()).is_err());
}
