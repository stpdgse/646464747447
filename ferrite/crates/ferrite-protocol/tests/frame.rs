use std::io::Write;

use bytes::BytesMut;
use ferrite_protocol::cipher::Encryptor;
use ferrite_protocol::varint;
use ferrite_protocol::{Compression, Frame, FrameDecoder, FrameEncoder, ProtocolError};
use proptest::prelude::*;

fn encode_all(enc: &mut FrameEncoder, packets: &[(i32, Vec<u8>)]) -> BytesMut {
    let mut out = BytesMut::new();
    for (id, body) in packets {
        enc.encode(*id, body, &mut out).unwrap();
    }
    out
}

fn drain(dec: &mut FrameDecoder) -> Vec<Frame> {
    let mut frames = Vec::new();
    while let Some(f) = dec.next_frame().unwrap() {
        frames.push(f);
    }
    frames
}

fn pseudo_random(len: usize, seed: u32) -> Vec<u8> {
    let mut x = seed.wrapping_mul(2654435761).wrapping_add(1);
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect()
}

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

#[test]
fn uncompressed_wire_format() {
    let mut enc = FrameEncoder::new();
    let out = encode_all(&mut enc, &[(0x21, vec![1, 2, 3])]);
    assert_eq!(&out[..], [4, 0x21, 1, 2, 3]);

    let mut dec = FrameDecoder::new();
    dec.queue(&out);
    assert_eq!(dec.next_frame().unwrap(), Some(Frame { id: 0x21, body: vec![1, 2, 3].into() }));
    assert_eq!(dec.next_frame().unwrap(), None);
    assert_eq!(dec.buffered(), 0);
}

#[test]
fn multi_byte_packet_ids_and_empty_bodies() {
    let mut enc = FrameEncoder::new();
    let out = encode_all(&mut enc, &[(300, vec![]), (-1, vec![9])]);
    let mut dec = FrameDecoder::new();
    dec.queue(&out);
    let frames = drain(&mut dec);
    assert_eq!(frames[0], Frame { id: 300, body: Default::default() });
    assert_eq!(frames[1], Frame { id: -1, body: vec![9].into() });
}

#[test]
fn partial_delivery_one_byte_at_a_time() {
    let packets: Vec<_> = (0..20).map(|i| (i, pseudo_random(i as usize * 37, i as u32))).collect();
    let mut enc = FrameEncoder::new();
    let wire = encode_all(&mut enc, &packets);
    let mut dec = FrameDecoder::new();
    let mut got = Vec::new();
    for byte in wire.iter() {
        dec.queue(&[*byte]);
        got.extend(drain(&mut dec));
    }
    let want: Vec<_> = packets.into_iter().map(|(id, b)| Frame { id, body: b.into() }).collect();
    assert_eq!(got, want);
}

#[test]
fn empty_frames_are_skipped_like_vanilla() {
    let mut dec = FrameDecoder::new();
    dec.queue(&[0, 0, 2, 0x05, 0xaa, 0]);
    assert_eq!(dec.next_frame().unwrap(), Some(Frame { id: 5, body: vec![0xaa].into() }));
    assert_eq!(dec.next_frame().unwrap(), None);
}

#[test]
fn compression_threshold_selects_the_representation() {
    let mut enc = FrameEncoder::new();
    enc.set_compression(Some(Compression { threshold: 64 }), 6).unwrap();

    // Below the threshold: data_length 0 and the payload verbatim.
    let small = encode_all(&mut enc, &[(7, vec![1; 10])]);
    assert_eq!(&small[..3], [12, 0, 7]);
    assert_eq!(small.len(), 1 + 1 + 1 + 10);

    // At or above: data_length is the uncompressed size and the rest is zlib.
    let big = encode_all(&mut enc, &[(7, vec![1; 200])]);
    let mut s = &big[..];
    let frame_len = varint::read_i32(&mut s).unwrap() as usize;
    assert_eq!(frame_len, s.len());
    assert_eq!(varint::read_i32(&mut s).unwrap(), 201);
    assert!(s.len() < 50, "highly compressible data should shrink: {}", s.len());

    let mut dec = FrameDecoder::new();
    dec.set_compression(Some(Compression { threshold: 64 }));
    dec.queue(&small);
    dec.queue(&big);
    let frames = drain(&mut dec);
    assert_eq!(frames[0].body.as_ref(), &[1; 10]);
    assert_eq!(frames[1].body.as_ref(), &[1; 200][..]);

    // Exactly at the threshold is compressed (`payload_len >= threshold`).
    let at = encode_all(&mut enc, &[(7, vec![2; 63])]);
    let mut s = &at[..];
    varint::read_i32(&mut s).unwrap();
    assert_eq!(varint::read_i32(&mut s).unwrap(), 64);
}

#[test]
fn decoder_accepts_streams_from_an_independent_zlib() {
    // Build the frame by hand with flate2 so we are not only testing our own encoder.
    let payload: Vec<u8> = [&[0x10u8][..], &pseudo_random(5000, 1)[..]].concat();
    let packed = zlib(&payload);
    let mut frame = Vec::new();
    let data_len = varint::encode_i32(payload.len() as i32);
    let inner_len = data_len.1 + packed.len();
    let l = varint::encode_i32(inner_len as i32);
    frame.extend_from_slice(&l.0[..l.1]);
    frame.extend_from_slice(&data_len.0[..data_len.1]);
    frame.extend_from_slice(&packed);

    let mut dec = FrameDecoder::new();
    dec.set_compression(Some(Compression { threshold: 256 }));
    dec.queue(&frame);
    let f = dec.next_frame().unwrap().unwrap();
    assert_eq!(f.id, 0x10);
    assert_eq!(f.body.as_ref(), &payload[1..]);
}

#[test]
fn encoder_output_is_readable_by_an_independent_zlib() {
    use std::io::Read;
    let mut enc = FrameEncoder::new();
    enc.set_compression(Some(Compression { threshold: 1 }), 4).unwrap();
    let body = pseudo_random(3000, 9);
    let out = encode_all(&mut enc, &[(0x2a, body.clone())]);
    let mut s = &out[..];
    varint::read_i32(&mut s).unwrap();
    let declared = varint::read_i32(&mut s).unwrap() as usize;
    let mut inflated = Vec::new();
    flate2::read::ZlibDecoder::new(s).read_to_end(&mut inflated).unwrap();
    assert_eq!(inflated.len(), declared);
    assert_eq!(inflated[0], 0x2a);
    assert_eq!(&inflated[1..], &body[..]);
}

#[test]
fn encryption_and_compression_compose() {
    let key = [0x42u8; 16];
    let packets: Vec<_> =
        (0..50).map(|i| (i, pseudo_random((i as usize % 7) * 300, i as u32))).collect();

    let mut enc = FrameEncoder::new();
    enc.set_compression(Some(Compression { threshold: 128 }), 6).unwrap();
    let mut wire = encode_all(&mut enc, &packets);
    Encryptor::new(&key).apply(&mut wire);

    let mut dec = FrameDecoder::new();
    dec.set_compression(Some(Compression { threshold: 128 }));
    dec.enable_encryption(&key);
    // Arbitrary, unaligned delivery through the zero-copy buffer path.
    let mut got = Vec::new();
    for chunk in wire.chunks(97) {
        dec.buffer_mut().extend_from_slice(chunk);
        dec.commit();
        got.extend(drain(&mut dec));
    }
    let want: Vec<_> = packets.into_iter().map(|(id, b)| Frame { id, body: b.into() }).collect();
    assert_eq!(got, want);
}

#[test]
fn encryption_can_start_mid_stream() {
    // The login flow sends plaintext packets, then everything after the key exchange encrypted.
    let key = [9u8; 16];
    let mut enc = FrameEncoder::new();
    let plain = encode_all(&mut enc, &[(1, vec![1, 2, 3])]);
    let mut secret = encode_all(&mut enc, &[(2, vec![4, 5, 6])]);
    Encryptor::new(&key).apply(&mut secret);

    let mut dec = FrameDecoder::new();
    dec.queue(&plain);
    assert_eq!(dec.next_frame().unwrap().unwrap().id, 1);
    dec.enable_encryption(&key);
    dec.queue(&secret);
    assert_eq!(dec.next_frame().unwrap().unwrap().id, 2);
}

#[test]
fn hostile_frames_are_rejected() {
    // 2^21 - 1 is the largest frame vanilla accepts: it is a valid length (and then simply
    // incomplete), 2^21 needs a fourth length byte and is rejected outright.
    let mut dec = FrameDecoder::new();
    dec.queue(&[0xff, 0xff, 0x7f]);
    assert_eq!(dec.next_frame(), Ok(None));
    let mut dec = FrameDecoder::new();
    dec.queue(&[0x80, 0x80, 0x80, 0x01]);
    assert_eq!(dec.next_frame(), Err(ProtocolError::LengthPrefixTooWide { max_bytes: 3 }));
    // Non-canonical 4-byte encoding of a small value.
    let mut dec = FrameDecoder::new();
    dec.queue(&[0x81, 0x80, 0x80, 0x00]);
    assert_eq!(dec.next_frame(), Err(ProtocolError::LengthPrefixTooWide { max_bytes: 3 }));
    // An endless run of continuation bytes is cut off as soon as the prefix could not be valid.
    let mut dec = FrameDecoder::new();
    dec.queue(&[0x80, 0x80, 0x80]);
    assert_eq!(dec.next_frame(), Err(ProtocolError::LengthPrefixTooWide { max_bytes: 3 }));
    let mut dec = FrameDecoder::new();
    dec.queue(&[0x80, 0x80]);
    assert_eq!(dec.next_frame(), Ok(None));
    // A configurable limit applies to the value itself.
    let mut dec = FrameDecoder::new().with_limits(100, 1000);
    dec.queue(&[101]);
    assert_eq!(dec.next_frame(), Err(ProtocolError::FrameTooLarge { len: 101, max: 100 }));

    // A 5-byte (negative) length is too wide for the serverbound limit...
    let mut dec = FrameDecoder::new();
    dec.queue(&[0xff, 0xff, 0xff, 0xff, 0x0f]);
    assert_eq!(dec.next_frame(), Err(ProtocolError::LengthPrefixTooWide { max_bytes: 3 }));
    // ...and, where wide prefixes are allowed (client side), reported as negative.
    let mut dec = FrameDecoder::new().with_limits(i32::MAX as usize, 1000);
    dec.queue(&[0xff, 0xff, 0xff, 0xff, 0x0f]);
    assert_eq!(dec.next_frame(), Err(ProtocolError::NegativeLength(-1)));

    // Overlong length prefix.
    let mut dec = FrameDecoder::new();
    dec.queue(&[0x80; 6]);
    assert_eq!(dec.next_frame(), Err(ProtocolError::VarIntTooLong));

    // Compressed packet that claims a size below the threshold.
    let mut dec = FrameDecoder::new();
    dec.set_compression(Some(Compression { threshold: 100 }));
    dec.queue(&[3, 10, 0, 0]);
    assert!(matches!(
        dec.next_frame(),
        Err(ProtocolError::BelowThreshold { len: 10, threshold: 100 })
    ));

    // Decompression bomb: tiny frame claiming more than the cap.
    let mut dec = FrameDecoder::new().with_limits(1 << 21, 1000);
    dec.set_compression(Some(Compression { threshold: 1 }));
    dec.queue(&[3, 0xe8, 0x07, 0x00]); // 1000 is allowed
    assert_eq!(dec.next_frame(), Err(ProtocolError::Decompress));
    let mut dec = FrameDecoder::new().with_limits(1 << 21, 1000);
    dec.set_compression(Some(Compression { threshold: 1 }));
    dec.queue(&[3, 0xe9, 0x07, 0x00]); // 1001 is not
    assert!(matches!(
        dec.next_frame(),
        Err(ProtocolError::DecompressedTooLarge { len: 1001, max: 1000 })
    ));

    // Corrupt zlib.
    let mut dec = FrameDecoder::new();
    dec.set_compression(Some(Compression { threshold: 1 }));
    dec.queue(&[6, 20, 1, 2, 3, 4, 5]);
    assert_eq!(dec.next_frame(), Err(ProtocolError::Decompress));

    // Declared size disagrees with the stream (too small and too large).
    for declared in [19usize, 21] {
        let packed = zlib(&[7u8; 20]);
        let mut frame = vec![(1 + packed.len()) as u8, declared as u8];
        frame.extend_from_slice(&packed);
        let mut dec = FrameDecoder::new();
        dec.set_compression(Some(Compression { threshold: 1 }));
        dec.queue(&frame);
        assert_eq!(dec.next_frame(), Err(ProtocolError::Decompress), "declared {declared}");
    }

    // Missing packet id.
    let mut dec = FrameDecoder::new();
    dec.set_compression(Some(Compression { threshold: 64 }));
    dec.queue(&[1, 0]);
    assert_eq!(dec.next_frame(), Err(ProtocolError::Eof));
}

#[test]
fn incomplete_frames_wait_without_error() {
    let mut dec = FrameDecoder::new();
    dec.queue(&[10, 1, 2]);
    assert_eq!(dec.next_frame().unwrap(), None);
    assert_eq!(dec.buffered(), 3);
    dec.queue(&[3; 8]);
    assert_eq!(dec.next_frame().unwrap().unwrap().body.len(), 9);
}

proptest! {
    #[test]
    fn any_stream_round_trips(
        packets in proptest::collection::vec((any::<i32>(), proptest::collection::vec(any::<u8>(), 0..600)), 0..12),
        threshold in prop::option::of(0usize..400),
        level in 1i32..=9,
        encrypted in any::<bool>(),
        chunk in 1usize..300,
    ) {
        let compression = threshold.map(|threshold| Compression { threshold });
        let mut enc = FrameEncoder::new();
        enc.set_compression(compression, level).unwrap();
        let mut wire = encode_all(&mut enc, &packets);
        let key = [0x5au8; 16];
        if encrypted { Encryptor::new(&key).apply(&mut wire); }

        let mut dec = FrameDecoder::new();
        dec.set_compression(compression);
        if encrypted { dec.enable_encryption(&key); }
        let mut got = Vec::new();
        for part in wire.chunks(chunk) {
            dec.queue(part);
            while let Some(f) = dec.next_frame().unwrap() { got.push(f); }
        }
        let want: Vec<_> = packets.into_iter().map(|(id, b)| Frame { id, body: b.into() }).collect();
        prop_assert_eq!(got, want);
        prop_assert_eq!(dec.buffered(), 0);
    }

    #[test]
    fn garbage_never_panics(
        bytes in proptest::collection::vec(any::<u8>(), 0..400),
        compressed in any::<bool>(),
    ) {
        let mut dec = FrameDecoder::new();
        if compressed { dec.set_compression(Some(Compression { threshold: 16 })); }
        dec.queue(&bytes);
        for _ in 0..bytes.len() + 1 {
            match dec.next_frame() { Ok(Some(_)) => {}, _ => break }
        }
    }
}

#[test]
fn encrypted_bytes_pipelined_behind_the_last_plaintext_frame_decode_correctly() {
    // A client may send its plaintext `key` packet and, in the same TCP segment, the first packets
    // it encrypts. The decoder must treat everything after the frame it just handed out as
    // ciphertext once encryption is enabled, even though those bytes were already buffered.
    let key = [0x77u8; 16];
    let mut enc = FrameEncoder::new();
    let plain = encode_all(&mut enc, &[(1, vec![1, 2, 3])]);
    let mut secret = encode_all(&mut enc, &[(2, vec![4, 5, 6]), (3, vec![7; 40])]);
    Encryptor::new(&key).apply(&mut secret);

    let mut one_read = plain.to_vec();
    one_read.extend_from_slice(&secret);

    let mut dec = FrameDecoder::new();
    dec.queue(&one_read);
    assert_eq!(dec.next_frame().unwrap().unwrap().id, 1);
    dec.enable_encryption(&key);
    assert_eq!(dec.next_frame().unwrap().unwrap(), Frame { id: 2, body: vec![4, 5, 6].into() });
    assert_eq!(dec.next_frame().unwrap().unwrap(), Frame { id: 3, body: vec![7; 40].into() });
    assert_eq!(dec.buffered(), 0);
}
