use bytes::BytesMut;
use ferrite_protocol::codec::*;
use ferrite_protocol::{BlockPos, BoundedStr, ProtocolError, VarInt, VarLong};
use proptest::prelude::*;
use uuid::Uuid;

fn enc<T: Encode>(v: &T) -> Vec<u8> {
    let mut b = BytesMut::new();
    v.encode(&mut b).unwrap();
    b.to_vec()
}

#[test]
fn integers_are_big_endian() {
    assert_eq!(enc(&0x0102_0304i32), [1, 2, 3, 4]);
    assert_eq!(enc(&-2i16), [0xff, 0xfe]);
    assert_eq!(enc(&1.0f32), [0x3f, 0x80, 0, 0]);
    assert_eq!(enc(&1.0f64), [0x3f, 0xf0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(decode_exact::<u16>(&[0x63, 0xdd]), Ok(25565));
}

#[test]
fn bool_treats_any_nonzero_byte_as_true() {
    assert_eq!(decode_exact::<bool>(&[0]), Ok(false));
    assert_eq!(decode_exact::<bool>(&[1]), Ok(true));
    assert_eq!(decode_exact::<bool>(&[0x80]), Ok(true));
}

#[test]
fn truncated_input_is_eof_and_leaves_no_partial_state() {
    let mut s: &[u8] = &[1, 2, 3];
    assert_eq!(i32::decode(&mut s), Err(ProtocolError::Eof));
    assert_eq!(u64::decode(&mut s), Err(ProtocolError::Eof));
    assert_eq!(Uuid::decode(&mut s), Err(ProtocolError::Eof));
    assert_eq!(decode_exact::<u8>(&[1, 2]), Err(ProtocolError::TrailingBytes(1)));
}

#[test]
fn strings_round_trip_and_enforce_limits() {
    let mut b = BytesMut::new();
    write_str(&mut b, "héllo", 16).unwrap();
    assert_eq!(&b[..], b"\x06h\xc3\xa9llo");
    let mut s = &b[..];
    assert_eq!(read_str(&mut s, 16), Ok("héllo"));

    // Limit is in UTF-16 units: a supplementary-plane char counts as two.
    let emoji = "😀😀";
    assert_eq!(emoji.len(), 8);
    let mut b = BytesMut::new();
    assert_eq!(write_str(&mut b, emoji, 3), Err(ProtocolError::StringTooLong { max: 3 }));
    write_str(&mut b, emoji, 4).unwrap();
    assert_eq!(read_str(&mut &b[..], 3), Err(ProtocolError::StringTooLong { max: 3 }));
    assert_eq!(read_str(&mut &b[..], 4), Ok(emoji));

    // Byte-length pre-check rejects before reading the payload.
    assert_eq!(read_str(&mut &[0x64][..], 10), Err(ProtocolError::StringTooLong { max: 10 }));
    assert_eq!(
        read_str(&mut &[0xff, 0xff, 0xff, 0xff, 0x0f][..], 10),
        Err(ProtocolError::NegativeLength(-1))
    );
    assert_eq!(read_str(&mut &[0x02, b'a'][..], 10), Err(ProtocolError::Eof));
    assert_eq!(read_str(&mut &[0x02, 0xff, 0xfe][..], 10), Err(ProtocolError::InvalidUtf8));
}

#[test]
fn default_string_limit_is_32767() {
    let ok = "a".repeat(32767);
    let too_long = "a".repeat(32768);
    let mut b = BytesMut::new();
    BoundedStr::<DEFAULT_MAX_STRING>(&ok).encode(&mut b).unwrap();
    assert_eq!(decode_exact::<BoundedStr>(&b).unwrap().0, ok);
    assert!(BoundedStr::<DEFAULT_MAX_STRING>(&too_long).encode(&mut BytesMut::new()).is_err());
    assert_eq!(
        decode_exact::<BoundedStr<16>>(&enc(&BoundedStr::<32>("0123456789abcdef!"))),
        Err(ProtocolError::StringTooLong { max: 16 })
    );
}

#[test]
fn block_position_matches_the_documented_packing() {
    // Worked example from the protocol documentation.
    let pos = BlockPos::new(18357644, 831, -20882616);
    // Fields as printed in the documentation: x (26 bits), z (26 bits), y (12 bits).
    let documented = u64::from_str_radix(
        &"01000110000001110110001100_10110000010101101101001000_001100111111".replace('_', ""),
        2,
    )
    .unwrap();
    assert_eq!(pos.pack() as u64, documented);
    assert_eq!(BlockPos::unpack(pos.pack()), pos);
    for p in [
        BlockPos::new(0, 0, 0),
        BlockPos::new(-1, -1, -1),
        BlockPos::new(-33554432, -2048, -33554432),
        BlockPos::new(33554431, 2047, 33554431),
    ] {
        assert_eq!(BlockPos::unpack(p.pack()), p);
        assert_eq!(decode_exact::<BlockPos>(&enc(&p)), Ok(p));
    }
    // Out-of-range coordinates wrap rather than corrupt neighbouring fields.
    assert_eq!(
        BlockPos::unpack(BlockPos::new(33554432, 2048, 33554432).pack()),
        BlockPos::new(-33554432, -2048, -33554432)
    );
}

#[test]
fn uuid_option_and_arrays() {
    let id = Uuid::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);
    assert_eq!(enc(&id), id.as_bytes());
    assert_eq!(decode_exact::<Uuid>(&enc(&id)), Ok(id));
    assert_eq!(enc(&None::<i32>), [0]);
    assert_eq!(enc(&Some(7i32)), [1, 0, 0, 0, 7]);
    assert_eq!(decode_exact::<Option<i32>>(&[1, 0, 0, 0, 7]), Ok(Some(7)));
    assert_eq!(decode_exact::<[u8; 3]>(&[9, 8, 7]), Ok([9, 8, 7]));
}

#[test]
fn lists_and_byte_arrays_are_bounded() {
    let mut b = BytesMut::new();
    write_list(&mut b, &[VarInt(1), VarInt(300)], 8).unwrap();
    assert_eq!(&b[..], [2, 1, 0xac, 0x02]);
    assert_eq!(read_list::<VarInt>(&mut &b[..], 8), Ok(vec![VarInt(1), VarInt(300)]));
    assert_eq!(
        read_list::<VarInt>(&mut &b[..], 1),
        Err(ProtocolError::LengthLimit { len: 2, max: 1 })
    );
    assert!(write_list(&mut BytesMut::new(), &[VarInt(0); 3], 2).is_err());

    // A huge claimed length with no data must fail fast without allocating that much.
    let hostile = [0xff, 0xff, 0xff, 0xff, 0x07];
    assert_eq!(read_list::<VarLong>(&mut &hostile[..], usize::MAX), Err(ProtocolError::Eof));
    // Zero-width items cannot be used to burn CPU.
    assert!(read_list::<ZeroWidth>(&mut &[0xff, 0xff, 0x7f][..], 1 << 20).is_err());

    let mut b = BytesMut::new();
    write_byte_array(&mut b, b"abc", 8).unwrap();
    assert_eq!(&b[..], [3, b'a', b'b', b'c']);
    assert_eq!(read_byte_array(&mut &b[..], 8), Ok(&b"abc"[..]));
    assert_eq!(read_byte_array(&mut &b[..], 2), Err(ProtocolError::LengthLimit { len: 3, max: 2 }));
    assert_eq!(read_byte_array(&mut &[5, 1][..], 8), Err(ProtocolError::Eof));
}

/// A type that occupies no bytes on the wire, used to prove the zero-width guard in `read_list`.
struct ZeroWidth;
impl<'a> Decode<'a> for ZeroWidth {
    fn decode(_: &mut &'a [u8]) -> ferrite_protocol::Result<Self> {
        Ok(ZeroWidth)
    }
}

proptest! {
    #[test]
    fn primitives_round_trip(a in any::<i64>(), b in any::<u32>(), c in any::<f64>(), d in any::<bool>(), e in any::<u128>()) {
        prop_assert_eq!(decode_exact::<i64>(&enc(&a)), Ok(a));
        prop_assert_eq!(decode_exact::<u32>(&enc(&b)), Ok(b));
        prop_assert_eq!(decode_exact::<f64>(&enc(&c)).unwrap().to_bits(), c.to_bits());
        prop_assert_eq!(decode_exact::<bool>(&enc(&d)), Ok(d));
        prop_assert_eq!(decode_exact::<u128>(&enc(&e)), Ok(e));
        prop_assert_eq!(decode_exact::<VarInt>(&enc(&VarInt(a as i32))), Ok(VarInt(a as i32)));
        prop_assert_eq!(decode_exact::<VarLong>(&enc(&VarLong(a))), Ok(VarLong(a)));
    }

    #[test]
    fn strings_round_trip(s in "\\PC{0,64}") {
        // 64 chars are at most 128 UTF-16 units (astral chars count twice).
        let mut b = BytesMut::new();
        write_str(&mut b, &s, 128).unwrap();
        prop_assert_eq!(read_str(&mut &b[..], 128), Ok(s.as_str()));
    }

    #[test]
    fn block_pos_round_trips(x in -33554432i32..33554432, y in -2048i32..2048, z in -33554432i32..33554432) {
        let p = BlockPos::new(x, y, z);
        prop_assert_eq!(BlockPos::unpack(p.pack()), p);
    }

    #[test]
    fn arbitrary_bytes_never_panic_the_decoders(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        let _ = read_str(&mut &bytes[..], 16);
        let _ = read_byte_array(&mut &bytes[..], 16);
        let _ = read_list::<VarInt>(&mut &bytes[..], 16);
        let _ = decode_exact::<Option<Uuid>>(&bytes);
    }
}
