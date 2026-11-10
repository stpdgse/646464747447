use crate::error::{NbtError, Result};
use crate::tag::{Compound, List, Tag};
use crate::{id, mutf8};

/// Appends a network-format root tag (no name).
pub fn write_network(tag: &Tag, out: &mut Vec<u8>) -> Result<()> {
    out.push(tag.type_id());
    payload(tag, out)
}

/// Appends a named root tag (the on-disk format).
pub fn write_named(name: &str, tag: &Tag, out: &mut Vec<u8>) -> Result<()> {
    out.push(tag.type_id());
    string(name, out)?;
    payload(tag, out)
}

fn string(s: &str, out: &mut Vec<u8>) -> Result<()> {
    let len = mutf8::encoded_len(s);
    let len16 = u16::try_from(len).map_err(|_| NbtError::StringTooLong(len))?;
    out.extend_from_slice(&len16.to_be_bytes());
    mutf8::encode(s, out);
    Ok(())
}

fn length(len: usize, out: &mut Vec<u8>) -> Result<()> {
    let len = i32::try_from(len).map_err(|_| NbtError::LengthOverflow(len))?;
    out.extend_from_slice(&len.to_be_bytes());
    Ok(())
}

fn payload(tag: &Tag, out: &mut Vec<u8>) -> Result<()> {
    match tag {
        Tag::Byte(v) => out.push(*v as u8),
        Tag::Short(v) => out.extend_from_slice(&v.to_be_bytes()),
        Tag::Int(v) => out.extend_from_slice(&v.to_be_bytes()),
        Tag::Long(v) => out.extend_from_slice(&v.to_be_bytes()),
        Tag::Float(v) => out.extend_from_slice(&v.to_be_bytes()),
        Tag::Double(v) => out.extend_from_slice(&v.to_be_bytes()),
        Tag::ByteArray(v) => byte_array(v, out)?,
        Tag::String(s) => string(s, out)?,
        Tag::List(l) => list(l, out)?,
        Tag::Compound(c) => compound(c, out)?,
        Tag::IntArray(v) => int_array(v, out)?,
        Tag::LongArray(v) => long_array(v, out)?,
    }
    Ok(())
}

fn byte_array(v: &[i8], out: &mut Vec<u8>) -> Result<()> {
    length(v.len(), out)?;
    out.extend(v.iter().map(|b| *b as u8));
    Ok(())
}

fn int_array(v: &[i32], out: &mut Vec<u8>) -> Result<()> {
    length(v.len(), out)?;
    v.iter().for_each(|x| out.extend_from_slice(&x.to_be_bytes()));
    Ok(())
}

fn long_array(v: &[i64], out: &mut Vec<u8>) -> Result<()> {
    length(v.len(), out)?;
    v.iter().for_each(|x| out.extend_from_slice(&x.to_be_bytes()));
    Ok(())
}

fn compound(c: &Compound, out: &mut Vec<u8>) -> Result<()> {
    for (key, value) in &c.0 {
        out.push(value.type_id());
        string(key, out)?;
        payload(value, out)?;
    }
    out.push(id::END);
    Ok(())
}

fn list(l: &List, out: &mut Vec<u8>) -> Result<()> {
    out.push(l.element_id());
    length(l.len(), out)?;
    match l {
        List::Empty => {}
        List::Byte(v) => out.extend(v.iter().map(|b| *b as u8)),
        List::Short(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_be_bytes())),
        List::Int(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_be_bytes())),
        List::Long(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_be_bytes())),
        List::Float(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_be_bytes())),
        List::Double(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_be_bytes())),
        List::ByteArray(v) => v.iter().try_for_each(|x| byte_array(x, out))?,
        List::String(v) => v.iter().try_for_each(|s| string(s, out))?,
        List::List(v) => v.iter().try_for_each(|x| list(x, out))?,
        List::Compound(v) => v.iter().try_for_each(|x| compound(x, out))?,
        List::IntArray(v) => v.iter().try_for_each(|x| int_array(x, out))?,
        List::LongArray(v) => v.iter().try_for_each(|x| long_array(x, out))?,
    }
    Ok(())
}
