/// A fixed-length array of `bits`-wide unsigned integers packed into `u64` words.
///
/// Vanilla's rule: entries never straddle a word boundary, so each word holds
/// `64 / bits` entries and the high bits of a word may be unused. That wastes a little space for
/// widths that do not divide 64, and makes every access a single shift and mask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackedArray {
    bits: u8,
    len: usize,
    data: Vec<u64>,
}

impl PackedArray {
    pub fn new(bits: u8, len: usize) -> Self {
        assert!((1..=32).contains(&bits), "unsupported entry width {bits}");
        let per_word = 64 / usize::from(bits);
        Self { bits, len, data: vec![0; len.div_ceil(per_word)] }
    }

    /// Words needed for `len` entries of `bits` bits.
    pub fn words_for(bits: u8, len: usize) -> usize {
        len.div_ceil(64 / usize::from(bits))
    }

    /// Wraps existing words (as read from disk). Fails if the length is wrong.
    pub fn from_words(bits: u8, len: usize, data: Vec<u64>) -> Option<Self> {
        ((1..=32).contains(&bits) && data.len() == Self::words_for(bits, len)).then_some(Self {
            bits,
            len,
            data,
        })
    }

    pub fn bits(&self) -> u8 {
        self.bits
    }

    pub fn words(&self) -> &[u64] {
        &self.data
    }

    #[inline]
    fn locate(&self, i: usize) -> (usize, u32) {
        let per_word = 64 / usize::from(self.bits);
        (i / per_word, ((i % per_word) * usize::from(self.bits)) as u32)
    }

    #[inline]
    fn mask(&self) -> u64 {
        (1u64 << self.bits) - 1
    }

    #[inline]
    pub fn get(&self, i: usize) -> u32 {
        assert!(i < self.len, "index {i} out of range for {} entries", self.len);
        let (word, shift) = self.locate(i);
        ((self.data[word] >> shift) & self.mask()) as u32
    }

    #[inline]
    pub fn set(&mut self, i: usize, value: u32) {
        assert!(i < self.len, "index {i} out of range for {} entries", self.len);
        debug_assert!(
            u64::from(value) <= self.mask(),
            "{value} does not fit in {} bits",
            self.bits
        );
        let (word, shift) = self.locate(i);
        let mask = self.mask();
        self.data[word] =
            (self.data[word] & !(mask << shift)) | ((u64::from(value) & mask) << shift);
    }

    /// Calls `f` with every entry in order, unpacking whole words at a time (no per-entry bounds
    /// checks), which is what bulk passes over a container should use.
    #[inline]
    pub fn for_each(&self, mut f: impl FnMut(u32)) {
        let per_word = 64 / usize::from(self.bits);
        let mask = self.mask();
        let bits = u32::from(self.bits);
        let mut remaining = self.len;
        for &word in &self.data {
            let n = per_word.min(remaining);
            let mut w = word;
            for _ in 0..n {
                f((w & mask) as u32);
                w >>= bits;
            }
            remaining -= n;
        }
    }

    /// Whether every entry is strictly below `limit`.
    pub fn all_below(&self, limit: u32) -> bool {
        let mut ok = true;
        self.for_each(|v| ok &= v < limit);
        ok
    }

    /// Builds an array by mapping every entry of `self` through `map` into a new width.
    pub fn remapped(&self, new_bits: u8, map: impl Fn(u32) -> u32) -> PackedArray {
        let mut out = PackedArray::new(new_bits, self.len);
        let mut i = 0;
        self.for_each(|v| {
            out.set(i, map(v));
            i += 1;
        });
        out
    }
}
