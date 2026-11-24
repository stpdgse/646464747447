//! Feeds arbitrary bytes to the frame decoder in arbitrary chunks, with every combination of
//! compression and encryption. It must never panic, never over-allocate, and never loop forever.
#![no_main]

use ferrite_protocol::{Compression, FrameDecoder};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&config, rest)) = data.split_first() else { return };
    let Some((&chunk, rest)) = rest.split_first() else { return };

    let mut dec = FrameDecoder::new().with_limits(1 << 16, 1 << 18);
    if config & 1 != 0 {
        dec.set_compression(Some(Compression { threshold: usize::from(config >> 2) }));
    }
    if config & 2 != 0 {
        dec.enable_encryption(&[config; 16]);
    }
    let chunk = usize::from(chunk).max(1);
    let mut frames = 0usize;
    for part in rest.chunks(chunk) {
        dec.queue(part);
        loop {
            match dec.next_frame() {
                Ok(Some(f)) => {
                    frames += 1;
                    assert!(f.body.len() <= 1 << 18);
                }
                Ok(None) => break,
                Err(_) => return,
            }
        }
    }
    // Every frame needs at least one length byte; more than the input length is impossible.
    assert!(frames <= rest.len());
});
