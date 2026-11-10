//! Per-block-state geometry and property facts.
//!
//! Extracted from the official server's own registries by `ferrite-datagen` (`java/ShapeDump.java`).
//! Fluids, support rules, pistons and redstone all need to know the shape of a block, and the
//! vanilla data reports do not include that.
//!
//! Face order everywhere is `down, up, north, south, west, east`.

pub use crate::generated::shapes::{SHAPES, STATE_FACTS};

/// An axis-aligned box in block-local coordinates (collision boxes may extend past 1.0, e.g. fences): `[min_x, min_y, min_z, max_x, max_y, max_z]`.
pub type Box6 = [f64; 6];

/// Bits in [`StateFacts::flags`].
pub mod flag {
    pub const AIR: u16 = 1;
    pub const SOLID: u16 = 1 << 1;
    pub const CAN_OCCLUDE: u16 = 1 << 2;
    pub const REDSTONE_CONDUCTOR: u16 = 1 << 3;
    pub const SOLID_RENDER: u16 = 1 << 4;
    pub const REPLACEABLE: u16 = 1 << 5;
    pub const PROPAGATES_SKYLIGHT: u16 = 1 << 6;
    pub const IGNITED_BY_LAVA: u16 = 1 << 7;
    pub const LIQUID: u16 = 1 << 8;
    pub const SIGNAL_SOURCE: u16 = 1 << 9;
    pub const ANALOG_OUTPUT: u16 = 1 << 10;
    pub const FULL_COLLISION_BLOCK: u16 = 1 << 11;
    pub const SHAPE_FOR_LIGHT_OCCLUSION: u16 = 1 << 12;
    pub const RANDOMLY_TICKING: u16 = 1 << 13;
    pub const HAS_BLOCK_ENTITY: u16 = 1 << 14;
    /// Destroy speed -1 (bedrock, barriers, portals, ...): immovable and unbreakable.
    pub const UNBREAKABLE: u16 = 1 << 15;
}

/// How a face can support another block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Support {
    /// The whole face is covered.
    Full = 0,
    /// The centre 2x2-pixel square is covered (torches, bells).
    Center = 1,
    /// A rigid, full-width ring is covered (fence-like support).
    Rigid = 2,
}

/// Everything the simulation needs to know about one block state beyond its identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateFacts {
    flags: u16,
    light_emission: u8,
    light_dampening: u8,
    piston: u8,
    fluid: u8,
    fluid_amount: u8,
    fluid_flags: u8,
    collision: u16,
    shape: u16,
    occlusion: u16,
    face_occlusion: [u16; 6],
    sturdy: u32,
}

impl StateFacts {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        flags: u16,
        light_emission: u8,
        light_dampening: u8,
        piston: u8,
        fluid: u8,
        fluid_amount: u8,
        fluid_flags: u8,
        collision: u16,
        shape: u16,
        occlusion: u16,
        face_occlusion: [u16; 6],
        sturdy: u32,
    ) -> Self {
        Self {
            flags,
            light_emission,
            light_dampening,
            piston,
            fluid,
            fluid_amount,
            fluid_flags,
            collision,
            shape,
            occlusion,
            face_occlusion,
            sturdy,
        }
    }

    #[inline]
    pub const fn has(&self, flag: u16) -> bool {
        self.flags & flag != 0
    }
    #[inline]
    pub const fn flags(&self) -> u16 {
        self.flags
    }
    pub const fn light_emission(&self) -> u8 {
        self.light_emission
    }
    pub const fn light_dampening(&self) -> u8 {
        self.light_dampening
    }
    /// Index into vanilla's push-reaction enum (`normal, destroy, block, ignore, push_only`).
    pub const fn piston_reaction(&self) -> u8 {
        self.piston
    }
    /// Id in the `minecraft:fluid` registry of the fluid this state contains (0 = empty).
    pub const fn fluid(&self) -> u8 {
        self.fluid
    }
    /// 0 for no fluid, 1..=8 otherwise (8 = a full source block).
    pub const fn fluid_amount(&self) -> u8 {
        self.fluid_amount
    }
    pub const fn fluid_is_source(&self) -> bool {
        self.fluid_flags & 1 != 0
    }
    pub const fn fluid_is_falling(&self) -> bool {
        self.fluid_flags & 2 != 0
    }
    pub fn collision_shape(&self) -> &'static [Box6] {
        SHAPES[usize::from(self.collision)]
    }
    /// Id of the collision shape; equal ids mean equal shapes, so this is a cheap cache key.
    pub const fn collision_id(&self) -> u16 {
        self.collision
    }
    pub fn outline_shape(&self) -> &'static [Box6] {
        SHAPES[usize::from(self.shape)]
    }
    pub fn occlusion_shape(&self) -> &'static [Box6] {
        SHAPES[usize::from(self.occlusion)]
    }
    /// The shape this state presents on face `face` for occlusion tests.
    pub fn face_occlusion_shape(&self, face: usize) -> &'static [Box6] {
        SHAPES[usize::from(self.face_occlusion[face])]
    }
    /// Id of the face-occlusion shape; equal ids mean equal shapes, so this is a cheap key.
    pub const fn face_occlusion_id(&self, face: usize) -> u16 {
        self.face_occlusion[face]
    }
    /// Whether `face` can support something with the given support type.
    #[inline]
    pub const fn is_face_sturdy(&self, face: usize, support: Support) -> bool {
        self.sturdy & (1 << (support as u32 * 6 + face as u32)) != 0
    }
}

/// Facts for a block-state id.
#[inline]
pub fn facts(state: u16) -> &'static StateFacts {
    &STATE_FACTS[usize::from(state)]
}

const _: () = assert!(STATE_FACTS.len() == crate::blocks::STATE_COUNT);
