//! Block loot tables and the named predicates they refer to, as the data generator wrote them.
//! They are embedded as minified JSON; `ferrite-loot` parses and evaluates them.

use std::fmt::Write;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::GenContext;

fn collect(dir: &Path, prefix: &str, out: &mut Vec<(String, String)>) -> Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {dir:?}"))?
        .collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let path = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            collect(&path, &format!("{prefix}{name}/"), out)?;
        } else if let Some(stem) = name.strip_suffix(".json") {
            let json: Value = serde_json::from_slice(&std::fs::read(&path)?)
                .with_context(|| format!("parsing {path:?}"))?;
            out.push((format!("minecraft:{prefix}{stem}"), serde_json::to_string(&json)?));
        }
    }
    Ok(())
}

pub fn generate(cx: &GenContext) -> Result<String> {
    let data = cx.reports.parent().unwrap_or(&cx.reports).join("data/minecraft");
    let mut blocks = Vec::new();
    collect(&data.join("loot_table/blocks"), "blocks/", &mut blocks)?;
    let mut predicates = Vec::new();
    collect(&data.join("predicate"), "", &mut predicates)?;
    let mut out = String::new();
    out.push_str(&cx.header);
    out.push('\n');
    blocks.sort();
    predicates.sort();
    for (label, list) in [("BLOCK_LOOT", &blocks), ("PREDICATES", &predicates)] {
        let _ = writeln!(out, "/// `(name, minified JSON)`, sorted by name.");
        let _ = writeln!(out, "pub static {label}: [(&str, &str); {}] = [", list.len());
        for (name, json) in list {
            let _ = writeln!(out, "    ({name:?}, {json:?}),");
        }
        out.push_str("];\n\n");
    }
    Ok(out)
}
