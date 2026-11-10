//! Connection handling.
//!
//! [`Lobby`] holds every *decision* about a connection (what the status says, who may join, when
//! to kick) and drives a [`ferrite_protocol::session::ServerSession`] without doing any I/O. The
//! backends are small loops that only move bytes:
//!
//! * [`tokio_backend`]: portable, work-stealing multi-thread runtime.
//! * `uring_backend` (Linux, feature `uring`): one `io_uring` runtime per core, each with its own
//!   `SO_REUSEPORT` listener so the kernel spreads connections and no state is shared.
//!
//! Because the policy is shared, a behavioural difference between backends is a bug, and the
//! end-to-end tests run the same scenarios against both.

mod lobby;
pub mod tokio_backend;
#[cfg(all(feature = "uring", target_os = "linux"))]
pub mod uring_backend;

pub use lobby::{ConnState, Flow, Lobby, LobbyConfig, Stats, StatsSnapshot, is_valid_player_name};
