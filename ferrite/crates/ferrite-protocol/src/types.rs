//! Owned wire types used by packet definitions. Each carries its protocol limit in its type, so a
//! packet definition reads like the specification and cannot forget a bound.

use std::ops::Deref;

use bytes::BufMut;
use ferrite_nbt::{Limits, Tag};

use crate::codec::{self, Decode, Encode};
use crate::error::{ProtocolError, Result};

/// A UTF-8 string of at most `MAX` UTF-16 code units.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Str<const MAX: usize = 32767>(pub String);

impl<const MAX: usize> Str<MAX> {
    pub fn new(s: impl Into<String>) -> Self {
        Str(s.into())
    }
}
impl<const MAX: usize> Deref for Str<MAX> {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}
impl<const MAX: usize> From<&str> for Str<MAX> {
    fn from(s: &str) -> Self {
        Str(s.to_owned())
    }
}
impl<const MAX: usize> Encode for Str<MAX> {
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        codec::write_str(buf, &self.0, MAX)
    }
}
impl<'a, const MAX: usize> Decode<'a> for Str<MAX> {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        codec::read_str(buf, MAX).map(|s| Str(s.to_owned()))
    }
}

/// A namespaced identifier such as `minecraft:stone`. A missing or empty namespace means
/// `minecraft`. Only the character set is validated, exactly as vanilla does.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Identifier(String);

impl Identifier {
    /// Validates and wraps `s`.
    pub fn new(s: impl Into<String>) -> Result<Self> {
        let s = s.into();
        let (ns, path) = s.split_once(':').unwrap_or(("minecraft", &s));
        let ns_ok = ns.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_'));
        let path_ok = path
            .bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_' | b'/'));
        // Like vanilla, an empty namespace or path is accepted (`:x` is `minecraft:x`).
        if !ns_ok || !path_ok {
            return Err(ProtocolError::InvalidIdentifier(s));
        }
        Ok(Identifier(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl Deref for Identifier {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}
impl Encode for Identifier {
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        codec::write_str(buf, &self.0, codec::DEFAULT_MAX_STRING)
    }
}
impl<'a> Decode<'a> for Identifier {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        Identifier::new(codec::read_str(buf, codec::DEFAULT_MAX_STRING)?)
    }
}

/// A VarInt-prefixed byte array of at most `MAX` bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ByteVec<const MAX: usize = { i32::MAX as usize }>(pub Vec<u8>);

impl<const MAX: usize> Encode for ByteVec<MAX> {
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        codec::write_byte_array(buf, &self.0, MAX)
    }
}
impl<'a, const MAX: usize> Decode<'a> for ByteVec<MAX> {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        codec::read_byte_array(buf, MAX).map(|b| ByteVec(b.to_vec()))
    }
}

/// A VarInt-prefixed list of at most `MAX` items.
#[derive(Clone, Debug, PartialEq)]
pub struct LenVec<T, const MAX: usize = 65536>(pub Vec<T>);

impl<T, const MAX: usize> Default for LenVec<T, MAX> {
    fn default() -> Self {
        LenVec(Vec::new())
    }
}
impl<T: Encode, const MAX: usize> Encode for LenVec<T, MAX> {
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        codec::write_list(buf, &self.0, MAX)
    }
}
impl<'a, T: Decode<'a>, const MAX: usize> Decode<'a> for LenVec<T, MAX> {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        codec::read_list(buf, MAX).map(LenVec)
    }
}

/// Everything up to the end of the packet, at most `MAX` bytes (plugin message payloads).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Remaining<const MAX: usize>(pub Vec<u8>);

impl<const MAX: usize> Encode for Remaining<MAX> {
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        if self.0.len() > MAX {
            return Err(ProtocolError::LengthLimit { len: self.0.len(), max: MAX });
        }
        buf.put_slice(&self.0);
        Ok(())
    }
}
impl<'a, const MAX: usize> Decode<'a> for Remaining<MAX> {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        if buf.len() > MAX {
            return Err(ProtocolError::LengthLimit { len: buf.len(), max: MAX });
        }
        let data = buf.to_vec();
        *buf = &[];
        Ok(Remaining(data))
    }
}

/// A network NBT value that must be present (a `TAG_End` root is rejected).
#[derive(Clone, Debug, PartialEq)]
pub struct Nbt(pub Tag);

impl Encode for Nbt {
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        let mut out = Vec::new();
        ferrite_nbt::write_network(&self.0, &mut out)?;
        buf.put_slice(&out);
        Ok(())
    }
}
impl<'a> Decode<'a> for Nbt {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        match ferrite_nbt::read_network(buf, Limits::default())? {
            Some(tag) => Ok(Nbt(tag)),
            None => Err(ProtocolError::Invalid("expected an NBT tag but found TAG_End")),
        }
    }
}

/// A text component in the binary protocol: a string tag for plain text, a compound otherwise.
#[derive(Clone, Debug, PartialEq)]
pub struct Text(pub Tag);

impl Text {
    /// Unstyled text.
    pub fn plain(s: impl Into<String>) -> Self {
        Text(Tag::String(s.into()))
    }
}
impl Encode for Text {
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        match self.0 {
            Tag::String(_) | Tag::Compound(_) => {}
            _ => {
                return Err(ProtocolError::Invalid(
                    "a text component must be a string or compound",
                ));
            }
        }
        Nbt(self.0.clone()).encode(buf)
    }
}
impl<'a> Decode<'a> for Text {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        let Nbt(tag) = Nbt::decode(buf)?;
        match tag {
            Tag::String(_) | Tag::Compound(_) => Ok(Text(tag)),
            _ => Err(ProtocolError::Invalid("a text component must be a string or compound")),
        }
    }
}

/// A text component serialised as a JSON string (used by the login-phase disconnect).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JsonText(pub String);

impl JsonText {
    /// `{"text":"..."}` with correct JSON escaping.
    pub fn plain(s: &str) -> Self {
        let mut out = String::with_capacity(s.len() + 12);
        out.push_str("{\"text\":\"");
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push_str("\"}");
        JsonText(out)
    }
}
impl Encode for JsonText {
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        codec::write_str(buf, &self.0, 262144)
    }
}
impl<'a> Decode<'a> for JsonText {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        codec::read_str(buf, 262144).map(|s| JsonText(s.to_owned()))
    }
}

/// A boolean followed by `A` when true and `B` when false.
#[derive(Clone, Debug, PartialEq)]
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

impl<A: Encode, B: Encode> Encode for Either<A, B> {
    fn encode<W: BufMut>(&self, buf: &mut W) -> Result<()> {
        match self {
            Either::Left(a) => {
                true.encode(buf)?;
                a.encode(buf)
            }
            Either::Right(b) => {
                false.encode(buf)?;
                b.encode(buf)
            }
        }
    }
}
impl<'a, A: Decode<'a>, B: Decode<'a>> Decode<'a> for Either<A, B> {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        if bool::decode(buf)? {
            A::decode(buf).map(Either::Left)
        } else {
            B::decode(buf).map(Either::Right)
        }
    }
}

/// A player profile: identity plus signed properties such as skin textures.
#[derive(Clone, Debug, PartialEq)]
pub struct GameProfile {
    pub uuid: uuid::Uuid,
    pub name: Str<16>,
    pub properties: LenVec<ProfileProperty, 16>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileProperty {
    pub name: Str<64>,
    pub value: Str<32767>,
    pub signature: Option<Str<1024>>,
}

impl Encode for ProfileProperty {
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        self.name.encode(buf)?;
        self.value.encode(buf)?;
        self.signature.encode(buf)
    }
}
impl<'a> Decode<'a> for ProfileProperty {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        Ok(Self {
            name: Decode::decode(buf)?,
            value: Decode::decode(buf)?,
            signature: Decode::decode(buf)?,
        })
    }
}
impl Encode for GameProfile {
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        self.uuid.encode(buf)?;
        self.name.encode(buf)?;
        self.properties.encode(buf)
    }
}
impl<'a> Decode<'a> for GameProfile {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        Ok(Self {
            uuid: Decode::decode(buf)?,
            name: Decode::decode(buf)?,
            properties: Decode::decode(buf)?,
        })
    }
}
