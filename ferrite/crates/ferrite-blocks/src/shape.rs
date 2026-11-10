//! Geometry queries on block shapes.

use std::collections::HashMap;

use ferrite_data::shapes::{Box6, SHAPES, facts};

use crate::pos::Direction;

const EPS: f64 = 1.0e-7;

fn fuzzy_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < EPS
}

/// The 2D rectangles (`[min_u, min_v, max_u, max_v]`) a shape presents on the block face at the
/// low (`at_max == false`) or high end of `axis` (0 = x, 1 = y, 2 = z). A shape that does not
/// reach that face presents nothing.
fn face_rects(shape: &[Box6], axis: usize, at_max: bool, out: &mut Vec<[f64; 4]>) {
    let (u, v) = match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    for b in shape {
        let touches = if at_max { fuzzy_eq(b[axis + 3], 1.0) } else { fuzzy_eq(b[axis], 0.0) };
        if touches {
            out.push([b[u], b[v], b[u + 3], b[v + 3]]);
        }
    }
}

/// Whether the union of `rects` covers the whole unit square.
fn covers_unit_square(rects: &[[f64; 4]]) -> bool {
    if rects.is_empty() {
        return false;
    }
    if rects.iter().any(|r| r[0] <= EPS && r[1] <= EPS && r[2] >= 1.0 - EPS && r[3] >= 1.0 - EPS) {
        return true;
    }
    // Coordinate compression: the square is covered iff every cell of the grid induced by the
    // rectangle edges is inside some rectangle.
    let mut us = vec![0.0, 1.0];
    let mut vs = vec![0.0, 1.0];
    for r in rects {
        us.extend([r[0].clamp(0.0, 1.0), r[2].clamp(0.0, 1.0)]);
        vs.extend([r[1].clamp(0.0, 1.0), r[3].clamp(0.0, 1.0)]);
    }
    let dedup = |c: &mut Vec<f64>| {
        c.sort_by(f64::total_cmp);
        c.dedup_by(|a, b| fuzzy_eq(*a, *b));
    };
    dedup(&mut us);
    dedup(&mut vs);
    for wu in us.windows(2) {
        let mu = (wu[0] + wu[1]) / 2.0;
        for wv in vs.windows(2) {
            let mv = (wv[0] + wv[1]) / 2.0;
            if !rects.iter().any(|r| r[0] < mu && mu < r[2] && r[1] < mv && mv < r[3]) {
                return false;
            }
        }
    }
    true
}

/// Whether the faces two neighbouring shapes present to each other together seal the boundary
/// between them. `dir` points from the block owning `from` to the block owning `to`.
pub fn merged_face_occludes(from: &[Box6], to: &[Box6], dir: Direction) -> bool {
    let axis = match dir {
        Direction::West | Direction::East => 0,
        Direction::Down | Direction::Up => 1,
        Direction::North | Direction::South => 2,
    };
    let positive = matches!(dir, Direction::East | Direction::Up | Direction::South);
    let mut rects = Vec::new();
    // The `from` block presents its face on the side `dir` points to; `to` presents the opposite.
    face_rects(from, axis, positive, &mut rects);
    face_rects(to, axis, !positive, &mut rects);
    covers_unit_square(&rects)
}

/// Memoises [`merged_face_occludes`] by collision-shape id.
#[derive(Debug, Default)]
pub struct OcclusionCache {
    map: HashMap<(u16, u16, u8), bool>,
}

impl OcclusionCache {
    /// Whether the collision shapes of `from_state` and `to_state` seal the face between them.
    #[inline]
    pub fn collision_faces_occlude(
        &mut self,
        from_state: u16,
        to_state: u16,
        dir: Direction,
    ) -> bool {
        let a = facts(from_state).collision_id();
        let b = facts(to_state).collision_id();
        if a == 0 && b == 0 {
            return false; // two empty shapes, by far the most common case
        }
        *self.map.entry((a, b, dir as u8)).or_insert_with(|| {
            merged_face_occludes(SHAPES[usize::from(a)], SHAPES[usize::from(b)], dir)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: [Box6; 1] = [[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]];
    const BOTTOM: [Box6; 1] = [[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]];
    const TOP: [Box6; 1] = [[0.0, 0.5, 0.0, 1.0, 1.0, 1.0]];

    #[test]
    fn a_full_cube_seals_every_face() {
        for d in Direction::ALL {
            assert!(merged_face_occludes(&FULL, &[], d));
            assert!(merged_face_occludes(&[], &FULL, d));
        }
    }

    #[test]
    fn empty_shapes_seal_nothing() {
        for d in Direction::ALL {
            assert!(!merged_face_occludes(&[], &[], d));
        }
    }

    #[test]
    fn half_slabs_seal_only_where_they_touch_the_face() {
        // Bottom slab: seals its bottom face, not its top; sides are half open.
        assert!(merged_face_occludes(&BOTTOM, &[], Direction::Down));
        assert!(!merged_face_occludes(&BOTTOM, &[], Direction::Up));
        assert!(!merged_face_occludes(&BOTTOM, &[], Direction::North));
    }

    #[test]
    fn a_slab_above_presents_its_low_face() {
        // `to` is above `from`; a bottom slab there touches the shared face, a top slab does not.
        assert!(merged_face_occludes(&[], &BOTTOM, Direction::Up));
        assert!(!merged_face_occludes(&[], &TOP, Direction::Up));
    }

    #[test]
    fn two_halves_can_seal_a_side_together() {
        // A bottom slab next to a top slab: together their side faces cover the whole square.
        assert!(merged_face_occludes(&BOTTOM, &TOP, Direction::East));
        assert!(!merged_face_occludes(&BOTTOM, &BOTTOM, Direction::East));
    }

    #[test]
    fn four_quarters_cover_and_three_do_not() {
        let q = |u: f64, v: f64| [u, v, u + 0.5, v + 0.5];
        let all = [q(0.0, 0.0), q(0.5, 0.0), q(0.0, 0.5), q(0.5, 0.5)];
        assert!(covers_unit_square(&all));
        assert!(!covers_unit_square(&all[..3]));
        assert!(!covers_unit_square(&[]));
    }
}
