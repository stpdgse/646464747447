use std::collections::VecDeque;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

use ferrite_protocol::SessionError;
use ferrite_protocol::crypto::ServerKey;
use ferrite_protocol::packets::play::Disconnect;
use ferrite_protocol::session::{Event, ServerSession, SessionConfig, offline_uuid};
use ferrite_protocol::types::{GameProfile, LenVec, Str, Text};
use tokio::sync::oneshot;

use crate::auth::{AuthRequest, AuthResult, Authenticator};

/// Settings for the lobby.
#[derive(Clone, Debug)]
pub struct LobbyConfig {
    pub motd: String,
    pub max_players: u32,
    pub session: SessionConfig,
    /// A connection that sends nothing for this long is dropped (slow-loris protection).
    pub read_timeout: Duration,
    /// `Some` turns on online mode: encryption and Mojang authentication.
    pub online: Option<OnlineConfig>,
    /// The game. Without one, players who finish configuration are politely disconnected.
    pub play: Option<Arc<dyn crate::play::PlayHost>>,
}

/// Online-mode settings.
#[derive(Clone, Debug)]
pub struct OnlineConfig {
    pub key: Arc<ServerKey>,
    pub authenticator: Arc<dyn Authenticator>,
    /// Send the client's IP to Mojang so it can reject proxied connections (vanilla's
    /// `prevent-proxy-connections`).
    pub prevent_proxy_connections: bool,
    /// Upper bound on waiting for the authenticator, whatever it does.
    pub auth_timeout: Duration,
}

impl Default for LobbyConfig {
    fn default() -> Self {
        Self {
            motd: "A Ferrite server".into(),
            max_players: 20,
            session: SessionConfig { brand: "ferrite".into(), ..SessionConfig::default() },
            read_timeout: Duration::from_secs(15),
            online: None,
            play: None,
        }
    }
}

/// Counters, readable from any thread.
#[derive(Debug, Default)]
pub struct Stats {
    pub connections: AtomicU64,
    pub status_requests: AtomicU64,
    pub logins: AtomicU64,
    pub entered_play: AtomicU64,
    pub protocol_errors: AtomicU64,
    pub timeouts: AtomicU64,
    /// Logins refused because Mojang did not vouch for the player.
    pub auth_rejected: AtomicU64,
    /// Logins that failed because verification itself was unavailable.
    pub auth_unavailable: AtomicU64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatsSnapshot {
    pub connections: u64,
    pub status_requests: u64,
    pub logins: u64,
    pub entered_play: u64,
    pub protocol_errors: u64,
    pub timeouts: u64,
    pub auth_rejected: u64,
    pub auth_unavailable: u64,
}

impl Stats {
    pub fn snapshot(&self) -> StatsSnapshot {
        let r = Ordering::Relaxed;
        StatsSnapshot {
            connections: self.connections.load(r),
            status_requests: self.status_requests.load(r),
            logins: self.logins.load(r),
            entered_play: self.entered_play.load(r),
            protocol_errors: self.protocol_errors.load(r),
            timeouts: self.timeouts.load(r),
            auth_rejected: self.auth_rejected.load(r),
            auth_unavailable: self.auth_unavailable.load(r),
        }
    }
}

/// What the I/O loop should do after [`Lobby::drive`].
#[derive(Clone, Debug, PartialEq)]
pub enum Flow {
    /// Keep reading.
    Continue,
    /// Flush the pending output, then close the connection.
    Close,
    /// The player finished configuration: flush, then switch the connection to the play loop
    /// (see [`Lobby::enter_play`]).
    Play(Box<crate::play::PlayStart>),
    /// Flush, then wait for [`Lobby::start_authentication`] and hand the outcome to
    /// [`Lobby::finish_authentication`]. Input that arrived behind the packet that triggered this
    /// is kept and processed afterwards.
    Authenticate(AuthRequest),
}

/// Per-connection state.
#[derive(Debug)]
pub struct ConnState {
    pub session: ServerSession,
    protocol_version: i32,
    counted_online: bool,
    peer: Option<IpAddr>,
    login_name: Option<String>,
    profile: Option<GameProfile>,
    client: Option<ferrite_protocol::packets::configuration::ClientInformation>,
    /// Events that arrived in the same read as one that paused processing (see [`Flow`]).
    backlog: VecDeque<Event>,
}

/// Shared by every connection and every backend thread.
#[derive(Debug)]
pub struct Lobby {
    cfg: LobbyConfig,
    online: AtomicU32,
    pub stats: Stats,
}

impl Lobby {
    pub fn new(cfg: LobbyConfig) -> Self {
        Self { cfg, online: AtomicU32::new(0), stats: Stats::default() }
    }

    pub fn config(&self) -> &LobbyConfig {
        &self.cfg
    }

    pub fn online(&self) -> u32 {
        self.online.load(Ordering::Relaxed)
    }

    /// Call when a connection is accepted. `peer` is the client's address, if known.
    pub fn connect(&self, peer: Option<IpAddr>) -> ConnState {
        self.stats.connections.fetch_add(1, Ordering::Relaxed);
        ConnState {
            session: ServerSession::new(self.cfg.session.clone()),
            protocol_version: 0,
            counted_online: false,
            peer,
            login_name: None,
            profile: None,
            client: None,
            backlog: VecDeque::new(),
        }
    }

    /// How long a backend should wait for authentication before giving up.
    pub fn auth_timeout(&self) -> Duration {
        self.cfg.online.as_ref().map_or(Duration::from_secs(10), |o| o.auth_timeout)
    }

    /// Starts verifying a login. Only meaningful in online mode.
    pub fn start_authentication(&self, request: AuthRequest) -> oneshot::Receiver<AuthResult> {
        match &self.cfg.online {
            Some(online) => online.authenticator.authenticate(request),
            None => {
                let (tx, rx) = oneshot::channel();
                let _ = tx.send(Ok(None));
                rx
            }
        }
    }

    /// Completes a login after [`Flow::Authenticate`]. `result` is `None` if the authenticator
    /// did not answer in time.
    pub fn finish_authentication(
        &self,
        conn: &mut ConnState,
        result: Option<AuthResult>,
    ) -> Result<Flow, SessionError> {
        match result {
            Some(Ok(Some(profile))) => {
                self.admit(conn, profile)?;
                self.drive(conn)
            }
            Some(Ok(None)) => {
                self.stats.auth_rejected.fetch_add(1, Ordering::Relaxed);
                conn.session.disconnect_translate("multiplayer.disconnect.unverified_username")?;
                Ok(Flow::Close)
            }
            Some(Err(_)) | None => {
                self.stats.auth_unavailable.fetch_add(1, Ordering::Relaxed);
                conn.session.disconnect_translate("multiplayer.disconnect.authservers_down")?;
                Ok(Flow::Close)
            }
        }
    }

    /// Reserves a player slot and sends `login_finished`.
    fn admit(&self, conn: &mut ConnState, profile: GameProfile) -> Result<Flow, SessionError> {
        if !self.try_reserve_slot() {
            conn.session.disconnect("The server is full")?;
            return Ok(Flow::Close);
        }
        conn.counted_online = true;
        self.stats.logins.fetch_add(1, Ordering::Relaxed);
        conn.profile = Some(profile.clone());
        conn.session.accept_login(profile)?;
        Ok(Flow::Continue)
    }

    /// Hands a connection that reached [`Flow::Play`] to the game. Returns the codec to drive and
    /// the game's handle, plus the queue the game writes to; `None` if the game refused the player.
    pub fn enter_play(
        &self,
        conn: &mut ConnState,
        start: crate::play::PlayStart,
    ) -> Option<crate::play::PlayEntry> {
        let host = self.cfg.play.as_ref()?;
        let (sink, outbox, queued) = crate::play::PlayerSink::new();
        let handle = host.join(start, sink)?;
        // The session is finished; keep its codec state and leave a blank one behind.
        let session =
            std::mem::replace(&mut conn.session, ServerSession::new(self.cfg.session.clone()));
        let (dec, enc, cipher, out) = session.into_parts().ok()?;
        Some(crate::play::PlayEntry {
            conn: crate::play::PlayConn::from_parts(dec, enc, cipher, out),
            handle,
            outbox,
            queued,
        })
    }

    /// Call exactly once when a connection ends, however it ended.
    pub fn disconnect(&self, conn: &mut ConnState) {
        if std::mem::take(&mut conn.counted_online) {
            self.online.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// The server list JSON.
    pub fn status_json(&self) -> String {
        format!(
            "{{\"version\":{{\"name\":\"{}\",\"protocol\":{}}},\"players\":{{\"max\":{},\"online\":{}}},\"description\":{{\"text\":\"{}\"}}}}",
            ferrite_data::GAME_VERSION,
            ferrite_data::PROTOCOL_VERSION,
            self.cfg.max_players,
            self.online(),
            json_escape(&self.cfg.motd),
        )
    }

    /// Processes everything the session has buffered, answering what the protocol needs answered.
    ///
    /// On `Err` the connection is finished; flush `conn.session.pending_output()` (it may carry a
    /// disconnect message) and close.
    pub fn drive(&self, conn: &mut ConnState) -> Result<Flow, SessionError> {
        // Events left over from before the last pause come first, to keep packet order.
        if conn.backlog.is_empty() {
            match conn.session.poll_all() {
                Ok(events) => conn.backlog.extend(events),
                Err(e) => {
                    self.stats.protocol_errors.fetch_add(1, Ordering::Relaxed);
                    return Err(e);
                }
            }
        }
        while let Some(event) = conn.backlog.pop_front() {
            match self.handle(conn, event)? {
                Flow::Continue => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Continue)
    }

    fn handle(&self, conn: &mut ConnState, event: Event) -> Result<Flow, SessionError> {
        match event {
            Event::Handshake { protocol_version, .. } => conn.protocol_version = protocol_version,
            Event::StatusRequested => {
                self.stats.status_requests.fetch_add(1, Ordering::Relaxed);
                conn.session.send_status(&self.status_json())?;
            }
            Event::LoginStarted { name, .. } => {
                if conn.protocol_version != ferrite_data::PROTOCOL_VERSION {
                    conn.session.reject_incompatible()?;
                    return Ok(Flow::Close);
                }
                if !is_valid_player_name(&name) {
                    conn.session.disconnect("Invalid username")?;
                    return Ok(Flow::Close);
                }
                match &self.cfg.online {
                    // Online mode: encrypt first, authenticate once the client has answered. The
                    // slot is only taken after authentication, so unauthenticated connections
                    // cannot hold the server full.
                    Some(online) => {
                        conn.login_name = Some(name);
                        conn.session.begin_encryption(online.key.clone(), true)?;
                    }
                    None => {
                        let profile = GameProfile {
                            uuid: offline_uuid(&name),
                            name: Str::new(name),
                            properties: LenVec(Vec::new()),
                        };
                        return self.admit(conn, profile);
                    }
                }
            }
            Event::EncryptionEstablished { server_hash, authenticate } => {
                let name = conn.login_name.clone().unwrap_or_default();
                if authenticate {
                    let ip = match &self.cfg.online {
                        Some(o) if o.prevent_proxy_connections => conn.peer,
                        _ => None,
                    };
                    return Ok(Flow::Authenticate(AuthRequest { name, server_hash, ip }));
                }
                let profile = GameProfile {
                    uuid: offline_uuid(&name),
                    name: Str::new(name),
                    properties: LenVec(Vec::new()),
                };
                return self.admit(conn, profile);
            }
            // Offline mode never asks for encryption or plugin answers, so any such packet is
            // unsolicited.
            Event::Login(_) => {
                conn.session.disconnect("Unexpected login packet")?;
                return Ok(Flow::Close);
            }
            Event::Configuring => {}
            Event::Configuration(ferrite_protocol::packets::configuration::ServerboundConfiguration::ClientInformation(info)) => {
                conn.client = Some(info);
            }
            Event::Configuration(_) => {}
            Event::EnteredPlay => {
                self.stats.entered_play.fetch_add(1, Ordering::Relaxed);
                if self.cfg.play.is_some()
                    && let Some(profile) = conn.profile.clone()
                {
                    return Ok(Flow::Play(Box::new(crate::play::PlayStart {
                        profile,
                        client: conn.client.clone(),
                    })));
                }
                // No game is attached; end the connection politely.
                conn.session.send_play(&Disconnect {
                    reason: Text::plain("Ferrite does not implement gameplay yet"),
                })?;
                return Ok(Flow::Close);
            }
        }
        Ok(Flow::Continue)
    }

    fn try_reserve_slot(&self) -> bool {
        self.online
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < self.cfg.max_players).then_some(n + 1)
            })
            .is_ok()
    }
}

/// Vanilla's rule: 1 to 16 characters, all of them printable ASCII except space.
pub fn is_valid_player_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 16 && name.chars().all(|c| c > ' ' && c <= '~')
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_names_follow_vanilla_rules() {
        for ok in ["a", "Notch", "Valid_Name1", "0123456789abcdef", "~!@#$%^&*()"] {
            assert!(is_valid_player_name(ok), "{ok:?}");
        }
        for bad in ["", "has space", "tab\t", "0123456789abcdefg", "ünï", "\u{7f}", "a\nb", "\0"]
        {
            assert!(!is_valid_player_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn status_json_is_escaped_and_reflects_online_count() {
        let lobby = Lobby::new(LobbyConfig {
            motd: "a \"quoted\"\\ line\nbreak\u{1}".into(),
            max_players: 5,
            ..LobbyConfig::default()
        });
        assert!(lobby.try_reserve_slot());
        let json = lobby.status_json();
        assert!(json.contains("\"online\":1"), "{json}");
        assert!(json.contains("\"max\":5"));
        assert!(json.contains(r#""text":"a \"quoted\"\\ line\nbreak\u0001""#), "{json}");
    }

    #[test]
    fn slots_never_exceed_the_maximum_under_contention() {
        let lobby = std::sync::Arc::new(Lobby::new(LobbyConfig {
            max_players: 100,
            ..LobbyConfig::default()
        }));
        let granted: u32 = (0..8)
            .map(|_| {
                let lobby = lobby.clone();
                std::thread::spawn(move || {
                    (0..1000).filter(|_| lobby.try_reserve_slot()).count() as u32
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|t| t.join().unwrap())
            .sum();
        assert_eq!(granted, 100);
        assert_eq!(lobby.online(), 100);
    }

    #[test]
    fn disconnect_releases_exactly_once() {
        let lobby = Lobby::new(LobbyConfig::default());
        let mut conn = lobby.connect(None);
        assert!(lobby.try_reserve_slot());
        conn.counted_online = true;
        lobby.disconnect(&mut conn);
        lobby.disconnect(&mut conn); // a second call must not underflow the counter
        assert_eq!(lobby.online(), 0);
    }
}
