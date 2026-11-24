use std::io::Write;
use std::process::{Command, Stdio};

use ferrite_protocol::crypto::{ServerKey, encrypt_with_public_key, minecraft_digest, server_hash};

fn have_openssl() -> bool {
    Command::new("openssl").arg("version").output().is_ok_and(|o| o.status.success())
}

fn openssl(args: &[&str], stdin: &[u8]) -> Vec<u8> {
    let mut child = Command::new("openssl")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "openssl {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    out.stdout
}

fn tmp(name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("ferrite-crypto-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name).to_string_lossy().into_owned()
}

#[test]
fn round_trip_and_ciphertext_shape() {
    let key = ServerKey::generate().unwrap();
    let secret = [7u8; 16];
    let ct = encrypt_with_public_key(key.public_key_der(), &secret).unwrap();
    assert_eq!(ct.len(), 128, "a 1024-bit key yields 128-byte ciphertexts");
    assert_eq!(key.decrypt(&ct).unwrap(), secret);
    // PKCS#1 v1.5 is randomized: the same plaintext never encrypts the same way twice.
    assert_ne!(ct, encrypt_with_public_key(key.public_key_der(), &secret).unwrap());
}

/// `rsa` 0.10 uses implicit rejection (the Marvin-attack mitigation): invalid padding does not
/// produce an error, which would leak padding validity through behaviour and timing, but a
/// pseudo-random plaintext. So "rejected" means: never the original plaintext. The session
/// additionally requires an exact length and a matching verify token, so it fails closed.
fn never_yields(key: &ServerKey, ciphertext: &[u8], original: &[u8]) {
    match key.decrypt(ciphertext) {
        Err(_) => {}
        Ok(plain) => assert_ne!(plain, original),
    }
}

#[test]
fn garbage_and_foreign_ciphertexts_never_decrypt_to_the_secret() {
    let key = ServerKey::generate().unwrap();
    let other = ServerKey::generate().unwrap();
    assert_ne!(key.public_key_der(), other.public_key_der());

    let secret = [1u8; 16];
    let foreign = encrypt_with_public_key(other.public_key_der(), &secret).unwrap();
    never_yields(&key, &foreign, &secret);
    for garbage in [&[][..], &[0u8; 128][..], &[0xffu8; 128][..], &foreign[..127], &[1u8; 129][..]]
    {
        never_yields(&key, garbage, &secret);
    }
    let mut tampered = encrypt_with_public_key(key.public_key_der(), &secret).unwrap();
    tampered[40] ^= 0x01;
    never_yields(&key, &tampered, &secret);
}

#[test]
fn rejection_is_deterministic_per_ciphertext() {
    // Implicit rejection derives its output from the key and the ciphertext, so an attacker cannot
    // tell a rejected ciphertext from a valid one by asking twice.
    let key = ServerKey::generate().unwrap();
    let bad = [0x5au8; 128];
    assert_eq!(key.decrypt(&bad).ok(), key.decrypt(&bad).ok());
}

#[test]
fn malformed_public_keys_are_rejected() {
    assert!(encrypt_with_public_key(&[], b"x").is_err());
    assert!(encrypt_with_public_key(&[0x30, 0x00], b"x").is_err());
    let key = ServerKey::generate().unwrap();
    let mut der = key.public_key_der().to_vec();
    der.truncate(der.len() - 3);
    assert!(encrypt_with_public_key(&der, b"x").is_err());
}

#[test]
fn server_hash_covers_id_secret_and_key() {
    let (secret, der) = ([9u8; 16], b"public key bytes");
    let h = server_hash("", &secret, der);
    assert_eq!(h, minecraft_digest(&[b"", &secret, der]));
    assert_ne!(h, server_hash("x", &secret, der));
    assert_ne!(h, server_hash("", &[8u8; 16], der));
    assert_ne!(h, server_hash("", &secret, b"other"));
}

/// OpenSSL is an independent implementation: it must accept our DER and decrypt what we encrypt,
/// and we must decrypt what it encrypts.
#[test]
fn openssl_interoperates_in_both_directions() {
    if !have_openssl() {
        eprintln!("SKIPPED: openssl not installed");
        return;
    }
    let secret = *b"0123456789abcdef";

    // Our public key, parsed by OpenSSL; OpenSSL encrypts, we decrypt.
    let key = ServerKey::generate().unwrap();
    let pub_path = tmp("server-pub.der");
    std::fs::write(&pub_path, key.public_key_der()).unwrap();
    let text = String::from_utf8(openssl(
        &["pkey", "-pubin", "-inform", "DER", "-in", &pub_path, "-text", "-noout"],
        b"",
    ))
    .unwrap();
    assert!(text.contains("1024 bit"), "{text}");
    assert!(text.contains("65537"), "exponent must be the usual F4: {text}");
    let ct = openssl(
        &[
            "pkeyutl",
            "-encrypt",
            "-pubin",
            "-inkey",
            &pub_path,
            "-keyform",
            "DER",
            "-pkeyopt",
            "rsa_padding_mode:pkcs1",
        ],
        &secret,
    );
    assert_eq!(key.decrypt(&ct).unwrap(), secret);

    // An OpenSSL key pair; we encrypt with its DER public key, OpenSSL decrypts.
    let priv_path = tmp("ossl-priv.pem");
    openssl(&["genrsa", "-out", &priv_path, "1024"], b"");
    let der = openssl(&["pkey", "-in", &priv_path, "-pubout", "-outform", "DER"], b"");
    let ours = encrypt_with_public_key(&der, &secret).unwrap();
    let plain = openssl(
        &["pkeyutl", "-decrypt", "-inkey", &priv_path, "-pkeyopt", "rsa_padding_mode:pkcs1"],
        &ours,
    );
    assert_eq!(plain, secret);
}
