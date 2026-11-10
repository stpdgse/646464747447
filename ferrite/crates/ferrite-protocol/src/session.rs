//! The server side of a connection from the handshake to the start of play, as a sans-IO state
//! machine: bytes in, [`Event`]s and bytes out, no sockets, no clocks, no threads.
//!
//! ```text
//!           Intention                      StatusRequest -> PingRequest
//! Handshake ─────────┬──────────────────▶ Status ───────────────────────▶ (closed by client)
//!                    │ Login / Transfer
//!                    ▼
//!                  Login ── accept_login() ──▶ AwaitLoginAck ──▶ Configuration ──▶ Play
//! ```
//!
//! The session owns everything that changes how bytes are *interpreted* (packet ids per state,
//! compression, encryption) and the parts of the dance that are fixed by the protocol (known-pack
//! negotiation, registry and tag data, finishing configuration). Everything that is a *decision*
//! (the status text, whether a player may join, who they are) is surfaced as an [`Event`].

use bytes::{Buf, BytesMut};
use ferrite_data::PROTOCOL_VERSION;
use md5::{Digest, Md5};
use uuid::Uuid;

use crate::cipher::Encryptor;
use crate::codec::VarInt;
use crate::error::SessionError;
use crate::frame::{Compression, FrameDecoder, FrameEncoder};
use crate::packet::Packet;
use crate::packets::configuration::{self as cfg, ServerboundConfiguration};
use crate::packets::handshake::{Intent, ServerboundHandshake};
use crate::packets::login::{self, ServerboundLogin};
use crate::packets::status::{self as status, ServerboundStatus};
use crate::types::{GameProfile, Identifier, JsonText, LenVec, Remaining, Str, Text};

type Result<T, E = SessionError> = core::result::Result<T, E>;

/// Static settings for new sessions.
#[derive(Clone, Debug)]
pub struct SessionConfig {
    /// `None` disables compression; otherwise packets of at least this many bytes are compressed.
    pub compression: Option<Compression>,
    /// zlib level used for outgoing packets (1 fastest, 12 smallest).
    pub compression_level: i32,
    /// Reported to the client as the `minecraft:brand` plugin message.
    pub brand: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            compression: Some(Compression { threshold: 256 }),
            compression_level: 4,
            brand: "vanilla".into(),
        }
    }
}

/// Where the connection currently is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Handshake,
    Status,
    /// Waiting for `hello`.
    Login,
    /// `hello` received, waiting for [`ServerSession::accept_login`] or a disconnect.
    LoginPending,
    /// `login_finished` sent, waiting for `login_acknowledged`.
    AwaitLoginAck,
    /// Configuration. Waiting for the client's known packs, or for it to acknowledge the end.
    Configuration,
    /// Configuration is complete; the caller now owns the connection (see [`ServerSession::into_parts`]).
    Play,
    Closed,
}

impl Phase {
    fn name(self) -> &'static str {
        match self {
            Phase::Handshake => "handshake",
            Phase::Status => "status",
            Phase::Login => "login",
            Phase::LoginPending => "login (awaiting decision)",
            Phase::AwaitLoginAck => "login (awaiting acknowledgement)",
            Phase::Configuration => "configuration",
            Phase::Play => "play",
            Phase::Closed => "closed",
        }
    }
}

/// Something the application has to react to.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Handshake {
        protocol_version: i32,
        address: String,
        port: u16,
        intent: Intent,
    },
    /// Answer with [`ServerSession::send_status`].
    StatusRequested,
    /// The client wants to log in. Answer with [`ServerSession::accept_login`] or
    /// [`ServerSession::disconnect`].
    LoginStarted {
        name: String,
        uuid: Uuid,
        transfer: bool,
    },
    /// A login packet the session does not handle itself (encryption response, plugin answers,
    /// cookies).
    Login(ServerboundLogin),
    /// The client acknowledged login; the session has started the configuration sequence.
    Configuring,
    /// A configuration packet the session does not handle itself (client information, plugin
    /// messages, keep-alives, resource pack responses, ...).
    Configuration(ServerboundConfiguration),
    /// Configuration is finished and the client acknowledged it. The connection is now in play.
    EnteredPlay,
}

/// One end of a connection, driven by the caller's I/O loop.
#[derive(Debug)]
pub struct ServerSession {
    cfg: SessionConfig,
    phase: Phase,
    dec: FrameDecoder,
    enc: FrameEncoder,
    cipher: Option<Encryptor>,
    out: BytesMut,
    status_requested: bool,
    sent_known_packs: bool,
    finish_sent: bool,
    transfer: bool,
}

impl ServerSession {
    pub fn new(cfg: SessionConfig) -> Self {
        Self {
            cfg,
            phase: Phase::Handshake,
            dec: FrameDecoder::new(),
            enc: FrameEncoder::new(),
            cipher: None,
            out: BytesMut::new(),
            status_requested: false,
            sent_known_packs: false,
            finish_sent: false,
            transfer: false,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Hands received bytes to the session (decrypting them if encryption is on).
    pub fn feed(&mut self, data: &[u8]) {
        self.dec.queue(data);
    }

    /// Zero-copy variant of [`feed`](Self::feed): read from the socket into this buffer, then call
    /// [`commit`](Self::commit).
    pub fn recv_buffer(&mut self) -> &mut BytesMut {
        self.dec.buffer_mut()
    }

    pub fn commit(&mut self) {
        self.dec.commit();
    }

    /// Bytes that must be written to the socket, already framed, compressed and encrypted.
    pub fn pending_output(&self) -> &[u8] {
        &self.out
    }

    /// Marks `n` bytes of [`pending_output`](Self::pending_output) as written.
    pub fn consume_output(&mut self, n: usize) {
        self.out.advance(n);
    }

    /// Takes all pending output.
    pub fn take_output(&mut self) -> BytesMut {
        self.out.split()
    }

    /// Turns on AES-CFB8 in both directions. Everything sent or received after this call is
    /// encrypted; call it right after sending the packet that precedes encryption.
    pub fn enable_encryption(&mut self, shared_secret: &[u8; 16]) {
        self.dec.enable_encryption(shared_secret);
        // Bytes already queued in the output buffer were framed before this point and are sent in
        // the clear; only packets sent from now on are encrypted.
        self.cipher = Some(Encryptor::new(shared_secret));
    }

    /// Splits the finished connection into the decoder/encoder and any leftover state, so the play
    /// loop can take over. Only valid in [`Phase::Play`].
    pub fn into_parts(self) -> Result<(FrameDecoder, FrameEncoder, Option<Encryptor>, BytesMut)> {
        if self.phase != Phase::Play {
            return Err(SessionError::WrongPhase { phase: self.phase.name() });
        }
        Ok((self.dec, self.enc, self.cipher, self.out))
    }

    // ----- outgoing -----

    fn send<P: Packet>(&mut self, packet: &P) -> Result<()> {
        let mut body = Vec::new();
        packet.encode(&mut body)?;
        let start = self.out.len();
        self.enc.encode(P::ID, &body, &mut self.out)?;
        if let Some(cipher) = &mut self.cipher {
            cipher.apply(&mut self.out[start..]);
        }
        Ok(())
    }

    /// Answers [`Event::StatusRequested`] with the server list JSON.
    pub fn send_status(&mut self, json: &str) -> Result<()> {
        if self.phase != Phase::Status {
            return Err(SessionError::WrongPhase { phase: self.phase.name() });
        }
        self.send(&status::StatusResponse { json: Str::new(json) })
    }

    /// Lets the player in: negotiates compression, then sends `login_finished`.
    pub fn accept_login(&mut self, profile: GameProfile) -> Result<()> {
        if self.phase != Phase::LoginPending {
            return Err(SessionError::WrongPhase { phase: self.phase.name() });
        }
        if let Some(c) = self.cfg.compression {
            self.send(&login::LoginCompression { threshold: VarInt(c.threshold as i32) })?;
            self.enc.set_compression(Some(c), self.cfg.compression_level)?;
            self.dec.set_compression(Some(c));
        }
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|_| SessionError::Random)?;
        let session_id = uuid::Builder::from_random_bytes(random).into_uuid();
        self.send(&login::LoginFinished { profile, session_id })?;
        self.phase = Phase::AwaitLoginAck;
        Ok(())
    }

    /// Sends a play-state packet. Only valid once the session has reached [`Phase::Play`] (it keeps
    /// the encoder, compression and encryption state until [`into_parts`](Self::into_parts)).
    pub fn send_play<P: Packet>(&mut self, packet: &P) -> Result<()> {
        if self.phase != Phase::Play || P::STATE != ferrite_data::State::Play {
            return Err(SessionError::WrongPhase { phase: self.phase.name() });
        }
        self.send(packet)
    }

    /// Kicks the client with a message appropriate to the current phase and closes the session.
    /// Only the login and configuration phases have a disconnect packet; in any other phase the
    /// session just closes.
    pub fn disconnect(&mut self, message: &str) -> Result<()> {
        match self.phase {
            Phase::Login | Phase::LoginPending | Phase::AwaitLoginAck => {
                self.send(&login::LoginDisconnect { reason: JsonText::plain(message) })?;
            }
            Phase::Configuration => {
                self.send(&cfg::Disconnect { reason: Text::plain(message) })?;
            }
            _ => {}
        }
        self.phase = Phase::Closed;
        Ok(())
    }

    /// Rejects a client whose protocol version this server does not speak, with the message
    /// vanilla shows ("Incompatible client! Please use 26.3").
    pub fn reject_incompatible(&mut self) -> Result<()> {
        let json = format!(
            "{{\"translate\":\"multiplayer.disconnect.incompatible\",\"with\":[\"{}\"]}}",
            ferrite_data::GAME_VERSION
        );
        match self.phase {
            Phase::Login | Phase::LoginPending | Phase::AwaitLoginAck => {
                self.send(&login::LoginDisconnect { reason: JsonText(json) })?;
                self.phase = Phase::Closed;
                Ok(())
            }
            _ => Err(SessionError::WrongPhase { phase: self.phase.name() }),
        }
    }

    // ----- incoming -----

    /// Processes at most one received packet. `Ok(None)` means more bytes are needed or the packet
    /// was handled internally; call again until the buffered input is exhausted
    /// (see [`poll_all`](Self::poll_all)).
    ///
    /// An error means the stream is hostile or corrupt: the session is closed and the connection
    /// should be dropped. Flush [`pending_output`](Self::pending_output) first, since it may
    /// already hold a disconnect message explaining why.
    pub fn poll(&mut self) -> Result<Option<Event>> {
        if matches!(self.phase, Phase::Closed | Phase::Play) {
            return Ok(None);
        }
        let result = self.poll_inner();
        if result.is_err() {
            self.phase = Phase::Closed;
        }
        result
    }

    /// Drains the decoder, returning every event produced.
    pub fn poll_all(&mut self) -> Result<Vec<Event>> {
        let mut events = Vec::new();
        loop {
            let before = self.dec.buffered();
            match self.poll()? {
                Some(e) => events.push(e),
                None if self.dec.buffered() < before => {} // consumed a packet we handled ourselves
                None => return Ok(events),
            }
            if matches!(self.phase, Phase::Closed | Phase::Play) {
                return Ok(events);
            }
        }
    }

    fn poll_inner(&mut self) -> Result<Option<Event>> {
        let Some(frame) = self.dec.next_frame()? else { return Ok(None) };
        match self.phase {
            Phase::Handshake => {
                self.on_handshake(ServerboundHandshake::decode(frame.id, &frame.body)?)
            }
            Phase::Status => self.on_status(ServerboundStatus::decode(frame.id, &frame.body)?),
            Phase::Login | Phase::LoginPending => {
                self.on_login(ServerboundLogin::decode(frame.id, &frame.body)?)
            }
            Phase::AwaitLoginAck => match ServerboundLogin::decode(frame.id, &frame.body)? {
                ServerboundLogin::LoginAcknowledged(_) => self.begin_configuration(),
                other => Err(SessionError::Unexpected {
                    packet: other_name_login(&other),
                    phase: self.phase.name(),
                }),
            },
            Phase::Configuration => {
                self.on_configuration(ServerboundConfiguration::decode(frame.id, &frame.body)?)
            }
            Phase::Play | Phase::Closed => {
                Err(SessionError::Unexpected { packet: "any", phase: self.phase.name() })
            }
        }
    }

    fn on_handshake(&mut self, msg: ServerboundHandshake) -> Result<Option<Event>> {
        let ServerboundHandshake::Intention(p) = msg;
        let intent = p.intent()?;
        self.transfer = intent == Intent::Transfer;
        self.phase = match intent {
            Intent::Status => Phase::Status,
            Intent::Login | Intent::Transfer => Phase::Login,
        };
        Ok(Some(Event::Handshake {
            protocol_version: p.protocol_version.0,
            address: p.server_address.0,
            port: p.server_port,
            intent,
        }))
    }

    fn on_status(&mut self, msg: ServerboundStatus) -> Result<Option<Event>> {
        match msg {
            ServerboundStatus::StatusRequest(_) => {
                // The list can only be requested once.
                if std::mem::replace(&mut self.status_requested, true) {
                    return Err(SessionError::Unexpected {
                        packet: "status_request",
                        phase: "status (already requested)",
                    });
                }
                Ok(Some(Event::StatusRequested))
            }
            ServerboundStatus::PingRequest(p) => {
                self.send(&status::PongResponse { timestamp: p.timestamp })?;
                Ok(None)
            }
        }
    }

    fn on_login(&mut self, msg: ServerboundLogin) -> Result<Option<Event>> {
        match msg {
            ServerboundLogin::Hello(p) => {
                // Only one `hello` per connection.
                if self.phase != Phase::Login {
                    return Err(SessionError::Unexpected {
                        packet: "hello",
                        phase: self.phase.name(),
                    });
                }
                self.phase = Phase::LoginPending;
                Ok(Some(Event::LoginStarted {
                    name: p.name.0,
                    uuid: p.uuid,
                    transfer: self.transfer,
                }))
            }
            ServerboundLogin::LoginAcknowledged(_) => Err(SessionError::Unexpected {
                packet: "login_acknowledged",
                phase: self.phase.name(),
            }),
            other => Ok(Some(Event::Login(other))),
        }
    }

    /// The fixed opening of the configuration phase, in vanilla's order.
    fn begin_configuration(&mut self) -> Result<Option<Event>> {
        self.phase = Phase::Configuration;
        let mut brand = Vec::new();
        crate::codec::write_str(&mut brand, &self.cfg.brand, 32767)?;
        self.send(&cfg::CustomPayload {
            channel: Identifier::new("minecraft:brand")?,
            data: Remaining(brand),
        })?;
        self.send(&cfg::UpdateEnabledFeatures {
            features: LenVec(vec![Identifier::new("minecraft:vanilla")?]),
        })?;
        self.send(&cfg::SelectKnownPacks { packs: LenVec(vec![core_pack()]) })?;
        self.sent_known_packs = true;
        Ok(Some(Event::Configuring))
    }

    fn on_configuration(&mut self, msg: ServerboundConfiguration) -> Result<Option<Event>> {
        match msg {
            ServerboundConfiguration::SelectKnownPacks(reply) => {
                if !self.sent_known_packs || self.finish_sent {
                    return Err(SessionError::Unexpected {
                        packet: "select_known_packs",
                        phase: self.phase.name(),
                    });
                }
                // Without the shared core pack we would have to ship every registry entry's full
                // data; that is not implemented, so such clients are turned away explicitly.
                if !reply.packs.0.contains(&core_pack()) {
                    let _ = self.disconnect(&format!(
                        "This server requires Minecraft {}",
                        ferrite_data::GAME_VERSION
                    ));
                    return Err(SessionError::MissingCorePack {
                        version: ferrite_data::GAME_VERSION,
                    });
                }
                self.send_registries_and_finish()?;
                Ok(None)
            }
            ServerboundConfiguration::FinishConfiguration(_) => {
                if !self.finish_sent {
                    return Err(SessionError::Unexpected {
                        packet: "finish_configuration",
                        phase: self.phase.name(),
                    });
                }
                self.phase = Phase::Play;
                Ok(Some(Event::EnteredPlay))
            }
            other => Ok(Some(Event::Configuration(other))),
        }
    }

    fn send_registries_and_finish(&mut self) -> Result<()> {
        use ferrite_data::configuration::{SYNCHRONIZED_REGISTRIES, TAG_REGISTRIES};
        for reg in SYNCHRONIZED_REGISTRIES.iter() {
            let entries = reg
                .entries
                .iter()
                .map(|name| Ok(cfg::RegistryEntry { id: Identifier::new(*name)?, data: None }))
                .collect::<Result<Vec<_>, SessionError>>()?;
            self.send(&cfg::RegistryData {
                registry: Identifier::new(reg.id)?,
                entries: LenVec(entries),
            })?;
        }
        let mut registries = Vec::with_capacity(TAG_REGISTRIES.len());
        for reg in TAG_REGISTRIES.iter() {
            let tags = reg
                .tags
                .iter()
                .map(|t| {
                    Ok(cfg::TagEntry {
                        name: Identifier::new(t.name)?,
                        entries: LenVec(t.entries.iter().map(|&i| VarInt(i32::from(i))).collect()),
                    })
                })
                .collect::<Result<Vec<_>, SessionError>>()?;
            registries.push(cfg::RegistryTags {
                registry: Identifier::new(reg.registry)?,
                tags: LenVec(tags),
            });
        }
        self.send(&cfg::UpdateTags { registries: LenVec(registries) })?;
        self.send(&cfg::FinishConfiguration {})?;
        self.finish_sent = true;
        Ok(())
    }
}

fn other_name_login(p: &ServerboundLogin) -> &'static str {
    match p {
        ServerboundLogin::Hello(_) => "hello",
        ServerboundLogin::Key(_) => "key",
        ServerboundLogin::CustomQueryAnswer(_) => "custom_query_answer",
        ServerboundLogin::LoginAcknowledged(_) => "login_acknowledged",
        ServerboundLogin::CookieResponse(_) => "cookie_response",
    }
}

/// The `minecraft:core` data pack at this server's game version.
pub fn core_pack() -> cfg::KnownPack {
    cfg::KnownPack {
        namespace: "minecraft".into(),
        id: "core".into(),
        version: ferrite_data::GAME_VERSION.into(),
    }
}

/// The UUID vanilla assigns to players in offline mode: a version-3 (MD5) UUID of
/// `OfflinePlayer:<name>`.
pub fn offline_uuid(name: &str) -> Uuid {
    let mut hash: [u8; 16] = Md5::digest(format!("OfflinePlayer:{name}").as_bytes()).into();
    hash[6] = (hash[6] & 0x0f) | 0x30;
    hash[8] = (hash[8] & 0x3f) | 0x80;
    Uuid::from_bytes(hash)
}

/// The protocol version this build speaks.
pub const SUPPORTED_PROTOCOL: i32 = PROTOCOL_VERSION;
