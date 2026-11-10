//! Differential tests against a real vanilla server.
//!
//! These speak the protocol to Mojang's own server using only this crate's encoders and decoders.
//! Every packet the server sends must decode with *zero* leftover bytes, so any wrong field
//! layout, limit or id shows up here. Run with `scripts/oracle.sh` (needs the server jar and an
//! explicit EULA opt-in), or point `FERRITE_ORACLE` at a running offline-mode 26.3 server.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use bytes::BytesMut;
use ferrite_data::{Direction, State, packet_name};
use ferrite_protocol::codec::VarInt;
use ferrite_protocol::packets::configuration::*;
use ferrite_protocol::packets::handshake::*;
use ferrite_protocol::packets::login::{ClientboundLogin, LoginAcknowledged, ServerboundHello};
use ferrite_protocol::packets::status::*;
use ferrite_protocol::types::Str;
use ferrite_protocol::{Compression, Frame, FrameDecoder, FrameEncoder, Packet};
use uuid::Uuid;

/// Tags of one registry as `(tag name, resolved ids)`.
type TagList = Vec<(String, Vec<i32>)>;

fn addr() -> String {
    std::env::var("FERRITE_ORACLE").expect("set FERRITE_ORACLE=host:port (see scripts/oracle.sh)")
}

struct Client {
    stream: TcpStream,
    dec: FrameDecoder,
    enc: FrameEncoder,
    trace: Vec<String>,
}

impl Client {
    fn connect() -> Self {
        let stream = TcpStream::connect(addr()).expect("connect to the oracle server");
        stream.set_read_timeout(Some(Duration::from_secs(15))).unwrap();
        stream.set_nodelay(true).unwrap();
        Client {
            stream,
            // The server may send much larger frames than a client ever may.
            dec: FrameDecoder::new().with_limits(1 << 26, 1 << 26),
            enc: FrameEncoder::new(),
            trace: Vec::new(),
        }
    }

    fn send<P: Packet>(&mut self, p: &P) {
        let mut body = Vec::new();
        p.encode(&mut body).unwrap();
        let mut out = BytesMut::new();
        self.enc.encode(P::ID, &body, &mut out).unwrap();
        self.stream.write_all(&out).unwrap();
        self.trace.push(format!("C->S {}", P::NAME));
    }

    fn recv(&mut self) -> Frame {
        let mut tmp = [0u8; 16384];
        loop {
            if let Some(f) = self.dec.next_frame().expect("frame decoding") {
                return f;
            }
            let n = self.stream.read(&mut tmp).expect("read from the oracle");
            assert!(
                n > 0,
                "server closed the connection; trace so far:\n{}",
                self.trace.join("\n")
            );
            self.dec.queue(&tmp[..n]);
        }
    }

    fn enable_compression(&mut self, threshold: usize) {
        let c = Some(Compression { threshold });
        self.dec.set_compression(c);
        self.enc.set_compression(c, 4).unwrap();
    }

    fn handshake(&mut self, intent: i32) {
        self.send(&Intention {
            protocol_version: VarInt(ferrite_data::PROTOCOL_VERSION),
            server_address: Str::new("localhost"),
            server_port: 25565,
            intent: VarInt(intent),
        });
    }
}

#[test]
#[ignore = "needs a vanilla server: scripts/oracle.sh"]
fn status_ping_round_trip() {
    let mut c = Client::connect();
    c.handshake(1);
    c.send(&StatusRequest {});
    let f = c.recv();
    let ClientboundStatus::StatusResponse(resp) = ClientboundStatus::decode(f.id, &f.body).unwrap()
    else {
        panic!("expected status_response, got id {}", f.id)
    };
    eprintln!("status json: {}", resp.json.0);
    let json: serde_like::Status = serde_like::parse(&resp.json.0);
    assert_eq!(json.protocol, ferrite_data::PROTOCOL_VERSION);
    assert_eq!(json.name, ferrite_data::GAME_VERSION);

    c.send(&PingRequest { timestamp: 0x0123_4567_89ab_cdef });
    let f = c.recv();
    assert_eq!(
        ClientboundStatus::decode(f.id, &f.body).unwrap(),
        PongResponse { timestamp: 0x0123_4567_89ab_cdef }.into()
    );
}

/// Pulls just `version.name` and `version.protocol` out of the status JSON without a JSON crate.
mod serde_like {
    pub struct Status {
        pub name: String,
        pub protocol: i32,
    }
    pub fn parse(json: &str) -> Status {
        let v = json.find("\"version\"").expect("version object");
        let rest = &json[v..];
        let name = field_str(rest, "\"name\":\"");
        let protocol = rest
            .split("\"protocol\":")
            .nth(1)
            .expect("protocol field")
            .split(|c: char| !c.is_ascii_digit() && c != '-')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        Status { name, protocol }
    }
    fn field_str(s: &str, key: &str) -> String {
        s.split(key).nth(1).expect("string field").split('"').next().unwrap().to_owned()
    }
}

#[test]
#[ignore = "needs a vanilla server: scripts/oracle.sh"]
fn offline_login_and_full_configuration() {
    let mut c = Client::connect();
    c.handshake(2);
    c.send(&ServerboundHello { name: Str::new("FerriteBot"), uuid: Uuid::nil() });

    // Login: optional compression, then login_finished.
    let mut compression = None;
    let profile = loop {
        let f = c.recv();
        let msg = ClientboundLogin::decode(f.id, &f.body).unwrap_or_else(|e| {
            panic!(
                "login packet id {} ({:?}) failed: {e}",
                f.id,
                packet_name(State::Login, Direction::Clientbound, f.id)
            )
        });
        c.trace.push(format!("S->C {}", msg.name()));
        match msg {
            ClientboundLogin::LoginCompression(p) => {
                compression = Some(p.threshold.0);
                c.enable_compression(p.threshold.0 as usize);
            }
            ClientboundLogin::LoginFinished(p) => {
                eprintln!("session_id: {} (profile uuid {})", p.session_id, p.profile.uuid);
                break p.profile;
            }
            other => panic!("unexpected login packet {other:?}"),
        }
    };
    eprintln!("compression: {compression:?}, profile: {profile:?}");
    assert_eq!(profile.name.0, "FerriteBot");
    // Vanilla derives offline UUIDs from "OfflinePlayer:<name>" (version 3).
    assert_eq!(profile.uuid.get_version_num(), 3);

    c.send(&LoginAcknowledged {});

    c.send(&ClientInformation {
        locale: Str::new("en_US"),
        view_distance: 2,
        chat_mode: VarInt(0),
        chat_colors: true,
        displayed_skin_parts: 0x7f,
        main_hand: VarInt(1),
        enable_text_filtering: false,
        allow_server_listings: true,
        particle_status: VarInt(0),
    });

    let mut registries: Vec<(String, usize, usize)> = Vec::new();
    let mut tags = 0usize;
    let mut dump_registries: Vec<(String, Vec<String>)> = Vec::new();
    let mut dump_tags: Vec<(String, TagList)> = Vec::new();
    let mut dump_order: Vec<String> = Vec::new();
    let mut sent_known = false;
    loop {
        let f = c.recv();
        let msg = ClientboundConfiguration::decode(f.id, &f.body).unwrap_or_else(|e| {
            panic!(
                "configuration packet id {} ({:?}) failed to decode: {e}\nbody: {:02x?}",
                f.id,
                packet_name(State::Configuration, Direction::Clientbound, f.id),
                &f.body[..f.body.len().min(64)]
            )
        });
        c.trace.push(format!("S->C {}", msg.name()));
        match msg {
            ClientboundConfiguration::SelectKnownPacks(p) => {
                eprintln!("server known packs: {:?}", p.packs);
                c.send(&ServerboundSelectKnownPacks { packs: p.packs });
                sent_known = true;
            }
            ClientboundConfiguration::RegistryData(p) => {
                let with_data = p.entries.0.iter().filter(|e| e.data.is_some()).count();
                registries.push((p.registry.as_str().to_owned(), p.entries.0.len(), with_data));
                dump_registries.push((
                    p.registry.as_str().to_owned(),
                    p.entries.0.iter().map(|e| e.id.as_str().to_owned()).collect(),
                ));
            }
            ClientboundConfiguration::UpdateTags(p) => {
                tags += p.registries.0.len();
                for r in &p.registries.0 {
                    dump_tags.push((
                        r.registry.as_str().to_owned(),
                        r.tags
                            .0
                            .iter()
                            .map(|t| {
                                (
                                    t.name.as_str().to_owned(),
                                    t.entries.0.iter().map(|v| v.0).collect(),
                                )
                            })
                            .collect(),
                    ));
                }
            }
            ClientboundConfiguration::CustomPayload(p) => {
                dump_order.push(format!("custom_payload {} {:02x?}", p.channel.as_str(), p.data.0));
            }
            ClientboundConfiguration::UpdateEnabledFeatures(p) => {
                dump_order.push(format!(
                    "features {:?}",
                    p.features.0.iter().map(|f| f.as_str()).collect::<Vec<_>>()
                ));
            }
            ClientboundConfiguration::KeepAlive(p) => c.send(&ServerboundKeepAlive { id: p.id }),
            ClientboundConfiguration::Ping(p) => c.send(&Pong { id: p.id }),
            ClientboundConfiguration::FinishConfiguration(_) => {
                c.send(&AcknowledgeFinishConfiguration {});
                break;
            }
            ClientboundConfiguration::Disconnect(p) => panic!("kicked during configuration: {p:?}"),
            _ => {}
        }
    }
    eprintln!("--- trace ---\n{}", c.trace.join("\n"));
    eprintln!("--- {} synchronized registries ---", registries.len());
    for (name, n, with_data) in &registries {
        eprintln!("{name}: {n} entries, {with_data} with inline data");
    }
    eprintln!("tag registries: {tags}");
    for line in &dump_order {
        eprintln!("{line}");
    }
    // Our generated configuration data must equal what vanilla sent: same registries in the same
    // order, same entries in the same id order, and the same tags with the same resolved ids.
    // (Vanilla iterates tags in hash order, which is unspecified; tags are compared by name.)
    let ours = ferrite_data::configuration::SYNCHRONIZED_REGISTRIES;
    assert_eq!(dump_registries.len(), ours.len(), "number of synchronized registries");
    for ((name, entries), mine) in dump_registries.iter().zip(ours.iter()) {
        assert_eq!(name, mine.id, "registry order");
        let mine_entries: Vec<&str> = mine.entries.to_vec();
        let theirs: Vec<&str> = entries.iter().map(String::as_str).collect();
        assert_eq!(theirs, mine_entries, "entries of {name}");
    }
    // Vanilla sends tag registries (and the tags inside them) in hash-iteration order, which
    // differs from one server start to the next, so compare by name rather than by position.
    use std::collections::BTreeMap;
    let our_tags = ferrite_data::configuration::TAG_REGISTRIES;
    let theirs: BTreeMap<&str, BTreeMap<&str, Vec<i32>>> = dump_tags
        .iter()
        .map(|(reg, tags)| {
            (reg.as_str(), tags.iter().map(|(n, ids)| (n.as_str(), ids.clone())).collect())
        })
        .collect();
    let ours: BTreeMap<&str, BTreeMap<&str, Vec<i32>>> = our_tags
        .iter()
        .map(|r| {
            (
                r.registry,
                r.tags
                    .iter()
                    .map(|t| (t.name, t.entries.iter().map(|&i| i32::from(i)).collect()))
                    .collect(),
            )
        })
        .collect();
    assert_eq!(
        theirs.keys().collect::<Vec<_>>(),
        ours.keys().collect::<Vec<_>>(),
        "tag registries"
    );
    for (name, their_tags) in &theirs {
        assert_eq!(their_tags, &ours[name], "tags of {name}");
    }
    eprintln!("generated configuration data matches vanilla exactly");

    if let Ok(dir) = std::env::var("FERRITE_ORACLE_DUMP") {
        dump_json(&dir, &dump_registries, &dump_tags);
    }
    assert!(sent_known, "vanilla must negotiate known packs");
    assert!(!registries.is_empty());

    // The next packet is the first play packet: `login`.
    let f = c.recv();
    assert_eq!(packet_name(State::Play, Direction::Clientbound, f.id), Some("login"));
}

/// Writes what vanilla sent as JSON so generated tables can be compared against it.
fn dump_json(dir: &str, regs: &[(String, Vec<String>)], tags: &[(String, TagList)]) {
    use std::fmt::Write;
    let mut out = String::from("{\"registries\":[");
    for (i, (name, entries)) in regs.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let list: Vec<String> = entries.iter().map(|e| format!("{e:?}")).collect();
        write!(out, "[{name:?},[{}]]", list.join(",")).unwrap();
    }
    out.push_str("],\"tags\":[");
    for (i, (reg, list)) in tags.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write!(out, "[{reg:?},[").unwrap();
        for (j, (tag, ids)) in list.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            let ids: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
            write!(out, "[{tag:?},[{}]]", ids.join(",")).unwrap();
        }
        out.push_str("]]");
    }
    out.push_str("]}");
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(format!("{dir}/vanilla-configuration.json"), out).unwrap();
}
