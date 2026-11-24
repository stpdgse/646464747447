//! Runs the real `ferrite-server` executable and talks to it over the network.

use std::net::{SocketAddr, TcpListener as StdTcp, UdpSocket as StdUdp};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use ferrite_remote::query::{self, parse_basic, parse_full, parse_handshake};
use ferrite_remote::rcon;
use ferrite_swarm::{join_once, status_once};
use tokio::net::UdpSocket;

struct Server {
    child: Child,
    addr: SocketAddr,
    rcon: SocketAddr,
    query: SocketAddr,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_tcp() -> u16 {
    StdTcp::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}
fn free_udp() -> u16 {
    StdUdp::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

fn command(extra: &[&str]) -> (Command, SocketAddr, SocketAddr, SocketAddr) {
    let (game, rcon, query) = (free_tcp(), free_tcp(), free_udp());
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ferrite-server"));
    cmd.args([
        "--addr",
        &format!("127.0.0.1:{game}"),
        "--rcon-port",
        &rcon.to_string(),
        "--query-port",
        &query.to_string(),
        "--max-players",
        "7",
        "--motd",
        "Binary Test",
    ])
    .args(extra)
    .env("FERRITE_RCON_PASSWORD", "s3cret")
    .stdout(Stdio::null())
    .stderr(Stdio::piped());
    (
        cmd,
        format!("127.0.0.1:{game}").parse().unwrap(),
        format!("127.0.0.1:{rcon}").parse().unwrap(),
        format!("127.0.0.1:{query}").parse().unwrap(),
    )
}

/// Starts the server, or returns `None` (after saying so) if io_uring is unavailable and not
/// required.
fn start(extra: &[&str]) -> Option<Server> {
    let (mut cmd, addr, rcon, query) = command(extra);
    let mut child = cmd.spawn().expect("spawn ferrite-server");
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if std::net::TcpStream::connect(addr).is_ok() && std::net::TcpStream::connect(rcon).is_ok()
        {
            return Some(Server { child, addr, rcon, query });
        }
        if let Some(status) = child.try_wait().unwrap() {
            use std::io::Read;
            let mut err = String::new();
            child.stderr.take().unwrap().read_to_string(&mut err).unwrap();
            if err.contains("io_uring unavailable")
                && std::env::var_os("FERRITE_REQUIRE_URING").is_none()
            {
                eprintln!("SKIPPED: io_uring is unavailable on this kernel");
                return None;
            }
            panic!("the server exited with {status} before it was ready:\n{err}");
        }
        assert!(Instant::now() < deadline, "the server did not become ready in time");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// `first_play_packet`: what a joining client receives first: `login` where the game runs (the
/// tokio backend), `disconnect` where it does not yet (io_uring).
async fn exercise(extra: &[&str], first_play_packet: &str) {
    let Some(server) = start(extra) else { return };

    // Status and a full offline join.
    status_once(server.addr).await.unwrap();
    let joined = join_once(server.addr, "BinaryBot").await.unwrap();
    assert_eq!(joined.first_play_packet, Some(first_play_packet));

    // RCON.
    let mut c = rcon::Client::connect(&server.rcon.to_string()).await.unwrap();
    assert!(!c.login("wrong").await.unwrap());
    assert!(c.login("s3cret").await.unwrap());
    assert_eq!(c.command("list").await.unwrap(), "There are 0 of a max of 7 players online: ");
    assert_eq!(
        c.command("nonsense arg").await.unwrap(),
        "Unknown or incomplete command. See below for errornonsense arg<--[HERE]"
    );

    // Query.
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    socket.connect(server.query).await.unwrap();
    let mut buf = [0u8; 2048];
    socket.send(&query::request(9, 1, None, false)).await.unwrap();
    let n =
        tokio::time::timeout(Duration::from_secs(2), socket.recv(&mut buf)).await.unwrap().unwrap();
    let (_, token) = parse_handshake(&buf[..n]).unwrap();
    socket.send(&query::request(0, 1, Some(token), false)).await.unwrap();
    let n =
        tokio::time::timeout(Duration::from_secs(2), socket.recv(&mut buf)).await.unwrap().unwrap();
    let basic = parse_basic(&buf[..n]).unwrap();
    assert_eq!(
        (basic.motd.as_str(), basic.max.as_str(), basic.host_port),
        ("Binary Test", "7", server.addr.port())
    );
    socket.send(&query::request(0, 1, Some(token), true)).await.unwrap();
    let n =
        tokio::time::timeout(Duration::from_secs(2), socket.recv(&mut buf)).await.unwrap().unwrap();
    let full = parse_full(&buf[..n]).unwrap();
    assert!(full.fields.iter().any(|(k, v)| k == "version" && v == "26.3"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tokio_backend_serves_game_query_and_rcon() {
    exercise(&["--online-mode", "false", "--backend", "tokio"], "login").await;
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn uring_backend_serves_game_query_and_rcon() {
    exercise(&["--online-mode", "false", "--backend", "uring", "--threads", "2"], "disconnect")
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn online_mode_is_on_by_default() {
    // With no flag the server must ask clients to authenticate: an offline-style bot is stopped
    // at the encryption request, and nobody gets a slot.
    let server = start(&["--backend", "tokio"]).expect("tokio backend always starts");
    status_once(server.addr).await.unwrap();
    let err = join_once(server.addr, "Impostor").await.unwrap_err().to_string();
    assert!(err.contains("Hello"), "the server must send an encryption request first: {err}");

    let mut c = rcon::Client::connect(&server.rcon.to_string()).await.unwrap();
    assert!(c.login("s3cret").await.unwrap());
    assert_eq!(c.command("list").await.unwrap(), "There are 0 of a max of 7 players online: ");
}

#[test]
fn rcon_without_a_password_is_refused_at_startup() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ferrite-server"));
    let out = cmd
        .args([
            "--addr",
            &format!("127.0.0.1:{}", free_tcp()),
            "--online-mode",
            "false",
            "--rcon-port",
        ])
        .arg(free_tcp().to_string())
        .env_remove("FERRITE_RCON_PASSWORD")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert!(!out.status.success(), "an RCON without a password must not start");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("non-empty password"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_port_conflict_is_reported_not_swallowed() {
    let taken = StdTcp::bind("127.0.0.1:0").unwrap();
    let port = taken.local_addr().unwrap().port();
    let out = Command::new(env!("CARGO_BIN_EXE_ferrite-server"))
        .args([
            "--addr",
            &format!("127.0.0.1:{}", free_tcp()),
            "--online-mode",
            "false",
            "--rcon-port",
            &port.to_string(),
        ])
        .env("FERRITE_RCON_PASSWORD", "x")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert!(!out.status.success(), "binding an RCON port that is in use must fail loudly");
}
