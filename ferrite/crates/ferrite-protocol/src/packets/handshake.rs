use crate::codec::VarInt;
use crate::error::{ProtocolError, Result};
use crate::packet::{packet, packet_enum};
use crate::types::Str;

packet! {
    /// First packet of every connection; selects the next state.
    Intention in Handshake Serverbound = handshake::serverbound::INTENTION "intention" {
        protocol_version: VarInt,
        /// The name the client connected with (not used by vanilla).
        server_address: Str<255>,
        server_port: u16,
        /// 1 = status, 2 = login, 3 = transfer. See [`Intention::intent`].
        intent: VarInt,
    }
}

/// What the client wants to do after the handshake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Status,
    Login,
    /// Login after being sent here by a `transfer` packet.
    Transfer,
}

impl Intention {
    pub fn intent(&self) -> Result<Intent> {
        match self.intent.0 {
            1 => Ok(Intent::Status),
            2 => Ok(Intent::Login),
            3 => Ok(Intent::Transfer),
            _ => Err(ProtocolError::Invalid("unknown handshake intent")),
        }
    }
}

packet_enum! {
    /// Every packet the client may send in the handshake state.
    ServerboundHandshake in Handshake Serverbound {
        Intention(Intention),
    }
}
