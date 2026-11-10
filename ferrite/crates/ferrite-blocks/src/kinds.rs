//! Per-block classification the behaviour code dispatches on, resolved once from the data tables.

use std::sync::OnceLock;

use ferrite_data::blocks::{BLOCKS, STATE_TO_BLOCK, block_by_name};
use ferrite_data::configuration::TAG_REGISTRIES;

/// What a block does when the world around it changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Behaviour {
    /// Nothing (yet).
    Inert,
    /// A water or lava block.
    Liquid,
    /// A block with a `waterlogged` property.
    Waterloggable,
}

/// What a block needs around it to stay in place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Support {
    /// The centre of the top face of the block below (standing torches).
    CenterBelow,
    /// The full face of the block it is attached to, opposite its `facing` (wall torches, ladders).
    FullBehind,
    /// The centre of the block below, or of the block above when `hanging` (lanterns).
    Lantern,
    /// Anything but air below (carpets).
    NotAirBelow,
    /// A full top face below, with a few named exceptions either way (snow layers).
    SnowLayer,
    /// A rigid or centre-supporting top face below (pressure plates).
    RigidOrCenterBelow,
    /// Redstone wire: a full top face (or a hopper) below.
    WireBase,
    /// A rigid top face below (repeaters, comparators).
    RigidBelow,
    /// Face-attached (levers, buttons): the full face of the block it is stuck to.
    Attached,
    /// A two-block door: each half needs the other, the lower one a full face below.
    Door,
}

/// Redstone-relevant blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Redstone {
    /// A block that always emits 15.
    PowerBlock,
    Lever,
    Button,
    Lamp,
    /// A redstone torch standing on a block.
    Torch,
    /// A redstone torch on a wall.
    WallTorch,
    Repeater,
    Wire,
}

/// How a block's side arms follow its horizontal neighbours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connects {
    /// Glass panes and bars: to sturdy faces, other panes and walls.
    Pane,
    /// Fences: to sturdy faces, fences of the same material class and aligned gates.
    Fence { wooden: bool },
}

#[derive(Clone, Copy, Debug)]
pub struct BlockKind {
    pub redstone: Option<Redstone>,
    pub connects: Option<Connects>,
    /// Never connected to, even with a sturdy face (leaves, pumpkins, shulker boxes, ...).
    pub connection_exception: bool,
    pub is_wall: bool,
    pub is_fence_gate: bool,
    /// Support the block needs, checked when the relevant neighbour changes.
    pub support: Option<Support>,
    pub behaviour: Behaviour,
    /// The block can contain a fluid of its own accord (waterloggable blocks, kelp, seagrass);
    /// flowing fluid never enters such a block.
    pub liquid_container: bool,
    /// Non-solid blocks that fluids nevertheless do not wash away or enter.
    pub refuses_fluid: bool,
    /// Counts as not blocking motion even though it is solid.
    pub never_blocks_motion: bool,
    /// Snow layers never rest on this block / always may, whatever its shape.
    pub is_trapdoor: bool,
    pub is_hopper: bool,
    pub snow_never: bool,
    pub snow_always: bool,
    /// Value index of `waterlogged=true`, and the property's stride, if the block has one.
    pub waterlogged: Option<(u16, u16)>,
}

#[derive(Debug)]
pub struct Kinds {
    pub by_block: Vec<BlockKind>,
    pub air: u16,
    pub water: u16,
    pub lava: u16,
    pub stone: u16,
    pub cobblestone: u16,
    pub obsidian: u16,
    pub basalt: u16,
    pub soul_soil: u16,
    pub blue_ice: u16,
    wire_default: u16,
    water_block: u16,
    lava_block: u16,
}

fn default_state(name: &str) -> u16 {
    block_by_name(name)
        .unwrap_or_else(|| panic!("block {name} is missing from the data"))
        .default_state
}

fn block_tag(name: &str) -> &'static [u16] {
    TAG_REGISTRIES
        .iter()
        .find(|r| r.registry == "minecraft:block")
        .and_then(|r| r.tags.iter().find(|t| t.name == name))
        .unwrap_or_else(|| panic!("block tag {name} is missing from the data"))
        .entries
}

impl Kinds {
    fn build() -> Kinds {
        let mut by_block: Vec<BlockKind> = BLOCKS
            .iter()
            .map(|b| {
                let mut stride = 1u16;
                let mut waterlogged = None;
                for p in b.properties.iter().rev() {
                    if p.name == "waterlogged" {
                        let idx =
                            p.values.iter().position(|v| *v == "true").expect("boolean") as u16;
                        waterlogged = Some((idx, stride));
                    }
                    stride *= p.values.len() as u16;
                }
                BlockKind {
                    redstone: None,
                    connects: None,
                    connection_exception: false,
                    is_wall: false,
                    is_fence_gate: false,
                    support: None,
                    behaviour: if waterlogged.is_some() {
                        Behaviour::Waterloggable
                    } else {
                        Behaviour::Inert
                    },
                    liquid_container: waterlogged.is_some(),
                    refuses_fluid: false,
                    never_blocks_motion: false,
                    is_trapdoor: false,
                    is_hopper: false,
                    snow_never: false,
                    snow_always: false,
                    waterlogged,
                }
            })
            .collect();
        let id =
            |name: &str| usize::from(block_by_name(name).unwrap_or_else(|| panic!("{name}")).id);
        for name in ["water", "lava"] {
            by_block[id(name)].behaviour = Behaviour::Liquid;
        }
        for name in ["kelp", "kelp_plant", "seagrass", "tall_seagrass"] {
            by_block[id(name)].liquid_container = true;
        }
        for name in [
            "ladder",
            "sugar_cane",
            "bubble_column",
            "nether_portal",
            "end_portal",
            "end_gateway",
            "structure_void",
        ] {
            by_block[id(name)].refuses_fluid = true;
        }
        for tag in ["minecraft:doors", "minecraft:signs"] {
            for &b in block_tag(tag) {
                by_block[usize::from(b)].refuses_fluid = true;
            }
        }
        for b in BLOCKS.iter() {
            let name = b.name.trim_start_matches("minecraft:");
            let has = |p: &str| b.properties.iter().any(|x| x.name == p);
            let support = if name.ends_with("wall_torch") || name == "ladder" {
                Some(Support::FullBehind)
            } else if name.ends_with("torch") {
                Some(Support::CenterBelow)
            } else if name == "snow" {
                Some(Support::SnowLayer)
            } else if name.ends_with("pressure_plate") {
                Some(Support::RigidOrCenterBelow)
            } else if name.ends_with("lantern") && has("hanging") {
                Some(Support::Lantern)
            } else {
                None
            };
            if support.is_some() {
                by_block[usize::from(b.id)].support = support;
            }
        }
        let wooden: Vec<u16> = block_tag("minecraft:wooden_fences").to_vec();
        for &b in block_tag("minecraft:fences") {
            by_block[usize::from(b)].connects =
                Some(Connects::Fence { wooden: wooden.contains(&b) });
        }
        for &b in block_tag("minecraft:walls") {
            by_block[usize::from(b)].is_wall = true;
        }
        for &b in block_tag("minecraft:fence_gates") {
            by_block[usize::from(b)].is_fence_gate = true;
        }
        for b in BLOCKS.iter() {
            let k = &mut by_block[usize::from(b.id)];
            let has = |p: &str| b.properties.iter().any(|x| x.name == p);
            // Panes and bars: four boolean arms, no centre post, and not a fence.
            if k.connects.is_none()
                && !k.is_wall
                && has("north")
                && has("east")
                && has("south")
                && has("west")
                && has("waterlogged")
                && !has("up")
                && b.properties.len() == 5
            {
                k.connects = Some(Connects::Pane);
            }
            let name = b.name.trim_start_matches("minecraft:");
            if name.ends_with("leaves")
                || matches!(
                    name,
                    "barrier" | "carved_pumpkin" | "jack_o_lantern" | "melon" | "pumpkin"
                )
            {
                k.connection_exception = true;
            }
        }
        for &b in block_tag("minecraft:shulker_boxes") {
            by_block[usize::from(b)].connection_exception = true;
        }
        for &b in block_tag("minecraft:cannot_support_snow_layer") {
            by_block[usize::from(b)].snow_never = true;
        }
        for &b in block_tag("minecraft:support_override_snow_layer") {
            by_block[usize::from(b)].snow_always = true;
        }
        for &b in block_tag("minecraft:doors") {
            by_block[usize::from(b)].support = Some(Support::Door);
        }
        for (name, kind) in [
            ("redstone_block", Redstone::PowerBlock),
            ("lever", Redstone::Lever),
            ("redstone_lamp", Redstone::Lamp),
            ("redstone_torch", Redstone::Torch),
            ("redstone_wall_torch", Redstone::WallTorch),
            ("repeater", Redstone::Repeater),
            ("redstone_wire", Redstone::Wire),
        ] {
            by_block[id(name)].redstone = Some(kind);
        }
        for &b in block_tag("minecraft:buttons") {
            by_block[usize::from(b)].redstone = Some(Redstone::Button);
        }
        by_block[id("repeater")].support = Some(Support::RigidBelow);
        by_block[id("redstone_wire")].support = Some(Support::WireBase);
        by_block[id("hopper")].is_hopper = true;
        for &b in block_tag("minecraft:trapdoors") {
            by_block[usize::from(b)].is_trapdoor = true;
        }
        by_block[id("lever")].support = Some(Support::Attached);
        for &b in block_tag("minecraft:buttons") {
            by_block[usize::from(b)].support = Some(Support::Attached);
        }
        for &b in block_tag("minecraft:wool_carpets") {
            by_block[usize::from(b)].support = Some(Support::NotAirBelow);
        }
        for name in ["cobweb", "bamboo_sapling"] {
            by_block[id(name)].never_blocks_motion = true;
        }
        Kinds {
            by_block,
            air: default_state("air"),
            water: default_state("water"),
            lava: default_state("lava"),
            stone: default_state("stone"),
            cobblestone: default_state("cobblestone"),
            obsidian: default_state("obsidian"),
            basalt: default_state("basalt"),
            soul_soil: default_state("soul_soil"),
            blue_ice: default_state("blue_ice"),
            wire_default: default_state("redstone_wire"),
            water_block: block_by_name("water").expect("water").first_state,
            lava_block: block_by_name("lava").expect("lava").first_state,
        }
    }

    pub fn get() -> &'static Kinds {
        static KINDS: OnceLock<Kinds> = OnceLock::new();
        KINDS.get_or_init(Kinds::build)
    }

    #[inline]
    pub fn of_state(&self, state: u16) -> &BlockKind {
        &self.by_block[usize::from(STATE_TO_BLOCK[usize::from(state)])]
    }

    /// The default state of redstone wire (power 0, no arms).
    pub fn wire_default(&self) -> u16 {
        self.wire_default
    }

    /// The water block state with the given `level` property (0..16).
    #[inline]
    pub fn water_level(&self, level: u8) -> u16 {
        self.water_block + u16::from(level)
    }

    #[inline]
    pub fn lava_level(&self, level: u8) -> u16 {
        self.lava_block + u16::from(level)
    }

    /// The value of property `name` in `state`, if the block has it.
    pub fn property(&self, state: u16, name: &str) -> Option<&'static str> {
        let block = &BLOCKS[usize::from(STATE_TO_BLOCK[usize::from(state)])];
        let (index, prop) = block.property(name)?;
        let mut values = [0u8; 16];
        block.value_indices(state, &mut values)?;
        Some(prop.values[usize::from(values[index])])
    }

    /// `state` with property `name` set to `value`; `None` if the block lacks either.
    pub fn with_property(&self, state: u16, name: &str, value: &str) -> Option<u16> {
        let block = &BLOCKS[usize::from(STATE_TO_BLOCK[usize::from(state)])];
        let (index, prop) = block.property(name)?;
        let mut values = [0u8; 16];
        let n = block.value_indices(state, &mut values)?;
        values[index] = prop.values.iter().position(|v| *v == value)? as u8;
        block.state_id(&values[..n])
    }

    /// Whether `state` has `waterlogged=true`.
    #[inline]
    pub fn is_waterlogged(&self, state: u16) -> bool {
        let block = &BLOCKS[usize::from(STATE_TO_BLOCK[usize::from(state)])];
        match self.by_block[usize::from(block.id)].waterlogged {
            Some((true_index, stride)) => {
                // boolean property: values are [true, false]
                ((state - block.first_state) / stride) % 2 == true_index
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrite_data::shapes::facts;

    #[test]
    fn waterlogged_detection_agrees_with_the_fluid_table_for_every_state() {
        let k = Kinds::get();
        for state in 0..ferrite_data::blocks::STATE_COUNT as u16 {
            let kind = k.of_state(state);
            if kind.waterlogged.is_some() {
                assert_eq!(
                    k.is_waterlogged(state),
                    facts(state).fluid_amount() > 0,
                    "state {state} ({})",
                    ferrite_data::blocks::block_of_state(state).unwrap().name
                );
            }
        }
    }

    #[test]
    fn liquid_levels_map_to_states() {
        let k = Kinds::get();
        assert_eq!(k.water_level(0), k.water);
        assert_eq!(k.lava_level(0), k.lava);
        assert_eq!(facts(k.water_level(3)).fluid_amount(), 5);
        assert!(facts(k.lava_level(8)).fluid_is_falling());
    }
}
