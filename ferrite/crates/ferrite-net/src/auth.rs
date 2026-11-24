//! Online-mode authentication: asking Mojang's session server whether the client really is who it
//! claims to be.
//!
//! The server only learns that from the session server's `hasJoined` endpoint, which answers
//! `200` with the player's profile if (and only if) the client registered the same *server hash*
//! (see [`ferrite_protocol::crypto::server_hash`]) moments earlier, and `204` otherwise.
//!
//! Authentication is slow (a TLS round trip to a third party), so it must never run on a
//! connection-handling thread. [`Authenticator::authenticate`] returns a channel instead, which
//! both the tokio and the `io_uring` backend can await without caring which runtime answers.

use std::fmt;
use std::net::IpAddr;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ferrite_protocol::types::{GameProfile, LenVec, ProfileProperty, Str};
use thiserror::Error;
use tokio::sync::oneshot;
use uuid::Uuid;

/// What to verify.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthRequest {
    /// The name the client logged in with.
    pub name: String,
    /// The Minecraft-style server hash computed from the key exchange.
    pub server_hash: String,
    /// The client's address, sent to Mojang only when anti-proxy checking is on.
    pub ip: Option<IpAddr>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// The session server could not be reached or misbehaved. The player is not at fault.
    #[error("authentication service unavailable: {0}")]
    Unavailable(String),
    /// The response was not something a legitimate session server would send.
    #[error("malformed authentication response: {0}")]
    Malformed(String),
}

/// `Ok(Some(profile))`: verified. `Ok(None)`: the session server does not know this player.
pub type AuthResult = Result<Option<GameProfile>, AuthError>;

pub trait Authenticator: Send + Sync + fmt::Debug + 'static {
    /// Starts verifying and returns immediately; the result arrives on the channel.
    fn authenticate(&self, request: AuthRequest) -> oneshot::Receiver<AuthResult>;
}

/// Talks to Mojang's session server from a small pool of worker threads.
///
/// A bounded queue provides backpressure: if verification falls behind (Mojang is slow during an
/// outage), new logins are answered with [`AuthError::Unavailable`] immediately rather than
/// piling up memory and sockets without limit.
pub struct MojangAuthenticator {
    queue: SyncSender<Job>,
}

struct Job {
    request: AuthRequest,
    reply: oneshot::Sender<AuthResult>,
}

impl fmt::Debug for MojangAuthenticator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MojangAuthenticator { .. }")
    }
}

/// Settings for [`MojangAuthenticator`].
#[derive(Clone, Debug)]
pub struct MojangConfig {
    /// Base URL of the session server; replaced by a local mock in tests.
    pub base_url: String,
    /// Worker threads, i.e. how many verifications can be in flight.
    pub workers: usize,
    /// Verifications that may wait for a free worker.
    pub queue_depth: usize,
    /// Per-request limit, covering connect, TLS, request and response.
    pub timeout: Duration,
}

impl Default for MojangConfig {
    fn default() -> Self {
        Self {
            base_url: "https://sessionserver.mojang.com".into(),
            workers: 8,
            queue_depth: 256,
            timeout: Duration::from_secs(5),
        }
    }
}

impl MojangAuthenticator {
    pub fn new(cfg: MojangConfig) -> Self {
        let (tx, rx) = mpsc::sync_channel::<Job>(cfg.queue_depth);
        let rx = Arc::new(Mutex::new(rx));
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(cfg.timeout))
            // 204 and 4xx are answers, not transport errors; inspect the status ourselves.
            .http_status_as_error(false)
            .user_agent("Ferrite")
            .build()
            .into();
        for i in 0..cfg.workers.max(1) {
            let (rx, agent, base) = (rx.clone(), agent.clone(), cfg.base_url.clone());
            std::thread::Builder::new()
                .name(format!("ferrite-auth-{i}"))
                .spawn(move || worker(rx, agent, base))
                .expect("spawn authentication worker");
        }
        Self { queue: tx }
    }
}

fn worker(queue: Arc<Mutex<Receiver<Job>>>, agent: ureq::Agent, base: String) {
    loop {
        // Hold the lock only while waiting for a job, not while verifying it.
        let job = {
            let Ok(guard) = queue.lock() else { return };
            match guard.recv() {
                Ok(job) => job,
                Err(_) => return, // the authenticator was dropped
            }
        };
        let result = has_joined(&agent, &base, &job.request);
        let _ = job.reply.send(result); // the connection may have gone away meanwhile
    }
}

impl Authenticator for MojangAuthenticator {
    fn authenticate(&self, request: AuthRequest) -> oneshot::Receiver<AuthResult> {
        let (reply, rx) = oneshot::channel();
        match self.queue.try_send(Job { request, reply }) {
            Ok(()) => {}
            Err(TrySendError::Full(job)) => {
                let _ = job
                    .reply
                    .send(Err(AuthError::Unavailable("verification queue is full".into())));
            }
            Err(TrySendError::Disconnected(job)) => {
                let _ = job.reply.send(Err(AuthError::Unavailable("authenticator stopped".into())));
            }
        }
        rx
    }
}

fn has_joined(agent: &ureq::Agent, base: &str, request: &AuthRequest) -> AuthResult {
    let url = format!("{}/session/minecraft/hasJoined", base.trim_end_matches('/'));
    let mut call =
        agent.get(&url).query("username", &request.name).query("serverId", &request.server_hash);
    let ip;
    if let Some(addr) = request.ip {
        ip = addr.to_string();
        call = call.query("ip", &ip);
    }
    let mut response = call.call().map_err(|e| AuthError::Unavailable(e.to_string()))?;
    match response.status().as_u16() {
        200 => {
            // A profile is a few hundred bytes plus a signed textures blob; cap the read anyway.
            let body = response
                .body_mut()
                .with_config()
                .limit(128 * 1024)
                .read_to_string()
                .map_err(|e| AuthError::Unavailable(e.to_string()))?;
            parse_profile(&body, &request.name).map(Some)
        }
        204 => Ok(None),
        // 5xx and rate limiting mean "try again later", not "this player is an impostor".
        429 | 500..=599 => Err(AuthError::Unavailable(format!("HTTP {}", response.status()))),
        other => Err(AuthError::Malformed(format!("unexpected HTTP status {other}"))),
    }
}

/// Parses the `hasJoined` response, treating it as untrusted: bounded sizes, a valid UUID, and the
/// name must match the one the client logged in with (compared case-insensitively, as Mojang
/// does). The name in the response is the authoritative spelling.
pub fn parse_profile(body: &str, requested_name: &str) -> Result<GameProfile, AuthError> {
    let bad = |what: &str| AuthError::Malformed(what.to_owned());
    let json: serde_json::Value = serde_json::from_str(body).map_err(|_| bad("not JSON"))?;

    let id = json.get("id").and_then(|v| v.as_str()).ok_or_else(|| bad("missing id"))?;
    let uuid = Uuid::parse_str(id).map_err(|_| bad("id is not a UUID"))?;
    let name = json.get("name").and_then(|v| v.as_str()).ok_or_else(|| bad("missing name"))?;
    if !name.eq_ignore_ascii_case(requested_name) {
        return Err(bad("profile name does not match the login name"));
    }

    let mut properties = Vec::new();
    if let Some(list) = json.get("properties") {
        let list = list.as_array().ok_or_else(|| bad("properties is not an array"))?;
        if list.len() > 16 {
            return Err(bad("too many profile properties"));
        }
        for p in list {
            let text = |key: &str| p.get(key).and_then(|v| v.as_str());
            let (Some(pname), Some(value)) = (text("name"), text("value")) else {
                return Err(bad("property without name or value"));
            };
            if pname.chars().count() > 64 || value.chars().count() > 32767 {
                return Err(bad("property too large"));
            }
            let signature = match text("signature") {
                Some(sig) if sig.chars().count() > 1024 => return Err(bad("signature too large")),
                Some(sig) => Some(Str::<1024>::new(sig)),
                None => None,
            };
            properties.push(ProfileProperty {
                name: Str::<64>::new(pname),
                value: Str::<32767>::new(value),
                signature,
            });
        }
    }
    if name.len() > 16 {
        return Err(bad("name too long"));
    }
    Ok(GameProfile { uuid, name: Str::<16>::new(name), properties: LenVec(properties) })
}

/// A fixed table of players, for tests and development servers. It does **not** check the server
/// hash: pair it with a real flow only where that does not matter.
#[derive(Debug, Default)]
pub struct StaticAuthenticator {
    players: Mutex<Vec<(String, Uuid)>>,
    pub requests: Mutex<Vec<AuthRequest>>,
}

impl StaticAuthenticator {
    pub fn new(players: impl IntoIterator<Item = (String, Uuid)>) -> Self {
        Self { players: Mutex::new(players.into_iter().collect()), requests: Mutex::default() }
    }
}

impl Authenticator for StaticAuthenticator {
    fn authenticate(&self, request: AuthRequest) -> oneshot::Receiver<AuthResult> {
        let (tx, rx) = oneshot::channel();
        let found = self
            .players
            .lock()
            .unwrap()
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(&request.name))
            .map(|(n, id)| GameProfile {
                uuid: *id,
                name: Str::new(n.clone()),
                properties: LenVec(Vec::new()),
            });
        self.requests.lock().unwrap().push(request);
        let _ = tx.send(Ok(found));
        rx
    }
}
