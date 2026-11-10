//! The play state.
//!
//! Layouts are pinned to what the official server really sends: `tests/play_captures.rs` decodes
//! packets captured from vanilla (see `ferrite-capture`) with these types, requires every byte to
//! be consumed, and re-encodes them byte-for-byte.

use uuid::Uuid;

use crate::codec::{Decode, Encode, VarInt, VarLong};
use crate::error::Result;
use crate::packet::packet;
use crate::types::{ByteVec, Identifier, LenVec, Nbt, Text};

/// A block position packed into 64 bits: 26 bits of x, 26 of z, 12 of y.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PackedPos(pub i64);

impl PackedPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        PackedPos(
            (((x as i64) & 0x3FF_FFFF) << 38)
                | (((z as i64) & 0x3FF_FFFF) << 12)
                | ((y as i64) & 0xFFF),
        )
    }
    pub const fn x(self) -> i32 {
        (self.0 >> 38) as i32
    }
    pub const fn y(self) -> i32 {
        ((self.0 << 52) >> 52) as i32
    }
    pub const fn z(self) -> i32 {
        ((self.0 << 26) >> 38) as i32
    }
}
impl Encode for PackedPos {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<()> {
        self.0.encode(buf)
    }
}
impl<'a> Decode<'a> for PackedPos {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        Ok(PackedPos(i64::decode(buf)?))
    }
}

/// An angle in 1/256 of a turn.
pub type Angle = u8;

/// The "properties" VarInt that leads relative entity movement: bit 0 is on-ground, the rest is a
/// step count for multi-tick paths. Only single-step movement (count 0) is supported.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OnGround(pub bool);

impl Encode for OnGround {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<()> {
        VarInt(i32::from(self.0)).encode(buf)
    }
}
impl<'a> Decode<'a> for OnGround {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        let v = VarInt::decode(buf)?.0;
        if v >> 1 != 0 {
            return Err(crate::error::ProtocolError::Invalid(
                "stepped entity movement is not supported",
            ));
        }
        Ok(OnGround(v & 1 != 0))
    }
}

/// A velocity in the compact "low precision" format: one zero byte, or 6 bytes (plus a VarInt for
/// large magnitudes) holding a scale and three 15-bit components.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LpVec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl LpVec3 {
    pub const ZERO: LpVec3 = LpVec3 { x: 0.0, y: 0.0, z: 0.0 };
}

fn lp_sanitize(v: f64) -> f64 {
    if v.is_nan() { 0.0 } else { v.clamp(-1.717_986_918_3e10, 1.717_986_918_3e10) }
}
fn lp_pack(v: f64) -> i64 {
    ((v * 0.5 + 0.5) * 32766.0).round() as i64
}
fn lp_unpack(v: u64) -> f64 {
    (v & 32767).min(32766) as f64 * 2.0 / 32766.0 - 1.0
}

impl Encode for LpVec3 {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<()> {
        let (x, y, z) = (lp_sanitize(self.x), lp_sanitize(self.y), lp_sanitize(self.z));
        let max = x.abs().max(y.abs()).max(z.abs());
        if max < 3.051_944_088_384_301e-5 {
            return 0u8.encode(buf);
        }
        let scale = max.ceil() as i64;
        let big = (scale & 3) != scale;
        let low = if big { (scale & 3) | 4 } else { scale };
        let packed = low
            | (lp_pack(x / scale as f64) << 3)
            | (lp_pack(y / scale as f64) << 18)
            | (lp_pack(z / scale as f64) << 33);
        (packed as u8).encode(buf)?;
        ((packed >> 8) as u8).encode(buf)?;
        ((packed >> 16) as u32).encode(buf)?;
        if big {
            VarInt((scale >> 2) as i32).encode(buf)?;
        }
        Ok(())
    }
}
impl<'a> Decode<'a> for LpVec3 {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        let b0 = u8::decode(buf)?;
        if b0 == 0 {
            return Ok(LpVec3::ZERO);
        }
        let b1 = u8::decode(buf)?;
        let rest = u32::decode(buf)?;
        let packed = (u64::from(rest) << 16) | (u64::from(b1) << 8) | u64::from(b0);
        let mut scale = u64::from(b0 & 3);
        if b0 & 4 == 4 {
            scale |= (VarInt::decode(buf)?.0 as u32 as u64) << 2;
        }
        let s = scale as f64;
        Ok(LpVec3 {
            x: lp_unpack(packed >> 3) * s,
            y: lp_unpack(packed >> 18) * s,
            z: lp_unpack(packed >> 33) * s,
        })
    }
}

/// An item stack with no component changes (the only kind Ferrite handles so far). Stacks that
/// carry component patches are rejected when decoding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemStack {
    /// 0 means the slot is empty.
    pub count: i32,
    /// Id in the `item` registry; meaningless when `count` is 0.
    pub item: i32,
}
impl Encode for ItemStack {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<()> {
        if self.count <= 0 {
            return VarInt(0).encode(buf);
        }
        VarInt(self.count).encode(buf)?;
        VarInt(self.item).encode(buf)?;
        VarInt(0).encode(buf)?; // components added
        VarInt(0).encode(buf) // components removed
    }
}
impl<'a> Decode<'a> for ItemStack {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        let count = VarInt::decode(buf)?.0;
        if count <= 0 {
            return Ok(ItemStack::default());
        }
        let item = VarInt::decode(buf)?.0;
        let added = VarInt::decode(buf)?.0;
        let removed = VarInt::decode(buf)?.0;
        if added != 0 || removed != 0 {
            return Err(crate::error::ProtocolError::Invalid(
                "item components are not supported yet",
            ));
        }
        Ok(ItemStack { count, item })
    }
}

/// One entry of a [`PlayerInfoUpdate`]. Which fields are present depends on the update's action
/// bits; absent ones are `None`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerInfoEntry {
    pub uuid: Uuid,
    /// `ADD_PLAYER`: name and profile properties.
    pub profile: Option<(crate::types::Str<16>, LenVec<crate::types::ProfileProperty, 16>)>,
    /// `UPDATE_GAME_MODE`.
    pub game_mode: Option<VarInt>,
    /// `UPDATE_LISTED`.
    pub listed: Option<bool>,
    /// `UPDATE_LATENCY` in milliseconds.
    pub latency: Option<VarInt>,
    /// `UPDATE_DISPLAY_NAME`: outer `Some` = the field is present, inner = the name (or none).
    pub display_name: Option<Option<Text>>,
    /// `UPDATE_LIST_ORDER`.
    pub list_order: Option<VarInt>,
    /// `UPDATE_HAT`.
    pub show_hat: Option<bool>,
}

/// Bits of [`PlayerInfoUpdate::actions`].
pub mod player_action {
    pub const ADD_PLAYER: u8 = 1;
    pub const INITIALIZE_CHAT: u8 = 2;
    pub const UPDATE_GAME_MODE: u8 = 4;
    pub const UPDATE_LISTED: u8 = 8;
    pub const UPDATE_LATENCY: u8 = 16;
    pub const UPDATE_DISPLAY_NAME: u8 = 32;
    pub const UPDATE_LIST_ORDER: u8 = 64;
    pub const UPDATE_HAT: u8 = 128;
}

/// Adds players to the tab list or updates them.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerInfoUpdate {
    /// Which fields each entry carries (see [`player_action`]). Chat sessions
    /// (`INITIALIZE_CHAT`) are not supported: entries are always sent without one.
    pub actions: u8,
    pub entries: Vec<PlayerInfoEntry>,
}

impl crate::packet::Packet for PlayerInfoUpdate {
    const STATE: ferrite_data::State = ferrite_data::State::Play;
    const DIRECTION: ferrite_data::Direction = ferrite_data::Direction::Clientbound;
    const ID: i32 = ferrite_data::generated::packets::play::clientbound::PLAYER_INFO_UPDATE;
    const NAME: &'static str = "player_info_update";
}
impl Encode for PlayerInfoUpdate {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<()> {
        use player_action::*;
        self.actions.encode(buf)?;
        VarInt(self.entries.len() as i32).encode(buf)?;
        let missing = || {
            crate::error::ProtocolError::Invalid("player info entry lacks a field its actions need")
        };
        for e in &self.entries {
            e.uuid.encode(buf)?;
            if self.actions & ADD_PLAYER != 0 {
                let (name, props) = e.profile.as_ref().ok_or_else(missing)?;
                name.encode(buf)?;
                props.encode(buf)?;
            }
            if self.actions & INITIALIZE_CHAT != 0 {
                false.encode(buf)?;
            }
            if self.actions & UPDATE_GAME_MODE != 0 {
                e.game_mode.ok_or_else(missing)?.encode(buf)?;
            }
            if self.actions & UPDATE_LISTED != 0 {
                e.listed.ok_or_else(missing)?.encode(buf)?;
            }
            if self.actions & UPDATE_LATENCY != 0 {
                e.latency.ok_or_else(missing)?.encode(buf)?;
            }
            if self.actions & UPDATE_DISPLAY_NAME != 0 {
                e.display_name.clone().ok_or_else(missing)?.encode(buf)?;
            }
            if self.actions & UPDATE_LIST_ORDER != 0 {
                e.list_order.ok_or_else(missing)?.encode(buf)?;
            }
            if self.actions & UPDATE_HAT != 0 {
                e.show_hat.ok_or_else(missing)?.encode(buf)?;
            }
        }
        Ok(())
    }
}
impl<'a> Decode<'a> for PlayerInfoUpdate {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        use player_action::*;
        let actions = u8::decode(buf)?;
        let n = VarInt::decode(buf)?.0;
        if !(0..=4096).contains(&n) {
            return Err(crate::error::ProtocolError::Invalid("too many player info entries"));
        }
        let mut entries = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let mut e = PlayerInfoEntry { uuid: Uuid::decode(buf)?, ..Default::default() };
            if actions & ADD_PLAYER != 0 {
                e.profile = Some((Decode::decode(buf)?, Decode::decode(buf)?));
            }
            if actions & INITIALIZE_CHAT != 0 && bool::decode(buf)? {
                return Err(crate::error::ProtocolError::Invalid(
                    "chat sessions are not supported yet",
                ));
            }
            if actions & UPDATE_GAME_MODE != 0 {
                e.game_mode = Some(Decode::decode(buf)?);
            }
            if actions & UPDATE_LISTED != 0 {
                e.listed = Some(Decode::decode(buf)?);
            }
            if actions & UPDATE_LATENCY != 0 {
                e.latency = Some(Decode::decode(buf)?);
            }
            if actions & UPDATE_DISPLAY_NAME != 0 {
                e.display_name = Some(Decode::decode(buf)?);
            }
            if actions & UPDATE_LIST_ORDER != 0 {
                e.list_order = Some(Decode::decode(buf)?);
            }
            if actions & UPDATE_HAT != 0 {
                e.show_hat = Some(Decode::decode(buf)?);
            }
            entries.push(e);
        }
        Ok(PlayerInfoUpdate { actions, entries })
    }
}

/// One entry of [`SetTime::clock_updates`].
#[derive(Clone, Debug, PartialEq)]
pub struct ClockUpdate {
    /// Id of the clock in the `world_clock` registry.
    pub clock: VarInt,
    pub total_ticks: VarLong,
    pub partial_tick: f32,
    pub rate: f32,
}
impl Encode for ClockUpdate {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<()> {
        self.clock.encode(buf)?;
        self.total_ticks.encode(buf)?;
        self.partial_tick.encode(buf)?;
        self.rate.encode(buf)
    }
}
impl<'a> Decode<'a> for ClockUpdate {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        Ok(ClockUpdate {
            clock: Decode::decode(buf)?,
            total_ticks: Decode::decode(buf)?,
            partial_tick: Decode::decode(buf)?,
            rate: Decode::decode(buf)?,
        })
    }
}

/// The player-independent part of the join information.
#[derive(Clone, Debug, PartialEq)]
pub struct SpawnInfo {
    /// Id in the `dimension_type` registry.
    pub dimension_type: VarInt,
    pub dimension: Identifier,
    pub seed: i64,
    pub game_mode: VarInt,
    pub previous_game_mode: Option<VarInt>,
    pub is_debug: bool,
    pub is_flat: bool,
    /// `(dimension, position)` of the last death.
    pub last_death: Option<(Identifier, PackedPos)>,
    pub portal_cooldown: VarInt,
    pub sea_level: VarInt,
}
impl Encode for SpawnInfo {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<()> {
        self.dimension_type.encode(buf)?;
        self.dimension.encode(buf)?;
        self.seed.encode(buf)?;
        self.game_mode.encode(buf)?;
        self.previous_game_mode.encode(buf)?;
        self.is_debug.encode(buf)?;
        self.is_flat.encode(buf)?;
        match &self.last_death {
            None => false.encode(buf)?,
            Some((dim, pos)) => {
                true.encode(buf)?;
                dim.encode(buf)?;
                pos.encode(buf)?;
            }
        }
        self.portal_cooldown.encode(buf)?;
        self.sea_level.encode(buf)
    }
}
impl<'a> Decode<'a> for SpawnInfo {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        Ok(SpawnInfo {
            dimension_type: Decode::decode(buf)?,
            dimension: Decode::decode(buf)?,
            seed: Decode::decode(buf)?,
            game_mode: Decode::decode(buf)?,
            previous_game_mode: Decode::decode(buf)?,
            is_debug: Decode::decode(buf)?,
            is_flat: Decode::decode(buf)?,
            last_death: if bool::decode(buf)? {
                Some((Identifier::decode(buf)?, PackedPos::decode(buf)?))
            } else {
                None
            },
            portal_cooldown: Decode::decode(buf)?,
            sea_level: Decode::decode(buf)?,
        })
    }
}

/// A bit set sent as little-endian bytes (bit `i` is bit `i % 8` of byte `i / 8`), the way chunk
/// light masks travel.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BitSetBytes(pub Vec<u8>);

impl BitSetBytes {
    /// A set with exactly the given bits, without trailing zero bytes.
    pub fn from_bits(bits: impl IntoIterator<Item = usize>) -> Self {
        let mut bytes: Vec<u8> = Vec::new();
        for b in bits {
            if bytes.len() <= b / 8 {
                bytes.resize(b / 8 + 1, 0);
            }
            bytes[b / 8] |= 1 << (b % 8);
        }
        BitSetBytes(bytes)
    }

    pub fn contains(&self, bit: usize) -> bool {
        self.0.get(bit / 8).is_some_and(|b| b & (1 << (bit % 8)) != 0)
    }
}
impl Encode for BitSetBytes {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<()> {
        ByteVec::<64>(self.0.clone()).encode(buf)
    }
}
impl<'a> Decode<'a> for BitSetBytes {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        Ok(BitSetBytes(ByteVec::<64>::decode(buf)?.0))
    }
}

/// A network NBT value that may be absent (`TAG_End` on the wire).
#[derive(Clone, Debug, PartialEq)]
pub struct OptNbt(pub Option<Nbt>);

impl Encode for OptNbt {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<()> {
        match &self.0 {
            None => 0u8.encode(buf),
            Some(n) => n.encode(buf),
        }
    }
}
impl<'a> Decode<'a> for OptNbt {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        if buf.first() == Some(&0) {
            *buf = &buf[1..];
            Ok(OptNbt(None))
        } else {
            Ok(OptNbt(Some(Nbt::decode(buf)?)))
        }
    }
}

/// A node of the command tree. Only unrestricted literal and root nodes are modelled.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandNode {
    /// Bits 0-1: type (0 root, 1 literal, 2 argument); 4: executable; 8: has redirect.
    pub flags: u8,
    pub children: LenVec<VarInt, 4096>,
    /// The literal's name, for literal nodes.
    pub name: Option<crate::types::Str<64>>,
}
impl Encode for CommandNode {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<()> {
        self.flags.encode(buf)?;
        self.children.encode(buf)?;
        if self.flags & 3 == 1 {
            self.name
                .as_ref()
                .ok_or(crate::error::ProtocolError::Invalid("a literal command node needs a name"))?
                .encode(buf)?;
        }
        Ok(())
    }
}
impl<'a> Decode<'a> for CommandNode {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        let flags = u8::decode(buf)?;
        if flags & 3 == 2 || flags & 8 != 0 {
            return Err(crate::error::ProtocolError::Invalid(
                "argument and redirect command nodes are not supported yet",
            ));
        }
        let children = LenVec::decode(buf)?;
        let name = if flags & 3 == 1 { Some(Decode::decode(buf)?) } else { None };
        Ok(CommandNode { flags, children, name })
    }
}

/// One heightmap of a chunk: a type id and 256 packed heights.
#[derive(Clone, Debug, PartialEq)]
pub struct Heightmap {
    /// 1 = world surface, 4 = motion blocking, 5 = motion blocking no leaves.
    pub kind: VarInt,
    pub data: LenVec<i64, 64>,
}
impl Encode for Heightmap {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<()> {
        self.kind.encode(buf)?;
        self.data.encode(buf)
    }
}
impl<'a> Decode<'a> for Heightmap {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        Ok(Heightmap { kind: Decode::decode(buf)?, data: Decode::decode(buf)? })
    }
}

/// A block entity attached to a chunk packet.
#[derive(Clone, Debug, PartialEq)]
pub struct ChunkBlockEntity {
    /// `x << 4 | z`, both within the chunk.
    pub packed_xz: u8,
    pub y: i16,
    /// Id in the `block_entity_type` registry.
    pub kind: VarInt,
    pub data: OptNbt,
}
impl Encode for ChunkBlockEntity {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<()> {
        self.packed_xz.encode(buf)?;
        self.y.encode(buf)?;
        self.kind.encode(buf)?;
        self.data.encode(buf)
    }
}
impl<'a> Decode<'a> for ChunkBlockEntity {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        Ok(ChunkBlockEntity {
            packed_xz: Decode::decode(buf)?,
            y: Decode::decode(buf)?,
            kind: Decode::decode(buf)?,
            data: Decode::decode(buf)?,
        })
    }
}

/// Light for the sections of a chunk. Bit `i` of a mask is the section `i - 1` counted from the
/// bottom of the world: there is one extra section below and one above.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LightData {
    pub sky_mask: BitSetBytes,
    pub block_mask: BitSetBytes,
    pub empty_sky_mask: BitSetBytes,
    pub empty_block_mask: BitSetBytes,
    /// 2048 bytes (4 bits per block) for each bit set in `sky_mask`, in order.
    pub sky: LenVec<ByteVec<2048>, 64>,
    pub block: LenVec<ByteVec<2048>, 64>,
}
impl Encode for LightData {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<()> {
        self.sky_mask.encode(buf)?;
        self.block_mask.encode(buf)?;
        self.empty_sky_mask.encode(buf)?;
        self.empty_block_mask.encode(buf)?;
        self.sky.encode(buf)?;
        self.block.encode(buf)
    }
}
impl<'a> Decode<'a> for LightData {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        Ok(LightData {
            sky_mask: Decode::decode(buf)?,
            block_mask: Decode::decode(buf)?,
            empty_sky_mask: Decode::decode(buf)?,
            empty_block_mask: Decode::decode(buf)?,
            sky: Decode::decode(buf)?,
            block: Decode::decode(buf)?,
        })
    }
}

packet! {
    /// A whole chunk column with its light.
    LevelChunkWithLight in Play Clientbound = play::clientbound::LEVEL_CHUNK_WITH_LIGHT "level_chunk_with_light" {
        x: i32,
        z: i32,
        heightmaps: LenVec<Heightmap, 8>,
        /// The sections, concatenated (see `ferrite-game`).
        buffer: ByteVec<2097152>,
        block_entities: LenVec<ChunkBlockEntity, 65536>,
        light: LightData,
    }

    /// Light only, for an already sent chunk.
    LightUpdate in Play Clientbound = play::clientbound::LIGHT_UPDATE "light_update" {
        x: VarInt,
        z: VarInt,
        light: LightData,
    }

    ForgetLevelChunk in Play Clientbound = play::clientbound::FORGET_LEVEL_CHUNK "forget_level_chunk" {
        /// The chunk position as one long: `z << 32 | x`.
        pos: i64,
    }

    BlockUpdate in Play Clientbound = play::clientbound::BLOCK_UPDATE "block_update" {
        pos: PackedPos,
        state: VarInt,
    }

    BlockChangedAck in Play Clientbound = play::clientbound::BLOCK_CHANGED_ACK "block_changed_ack" {
        sequence: VarInt,
    }
}

/// A fixed-size byte array (no length prefix).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fixed<const N: usize>(pub [u8; N]);

impl<const N: usize> Encode for Fixed<N> {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<()> {
        buf.put_slice(&self.0);
        Ok(())
    }
}
impl<'a, const N: usize> Decode<'a> for Fixed<N> {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        if buf.len() < N {
            return Err(crate::error::ProtocolError::Eof);
        }
        let (a, rest) = buf.split_at(N);
        *buf = rest;
        Ok(Fixed(a.try_into().expect("length checked")))
    }
}

packet! {
    PlayerInfoRemove in Play Clientbound = play::clientbound::PLAYER_INFO_REMOVE "player_info_remove" {
        uuids: LenVec<Uuid, 4096>,
    }

    /// Spawns an entity. For players `kind` is the `player` entity type and `data` is 0.
    AddEntity in Play Clientbound = play::clientbound::ADD_ENTITY "add_entity" {
        id: VarInt,
        uuid: Uuid,
        /// Id in the `entity_type` registry.
        kind: VarInt,
        x: f64,
        y: f64,
        z: f64,
        velocity: LpVec3,
        x_rot: Angle,
        y_rot: Angle,
        y_head_rot: Angle,
        data: VarInt,
    }

    RemoveEntities in Play Clientbound = play::clientbound::REMOVE_ENTITIES "remove_entities" {
        ids: LenVec<VarInt, 4096>,
    }

    /// Relative movement in 1/4096 of a block (at most 8 blocks per packet).
    MoveEntityPos in Play Clientbound = play::clientbound::MOVE_ENTITY_POS "move_entity_pos" {
        id: VarInt,
        on_ground: OnGround,
        dx: i16,
        dy: i16,
        dz: i16,
    }

    MoveEntityPosRot in Play Clientbound = play::clientbound::MOVE_ENTITY_POS_ROT "move_entity_pos_rot" {
        id: VarInt,
        on_ground: OnGround,
        dx: i16,
        dy: i16,
        dz: i16,
        y_rot: Angle,
        x_rot: Angle,
    }

    MoveEntityRot in Play Clientbound = play::clientbound::MOVE_ENTITY_ROT "move_entity_rot" {
        id: VarInt,
        y_rot: Angle,
        x_rot: Angle,
        on_ground: bool,
    }

    RotateHead in Play Clientbound = play::clientbound::ROTATE_HEAD "rotate_head" {
        id: VarInt,
        y_head_rot: Angle,
    }

    /// Absolute position, for moves too large for the relative packets.
    TeleportEntity in Play Clientbound = play::clientbound::TELEPORT_ENTITY "teleport_entity" {
        id: VarInt,
        x: f64,
        y: f64,
        z: f64,
        dx: f64,
        dy: f64,
        dz: f64,
        y_rot: f32,
        x_rot: f32,
        relative: u32,
        on_ground: bool,
    }

    SetEntityData in Play Clientbound = play::clientbound::SET_ENTITY_DATA "set_entity_data" {
        id: VarInt,
        /// Entries (index, serializer id, value), ended by an index of 255.
        data: crate::types::Remaining<65536>,
    }

    ContainerSetContent in Play Clientbound = play::clientbound::CONTAINER_SET_CONTENT "container_set_content" {
        window_id: VarInt,
        state_id: VarInt,
        items: LenVec<ItemStack, 128>,
        carried: ItemStack,
    }

    /// The command tree; Ferrite sends only the empty root for now.
    Commands in Play Clientbound = play::clientbound::COMMANDS "commands" {
        nodes: LenVec<CommandNode, 4096>,
        root: VarInt,
    }

    SystemChat in Play Clientbound = play::clientbound::SYSTEM_CHAT "system_chat" {
        content: Text,
        /// Show above the hotbar instead of in the chat.
        overlay: bool,
    }

    /// Kicks the player. Same layout as the configuration-phase packet of the same name.
    Disconnect in Play Clientbound = play::clientbound::DISCONNECT "disconnect" {
        reason: Text,
    }

    /// First play packet: tells the client about the world it is entering.
    Login in Play Clientbound = play::clientbound::LOGIN "login" {
        entity_id: i32,
        hardcore: bool,
        levels: LenVec<Identifier>,
        max_players: VarInt,
        view_distance: VarInt,
        simulation_distance: VarInt,
        reduced_debug_info: bool,
        show_death_screen: bool,
        limited_crafting: bool,
        spawn: SpawnInfo,
        online_mode: bool,
        enforces_secure_chat: bool,
    }

    ChangeDifficulty in Play Clientbound = play::clientbound::CHANGE_DIFFICULTY "change_difficulty" {
        difficulty: u8,
        locked: bool,
    }

    /// `flags`: 1 invulnerable, 2 flying, 4 may fly, 8 instant build.
    PlayerAbilities in Play Clientbound = play::clientbound::PLAYER_ABILITIES "player_abilities" {
        flags: u8,
        flying_speed: f32,
        walking_speed: f32,
    }

    SetHeldSlot in Play Clientbound = play::clientbound::SET_HELD_SLOT "set_held_slot" {
        slot: VarInt,
    }

    /// Teleports the player. The client must answer with `accept_teleportation`.
    PlayerPosition in Play Clientbound = play::clientbound::PLAYER_POSITION "player_position" {
        teleport_id: VarInt,
        x: f64,
        y: f64,
        z: f64,
        dx: f64,
        dy: f64,
        dz: f64,
        y_rot: f32,
        x_rot: f32,
        /// Bit set of components that are relative instead of absolute.
        relative: u32,
    }

    /// `event` 13 is "level chunks load start".
    GameEvent in Play Clientbound = play::clientbound::GAME_EVENT "game_event" {
        event: u8,
        param: f32,
    }

    SetChunkCacheCenter in Play Clientbound = play::clientbound::SET_CHUNK_CACHE_CENTER "set_chunk_cache_center" {
        x: VarInt,
        z: VarInt,
    }

    ChunkBatchStart in Play Clientbound = play::clientbound::CHUNK_BATCH_START "chunk_batch_start" {}

    ChunkBatchFinished in Play Clientbound = play::clientbound::CHUNK_BATCH_FINISHED "chunk_batch_finished" {
        batch_size: VarInt,
    }

    BundleDelimiter in Play Clientbound = play::clientbound::BUNDLE_DELIMITER "bundle_delimiter" {}

    KeepAlive in Play Clientbound = play::clientbound::KEEP_ALIVE "keep_alive" {
        id: i64,
    }

    SetDefaultSpawnPosition in Play Clientbound = play::clientbound::SET_DEFAULT_SPAWN_POSITION "set_default_spawn_position" {
        dimension: Identifier,
        pos: PackedPos,
        yaw: f32,
        pitch: f32,
    }

    SetTime in Play Clientbound = play::clientbound::SET_TIME "set_time" {
        game_time: i64,
        clock_updates: LenVec<ClockUpdate>,
    }

    InitializeBorder in Play Clientbound = play::clientbound::INITIALIZE_BORDER "initialize_border" {
        center_x: f64,
        center_z: f64,
        old_size: f64,
        new_size: f64,
        lerp_time: VarLong,
        absolute_max_size: VarInt,
        warning_blocks: VarInt,
        warning_time: VarInt,
    }

    TickingState in Play Clientbound = play::clientbound::TICKING_STATE "ticking_state" {
        tick_rate: f32,
        frozen: bool,
    }

    TickingStep in Play Clientbound = play::clientbound::TICKING_STEP "ticking_step" {
        steps: VarInt,
    }

    ServerData in Play Clientbound = play::clientbound::SERVER_DATA "server_data" {
        motd: Text,
        icon: Option<LenVec<u8, 16384>>,
    }

    SetHealth in Play Clientbound = play::clientbound::SET_HEALTH "set_health" {
        health: f32,
        food: VarInt,
        saturation: f32,
    }

    SetExperience in Play Clientbound = play::clientbound::SET_EXPERIENCE "set_experience" {
        progress: f32,
        total: VarInt,
        level: VarInt,
    }

    PostEffects in Play Clientbound = play::clientbound::POST_EFFECTS "post_effects" {
        effects: LenVec<VarInt>,
    }

    /// Position only. `flags`: 1 on ground, 2 pushing against a wall.
    MovePlayerPos in Play Serverbound = play::serverbound::MOVE_PLAYER_POS "move_player_pos" {
        x: f64,
        y: f64,
        z: f64,
        flags: u8,
    }

    MovePlayerPosRot in Play Serverbound = play::serverbound::MOVE_PLAYER_POS_ROT "move_player_pos_rot" {
        x: f64,
        y: f64,
        z: f64,
        y_rot: f32,
        x_rot: f32,
        flags: u8,
    }

    MovePlayerRot in Play Serverbound = play::serverbound::MOVE_PLAYER_ROT "move_player_rot" {
        y_rot: f32,
        x_rot: f32,
        flags: u8,
    }

    MovePlayerStatusOnly in Play Serverbound = play::serverbound::MOVE_PLAYER_STATUS_ONLY "move_player_status_only" {
        flags: u8,
    }

    /// Digging and similar actions. `action`: 0 start destroy, 1 abort, 2 finish destroy, 3 drop
    /// stack, 4 drop item, 5 release use item, 6 swap hands.
    PlayerAction in Play Serverbound = play::serverbound::PLAYER_ACTION "player_action" {
        action: VarInt,
        pos: PackedPos,
        /// Face of the block: 0 down, 1 up, 2 north, 3 south, 4 west, 5 east.
        face: u8,
        sequence: VarInt,
    }

    UseItemOn in Play Serverbound = play::serverbound::USE_ITEM_ON "use_item_on" {
        hand: VarInt,
        pos: PackedPos,
        face: VarInt,
        cursor_x: f32,
        cursor_y: f32,
        cursor_z: f32,
        inside: bool,
        world_border_hit: bool,
        sequence: VarInt,
    }

    UseItem in Play Serverbound = play::serverbound::USE_ITEM "use_item" {
        hand: VarInt,
        sequence: VarInt,
        y_rot: f32,
        x_rot: f32,
    }

    SetCarriedItem in Play Serverbound = play::serverbound::SET_CARRIED_ITEM "set_carried_item" {
        slot: i16,
    }

    SetCreativeModeSlot in Play Serverbound = play::serverbound::SET_CREATIVE_MODE_SLOT "set_creative_mode_slot" {
        slot: i16,
        item: ItemStack,
    }

    /// The player swung at the air. Carries no data in this version.
    Punch in Play Serverbound = play::serverbound::PUNCH "punch" {}

    /// Sneaking, sprinting and the like.
    PlayerCommand in Play Serverbound = play::serverbound::PLAYER_COMMAND "player_command" {
        entity_id: VarInt,
        action: VarInt,
        data: VarInt,
    }

    PlayerInput in Play Serverbound = play::serverbound::PLAYER_INPUT "player_input" {
        flags: u8,
    }

    /// A chat message. Ferrite does not verify signatures; `signature` is kept only so the
    /// packet round-trips.
    Chat in Play Serverbound = play::serverbound::CHAT "chat" {
        message: crate::types::Str<256>,
        timestamp: i64,
        salt: i64,
        signature: Option<Fixed<256>>,
        /// Last-seen-messages update: how many messages were acknowledged, which of the last 20
        /// (a 20-bit set), and a checksum.
        offset: VarInt,
        acknowledged: Fixed<3>,
        checksum: i8,
    }

    /// Settings the client sends after joining and whenever they change.
    PlayClientInformation in Play Serverbound = play::serverbound::CLIENT_INFORMATION "client_information" {
        locale: crate::types::Str<16>,
        view_distance: i8,
        chat_mode: VarInt,
        chat_colors: bool,
        displayed_skin_parts: u8,
        main_hand: VarInt,
        enable_text_filtering: bool,
        allow_server_listings: bool,
        particle_status: VarInt,
    }

    /// A plugin message (`minecraft:brand` is sent right after joining).
    ServerboundCustomPayload in Play Serverbound = play::serverbound::CUSTOM_PAYLOAD "custom_payload" {
        channel: Identifier,
        data: crate::types::Remaining<32767>,
    }

    ChatCommand in Play Serverbound = play::serverbound::CHAT_COMMAND "chat_command" {
        command: crate::types::Str<32767>,
    }

    /// Echo of a teleport: the id plus the position and rotation the client applied.
    AcceptTeleportation in Play Serverbound = play::serverbound::ACCEPT_TELEPORTATION "accept_teleportation" {
        teleport_id: VarInt,
        x: f64,
        y: f64,
        z: f64,
        y_rot: f32,
        x_rot: f32,
    }

    ServerboundKeepAlive in Play Serverbound = play::serverbound::KEEP_ALIVE "keep_alive" {
        id: i64,
    }

    ChunkBatchReceived in Play Serverbound = play::serverbound::CHUNK_BATCH_RECEIVED "chunk_batch_received" {
        chunks_per_tick: f32,
    }

    PlayerLoaded in Play Serverbound = play::serverbound::PLAYER_LOADED "player_loaded" {}

    ClientTickEnd in Play Serverbound = play::serverbound::CLIENT_TICK_END "client_tick_end" {}
}

/// Uses `Uuid` in later packets; imported here so the module's public surface stays stable.
#[allow(dead_code)]
type _Uuid = Uuid;
