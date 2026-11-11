//! Whatever packets we can encode must decode back identically under any settings.
#![no_main]

use bytes::BytesMut;
use ferrite_protocol::cipher::Encryptor;
use ferrite_protocol::{Compression, Frame, FrameDecoder, FrameEncoder};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&config, rest)) = data.split_first() else { return };
    let Some((&chunk, mut rest)) = rest.split_first() else { return };
    let compression =
        (config & 1 != 0).then(|| Compression { threshold: usize::from(config >> 2) });
    let encrypted = config & 2 != 0;
    let key = [config; 16];

    // Carve the rest of the input into packets: [id: 2 bytes][len: 1 byte][body].
    let mut packets = Vec::new();
    while let [a, b, len, tail @ ..] = rest {
        let take = usize::from(*len).min(tail.len());
        let id = i32::from(i16::from_le_bytes([*a, *b]));
        packets.push((id, tail[..take].to_vec()));
        rest = &tail[take..];
    }

    let mut enc = FrameEncoder::new();
    enc.set_compression(compression, 1 + i32::from(config >> 5)).unwrap();
    let mut wire = BytesMut::new();
    for (id, body) in &packets {
        enc.encode(*id, body, &mut wire).unwrap();
    }
    if encrypted {
        Encryptor::new(&key).apply(&mut wire);
    }

    let mut dec = FrameDecoder::new();
    dec.set_compression(compression);
    if encrypted {
        dec.enable_encryption(&key);
    }
    let mut got = Vec::new();
    for part in wire.chunks(usize::from(chunk).max(1)) {
        dec.queue(part);
        while let Some(f) = dec.next_frame().expect("our own output must decode") {
            got.push(f);
        }
    }
    let want: Vec<_> =
        packets.into_iter().map(|(id, b)| Frame { id, body: b.into() }).collect();
    assert_eq!(got, want);
});
