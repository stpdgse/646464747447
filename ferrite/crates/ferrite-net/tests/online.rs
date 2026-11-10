//! Online-mode tests: the real key exchange, encryption and `MojangAuthenticator`, talking to an
//! in-process mock of Mojang's session server.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ferrite_net::auth::{
    AuthError, AuthRequest, Authenticator, MojangAuthenticator, MojangConfig, parse_profile,
};
use ferrite_net::{Lobby, LobbyConfig, OnlineConfig};
use ferrite_protocol::crypto::ServerKey;
use ferrite_swarm::join_online_once;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use uuid::Uuid;

// ---------------------------------------------------------------------------------------------
// A mock of sessionserver.mojang.com
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
enum Behavior {
    /// Behave like Mojang: 200 with the profile if the hash was registered, else 204.
    #[default]
    Mojang,
    Status(u16),
    Hang,
    Body(&'static str),
}

#[derive(Default)]
struct MockState {
    /// server hash -> (name, uuid), filled by the "client" side of a test.
    joined: HashMap<String, (String, Uuid)>,
    behavior: Behavior,
    /// Decoded query strings of every `hasJoined` call.
    queries: Vec<HashMap<String, String>>,
}

struct MockSession {
    addr: SocketAddr,
    state: Arc<Mutex<MockState>>,
}

impl MockSession {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let state = Arc::new(Mutex::new(MockState::default()));
        let shared = state.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else { return };
                tokio::spawn(handle_http(stream, shared.clone()));
            }
        });
        MockSession { addr, state }
    }

    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    fn register(&self, hash: &str, name: &str, uuid: Uuid) {
        self.state.lock().unwrap().joined.insert(hash.to_owned(), (name.to_owned(), uuid));
    }

    fn set(&self, behavior: Behavior) {
        self.state.lock().unwrap().behavior = behavior;
    }

    fn queries(&self) -> Vec<HashMap<String, String>> {
        self.state.lock().unwrap().queries.clone()
    }

    fn authenticator(&self, cfg: MojangConfig) -> Arc<MojangAuthenticator> {
        Arc::new(MojangAuthenticator::new(MojangConfig { base_url: self.url(), ..cfg }))
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap()
}

async fn handle_http(mut stream: TcpStream, state: Arc<Mutex<MockState>>) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 2048];
    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut chunk).await {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    let head = String::from_utf8_lossy(&buf).to_string();
    let target = head.split_whitespace().nth(1).unwrap_or("/").to_owned();
    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    assert_eq!(path, "/session/minecraft/hasJoined");
    let params: HashMap<String, String> = query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (percent_decode(k), percent_decode(v))
        })
        .collect();

    let (behavior, found) = {
        let mut st = state.lock().unwrap();
        st.queries.push(params.clone());
        let found = params
            .get("serverId")
            .and_then(|h| st.joined.get(h))
            .filter(|(n, _)| {
                Some(n.to_ascii_lowercase())
                    == params.get("username").map(|u| u.to_ascii_lowercase())
            })
            .cloned();
        (st.behavior.clone(), found)
    };

    let (status, body): (u16, String) = match behavior {
        Behavior::Hang => {
            tokio::time::sleep(Duration::from_secs(30)).await;
            return;
        }
        Behavior::Status(code) => (code, String::new()),
        Behavior::Body(b) => (200, b.to_owned()),
        Behavior::Mojang => match found {
            Some((name, uuid)) => (
                200,
                format!(
                    "{{\"id\":\"{}\",\"name\":\"{name}\",\"properties\":[{{\"name\":\"textures\",\"value\":\"dGV4\",\"signature\":\"c2ln\"}}]}}",
                    uuid.simple()
                ),
            ),
            None => (204, String::new()),
        },
    };
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        _ => "Error",
    };
    let reply = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(reply.as_bytes()).await;
    let _ = stream.shutdown().await;
}

// ---------------------------------------------------------------------------------------------
// Servers under test
// ---------------------------------------------------------------------------------------------

fn key() -> Arc<ServerKey> {
    use std::sync::OnceLock;
    static KEY: OnceLock<Arc<ServerKey>> = OnceLock::new();
    KEY.get_or_init(|| Arc::new(ServerKey::generate().unwrap())).clone()
}

fn online_cfg(authenticator: Arc<dyn Authenticator>) -> LobbyConfig {
    LobbyConfig {
        max_players: 1000,
        online: Some(OnlineConfig {
            key: key(),
            authenticator,
            prevent_proxy_connections: false,
            auth_timeout: Duration::from_secs(5),
        }),
        ..LobbyConfig::default()
    }
}

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

async fn wait_for(what: &str, mut cond: impl FnMut() -> bool) {
    for _ in 0..300 {
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("timed out waiting for {what}");
}

fn uuid(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

// ---------------------------------------------------------------------------------------------
// Scenarios, run against both backends
// ---------------------------------------------------------------------------------------------

async fn a_verified_player_joins_and_the_hashes_agree(start: impl AsyncFn(LobbyConfig) -> Running) {
    let mock = MockSession::start().await;
    let s = start(online_cfg(mock.authenticator(MojangConfig::default()))).await;

    let client_hash = Mutex::new(String::new());
    let outcome = join_online_once(s.addr, "Notch", |hash| {
        // What a real client does against Mojang's `join` endpoint, before the server asks.
        mock.register(hash, "Notch", uuid(0xabc));
        *client_hash.lock().unwrap() = hash.to_owned();
    })
    .await
    .unwrap();
    assert_eq!(outcome.first_play_packet, Some("disconnect"));
    assert_eq!(outcome.registries, ferrite_data::configuration::SYNCHRONIZED_REGISTRIES.len());

    // The server computed the very same hash independently from its side of the key exchange.
    let queries = mock.queries();
    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0]["username"], "Notch");
    assert_eq!(queries[0]["serverId"], *client_hash.lock().unwrap());
    assert!(!queries[0].contains_key("ip"), "the address is only sent when anti-proxy is on");

    wait_for("the slot to be released", || s.lobby.online() == 0).await;
    let st = s.lobby.stats.snapshot();
    assert_eq!((st.logins, st.entered_play, st.auth_rejected, st.auth_unavailable), (1, 1, 0, 0));
}

async fn an_impostor_is_refused_and_takes_no_slot(start: impl AsyncFn(LobbyConfig) -> Running) {
    let mock = MockSession::start().await;
    let s = start(online_cfg(mock.authenticator(MojangConfig::default()))).await;

    // The client never registers its hash with the session server: it cannot prove who it is.
    let err = join_online_once(s.addr, "Notch", |_| {}).await.unwrap_err().to_string();
    assert!(err.contains("multiplayer.disconnect.unverified_username"), "{err}");

    let st = s.lobby.stats.snapshot();
    assert_eq!((st.auth_rejected, st.logins, s.lobby.online()), (1, 0, 0));
    assert_eq!(mock.queries().len(), 1);

    // Registering the hash for a *different* name does not help either.
    let err = join_online_once(s.addr, "Victim", |h| mock.register(h, "Attacker", uuid(1)))
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("unverified_username"), "{err}");
}

async fn an_unavailable_session_server_is_not_blamed_on_the_player(
    start: impl AsyncFn(LobbyConfig) -> Running,
) {
    let mock = MockSession::start().await;
    mock.set(Behavior::Status(503));
    let s = start(online_cfg(mock.authenticator(MojangConfig::default()))).await;
    let err = join_online_once(s.addr, "Steve", |_| {}).await.unwrap_err().to_string();
    assert!(err.contains("multiplayer.disconnect.authservers_down"), "{err}");
    assert_eq!(s.lobby.stats.snapshot().auth_unavailable, 1);
    assert_eq!(s.lobby.online(), 0);
}

async fn a_hanging_session_server_times_out(start: impl AsyncFn(LobbyConfig) -> Running) {
    let mock = MockSession::start().await;
    mock.set(Behavior::Hang);
    // The authenticator waits far longer than the lobby does, so the lobby's own limit applies.
    let mut cfg = online_cfg(mock.authenticator(MojangConfig {
        timeout: Duration::from_secs(20),
        ..MojangConfig::default()
    }));
    cfg.online.as_mut().unwrap().auth_timeout = Duration::from_millis(300);
    let s = start(cfg).await;

    let started = std::time::Instant::now();
    let err = join_online_once(s.addr, "Steve", |_| {}).await.unwrap_err().to_string();
    assert!(err.contains("authservers_down"), "{err}");
    assert!(started.elapsed() < Duration::from_secs(5), "must not wait for the authenticator");
    assert_eq!(s.lobby.stats.snapshot().auth_unavailable, 1);
}

async fn slots_are_only_taken_after_authentication(start: impl AsyncFn(LobbyConfig) -> Running) {
    let mock = MockSession::start().await;
    mock.set(Behavior::Hang);
    let mut cfg = online_cfg(mock.authenticator(MojangConfig::default()));
    cfg.max_players = 1;
    cfg.online.as_mut().unwrap().auth_timeout = Duration::from_millis(800);
    let s = start(cfg).await;

    // Several connections sit in the middle of authentication; none of them holds a slot.
    let addr = s.addr;
    let pending: Vec<_> = (0..4)
        .map(|i| {
            tokio::spawn(async move { join_online_once(addr, &format!("Slow{i}"), |_| {}).await })
        })
        .collect();
    wait_for("all four to reach the session server", || mock.queries().len() == 4).await;
    assert_eq!(s.lobby.online(), 0, "unauthenticated connections must not occupy slots");
    for p in pending {
        assert!(p.await.unwrap().is_err());
    }

    // The single slot is still free for a real player.
    mock.set(Behavior::Mojang);
    assert!(join_online_once(s.addr, "Real", |h| mock.register(h, "Real", uuid(7))).await.is_ok());
}

async fn the_client_address_is_forwarded_only_when_anti_proxy_is_on(
    start: impl AsyncFn(LobbyConfig) -> Running,
) {
    let mock = MockSession::start().await;
    let mut cfg = online_cfg(mock.authenticator(MojangConfig::default()));
    cfg.online.as_mut().unwrap().prevent_proxy_connections = true;
    let s = start(cfg).await;
    join_online_once(s.addr, "Proxy", |h| mock.register(h, "Proxy", uuid(2))).await.unwrap();
    assert_eq!(mock.queries()[0]["ip"], "127.0.0.1");
}

async fn a_garbage_key_packet_closes_the_connection(start: impl AsyncFn(LobbyConfig) -> Running) {
    use ferrite_protocol::packets::login::{ClientboundLogin, Key, ServerboundHello};
    use ferrite_protocol::types::{ByteVec, Str};
    let mock = MockSession::start().await;
    let s = start(online_cfg(mock.authenticator(MojangConfig::default()))).await;

    let mut c = ferrite_swarm::Conn::connect(s.addr).await.unwrap();
    c.handshake(2).await.unwrap();
    c.send(&ServerboundHello { name: Str::new("Evil"), uuid: Uuid::nil() }).await.unwrap();
    let f = c.recv().await.unwrap();
    assert!(matches!(ClientboundLogin::decode(f.id, &f.body).unwrap(), ClientboundLogin::Hello(_)));
    c.send(&Key {
        shared_secret: ByteVec(vec![0xde; 128]),
        verify_token: ByteVec(vec![0xad; 128]),
    })
    .await
    .unwrap();
    // "Protocol error", then the connection ends; Mojang is never asked.
    let f = c.recv().await.unwrap();
    assert!(matches!(
        ClientboundLogin::decode(f.id, &f.body).unwrap(),
        ClientboundLogin::LoginDisconnect(_)
    ));
    wait_for("the error to be counted", || s.lobby.stats.snapshot().protocol_errors == 1).await;
    assert!(mock.queries().is_empty());
}

async fn many_concurrent_online_logins_succeed(start: impl AsyncFn(LobbyConfig) -> Running) {
    let mock = MockSession::start().await;
    let s = start(online_cfg(
        mock.authenticator(MojangConfig { workers: 16, ..MojangConfig::default() }),
    ))
    .await;
    let addr = s.addr;
    let mock_state = mock.state.clone();
    let tasks: Vec<_> = (0..120u128)
        .map(|i| {
            let mock_state = mock_state.clone();
            tokio::spawn(async move {
                let name = format!("P{i}");
                join_online_once(addr, &name.clone(), move |h| {
                    mock_state
                        .lock()
                        .unwrap()
                        .joined
                        .insert(h.to_owned(), (name, Uuid::from_u128(i)));
                })
                .await
            })
        })
        .collect();
    for t in tasks {
        t.await.unwrap().unwrap();
    }
    wait_for("all slots released", || s.lobby.online() == 0).await;
    let st = s.lobby.stats.snapshot();
    assert_eq!((st.logins, st.entered_play, st.auth_rejected), (120, 120, 0));
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
            t!(a_verified_player_joins_and_the_hashes_agree);
            t!(an_impostor_is_refused_and_takes_no_slot);
            t!(an_unavailable_session_server_is_not_blamed_on_the_player);
            t!(a_hanging_session_server_times_out);
            t!(slots_are_only_taken_after_authentication);
            t!(the_client_address_is_forwarded_only_when_anti_proxy_is_on);
            t!(a_garbage_key_packet_closes_the_connection);
            t!(many_concurrent_online_logins_succeed);
        }
    };
}

fn always() -> bool {
    true
}

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

// ---------------------------------------------------------------------------------------------
// The authenticator on its own
// ---------------------------------------------------------------------------------------------

fn request(name: &str, hash: &str) -> AuthRequest {
    AuthRequest { name: name.to_owned(), server_hash: hash.to_owned(), ip: None }
}

async fn ask(
    a: &MojangAuthenticator,
    name: &str,
    hash: &str,
) -> Result<Option<ferrite_protocol::types::GameProfile>, AuthError> {
    a.authenticate(request(name, hash)).await.expect("the authenticator always answers")
}

#[tokio::test]
async fn mojang_authenticator_maps_every_status_code() {
    let mock = MockSession::start().await;
    let auth = mock.authenticator(MojangConfig::default());

    mock.register("good-hash", "Notch", uuid(0x1234));
    let profile = ask(&auth, "notch", "good-hash").await.unwrap().expect("verified");
    assert_eq!(profile.uuid, uuid(0x1234));
    assert_eq!(profile.name.0, "Notch", "the name in the response is the authoritative spelling");
    assert_eq!(profile.properties.0.len(), 1);
    assert_eq!(profile.properties.0[0].signature.as_ref().unwrap().0, "c2ln");

    assert_eq!(ask(&auth, "Notch", "unknown-hash").await, Ok(None), "204 means not verified");

    for (code, unavailable) in
        [(429, true), (500, true), (502, true), (503, true), (403, false), (404, false)]
    {
        mock.set(Behavior::Status(code));
        let r = ask(&auth, "Notch", "good-hash").await;
        if unavailable {
            assert!(matches!(r, Err(AuthError::Unavailable(_))), "{code}: {r:?}");
        } else {
            assert!(matches!(r, Err(AuthError::Malformed(_))), "{code}: {r:?}");
        }
    }
}

#[tokio::test]
async fn hostile_session_server_responses_are_rejected() {
    let mock = MockSession::start().await;
    let auth = mock.authenticator(MojangConfig::default());
    for body in [
        "not json",
        "{}",
        r#"{"id":"zz","name":"Notch"}"#,
        r#"{"id":"00000000000000000000000000000001"}"#,
        r#"{"id":"00000000000000000000000000000001","name":"Someone"}"#,
        r#"{"id":"00000000000000000000000000000001","name":"Notch","properties":"x"}"#,
        r#"{"id":"00000000000000000000000000000001","name":"Notch","properties":[{"name":"a"}]}"#,
    ] {
        mock.set(Behavior::Body(body));
        let r = ask(&auth, "Notch", "h").await;
        assert!(matches!(r, Err(AuthError::Malformed(_))), "{body}: {r:?}");
    }
}

#[test]
fn profile_parsing_enforces_protocol_limits() {
    let id = "00000000000000000000000000000001";
    let many =
        (0..17).map(|i| format!(r#"{{"name":"p{i}","value":"v"}}"#)).collect::<Vec<_>>().join(",");
    assert!(
        parse_profile(&format!(r#"{{"id":"{id}","name":"Notch","properties":[{many}]}}"#), "Notch")
            .is_err()
    );
    let long_value = "x".repeat(32768);
    assert!(parse_profile(&format!(r#"{{"id":"{id}","name":"Notch","properties":[{{"name":"t","value":"{long_value}"}}]}}"#), "Notch").is_err());
    let long_sig = "x".repeat(1025);
    assert!(parse_profile(&format!(r#"{{"id":"{id}","name":"Notch","properties":[{{"name":"t","value":"v","signature":"{long_sig}"}}]}}"#), "Notch").is_err());
    assert!(
        parse_profile(&format!(r#"{{"id":"{id}","name":"{}"}}"#, "n".repeat(17)), &"n".repeat(17))
            .is_err(),
        "names are at most 16 characters"
    );
    // Dashes in the id are accepted by the UUID parser; Mojang sends them without.
    assert!(
        parse_profile(r#"{"id":"00000000-0000-0000-0000-000000000001","name":"Notch"}"#, "Notch")
            .is_ok()
    );
}

#[tokio::test]
async fn unusual_but_legal_usernames_survive_url_encoding() {
    let mock = MockSession::start().await;
    let auth = mock.authenticator(MojangConfig::default());
    for name in ["a&b=c", "x#y", "100%", "p+q", "~!@$^*()", "a/b?c"] {
        assert!(ferrite_net::is_valid_player_name(name), "{name:?} must be a legal name");
        mock.register("h", name, uuid(5));
        let r = ask(&auth, name, "h").await;
        assert!(matches!(r, Ok(Some(_))), "{name:?}: {r:?}");
        let q = mock.queries();
        assert_eq!(q.last().unwrap()["username"], name, "must round-trip through the query string");
    }
}

#[tokio::test]
async fn the_verification_queue_applies_backpressure() {
    let mock = MockSession::start().await;
    mock.set(Behavior::Hang);
    let auth = mock.authenticator(MojangConfig {
        workers: 1,
        queue_depth: 1,
        timeout: Duration::from_secs(20),
        ..MojangConfig::default()
    });

    // One request occupies the worker, one waits in the queue, the rest are refused immediately.
    let first = auth.authenticate(request("A", "h1"));
    wait_for("the worker to pick up the first request", || mock.queries().len() == 1).await;
    let second = auth.authenticate(request("B", "h2"));
    let third = tokio::time::timeout(Duration::from_secs(1), auth.authenticate(request("C", "h3")))
        .await
        .expect("a full queue must answer immediately, not block")
        .unwrap();
    assert!(matches!(third, Err(AuthError::Unavailable(m)) if m.contains("queue is full")));
    drop((first, second));
}

#[tokio::test]
async fn an_unreachable_session_server_is_unavailable_not_a_rejection() {
    let dead = {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        l.local_addr().unwrap()
    };
    let auth = MojangAuthenticator::new(MojangConfig {
        base_url: format!("http://{dead}"),
        ..MojangConfig::default()
    });
    let r = ask(&auth, "Notch", "h").await;
    assert!(matches!(r, Err(AuthError::Unavailable(_))), "{r:?}");
}
