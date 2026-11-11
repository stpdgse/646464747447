//! AES-128-CFB8, the stream cipher used after the online-mode key exchange.
//!
//! CFB8 feeds the previous 16 *ciphertext* bytes through AES for every single byte:
//!
//! ```text
//! keystream[i] = AES(c[i-16 .. i])[0]        c[j] = p[j] ^ keystream[j]
//! ```
//!
//! Encryption is therefore a serial dependency chain, one AES block per byte, and cannot be
//! parallelised. **Decryption can**: every window is made of ciphertext we already have, so
//! [`Decryptor`] batches windows and lets AES-NI pipeline them.

use aes::Aes128;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockEncrypt, KeyInit};

type Block = GenericArray<u8, aes::cipher::consts::U16>;

/// Number of AES blocks handed to the cipher at once while decrypting.
const BATCH: usize = 256;

/// Encrypts the outbound byte stream.
#[derive(Clone)]
pub struct Encryptor {
    cipher: Aes128,
    /// Sliding window over the last 16 ciphertext bytes; `window[off..off + 16]` is the state.
    window: [u8; 32],
    off: usize,
}

impl std::fmt::Debug for Encryptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Encryptor { .. }")
    }
}

impl Encryptor {
    /// Creates the cipher from the negotiated shared secret, which also serves as the IV.
    pub fn new(shared_secret: &[u8; 16]) -> Self {
        Self::with_iv(shared_secret, shared_secret)
    }

    /// Creates the cipher with an explicit IV (Minecraft always uses the key as the IV).
    pub fn with_iv(key: &[u8; 16], iv: &[u8; 16]) -> Self {
        let mut window = [0u8; 32];
        window[..16].copy_from_slice(iv);
        Self { cipher: Aes128::new(key.into()), window, off: 0 }
    }

    /// Encrypts `data` in place, continuing the stream from the previous call.
    pub fn apply(&mut self, data: &mut [u8]) {
        for byte in data {
            let mut block = Block::clone_from_slice(&self.window[self.off..self.off + 16]);
            self.cipher.encrypt_block(&mut block);
            *byte ^= block[0];
            self.window[self.off + 16] = *byte;
            self.off += 1;
            if self.off == 16 {
                self.window.copy_within(16..32, 0);
                self.off = 0;
            }
        }
    }
}

/// Decrypts the inbound byte stream.
#[derive(Clone)]
pub struct Decryptor {
    cipher: Aes128,
    /// The previous 16 ciphertext bytes.
    history: [u8; 16],
}

impl std::fmt::Debug for Decryptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Decryptor { .. }")
    }
}

impl Decryptor {
    /// Creates the cipher from the negotiated shared secret, which also serves as the IV.
    pub fn new(shared_secret: &[u8; 16]) -> Self {
        Self::with_iv(shared_secret, shared_secret)
    }

    /// Creates the cipher with an explicit IV (Minecraft always uses the key as the IV).
    pub fn with_iv(key: &[u8; 16], iv: &[u8; 16]) -> Self {
        Self { cipher: Aes128::new(key.into()), history: *iv }
    }

    /// Decrypts `data` in place, continuing the stream from the previous call.
    pub fn apply(&mut self, data: &mut [u8]) {
        let mut ext = [0u8; 16 + BATCH];
        let mut blocks = [Block::default(); BATCH];
        for chunk in data.chunks_mut(BATCH) {
            let n = chunk.len();
            ext[..16].copy_from_slice(&self.history);
            ext[16..16 + n].copy_from_slice(chunk);
            for (i, block) in blocks[..n].iter_mut().enumerate() {
                block.copy_from_slice(&ext[i..i + 16]);
            }
            self.cipher.encrypt_blocks(&mut blocks[..n]);
            for (i, byte) in chunk.iter_mut().enumerate() {
                *byte ^= blocks[i][0];
            }
            self.history.copy_from_slice(&ext[n..n + 16]);
        }
    }
}
