//! Remote administration protocols.
//!
//! * [`query`]: the GameSpy4-style UDP query used by server list tools.
//! * [`rcon`]: Source RCON over TCP, for running console commands remotely.
//!
//! Both are verified against a real vanilla server (`tests/vanilla.rs`).

pub mod query;
pub mod rcon;
