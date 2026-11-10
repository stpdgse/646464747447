//! Sanity checks of the extracted shape table against facts that are true of vanilla by
//! construction (so a transposed column or shifted id shows up immediately).

use ferrite_data::blocks::{block_by_name, block_of_state};
use ferrite_data::shapes::{Support, facts, flag};

const DOWN: usize = 0;
const UP: usize = 1;
const NORTH: usize = 2;

fn default_state(name: &str) -> u16 {
    block_by_name(name).unwrap_or_else(|| panic!("no block {name}")).default_state
}

#[test]
fn air_is_empty_and_flagged() {
    let f = facts(default_state("air"));
    assert!(f.has(flag::AIR));
    assert!(f.collision_shape().is_empty());
    assert_eq!(f.fluid_amount(), 0);
}

#[test]
fn stone_is_a_full_solid_cube() {
    let f = facts(default_state("stone"));
    assert!(f.has(flag::SOLID | flag::CAN_OCCLUDE | flag::REDSTONE_CONDUCTOR));
    assert!(f.has(flag::FULL_COLLISION_BLOCK));
    assert_eq!(f.collision_shape(), &[[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]);
    for face in 0..6 {
        assert!(f.is_face_sturdy(face, Support::Full), "face {face}");
        assert!(f.is_face_sturdy(face, Support::Center));
        assert!(f.is_face_sturdy(face, Support::Rigid));
    }
}

#[test]
fn glass_is_solid_looking_but_does_not_conduct() {
    let f = facts(default_state("glass"));
    assert!(!f.has(flag::REDSTONE_CONDUCTOR));
    assert!(f.has(flag::FULL_COLLISION_BLOCK));
}

#[test]
fn fluid_states_carry_amount_and_source_flags() {
    let water = block_by_name("water").unwrap();
    let mut seen_source = false;
    for level in 0..16u8 {
        let state = water.state_id(&[level]).unwrap();
        let f = facts(state);
        assert!(f.fluid_amount() >= 1, "level {level} has fluid");
        assert_eq!(f.fluid_is_source(), level == 0);
        assert_eq!(f.fluid_is_falling(), level >= 8, "level {level}");
        match level {
            0 => assert_eq!(f.fluid_amount(), 8),
            1..=7 => assert_eq!(usize::from(f.fluid_amount()), 8 - usize::from(level)),
            _ => assert_eq!(f.fluid_amount(), 8),
        }
        seen_source |= f.fluid_is_source();
        assert!(f.has(flag::LIQUID));
    }
    assert!(seen_source);
    // water and lava are different fluids
    let lava = facts(default_state("lava"));
    assert_ne!(facts(default_state("water")).fluid(), lava.fluid());
    assert_ne!(lava.fluid(), 0);
}

#[test]
fn slab_occupies_half_the_block() {
    let slab = block_by_name("stone_slab").unwrap();
    let bottom = slab
        .state_id(&{
            let mut v = vec![0u8; slab.properties.len()];
            for (i, p) in slab.properties.iter().enumerate() {
                if p.name == "type" {
                    v[i] = p.values.iter().position(|x| *x == "bottom").unwrap() as u8;
                } else if p.name == "waterlogged" {
                    v[i] = p.values.iter().position(|x| *x == "false").unwrap() as u8;
                }
            }
            v
        })
        .unwrap();
    let f = facts(bottom);
    assert_eq!(f.collision_shape(), &[[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]]);
    assert!(f.is_face_sturdy(DOWN, Support::Full));
    assert!(!f.is_face_sturdy(UP, Support::Full));
    // The top of a bottom slab sits at y = 0.5, so nothing above it is supported by the block.
    assert!(!f.is_face_sturdy(UP, Support::Center));
}

#[test]
fn torch_supports_nothing_but_is_a_light_source() {
    let f = facts(default_state("torch"));
    assert!(f.collision_shape().is_empty());
    assert_eq!(f.light_emission(), 14);
    assert!(!f.is_face_sturdy(UP, Support::Full));
}

#[test]
fn redstone_block_is_a_signal_source_and_lever_is_not_a_conductor() {
    assert!(facts(default_state("redstone_block")).has(flag::SIGNAL_SOURCE));
    // The redstone block powers neighbours but cannot itself be powered through.
    assert!(!facts(default_state("redstone_block")).has(flag::REDSTONE_CONDUCTOR));
    assert!(facts(default_state("lever")).has(flag::SIGNAL_SOURCE));
    assert!(!facts(default_state("lever")).has(flag::REDSTONE_CONDUCTOR));
}

#[test]
fn fence_collision_is_taller_than_a_block() {
    let f = facts(default_state("oak_fence"));
    let top = f.collision_shape().iter().map(|b| b[4]).fold(0.0, f64::max);
    assert_eq!(top, 1.5);
}

#[test]
fn every_state_is_internally_consistent() {
    let mut fluid_states = 0;
    for state in 0..ferrite_data::blocks::STATE_COUNT as u16 {
        let f = facts(state);
        assert!(block_of_state(state).is_some());
        // A fluid-free state has no fluid flags set.
        if f.fluid_amount() == 0 {
            assert!(!f.fluid_is_source() && !f.fluid_is_falling(), "state {state}");
            assert_eq!(f.fluid(), 0);
        } else {
            fluid_states += 1;
        }
        // A face that is Full-sturdy must also be Center- and Rigid-sturdy.
        for face in 0..6 {
            if f.is_face_sturdy(face, Support::Full) {
                assert!(f.is_face_sturdy(face, Support::Center), "state {state} face {face}");
            }
        }
        // Air has no collision.
        if f.has(flag::AIR) {
            assert!(f.collision_shape().is_empty());
        }
        let _ = NORTH;
        let _ = f.occlusion_shape();
        let _ = f.outline_shape();
    }
    assert!(fluid_states > 100, "waterlogged blocks and fluids carry fluid ({fluid_states})");
}

#[test]
fn block_entity_and_unbreakable_flags() {
    assert!(facts(default_state("chest")).has(flag::HAS_BLOCK_ENTITY));
    assert!(facts(default_state("furnace")).has(flag::HAS_BLOCK_ENTITY));
    assert!(!facts(default_state("stone")).has(flag::HAS_BLOCK_ENTITY));
    assert!(facts(default_state("bedrock")).has(flag::UNBREAKABLE));
    assert!(facts(default_state("barrier")).has(flag::UNBREAKABLE));
    assert!(
        !facts(default_state("obsidian")).has(flag::UNBREAKABLE),
        "obsidian is hard, not unbreakable"
    );
    assert!(!facts(default_state("stone")).has(flag::UNBREAKABLE));
}
