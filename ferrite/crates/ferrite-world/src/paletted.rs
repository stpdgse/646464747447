use crate::packed::PackedArray;

/// Which kind of data a container holds; this fixes its width rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// 16x16x16 block-state ids. Indirect palettes use 4 to 8 bits per entry, larger ones store ids
    /// directly in 16 bits.
    Blocks,
    /// 4x4x4 biome ids. Indirect palettes use 1 to 3 bits, larger ones store ids directly in 7.
    Biomes,
}

impl Kind {
    pub const fn len(self) -> usize {
        match self {
            Kind::Blocks => 4096,
            Kind::Biomes => 64,
        }
    }

    pub const fn is_empty(self) -> bool {
        false
    }

    pub const fn min_bits(self) -> u8 {
        match self {
            Kind::Blocks => 4,
            Kind::Biomes => 1,
        }
    }

    pub const fn max_indirect_bits(self) -> u8 {
        match self {
            Kind::Blocks => 8,
            Kind::Biomes => 3,
        }
    }

    pub const fn direct_bits(self) -> u8 {
        match self {
            Kind::Blocks => 16,
            Kind::Biomes => 7,
        }
    }

    /// Largest id that fits the direct width.
    pub const fn max_value(self) -> u16 {
        ((1u32 << self.direct_bits()) - 1) as u16
    }
}

/// `ceil(log2(n))`, with 0 for `n <= 1`.
fn ceil_log2(n: usize) -> u8 {
    if n <= 1 { 0 } else { (usize::BITS - (n - 1).leading_zeros()) as u8 }
}

#[derive(Clone, Debug)]
enum Storage {
    /// Every entry has the same value.
    Single(u16),
    /// Entries are indices into `palette`.
    Indirect { palette: Vec<u16>, data: PackedArray },
    /// Entries are the values themselves.
    Direct(PackedArray),
}

/// A cube of ids stored compactly: one value, a palette plus packed indices, or packed raw values,
/// whichever the contents need. It grows its representation on demand as new values appear.
#[derive(Clone, Debug)]
pub struct PalettedContainer {
    kind: Kind,
    storage: Storage,
}

impl PalettedContainer {
    /// A container where every entry is `value`.
    pub fn filled(kind: Kind, value: u16) -> Self {
        assert!(value <= kind.max_value(), "value {value} does not fit a {kind:?} container");
        Self { kind, storage: Storage::Single(value) }
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn len(&self) -> usize {
        self.kind.len()
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn get(&self, i: usize) -> u16 {
        match &self.storage {
            Storage::Single(v) => {
                assert!(i < self.len(), "index {i} out of range");
                *v
            }
            Storage::Indirect { palette, data } => palette[data.get(i) as usize],
            Storage::Direct(data) => data.get(i) as u16,
        }
    }

    pub fn set(&mut self, i: usize, value: u16) {
        assert!(i < self.len(), "index {i} out of range");
        assert!(
            value <= self.kind.max_value(),
            "value {value} does not fit a {:?} container",
            self.kind
        );
        match &mut self.storage {
            Storage::Single(current) => {
                if *current == value {
                    return;
                }
                // Turn into a two-entry palette: index 0 is the old value, everything points at it.
                let bits = self.kind.min_bits();
                let mut data = PackedArray::new(bits, self.kind.len());
                data.set(i, 1);
                self.storage = Storage::Indirect { palette: vec![*current, value], data };
            }
            Storage::Indirect { palette, data } => {
                let index = match palette.iter().position(|&p| p == value) {
                    Some(index) => index,
                    None => {
                        palette.push(value);
                        palette.len() - 1
                    }
                };
                if index >= 1usize << data.bits() {
                    self.grow(i, index, value);
                } else {
                    data.set(i, index as u32);
                }
            }
            Storage::Direct(data) => data.set(i, u32::from(value)),
        }
    }

    /// The palette no longer fits its index width: widen it, or give up on the palette entirely.
    fn grow(&mut self, i: usize, index: usize, value: u16) {
        let Storage::Indirect { palette, data } = &mut self.storage else { unreachable!() };
        let new_bits = data.bits() + 1;
        if new_bits > self.kind.max_indirect_bits() {
            let mut direct = PackedArray::new(self.kind.direct_bits(), self.kind.len());
            for j in 0..self.kind.len() {
                direct.set(j, u32::from(palette[data.get(j) as usize]));
            }
            direct.set(i, u32::from(value));
            self.storage = Storage::Direct(direct);
        } else {
            let mut wider = PackedArray::new(new_bits, self.kind.len());
            for j in 0..self.kind.len() {
                wider.set(j, data.get(j));
            }
            wider.set(i, index as u32);
            *data = wider;
        }
    }

    /// All values in order.
    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        (0..self.len()).map(|i| self.get(i))
    }

    /// Whether every entry is the same, and if so which value.
    pub fn uniform(&self) -> Option<u16> {
        match &self.storage {
            Storage::Single(v) => Some(*v),
            _ => {
                let first = self.get(0);
                self.iter().all(|v| v == first).then_some(first)
            }
        }
    }

    /// The same contents in canonical form: a single value if uniform, otherwise the smallest
    /// palette (sorted ascending) at the narrowest width, or direct storage when the palette would
    /// be too wide. Two containers with equal contents have equal canonical forms, which is what
    /// makes serialised chunks deterministic.
    ///
    /// Indirect containers only need their *palette* sorted and their indices remapped, never a
    /// sort of all 4096 entries, and a container that is already canonical is returned as is.
    pub fn canonical(&self) -> Self {
        match &self.storage {
            Storage::Single(_) => self.clone(),
            Storage::Indirect { palette, data } => self.canonical_indirect(palette, data),
            Storage::Direct(data) => self.canonical_direct(data),
        }
    }

    fn canonical_indirect(&self, palette: &[u16], data: &PackedArray) -> Self {
        // Which palette entries are actually referenced?
        let mut used = [false; 256];
        data.for_each(|i| used[i as usize] = true);
        let mut values: Vec<u16> =
            palette.iter().enumerate().filter(|(i, _)| used[*i]).map(|(_, &v)| v).collect();
        values.sort_unstable();
        values.dedup();
        if values.len() == 1 {
            return Self::filled(self.kind, values[0]);
        }
        let bits = ceil_log2(values.len()).max(self.kind.min_bits());
        if palette == values.as_slice() && bits == data.bits() {
            return self.clone(); // already canonical
        }
        // old index -> new index, through the value (two old indices may hold the same value).
        let mut remap = [0u32; 256];
        for (i, &v) in palette.iter().enumerate() {
            if used[i] {
                remap[i] =
                    values.binary_search(&v).expect("a used value is in the new palette") as u32;
            }
        }
        let data = data.remapped(bits, |old| remap[old as usize]);
        Self { kind: self.kind, storage: Storage::Indirect { palette: values, data } }
    }

    fn canonical_direct(&self, data: &PackedArray) -> Self {
        // A bitset over every possible id: cheaper than sorting 4096 values.
        let mut seen = vec![0u64; (usize::from(self.kind.max_value()) + 1).div_ceil(64)];
        data.for_each(|v| seen[(v >> 6) as usize] |= 1 << (v & 63));
        let mut values: Vec<u16> = Vec::new();
        for (w, &word) in seen.iter().enumerate() {
            let mut bits = word;
            while bits != 0 {
                values.push((w * 64 + bits.trailing_zeros() as usize) as u16);
                bits &= bits - 1;
            }
        }
        if values.len() == 1 {
            return Self::filled(self.kind, values[0]);
        }
        let bits = ceil_log2(values.len()).max(self.kind.min_bits());
        if bits > self.kind.max_indirect_bits() {
            return self.clone(); // genuinely needs direct storage
        }
        let mut index = vec![0u32; usize::from(self.kind.max_value()) + 1];
        for (i, &v) in values.iter().enumerate() {
            index[usize::from(v)] = i as u32;
        }
        let data = data.remapped(bits, |v| index[v as usize]);
        Self { kind: self.kind, storage: Storage::Indirect { palette: values, data } }
    }

    /// Whether every id that can appear satisfies `ok`. For an indirect container only the palette
    /// is checked (an unused palette entry is rejected too, which a canonical writer never emits),
    /// so this costs a few dozen checks instead of 4096.
    pub fn all_ids(&self, ok: impl Fn(u16) -> bool) -> bool {
        match &self.storage {
            Storage::Single(v) => ok(*v),
            Storage::Indirect { palette, .. } => palette.iter().all(|&v| ok(v)),
            Storage::Direct(data) => {
                let mut all = true;
                data.for_each(|v| all &= ok(v as u16));
                all
            }
        }
    }

    /// A view of the representation, for serialisation.
    pub fn repr(&self) -> Repr<'_> {
        match &self.storage {
            Storage::Single(v) => Repr::Single(*v),
            Storage::Indirect { palette, data } => {
                Repr::Indirect { palette, data: data.words(), bits: data.bits() }
            }
            Storage::Direct(data) => Repr::Direct { data: data.words() },
        }
    }

    /// Rebuilds a container from a decoded representation. Every structural property is checked;
    /// the caller has already checked that ids are in range for the world.
    pub fn from_repr(kind: Kind, repr: OwnedRepr) -> Option<Self> {
        let storage = match repr {
            OwnedRepr::Single(v) => {
                if v > kind.max_value() {
                    return None;
                }
                Storage::Single(v)
            }
            OwnedRepr::Indirect { palette, data, bits } => {
                if !(kind.min_bits()..=kind.max_indirect_bits()).contains(&bits)
                    || palette.is_empty()
                    || palette.len() > 1usize << bits
                    || palette.iter().any(|&p| p > kind.max_value())
                {
                    return None;
                }
                let data = PackedArray::from_words(bits, kind.len(), data)?;
                if !data.all_below(palette.len() as u32) {
                    return None;
                }
                Storage::Indirect { palette, data }
            }
            OwnedRepr::Direct { data } => {
                let data = PackedArray::from_words(kind.direct_bits(), kind.len(), data)?;
                if !data.all_below(u32::from(kind.max_value()) + 1) {
                    return None;
                }
                Storage::Direct(data)
            }
        };
        Some(Self { kind, storage })
    }

    /// Approximate heap size in bytes.
    pub fn heap_size(&self) -> usize {
        match &self.storage {
            Storage::Single(_) => 0,
            Storage::Indirect { palette, data } => palette.len() * 2 + data.words().len() * 8,
            Storage::Direct(data) => data.words().len() * 8,
        }
    }
}

impl PartialEq for PalettedContainer {
    /// Equal when the contents are equal, whatever the representation.
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.iter().eq(other.iter())
    }
}
impl Eq for PalettedContainer {}

/// Borrowed view of a container's representation.
#[derive(Debug)]
pub enum Repr<'a> {
    Single(u16),
    Indirect { palette: &'a [u16], data: &'a [u64], bits: u8 },
    Direct { data: &'a [u64] },
}

/// Owned representation, as decoded from bytes.
#[derive(Debug)]
pub enum OwnedRepr {
    Single(u16),
    Indirect { palette: Vec<u16>, data: Vec<u64>, bits: u8 },
    Direct { data: Vec<u64> },
}
