//! Entity types: hitboxes, categories and default attributes, generated from the game's own
//! registries.

pub use crate::generated::entities::ENTITIES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobCategory {
    Monster,
    Creature,
    Ambient,
    Axolotls,
    UndergroundWaterCreature,
    WaterCreature,
    WaterAmbient,
    Misc,
}

/// The default attribute values of a living entity type (`None` where it has no such attribute).
#[derive(Clone, Copy, Debug)]
pub struct Attributes {
    pub max_health: Option<f64>,
    pub movement_speed: Option<f64>,
    pub attack_damage: Option<f64>,
    pub follow_range: Option<f64>,
    pub knockback_resistance: Option<f64>,
    pub armor: Option<f64>,
}

#[derive(Clone, Copy, Debug)]
pub struct EntityInfo {
    pub name: &'static str,
    pub width: f32,
    pub height: f32,
    pub eye_height: f32,
    pub category: MobCategory,
    /// How far (in chunks) clients see it.
    pub tracking_range: i32,
    /// Ticks between position updates.
    pub update_interval: i32,
    pub fire_immune: bool,
    pub summonable: bool,
    /// Present for living entities.
    pub attributes: Option<Attributes>,
}

/// The facts about the entity type with this protocol id.
pub fn entity_info(id: u32) -> Option<&'static EntityInfo> {
    ENTITIES.get(id as usize)
}

/// By name, e.g. `minecraft:cow`.
pub fn entity_by_name(name: &str) -> Option<(u32, &'static EntityInfo)> {
    let id = crate::registries::entity_type::by_name.get(name).copied()?;
    entity_info(id).map(|e| (id, e))
}
