use cfb8::cipher::{AsyncStreamCipher, KeyIvInit};
use ferrite_protocol::cipher::{Decryptor, Encryptor};
use proptest::prelude::*;

type Oracle = cfb8::Encryptor<aes::Aes128>;
type OracleDec = cfb8::Decryptor<aes::Aes128>;

fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

/// NIST SP 800-38A, F.3.7 CFB8-AES128, checked against our implementation directly.
#[test]
fn nist_sp800_38a_cfb8_aes128_vector() {
    let key: [u8; 16] = hex("2b7e151628aed2a6abf7158809cf4f3c").try_into().unwrap();
    let iv: [u8; 16] = hex("000102030405060708090a0b0c0d0e0f").try_into().unwrap();
    let plain = hex("6bc1bee22e409f96e93d7e117393172aae2d");
    let cipher = hex("3b79424c9c0dd436bace9e0ed4586a4f32b9");

    let mut data = plain.clone();
    Encryptor::with_iv(&key, &iv).apply(&mut data);
    assert_eq!(data, cipher, "encrypt");
    Decryptor::with_iv(&key, &iv).apply(&mut data);
    assert_eq!(data, plain, "decrypt");

    // Byte-at-a-time delivery must give the same stream.
    let mut enc = Encryptor::with_iv(&key, &iv);
    let mut dec = Decryptor::with_iv(&key, &iv);
    let mut one_by_one = plain.clone();
    for byte in one_by_one.chunks_mut(1) {
        enc.apply(byte);
    }
    assert_eq!(one_by_one, cipher);
    for byte in one_by_one.chunks_mut(1) {
        dec.apply(byte);
    }
    assert_eq!(one_by_one, plain);

    // The oracle crate agrees with the same vector, so the property tests below are meaningful.
    let mut oracle = plain.clone();
    Oracle::new(&key.into(), (&iv).into()).encrypt(&mut oracle);
    assert_eq!(oracle, cipher);
}

fn chunked<'a>(data: &'a mut [u8], cuts: &[usize]) -> Vec<&'a mut [u8]> {
    let mut out = Vec::new();
    let mut rest = data;
    let mut taken = 0;
    for &c in cuts {
        let c = c.min(rest.len());
        let (head, tail) = rest.split_at_mut(c);
        out.push(head);
        rest = tail;
        taken += c;
    }
    let _ = taken;
    out.push(rest);
    out
}

proptest! {
    #[test]
    fn encryptor_matches_the_oracle_for_any_chunking(
        key in any::<[u8; 16]>(),
        data in proptest::collection::vec(any::<u8>(), 0..2000),
        cuts in proptest::collection::vec(0usize..700, 0..6),
    ) {
        let mut expected = data.clone();
        Oracle::new(&key.into(), (&key).into()).encrypt(&mut expected);

        let mut actual = data.clone();
        let mut enc = Encryptor::new(&key);
        for part in chunked(&mut actual, &cuts) {
            enc.apply(part);
        }
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn decryptor_matches_the_oracle_for_any_chunking(
        key in any::<[u8; 16]>(),
        cipher_text in proptest::collection::vec(any::<u8>(), 0..2000),
        cuts in proptest::collection::vec(0usize..700, 0..6),
    ) {
        let mut expected = cipher_text.clone();
        OracleDec::new(&key.into(), (&key).into()).decrypt(&mut expected);

        let mut actual = cipher_text.clone();
        let mut dec = Decryptor::new(&key);
        for part in chunked(&mut actual, &cuts) {
            dec.apply(part);
        }
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn decrypt_inverts_encrypt_across_independent_chunkings(
        key in any::<[u8; 16]>(),
        data in proptest::collection::vec(any::<u8>(), 0..3000),
        enc_cuts in proptest::collection::vec(0usize..900, 0..5),
        dec_cuts in proptest::collection::vec(0usize..900, 0..5),
    ) {
        let mut wire = data.clone();
        let mut enc = Encryptor::new(&key);
        for part in chunked(&mut wire, &enc_cuts) {
            enc.apply(part);
        }
        if !data.is_empty() {
            prop_assert_ne!(&wire, &data);
        }
        let mut dec = Decryptor::new(&key);
        for part in chunked(&mut wire, &dec_cuts) {
            dec.apply(part);
        }
        prop_assert_eq!(wire, data);
    }
}

#[test]
fn large_buffers_cross_batch_boundaries_correctly() {
    let key = [7u8; 16];
    let data: Vec<u8> = (0..100_000u32).map(|i| (i * 31 % 251) as u8).collect();
    let mut expected = data.clone();
    Oracle::new(&key.into(), (&key).into()).encrypt(&mut expected);
    let mut wire = data.clone();
    Encryptor::new(&key).apply(&mut wire);
    assert_eq!(wire, expected);
    Decryptor::new(&key).apply(&mut wire);
    assert_eq!(wire, data);
}
