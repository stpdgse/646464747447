use crate::codec::VarInt;
use crate::packet::{packet, packet_enum};
use crate::types::{ByteVec, GameProfile, Identifier, JsonText, Remaining, Str};
use uuid::Uuid;

/// The vanilla client refuses login-plugin payloads above this size.
pub const MAX_QUERY_PAYLOAD: usize = 1_048_576;

packet! {
    LoginDisconnect in Login Clientbound = login::clientbound::LOGIN_DISCONNECT "login_disconnect" {
        reason: JsonText,
    }
    /// "Encryption Request": the server's public key and a verify token.
    Hello in Login Clientbound = login::clientbound::HELLO "hello" {
        /// Always empty from the vanilla server.
        server_id: Str<20>,
        public_key: ByteVec,
        verify_token: ByteVec,
        should_authenticate: bool,
    }
    LoginFinished in Login Clientbound = login::clientbound::LOGIN_FINISHED "login_finished" {
        profile: GameProfile,
        /// A per-login session identifier (confirmed present against the vanilla server).
        session_id: Uuid,
    }
    LoginCompression in Login Clientbound = login::clientbound::LOGIN_COMPRESSION "login_compression" {
        threshold: VarInt,
    }
    CustomQuery in Login Clientbound = login::clientbound::CUSTOM_QUERY "custom_query" {
        message_id: VarInt,
        channel: Identifier,
        data: Remaining<MAX_QUERY_PAYLOAD>,
    }
    CookieRequest in Login Clientbound = login::clientbound::COOKIE_REQUEST "cookie_request" {
        key: Identifier,
    }

    /// "Login Start".
    ServerboundHello in Login Serverbound = login::serverbound::HELLO "hello" {
        name: Str<16>,
        uuid: Uuid,
    }
    /// "Encryption Response".
    Key in Login Serverbound = login::serverbound::KEY "key" {
        shared_secret: ByteVec,
        verify_token: ByteVec,
    }
    CustomQueryAnswer in Login Serverbound = login::serverbound::CUSTOM_QUERY_ANSWER "custom_query_answer" {
        message_id: VarInt,
        data: Option<Remaining<MAX_QUERY_PAYLOAD>>,
    }
    LoginAcknowledged in Login Serverbound = login::serverbound::LOGIN_ACKNOWLEDGED "login_acknowledged" {}
    CookieResponse in Login Serverbound = login::serverbound::COOKIE_RESPONSE "cookie_response" {
        key: Identifier,
        payload: Option<ByteVec<5120>>,
    }
}

packet_enum! {
    ClientboundLogin in Login Clientbound {
        LoginDisconnect(LoginDisconnect),
        Hello(Hello),
        LoginFinished(LoginFinished),
        LoginCompression(LoginCompression),
        CustomQuery(CustomQuery),
        CookieRequest(CookieRequest),
    }
}

packet_enum! {
    ServerboundLogin in Login Serverbound {
        Hello(ServerboundHello),
        Key(Key),
        CustomQueryAnswer(CustomQueryAnswer),
        LoginAcknowledged(LoginAcknowledged),
        CookieResponse(CookieResponse),
    }
}
