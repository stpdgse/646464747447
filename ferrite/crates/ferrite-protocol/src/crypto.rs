//! The cryptography of online-mode logins.
//!
//! * The server holds a 1024-bit RSA key (vanilla's choice; both vanilla clients and this code
//!   accept longer ones) and sends its X.509 `SubjectPublicKeyInfo` DER encoding to the client.
//! * The client encrypts a random 16-byte shared secret and the server's verify token with it
//!   (PKCS#1 v1.5). The shared secret then keys AES-128-CFB8 in both directions (see
//!   [`crate::cipher`]).
//! * Both sides derive the *server hash* with Minecraft's non-standard SHA-1 hexdigest and the
//!   client proves its identity by registering it with Mojang's session server.
//!
//! RSA decryption of attacker-supplied ciphertext is exactly the setting of the "Marvin" timing
//! attack (RUSTSEC-2023-0071). This uses `rsa` 0.10, whose decryption is constant-time; do not
//! downgrade it.

use getrandom::SysRng;
use rsa::pkcs8::{DecodePublicKey, EncodePublicKey};
use rsa::{Pkcs1v15Encrypt, PrivateKey as RsaPrivateKey, PublicKey as RsaPublicKey};
use sha1::{Digest, Sha1};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("RSA operation failed: {0}")]
    Rsa(String),
    #[error("the public key is not a valid DER SubjectPublicKeyInfo: {0}")]
    BadPublicKey(String),
}

/// The server's long-lived RSA key pair.
pub struct ServerKey {
    private: RsaPrivateKey,
    public_der: Vec<u8>,
}

impl std::fmt::Debug for ServerKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ServerKey { .. }")
    }
}

impl ServerKey {
    /// Vanilla's key size.
    pub const BITS: usize = 1024;

    /// Generates a fresh key pair from the operating system's randomness.
    pub fn generate() -> Result<Self, CryptoError> {
        Self::generate_with_bits(Self::BITS)
    }

    pub fn generate_with_bits(bits: usize) -> Result<Self, CryptoError> {
        let private =
            RsaPrivateKey::new(&mut SysRng, bits).map_err(|e| CryptoError::Rsa(e.to_string()))?;
        let public_der = RsaPublicKey::from(&private)
            .to_public_key_der()
            .map_err(|e| CryptoError::Rsa(e.to_string()))?
            .into_vec();
        Ok(Self { private, public_der })
    }

    /// The DER-encoded `SubjectPublicKeyInfo` sent in the encryption request.
    pub fn public_key_der(&self) -> &[u8] {
        &self.public_der
    }

    /// Decrypts a PKCS#1 v1.5 ciphertext from a client.
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        self.private
            .decrypt(Pkcs1v15Encrypt, ciphertext)
            .map_err(|e| CryptoError::Rsa(e.to_string()))
    }
}

/// What a client does with the server's public key: PKCS#1 v1.5 encryption.
pub fn encrypt_with_public_key(public_key_der: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let key = RsaPublicKey::from_public_key_der(public_key_der)
        .map_err(|e| CryptoError::BadPublicKey(e.to_string()))?;
    key.encrypt(&mut SysRng, Pkcs1v15Encrypt, data).map_err(|e| CryptoError::Rsa(e.to_string()))
}

/// Minecraft's SHA-1 "hexdigest": the 20 digest bytes read as one big two's-complement integer and
/// printed in base 16 with a leading `-` if negative and **no** zero padding.
///
/// ```
/// use ferrite_protocol::crypto::minecraft_digest;
/// assert_eq!(minecraft_digest(&[b"Notch"]), "4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48");
/// assert_eq!(minecraft_digest(&[b"jeb_"]), "-7c9d5b0044c130109a5d7b5fb5c317c02b4e28c1");
/// assert_eq!(minecraft_digest(&[b"simon"]), "88e16a1019277b15d58faf0541e11910eb756f6");
/// ```
pub fn minecraft_digest(parts: &[&[u8]]) -> String {
    let mut sha = Sha1::new();
    for part in parts {
        sha.update(part);
    }
    signed_hex(sha.finalize().into())
}

fn signed_hex(mut digest: [u8; 20]) -> String {
    let negative = digest[0] & 0x80 != 0;
    if negative {
        // Two's complement negate: invert, then add one.
        let mut carry = true;
        for byte in digest.iter_mut().rev() {
            *byte = !*byte;
            if carry {
                let (v, overflow) = byte.overflowing_add(1);
                *byte = v;
                carry = overflow;
            }
        }
    }
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    let trimmed = hex.trim_start_matches('0');
    let body = if trimmed.is_empty() { "0" } else { trimmed };
    if negative { format!("-{body}") } else { body.to_owned() }
}

/// The hash both sides compute for session authentication.
pub fn server_hash(server_id: &str, shared_secret: &[u8; 16], public_key_der: &[u8]) -> String {
    minecraft_digest(&[server_id.as_bytes(), shared_secret, public_key_der])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_hex_handles_sign_padding_and_edge_values() {
        assert_eq!(signed_hex([0; 20]), "0");
        let mut one = [0u8; 20];
        one[19] = 1;
        assert_eq!(signed_hex(one), "1");
        assert_eq!(signed_hex([0xff; 20]), "-1");
        // The most negative value negates to itself.
        let mut min = [0u8; 20];
        min[0] = 0x80;
        assert_eq!(signed_hex(min), format!("-8{}", "0".repeat(39)));
        // Leading zero bytes are dropped rather than padded.
        let mut lead = [0u8; 20];
        lead[1] = 0x12;
        assert_eq!(signed_hex(lead), format!("12{}", "00".repeat(18)));
        // A negative value: bytes ff ee ff .. ff negate (invert, add one) to 00 11 00 .. 00 01.
        let mut neg = [0xffu8; 20];
        neg[1] = 0xee;
        assert_eq!(signed_hex(neg), format!("-11{}01", "00".repeat(17)));
    }

    #[test]
    fn digest_of_parts_equals_digest_of_the_concatenation() {
        let whole = minecraft_digest(&[b"abcdef"]);
        assert_eq!(minecraft_digest(&[b"abc", b"", b"def"]), whole);
        assert_eq!(minecraft_digest(&[b"a", b"b", b"c", b"d", b"e", b"f"]), whole);
    }
}
