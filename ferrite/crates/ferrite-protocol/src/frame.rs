//! Packet framing: length prefix, optional zlib compression, and the packet id.
//!
//! ```text
//! uncompressed:  VarInt length | VarInt id | body
//! compressed:    VarInt length | VarInt data_length | (id | body), zlib-compressed unless
//!                data_length == 0, in which case it follows verbatim
//! ```
//!
//! Encryption is applied to the whole byte stream *outside* of framing, see [`crate::cipher`].

use bytes::{Buf, Bytes, BytesMut};
use libdeflater::{CompressionLvl, Compressor, Decompressor};

use crate::cipher::Decryptor;
use crate::error::{ProtocolError, Result};
use crate::varint::{self, Peek};

/// Largest frame the vanilla client may send (VarInt length prefix of at most 3 bytes).
pub const MAX_SERVERBOUND_FRAME: usize = (1 << 21) - 1;
/// Largest decompressed packet vanilla accepts.
pub const MAX_UNCOMPRESSED: usize = 8 * 1024 * 1024;

/// A decoded packet: its id and the body that follows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub id: i32,
    pub body: Bytes,
}

/// Compression settings negotiated with `set_compression`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Compression {
    /// Packets with at least this many bytes (id + body) are compressed.
    pub threshold: usize,
}

/// Turns a byte stream into packets.
pub struct FrameDecoder {
    buf: BytesMut,
    /// Everything before this offset has already been decrypted.
    decrypted: usize,
    cipher: Option<Decryptor>,
    compression: Option<Compression>,
    max_frame: usize,
    max_uncompressed: usize,
    /// Created on first use: most connections (status pings) never see compressed data.
    inflater: Option<Decompressor>,
}

impl std::fmt::Debug for FrameDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameDecoder")
            .field("buffered", &self.buf.len())
            .field("encrypted", &self.cipher.is_some())
            .field("compression", &self.compression)
            .finish()
    }
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self {
            buf: BytesMut::with_capacity(4096),
            decrypted: 0,
            cipher: None,
            compression: None,
            max_frame: MAX_SERVERBOUND_FRAME,
            max_uncompressed: MAX_UNCOMPRESSED,
            inflater: None,
        }
    }

    /// Overrides the frame size limits (for clients, which may receive far larger frames).
    pub fn with_limits(mut self, max_frame: usize, max_uncompressed: usize) -> Self {
        self.max_frame = max_frame;
        self.max_uncompressed = max_uncompressed;
        self
    }

    /// Starts decrypting. Call it right after taking the last plaintext frame out of the decoder.
    ///
    /// Everything still buffered at that point sits *behind* that frame and is therefore
    /// ciphertext, even though it was received (and marked decrypted) before encryption was
    /// enabled. A client may well pipeline its first encrypted packets in the same TCP segment as
    /// the plaintext `key` packet, so those bytes are decrypted now.
    pub fn enable_encryption(&mut self, shared_secret: &[u8; 16]) {
        self.cipher = Some(Decryptor::new(shared_secret));
        self.decrypted = 0;
        self.commit();
    }

    pub fn set_compression(&mut self, compression: Option<Compression>) {
        self.compression = compression;
    }

    /// The raw receive buffer. Read from the socket straight into it, then call [`commit`].
    ///
    /// [`commit`]: Self::commit
    pub fn buffer_mut(&mut self) -> &mut BytesMut {
        &mut self.buf
    }

    /// Declares that new bytes were appended through [`buffer_mut`](Self::buffer_mut), decrypting
    /// them if encryption is on.
    pub fn commit(&mut self) {
        if let Some(cipher) = &mut self.cipher {
            cipher.apply(&mut self.buf[self.decrypted..]);
        }
        self.decrypted = self.buf.len();
    }

    /// Appends received bytes. Equivalent to writing into [`buffer_mut`](Self::buffer_mut) and
    /// calling [`commit`](Self::commit).
    pub fn queue(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
        self.commit();
    }

    /// Number of buffered bytes not yet turned into packets.
    pub fn buffered(&self) -> usize {
        self.buf.len()
    }

    /// Extracts the next complete packet, `Ok(None)` if more bytes are needed.
    ///
    /// An error means the stream is corrupt or hostile; the connection should be closed.
    pub fn next_frame(&mut self) -> Result<Option<Frame>> {
        // Vanilla's splitter reads at most as many length bytes as the size limit needs (3 for
        // the serverbound limit) and rejects anything wider, even a non-canonical encoding.
        let max_header = varint::len_i32(self.max_frame as i32);
        loop {
            let (len, header) = match varint::peek_i32(&self.buf)? {
                Peek::Done(len, n) if n <= max_header => (len, n),
                Peek::Done(..) => {
                    return Err(ProtocolError::LengthPrefixTooWide { max_bytes: max_header });
                }
                Peek::Incomplete if self.buf.len() >= max_header => {
                    return Err(ProtocolError::LengthPrefixTooWide { max_bytes: max_header });
                }
                Peek::Incomplete => return Ok(None),
            };
            let len = usize::try_from(len).map_err(|_| ProtocolError::NegativeLength(len))?;
            if len > self.max_frame {
                return Err(ProtocolError::FrameTooLarge { len, max: self.max_frame });
            }
            let total = header + len;
            if self.buf.len() < total {
                // Make the next socket read land in one allocation instead of growing in steps.
                self.buf.reserve(total - self.buf.len());
                return Ok(None);
            }
            self.buf.advance(header);
            let payload = self.buf.split_to(len).freeze();
            self.decrypted -= total;

            // Vanilla silently skips empty frames.
            if payload.is_empty() {
                continue;
            }
            return self.open(payload).map(Some);
        }
    }

    /// Removes compression and splits off the packet id.
    fn open(&mut self, payload: Bytes) -> Result<Frame> {
        let mut payload = match self.compression {
            None => payload,
            Some(Compression { threshold }) => {
                let (declared, n) = match varint::peek_i32(&payload)? {
                    Peek::Done(v, n) => (v, n),
                    Peek::Incomplete => return Err(ProtocolError::Eof),
                };
                let declared = usize::try_from(declared)
                    .map_err(|_| ProtocolError::NegativeLength(declared))?;
                let rest = payload.slice(n..);
                if declared == 0 {
                    rest
                } else {
                    if declared < threshold {
                        return Err(ProtocolError::BelowThreshold { len: declared, threshold });
                    }
                    if declared > self.max_uncompressed {
                        return Err(ProtocolError::DecompressedTooLarge {
                            len: declared,
                            max: self.max_uncompressed,
                        });
                    }
                    let mut out = vec![0u8; declared];
                    let written = self
                        .inflater
                        .get_or_insert_with(Decompressor::new)
                        .zlib_decompress(&rest, &mut out)
                        .map_err(|_| ProtocolError::Decompress)?;
                    if written != declared {
                        return Err(ProtocolError::Decompress);
                    }
                    Bytes::from(out)
                }
            }
        };
        let id = match varint::peek_i32(&payload)? {
            Peek::Done(id, n) => {
                payload.advance(n);
                id
            }
            Peek::Incomplete => return Err(ProtocolError::Eof),
        };
        Ok(Frame { id, body: payload })
    }
}

/// Turns packets into bytes ready for the socket (before encryption).
pub struct FrameEncoder {
    compression: Option<Compression>,
    /// Created when compression is first enabled; libdeflate's compressor is a large allocation
    /// that a connection which never compresses should not pay for.
    deflater: Option<Compressor>,
    level: i32,
    joined: Vec<u8>,
    packed: Vec<u8>,
}

impl std::fmt::Debug for FrameEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameEncoder").field("compression", &self.compression).finish()
    }
}

impl Default for FrameEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameEncoder {
    pub fn new() -> Self {
        Self { compression: None, deflater: None, level: 6, joined: Vec::new(), packed: Vec::new() }
    }

    /// Enables compression with the given threshold and zlib level (1 fastest .. 12 smallest).
    pub fn set_compression(&mut self, compression: Option<Compression>, level: i32) -> Result<()> {
        if compression.is_some() && (self.deflater.is_none() || level != self.level) {
            let lvl = CompressionLvl::new(level).map_err(|_| ProtocolError::Compress)?;
            self.deflater = Some(Compressor::new(lvl));
            self.level = level;
        }
        self.compression = compression;
        Ok(())
    }

    /// Appends one framed packet (`id` followed by `body`) to `out`.
    pub fn encode(&mut self, id: i32, body: &[u8], out: &mut BytesMut) -> Result<()> {
        let (id_bytes, id_len) = varint::encode_i32(id);
        let payload_len = id_len + body.len();
        let Some(Compression { threshold }) = self.compression else {
            put_varint(out, payload_len)?;
            out.extend_from_slice(&id_bytes[..id_len]);
            out.extend_from_slice(body);
            return Ok(());
        };

        if payload_len < threshold {
            // data_length == 0 marks "not compressed".
            put_varint(out, 1 + payload_len)?;
            out.extend_from_slice(&[0]);
            out.extend_from_slice(&id_bytes[..id_len]);
            out.extend_from_slice(body);
            return Ok(());
        }

        self.joined.clear();
        self.joined.extend_from_slice(&id_bytes[..id_len]);
        self.joined.extend_from_slice(body);
        // `compression` is only ever `Some` after `set_compression` created the compressor.
        let deflater = self.deflater.as_mut().ok_or(ProtocolError::Compress)?;
        let bound = deflater.zlib_compress_bound(payload_len);
        self.packed.resize(bound, 0);
        let packed_len = deflater
            .zlib_compress(&self.joined, &mut self.packed)
            .map_err(|_| ProtocolError::Compress)?;

        let data_len_bytes = varint::len_i32(payload_len as i32);
        put_varint(out, data_len_bytes + packed_len)?;
        put_varint(out, payload_len)?;
        out.extend_from_slice(&self.packed[..packed_len]);
        Ok(())
    }
}

fn put_varint(out: &mut BytesMut, value: usize) -> Result<()> {
    let value = i32::try_from(value)
        .map_err(|_| ProtocolError::FrameTooLarge { len: value, max: i32::MAX as usize })?;
    varint::put_i32(out, value);
    Ok(())
}
