use std::collections::BTreeSet;

use ferrite_data::{Direction, State, packet_count, packet_name};
use ferrite_nbt::{Compound, Tag};
use ferrite_protocol::codec::{Decode, Encode, VarInt, decode_exact};
use ferrite_protocol::packets::configuration::{self, *};
use ferrite_protocol::packets::handshake::*;
use ferrite_protocol::packets::login::{self, *};
use ferrite_protocol::packets::status::*;
use ferrite_protocol::types::*;
use ferrite_protocol::{Packet, ProtocolError};
use uuid::Uuid;

fn names_of(state: State, dir: Direction) -> BTreeSet<&'static str> {
    (0..packet_count(state, dir) as i32).map(|id| packet_name(state, dir, id).unwrap()).collect()
}

fn check_complete(state: State, dir: Direction, enum_names: &[&'static str]) {
    let have: BTreeSet<_> = enum_names.iter().copied().collect();
    assert_eq!(have.len(), enum_names.len(), "{state:?}/{dir:?}: duplicate variant");
    assert_eq!(
        have,
        names_of(state, dir),
        "{state:?}/{dir:?}: enum must cover exactly the generated packets"
    );
}

#[test]
fn every_generated_packet_before_play_has_a_definition() {
    use Direction::*;
    use State::*;
    check_complete(Handshake, Serverbound, ServerboundHandshake::NAMES);
    check_complete(Status, Clientbound, ClientboundStatus::NAMES);
    check_complete(Status, Serverbound, ServerboundStatus::NAMES);
    check_complete(Login, Clientbound, ClientboundLogin::NAMES);
    check_complete(Login, Serverbound, ServerboundLogin::NAMES);
    check_complete(Configuration, Clientbound, ClientboundConfiguration::NAMES);
    check_complete(Configuration, Serverbound, ServerboundConfiguration::NAMES);
}

#[test]
fn packet_ids_resolve_back_to_their_official_names() {
    fn check<P: Packet>() {
        assert_eq!(packet_name(P::STATE, P::DIRECTION, P::ID), Some(P::NAME), "{}", P::NAME);
    }
    check::<Intention>();
    check::<StatusResponse>();
    check::<PongResponse>();
    check::<StatusRequest>();
    check::<PingRequest>();
    check::<LoginDisconnect>();
    check::<Hello>();
    check::<LoginFinished>();
    check::<LoginCompression>();
    check::<CustomQuery>();
    check::<login::CookieRequest>();
    check::<ServerboundHello>();
    check::<Key>();
    check::<CustomQueryAnswer>();
    check::<LoginAcknowledged>();
    check::<login::CookieResponse>();
    check::<RegistryData>();
    check::<UpdateTags>();
    check::<SelectKnownPacks>();
    check::<ClientInformation>();
    check::<ServerboundSelectKnownPacks>();
}

fn round_trip<P>(p: P)
where
    P: Packet + for<'a> Decode<'a> + PartialEq + std::fmt::Debug,
{
    let mut body = Vec::new();
    p.encode(&mut body).unwrap();
    assert_eq!(decode_exact::<P>(&body), Ok(p), "{}", P::NAME);
}

fn uuid(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn kp() -> KnownPack {
    KnownPack { namespace: "minecraft".into(), id: "core".into(), version: "26.3".into() }
}

#[test]
fn handshake_wire_format_and_intents() {
    let p = Intention {
        protocol_version: VarInt(777),
        server_address: "localhost".into(),
        server_port: 25565,
        intent: VarInt(1),
    };
    let mut body = Vec::new();
    p.encode(&mut body).unwrap();
    assert_eq!(
        body,
        [0x89, 0x06, 9, b'l', b'o', b'c', b'a', b'l', b'h', b'o', b's', b't', 0x63, 0xdd, 0x01]
    );
    assert_eq!(p.intent(), Ok(Intent::Status));
    assert_eq!(Intention { intent: VarInt(2), ..p.clone() }.intent(), Ok(Intent::Login));
    assert_eq!(Intention { intent: VarInt(3), ..p.clone() }.intent(), Ok(Intent::Transfer));
    assert!(Intention { intent: VarInt(4), ..p.clone() }.intent().is_err());
    // The address is capped at 255 characters.
    assert!(
        Intention { server_address: Str::new("a".repeat(256)), ..p.clone() }
            .encode(&mut Vec::new())
            .is_err()
    );
    round_trip(p);
}

#[test]
fn status_packets() {
    round_trip(StatusResponse { json: Str::new("{\"version\":{}}") });
    round_trip(PongResponse { timestamp: -1 });
    round_trip(PingRequest { timestamp: 1_700_000_000_123 });
    let mut b = Vec::new();
    StatusRequest {}.encode(&mut b).unwrap();
    assert!(b.is_empty());
    assert_eq!(ServerboundStatus::decode(StatusRequest::ID, &[]), Ok(StatusRequest {}.into()));
    assert_eq!(
        ServerboundStatus::decode(StatusRequest::ID, &[0]),
        Err(ProtocolError::TrailingBytes(1))
    );
}

#[test]
fn login_packets() {
    round_trip(LoginDisconnect { reason: JsonText::plain("bye \"now\"\n") });
    round_trip(Hello {
        server_id: Str::new(""),
        public_key: ByteVec(vec![1, 2, 3]),
        verify_token: ByteVec(vec![9; 4]),
        should_authenticate: true,
    });
    round_trip(LoginFinished {
        profile: GameProfile {
            uuid: uuid(7),
            name: "Notch".into(),
            properties: LenVec(vec![
                ProfileProperty {
                    name: "textures".into(),
                    value: "abc".into(),
                    signature: Some("sig".into()),
                },
                ProfileProperty { name: "x".into(), value: "y".into(), signature: None },
            ]),
        },
        session_id: uuid(8),
    });
    round_trip(LoginCompression { threshold: VarInt(256) });
    round_trip(CustomQuery {
        message_id: VarInt(5),
        channel: Identifier::new("velocity:player_info").unwrap(),
        data: Remaining(vec![1, 2, 3]),
    });
    round_trip(login::CookieRequest { key: Identifier::new("a:b").unwrap() });
    round_trip(ServerboundHello { name: "Notch".into(), uuid: uuid(1) });
    round_trip(Key { shared_secret: ByteVec(vec![0; 128]), verify_token: ByteVec(vec![1; 128]) });
    round_trip(CustomQueryAnswer { message_id: VarInt(5), data: None });
    round_trip(CustomQueryAnswer { message_id: VarInt(5), data: Some(Remaining(vec![4, 5])) });
    round_trip(LoginAcknowledged {});
    round_trip(login::CookieResponse {
        key: Identifier::new("a:b").unwrap(),
        payload: Some(ByteVec(vec![1])),
    });

    // The username is limited to 16 characters.
    assert!(
        ServerboundHello { name: Str::new("a".repeat(17)), uuid: uuid(0) }
            .encode(&mut Vec::new())
            .is_err()
    );
    // A cookie payload above 5 KiB is refused in both directions.
    assert!(
        login::CookieResponse {
            key: Identifier::new("a:b").unwrap(),
            payload: Some(ByteVec(vec![0; 5121]))
        }
        .encode(&mut Vec::new())
        .is_err()
    );
}

#[test]
fn json_text_escapes_correctly() {
    assert_eq!(JsonText::plain("a\"b\\c\n\u{1}").0, "{\"text\":\"a\\\"b\\\\c\\n\\u0001\"}");
}

#[test]
fn configuration_packets() {
    let dim = Nbt(Tag::Compound(Compound::new().with("height", Tag::Int(384))));
    round_trip(configuration::CookieRequest { key: Identifier::new("a:b").unwrap() });
    round_trip(CustomPayload {
        channel: Identifier::new("minecraft:brand").unwrap(),
        data: Remaining(b"\x06ferrite".to_vec()),
    });
    round_trip(Disconnect { reason: Text::plain("kicked") });
    round_trip(FinishConfiguration {});
    round_trip(KeepAlive { id: i64::MIN });
    round_trip(Ping { id: -5 });
    round_trip(ResetChat {});
    round_trip(RegistryData {
        registry: Identifier::new("minecraft:dimension_type").unwrap(),
        entries: LenVec(vec![
            RegistryEntry { id: Identifier::new("minecraft:overworld").unwrap(), data: Some(dim) },
            RegistryEntry { id: Identifier::new("minecraft:the_nether").unwrap(), data: None },
        ]),
    });
    round_trip(ResourcePackPop { id: None });
    round_trip(ResourcePackPop { id: Some(uuid(3)) });
    round_trip(ResourcePackPush {
        id: uuid(4),
        url: Str::new("https://x/y.zip"),
        hash: Str::new("0".repeat(40)),
        forced: true,
        prompt: Some(Text::plain("hi")),
    });
    round_trip(PostEffects { effects: LenVec(vec![Identifier::new("minecraft:blur").unwrap()]) });
    round_trip(StoreCookie { key: Identifier::new("a:b").unwrap(), payload: ByteVec(vec![1, 2]) });
    round_trip(Transfer { host: Str::new("example.org"), port: VarInt(25565) });
    round_trip(UpdateEnabledFeatures {
        features: LenVec(vec![Identifier::new("minecraft:vanilla").unwrap()]),
    });
    round_trip(UpdateTags {
        registries: LenVec(vec![RegistryTags {
            registry: Identifier::new("minecraft:block").unwrap(),
            tags: LenVec(vec![TagEntry {
                name: Identifier::new("minecraft:climbable").unwrap(),
                entries: LenVec(vec![VarInt(1), VarInt(300)]),
            }]),
        }]),
    });
    round_trip(SelectKnownPacks { packs: LenVec(vec![kp()]) });
    round_trip(CustomReportDetails {
        details: LenVec(vec![ReportDetail { title: "t".into(), description: "d".into() }]),
    });
    round_trip(ServerLinks {
        links: LenVec(vec![
            ServerLink {
                label: Either::Left(VarInt(BuiltInLink::Website as i32)),
                url: Str::new("https://a"),
            },
            ServerLink { label: Either::Right(Text::plain("Custom")), url: Str::new("https://b") },
        ]),
    });
    round_trip(ClearDialog {});
    round_trip(ShowDialog {
        dialog: Nbt(Tag::Compound(Compound::new().with("type", "minecraft:notice".into()))),
    });
    round_trip(CodeOfConduct { text: Str::new("be nice") });

    round_trip(ClientInformation {
        locale: Str::new("en_GB"),
        view_distance: 12,
        chat_mode: VarInt(0),
        chat_colors: true,
        displayed_skin_parts: 0x7f,
        main_hand: VarInt(1),
        enable_text_filtering: false,
        allow_server_listings: true,
        particle_status: VarInt(0),
    });
    round_trip(ServerboundCookieResponse { key: Identifier::new("a:b").unwrap(), payload: None });
    round_trip(ServerboundCustomPayload {
        channel: Identifier::new("a:b").unwrap(),
        data: Remaining(vec![1]),
    });
    round_trip(AcknowledgeFinishConfiguration {});
    round_trip(ServerboundKeepAlive { id: 99 });
    round_trip(Pong { id: 7 });
    round_trip(ResourcePackResponse { id: uuid(9), result: VarInt(3) });
    round_trip(ServerboundSelectKnownPacks { packs: LenVec(vec![kp()]) });
    round_trip(CustomClickAction {
        id: Identifier::new("a:b").unwrap(),
        payload: ByteVec(vec![0]),
    });
    round_trip(AcceptCodeOfConduct {});
}

#[test]
fn limits_from_the_specification_are_enforced() {
    // Known packs: at most 64 from the client.
    let many = LenVec::<KnownPack, 64>((0..65).map(|_| kp()).collect());
    assert!(matches!(
        many.encode(&mut Vec::new()),
        Err(ProtocolError::LengthLimit { len: 65, max: 64 })
    ));
    // Client plugin payloads are capped at 32767 bytes, server ones at 1 MiB.
    assert!(
        ServerboundCustomPayload {
            channel: Identifier::new("a:b").unwrap(),
            data: Remaining(vec![0; 32768])
        }
        .encode(&mut Vec::new())
        .is_err()
    );
    assert!(
        CustomPayload { channel: Identifier::new("a:b").unwrap(), data: Remaining(vec![0; 32768]) }
            .encode(&mut Vec::new())
            .is_ok()
    );
    // Locale is limited to 16 characters.
    assert!(
        ClientInformation {
            locale: Str::new("x".repeat(17)),
            view_distance: 1,
            chat_mode: VarInt(0),
            chat_colors: true,
            displayed_skin_parts: 0,
            main_hand: VarInt(1),
            enable_text_filtering: false,
            allow_server_listings: true,
            particle_status: VarInt(0)
        }
        .encode(&mut Vec::new())
        .is_err()
    );
}

#[test]
fn identifiers_are_validated() {
    for ok in ["minecraft:stone", "stone", "a.b-c_d:e/f", "x:y/z/0", ":x", ""] {
        assert!(Identifier::new(ok).is_ok(), "{ok}");
    }
    for bad in ["Minecraft:stone", "a:B", "a b", "a:b:c", "é:x", "a:b\n", "a:b c"] {
        assert!(Identifier::new(bad).is_err(), "{bad:?}");
    }
    // Decoding validates too, so a hostile packet cannot smuggle a bad identifier into the server.
    let mut body = Vec::new();
    Str::<32767>::new("Bad Id").encode(&mut body).unwrap();
    assert!(matches!(decode_exact::<Identifier>(&body), Err(ProtocolError::InvalidIdentifier(_))));
}

#[test]
fn unknown_ids_and_trailing_bytes_are_errors() {
    assert_eq!(
        ServerboundHandshake::decode(5, &[]),
        Err(ProtocolError::UnknownPacket {
            state: State::Handshake,
            direction: Direction::Serverbound,
            id: 5
        })
    );
    let mut body = Vec::new();
    PingRequest { timestamp: 1 }.encode(&mut body).unwrap();
    body.push(0);
    assert_eq!(
        ServerboundStatus::decode(PingRequest::ID, &body),
        Err(ProtocolError::TrailingBytes(1))
    );
    assert_eq!(ServerboundStatus::decode(PingRequest::ID, &body[..4]), Err(ProtocolError::Eof));
}

#[test]
fn dispatch_enums_report_ids_and_round_trip() {
    let msg: ClientboundLogin = LoginCompression { threshold: VarInt(256) }.into();
    assert_eq!(msg.name(), "login_compression");
    let mut body = Vec::new();
    msg.encode_body(&mut body).unwrap();
    assert_eq!(ClientboundLogin::decode(msg.id(), &body), Ok(msg));
}

#[test]
fn text_components_must_be_strings_or_compounds() {
    assert!(Text(Tag::Int(1)).encode(&mut Vec::new()).is_err());
    let mut body = Vec::new();
    Nbt(Tag::Int(1)).encode(&mut body).unwrap();
    assert!(decode_exact::<Text>(&body).is_err());
    // A lone TAG_End is "no tag" and not a valid required NBT value.
    assert!(decode_exact::<Nbt>(&[0]).is_err());
    // `Option<Nbt>` is bool-prefixed: absent is a lone 0, present-but-TAG_End is invalid.
    assert_eq!(decode_exact::<Option<Nbt>>(&[0]), Ok(None));
    assert!(decode_exact::<Option<Nbt>>(&[1, 0]).is_err());
}
