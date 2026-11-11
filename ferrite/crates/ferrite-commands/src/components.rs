//! Checking item component values the way the official codecs do, with their messages.
//!
//! Only the components in [`VALIDATED`] are checked so far; any other component accepts whatever
//! SNBT it is given. Each entry was verified against the official parser (see
//! `tests/data/components.txt`).

use ferrite_nbt::Tag;
use ferrite_nbt::snbt::to_snbt;

/// The components whose codecs are ported.
pub const VALIDATED: &[&str] = &[
    "minecraft:max_stack_size",
    "minecraft:max_damage",
    "minecraft:damage",
    "minecraft:repair_cost",
    "minecraft:unbreakable",
    "minecraft:enchantment_glint_override",
    "minecraft:custom_data",
    "minecraft:rarity",
];

/// A number the way a codec reads it: any numeric tag, converted like Java's `intValue`.
fn int_value(t: &Tag) -> Option<i32> {
    match t {
        Tag::Byte(v) => Some(i32::from(*v)),
        Tag::Short(v) => Some(i32::from(*v)),
        Tag::Int(v) => Some(*v),
        Tag::Long(v) => Some(*v as i32),
        Tag::Float(v) => Some(*v as i32),
        Tag::Double(v) => Some(*v as i32),
        _ => None,
    }
}

fn int_in(t: &Tag, check: impl Fn(i32) -> Option<String>) -> Result<(), String> {
    let Some(v) = int_value(t) else {
        return Err("Not a number".into());
    };
    match check(v) {
        Some(message) => Err(message),
        None => Ok(()),
    }
}

/// Checks `value` as the component `name`; the error is the codec's message.
pub fn validate(name: &str, value: &Tag) -> Result<(), String> {
    match name {
        "minecraft:max_stack_size" => int_in(value, |v| {
            (!(1..=99).contains(&v)).then(|| format!("Value must be within range [1;99]: {v}"))
        }),
        "minecraft:max_damage" => {
            int_in(value, |v| (v < 1).then(|| format!("Value must be positive: {v}")))
        }
        "minecraft:damage" | "minecraft:repair_cost" => {
            int_in(value, |v| (v < 0).then(|| format!("Value must be non-negative: {v}")))
        }
        "minecraft:unbreakable" => match value {
            Tag::Compound(_) => Ok(()),
            other => Err(format!("Not a map: {}", to_snbt(other))),
        },
        "minecraft:enchantment_glint_override" => int_in(value, |_| None),
        "minecraft:custom_data" => match value {
            Tag::Compound(_) => Ok(()),
            Tag::String(text) => match ferrite_nbt::parse::parse(text) {
                Ok(Tag::Compound(_)) => Ok(()),
                _ => Err(format!(
                    "Failed to parse either. First: Not a compound tag: {}; Second: Not a valid SNBT compound",
                    to_snbt(value)
                )),
            },
            other => Err(format!(
                "Failed to parse either. First: Not a compound tag: {}; Second: Not a string",
                to_snbt(other)
            )),
        },
        "minecraft:rarity" => match value {
            Tag::String(v) if matches!(v.as_str(), "common" | "uncommon" | "rare" | "epic") => {
                Ok(())
            }
            Tag::String(v) => Err(format!("Unknown element name:{v}")),
            _ => Err("Not a string".into()),
        },
        _ => Ok(()),
    }
}
