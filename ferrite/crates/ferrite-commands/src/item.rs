//! Item stacks in commands: `minecraft:stone[max_stack_size=5,!damage]`.
//!
//! The syntax, the component lookup and the error positions are the official parser's. Component
//! values are read as SNBT and then checked against the component's codec, for the components
//! listed in [`crate::components::VALIDATED`]; the others are accepted as read.

use ferrite_nbt::Tag;

use crate::components;
use crate::identifier::{read_identifier, registry_contains};
use crate::reader::{Arg, Message, Reader, Result};

/// A change to a default component.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    Set(Tag),
    Remove,
}

/// An item with the component changes a command gave it.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemSpec {
    /// A tag (`#minecraft:logs`) instead of an item, for predicates.
    pub tag: Option<String>,
    /// The item's registry name (empty for tags and `*`).
    pub item: String,
    pub components: Vec<(String, Edit)>,
}

/// Components that exist in the registry but cannot be written in text (no persistent codec).
const TRANSIENT: &[&str] = &[
    "minecraft:creative_slot_lock",
    "minecraft:additional_trade_cost",
    "minecraft:map_post_processing",
];

fn s(x: &str) -> Arg {
    Arg::Str(x.to_string())
}

/// Parses an item stack (`ItemParser`).
pub fn parse_item_stack(reader: &mut Reader) -> Result<ItemSpec> {
    let start = reader.cursor();
    let id = read_identifier(reader)?;
    if registry_contains("minecraft:item", &id) != Some(true) {
        reader.set_cursor(start);
        return Err(reader.error(Message::tr_with("argument.item.id.invalid", vec![s(&id)])));
    }
    let components = if reader.can_read() && reader.peek() == '[' {
        reader.skip();
        read_components(reader)?
    } else {
        Vec::new()
    };
    Ok(ItemSpec { tag: None, item: id, components })
}

fn read_components(reader: &mut Reader) -> Result<Vec<(String, Edit)>> {
    let mut out: Vec<(String, Edit)> = Vec::new();
    reader.skip_whitespace();
    while reader.can_read() && reader.peek() != ']' {
        reader.skip_whitespace();
        let removal = reader.can_read() && reader.peek() == '!';
        if removal {
            reader.skip();
        }
        let at = reader.cursor();
        let name = read_identifier(reader)?;
        let known = registry_contains("minecraft:data_component_type", &name) == Some(true)
            && !TRANSIENT.contains(&name.as_str());
        if !known {
            reader.set_cursor(at);
            return Err(
                reader.error(Message::tr_with("arguments.item.component.unknown", vec![s(&name)]))
            );
        }
        if out.iter().any(|(n, _)| *n == name) {
            return Err(reader.error_without_position(Message::tr_with(
                "arguments.item.component.repeated",
                vec![s(&name)],
            )));
        }
        if removal {
            out.push((name, Edit::Remove));
        } else {
            reader.skip_whitespace();
            reader.expect('=')?;
            reader.skip_whitespace();
            let value_at = reader.cursor();
            let tag = crate::nbt::read_tag(reader)?;
            if let Err(why) = components::validate(&name, &tag) {
                reader.set_cursor(value_at);
                return Err(reader.error(Message::tr_with(
                    "arguments.item.component.malformed",
                    vec![s(&name), Arg::Str(why)],
                )));
            }
            out.push((name, Edit::Set(tag)));
        }
        reader.skip_whitespace();
        if reader.can_read() && reader.peek() == ',' {
            reader.skip();
            reader.skip_whitespace();
            continue;
        }
        break;
    }
    reader.expect(']')?;
    Ok(out)
}
