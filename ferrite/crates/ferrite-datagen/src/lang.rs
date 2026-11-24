//! The English translation table (`en_us.json`): the server renders some messages itself (log
//! lines, codec error texts), and for those it needs the strings clients would otherwise show.

use std::fmt::Write as _;
use std::io::Read;

use anyhow::{Context, Result, bail};

use crate::GenContext;

/// Finds the version's own server jar inside the bundler jar.
fn inner_server_jar(bundle: &[u8]) -> Result<Vec<u8>> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bundle)).context("opening bundler")?;
    let name = zip
        .file_names()
        .filter_map(|n| n.ok().map(|c| c.into_owned()))
        .find(|n| n.starts_with("META-INF/versions/") && n.ends_with(".jar"))
        .context("no server jar inside the bundler")?;
    let mut bytes = Vec::new();
    zip.by_name(&name)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

pub fn generate(cx: &GenContext, bundle: &[u8]) -> Result<String> {
    let inner = inner_server_jar(bundle)?;
    let mut zip =
        zip::ZipArchive::new(std::io::Cursor::new(inner)).context("opening server jar")?;
    let mut text = String::new();
    zip.by_name("assets/minecraft/lang/en_us.json")
        .context("server jar has no en_us.json")?
        .read_to_string(&mut text)?;
    let map: std::collections::BTreeMap<String, String> =
        serde_json::from_str(&text).context("parsing en_us.json")?;
    if map.len() < 1000 {
        bail!("en_us.json looks truncated ({} keys)", map.len());
    }
    let mut out = String::new();
    out.push_str(&cx.header);
    out.push_str("\n/// English strings by translation key.\n");
    out.push_str("/// Sorted by key, for binary search.\n");
    let _ = writeln!(out, "pub static EN_US: [(&str, &str); {}] = [", map.len());
    for (k, v) in &map {
        let _ = writeln!(out, "    ({k:?}, {v:?}),");
    }
    out.push_str("];\n");
    Ok(out)
}
