//! NBT in commands: the SNBT the official parser reads, read straight from the command text.

use ferrite_nbt::parse::parse_prefix;
use ferrite_nbt::{Compound, Tag};

use crate::reader::{Arg, Message, Reader, Result, SyntaxError};

/// `TagParser.parseAsArgument`: one value (after optional whitespace); the reader is left just
/// after it.
pub fn read_tag(reader: &mut Reader) -> Result<Tag> {
    let rest = reader.rest();
    match parse_prefix(&rest) {
        Ok((tag, used)) => {
            reader.set_cursor(reader.cursor() + used);
            Ok(tag)
        }
        Err(e) => {
            let at = reader.cursor() + e.cursor;
            Err(SyntaxError {
                message: Message::tr_with(e.key, e.args.into_iter().map(Arg::Str).collect()),
                cursor: at as i64,
            })
        }
    }
}

/// `TagParser.parseCompoundAsArgument`.
pub fn read_compound(reader: &mut Reader) -> Result<Compound> {
    match read_tag(reader)? {
        Tag::Compound(c) => Ok(c),
        _ => Err(reader.error(Message::tr("argument.nbt.expected.compound"))),
    }
}
