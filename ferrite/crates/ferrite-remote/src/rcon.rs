//! Source RCON over TCP.
//!
//! ```text
//! length: i32 LE (bytes that follow) | id: i32 LE | type: i32 LE | payload (NUL-terminated) | NUL
//! ```
//!
//! Type 3 logs in, type 2 runs a command (and is also the type of the login reply), type 0 carries
//! command output. A failed login or an unauthenticated command is answered with id `-1`.
//!
//! RCON is **unencrypted**; expose it only on trusted networks.

use std::io;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

pub const TYPE_RESPONSE: i32 = 0;
pub const TYPE_COMMAND: i32 = 2;
pub const TYPE_LOGIN: i32 = 3;

/// Smallest legal body: id + type + two NULs.
const MIN_BODY: usize = 10;
/// Largest packet body a client may send (vanilla's limit).
const MAX_BODY: usize = 1460;
/// Output is sent in fragments of at most this many bytes.
pub const FRAGMENT: usize = 4096;

/// Runs commands for RCON clients.
pub trait CommandRunner: Send + Sync + 'static {
    fn run(&self, command: &str) -> String;
}

#[derive(Clone, Debug)]
pub struct RconConfig {
    pub password: String,
    /// Wrong passwords tolerated per connection before it is closed.
    pub max_failures: u32,
    /// Pause after each wrong password, to make guessing slow.
    pub failure_delay: Duration,
    /// Idle connections are dropped after this long.
    pub idle_timeout: Duration,
}

impl RconConfig {
    /// Refuses an empty password, as vanilla does: an RCON without one is an open door.
    pub fn new(password: impl Into<String>) -> Result<Self, &'static str> {
        let password = password.into();
        if password.is_empty() {
            return Err("RCON requires a non-empty password");
        }
        Ok(Self {
            password,
            max_failures: 5,
            failure_delay: Duration::from_millis(500),
            idle_timeout: Duration::from_secs(120),
        })
    }
}

/// Accepts RCON connections until the task is dropped.
pub async fn serve(
    listener: TcpListener,
    cfg: Arc<RconConfig>,
    runner: Arc<dyn CommandRunner>,
) -> io::Result<()> {
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!("rcon accept failed: {e}");
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
        };
        let (cfg, runner) = (cfg.clone(), runner.clone());
        tokio::spawn(async move {
            if let Err(e) = handle(stream, &cfg, &*runner).await {
                tracing::debug!("rcon connection from {peer} ended: {e}");
            }
        });
    }
}

/// One decoded request.
#[derive(Debug, PartialEq, Eq)]
pub struct Packet {
    pub id: i32,
    pub kind: i32,
    pub payload: Vec<u8>,
}

/// Reads one packet, or `Ok(None)` on a clean EOF between packets.
pub async fn read_packet(stream: &mut TcpStream, max_body: usize) -> io::Result<Option<Packet>> {
    let mut len = [0u8; 4];
    match stream.read_exact(&mut len).await {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = i32::from_le_bytes(len);
    let bad = |m: &'static str| io::Error::new(io::ErrorKind::InvalidData, m);
    let len = usize::try_from(len).map_err(|_| bad("negative length"))?;
    if !(MIN_BODY..=max_body).contains(&len) {
        return Err(bad("packet length out of range"));
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).await?;
    if body[len - 2..] != [0, 0] {
        return Err(bad("payload is not NUL-terminated"));
    }
    Ok(Some(Packet {
        id: i32::from_le_bytes(body[0..4].try_into().unwrap()),
        kind: i32::from_le_bytes(body[4..8].try_into().unwrap()),
        payload: body[8..len - 2].to_vec(),
    }))
}

pub fn encode_packet(id: i32, kind: i32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 14);
    out.extend_from_slice(&((payload.len() + MIN_BODY) as i32).to_le_bytes());
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(payload);
    out.extend_from_slice(&[0, 0]);
    out
}

/// Splits output into fragments of at most [`FRAGMENT`] bytes without cutting a character; empty
/// output is one empty fragment, so the client always gets a reply.
pub fn fragments(output: &str) -> Vec<&str> {
    if output.is_empty() {
        return vec![""];
    }
    let mut parts = Vec::new();
    let mut rest = output;
    while !rest.is_empty() {
        let mut end = FRAGMENT.min(rest.len());
        while !rest.is_char_boundary(end) {
            end -= 1;
        }
        let (head, tail) = rest.split_at(end);
        parts.push(head);
        rest = tail;
    }
    parts
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    // Compare against the longer length so the loop count does not reveal the password length.
    let n = a.len().max(b.len());
    let mut diff = a.len() ^ b.len();
    for i in 0..n {
        diff |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
    }
    diff == 0
}

async fn handle(
    mut stream: TcpStream,
    cfg: &RconConfig,
    runner: &dyn CommandRunner,
) -> io::Result<()> {
    let _ = stream.set_nodelay(true);
    let mut authenticated = false;
    let mut failures = 0u32;
    loop {
        let packet = match timeout(cfg.idle_timeout, read_packet(&mut stream, MAX_BODY)).await {
            Ok(r) => r?,
            Err(_) => return Ok(()),
        };
        let Some(Packet { id, kind, payload }) = packet else { return Ok(()) };

        match kind {
            TYPE_LOGIN => {
                if constant_time_eq(&payload, cfg.password.as_bytes()) {
                    authenticated = true;
                    stream.write_all(&encode_packet(id, TYPE_COMMAND, b"")).await?;
                } else {
                    failures += 1;
                    stream.write_all(&encode_packet(-1, TYPE_COMMAND, b"")).await?;
                    if failures >= cfg.max_failures {
                        return Ok(());
                    }
                    tokio::time::sleep(cfg.failure_delay).await;
                }
            }
            TYPE_COMMAND if authenticated => {
                let output = runner.run(&String::from_utf8_lossy(&payload));
                let mut reply = Vec::new();
                for part in fragments(&output) {
                    reply.extend_from_slice(&encode_packet(id, TYPE_RESPONSE, part.as_bytes()));
                }
                stream.write_all(&reply).await?;
            }
            TYPE_COMMAND => {
                stream.write_all(&encode_packet(-1, TYPE_COMMAND, b"")).await?;
            }
            other => {
                let text = format!("Unknown request {other:x}");
                stream.write_all(&encode_packet(id, TYPE_RESPONSE, text.as_bytes())).await?;
            }
        }
    }
}

/// A small RCON client, used by the tests and the vanilla oracle.
#[derive(Debug)]
pub struct Client {
    stream: TcpStream,
    next_id: i32,
}

impl Client {
    pub async fn connect(addr: &str) -> io::Result<Self> {
        Ok(Self { stream: TcpStream::connect(addr).await?, next_id: 1 })
    }

    fn id(&mut self) -> i32 {
        self.next_id += 1;
        self.next_id
    }

    /// `Ok(true)` if the password was accepted.
    pub async fn login(&mut self, password: &str) -> io::Result<bool> {
        let id = self.id();
        self.stream.write_all(&encode_packet(id, TYPE_LOGIN, password.as_bytes())).await?;
        let reply = self.read().await?;
        Ok(reply.id == id)
    }

    /// Runs a command and gathers all fragments of its output.
    ///
    /// The end of the output is detected with a second request of an invalid type, which the
    /// server always answers. It must be sent only **after** the first reply has arrived: the
    /// vanilla server cannot cope with two packets arriving in one read and drops the connection.
    pub async fn command(&mut self, command: &str) -> io::Result<String> {
        let id = self.id();
        self.stream.write_all(&encode_packet(id, TYPE_COMMAND, command.as_bytes())).await?;
        let first = self.read().await?;
        if first.id == -1 {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "not authenticated"));
        }
        let mut output = first.payload;

        let sentinel = self.id();
        self.stream.write_all(&encode_packet(sentinel, 200, b"")).await?;
        loop {
            let p = self.read().await?;
            if p.id == sentinel {
                return Ok(String::from_utf8_lossy(&output).into_owned());
            }
            output.extend_from_slice(&p.payload);
        }
    }

    /// Sends a raw packet.
    pub async fn send(&mut self, id: i32, kind: i32, payload: &[u8]) -> io::Result<()> {
        self.stream.write_all(&encode_packet(id, kind, payload)).await
    }

    /// Reads one reply packet (client-side limit: far larger than a client may send).
    pub async fn read(&mut self) -> io::Result<Packet> {
        read_packet(&mut self.stream, 1 << 20).await?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "server closed the connection")
        })
    }
}
