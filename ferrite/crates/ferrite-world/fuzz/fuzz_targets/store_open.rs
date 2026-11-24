//! Arbitrary bytes as the contents of a segment file. Opening must never panic, and whatever
//! survives recovery must be readable without panicking.
#![no_main]

use ferrite_world::store::{Options, Store};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let dir = tempfile::tempdir().unwrap();
    // Plant a plausible record at the start half of the time so the scan gets past the first byte.
    let mut bytes = Vec::new();
    if data.first().is_some_and(|b| b & 1 == 1) {
        bytes.extend_from_slice(b"FRC1\x01");
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&2i32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
    }
    bytes.extend_from_slice(data);
    std::fs::write(dir.path().join("seg-00000001.log"), &bytes).unwrap();
    if let Ok(store) = Store::open(dir.path(), Options::default()) {
        for key in store.keys() {
            let _ = store.get(key);
        }
        // A recovered store must accept new writes.
        store.put(ferrite_world::ChunkPos::new(0, 0), b"x").unwrap();
        assert_eq!(store.get(ferrite_world::ChunkPos::new(0, 0)).unwrap().unwrap(), b"x");
    }
});
