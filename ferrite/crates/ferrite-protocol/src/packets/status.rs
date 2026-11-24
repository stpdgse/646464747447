use crate::packet::{packet, packet_enum};
use crate::types::Str;

packet! {
    StatusResponse in Status Clientbound = status::clientbound::STATUS_RESPONSE "status_response" {
        /// The server list JSON.
        json: Str<32767>,
    }
    PongResponse in Status Clientbound = status::clientbound::PONG_RESPONSE "pong_response" {
        timestamp: i64,
    }
    StatusRequest in Status Serverbound = status::serverbound::STATUS_REQUEST "status_request" {}
    PingRequest in Status Serverbound = status::serverbound::PING_REQUEST "ping_request" {
        timestamp: i64,
    }
}

packet_enum! {
    ClientboundStatus in Status Clientbound {
        StatusResponse(StatusResponse),
        PongResponse(PongResponse),
    }
}

packet_enum! {
    ServerboundStatus in Status Serverbound {
        StatusRequest(StatusRequest),
        PingRequest(PingRequest),
    }
}
