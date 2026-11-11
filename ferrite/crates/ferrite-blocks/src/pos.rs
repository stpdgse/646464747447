//! Block positions and directions.

use ferrite_world::ChunkPos;

/// One of the six axis directions. The discriminants are vanilla's ids and the face order used
/// by [`ferrite_data::shapes`]: `down, up, north, south, west, east`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Direction {
    Down = 0,
    Up = 1,
    North = 2,
    South = 3,
    West = 4,
    East = 5,
}

impl Direction {
    pub const ALL: [Direction; 6] = [
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ];

    /// The horizontal directions in vanilla's plane order.
    pub const HORIZONTAL: [Direction; 4] =
        [Direction::North, Direction::East, Direction::South, Direction::West];

    #[inline]
    pub const fn opposite(self) -> Direction {
        match self {
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
            Direction::East => Direction::West,
        }
    }

    /// Unit step `(dx, dy, dz)`. North is -z, east is +x.
    #[inline]
    pub const fn step(self) -> (i32, i32, i32) {
        match self {
            Direction::Down => (0, -1, 0),
            Direction::Up => (0, 1, 0),
            Direction::North => (0, 0, -1),
            Direction::South => (0, 0, 1),
            Direction::West => (-1, 0, 0),
            Direction::East => (1, 0, 0),
        }
    }

    /// Parses vanilla's lowercase direction names.
    pub fn from_name(name: &str) -> Option<Direction> {
        Some(match name {
            "down" => Direction::Down,
            "up" => Direction::Up,
            "north" => Direction::North,
            "south" => Direction::South,
            "west" => Direction::West,
            "east" => Direction::East,
            _ => return None,
        })
    }

    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The next horizontal direction clockwise seen from above (north -> east -> south -> west).
    pub const fn clockwise(self) -> Direction {
        match self {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
            other => other,
        }
    }

    pub const fn counter_clockwise(self) -> Direction {
        match self {
            Direction::North => Direction::West,
            Direction::West => Direction::South,
            Direction::South => Direction::East,
            Direction::East => Direction::North,
            other => other,
        }
    }

    /// Vanilla's lowercase name, which is also the name of the matching arm property.
    pub const fn name(self) -> &'static str {
        match self {
            Direction::Down => "down",
            Direction::Up => "up",
            Direction::North => "north",
            Direction::South => "south",
            Direction::West => "west",
            Direction::East => "east",
        }
    }

    /// Whether the direction runs along the x axis.
    pub const fn is_x_axis(self) -> bool {
        matches!(self, Direction::West | Direction::East)
    }

    #[inline]
    pub const fn is_horizontal(self) -> bool {
        (self as u8) >= 2
    }
}

/// An absolute block position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub const fn offset(self, d: Direction) -> Self {
        let (dx, dy, dz) = d.step();
        Self { x: self.x + dx, y: self.y + dy, z: self.z + dz }
    }

    #[inline]
    pub const fn below(self) -> Self {
        Self { x: self.x, y: self.y - 1, z: self.z }
    }

    #[inline]
    pub const fn above(self) -> Self {
        Self { x: self.x, y: self.y + 1, z: self.z }
    }

    #[inline]
    pub const fn chunk(self) -> ChunkPos {
        ChunkPos::containing(self.x, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opposites_and_steps_are_consistent() {
        for d in Direction::ALL {
            assert_eq!(d.opposite().opposite(), d);
            let (a, b, c) = d.step();
            let (x, y, z) = d.opposite().step();
            assert_eq!((a + x, b + y, c + z), (0, 0, 0));
            assert_eq!(Direction::ALL[d.index()], d);
            let p = BlockPos::new(3, -5, 7);
            assert_eq!(p.offset(d).offset(d.opposite()), p);
        }
        assert_eq!(BlockPos::new(-1, 0, 16).chunk(), ChunkPos::new(-1, 1));
    }
}
