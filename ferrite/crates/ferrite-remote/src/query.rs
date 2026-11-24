//! The Query protocol: a UDP request/response protocol for server information.
//!
//! A client first asks for a *challenge token* (type 9), then sends it back with a stat request
//! (type 0). The token is bound to the client's address and every token expires every 30 seconds,
//! which is what stops spoofed requests from turning the server into a traffic amplifier: the
//! only thing a spoofed source can get is the short handshake reply.

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;

pub const MAGIC: u16 = 0xFEFD;
const TYPE_STAT: u8 = 0;
const TYPE_HANDSHAKE: u8 = 9;
/// Tokens and rate-limit windows are discarded this often.
const ROTATE: Duration = Duration::from_secs(30);
/// Requests answered per address per rotation period before further ones are ignored.
const MAX_REQUESTS_PER_ROTATION: u32 = 600;

/// What the server tells queriers.
pub trait ServerInfo: Send + Sync + 'static {
    fn motd(&self) -> String;
    fn version(&self) -> String;
    /// Level name, `world` for vanilla defaults.
    fn map(&self) -> String;
    fn online(&self) -> u32;
    fn max_players(&self) -> u32;
    fn players(&self) -> Vec<String>;
    fn host_port(&self) -> u16;
    fn host_ip(&self) -> String;
}

/// Answers one datagram. Returns `None` when the request must be ignored (malformed, bad token,
/// or rate limited), because replying to those would help an attacker.
///
/// One deliberate difference from vanilla: a repeated handshake from the same address returns the
/// *same* token for the rest of the period, where vanilla issues a new one and invalidates the
/// old. Vanilla's behaviour lets anyone who can spoof a source address knock that address's
/// legitimate token out; clients cannot tell the difference.
pub struct QueryHandler {
    info: Arc<dyn ServerInfo>,
    tokens: HashMap<SocketAddr, i32>,
    seen: HashMap<SocketAddr, u32>,
    rotated: Instant,
}

impl std::fmt::Debug for QueryHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryHandler").field("tokens", &self.tokens.len()).finish_non_exhaustive()
    }
}

impl QueryHandler {
    pub fn new(info: Arc<dyn ServerInfo>) -> Self {
        Self { info, tokens: HashMap::new(), seen: HashMap::new(), rotated: Instant::now() }
    }

    pub fn handle(&mut self, from: SocketAddr, datagram: &[u8], now: Instant) -> Option<Vec<u8>> {
        if now.duration_since(self.rotated) >= ROTATE {
            self.tokens.clear();
            self.seen.clear();
            self.rotated = now;
        }
        // magic (2) + type (1) + session id (4)
        let (head, payload) = datagram.split_at_checked(7)?;
        if u16::from_be_bytes([head[0], head[1]]) != MAGIC {
            return None;
        }
        let count = self.seen.entry(from).or_insert(0);
        *count += 1;
        if *count > MAX_REQUESTS_PER_ROTATION {
            return None;
        }
        // Echoed verbatim, as vanilla does. (Vanilla corrupts session ids with a high bit set,
        // which is why clients are told to use only the low nibble of each byte; echoing the four
        // bytes unchanged is correct for every client that does.)
        let session = u32::from_be_bytes([head[3], head[4], head[5], head[6]]);

        match head[2] {
            TYPE_HANDSHAKE => {
                let token = self.tokens.get(&from).copied().unwrap_or_else(|| {
                    let mut b = [0u8; 4];
                    getrandom::fill(&mut b).expect("OS randomness");
                    // Non-negative so the decimal string round-trips through a signed parse.
                    let token = (i32::from_be_bytes(b) & 0x7FFF_FFFF).max(1);
                    self.tokens.insert(from, token);
                    token
                });
                let mut out = vec![TYPE_HANDSHAKE];
                out.extend_from_slice(&session.to_be_bytes());
                out.extend_from_slice(token.to_string().as_bytes());
                out.push(0);
                Some(out)
            }
            TYPE_STAT => {
                let token = i32::from_be_bytes(payload.get(..4)?.try_into().ok()?);
                if self.tokens.get(&from) != Some(&token) {
                    return None;
                }
                let mut out = vec![TYPE_STAT];
                out.extend_from_slice(&session.to_be_bytes());
                // Exactly like vanilla: a request padded to 8 bytes asks for the full stat, and
                // any other length (4 or more) for the basic one.
                if payload.len() == 8 {
                    self.full_stat(&mut out);
                } else {
                    self.basic_stat(&mut out);
                }
                Some(out)
            }
            _ => None,
        }
    }

    fn basic_stat(&self, out: &mut Vec<u8>) {
        let i = &self.info;
        for s in
            [i.motd(), "SMP".into(), i.map(), i.online().to_string(), i.max_players().to_string()]
        {
            push_str(out, &s);
        }
        out.extend_from_slice(&i.host_port().to_le_bytes());
        push_str(out, &i.host_ip());
    }

    fn full_stat(&self, out: &mut Vec<u8>) {
        let i = &self.info;
        out.extend_from_slice(b"splitnum\0\x80\0");
        for (k, v) in [
            ("hostname", i.motd()),
            ("gametype", "SMP".into()),
            ("game_id", "MINECRAFT".into()),
            ("version", i.version()),
            ("plugins", String::new()),
            ("map", i.map()),
            ("numplayers", i.online().to_string()),
            ("maxplayers", i.max_players().to_string()),
            ("hostport", i.host_port().to_string()),
            ("hostip", i.host_ip()),
        ] {
            push_str(out, k);
            push_str(out, &v);
        }
        out.push(0);
        out.extend_from_slice(b"\x01player_\0\0");
        for p in i.players() {
            push_str(out, &p);
        }
        out.push(0);
    }
}

fn push_str(out: &mut Vec<u8>, s: &str) {
    // The wire format is NUL-terminated, so an embedded NUL would corrupt the field.
    out.extend(s.bytes().filter(|&b| b != 0));
    out.push(0);
}

/// Serves Query on `socket` until the task is dropped.
pub async fn serve(socket: UdpSocket, info: Arc<dyn ServerInfo>) -> io::Result<()> {
    let mut handler = QueryHandler::new(info);
    // Requests are at most 15 bytes; anything larger is not ours to parse.
    let mut buf = [0u8; 64];
    loop {
        let (n, from) = socket.recv_from(&mut buf).await?;
        if let Some(reply) = handler.handle(from, &buf[..n], Instant::now()) {
            // A failed send (unreachable peer) must not stop the server.
            let _ = socket.send_to(&reply, from).await;
        }
    }
}

/// A parsed full-stat reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullStat {
    pub session: u32,
    pub fields: Vec<(String, String)>,
    pub players: Vec<String>,
}

/// A parsed basic-stat reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicStat {
    pub session: u32,
    pub motd: String,
    pub gametype: String,
    pub map: String,
    pub online: String,
    pub max: String,
    pub host_port: u16,
    pub host_ip: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum QueryError {
    #[error("malformed query reply: {0}")]
    Malformed(&'static str),
}

/// Builds the request datagram for `kind` (9 = handshake, 0 = stat).
pub fn request(kind: u8, session: u32, token: Option<i32>, full: bool) -> Vec<u8> {
    let mut out = MAGIC.to_be_bytes().to_vec();
    out.push(kind);
    out.extend_from_slice(&session.to_be_bytes());
    if let Some(t) = token {
        out.extend_from_slice(&t.to_be_bytes());
        if full {
            out.extend_from_slice(&[0; 4]);
        }
    }
    out
}

/// Parses a handshake reply into `(session, token)`.
pub fn parse_handshake(reply: &[u8]) -> Result<(u32, i32), QueryError> {
    let (head, rest) = reply.split_at_checked(5).ok_or(QueryError::Malformed("short"))?;
    if head[0] != TYPE_HANDSHAKE {
        return Err(QueryError::Malformed("not a handshake reply"));
    }
    let text = rest.strip_suffix(&[0]).ok_or(QueryError::Malformed("token not terminated"))?;
    let token = std::str::from_utf8(text)
        .ok()
        .and_then(|t| t.parse().ok())
        .ok_or(QueryError::Malformed("token is not a decimal integer"))?;
    Ok((u32::from_be_bytes(head[1..5].try_into().unwrap()), token))
}

fn cstr<'a>(buf: &mut &'a [u8]) -> Result<&'a str, QueryError> {
    let end =
        buf.iter().position(|&b| b == 0).ok_or(QueryError::Malformed("unterminated string"))?;
    let s = std::str::from_utf8(&buf[..end]).map_err(|_| QueryError::Malformed("not UTF-8"))?;
    *buf = &buf[end + 1..];
    Ok(s)
}

pub fn parse_basic(reply: &[u8]) -> Result<BasicStat, QueryError> {
    let (head, mut rest) = reply.split_at_checked(5).ok_or(QueryError::Malformed("short"))?;
    if head[0] != TYPE_STAT {
        return Err(QueryError::Malformed("not a stat reply"));
    }
    let session = u32::from_be_bytes(head[1..5].try_into().unwrap());
    let motd = cstr(&mut rest)?.to_owned();
    let gametype = cstr(&mut rest)?.to_owned();
    let map = cstr(&mut rest)?.to_owned();
    let online = cstr(&mut rest)?.to_owned();
    let max = cstr(&mut rest)?.to_owned();
    let (port, mut rest) = rest.split_at_checked(2).ok_or(QueryError::Malformed("no port"))?;
    let host_ip = cstr(&mut rest)?.to_owned();
    Ok(BasicStat {
        session,
        motd,
        gametype,
        map,
        online,
        max,
        host_port: u16::from_le_bytes([port[0], port[1]]),
        host_ip,
    })
}

pub fn parse_full(reply: &[u8]) -> Result<FullStat, QueryError> {
    let (head, rest) = reply.split_at_checked(5).ok_or(QueryError::Malformed("short"))?;
    if head[0] != TYPE_STAT {
        return Err(QueryError::Malformed("not a stat reply"));
    }
    let session = u32::from_be_bytes(head[1..5].try_into().unwrap());
    let mut rest = rest
        .strip_prefix(b"splitnum\0\x80\0")
        .ok_or(QueryError::Malformed("no splitnum padding"))?;
    let mut fields = Vec::new();
    loop {
        let key = cstr(&mut rest)?;
        if key.is_empty() {
            break;
        }
        fields.push((key.to_owned(), cstr(&mut rest)?.to_owned()));
    }
    let mut rest =
        rest.strip_prefix(b"\x01player_\0").ok_or(QueryError::Malformed("no player section"))?;
    // The padding is `01 player_ 00 00`: the final 00 is the first (empty) terminator.
    rest = rest.strip_prefix(&[0]).ok_or(QueryError::Malformed("bad player padding"))?;
    let mut players = Vec::new();
    loop {
        let name = cstr(&mut rest)?;
        if name.is_empty() {
            break;
        }
        players.push(name.to_owned());
    }
    Ok(FullStat { session, fields, players })
}
