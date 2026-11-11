//! The client-side reply parsers must survive any bytes a malicious server could send.
#![no_main]

use ferrite_remote::query::{parse_basic, parse_full, parse_handshake};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = parse_handshake(data);
    let _ = parse_basic(data);
    if let Ok(stat) = parse_full(data) {
        // Whatever parsed must be internally consistent.
        assert!(stat.fields.iter().all(|(k, _)| !k.is_empty()));
        assert!(stat.players.iter().all(|p| !p.is_empty()));
    }
});
