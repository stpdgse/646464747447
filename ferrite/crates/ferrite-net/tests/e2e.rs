//! End-to-end tests: real sockets, real protocol, every scenario against every backend.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use ferrite_net::{Lobby, LobbyConfig};
use ferrite_protocol::codec::VarInt;
use ferrite_protocol::packets::handshake::Intention;
use ferrite_protocol::packets::login::{ClientboundLogin, LoginAcknowledged, ServerboundHello};
use ferrite_protocol::types::Str;
use ferrite_swarm::{Conn, Mode, RunConfig, join_once, run, status_once};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

/// A running server under test; dropping it stops it.
struct Running {
    addr: SocketAddr,
    lobby: Arc<Lobby>,
    _stop: Box<dyn std::any::Any + Send>,
}

async fn start_tokio(cfg: LobbyConfig) -> Running {
    let lobby = Arc::new(Lobby::new(cfg));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(ferrite_net::tokio_backend::serve(listener, lobby.clone(), async {
        let _ = rx.await;
    }));
    Running { addr, lobby, _stop: Box::new(tx) }
}

#[cfg(all(feature = "uring", target_os = "linux"))]
async fn start_uring(cfg: LobbyConfig) -> Running {
    struct Stop(Option<ferrite_net::uring_backend::Server>);
    impl Drop for Stop {
        fn drop(&mut self) {
            if let Some(s) = self.0.take() {
                let _ = s.shutdown();
            }
        }
    }
    let lobby = Arc::new(Lobby::new(cfg));
    let server =
        ferrite_net::uring_backend::serve("127.0.0.1:0".parse().unwrap(), 2, lobby.clone())
            .expect("io_uring must be available to run these tests");
    Running { addr: server.local_addr(), lobby, _stop: Box::new(Stop(Some(server))) }
}

fn cfg() -> LobbyConfig {
    LobbyConfig { max_players: 1000, ..LobbyConfig::default() }
}

async fn wait_for(what: &str, mut cond: impl FnMut() -> bool) {
    for _ in 0..200 {
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("timed out waiting for {what}");
}

async fn status_reports_motd_and_players(start: impl AsyncFn(LobbyConfig) -> Running) {
    let s = start(LobbyConfig { motd: "Hello \"world\"".into(), max_players: 7, ..cfg() }).await;
    let mut c = Conn::connect(s.addr).await.unwrap();
    c.send(&Intention {
        protocol_version: VarInt(777),
        server_address: Str::new("x"),
        server_port: 1,
        intent: VarInt(1),
    })
    .await
    .unwrap();
    c.send(&ferrite_protocol::packets::status::StatusRequest {}).await.unwrap();
    let f = c.recv().await.unwrap();
    let ferrite_protocol::packets::status::ClientboundStatus::StatusResponse(r) =
        ferrite_protocol::packets::status::ClientboundStatus::decode(f.id, &f.body).unwrap()
    else {
        panic!()
    };
    assert_eq!(
        r.json.0,
        "{\"version\":{\"name\":\"26.3\",\"protocol\":777},\"players\":{\"max\":7,\"online\":0},\"description\":{\"text\":\"Hello \\\"world\\\"\"}}"
    );
    assert!(status_once(s.addr).await.is_ok());
}

async fn join_reaches_play_with_all_registries(start: impl AsyncFn(LobbyConfig) -> Running) {
    let s = start(cfg()).await;
    let outcome = join_once(s.addr, "Steve").await.unwrap();
    assert_eq!(outcome.first_play_packet, Some("disconnect"));
    assert_eq!(outcome.registries, ferrite_data::configuration::SYNCHRONIZED_REGISTRIES.len());
    wait_for("the player to be released", || s.lobby.online() == 0).await;
    let stats = s.lobby.stats.snapshot();
    assert_eq!((stats.logins, stats.entered_play, stats.protocol_errors), (1, 1, 0));
}

async fn wrong_protocol_version_is_rejected_like_vanilla(
    start: impl AsyncFn(LobbyConfig) -> Running,
) {
    let s = start(cfg()).await;
    let mut c = Conn::connect(s.addr).await.unwrap();
    c.send(&Intention {
        protocol_version: VarInt(700),
        server_address: Str::new("x"),
        server_port: 1,
        intent: VarInt(2),
    })
    .await
    .unwrap();
    c.send(&ServerboundHello { name: Str::new("Old"), uuid: Uuid::nil() }).await.unwrap();
    let f = c.recv().await.unwrap();
    let ClientboundLogin::LoginDisconnect(d) = ClientboundLogin::decode(f.id, &f.body).unwrap()
    else {
        panic!()
    };
    assert_eq!(
        d.reason.0,
        "{\"translate\":\"multiplayer.disconnect.incompatible\",\"with\":[\"26.3\"]}"
    );
    assert!(matches!(c.recv().await, Err(ferrite_swarm::SwarmError::Closed)));
    assert_eq!(s.lobby.stats.snapshot().logins, 0);
}

async fn invalid_names_are_refused(start: impl AsyncFn(LobbyConfig) -> Running) {
    let s = start(cfg()).await;
    for bad in ["has space", "tab\t", "ünïcode", "\u{7f}del"] {
        let err = join_once(s.addr, bad).await.unwrap_err().to_string();
        assert!(err.contains("Invalid username"), "{bad:?}: {err}");
    }
    assert!(join_once(s.addr, "Valid_Name1").await.is_ok());
}

async fn a_full_server_refuses_and_frees_slots(start: impl AsyncFn(LobbyConfig) -> Running) {
    let s = start(LobbyConfig { max_players: 1, ..cfg() }).await;

    // The first player logs in and lingers in configuration, holding the only slot.
    let mut first = Conn::connect(s.addr).await.unwrap();
    first.login("First").await.unwrap();
    assert_eq!(s.lobby.online(), 1);

    let err = join_once(s.addr, "Second").await.unwrap_err().to_string();
    assert!(err.contains("The server is full"), "{err}");

    drop(first);
    wait_for("the slot to be freed", || s.lobby.online() == 0).await;
    assert!(join_once(s.addr, "Second").await.is_ok());
}

async fn garbage_closes_the_connection_and_is_counted(start: impl AsyncFn(LobbyConfig) -> Running) {
    let s = start(cfg()).await;
    let mut raw = TcpStream::connect(s.addr).await.unwrap();
    raw.write_all(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]).await.unwrap();
    let mut buf = [0u8; 64];
    let n = tokio::time::timeout(Duration::from_secs(5), raw.read(&mut buf))
        .await
        .unwrap()
        .unwrap_or(0);
    assert_eq!(n, 0, "the server must close without answering garbage");
    wait_for("the protocol error to be counted", || s.lobby.stats.snapshot().protocol_errors == 1)
        .await;
    // The server is still healthy.
    assert!(status_once(s.addr).await.is_ok());
}

async fn silent_connections_time_out(start: impl AsyncFn(LobbyConfig) -> Running) {
    let s = start(LobbyConfig { read_timeout: Duration::from_millis(200), ..cfg() }).await;
    let mut idle = TcpStream::connect(s.addr).await.unwrap();
    let mut buf = [0u8; 8];
    let n = tokio::time::timeout(Duration::from_secs(5), idle.read(&mut buf))
        .await
        .unwrap()
        .unwrap_or(0);
    assert_eq!(n, 0);
    wait_for("the timeout to be counted", || s.lobby.stats.snapshot().timeouts == 1).await;
}

async fn byte_at_a_time_delivery_works(start: impl AsyncFn(LobbyConfig) -> Running) {
    use bytes::BytesMut;
    use ferrite_protocol::{Encode, FrameEncoder, Packet};
    let s = start(cfg()).await;
    let mut enc = FrameEncoder::new();
    let mut wire = BytesMut::new();
    let mut body = Vec::new();
    Intention {
        protocol_version: VarInt(777),
        server_address: Str::new("slow"),
        server_port: 1,
        intent: VarInt(1),
    }
    .encode(&mut body)
    .unwrap();
    enc.encode(Intention::ID, &body, &mut wire).unwrap();
    enc.encode(ferrite_protocol::packets::status::StatusRequest::ID, &[], &mut wire).unwrap();

    let mut raw = TcpStream::connect(s.addr).await.unwrap();
    raw.set_nodelay(true).unwrap();
    for b in wire.iter() {
        raw.write_all(&[*b]).await.unwrap();
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    let mut buf = vec![0u8; 512];
    let n =
        tokio::time::timeout(Duration::from_secs(5), raw.read(&mut buf)).await.unwrap().unwrap();
    assert!(n > 20 && String::from_utf8_lossy(&buf[..n]).contains("\"protocol\":777"));
}

async fn disconnecting_mid_configuration_does_not_leak_a_slot(
    start: impl AsyncFn(LobbyConfig) -> Running,
) {
    let s = start(LobbyConfig { max_players: 3, ..cfg() }).await;
    for i in 0..10 {
        let mut c = Conn::connect(s.addr).await.unwrap();
        c.send(&Intention {
            protocol_version: VarInt(777),
            server_address: Str::new("x"),
            server_port: 1,
            intent: VarInt(2),
        })
        .await
        .unwrap();
        c.send(&ServerboundHello { name: Str::new(format!("Quit{i}")), uuid: Uuid::nil() })
            .await
            .unwrap();
        let _ = c.recv().await.unwrap(); // login_compression
        drop(c); // vanish without finishing
    }
    wait_for("all slots to be released", || s.lobby.online() == 0).await;
    assert!(join_once(s.addr, "After").await.is_ok());
}

async fn many_concurrent_joins_all_succeed(start: impl AsyncFn(LobbyConfig) -> Running) {
    let s = start(cfg()).await;
    let report = run(RunConfig {
        addr: s.addr,
        mode: Mode::Join,
        concurrency: 48,
        duration: Duration::from_secs(30),
        total: Some(600),
    })
    .await;
    assert_eq!((report.ok, report.failed), (600, 0), "{report}");
    wait_for("all players to be released", || s.lobby.online() == 0).await;
    assert_eq!(s.lobby.stats.snapshot().entered_play, 600);
}

async fn a_stalled_configuration_times_out_and_frees_its_slot(
    start: impl AsyncFn(LobbyConfig) -> Running,
) {
    // A client that logs in but never answers the known-packs request must not be let into play,
    // and must not hold its slot forever.
    let s =
        start(LobbyConfig { read_timeout: Duration::from_millis(300), max_players: 1, ..cfg() })
            .await;
    let mut c = Conn::connect(s.addr).await.unwrap();
    c.login("Mute").await.unwrap();
    c.send(&LoginAcknowledged {}).await.unwrap();
    assert_eq!(s.lobby.online(), 1);
    wait_for("the stall to time out", || s.lobby.stats.snapshot().timeouts == 1).await;
    wait_for("the slot to be freed", || s.lobby.online() == 0).await;
    assert_eq!(s.lobby.stats.snapshot().entered_play, 0);
}

fn always() -> bool {
    true
}

/// `io_uring` can be disabled by the kernel or a sandbox. Skip loudly when it is, unless the
/// environment insists on it (CI sets FERRITE_REQUIRE_URING=1 so a skip there is a failure).
#[cfg(all(feature = "uring", target_os = "linux"))]
fn uring_usable() -> bool {
    if ferrite_net::uring_backend::is_available() {
        return true;
    }
    assert!(
        std::env::var_os("FERRITE_REQUIRE_URING").is_none(),
        "io_uring is required (FERRITE_REQUIRE_URING is set) but unavailable"
    );
    eprintln!("SKIPPED: io_uring is unavailable on this kernel");
    false
}

macro_rules! backend_tests {
    ($module:ident, $start:path, $attr:meta, $usable:path) => {
        mod $module {
            use super::*;
            macro_rules! t {
                ($name:ident) => {
                    #[$attr]
                    async fn $name() {
                        if $usable() {
                            super::$name($start).await
                        }
                    }
                };
            }
            t!(status_reports_motd_and_players);
            t!(join_reaches_play_with_all_registries);
            t!(wrong_protocol_version_is_rejected_like_vanilla);
            t!(invalid_names_are_refused);
            t!(a_full_server_refuses_and_frees_slots);
            t!(garbage_closes_the_connection_and_is_counted);
            t!(silent_connections_time_out);
            t!(byte_at_a_time_delivery_works);
            t!(disconnecting_mid_configuration_does_not_leak_a_slot);
            t!(many_concurrent_joins_all_succeed);
            t!(a_stalled_configuration_times_out_and_frees_its_slot);
        }
    };
}

backend_tests!(
    tokio_backend,
    start_tokio,
    tokio::test(flavor = "multi_thread", worker_threads = 4),
    always
);
#[cfg(all(feature = "uring", target_os = "linux"))]
backend_tests!(
    uring_backend,
    start_uring,
    tokio::test(flavor = "multi_thread", worker_threads = 4),
    uring_usable
);
