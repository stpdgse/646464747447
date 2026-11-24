//! Drives a whole server session with attacker-controlled bytes, fed in arbitrary chunks, while
//! the "application" cooperates (accepts logins, answers status). Whatever the bytes are, the
//! session must never panic, and a closed session must stay closed and silent.
#![no_main]

use bytes::BytesMut;
use ferrite_protocol::codec::VarInt;
use ferrite_protocol::packets::handshake::Intention;
use ferrite_protocol::packets::login::ServerboundHello;
use ferrite_protocol::session::{Event, Phase, ServerSession, SessionConfig};
use ferrite_protocol::types::{GameProfile, LenVec, Str};
use ferrite_protocol::{Compression, FrameEncoder, Packet};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&config, rest)) = data.split_first() else { return };
    let Some((&chunk, rest)) = rest.split_first() else { return };

    let cfg = SessionConfig {
        compression: (config & 1 != 0).then(|| Compression { threshold: usize::from(config >> 3) }),
        ..SessionConfig::default()
    };
    let mut session = ServerSession::new(cfg);
    if config & 2 != 0 {
        session.enable_encryption(&[config; 16]);
    }

    // Bits 2 and 4 start the stream with a *valid* handshake (login or status) and, for login,
    // a hello, so the fuzzer spends its time in the deeper states instead of the first check.
    let mut prefix = BytesMut::new();
    if config & 4 != 0 {
        let mut enc = FrameEncoder::new();
        let frame = |enc: &mut FrameEncoder, out: &mut BytesMut, p: &dyn Fn(&mut Vec<u8>), id: i32| {
            let mut body = Vec::new();
            p(&mut body);
            enc.encode(id, &body, out).unwrap();
        };
        let intent = if config & 16 != 0 { 1 } else { 2 };
        let handshake = Intention {
            protocol_version: VarInt(777),
            server_address: Str::new("h"),
            server_port: 1,
            intent: VarInt(intent),
        };
        frame(&mut enc, &mut prefix, &|b| ferrite_protocol::Encode::encode(&handshake, b).unwrap(), Intention::ID);
        if intent == 2 {
            let hello = ServerboundHello { name: Str::new("Fuzz"), uuid: uuid::Uuid::nil() };
            frame(&mut enc, &mut prefix, &|b| ferrite_protocol::Encode::encode(&hello, b).unwrap(), ServerboundHello::ID);
        }
    }
    let mut stream = prefix.to_vec();
    stream.extend_from_slice(rest);

    for part in stream.chunks(usize::from(chunk).max(1)) {
        session.feed(part);
        let events = match session.poll_all() {
            Ok(e) => e,
            Err(_) => {
                assert_eq!(session.phase(), Phase::Closed);
                assert!(session.poll().unwrap().is_none());
                return;
            }
        };
        for event in events {
            match event {
                Event::StatusRequested => {
                    let _ = session.send_status("{}");
                }
                Event::LoginStarted { name, uuid, .. } => {
                    let profile = GameProfile {
                        uuid,
                        name: Str::new(name),
                        properties: LenVec(vec![]),
                    };
                    let _ = session.accept_login(profile);
                }
                _ => {}
            }
        }
        // Keep the output buffer from growing without bound inside the fuzzer.
        let _ = session.take_output();
        if matches!(session.phase(), Phase::Play | Phase::Closed) {
            break;
        }
    }
});
