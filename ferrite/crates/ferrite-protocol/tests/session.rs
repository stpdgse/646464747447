use bytes::BytesMut;
use ferrite_data::{Direction, State, packet_name};
use ferrite_protocol::cipher::Encryptor;
use ferrite_protocol::codec::VarInt;
use ferrite_protocol::packets::configuration::*;
use ferrite_protocol::packets::handshake::{Intent, Intention};
use ferrite_protocol::packets::login::{
    self, ClientboundLogin, LoginAcknowledged, ServerboundHello,
};
use ferrite_protocol::packets::status::{self, ClientboundStatus, PingRequest, StatusRequest};
use ferrite_protocol::session::{
    Event, Phase, ServerSession, SessionConfig, core_pack, offline_uuid,
};
use ferrite_protocol::types::{GameProfile, LenVec, Str};
use ferrite_protocol::{Compression, Frame, FrameDecoder, FrameEncoder, Packet, SessionError};
use uuid::Uuid;

/// A client built from the same codec the vanilla oracle already validates.
struct Client {
    enc: FrameEncoder,
    dec: FrameDecoder,
    cipher: Option<Encryptor>,
}

impl Client {
    fn new() -> Self {
        Client {
            enc: FrameEncoder::new(),
            dec: FrameDecoder::new().with_limits(1 << 26, 1 << 26),
            cipher: None,
        }
    }

    fn wire<P: Packet>(&mut self, p: &P) -> BytesMut {
        let mut body = Vec::new();
        p.encode(&mut body).unwrap();
        let mut out = BytesMut::new();
        self.enc.encode(P::ID, &body, &mut out).unwrap();
        if let Some(c) = &mut self.cipher {
            c.apply(&mut out);
        }
        out
    }

    fn send<P: Packet>(&mut self, s: &mut ServerSession, p: &P) {
        let bytes = self.wire(p);
        s.feed(&bytes);
    }

    /// Everything the server wrote since last call, decoded into frames.
    fn frames(&mut self, s: &mut ServerSession) -> Vec<Frame> {
        let out = s.take_output();
        self.dec.queue(&out);
        let mut v = Vec::new();
        while let Some(f) = self.dec.next_frame().unwrap() {
            v.push(f);
        }
        v
    }

    fn compression(&mut self, threshold: usize) {
        let c = Some(Compression { threshold });
        self.enc.set_compression(c, 4).unwrap();
        self.dec.set_compression(c);
    }
}

fn names(state: State, frames: &[Frame]) -> Vec<&'static str> {
    frames.iter().map(|f| packet_name(state, Direction::Clientbound, f.id).unwrap()).collect()
}

fn handshake(c: &mut Client, s: &mut ServerSession, intent: i32) {
    c.send(
        s,
        &Intention {
            protocol_version: VarInt(ferrite_data::PROTOCOL_VERSION),
            server_address: Str::new("localhost"),
            server_port: 25565,
            intent: VarInt(intent),
        },
    );
}

fn profile(name: &str) -> GameProfile {
    GameProfile { uuid: offline_uuid(name), name: Str::new(name), properties: LenVec(vec![]) }
}

/// Runs login up to and including the client's acknowledgement, with the given compression.
fn login(c: &mut Client, s: &mut ServerSession, name: &str) {
    handshake(c, s, 2);
    c.send(s, &ServerboundHello { name: Str::new(name), uuid: Uuid::nil() });
    let events = s.poll_all().unwrap();
    assert!(matches!(
        &events[..],
        [Event::Handshake { intent: Intent::Login, .. }, Event::LoginStarted { .. }]
    ));
    s.accept_login(profile(name)).unwrap();
    let out = s.take_output();
    c.dec.queue(&out);
    let mut seen = Vec::new();
    while let Some(f) = c.dec.next_frame().unwrap() {
        match ClientboundLogin::decode(f.id, &f.body).unwrap() {
            ClientboundLogin::LoginCompression(p) => {
                c.compression(p.threshold.0 as usize);
                seen.push("login_compression");
            }
            ClientboundLogin::LoginFinished(p) => {
                assert_eq!(p.profile.name.0, name);
                seen.push("login_finished");
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(seen, ["login_compression", "login_finished"]);
    c.send(s, &LoginAcknowledged {});
}

#[test]
fn offline_uuid_matches_what_vanilla_assigned() {
    // Taken from a real vanilla 26.3 server in offline mode (tests/oracle.rs).
    assert_eq!(offline_uuid("FerriteBot").to_string(), "d0903b53-c1c7-39a3-a164-4b84a25b9430");
    assert_eq!(offline_uuid("FerriteBot").get_version_num(), 3);
}

#[test]
fn status_flow() {
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    handshake(&mut c, &mut s, 1);
    c.send(&mut s, &StatusRequest {});
    c.send(&mut s, &PingRequest { timestamp: 42 });
    let events = s.poll_all().unwrap();
    assert!(matches!(
        &events[..],
        [
            Event::Handshake { intent: Intent::Status, protocol_version: 777, .. },
            Event::StatusRequested
        ]
    ));
    assert_eq!(s.phase(), Phase::Status);

    s.send_status("{\"description\":\"hi\"}").unwrap();
    let frames = c.frames(&mut s);
    // The pong was produced while processing, before the application answered the request.
    assert_eq!(names(State::Status, &frames), ["pong_response", "status_response"]);
    assert_eq!(
        ClientboundStatus::decode(frames[1].id, &frames[1].body).unwrap(),
        status::StatusResponse { json: Str::new("{\"description\":\"hi\"}") }.into()
    );
}

#[test]
fn a_second_status_request_closes_the_session() {
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    handshake(&mut c, &mut s, 1);
    c.send(&mut s, &StatusRequest {});
    c.send(&mut s, &StatusRequest {});
    assert!(matches!(s.poll_all(), Err(SessionError::Unexpected { packet: "status_request", .. })));
    assert_eq!(s.phase(), Phase::Closed);
    assert_eq!(s.poll(), Ok(None), "a closed session produces nothing");
}

#[test]
fn full_offline_join_reproduces_the_vanilla_packet_sequence() {
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    login(&mut c, &mut s, "FerriteBot");

    let events = s.poll_all().unwrap();
    assert_eq!(events, [Event::Configuring]);
    assert_eq!(s.phase(), Phase::Configuration);

    // The opening, exactly as the vanilla server sends it.
    let opening = c.frames(&mut s);
    assert_eq!(
        names(State::Configuration, &opening),
        ["custom_payload", "update_enabled_features", "select_known_packs"]
    );
    let ClientboundConfiguration::CustomPayload(brand) =
        ClientboundConfiguration::decode(opening[0].id, &opening[0].body).unwrap()
    else {
        panic!()
    };
    assert_eq!(brand.channel.as_str(), "minecraft:brand");
    assert_eq!(brand.data.0, [7, b'v', b'a', b'n', b'i', b'l', b'l', b'a']);
    let ClientboundConfiguration::SelectKnownPacks(packs) =
        ClientboundConfiguration::decode(opening[2].id, &opening[2].body).unwrap()
    else {
        panic!()
    };
    assert_eq!(packs.packs.0, [core_pack()]);

    // The client answers; the server sends all registries, the tags, and finishes.
    c.send(&mut s, &ServerboundSelectKnownPacks { packs: LenVec(vec![core_pack()]) });
    assert_eq!(s.poll_all().unwrap(), []);
    let rest = c.frames(&mut s);
    let mut expect =
        vec!["registry_data"; ferrite_data::configuration::SYNCHRONIZED_REGISTRIES.len()];
    expect.extend(["update_tags", "finish_configuration"]);
    assert_eq!(names(State::Configuration, &rest), expect);

    // The data matches the tables that were verified against vanilla.
    for (frame, reg) in rest.iter().zip(ferrite_data::configuration::SYNCHRONIZED_REGISTRIES.iter())
    {
        let ClientboundConfiguration::RegistryData(p) =
            ClientboundConfiguration::decode(frame.id, &frame.body).unwrap()
        else {
            panic!()
        };
        assert_eq!(p.registry.as_str(), reg.id);
        assert!(p.entries.0.iter().map(|e| e.id.as_str()).eq(reg.entries.iter().copied()));
        assert!(p.entries.0.iter().all(|e| e.data.is_none()));
    }
    let ClientboundConfiguration::UpdateTags(tags) =
        ClientboundConfiguration::decode(rest[rest.len() - 2].id, &rest[rest.len() - 2].body)
            .unwrap()
    else {
        panic!()
    };
    assert_eq!(tags.registries.0.len(), ferrite_data::configuration::TAG_REGISTRIES.len());

    // Client information is forwarded for the application; the acknowledgement enters play.
    c.send(
        &mut s,
        &ClientInformation {
            locale: Str::new("en_US"),
            view_distance: 8,
            chat_mode: VarInt(0),
            chat_colors: true,
            displayed_skin_parts: 0x7f,
            main_hand: VarInt(1),
            enable_text_filtering: false,
            allow_server_listings: true,
            particle_status: VarInt(0),
        },
    );
    c.send(&mut s, &AcknowledgeFinishConfiguration {});
    let events = s.poll_all().unwrap();
    assert!(matches!(
        &events[..],
        [Event::Configuration(ServerboundConfiguration::ClientInformation(_)), Event::EnteredPlay]
    ));
    assert_eq!(s.phase(), Phase::Play);
    assert_eq!(s.poll(), Ok(None));
}

#[test]
fn works_with_byte_at_a_time_delivery_and_encryption() {
    let key = [0x42u8; 16];
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();

    handshake(&mut c, &mut s, 2);
    s.poll_all().unwrap();
    // Encryption starts after the (plaintext) handshake: both sides switch at the same byte.
    s.enable_encryption(&key);
    c.cipher = Some(Encryptor::new(&key));
    let mut client_dec = ferrite_protocol::cipher::Decryptor::new(&key);

    let hello = c.wire(&ServerboundHello { name: Str::new("Slow"), uuid: Uuid::nil() });
    let mut events = Vec::new();
    for b in hello.iter() {
        s.feed(&[*b]);
        events.extend(s.poll_all().unwrap());
    }
    assert!(matches!(&events[..], [Event::LoginStarted { name, .. }] if name == "Slow"));
    s.accept_login(profile("Slow")).unwrap();

    // The server's output is encrypted: decrypt it, then it must parse.
    let mut raw = s.take_output();
    client_dec.apply(&mut raw);
    c.dec.queue(&raw);
    let first = c.dec.next_frame().unwrap().unwrap();
    assert_eq!(
        packet_name(State::Login, Direction::Clientbound, first.id),
        Some("login_compression")
    );
}

#[test]
fn clients_without_the_core_pack_are_turned_away_politely() {
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    login(&mut c, &mut s, "Modded");
    s.poll_all().unwrap();
    c.frames(&mut s);
    c.send(&mut s, &ServerboundSelectKnownPacks { packs: LenVec(vec![]) });
    assert!(matches!(s.poll_all(), Err(SessionError::MissingCorePack { .. })));
    // The explanation is already queued for the caller to flush before closing.
    let frames = c.frames(&mut s);
    assert_eq!(names(State::Configuration, &frames), ["disconnect"]);
    assert_eq!(s.phase(), Phase::Closed);
}

#[test]
fn out_of_order_packets_are_rejected() {
    // finish_configuration acknowledgement before the server has sent finish_configuration.
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    login(&mut c, &mut s, "Early");
    s.poll_all().unwrap();
    c.send(&mut s, &AcknowledgeFinishConfiguration {});
    assert!(matches!(
        s.poll_all(),
        Err(SessionError::Unexpected { packet: "finish_configuration", .. })
    ));

    // A configuration packet while login is not yet acknowledged.
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    handshake(&mut c, &mut s, 2);
    c.send(&mut s, &ServerboundHello { name: Str::new("X"), uuid: Uuid::nil() });
    s.poll_all().unwrap();
    s.accept_login(profile("X")).unwrap();
    c.frames(&mut s);
    c.compression(256);
    c.send(&mut s, &ServerboundKeepAlive { id: 1 });
    assert!(s.poll_all().is_err());

    // Unknown packet ids.
    let mut s = ServerSession::new(SessionConfig::default());
    s.feed(&[2, 0x7f, 0]);
    assert!(matches!(s.poll(), Err(SessionError::Protocol(_))));

    // A handshake with an invalid intent.
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    handshake(&mut c, &mut s, 9);
    assert!(s.poll().is_err());
}

#[test]
fn actions_are_refused_in_the_wrong_phase() {
    let mut s = ServerSession::new(SessionConfig::default());
    assert!(matches!(s.accept_login(profile("x")), Err(SessionError::WrongPhase { .. })));
    assert!(matches!(s.send_status("{}"), Err(SessionError::WrongPhase { .. })));
    assert!(matches!(s.reject_incompatible(), Err(SessionError::WrongPhase { .. })));
    assert!(matches!(s.into_parts(), Err(SessionError::WrongPhase { .. })));
}

#[test]
fn incompatible_clients_get_vanilla_style_rejection() {
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    c.send(
        &mut s,
        &Intention {
            protocol_version: VarInt(700),
            server_address: Str::new("h"),
            server_port: 1,
            intent: VarInt(2),
        },
    );
    c.send(&mut s, &ServerboundHello { name: Str::new("Old"), uuid: Uuid::nil() });
    let events = s.poll_all().unwrap();
    assert!(matches!(events[0], Event::Handshake { protocol_version: 700, .. }));
    s.reject_incompatible().unwrap();
    let frames = c.frames(&mut s);
    let ClientboundLogin::LoginDisconnect(d) =
        ClientboundLogin::decode(frames[0].id, &frames[0].body).unwrap()
    else {
        panic!()
    };
    assert_eq!(
        d.reason.0,
        "{\"translate\":\"multiplayer.disconnect.incompatible\",\"with\":[\"26.3\"]}"
    );
    assert_eq!(s.phase(), Phase::Closed);
}

#[test]
fn leftover_bytes_survive_the_handover_to_play() {
    let mut s = ServerSession::new(SessionConfig { compression: None, ..SessionConfig::default() });
    let mut c = Client::new();
    handshake(&mut c, &mut s, 2);
    c.send(&mut s, &ServerboundHello { name: Str::new("Fast"), uuid: Uuid::nil() });
    s.poll_all().unwrap();
    s.accept_login(profile("Fast")).unwrap();
    c.frames(&mut s);
    c.send(&mut s, &LoginAcknowledged {});
    s.poll_all().unwrap();
    c.frames(&mut s);
    c.send(&mut s, &ServerboundSelectKnownPacks { packs: LenVec(vec![core_pack()]) });
    s.poll_all().unwrap();
    c.frames(&mut s);

    // The client's acknowledgement and its first play packet arrive in one read.
    let mut burst = c.wire(&AcknowledgeFinishConfiguration {});
    burst.extend_from_slice(&[3, 0x7e, 1, 2]);
    s.feed(&burst);
    assert_eq!(s.poll_all().unwrap(), [Event::EnteredPlay]);
    let (mut dec, _enc, cipher, _out) = s.into_parts().unwrap();
    assert!(cipher.is_none());
    let next = dec.next_frame().unwrap().unwrap();
    assert_eq!((next.id, next.body.as_ref()), (0x7e, &[1u8, 2][..]));
}

#[test]
fn without_compression_no_set_compression_packet_is_sent() {
    let mut s = ServerSession::new(SessionConfig { compression: None, ..SessionConfig::default() });
    let mut c = Client::new();
    handshake(&mut c, &mut s, 2);
    c.send(&mut s, &ServerboundHello { name: Str::new("Plain"), uuid: Uuid::nil() });
    s.poll_all().unwrap();
    s.accept_login(profile("Plain")).unwrap();
    let frames = c.frames(&mut s);
    assert_eq!(names(State::Login, &frames), ["login_finished"]);
}

#[test]
fn login_plugin_answers_and_cookies_are_forwarded_to_the_application() {
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    handshake(&mut c, &mut s, 2);
    c.send(&mut s, &ServerboundHello { name: Str::new("Fwd"), uuid: Uuid::nil() });
    c.send(&mut s, &login::CustomQueryAnswer { message_id: VarInt(1), data: None });
    c.send(
        &mut s,
        &login::CookieResponse {
            key: ferrite_protocol::types::Identifier::new("a:b").unwrap(),
            payload: None,
        },
    );
    let events = s.poll_all().unwrap();
    assert!(matches!(
        &events[..],
        [Event::Handshake { .. }, Event::LoginStarted { .. }, Event::Login(_), Event::Login(_)]
    ));
    assert_eq!(s.phase(), Phase::LoginPending);
}

#[test]
fn play_packets_can_be_sent_only_in_play() {
    use ferrite_protocol::packets::play::Disconnect;
    use ferrite_protocol::types::Text;
    let kick = Disconnect { reason: Text::plain("bye") };

    let mut s = ServerSession::new(SessionConfig { compression: None, ..SessionConfig::default() });
    assert!(matches!(s.send_play(&kick), Err(SessionError::WrongPhase { .. })));

    let mut c = Client::new();
    handshake(&mut c, &mut s, 2);
    c.send(&mut s, &ServerboundHello { name: Str::new("P"), uuid: Uuid::nil() });
    s.poll_all().unwrap();
    s.accept_login(profile("P")).unwrap();
    c.frames(&mut s);
    c.send(&mut s, &LoginAcknowledged {});
    s.poll_all().unwrap();
    c.frames(&mut s);
    c.send(&mut s, &ServerboundSelectKnownPacks { packs: LenVec(vec![core_pack()]) });
    s.poll_all().unwrap();
    c.frames(&mut s);
    c.send(&mut s, &AcknowledgeFinishConfiguration {});
    assert_eq!(s.poll_all().unwrap(), [Event::EnteredPlay]);

    s.send_play(&kick).unwrap();
    let frames = c.frames(&mut s);
    assert_eq!(frames.len(), 1);
    assert_eq!(packet_name(State::Play, Direction::Clientbound, frames[0].id), Some("disconnect"));
    // A configuration packet is not a play packet.
    assert!(matches!(s.send_play(&FinishConfiguration {}), Err(SessionError::WrongPhase { .. })));
}

// ---------------------------------------------------------------------------------------------
// Online mode: RSA key exchange, AES-CFB8, and the server hash.
// ---------------------------------------------------------------------------------------------

use std::sync::Arc;

use ferrite_protocol::crypto::{ServerKey, encrypt_with_public_key, server_hash};
use ferrite_protocol::types::ByteVec;

fn server_key() -> Arc<ServerKey> {
    use std::sync::OnceLock;
    static KEY: OnceLock<Arc<ServerKey>> = OnceLock::new();
    KEY.get_or_init(|| Arc::new(ServerKey::generate().unwrap())).clone()
}

struct Exchange {
    secret: [u8; 16],
    token: Vec<u8>,
    public_key: Vec<u8>,
}

/// Handshake and hello, then the server's encryption request, decoded as a client would.
fn start_online(
    c: &mut Client,
    s: &mut ServerSession,
    key: &Arc<ServerKey>,
    name: &str,
) -> Exchange {
    handshake(c, s, 2);
    c.send(s, &ServerboundHello { name: Str::new(name), uuid: Uuid::nil() });
    let events = s.poll_all().unwrap();
    assert!(matches!(&events[..], [Event::Handshake { .. }, Event::LoginStarted { .. }]));
    s.begin_encryption(key.clone(), true).unwrap();

    let frames = c.frames(s);
    assert_eq!(names(State::Login, &frames), ["hello"]);
    let ClientboundLogin::Hello(h) =
        ClientboundLogin::decode(frames[0].id, &frames[0].body).unwrap()
    else {
        panic!()
    };
    assert_eq!(h.server_id.0, "", "vanilla always sends an empty server id");
    assert_eq!(h.public_key.0, key.public_key_der());
    assert_eq!(h.verify_token.0.len(), 4);
    assert!(h.should_authenticate);
    Exchange { secret: [0x42; 16], token: h.verify_token.0, public_key: h.public_key.0 }
}

/// The client's reply, plaintext, after which both directions are encrypted.
fn answer(c: &mut Client, s: &mut ServerSession, x: &Exchange) {
    let key_packet = login::Key {
        shared_secret: ByteVec(encrypt_with_public_key(&x.public_key, &x.secret).unwrap()),
        verify_token: ByteVec(encrypt_with_public_key(&x.public_key, &x.token).unwrap()),
    };
    c.send(s, &key_packet);
    c.cipher = Some(Encryptor::new(&x.secret));
    c.dec.enable_encryption(&x.secret);
}

#[test]
fn online_login_encrypts_everything_and_reports_the_server_hash() {
    let key = server_key();
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    let x = start_online(&mut c, &mut s, &key, "Online");
    answer(&mut c, &mut s, &x);

    let events = s.poll_all().unwrap();
    let expected_hash = server_hash("", &x.secret, key.public_key_der());
    assert_eq!(
        events,
        [Event::EncryptionEstablished { server_hash: expected_hash, authenticate: true }]
    );

    // After authentication succeeds the application accepts the login. The output is encrypted:
    // reading it as plaintext must fail, reading it through the client's cipher must work.
    s.accept_login(profile("Online")).unwrap();
    let raw = s.pending_output().to_vec();
    let mut plain_reader = FrameDecoder::new();
    plain_reader.queue(&raw);
    assert!(
        plain_reader.next_frame().is_err()
            || plain_reader.next_frame().map(|f| f.is_none()).unwrap_or(true)
            || ClientboundLogin::decode(
                plain_reader.next_frame().ok().flatten().map_or(0, |f| f.id),
                &[]
            )
            .is_err(),
        "encrypted output must not parse as plaintext framing"
    );
    let out = s.take_output();
    c.dec.queue(&out);
    let first = c.dec.next_frame().unwrap().unwrap();
    assert_eq!(
        packet_name(State::Login, Direction::Clientbound, first.id),
        Some("login_compression")
    );
    c.compression(256);
    let second = c.dec.next_frame().unwrap().unwrap();
    assert_eq!(
        packet_name(State::Login, Direction::Clientbound, second.id),
        Some("login_finished")
    );

    // The rest of the join works over the encrypted stream, including compression.
    c.send(&mut s, &LoginAcknowledged {});
    assert_eq!(s.poll_all().unwrap(), [Event::Configuring]);
    c.frames(&mut s);
    c.send(&mut s, &ServerboundSelectKnownPacks { packs: LenVec(vec![core_pack()]) });
    s.poll_all().unwrap();
    let rest = c.frames(&mut s);
    assert_eq!(names(State::Configuration, &rest).last(), Some(&"finish_configuration"));
    c.send(&mut s, &AcknowledgeFinishConfiguration {});
    assert_eq!(s.poll_all().unwrap(), [Event::EnteredPlay]);
}

#[test]
fn encrypted_packets_pipelined_with_the_key_packet_are_not_lost() {
    // The client sends `key` and immediately more (encrypted) data in the same TCP segment.
    let key = server_key();
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    let x = start_online(&mut c, &mut s, &key, "Eager");

    let mut burst = c.wire(&login::Key {
        shared_secret: ByteVec(encrypt_with_public_key(&x.public_key, &x.secret).unwrap()),
        verify_token: ByteVec(encrypt_with_public_key(&x.public_key, &x.token).unwrap()),
    });
    c.cipher = Some(Encryptor::new(&x.secret));
    c.dec.enable_encryption(&x.secret);
    // A custom_query_answer is the kind of packet a client may legitimately send next.
    burst.extend_from_slice(
        &c.wire(&login::CustomQueryAnswer { message_id: VarInt(9), data: None }),
    );
    s.feed(&burst);

    let events = s.poll_all().unwrap();
    assert!(
        matches!(&events[..], [Event::EncryptionEstablished { .. }, Event::Login(_)]),
        "{events:?}"
    );
}

#[test]
fn a_wrong_verify_token_or_secret_is_refused_with_a_generic_error() {
    let key = server_key();
    for case in ["wrong token", "short secret", "long secret", "other key"] {
        let mut s = ServerSession::new(SessionConfig::default());
        let mut c = Client::new();
        let x = start_online(&mut c, &mut s, &key, "Bad");
        let (secret, token): (Vec<u8>, Vec<u8>) = match case {
            "wrong token" => (x.secret.to_vec(), vec![0, 0, 0, 0]),
            "short secret" => (vec![1; 15], x.token.clone()),
            "long secret" => (vec![1; 17], x.token.clone()),
            _ => (x.secret.to_vec(), x.token.clone()),
        };
        let target = if case == "other key" {
            ServerKey::generate().unwrap().public_key_der().to_vec()
        } else {
            x.public_key.clone()
        };
        c.send(
            &mut s,
            &login::Key {
                shared_secret: ByteVec(encrypt_with_public_key(&target, &secret).unwrap()),
                verify_token: ByteVec(encrypt_with_public_key(&target, &token).unwrap()),
            },
        );
        assert!(matches!(s.poll_all(), Err(SessionError::BadEncryptionResponse(_))), "{case}");
        assert_eq!(s.phase(), Phase::Closed, "{case}");
        // The client is told why, in the clear (encryption never started).
        let frames = c.frames(&mut s);
        assert_eq!(names(State::Login, &frames), ["login_disconnect"], "{case}");
    }
    // A garbage-length ciphertext also fails.
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    let _ = start_online(&mut c, &mut s, &key, "Bad");
    c.send(
        &mut s,
        &login::Key { shared_secret: ByteVec(vec![1; 10]), verify_token: ByteVec(vec![]) },
    );
    assert!(s.poll_all().is_err());
}

#[test]
fn the_encryption_exchange_cannot_be_misused() {
    let key = server_key();

    // A key packet nobody asked for.
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    handshake(&mut c, &mut s, 2);
    c.send(&mut s, &ServerboundHello { name: Str::new("X"), uuid: Uuid::nil() });
    s.poll_all().unwrap();
    c.send(&mut s, &login::Key { shared_secret: ByteVec(vec![1]), verify_token: ByteVec(vec![2]) });
    assert!(matches!(s.poll_all(), Err(SessionError::Unexpected { packet: "key", .. })));

    // Requesting encryption before hello, or twice, or after login was accepted.
    let mut s = ServerSession::new(SessionConfig::default());
    assert!(matches!(s.begin_encryption(key.clone(), true), Err(SessionError::WrongPhase { .. })));
    let mut c = Client::new();
    let x = start_online(&mut c, &mut s, &key, "Twice");
    assert!(matches!(s.begin_encryption(key.clone(), true), Err(SessionError::WrongPhase { .. })));

    // Replaying the key packet after the exchange completed.
    answer(&mut c, &mut s, &x);
    s.poll_all().unwrap();
    c.send(
        &mut s,
        &login::Key {
            shared_secret: ByteVec(encrypt_with_public_key(&x.public_key, &x.secret).unwrap()),
            verify_token: ByteVec(encrypt_with_public_key(&x.public_key, &x.token).unwrap()),
        },
    );
    assert!(matches!(s.poll_all(), Err(SessionError::Unexpected { packet: "key", .. })));
}

#[test]
fn authentication_can_be_declined_by_the_application_with_a_translated_message() {
    let key = server_key();
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    let x = start_online(&mut c, &mut s, &key, "Impostor");
    answer(&mut c, &mut s, &x);
    s.poll_all().unwrap();

    s.disconnect_translate("multiplayer.disconnect.unverified_username").unwrap();
    let frames = c.frames(&mut s);
    let ClientboundLogin::LoginDisconnect(d) =
        ClientboundLogin::decode(frames[0].id, &frames[0].body).unwrap()
    else {
        panic!()
    };
    assert_eq!(d.reason.0, "{\"translate\":\"multiplayer.disconnect.unverified_username\"}");
    assert_eq!(s.phase(), Phase::Closed);

    // Keys are restricted so they cannot break out of the JSON string.
    let mut s = ServerSession::new(SessionConfig::default());
    assert!(s.disconnect_translate("a\"b").is_err());
    assert!(s.disconnect_translate("a b").is_err());
}

#[test]
fn offline_encryption_flag_is_reported_when_authentication_is_off() {
    // Since 1.20.5 a server may encrypt without authenticating (should_authenticate = false).
    let key = server_key();
    let mut s = ServerSession::new(SessionConfig::default());
    let mut c = Client::new();
    handshake(&mut c, &mut s, 2);
    c.send(&mut s, &ServerboundHello { name: Str::new("Quiet"), uuid: Uuid::nil() });
    s.poll_all().unwrap();
    s.begin_encryption(key.clone(), false).unwrap();
    let frames = c.frames(&mut s);
    let ClientboundLogin::Hello(h) =
        ClientboundLogin::decode(frames[0].id, &frames[0].body).unwrap()
    else {
        panic!()
    };
    assert!(!h.should_authenticate);
    let x = Exchange { secret: [3; 16], token: h.verify_token.0, public_key: h.public_key.0 };
    answer(&mut c, &mut s, &x);
    let events = s.poll_all().unwrap();
    assert!(matches!(&events[..], [Event::EncryptionEstablished { authenticate: false, .. }]));
}
