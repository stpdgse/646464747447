use std::io::ErrorKind;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ferrite_remote::rcon::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

struct Runner(Mutex<Vec<String>>);
impl CommandRunner for Runner {
    fn run(&self, command: &str) -> String {
        self.0.lock().unwrap().push(command.to_owned());
        match command {
            "empty" => String::new(),
            "long" => "a".repeat(10_000),
            "unicode" => "é".repeat(3000),
            other => format!("ran: {other}"),
        }
    }
}

async fn start(cfg: RconConfig) -> (String, Arc<Runner>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let runner = Arc::new(Runner(Mutex::new(Vec::new())));
    tokio::spawn(serve(listener, Arc::new(cfg), runner.clone()));
    (addr, runner)
}

fn cfg() -> RconConfig {
    RconConfig { failure_delay: Duration::ZERO, ..RconConfig::new("hunter2").unwrap() }
}

#[test]
fn an_empty_password_is_refused() {
    assert!(RconConfig::new("").is_err());
    assert!(RconConfig::new("x").is_ok());
}

#[test]
fn packets_match_the_documented_layout() {
    // length 10 + 3, id 7, type 2, payload "abc", two NULs.
    assert_eq!(
        encode_packet(7, 2, b"abc"),
        [13, 0, 0, 0, 7, 0, 0, 0, 2, 0, 0, 0, b'a', b'b', b'c', 0, 0]
    );
    assert_eq!(encode_packet(-1, 2, b""), [10, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 2, 0, 0, 0, 0, 0]);
}

#[test]
fn output_is_fragmented_at_4096_bytes_without_splitting_characters() {
    assert_eq!(fragments("").len(), 1, "empty output still produces one reply");
    assert_eq!(fragments("abc"), ["abc"]);
    let long = "a".repeat(10_000);
    let parts = fragments(&long);
    assert_eq!(parts.iter().map(|p| p.len()).collect::<Vec<_>>(), [4096, 4096, 1808]);
    assert_eq!(parts.concat(), long);
    let accents = "é".repeat(3000); // 6000 bytes of 2-byte characters
    let parts = fragments(&accents);
    assert!(parts.iter().all(|p| p.len() <= 4096 && p.is_char_boundary(p.len())));
    assert_eq!(parts.concat(), accents);
    // Exactly one fragment when the output is exactly 4096 bytes.
    assert_eq!(fragments(&"a".repeat(4096)).len(), 1);
    assert_eq!(fragments(&"a".repeat(4097)).len(), 2);
}

#[tokio::test]
async fn login_and_commands() {
    let (addr, runner) = start(cfg()).await;
    let mut c = Client::connect(&addr).await.unwrap();
    assert!(!c.login("wrong").await.unwrap(), "a wrong password is answered with id -1");
    assert!(c.login("hunter2").await.unwrap());
    assert_eq!(c.command("list").await.unwrap(), "ran: list");
    assert_eq!(c.command("empty").await.unwrap(), "");
    assert_eq!(c.command("long").await.unwrap().len(), 10_000);
    assert_eq!(c.command("unicode").await.unwrap(), "é".repeat(3000));
    assert_eq!(*runner.0.lock().unwrap(), ["list", "empty", "long", "unicode"]);
}

#[tokio::test]
async fn nothing_runs_before_authentication() {
    let (addr, runner) = start(cfg()).await;
    let mut c = Client::connect(&addr).await.unwrap();
    let err = c.command("stop").await.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::PermissionDenied);
    assert!(
        runner.0.lock().unwrap().is_empty(),
        "an unauthenticated command must never reach the runner"
    );
    // A failed login does not authenticate either.
    assert!(!c.login("nope").await.unwrap());
    assert!(c.command("stop").await.is_err());
    assert!(runner.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn long_output_arrives_in_order_under_one_request_id() {
    let (addr, _) = start(cfg()).await;
    let mut c = Client::connect(&addr).await.unwrap();
    assert!(c.login("hunter2").await.unwrap());
    c.send(55, TYPE_COMMAND, b"long").await.unwrap();
    let mut sizes = Vec::new();
    while sizes.iter().sum::<usize>() < 10_000 {
        let p = c.read().await.unwrap();
        assert_eq!((p.id, p.kind), (55, TYPE_RESPONSE));
        sizes.push(p.payload.len());
    }
    assert_eq!(sizes, [4096, 4096, 1808]);
}

#[tokio::test]
async fn unknown_request_types_are_answered_like_vanilla() {
    let (addr, _) = start(cfg()).await;
    let mut c = Client::connect(&addr).await.unwrap();
    c.send(9, 200, b"").await.unwrap();
    let p = c.read().await.unwrap();
    assert_eq!((p.id, p.kind, p.payload.as_slice()), (9, 0, &b"Unknown request c8"[..]));
}

#[tokio::test]
async fn repeated_wrong_passwords_close_the_connection() {
    let (addr, _) = start(RconConfig { max_failures: 3, ..cfg() }).await;
    let mut c = Client::connect(&addr).await.unwrap();
    for _ in 0..3 {
        assert!(!c.login("bad").await.unwrap());
    }
    // The server hangs up after the third failure.
    assert!(c.login("hunter2").await.is_err(), "the connection must be closed");
}

#[tokio::test]
async fn failed_logins_are_slowed_down() {
    let (addr, _) = start(RconConfig { failure_delay: Duration::from_millis(150), ..cfg() }).await;
    let mut c = Client::connect(&addr).await.unwrap();
    let started = std::time::Instant::now();
    assert!(!c.login("a").await.unwrap());
    assert!(!c.login("b").await.unwrap());
    assert!(
        started.elapsed() >= Duration::from_millis(140),
        "the second attempt must wait for the delay"
    );
}

async fn raw(addr: &str, bytes: &[u8]) -> Vec<u8> {
    let mut s = TcpStream::connect(addr).await.unwrap();
    s.write_all(bytes).await.unwrap();
    let mut out = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(2), s.read_to_end(&mut out)).await;
    out
}

#[tokio::test]
async fn malformed_packets_close_the_connection_without_a_reply() {
    let (addr, runner) = start(cfg()).await;
    let mut oversized = ((1461i32).to_le_bytes()).to_vec();
    oversized.extend_from_slice(&[0; 1461]);
    for (name, bytes) in [
        ("negative length", (-5i32).to_le_bytes().to_vec()),
        ("length below the minimum", 9i32.to_le_bytes().to_vec()),
        ("length above the maximum", i32::MAX.to_le_bytes().to_vec()),
        ("oversized body", oversized),
        (
            "unterminated payload",
            [vec![12, 0, 0, 0], vec![1, 0, 0, 0, 2, 0, 0, 0], vec![b'a', b'b', b'c', b'd']]
                .concat(),
        ),
    ] {
        assert!(raw(&addr, &bytes).await.is_empty(), "{name}");
    }
    assert!(runner.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_huge_length_does_not_make_the_server_allocate_it() {
    // Claims ~2 GiB, sends nothing: must be rejected from the header alone.
    let (addr, _) = start(cfg()).await;
    let reply = raw(&addr, &(i32::MAX - 7).to_le_bytes()).await;
    assert!(reply.is_empty());
}

#[tokio::test]
async fn idle_connections_are_dropped() {
    let (addr, _) = start(RconConfig { idle_timeout: Duration::from_millis(200), ..cfg() }).await;
    let mut s = TcpStream::connect(&addr).await.unwrap();
    let mut buf = [0u8; 4];
    let n = tokio::time::timeout(Duration::from_secs(3), s.read(&mut buf)).await.unwrap().unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn many_clients_at_once() {
    let (addr, _) = start(cfg()).await;
    let tasks: Vec<_> = (0..40)
        .map(|i| {
            let addr = addr.clone();
            tokio::spawn(async move {
                let mut c = Client::connect(&addr).await.unwrap();
                assert!(c.login("hunter2").await.unwrap());
                assert_eq!(c.command(&format!("cmd{i}")).await.unwrap(), format!("ran: cmd{i}"));
            })
        })
        .collect();
    for t in tasks {
        t.await.unwrap();
    }
}
