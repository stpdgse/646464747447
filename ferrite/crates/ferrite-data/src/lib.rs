//! Static Minecraft Java Edition data tables.
//!
//! Everything under [`generated`] is produced by `ferrite-datagen` from the data reports of the
//! official server (`java -DbundlerMainClass=net.minecraft.data.Main -jar server.jar --reports`).
//! Never edit those files by hand; regenerate them with `cargo datagen`.

#![forbid(unsafe_code)]

pub mod blocks;
pub mod configuration;
pub mod entities;
#[allow(clippy::all, missing_docs, non_upper_case_globals)]
pub mod generated;
pub mod items;
pub mod lang;
pub mod loot;
pub mod registries;
pub mod shapes;
pub mod timelines;

pub use generated::meta::{
    DATA_PACK_MAJOR, DATA_PACK_MINOR, GAME_VERSION, PROTOCOL_VERSION, RESOURCE_PACK_MAJOR,
    RESOURCE_PACK_MINOR, WORLD_VERSION,
};

/// A connection protocol state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum State {
    Handshake,
    Status,
    Login,
    Configuration,
    Play,
}

/// The direction a packet travels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Direction {
    /// Client to server.
    Serverbound,
    /// Server to client.
    Clientbound,
}

impl State {
    pub const ALL: [State; 5] =
        [State::Handshake, State::Status, State::Login, State::Configuration, State::Play];
}

/// Looks up the registry name of a packet (for logging and tracing).
pub fn packet_name(state: State, direction: Direction, id: i32) -> Option<&'static str> {
    generated::packets::packet_name(state, direction, id)
}

/// Returns the number of packets defined for `state` in `direction`.
pub fn packet_count(state: State, direction: Direction) -> usize {
    generated::packets::packet_count(state, direction)
}
