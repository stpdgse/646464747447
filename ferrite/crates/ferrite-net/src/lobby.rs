use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

use ferrite_protocol::SessionError;
use ferrite_protocol::packets::play::Disconnect;
use ferrite_protocol::session::{Event, ServerSession, SessionConfig, offline_uuid};
use ferrite_protocol::types::{GameProfile, LenVec, Str, Text};

/// Settings for the lobby.
#[derive(Clone, Debug)]
pub struct LobbyConfig {
    pub motd: String,
    pub max_players: u32,
    pub session: SessionConfig,
    /// A connection that sends nothing for this long is dropped (slow-loris protection).
    pub read_timeout: Duration,
}

impl Default for LobbyConfig {
    fn default() -> Self {
        Self {
            motd: "A Ferrite server".into(),
            max_players: 20,
            session: SessionConfig { brand: "ferrite".into(), ..SessionConfig::default() },
            read_timeout: Duration::from_secs(15),
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
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatsSnapshot {
    pub connections: u64,
    pub status_requests: u64,
    pub logins: u64,
    pub entered_play: u64,
    pub protocol_errors: u64,
    pub timeouts: u64,
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
        }
    }
}

/// What the I/O loop should do after [`Lobby::drive`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    /// Keep reading.
    Continue,
    /// Flush the pending output, then close the connection.
    Close,
}

/// Per-connection state.
#[derive(Debug)]
pub struct ConnState {
    pub session: ServerSession,
    protocol_version: i32,
    counted_online: bool,
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

    /// Call when a connection is accepted.
    pub fn connect(&self) -> ConnState {
        self.stats.connections.fetch_add(1, Ordering::Relaxed);
        ConnState {
            session: ServerSession::new(self.cfg.session.clone()),
            protocol_version: 0,
            counted_online: false,
        }
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
        let events = match conn.session.poll_all() {
            Ok(e) => e,
            Err(e) => {
                self.stats.protocol_errors.fetch_add(1, Ordering::Relaxed);
                return Err(e);
            }
        };
        for event in events {
            if self.handle(conn, event)? == Flow::Close {
                return Ok(Flow::Close);
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
                if !self.try_reserve_slot() {
                    conn.session.disconnect("The server is full")?;
                    return Ok(Flow::Close);
                }
                conn.counted_online = true;
                self.stats.logins.fetch_add(1, Ordering::Relaxed);
                let profile = GameProfile {
                    uuid: offline_uuid(&name),
                    name: Str::new(name),
                    properties: LenVec(Vec::new()),
                };
                conn.session.accept_login(profile)?;
            }
            // Offline mode never asks for encryption or plugin answers, so any such packet is
            // unsolicited.
            Event::Login(_) => {
                conn.session.disconnect("Unexpected login packet")?;
                return Ok(Flow::Close);
            }
            Event::Configuring | Event::Configuration(_) => {}
            Event::EnteredPlay => {
                self.stats.entered_play.fetch_add(1, Ordering::Relaxed);
                // Gameplay does not exist yet; end the connection politely.
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
        let mut conn = lobby.connect();
        assert!(lobby.try_reserve_slot());
        conn.counted_online = true;
        lobby.disconnect(&mut conn);
        lobby.disconnect(&mut conn); // a second call must not underflow the counter
        assert_eq!(lobby.online(), 0);
    }
}
