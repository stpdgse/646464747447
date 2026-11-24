use uuid::Uuid;

use crate::codec::VarInt;
use crate::packet::{packet, packet_enum};
use crate::types::{ByteVec, Either, Identifier, LenVec, Nbt, Remaining, Str, Text};

/// Maximum plugin payload: 1 MiB towards the client, 32 KiB towards the server.
pub const MAX_CLIENTBOUND_PAYLOAD: usize = 1_048_576;
pub const MAX_SERVERBOUND_PAYLOAD: usize = 32_767;

/// One synchronized-registry entry. `data` is omitted when both sides know the entry from a
/// shared data pack (see [`KnownPack`]).
#[derive(Clone, Debug, PartialEq)]
pub struct RegistryEntry {
    pub id: Identifier,
    pub data: Option<Nbt>,
}

/// The tags of one registry.
#[derive(Clone, Debug, PartialEq)]
pub struct RegistryTags {
    pub registry: Identifier,
    pub tags: LenVec<TagEntry, 65536>,
}

/// One tag: a name and the numeric ids of its members.
#[derive(Clone, Debug, PartialEq)]
pub struct TagEntry {
    pub name: Identifier,
    /// Numeric ids of the tagged entries.
    pub entries: LenVec<VarInt, 65536>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownPack {
    pub namespace: Str,
    pub id: Str,
    pub version: Str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportDetail {
    pub title: Str<128>,
    pub description: Str<4096>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServerLink {
    /// A built-in label id (see [`BuiltInLink`]) or a custom text component.
    pub label: Either<VarInt, Text>,
    pub url: Str,
}

/// Built-in server link labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum BuiltInLink {
    BugReport = 0,
    CommunityGuidelines,
    Support,
    Status,
    Feedback,
    Community,
    Website,
    Forums,
    News,
    Announcements,
}

macro_rules! impl_codec {
    ($t:ty { $($f:ident),* }) => {
        impl crate::codec::Encode for $t {
            fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> crate::error::Result<()> {
                $(crate::codec::Encode::encode(&self.$f, buf)?;)*
                Ok(())
            }
        }
        impl<'a> crate::codec::Decode<'a> for $t {
            fn decode(buf: &mut &'a [u8]) -> crate::error::Result<Self> {
                Ok(Self { $($f: crate::codec::Decode::decode(buf)?),* })
            }
        }
    };
}
impl_codec!(RegistryEntry { id, data });
impl_codec!(RegistryTags { registry, tags });
impl_codec!(TagEntry { name, entries });
impl_codec!(KnownPack { namespace, id, version });
impl_codec!(ReportDetail { title, description });
impl_codec!(ServerLink { label, url });

packet! {
    CookieRequest in Configuration Clientbound = configuration::clientbound::COOKIE_REQUEST "cookie_request" {
        key: Identifier,
    }
    CustomPayload in Configuration Clientbound = configuration::clientbound::CUSTOM_PAYLOAD "custom_payload" {
        channel: Identifier,
        data: Remaining<MAX_CLIENTBOUND_PAYLOAD>,
    }
    Disconnect in Configuration Clientbound = configuration::clientbound::DISCONNECT "disconnect" {
        reason: Text,
    }
    FinishConfiguration in Configuration Clientbound = configuration::clientbound::FINISH_CONFIGURATION "finish_configuration" {}
    KeepAlive in Configuration Clientbound = configuration::clientbound::KEEP_ALIVE "keep_alive" {
        id: i64,
    }
    Ping in Configuration Clientbound = configuration::clientbound::PING "ping" {
        id: i32,
    }
    ResetChat in Configuration Clientbound = configuration::clientbound::RESET_CHAT "reset_chat" {}
    RegistryData in Configuration Clientbound = configuration::clientbound::REGISTRY_DATA "registry_data" {
        registry: Identifier,
        entries: LenVec<RegistryEntry, 65536>,
    }
    ResourcePackPop in Configuration Clientbound = configuration::clientbound::RESOURCE_PACK_POP "resource_pack_pop" {
        /// `None` removes every pack.
        id: Option<Uuid>,
    }
    ResourcePackPush in Configuration Clientbound = configuration::clientbound::RESOURCE_PACK_PUSH "resource_pack_push" {
        id: Uuid,
        url: Str,
        hash: Str<40>,
        forced: bool,
        prompt: Option<Text>,
    }
    PostEffects in Configuration Clientbound = configuration::clientbound::POST_EFFECTS "post_effects" {
        effects: LenVec<Identifier, 65536>,
    }
    StoreCookie in Configuration Clientbound = configuration::clientbound::STORE_COOKIE "store_cookie" {
        key: Identifier,
        payload: ByteVec<5120>,
    }
    Transfer in Configuration Clientbound = configuration::clientbound::TRANSFER "transfer" {
        host: Str,
        port: VarInt,
    }
    UpdateEnabledFeatures in Configuration Clientbound = configuration::clientbound::UPDATE_ENABLED_FEATURES "update_enabled_features" {
        features: LenVec<Identifier, 65536>,
    }
    UpdateTags in Configuration Clientbound = configuration::clientbound::UPDATE_TAGS "update_tags" {
        registries: LenVec<RegistryTags, 65536>,
    }
    SelectKnownPacks in Configuration Clientbound = configuration::clientbound::SELECT_KNOWN_PACKS "select_known_packs" {
        packs: LenVec<KnownPack, 64>,
    }
    CustomReportDetails in Configuration Clientbound = configuration::clientbound::CUSTOM_REPORT_DETAILS "custom_report_details" {
        details: LenVec<ReportDetail, 32>,
    }
    ServerLinks in Configuration Clientbound = configuration::clientbound::SERVER_LINKS "server_links" {
        links: LenVec<ServerLink, 65536>,
    }
    ClearDialog in Configuration Clientbound = configuration::clientbound::CLEAR_DIALOG "clear_dialog" {}
    ShowDialog in Configuration Clientbound = configuration::clientbound::SHOW_DIALOG "show_dialog" {
        dialog: Nbt,
    }
    CodeOfConduct in Configuration Clientbound = configuration::clientbound::CODE_OF_CONDUCT "code_of_conduct" {
        text: Str,
    }

    ClientInformation in Configuration Serverbound = configuration::serverbound::CLIENT_INFORMATION "client_information" {
        locale: Str<16>,
        view_distance: i8,
        /// 0 = enabled, 1 = commands only, 2 = hidden.
        chat_mode: VarInt,
        chat_colors: bool,
        /// Bit mask: cape, jacket, left sleeve, right sleeve, left pants, right pants, hat.
        displayed_skin_parts: u8,
        /// 0 = left, 1 = right.
        main_hand: VarInt,
        enable_text_filtering: bool,
        allow_server_listings: bool,
        /// 0 = all, 1 = decreased, 2 = minimal.
        particle_status: VarInt,
    }
    ServerboundCookieResponse in Configuration Serverbound = configuration::serverbound::COOKIE_RESPONSE "cookie_response" {
        key: Identifier,
        payload: Option<ByteVec<5120>>,
    }
    ServerboundCustomPayload in Configuration Serverbound = configuration::serverbound::CUSTOM_PAYLOAD "custom_payload" {
        channel: Identifier,
        data: Remaining<MAX_SERVERBOUND_PAYLOAD>,
    }
    AcknowledgeFinishConfiguration in Configuration Serverbound = configuration::serverbound::FINISH_CONFIGURATION "finish_configuration" {}
    ServerboundKeepAlive in Configuration Serverbound = configuration::serverbound::KEEP_ALIVE "keep_alive" {
        id: i64,
    }
    Pong in Configuration Serverbound = configuration::serverbound::PONG "pong" {
        id: i32,
    }
    ResourcePackResponse in Configuration Serverbound = configuration::serverbound::RESOURCE_PACK "resource_pack" {
        id: Uuid,
        /// 0 loaded, 1 declined, 2 download failed, 3 accepted, 4 downloaded, 5 invalid URL,
        /// 6 reload failed, 7 discarded.
        result: VarInt,
    }
    ServerboundSelectKnownPacks in Configuration Serverbound = configuration::serverbound::SELECT_KNOWN_PACKS "select_known_packs" {
        packs: LenVec<KnownPack, 64>,
    }
    CustomClickAction in Configuration Serverbound = configuration::serverbound::CUSTOM_CLICK_ACTION "custom_click_action" {
        id: Identifier,
        /// Length-prefixed network NBT, at most 64 KiB (it may be a lone `TAG_End`).
        payload: ByteVec<65536>,
    }
    AcceptCodeOfConduct in Configuration Serverbound = configuration::serverbound::ACCEPT_CODE_OF_CONDUCT "accept_code_of_conduct" {}
}

packet_enum! {
    ClientboundConfiguration in Configuration Clientbound {
        CookieRequest(CookieRequest),
        CustomPayload(CustomPayload),
        Disconnect(Disconnect),
        FinishConfiguration(FinishConfiguration),
        KeepAlive(KeepAlive),
        Ping(Ping),
        ResetChat(ResetChat),
        RegistryData(RegistryData),
        ResourcePackPop(ResourcePackPop),
        ResourcePackPush(ResourcePackPush),
        PostEffects(PostEffects),
        StoreCookie(StoreCookie),
        Transfer(Transfer),
        UpdateEnabledFeatures(UpdateEnabledFeatures),
        UpdateTags(UpdateTags),
        SelectKnownPacks(SelectKnownPacks),
        CustomReportDetails(CustomReportDetails),
        ServerLinks(ServerLinks),
        ClearDialog(ClearDialog),
        ShowDialog(ShowDialog),
        CodeOfConduct(CodeOfConduct),
    }
}

packet_enum! {
    ServerboundConfiguration in Configuration Serverbound {
        ClientInformation(ClientInformation),
        CookieResponse(ServerboundCookieResponse),
        CustomPayload(ServerboundCustomPayload),
        FinishConfiguration(AcknowledgeFinishConfiguration),
        KeepAlive(ServerboundKeepAlive),
        Pong(Pong),
        ResourcePackResponse(ResourcePackResponse),
        SelectKnownPacks(ServerboundSelectKnownPacks),
        CustomClickAction(CustomClickAction),
        AcceptCodeOfConduct(AcceptCodeOfConduct),
    }
}
