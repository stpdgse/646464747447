use std::io;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use bytes::BytesMut;
use ferrite_data::{Direction, PROTOCOL_VERSION, State, packet_name};
use ferrite_protocol::cipher::Encryptor;
use ferrite_protocol::codec::VarInt;
use ferrite_protocol::crypto::{encrypt_with_public_key, server_hash};
use ferrite_protocol::packets::configuration::{
    AcknowledgeFinishConfiguration, ClientInformation, ClientboundConfiguration, Pong,
    ServerboundKeepAlive, ServerboundSelectKnownPacks,
};
use ferrite_protocol::packets::handshake::Intention;
use ferrite_protocol::packets::login::{
    ClientboundLogin, Key, LoginAcknowledged, ServerboundHello,
};
use ferrite_protocol::packets::status::{ClientboundStatus, PingRequest, StatusRequest};
use ferrite_protocol::session::core_pack;
use ferrite_protocol::types::{ByteVec, LenVec, Str};
use ferrite_protocol::{Compression, Frame, FrameDecoder, FrameEncoder, Packet, ProtocolError};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream};
use tokio::time::timeout;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum SwarmError {
    #[error("i/o: {0}")]
    Io(#[from] io::Error),
    #[error("protocol: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("timed out waiting for the server")]
    Timeout,
    #[error("server closed the connection")]
    Closed,
    #[error("{0}")]
    Unexpected(String),
}

type Result<T> = std::result::Result<T, SwarmError>;

/// How long a bot waits for any single server packet.
const STEP_TIMEOUT: Duration = Duration::from_secs(10);

/// One client connection: a socket plus the codec state.
#[derive(Debug)]
pub struct Conn {
    stream: TcpStream,
    dec: FrameDecoder,
    enc: FrameEncoder,
    cipher: Option<Encryptor>,
    scratch: Vec<u8>,
}

impl Conn {
    pub async fn connect(addr: SocketAddr) -> Result<Self> {
        let socket = if addr.is_ipv4() { TcpSocket::new_v4()? } else { TcpSocket::new_v6()? };
        // Close with an RST instead of lingering in TIME_WAIT: at tens of thousands of
        // connections per second the ephemeral port range would otherwise be exhausted.
        socket2::SockRef::from(&socket).set_linger(Some(Duration::ZERO))?;
        let stream = timeout(STEP_TIMEOUT, socket.connect(addr))
            .await
            .map_err(|_| SwarmError::Timeout)??;
        stream.set_nodelay(true)?;
        Ok(Conn {
            stream,
            // A client must accept anything a server may legitimately send.
            dec: FrameDecoder::new().with_limits(1 << 26, 1 << 26),
            enc: FrameEncoder::new(),
            cipher: None,
            scratch: vec![0; 16 * 1024],
        })
    }

    pub async fn send<P: Packet>(&mut self, packet: &P) -> Result<()> {
        let mut body = Vec::new();
        packet.encode(&mut body)?;
        let mut out = BytesMut::with_capacity(body.len() + 8);
        self.enc.encode(P::ID, &body, &mut out)?;
        if let Some(cipher) = &mut self.cipher {
            cipher.apply(&mut out);
        }
        self.stream.write_all(&out).await?;
        Ok(())
    }

    /// Sends a packet whose body is already encoded (used to answer packets generically).
    pub async fn send_raw(&mut self, id: i32, body: &[u8]) -> Result<()> {
        let mut out = BytesMut::with_capacity(body.len() + 8);
        self.enc.encode(id, body, &mut out)?;
        if let Some(cipher) = &mut self.cipher {
            cipher.apply(&mut out);
        }
        self.stream.write_all(&out).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<Frame> {
        loop {
            if let Some(frame) = self.dec.next_frame()? {
                return Ok(frame);
            }
            let n = timeout(STEP_TIMEOUT, self.stream.read(&mut self.scratch))
                .await
                .map_err(|_| SwarmError::Timeout)??;
            if n == 0 {
                return Err(SwarmError::Closed);
            }
            self.dec.queue(&self.scratch[..n]);
        }
    }

    fn enable_compression(&mut self, threshold: usize) -> Result<()> {
        let c = Some(Compression { threshold });
        self.dec.set_compression(c);
        self.enc.set_compression(c, 4)?;
        Ok(())
    }

    /// Sends the handshake. `intent`: 1 = status, 2 = login, 3 = transfer.
    pub async fn handshake(&mut self, intent: i32) -> Result<()> {
        self.send(&Intention {
            protocol_version: VarInt(PROTOCOL_VERSION),
            server_address: Str::new("ferrite-swarm"),
            server_port: 25565,
            intent: VarInt(intent),
        })
        .await
    }
}

impl Conn {
    /// Handshakes with the login intent, sends `hello`, enables compression if the server asks for
    /// it, and returns once the server has sent `login_finished`.
    pub async fn login(&mut self, name: &str) -> Result<()> {
        self.handshake(2).await?;
        self.send(&ServerboundHello { name: Str::new(name), uuid: Uuid::nil() }).await?;
        self.finish_login().await
    }

    /// Reads login packets up to `login_finished`, enabling compression when asked to.
    async fn finish_login(&mut self) -> Result<()> {
        loop {
            let frame = self.recv().await?;
            match ClientboundLogin::decode(frame.id, &frame.body)? {
                ClientboundLogin::LoginCompression(p) => {
                    self.enable_compression(p.threshold.0 as usize)?
                }
                ClientboundLogin::LoginFinished(_) => return Ok(()),
                ClientboundLogin::LoginDisconnect(d) => {
                    return Err(SwarmError::Unexpected(format!("login refused: {}", d.reason.0)));
                }
                other => return Err(SwarmError::Unexpected(format!("login: {other:?}"))),
            }
        }
    }
}

impl Conn {
    /// Like [`login`](Self::login) for an online-mode server: answers the encryption request and
    /// switches to AES-CFB8.
    ///
    /// `register_hash` receives the server hash, which a real client would send to Mojang's
    /// `join` endpoint before the server asks `hasJoined`. It runs *before* the key packet is
    /// sent, because the server verifies as soon as it has it.
    pub async fn login_online(
        &mut self,
        name: &str,
        register_hash: impl FnOnce(&str),
    ) -> Result<()> {
        self.handshake(2).await?;
        self.send(&ServerboundHello { name: Str::new(name), uuid: Uuid::nil() }).await?;

        let frame = self.recv().await?;
        let hello = match ClientboundLogin::decode(frame.id, &frame.body)? {
            ClientboundLogin::Hello(h) => h,
            ClientboundLogin::LoginDisconnect(d) => {
                return Err(SwarmError::Unexpected(format!("login refused: {}", d.reason.0)));
            }
            other => return Err(SwarmError::Unexpected(format!("expected hello: {other:?}"))),
        };

        let mut secret = [0u8; 16];
        getrandom::fill(&mut secret).map_err(|e| SwarmError::Unexpected(e.to_string()))?;
        let hash = server_hash(&hello.server_id.0, &secret, &hello.public_key.0);
        register_hash(&hash);

        let crypto =
            |e: ferrite_protocol::crypto::CryptoError| SwarmError::Unexpected(e.to_string());
        self.send(&Key {
            shared_secret: ByteVec(
                encrypt_with_public_key(&hello.public_key.0, &secret).map_err(crypto)?,
            ),
            verify_token: ByteVec(
                encrypt_with_public_key(&hello.public_key.0, &hello.verify_token.0)
                    .map_err(crypto)?,
            ),
        })
        .await?;
        // From here on both directions are encrypted.
        self.dec.enable_encryption(&secret);
        self.cipher = Some(Encryptor::new(&secret));

        self.finish_login().await
    }
}

/// Does one server-list ping on a fresh connection. Returns the whole round trip.
pub async fn status_once(addr: SocketAddr) -> Result<Duration> {
    let start = Instant::now();
    let mut conn = Conn::connect(addr).await?;
    conn.handshake(1).await?;
    conn.send(&StatusRequest {}).await?;
    let frame = conn.recv().await?;
    match ClientboundStatus::decode(frame.id, &frame.body)? {
        ClientboundStatus::StatusResponse(r) if r.json.contains("\"protocol\"") => {}
        other => return Err(SwarmError::Unexpected(format!("status reply: {other:?}"))),
    }
    let token = 0x5eed_0000_0000_0001u64 as i64;
    conn.send(&PingRequest { timestamp: token }).await?;
    let frame = conn.recv().await?;
    match ClientboundStatus::decode(frame.id, &frame.body)? {
        ClientboundStatus::PongResponse(p) if p.timestamp == token => {}
        other => return Err(SwarmError::Unexpected(format!("pong: {other:?}"))),
    }
    Ok(start.elapsed())
}

/// Opens one connection, reads the status once, then ping-pongs on it until `deadline`, calling
/// `record` with each round trip. This is the steady-state case: no connection churn, just small
/// requests and replies on long-lived sockets, like a game connection.
pub async fn ping_session(
    addr: SocketAddr,
    deadline: Instant,
    mut record: impl FnMut(Duration),
) -> Result<()> {
    let mut conn = Conn::connect(addr).await?;
    conn.handshake(1).await?;
    conn.send(&StatusRequest {}).await?;
    let frame = conn.recv().await?;
    if !matches!(
        ClientboundStatus::decode(frame.id, &frame.body)?,
        ClientboundStatus::StatusResponse(_)
    ) {
        return Err(SwarmError::Unexpected("expected a status response".into()));
    }
    let mut token = 0i64;
    while Instant::now() < deadline {
        token = token.wrapping_add(1);
        let start = Instant::now();
        conn.send(&PingRequest { timestamp: token }).await?;
        let frame = conn.recv().await?;
        match ClientboundStatus::decode(frame.id, &frame.body)? {
            ClientboundStatus::PongResponse(p) if p.timestamp == token => record(start.elapsed()),
            other => return Err(SwarmError::Unexpected(format!("pong: {other:?}"))),
        }
    }
    Ok(())
}

/// What a join attempt achieved.
#[derive(Clone, Debug)]
pub struct JoinOutcome {
    /// Time from `connect` to the first play packet.
    pub latency: Duration,
    /// Name of the first packet the server sent in the play state.
    pub first_play_packet: Option<&'static str>,
    /// Number of `registry_data` packets received during configuration.
    pub registries: usize,
}

/// Logs in as `name` and walks through configuration to the first play packet.
pub async fn join_once(addr: SocketAddr, name: &str) -> Result<JoinOutcome> {
    let start = Instant::now();
    let mut conn = Conn::connect(addr).await?;
    conn.login(name).await?;
    configure_and_enter(conn, start).await
}

/// [`join_once`] for an online-mode server. See [`Conn::login_online`] for `register_hash`.
pub async fn join_online_once(
    addr: SocketAddr,
    name: &str,
    register_hash: impl FnOnce(&str),
) -> Result<JoinOutcome> {
    let start = Instant::now();
    let mut conn = Conn::connect(addr).await?;
    conn.login_online(name, register_hash).await?;
    configure_and_enter(conn, start).await
}

/// One recorded clientbound play packet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Captured {
    pub id: i32,
    pub name: &'static str,
    pub body: Vec<u8>,
}

/// Joins `addr` as `name`, stays in the play state for `listen`, and returns every clientbound
/// play packet exactly as received. The bot behaves like a minimal client: it answers
/// keep-alives and pings, confirms teleports, acknowledges chunk batches and reports that it has
/// loaded, so the server keeps streaming the world to it.
pub async fn capture_play(addr: SocketAddr, name: &str, listen: Duration) -> Result<Vec<Captured>> {
    let mut conn = Conn::connect(addr).await?;
    conn.login(name).await?;
    configure(&mut conn).await?;
    let mut out = Vec::new();
    let deadline = Instant::now() + listen;
    let mut loaded_sent = false;
    while Instant::now() < deadline {
        let frame = match conn.recv().await {
            Ok(f) => f,
            Err(SwarmError::Timeout) => break,
            Err(e) => return Err(e),
        };
        let packet_name = packet_name(State::Play, Direction::Clientbound, frame.id).unwrap_or("?");
        match packet_name {
            "keep_alive" => {
                conn.send_raw(
                    ferrite_data::generated::packets::play::serverbound::KEEP_ALIVE,
                    &frame.body,
                )
                .await?
            }
            "ping" => {
                conn.send_raw(
                    ferrite_data::generated::packets::play::serverbound::PONG,
                    &frame.body,
                )
                .await?
            }
            "player_position" => {
                // player_position: id, x, y, z, dx, dy, dz, yRot, xRot, flags. The confirmation
                // echoes the id, the position and the rotation.
                let mut rest = &frame.body[..];
                let teleport = <VarInt as ferrite_protocol::codec::Decode>::decode(&mut rest)?;
                let mut id_only = Vec::new();
                ferrite_protocol::codec::Encode::encode(&teleport, &mut id_only)?;
                id_only.extend_from_slice(&rest[..24]); // x, y, z
                id_only.extend_from_slice(&rest[48..56]); // yRot, xRot
                conn.send_raw(
                    ferrite_data::generated::packets::play::serverbound::ACCEPT_TELEPORTATION,
                    &id_only,
                )
                .await?;
                if !loaded_sent {
                    conn.send_raw(
                        ferrite_data::generated::packets::play::serverbound::PLAYER_LOADED,
                        &[],
                    )
                    .await?;
                    loaded_sent = true;
                }
            }
            "chunk_batch_finished" => {
                conn.send_raw(
                    ferrite_data::generated::packets::play::serverbound::CHUNK_BATCH_RECEIVED,
                    &25.0f32.to_be_bytes(),
                )
                .await?
            }
            _ => {}
        }
        out.push(Captured { id: frame.id, name: packet_name, body: frame.body.to_vec() });
    }
    Ok(out)
}

/// Everything after `login_finished`: acknowledge, configure, and read the first play packet.
async fn configure_and_enter(mut conn: Conn, start: Instant) -> Result<JoinOutcome> {
    let registries = configure(&mut conn).await?;
    // The next packet belongs to the play state.
    let frame = conn.recv().await?;
    Ok(JoinOutcome {
        latency: start.elapsed(),
        first_play_packet: packet_name(State::Play, Direction::Clientbound, frame.id),
        registries,
    })
}

/// Acknowledges the login and walks through configuration; returns the number of registries seen.
async fn configure(conn: &mut Conn) -> Result<usize> {
    conn.send(&LoginAcknowledged {}).await?;
    conn.send(&ClientInformation {
        locale: Str::new("en_US"),
        view_distance: 2,
        chat_mode: VarInt(0),
        chat_colors: true,
        displayed_skin_parts: 0x7f,
        main_hand: VarInt(1),
        enable_text_filtering: false,
        allow_server_listings: true,
        particle_status: VarInt(0),
    })
    .await?;

    let mut registries = 0;
    loop {
        let frame = conn.recv().await?;
        match ClientboundConfiguration::decode(frame.id, &frame.body)? {
            ClientboundConfiguration::SelectKnownPacks(p) => {
                // Answer with what the server offered, as a vanilla client with the same pack does.
                let packs = if p.packs.0.contains(&core_pack()) { p.packs } else { LenVec(vec![]) };
                conn.send(&ServerboundSelectKnownPacks { packs }).await?;
            }
            ClientboundConfiguration::RegistryData(_) => registries += 1,
            ClientboundConfiguration::KeepAlive(p) => {
                conn.send(&ServerboundKeepAlive { id: p.id }).await?
            }
            ClientboundConfiguration::Ping(p) => conn.send(&Pong { id: p.id }).await?,
            ClientboundConfiguration::Disconnect(d) => {
                return Err(SwarmError::Unexpected(format!(
                    "kicked in configuration: {:?}",
                    d.reason
                )));
            }
            ClientboundConfiguration::FinishConfiguration(_) => {
                conn.send(&AcknowledgeFinishConfiguration {}).await?;
                break;
            }
            _ => {}
        }
    }

    Ok(registries)
}
