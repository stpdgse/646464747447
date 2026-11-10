//! Differential tests against Mojang's real server, using this crate's own client and parsers.
//! Run through `scripts/oracle.sh`, which starts a vanilla server with Query and RCON enabled
//! (RCON password `ferrite-oracle`).

use std::time::Duration;

use ferrite_remote::query::*;
use ferrite_remote::rcon::{Client, TYPE_RESPONSE};
use tokio::net::UdpSocket;

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("set {name} (see scripts/oracle.sh)"))
}

async fn udp_exchange(socket: &UdpSocket, datagram: &[u8]) -> Option<Vec<u8>> {
    socket.send(datagram).await.unwrap();
    let mut buf = [0u8; 4096];
    match tokio::time::timeout(Duration::from_millis(800), socket.recv(&mut buf)).await {
        Ok(Ok(n)) => Some(buf[..n].to_vec()),
        _ => None,
    }
}

async fn query_socket() -> UdpSocket {
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    socket.connect(env("FERRITE_ORACLE_QUERY")).await.unwrap();
    socket
}

#[tokio::test]
#[ignore = "needs a vanilla server: scripts/oracle.sh"]
async fn query_matches_vanilla() {
    let socket = query_socket().await;

    // Handshake: decimal token, and the session id is masked to the low nibbles.
    let reply = udp_exchange(&socket, &request(9, 0x7F7F_7F7F, None, false))
        .await
        .expect("handshake reply");
    let (session, token) = parse_handshake(&reply).expect("vanilla's handshake must parse");
    eprintln!("vanilla handshake: session {session:#x}, token {token}");
    assert_eq!(session, 0x7F7F_7F7F, "vanilla echoes the session id unchanged");

    // Basic stat.
    let reply =
        udp_exchange(&socket, &request(0, 1, Some(token), false)).await.expect("basic stat");
    let basic = parse_basic(&reply).expect("vanilla's basic stat must parse");
    eprintln!("vanilla basic stat: {basic:?}");
    assert_eq!(
        (
            basic.motd.as_str(),
            basic.gametype.as_str(),
            basic.map.as_str(),
            basic.online.as_str(),
            basic.max.as_str()
        ),
        ("ferrite oracle", "SMP", "world", "0", "4")
    );

    // Full stat: the same fields, in the same order, as ours.
    let reply = udp_exchange(&socket, &request(0, 1, Some(token), true)).await.expect("full stat");
    let full = parse_full(&reply).expect("vanilla's full stat must parse");
    eprintln!("vanilla full stat: {full:?}");
    let keys: Vec<&str> = full.fields.iter().map(|(k, _)| k.as_str()).collect();
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
    let value = |k: &str| full.fields.iter().find(|(key, _)| key == k).unwrap().1.clone();
    assert_eq!(value("game_id"), "MINECRAFT");
    assert_eq!(value("version"), ferrite_data_version());
    assert_eq!(value("plugins"), "", "the vanilla plugin list is empty");

    // The payload length decides basic versus full: exactly 8 bytes is full, other lengths are basic.
    for (extra, expect_full) in [
        (&[0u8; 4][..], true),
        (&[0xaa; 4][..], true),
        (&[0u8; 1][..], false),
        (&[0u8; 5][..], false),
        (&[0u8; 8][..], false),
    ] {
        let mut d = request(0, 1, Some(token), false);
        d.extend_from_slice(extra);
        let reply = udp_exchange(&socket, &d)
            .await
            .unwrap_or_else(|| panic!("no reply for {} extra bytes", extra.len()));
        assert_eq!(reply[5..].starts_with(b"splitnum"), expect_full, "{} extra bytes", extra.len());
    }

    // A wrong token is answered with silence, exactly like ours.
    assert!(
        udp_exchange(&socket, &request(0, 1, Some(token.wrapping_add(1)), false)).await.is_none()
    );
    // So is a stat request before any handshake from a fresh address.
    let fresh = query_socket().await;
    assert!(udp_exchange(&fresh, &request(0, 1, Some(token), false)).await.is_none());
}

fn ferrite_data_version() -> String {
    "26.3".to_owned()
}

#[tokio::test]
#[ignore = "needs a vanilla server: scripts/oracle.sh"]
async fn rcon_matches_vanilla() {
    let addr = env("FERRITE_ORACLE_RCON");

    let mut c = Client::connect(&addr).await.unwrap();
    assert!(!c.login("wrong-password").await.unwrap(), "a wrong password is answered with id -1");
    assert!(c.login("ferrite-oracle").await.unwrap());

    let list = c.command("list").await.unwrap();
    eprintln!("vanilla `list`: {list:?}");
    assert_eq!(list, "There are 0 of a max of 4 players online: ");

    let unknown = c.command("definitelynotacommand").await.unwrap();
    eprintln!("vanilla unknown command: {unknown:?}");
    assert!(unknown.starts_with("Unknown or incomplete command"));

    // An invalid request type is answered with "Unknown request <hex type>" under the same id.
    c.send(321, 200, b"").await.unwrap();
    let p = c.read().await.unwrap();
    assert_eq!(
        (p.id, p.kind, p.payload.as_slice()),
        (321, TYPE_RESPONSE, &b"Unknown request c8"[..])
    );

    // Long output is fragmented at exactly 4096 bytes.
    c.send(77, 2, b"help").await.unwrap();
    let mut sizes = Vec::new();
    loop {
        let p = c.read().await.unwrap();
        assert_eq!((p.id, p.kind), (77, TYPE_RESPONSE));
        sizes.push(p.payload.len());
        if p.payload.len() < 4096 {
            break;
        }
    }
    eprintln!("vanilla `help` came in {} fragment(s): {sizes:?}", sizes.len());
    assert!(sizes.iter().all(|&n| n <= 4096));

    // Commands need a login first.
    let mut anon = Client::connect(&addr).await.unwrap();
    anon.send(5, 2, b"list").await.unwrap();
    assert_eq!(
        anon.read().await.unwrap().id,
        -1,
        "an unauthenticated command is answered with id -1"
    );
}
