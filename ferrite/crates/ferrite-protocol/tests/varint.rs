use ferrite_protocol::ProtocolError;
use ferrite_protocol::varint::*;
use proptest::prelude::*;

/// Vectors from the protocol documentation.
const INT_VECTORS: &[(i32, &[u8])] = &[
    (0, &[0x00]),
    (1, &[0x01]),
    (2, &[0x02]),
    (127, &[0x7f]),
    (128, &[0x80, 0x01]),
    (255, &[0xff, 0x01]),
    (25565, &[0xdd, 0xc7, 0x01]),
    (2097151, &[0xff, 0xff, 0x7f]),
    (2147483647, &[0xff, 0xff, 0xff, 0xff, 0x07]),
    (-1, &[0xff, 0xff, 0xff, 0xff, 0x0f]),
    (-2147483648, &[0x80, 0x80, 0x80, 0x80, 0x08]),
];

const LONG_VECTORS: &[(i64, &[u8])] = &[
    (0, &[0x00]),
    (1, &[0x01]),
    (127, &[0x7f]),
    (128, &[0x80, 0x01]),
    (255, &[0xff, 0x01]),
    (2147483647, &[0xff, 0xff, 0xff, 0xff, 0x07]),
    (9223372036854775807, &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f]),
    (-1, &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01]),
    (-2147483648, &[0x80, 0x80, 0x80, 0x80, 0xf8, 0xff, 0xff, 0xff, 0xff, 0x01]),
    (-9223372036854775808, &[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01]),
];

#[test]
fn varint_known_vectors() {
    for &(value, bytes) in INT_VECTORS {
        let (buf, n) = encode_i32(value);
        assert_eq!(&buf[..n], bytes, "encode {value}");
        assert_eq!(len_i32(value), bytes.len(), "len {value}");
        assert_eq!(peek_i32(bytes), Ok(Peek::Done(value, bytes.len())), "decode {value}");
        let mut slice = bytes;
        assert_eq!(read_i32(&mut slice), Ok(value));
        assert!(slice.is_empty());
    }
}

#[test]
fn varlong_known_vectors() {
    for &(value, bytes) in LONG_VECTORS {
        let (buf, n) = encode_i64(value);
        assert_eq!(&buf[..n], bytes, "encode {value}");
        assert_eq!(len_i64(value), bytes.len(), "len {value}");
        assert_eq!(peek_i64(bytes), Ok(Peek::Done(value, bytes.len())), "decode {value}");
    }
}

#[test]
fn length_formula_is_exact_at_every_boundary() {
    for bits in 0..32 {
        for delta in [-1i64, 0, 1] {
            let v = ((1i64 << bits) + delta) as i32;
            assert_eq!(len_i32(v), encode_i32(v).1, "i32 {v}");
        }
    }
    for bits in 0..64 {
        for delta in [-1i128, 0, 1] {
            let v = ((1i128 << bits) + delta) as i64;
            assert_eq!(len_i64(v), encode_i64(v).1, "i64 {v}");
        }
    }
    assert_eq!(len_i32(i32::MIN), 5);
    assert_eq!(len_i64(i64::MIN), 10);
}

#[test]
fn incomplete_prefixes_ask_for_more() {
    for &(_, bytes) in INT_VECTORS {
        for cut in 0..bytes.len() {
            assert_eq!(peek_i32(&bytes[..cut]), Ok(Peek::Incomplete));
            let mut s = &bytes[..cut];
            assert_eq!(read_i32(&mut s), Err(ProtocolError::Eof));
        }
    }
    for &(_, bytes) in LONG_VECTORS {
        for cut in 0..bytes.len() {
            assert_eq!(peek_i64(&bytes[..cut]), Ok(Peek::Incomplete));
        }
    }
}

#[test]
fn overlong_varints_are_rejected() {
    // Six continuation bytes: vanilla throws "VarInt too big".
    assert_eq!(peek_i32(&[0x80; 6]), Err(ProtocolError::VarIntTooLong));
    // Five bytes with the continuation bit still set on the fifth.
    assert_eq!(peek_i32(&[0x80, 0x80, 0x80, 0x80, 0x80]), Err(ProtocolError::VarIntTooLong));
    assert_eq!(peek_i64(&[0x80; 11]), Err(ProtocolError::VarLongTooLong));
    assert_eq!(peek_i64(&[0xff; 10]), Err(ProtocolError::VarLongTooLong));
}

#[test]
fn bits_beyond_the_integer_width_are_discarded_like_vanilla() {
    // 5th byte 0x7f: only its low 4 bits fit in an i32.
    assert_eq!(peek_i32(&[0xff, 0xff, 0xff, 0xff, 0x7f]), Ok(Peek::Done(-1, 5)));
    // Non-canonical but valid: zero padded with a continuation byte.
    assert_eq!(peek_i32(&[0x81, 0x80, 0x00]), Ok(Peek::Done(1, 3)));
}

#[test]
fn decoding_does_not_consume_beyond_the_value() {
    let mut s: &[u8] = &[0xdd, 0xc7, 0x01, 0xaa, 0xbb];
    assert_eq!(read_i32(&mut s), Ok(25565));
    assert_eq!(s, &[0xaa, 0xbb]);
}

proptest! {
    #[test]
    fn varint_round_trips(v in any::<i32>()) {
        let (buf, n) = encode_i32(v);
        prop_assert_eq!(n, len_i32(v));
        prop_assert_eq!(peek_i32(&buf[..n]), Ok(Peek::Done(v, n)));
    }

    #[test]
    fn varlong_round_trips(v in any::<i64>()) {
        let (buf, n) = encode_i64(v);
        prop_assert_eq!(n, len_i64(v));
        prop_assert_eq!(peek_i64(&buf[..n]), Ok(Peek::Done(v, n)));
    }

    #[test]
    fn decoder_never_panics_and_respects_bounds(bytes in proptest::collection::vec(any::<u8>(), 0..16)) {
        match peek_i32(&bytes) {
            Ok(Peek::Done(_, n)) => prop_assert!(n <= MAX_VARINT_LEN && n <= bytes.len()),
            Ok(Peek::Incomplete) => prop_assert!(bytes.len() < MAX_VARINT_LEN),
            Err(_) => prop_assert!(bytes.len() >= MAX_VARINT_LEN),
        }
        match peek_i64(&bytes) {
            Ok(Peek::Done(_, n)) => prop_assert!(n <= MAX_VARLONG_LEN && n <= bytes.len()),
            Ok(Peek::Incomplete) => prop_assert!(bytes.len() < MAX_VARLONG_LEN),
            Err(_) => prop_assert!(bytes.len() >= MAX_VARLONG_LEN),
        }
    }
}

#[test]
fn put_i32_matches_encode_for_every_length_and_buffer_kind() {
    use bytes::{BufMut, BytesMut};
    for &(value, bytes) in INT_VECTORS {
        let mut v = Vec::new();
        put_i32(&mut v, value);
        assert_eq!(v, bytes);
        let mut b = BytesMut::new();
        put_i32(&mut b, value);
        assert_eq!(&b[..], bytes);
        // A buffer with fewer than 8 bytes of room takes the fallback path.
        let mut backing = [0u8; 6];
        let mut tight: &mut [u8] = &mut backing;
        put_i32(&mut tight, value);
        let used = 6 - tight.remaining_mut();
        assert_eq!(&backing[..used], bytes, "tight buffer {value}");
    }
}

proptest! {
    #[test]
    fn wide_store_never_clobbers_following_bytes(v in any::<i32>(), tail in any::<u8>()) {
        let mut buf = Vec::with_capacity(64);
        put_i32(&mut buf, v);
        let n = buf.len();
        buf.push(tail);
        prop_assert_eq!(n, len_i32(v));
        prop_assert_eq!(buf[n], tail);
        prop_assert_eq!(peek_i32(&buf), Ok(Peek::Done(v, n)));
    }
}
