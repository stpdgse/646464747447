//! The support rules of plants and other blocks whose needs go beyond "a sturdy face somewhere".
//!
//! Which ground a plant accepts comes from the game's `supports_*` block tags (see
//! [`crate::kinds`]); the rules here are the ones that combine ground with surroundings
//! (cacti, sugar cane, lily pads, chorus, mushrooms). They are checked at placement and, like
//! every support rule, again when the relevant neighbour changes.

use ferrite_data::blocks::block_by_name;
use ferrite_data::shapes::{Support as FaceSupport, facts, flag};

use crate::fluid::{FluidState, Kind};
use crate::kinds::{Kinds, Support, block_id_of, block_tag};
use crate::level::BlockAccess;
use crate::pos::{BlockPos, Direction};
use crate::world::World;

fn is_block(state: u16, name: &str) -> bool {
    block_by_name(name).is_some_and(|b| b.id == block_id_of(state))
}

fn in_tag(state: u16, tag: &str) -> bool {
    block_tag(tag).contains(&block_id_of(state))
}

impl<A: BlockAccess> World<A> {
    /// How lit `pos` is (block light and sky light combined). There is no lighting engine yet,
    /// so everything counts as fully lit, like open sky at noon.
    pub(crate) fn raw_brightness(&self, _pos: BlockPos) -> u8 {
        15
    }

    /// How many steps of scaffolding separate `pos` from something that holds it up (7 = none).
    pub(crate) fn scaffolding_distance(&self, pos: BlockPos) -> u8 {
        let kinds = Kinds::get();
        let scaffolding = block_by_name("scaffolding").map_or(u16::MAX, |b| b.id);
        let below_pos = pos.below();
        let mut best = 7u8;
        match self.block(below_pos) {
            Some(b) if block_id_of(b) == scaffolding => {
                best = kinds.property(b, "distance").and_then(|d| d.parse().ok()).unwrap_or(7);
            }
            Some(b) if facts(b).is_face_sturdy(Direction::Up.index(), FaceSupport::Full) => {
                return 0;
            }
            _ => {}
        }
        for side in Direction::HORIZONTAL {
            if let Some(n) = self.block(pos.offset(side))
                && block_id_of(n) == scaffolding
            {
                let d: u8 = kinds.property(n, "distance").and_then(|d| d.parse().ok()).unwrap_or(7);
                best = best.min(d + 1);
                if best == 1 {
                    break;
                }
            }
        }
        best
    }

    /// Whether `state` (a spike pointing `tip`) has something behind it to grow from: a sturdy
    /// face, or another spike pointing the same way.
    pub(crate) fn spike_base_holds(&self, state: u16, pos: BlockPos, tip: Direction) -> bool {
        let base = pos.offset(tip.opposite());
        self.block(base).is_some_and(|s| {
            facts(s).is_face_sturdy(tip.index(), FaceSupport::Full)
                || self.is_spike_pointing(state, s, tip)
        })
    }

    /// Whether `other` is the same kind of spike as `like` pointing in direction `tip`.
    pub(crate) fn is_spike_pointing(&self, like: u16, other: u16, tip: Direction) -> bool {
        block_id_of(other) == block_id_of(like)
            && Kinds::get().property(other, "vertical_direction") == Some(tip.name())
    }

    /// Whether a chorus plant joins `neighbor`, which lies in direction `dir` from it.
    pub(crate) fn chorus_joins(neighbor: u16, dir: Direction) -> bool {
        is_block(neighbor, "chorus_plant")
            || is_block(neighbor, "chorus_flower")
            || (dir == Direction::Down && is_block(neighbor, "end_stone"))
    }

    /// A chorus plant with all six arms set from the blocks around `pos`.
    pub(crate) fn chorus_plant_arms(&self, state: u16, pos: BlockPos) -> u16 {
        let kinds = Kinds::get();
        Direction::ALL.into_iter().fold(state, |s, dir| {
            let joins = self.block(pos.offset(dir)).is_some_and(|n| Self::chorus_joins(n, dir));
            kinds.with_property(s, dir.name(), if joins { "true" } else { "false" }).unwrap_or(s)
        })
    }

    fn fluid_at(&self, pos: BlockPos) -> FluidState {
        self.block(pos).map_or(FluidState::EMPTY, FluidState::of)
    }

    /// The rules of [`Support`] variants too involved to sit in `World::survives`.
    pub(crate) fn survives_special(
        &self,
        rule: Support,
        state: u16,
        dir: Direction,
        pos: BlockPos,
    ) -> bool {
        let below_pos = pos.below();
        let below = self.block(below_pos);
        let sturdy = |at: BlockPos, face: Direction, support: FaceSupport| {
            self.block(at).is_some_and(|s| facts(s).is_face_sturdy(face.index(), support))
        };
        match rule {
            Support::SturdyBelow => {
                dir != Direction::Down || sturdy(below_pos, Direction::Up, FaceSupport::Full)
            }
            Support::FullAbove => {
                if dir != Direction::Up {
                    return true;
                }
                self.block(pos.above()).is_some_and(|s| {
                    block_id_of(s) == block_id_of(state)
                        || facts(s).is_face_sturdy(Direction::Down.index(), FaceSupport::Full)
                        // Hanging moss also hangs from full cubes that are not sturdy (leaves);
                        // hanging roots do not.
                        || (!is_block(state, "hanging_roots")
                            && crate::shape::merged_face_occludes(
                                facts(s).collision_shape(),
                                &[],
                                Direction::Down,
                            ))
                })
            }
            Support::AnyFaceBelow => {
                dir != Direction::Down
                    || below.is_some_and(|s| {
                        crate::shape::presents_face(facts(s).collision_shape(), Direction::Up)
                            || facts(s).is_face_sturdy(Direction::Up.index(), FaceSupport::Center)
                            || facts(s).is_face_sturdy(Direction::Up.index(), FaceSupport::Full)
                    })
            }
            Support::Cactus => {
                for side in Direction::HORIZONTAL {
                    let at = pos.offset(side);
                    let solid = self.block(at).is_some_and(|s| facts(s).has(flag::SOLID));
                    let lava = self.fluid_at(at).is(Kind::Lava) && !self.fluid_at(at).is_empty();
                    if solid || lava {
                        return false;
                    }
                }
                let ground = below.is_some_and(|s| {
                    is_block(s, "cactus") || in_tag(s, "minecraft:supports_cactus")
                });
                let liquid_above =
                    self.block(pos.above()).is_some_and(|s| facts(s).has(flag::LIQUID));
                ground && !liquid_above
            }
            Support::CactusFlower => {
                dir != Direction::Down
                    || below.is_some_and(|s| {
                        in_tag(s, "minecraft:support_override_cactus_flower")
                            || facts(s).is_face_sturdy(Direction::Up.index(), FaceSupport::Center)
                    })
            }
            Support::SugarCane => {
                let Some(b) = below else { return false };
                if block_id_of(b) == block_id_of(state) {
                    return true;
                }
                if !in_tag(b, "minecraft:supports_sugar_cane") {
                    return false;
                }
                Direction::HORIZONTAL.into_iter().any(|side| {
                    let at = below_pos.offset(side);
                    let wet = {
                        let f = self.fluid_at(at);
                        f.is(Kind::Water) && !f.is_empty()
                    };
                    wet || self
                        .block(at)
                        .is_some_and(|s| in_tag(s, "minecraft:supports_sugar_cane_adjacently"))
                })
            }
            Support::LilyPad => {
                let Some(b) = below else { return false };
                let water = {
                    let f = FluidState::of(b);
                    f.is(Kind::Water) && f.source
                };
                (water || in_tag(b, "minecraft:supports_lily_pad"))
                    && self.fluid_at(pos.above()).is_empty()
            }
            Support::Frogspawn => {
                let water_below = below.is_some_and(|b| {
                    let f = FluidState::of(b);
                    f.is(Kind::Water) && f.source
                });
                water_below && self.fluid_at(pos).is_empty()
            }
            Support::Mushroom => {
                let Some(b) = below else { return false };
                if in_tag(b, "minecraft:overrides_mushroom_light_requirement") {
                    return true;
                }
                self.raw_brightness(pos) < 13 && facts(b).has(flag::SOLID_RENDER)
            }
            Support::ChorusPlant => {
                let Some(b) = below else { return false };
                let above = self.block(pos.above());
                let both_solid =
                    !above.is_none_or(|s| facts(s).has(flag::AIR)) && !facts(b).has(flag::AIR);
                let is_plant = |s: u16| is_block(s, "chorus_plant");
                for side in Direction::HORIZONTAL {
                    let at = pos.offset(side);
                    if self.block(at).is_some_and(is_plant) {
                        if both_solid {
                            return false;
                        }
                        if self.block(at.below()).is_some_and(|s| {
                            is_plant(s) || in_tag(s, "minecraft:supports_chorus_plant")
                        }) {
                            return true;
                        }
                    }
                }
                is_plant(b) || in_tag(b, "minecraft:supports_chorus_plant")
            }
            Support::ChorusFlower => {
                let Some(b) = below else { return false };
                if is_block(b, "chorus_plant") || in_tag(b, "minecraft:supports_chorus_flower") {
                    return true;
                }
                if !facts(b).has(flag::AIR) {
                    return false;
                }
                let mut found = false;
                for side in Direction::HORIZONTAL {
                    let Some(n) = self.block(pos.offset(side)) else { return false };
                    if is_block(n, "chorus_plant") {
                        if found {
                            return false;
                        }
                        found = true;
                    } else if !facts(n).has(flag::AIR) {
                        return false;
                    }
                }
                found
            }
            Support::WaterPlant => {
                dir != Direction::Down
                    || below.is_some_and(|s| {
                        facts(s).is_face_sturdy(Direction::Up.index(), FaceSupport::Full)
                            && !in_tag(s, "minecraft:cannot_support_kelp")
                    })
            }
            Support::Growing { up, same } => {
                let behind_dir = if up { Direction::Down } else { Direction::Up };
                if dir != behind_dir {
                    return true;
                }
                let Some(b) = self.block(pos.offset(behind_dir)) else { return false };
                if in_tag(b, "minecraft:cannot_support_kelp")
                    && same.len() == 2
                    && is_block(state, "kelp")
                {
                    return false;
                }
                same.contains(&block_id_of(b))
                    || facts(b).is_face_sturdy(behind_dir.opposite().index(), FaceSupport::Full)
            }
            Support::Bell => {
                let kinds = Kinds::get();
                match kinds.property(state, "attachment") {
                    Some("floor") => {
                        dir != Direction::Down
                            || sturdy(below_pos, Direction::Up, FaceSupport::Full)
                    }
                    Some("ceiling") => {
                        dir != Direction::Up
                            || sturdy(pos.above(), Direction::Down, FaceSupport::Center)
                    }
                    _ => {
                        let facing = kinds
                            .property(state, "facing")
                            .and_then(Direction::from_name)
                            .unwrap_or(Direction::North);
                        dir != facing
                            || sturdy(pos.offset(facing), facing.opposite(), FaceSupport::Full)
                    }
                }
            }
            Support::Propagule => {
                if Kinds::get().property(state, "hanging") == Some("true") {
                    self.block(pos.above())
                        .is_some_and(|s| in_tag(s, "minecraft:supports_hanging_mangrove_propagule"))
                } else {
                    below.is_some_and(|s| in_tag(s, "minecraft:supports_mangrove_propagule"))
                }
            }
            Support::Scaffolding => self.scaffolding_distance(pos) < 7,
            Support::Spike => {
                let tip = Kinds::get()
                    .property(state, "vertical_direction")
                    .and_then(Direction::from_name)
                    .unwrap_or(Direction::Down);
                self.spike_base_holds(state, pos, tip)
            }
            _ => true,
        }
    }
}
