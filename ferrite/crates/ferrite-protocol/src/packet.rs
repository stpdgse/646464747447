//! Packet declaration. Every packet is bound to its generated id at compile time, so a typo in a
//! name is a compile error and a protocol bump that moves ids needs no source change.

use bytes::BufMut;
use ferrite_data::{Direction, State};

use crate::codec::Encode;
use crate::error::Result;

/// A packet with a fixed place in the protocol.
pub trait Packet: Encode {
    const STATE: State;
    const DIRECTION: Direction;
    /// Numeric id, taken from the generated tables.
    const ID: i32;
    /// Official registry name, e.g. `login_finished`.
    const NAME: &'static str;
}

/// Writes the packet *body* (everything after the id) into `buf`.
pub fn encode_body<P: Packet, B: BufMut>(packet: &P, buf: &mut B) -> Result<()> {
    packet.encode(buf)
}

/// Declares packet structs.
///
/// ```text
/// packet! {
///     /// Docs.
///     Intention in Handshake Serverbound = handshake::serverbound::INTENTION "intention" {
///         protocol_version: VarInt,
///     }
/// }
/// ```
macro_rules! packet {
    ($(
        $(#[$meta:meta])*
        $name:ident in $state:ident $dir:ident = $module:ident :: $dirmod:ident :: $id:ident $official:literal {
            $($(#[$fmeta:meta])* $field:ident : $ty:ty),* $(,)?
        }
    )*) => {$(
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name {
            $($(#[$fmeta])* pub $field: $ty,)*
        }
        impl $crate::packet::Packet for $name {
            const STATE: ::ferrite_data::State = ::ferrite_data::State::$state;
            const DIRECTION: ::ferrite_data::Direction = ::ferrite_data::Direction::$dir;
            const ID: i32 = ::ferrite_data::generated::packets::$module::$dirmod::$id;
            const NAME: &'static str = $official;
        }
        impl $crate::codec::Encode for $name {
            #[allow(unused_variables)]
            fn encode<B: ::bytes::BufMut>(&self, buf: &mut B) -> $crate::error::Result<()> {
                $($crate::codec::Encode::encode(&self.$field, buf)?;)*
                Ok(())
            }
        }
        impl<'a> $crate::codec::Decode<'a> for $name {
            #[allow(unused_variables)]
            fn decode(buf: &mut &'a [u8]) -> $crate::error::Result<Self> {
                Ok(Self { $($field: $crate::codec::Decode::decode(buf)?,)* })
            }
        }
    )*};
}

/// Declares the enum of every packet in one state and direction, with decode-by-id.
macro_rules! packet_enum {
    ($(#[$meta:meta])* $enum:ident in $state:ident $dir:ident { $($variant:ident($ty:ty)),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq)]
        pub enum $enum { $($variant($ty)),* }

        impl $enum {
            /// Every packet name this enum covers, for completeness tests.
            pub const NAMES: &'static [&'static str] = &[$(<$ty as $crate::packet::Packet>::NAME),*];

            /// Decodes the body of the packet with the given id. The body must be fully consumed.
            pub fn decode(id: i32, body: &[u8]) -> $crate::error::Result<Self> {
                match id {
                    $(<$ty as $crate::packet::Packet>::ID => Ok(Self::$variant($crate::codec::decode_exact(body)?)),)*
                    _ => Err($crate::error::ProtocolError::UnknownPacket {
                        state: ::ferrite_data::State::$state,
                        direction: ::ferrite_data::Direction::$dir,
                        id,
                    }),
                }
            }

            pub fn id(&self) -> i32 {
                match self { $(Self::$variant(_) => <$ty as $crate::packet::Packet>::ID),* }
            }

            pub fn name(&self) -> &'static str {
                match self { $(Self::$variant(_) => <$ty as $crate::packet::Packet>::NAME),* }
            }

            /// Writes the body (without the id).
            pub fn encode_body<B: ::bytes::BufMut>(&self, buf: &mut B) -> $crate::error::Result<()> {
                match self { $(Self::$variant(p) => $crate::codec::Encode::encode(p, buf)),* }
            }
        }

        $(impl From<$ty> for $enum {
            fn from(p: $ty) -> Self { Self::$variant(p) }
        })*
    };
}

pub(crate) use {packet, packet_enum};
