//! Protocol-correct bot clients and a load generator.
//!
//! The bots use the same codec the server uses, and they are themselves checked against Mojang's
//! vanilla server (`tests/vanilla.rs`), so a swarm result means "real clients would have worked".

mod client;
mod report;
mod runner;

pub use client::{Conn, JoinOutcome, SwarmError, join_once, ping_session, status_once};
pub use report::Report;
pub use runner::{Mode, RunConfig, run};
