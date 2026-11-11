/// A tiny, fixed, platform-independent 64-bit hasher for fingerprinting simulation state.
///
/// `std`'s hashers are randomly seeded or may change between releases, which would make "same
/// state, same hash" unprovable. This one is defined entirely by the code below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StableHasher(u64);

impl Default for StableHasher {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl StableHasher {
    pub fn write_u64(&mut self, v: u64) {
        self.0 = (self.0 ^ v).rotate_left(23).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (v >> 29);
    }

    /// Hashes the exact bit pattern, so `-0.0 != 0.0` and NaN payloads count.
    pub fn write_f64(&mut self, v: f64) {
        self.write_u64(v.to_bits());
    }

    pub fn finish(self) -> u64 {
        let mut z = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}
