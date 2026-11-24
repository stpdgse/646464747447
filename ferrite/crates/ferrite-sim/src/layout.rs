/// Index of a region in a [`Layout`], row-major.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RegionId(pub u32);

/// An axis-aligned rectangle, half-open: `min <= p < max`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: [f64; 2],
    pub max: [f64; 2],
}

impl Bounds {
    pub fn contains(&self, p: [f64; 2]) -> bool {
        p[0] >= self.min[0] && p[0] < self.max[0] && p[1] >= self.min[1] && p[1] < self.max[1]
    }

    /// Grows the rectangle by `by` on every side.
    pub fn expanded(&self, by: f64) -> Bounds {
        Bounds {
            min: [self.min[0] - by, self.min[1] - by],
            max: [self.max[0] + by, self.max[1] + by],
        }
    }
}

/// A `cols x rows` grid of square regions of side `cell`, starting at the origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub cols: u32,
    pub rows: u32,
    pub cell: f64,
}

impl Layout {
    pub fn new(cols: u32, rows: u32, cell: f64) -> Self {
        assert!(cols > 0 && rows > 0 && cell > 0.0, "a layout needs at least one region");
        Self { cols, rows, cell }
    }

    pub fn len(&self) -> usize {
        (self.cols * self.rows) as usize
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    /// Total world width and height.
    pub fn extent(&self) -> [f64; 2] {
        [f64::from(self.cols) * self.cell, f64::from(self.rows) * self.cell]
    }

    pub fn id(&self, col: u32, row: u32) -> RegionId {
        RegionId(row * self.cols + col)
    }

    pub fn coords(&self, id: RegionId) -> (u32, u32) {
        (id.0 % self.cols, id.0 / self.cols)
    }

    pub fn bounds(&self, id: RegionId) -> Bounds {
        let (c, r) = self.coords(id);
        let (x, y) = (f64::from(c) * self.cell, f64::from(r) * self.cell);
        Bounds { min: [x, y], max: [x + self.cell, y + self.cell] }
    }

    /// The region that owns point `p`. Points outside the world belong to the nearest edge region.
    pub fn region_of(&self, p: [f64; 2]) -> RegionId {
        let clamp = |v: f64, n: u32| ((v / self.cell).floor().max(0.0) as u32).min(n - 1);
        self.id(clamp(p[0], self.cols), clamp(p[1], self.rows))
    }

    /// The regions adjacent to `id`, including diagonals, in a fixed order.
    pub fn neighbors(&self, id: RegionId) -> impl Iterator<Item = RegionId> + '_ {
        let (c, r) = self.coords(id);
        (-1i64..=1).flat_map(move |dr| (-1i64..=1).map(move |dc| (dc, dr))).filter_map(
            move |(dc, dr)| {
                if dc == 0 && dr == 0 {
                    return None;
                }
                let (nc, nr) = (i64::from(c) + dc, i64::from(r) + dr);
                (nc >= 0 && nr >= 0 && nc < i64::from(self.cols) && nr < i64::from(self.rows))
                    .then(|| self.id(nc as u32, nr as u32))
            },
        )
    }
}
