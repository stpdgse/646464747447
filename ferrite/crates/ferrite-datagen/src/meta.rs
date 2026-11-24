use std::io::Read;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::GenContext;

#[derive(Debug, Deserialize)]
pub struct VersionJson {
    pub id: String,
    pub world_version: u32,
    pub protocol_version: u32,
    pub pack_version: PackVersion,
}

#[derive(Debug, Deserialize)]
pub struct PackVersion {
    pub resource_major: u32,
    pub resource_minor: u32,
    pub data_major: u32,
    pub data_minor: u32,
}

pub fn read_version_json(jar: &[u8]) -> Result<VersionJson> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(jar)).context("opening server.jar")?;
    let mut file = zip.by_name("version.json").context("server.jar has no version.json")?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    serde_json::from_str(&text).context("parsing version.json")
}

pub fn generate(cx: &GenContext, v: &VersionJson) -> String {
    format!(
        "{header}\n\
         /// Minecraft release these tables were generated from.\n\
         pub const GAME_VERSION: &str = {id:?};\n\
         /// Network protocol version sent in the handshake.\n\
         pub const PROTOCOL_VERSION: i32 = {proto};\n\
         /// World data version stored in level files and chunks.\n\
         pub const WORLD_VERSION: i32 = {world};\n\
         pub const RESOURCE_PACK_MAJOR: u32 = {rmaj};\n\
         pub const RESOURCE_PACK_MINOR: u32 = {rmin};\n\
         pub const DATA_PACK_MAJOR: u32 = {dmaj};\n\
         pub const DATA_PACK_MINOR: u32 = {dmin};\n",
        header = cx.header,
        id = v.id,
        proto = v.protocol_version,
        world = v.world_version,
        rmaj = v.pack_version.resource_major,
        rmin = v.pack_version.resource_minor,
        dmaj = v.pack_version.data_major,
        dmin = v.pack_version.data_minor,
    )
}
