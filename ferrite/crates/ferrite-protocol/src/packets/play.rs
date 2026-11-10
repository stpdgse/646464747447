//! The play state. Only what the networking layer needs so far; gameplay packets arrive with the
//! subsystems that use them.

use crate::packet::packet;
use crate::types::Text;

packet! {
    /// Kicks the player. Same layout as the configuration-phase packet of the same name.
    Disconnect in Play Clientbound = play::clientbound::DISCONNECT "disconnect" {
        reason: Text,
    }
}
