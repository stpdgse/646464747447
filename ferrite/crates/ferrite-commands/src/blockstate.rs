//! Block states and block predicates in commands: `chest[facing=north]`, `#minecraft:logs`.
//!
//! The syntax and errors of the official `BlockStateParser`. Block entity NBT (`{...}`) needs the
//! SNBT parser and is reported as unsupported.

use crate::identifier::read_identifier;
use crate::reader::{Arg, Message, Reader, Result, SyntaxError};

/// A block with the properties a command gave it.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockSpec {
    /// A tag (`#minecraft:logs`) instead of a block, for predicates.
    pub tag: Option<String>,
    /// The block's registry name (empty for tags).
    pub block: String,
    pub properties: Vec<(String, String)>,
    /// Block entity data (`chest{Lock:"x"}`).
    pub nbt: Option<ferrite_nbt::Compound>,
}

fn err(reader: &Reader, key: &str, args: Vec<Arg>) -> SyntaxError {
    reader.error(Message::tr_with(key, args))
}

fn s(x: &str) -> Arg {
    Arg::Str(x.to_string())
}

/// Parses a block state (`allow_tags` false) or a block predicate (true).
pub fn parse_block(reader: &mut Reader, allow_tags: bool) -> Result<BlockSpec> {
    if reader.can_read() && reader.peek() == '#' {
        if !allow_tags {
            return Err(err(reader, "argument.block.tag.disallowed", vec![]));
        }
        reader.skip();
        let tag = read_identifier(reader)?;
        let mut properties = Vec::new();
        if reader.can_read() && reader.peek() == '[' {
            reader.skip();
            read_loose_properties(reader, &mut properties)?;
        }
        let nbt = read_block_nbt(reader)?;
        return Ok(BlockSpec { tag: Some(tag), block: String::new(), properties, nbt });
    }
    let id_start = reader.cursor();
    let id = read_identifier(reader)?;
    let Some(info) = ferrite_data::blocks::block_by_name(&id) else {
        reader.set_cursor(id_start);
        return Err(err(reader, "argument.block.id.invalid", vec![s(&id)]));
    };
    let mut properties: Vec<(String, String)> = Vec::new();
    if reader.can_read() && reader.peek() == '[' {
        reader.skip();
        reader.skip_whitespace();
        while reader.can_read() && reader.peek() != ']' {
            reader.skip_whitespace();
            let at = reader.cursor();
            let name = reader.read_string()?;
            let Some((_, prop)) = info.property(&name) else {
                reader.set_cursor(at);
                return Err(err(reader, "argument.block.property.unknown", vec![s(&id), s(&name)]));
            };
            if properties.iter().any(|(n, _)| *n == name) {
                reader.set_cursor(at);
                return Err(err(
                    reader,
                    "argument.block.property.duplicate",
                    vec![s(&name), s(&id)],
                ));
            }
            reader.skip_whitespace();
            if !reader.can_read() || reader.peek() != '=' {
                return Err(err(reader, "argument.block.property.novalue", vec![s(&name), s(&id)]));
            }
            reader.skip();
            reader.skip_whitespace();
            let value_at = reader.cursor();
            let value = reader.read_string()?;
            if !prop.values.contains(&value.as_str()) {
                reader.set_cursor(value_at);
                return Err(err(
                    reader,
                    "argument.block.property.invalid",
                    vec![s(&id), s(&value), s(&name)],
                ));
            }
            properties.push((name, value));
            reader.skip_whitespace();
            if reader.can_read() {
                if reader.peek() == ',' {
                    reader.skip();
                    continue;
                }
                if reader.peek() != ']' {
                    return Err(err(reader, "argument.block.property.unclosed", vec![]));
                }
                break;
            }
        }
        if reader.can_read() {
            reader.skip();
        } else {
            return Err(err(reader, "argument.block.property.unclosed", vec![]));
        }
    }
    let nbt = read_block_nbt(reader)?;
    Ok(BlockSpec { tag: None, block: id, properties, nbt })
}

fn read_block_nbt(reader: &mut Reader) -> Result<Option<ferrite_nbt::Compound>> {
    if reader.can_read() && reader.peek() == '{' {
        crate::nbt::read_compound(reader).map(Some)
    } else {
        Ok(None)
    }
}

/// Tag properties are not checked against any block.
fn read_loose_properties(reader: &mut Reader, out: &mut Vec<(String, String)>) -> Result<()> {
    reader.skip_whitespace();
    while reader.can_read() && reader.peek() != ']' {
        reader.skip_whitespace();
        let name = reader.read_string()?;
        reader.skip_whitespace();
        if reader.can_read() && reader.peek() == '=' {
            reader.skip();
            reader.skip_whitespace();
            let value = reader.read_string()?;
            out.push((name, value));
        } else {
            return Err(err(reader, "argument.block.property.unclosed", vec![]));
        }
        reader.skip_whitespace();
        if reader.can_read() {
            if reader.peek() == ',' {
                reader.skip();
                continue;
            }
            if reader.peek() != ']' {
                return Err(err(reader, "argument.block.property.unclosed", vec![]));
            }
            break;
        }
    }
    if reader.can_read() {
        reader.skip();
        Ok(())
    } else {
        Err(err(reader, "argument.block.property.unclosed", vec![]))
    }
}
