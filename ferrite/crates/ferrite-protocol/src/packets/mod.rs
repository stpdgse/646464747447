//! Packet definitions for the states before play.
//!
//! Field layouts follow the protocol specification for the release in `ferrite_data`, and are
//! checked against a real vanilla server by the oracle tests (`tests/oracle.rs`).

pub mod configuration;
pub mod handshake;
pub mod login;
pub mod play;
pub mod status;
