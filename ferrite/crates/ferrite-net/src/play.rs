//! The play state, from the network's point of view.
//!
//! Once a connection finishes configuration the [`Lobby`](crate::Lobby) hands it to a
//! [`PlayHost`] (the game). From then on the connection is two queues: raw frames the client sends
//! go to the game's [`PlayerHandle`], and packets the game queues through the [`PlayerSink`] are
//! framed, compressed and encrypted here. The network layer never interprets a play packet.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bytes::{Bytes, BytesMut};
use ferrite_protocol::cipher::Encryptor;
use ferrite_protocol::packets::configuration::ClientInformation;
use ferrite_protocol::types::GameProfile;
use ferrite_protocol::{FrameDecoder, FrameEncoder, ProtocolError};
use tokio::sync::mpsc;

/// What the game is told when a player enters the play state.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayStart {
    pub profile: GameProfile,
    /// The last client information received during configuration, if any.
    pub client: Option<ClientInformation>,
}

/// A message from the game to a connection.
#[derive(Debug)]
pub enum Outgoing {
    /// One play packet: its id and body (without the length prefix).
    Packet { id: i32, body: Bytes },
    /// Flush everything queued so far, then close the connection.
    Close,
}

/// How many bytes may wait for a slow client before it is considered gone.
pub const MAX_QUEUED_BYTES: usize = 32 * 1024 * 1024;

/// The game's side of a connection's outbound queue. Cheap to clone; usable from any thread.
#[derive(Clone)]
pub struct PlayerSink {
    tx: mpsc::UnboundedSender<Outgoing>,
    queued: Arc<AtomicUsize>,
}

impl fmt::Debug for PlayerSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlayerSink").field("queued", &self.queued.load(Ordering::Relaxed)).finish()
    }
}

impl PlayerSink {
    pub(crate) fn new() -> (Self, mpsc::UnboundedReceiver<Outgoing>, Arc<AtomicUsize>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let queued = Arc::new(AtomicUsize::new(0));
        (Self { tx, queued: queued.clone() }, rx, queued)
    }

    /// Queues a packet. Returns `false` if the connection is gone or has fallen so far behind
    /// that it is being dropped.
    pub fn send(&self, id: i32, body: impl Into<Bytes>) -> bool {
        let body = body.into();
        let n = self.queued.fetch_add(body.len() + 8, Ordering::Relaxed) + body.len() + 8;
        if n > MAX_QUEUED_BYTES {
            let _ = self.tx.send(Outgoing::Close);
            return false;
        }
        self.tx.send(Outgoing::Packet { id, body }).is_ok()
    }

    /// Asks the connection to close once everything queued has been written.
    pub fn close(&self) {
        let _ = self.tx.send(Outgoing::Close);
    }

    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

/// The game's side of a connection's inbound traffic.
pub trait PlayerHandle: Send {
    /// A packet from the client: its id and body.
    fn packet(&mut self, id: i32, body: Bytes);
}

/// Implemented by the game. Called from connection tasks, possibly concurrently.
pub trait PlayHost: fmt::Debug + Send + Sync {
    /// A player finished configuration. Return the handle that receives their packets, or `None`
    /// to refuse them (the connection is then closed). Dropping the handle means they left.
    fn join(&self, start: PlayStart, sink: PlayerSink) -> Option<Box<dyn PlayerHandle>>;
}

/// The codec state of a connection in the play state.
#[derive(Debug)]
pub struct PlayConn {
    dec: FrameDecoder,
    enc: FrameEncoder,
    cipher: Option<Encryptor>,
    out: BytesMut,
}

impl PlayConn {
    pub(crate) fn from_parts(
        dec: FrameDecoder,
        enc: FrameEncoder,
        cipher: Option<Encryptor>,
        out: BytesMut,
    ) -> Self {
        Self { dec, enc, cipher, out }
    }

    /// Frames, compresses and encrypts one packet into the output buffer.
    pub fn queue(&mut self, id: i32, body: &[u8]) -> Result<(), ProtocolError> {
        let start = self.out.len();
        self.enc.encode(id, body, &mut self.out)?;
        if let Some(cipher) = &mut self.cipher {
            cipher.apply(&mut self.out[start..]);
        }
        Ok(())
    }

    /// Adds received bytes.
    pub fn feed(&mut self, data: &[u8]) {
        self.dec.queue(data);
    }

    /// The next complete packet, if one has arrived.
    pub fn next_frame(&mut self) -> Result<Option<ferrite_protocol::Frame>, ProtocolError> {
        self.dec.next_frame()
    }

    pub fn pending_output(&self) -> &[u8] {
        &self.out
    }

    pub fn consume_output(&mut self, n: usize) {
        let _ = self.out.split_to(n);
    }
}

/// Everything a backend needs to run a connection in the play state.
pub struct PlayEntry {
    pub conn: PlayConn,
    pub handle: Box<dyn PlayerHandle>,
    /// Packets the game queued for this player.
    pub outbox: mpsc::UnboundedReceiver<Outgoing>,
    /// Bytes queued and not yet written (kept in step by the backend).
    pub queued: Arc<AtomicUsize>,
}

impl fmt::Debug for PlayEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlayEntry").finish_non_exhaustive()
    }
}
