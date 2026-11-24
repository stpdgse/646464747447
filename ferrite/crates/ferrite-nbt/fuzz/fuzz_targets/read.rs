//! Arbitrary bytes into both NBT readers under tight and loose limits. Anything that parses must
//! re-encode to bytes that parse and re-encode to the *same bytes* (compared as bytes, not as
//! tags, because `NaN != NaN` under IEEE equality), and limits must never be exceeded.
#![no_main]

use ferrite_nbt::{Limits, read_named, read_network, write_named, write_network};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for limits in [Limits::new(512, 4), Limits::new(1 << 16, 32), Limits::default()] {
        let mut s = data;
        if let Ok(Some(tag)) = read_network(&mut s, limits) {
            let mut out = Vec::new();
            if write_network(&tag, &mut out).is_ok() {
                let mut again = &out[..];
                let reparsed = read_network(&mut again, Limits::UNLIMITED).unwrap().unwrap();
                assert!(again.is_empty());
                let mut out2 = Vec::new();
                write_network(&reparsed, &mut out2).unwrap();
                assert_eq!(out, out2, "encoding must be a fixed point");
            }
        }
        let mut s = data;
        if let Ok((name, tag)) = read_named(&mut s, limits) {
            let mut out = Vec::new();
            if write_named(&name, &tag, &mut out).is_ok() {
                let mut again = &out[..];
                let (name2, reparsed) = read_named(&mut again, Limits::UNLIMITED).unwrap();
                assert_eq!(name, name2);
                let mut out2 = Vec::new();
                write_named(&name2, &reparsed, &mut out2).unwrap();
                assert_eq!(out, out2, "encoding must be a fixed point");
            }
        }
    }
});
