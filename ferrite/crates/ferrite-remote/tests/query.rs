use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ferrite_remote::query::*;
use proptest::prelude::*;
use tokio::net::UdpSocket;

struct Info {
    players: Vec<String>,
}
impl ServerInfo for Info {
    fn motd(&self) -> String {
        "A Ferrite Server".into()
    }
    fn version(&self) -> String {
        "26.3".into()
    }
    fn map(&self) -> String {
        "world".into()
    }
    fn online(&self) -> u32 {
        self.players.len() as u32
    }
    fn max_players(&self) -> u32 {
        20
    }
    fn players(&self) -> Vec<String> {
        self.players.clone()
    }
    fn host_port(&self) -> u16 {
        25565
    }
    fn host_ip(&self) -> String {
        "127.0.0.1".into()
    }
}

fn handler(players: &[&str]) -> QueryHandler {
    QueryHandler::new(Arc::new(Info { players: players.iter().map(|s| s.to_string()).collect() }))
}

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([203, 0, 113, 9], port))
}

fn token_for(h: &mut QueryHandler, from: SocketAddr, now: Instant) -> i32 {
    let reply =
        h.handle(from, &request(9, 1, None, false), now).expect("handshake is always answered");
    parse_handshake(&reply).unwrap().1
}

#[test]
fn handshake_matches_the_documented_wire_format() {
    let mut h = handler(&[]);
    let reply =
        h.handle(addr(1), &[0xFE, 0xFD, 0x09, 0x00, 0x00, 0x00, 0x01], Instant::now()).unwrap();
    assert_eq!(&reply[..5], [0x09, 0, 0, 0, 1]);
    assert_eq!(*reply.last().unwrap(), 0, "the token string is NUL-terminated");
    let (session, token) = parse_handshake(&reply).unwrap();
    assert_eq!((session, reply.len() > 6), (1, true));
    assert_eq!(&reply[5..reply.len() - 1], token.to_string().as_bytes());
    // Asking again from the same address returns the same token within the period.
    assert_eq!(token_for(&mut h, addr(1), Instant::now()), token);
}

#[test]
fn basic_stat_matches_the_documented_layout() {
    let mut h = handler(&["Alex", "Steve"]);
    let now = Instant::now();
    let token = token_for(&mut h, addr(1), now);
    let reply = h.handle(addr(1), &request(0, 1, Some(token), false), now).unwrap();
    let stat = parse_basic(&reply).unwrap();
    assert_eq!(
        stat,
        BasicStat {
            session: 1,
            motd: "A Ferrite Server".into(),
            gametype: "SMP".into(),
            map: "world".into(),
            online: "2".into(),
            max: "20".into(),
            host_port: 25565,
            host_ip: "127.0.0.1".into(),
        }
    );
    // hostport is the one little-endian field: 25565 = 0x63DD, sent DD 63.
    let at = reply.windows(2).position(|w| w == [0xDD, 0x63]).expect("little-endian port");
    assert!(at > 5);
}

#[test]
fn full_stat_lists_fields_and_players() {
    let mut h = handler(&["barneygale", "TkTech"]);
    let now = Instant::now();
    let token = token_for(&mut h, addr(1), now);
    let reply = h.handle(addr(1), &request(0, 1, Some(token), true), now).unwrap();
    assert_eq!(&reply[5..16], b"splitnum\0\x80\0", "the 11 constant padding bytes");
    let stat = parse_full(&reply).unwrap();
    let keys: Vec<_> = stat.fields.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(
        keys,
        [
            "hostname",
            "gametype",
            "game_id",
            "version",
            "plugins",
            "map",
            "numplayers",
            "maxplayers",
            "hostport",
            "hostip"
        ]
    );
    assert_eq!(stat.fields[0].1, "A Ferrite Server");
    assert_eq!(
        stat.fields[4],
        ("plugins".to_string(), String::new()),
        "vanilla sends an empty plugin list"
    );
    assert_eq!(stat.fields[6].1, "2");
    assert_eq!(stat.players, ["barneygale", "TkTech"]);
    assert!(reply.ends_with(&[0, 0]));

    // With nobody online the player list is just its terminator.
    let mut h = handler(&[]);
    let token = token_for(&mut h, addr(1), now);
    let stat =
        parse_full(&h.handle(addr(1), &request(0, 1, Some(token), true), now).unwrap()).unwrap();
    assert!(stat.players.is_empty());
}

#[test]
fn the_session_id_is_echoed_verbatim() {
    // Vanilla echoes it unchanged (and corrupts ids with high bits set; we do not).
    let mut h = handler(&[]);
    let now = Instant::now();
    for session in [0u32, 1, 0x7F7F_7F7F, 0x0F0F_0F0F, 0xFFFF_FFFF, 0x1234_5678] {
        let reply = h.handle(addr(1), &request(9, session, None, false), now).unwrap();
        let (echoed, token) = parse_handshake(&reply).unwrap();
        assert_eq!(echoed, session);
        let stat = h.handle(addr(1), &request(0, session, Some(token), false), now).unwrap();
        assert_eq!(parse_basic(&stat).unwrap().session, session);
    }
}

#[test]
fn basic_versus_full_follows_the_payload_length_exactly_like_vanilla() {
    // Measured against vanilla 26.3: exactly 8 bytes is a full stat (whatever the padding holds),
    // any other length of 4 or more is a basic stat, and fewer than 4 is ignored.
    let mut h = handler(&["p"]);
    let now = Instant::now();
    let token = token_for(&mut h, addr(1), now);
    let with_payload = |extra: &[u8]| {
        let mut d = request(0, 1, Some(token), false);
        d.extend_from_slice(extra);
        d
    };
    let is_full = |reply: &[u8]| reply[5..].starts_with(b"splitnum");
    for (extra, full) in [
        (&[][..], false),
        (&[0][..], false),
        (&[0; 2][..], false),
        (&[0; 3][..], false),
        (&[0; 4][..], true),
        (&[0xaa, 0xbb, 0xcc, 0xdd][..], true),
        (&[0; 5][..], false),
        (&[0; 8][..], false),
    ] {
        let reply = h.handle(addr(1), &with_payload(extra), now).unwrap();
        assert_eq!(is_full(&reply), full, "{} extra bytes", extra.len());
    }
    // Fewer than four bytes of token is not a request.
    assert!(h.handle(addr(1), &request(0, 1, Some(token), false)[..9], now).is_none());
}

#[test]
fn requests_without_a_valid_token_get_no_reply_at_all() {
    let mut h = handler(&[]);
    let now = Instant::now();
    // Stat before any handshake, and with a guessed token.
    assert!(h.handle(addr(1), &request(0, 1, Some(0), false), now).is_none());
    let token = token_for(&mut h, addr(1), now);
    assert!(h.handle(addr(1), &request(0, 1, Some(token.wrapping_add(1)), false), now).is_none());
    // A token belongs to an address: replaying it from another address (a spoofed source) fails.
    assert!(h.handle(addr(2), &request(0, 1, Some(token), false), now).is_none());
    assert!(
        h.handle(SocketAddr::from(([203, 0, 113, 10], 1)), &request(0, 1, Some(token), false), now)
            .is_none()
    );
    // The legitimate owner still works.
    assert!(h.handle(addr(1), &request(0, 1, Some(token), false), now).is_some());
}

#[test]
fn tokens_expire_when_the_period_rolls_over() {
    let mut h = handler(&[]);
    let start = Instant::now();
    let token = token_for(&mut h, addr(1), start);
    assert!(
        h.handle(addr(1), &request(0, 1, Some(token), false), start + Duration::from_secs(29))
            .is_some()
    );
    assert!(
        h.handle(addr(1), &request(0, 1, Some(token), false), start + Duration::from_secs(31))
            .is_none()
    );
    // A fresh handshake gets a new token that works.
    let fresh = token_for(&mut h, addr(1), start + Duration::from_secs(32));
    assert!(
        h.handle(addr(1), &request(0, 1, Some(fresh), false), start + Duration::from_secs(33))
            .is_some()
    );
}

#[test]
fn malformed_datagrams_are_ignored() {
    let mut h = handler(&[]);
    let now = Instant::now();
    let token = token_for(&mut h, addr(1), now);
    for bad in [
        vec![],
        vec![0xFE],
        vec![0xFE, 0xFD, 9, 0, 0, 0],             // too short
        vec![0xFE, 0xFC, 9, 0, 0, 0, 1],          // wrong magic
        vec![0xFE, 0xFD, 7, 0, 0, 0, 1],          // unknown type
        vec![0xFE, 0xFD, 0, 0, 0, 0, 1],          // stat without a token
        vec![0xFE, 0xFD, 0, 0, 0, 0, 1, 1, 2, 3], // truncated token
    ] {
        assert!(h.handle(addr(1), &bad, now).is_none(), "{bad:02x?}");
    }

    // A valid token does not rescue a datagram with the wrong magic or an unknown type.
    let mut wrong_magic = request(0, 1, Some(token), false);
    wrong_magic[1] = 0xFC;
    assert!(h.handle(addr(1), &wrong_magic, now).is_none());
    let mut wrong_type = request(0, 1, Some(token), false);
    wrong_type[2] = 0x07;
    assert!(h.handle(addr(1), &wrong_type, now).is_none());
}

#[test]
fn a_flooding_address_is_cut_off_but_others_are_unaffected() {
    let mut h = handler(&[]);
    let now = Instant::now();
    let mut answered = 0;
    for _ in 0..2000 {
        if h.handle(addr(1), &request(9, 1, None, false), now).is_some() {
            answered += 1;
        }
    }
    assert_eq!(answered, 600, "the per-address limit applies");
    assert!(h.handle(addr(2), &request(9, 1, None, false), now).is_some());
    // The window resets with the token period.
    assert!(
        h.handle(addr(1), &request(9, 1, None, false), now + Duration::from_secs(31)).is_some()
    );
}

#[test]
fn an_unauthenticated_request_has_a_tiny_bounded_amplification_factor() {
    // The only reply an attacker with a spoofed source address can provoke is the handshake:
    // type (1) + session id (4) + a token of at most 10 decimal digits + NUL = 16 bytes for a
    // 7-byte request. The token is random, so check many of them rather than one lucky value.
    let mut h = handler(&[]);
    let req = request(9, 1, None, false);
    assert_eq!(req.len(), 7);
    let mut longest = 0;
    for i in 0..500u16 {
        let from = SocketAddr::from(([198, 51, 100, (i % 250) as u8], 1000 + i));
        let reply = h.handle(from, &req, Instant::now()).unwrap();
        assert!(reply.len() <= 16, "{} bytes", reply.len());
        longest = longest.max(reply.len());
    }
    assert!(longest >= 15, "tokens are large enough that both 9 and 10 digit lengths occur");
    assert!(longest * 10 <= req.len() * 10 * 3, "amplification stays under 3x");
}

#[test]
fn embedded_nul_bytes_cannot_corrupt_fields() {
    struct Evil;
    impl ServerInfo for Evil {
        fn motd(&self) -> String {
            "a\0b".into()
        }
        fn version(&self) -> String {
            "v".into()
        }
        fn map(&self) -> String {
            "w".into()
        }
        fn online(&self) -> u32 {
            0
        }
        fn max_players(&self) -> u32 {
            1
        }
        fn players(&self) -> Vec<String> {
            vec!["x\0y".into()]
        }
        fn host_port(&self) -> u16 {
            1
        }
        fn host_ip(&self) -> String {
            "i".into()
        }
    }
    let mut h = QueryHandler::new(Arc::new(Evil));
    let now = Instant::now();
    let token = token_for(&mut h, addr(1), now);
    assert_eq!(
        parse_basic(&h.handle(addr(1), &request(0, 1, Some(token), false), now).unwrap())
            .unwrap()
            .motd,
        "ab"
    );
    assert_eq!(
        parse_full(&h.handle(addr(1), &request(0, 1, Some(token), true), now).unwrap())
            .unwrap()
            .players,
        ["xy"]
    );
}

proptest! {
    #[test]
    fn arbitrary_datagrams_never_panic_and_never_amplify(data in proptest::collection::vec(any::<u8>(), 0..64)) {
        let mut h = handler(&["p"]);
        let _ = token_for(&mut h, addr(1), Instant::now());
        if let Some(reply) = h.handle(addr(1), &data, Instant::now()) {
            // Anything answered was a well-formed request.
            prop_assert!(data.len() >= 7 && data[0] == 0xFE && data[1] == 0xFD);
            prop_assert!(!reply.is_empty());
        }
    }

    #[test]
    fn replies_parse_back_for_any_server_strings(motd in "[ -~]{0,40}", map in "[ -~]{0,20}", n in 0u32..1000) {
        struct I { motd: String, map: String, n: u32 }
        impl ServerInfo for I {
            fn motd(&self) -> String { self.motd.clone() }
            fn version(&self) -> String { "v".into() }
            fn map(&self) -> String { self.map.clone() }
            fn online(&self) -> u32 { self.n }
            fn max_players(&self) -> u32 { 1000 }
            fn players(&self) -> Vec<String> { Vec::new() }
            fn host_port(&self) -> u16 { 1 }
            fn host_ip(&self) -> String { "i".into() }
        }
        let mut h = QueryHandler::new(Arc::new(I { motd: motd.clone(), map: map.clone(), n }));
        let now = Instant::now();
        let token = token_for(&mut h, addr(1), now);
        let basic = parse_basic(&h.handle(addr(1), &request(0, 1, Some(token), false), now).unwrap()).unwrap();
        prop_assert_eq!((basic.motd, basic.map, basic.online), (motd, map, n.to_string()));
    }
}

#[tokio::test]
async fn works_over_a_real_udp_socket() {
    let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(serve(server, Arc::new(Info { players: vec!["Alex".into()] })));

    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    client.connect(server_addr).await.unwrap();
    let mut buf = [0u8; 2048];

    client.send(&request(9, 1, None, false)).await.unwrap();
    let n =
        tokio::time::timeout(Duration::from_secs(2), client.recv(&mut buf)).await.unwrap().unwrap();
    let (_, token) = parse_handshake(&buf[..n]).unwrap();

    client.send(&request(0, 1, Some(token), true)).await.unwrap();
    let n =
        tokio::time::timeout(Duration::from_secs(2), client.recv(&mut buf)).await.unwrap().unwrap();
    assert_eq!(parse_full(&buf[..n]).unwrap().players, ["Alex"]);

    // A bad token gets silence, not an error reply.
    client.send(&request(0, 1, Some(token.wrapping_add(7)), false)).await.unwrap();
    assert!(tokio::time::timeout(Duration::from_millis(300), client.recv(&mut buf)).await.is_err());
}
