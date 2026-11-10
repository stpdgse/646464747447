//! Arbitrary bytes into the chunk decoder, raw and compressed, with and without a valid header in
//! front. Anything that decodes must re-encode and decode to the identical chunk.
#![no_main]

use ferrite_world::codec::{decode, decode_raw, encode_raw};
use ferrite_world::Limits;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let limits = Limits::vanilla();
    let _ = decode(data, &limits);

    let mut framed = b"FCK1\x01\x00".to_vec();
    framed.extend_from_slice(&0i32.to_le_bytes());
    framed.extend_from_slice(&((data.first().copied().unwrap_or(1) as i32 % 4 + 1) * 16).to_le_bytes());
    framed.extend_from_slice(data.get(1..).unwrap_or(&[]));
    for candidate in [data, &framed[..]] {
        if let Ok(chunk) = decode_raw(candidate, &limits) {
            let again = decode_raw(&encode_raw(&chunk), &limits).expect("our own output must decode");
            assert_eq!(again, chunk);
        }
    }
});
