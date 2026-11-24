//! Generates the data a server sends during the configuration phase: the entries of every
//! synchronized registry and the fully resolved tags.
//!
//! * Entry ids of a synchronized registry are the positions of its entry names sorted bytewise.
//! * A tag's ids are resolved depth-first, de-duplicated by first occurrence; `#tag` references
//!   are expanded; `{"id": .., "required": false}` entries that do not exist are skipped.
//!
//! Both rules were derived from, and are checked against, a real vanilla server (see
//! `ferrite-protocol/tests/oracle.rs`).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

use crate::GenContext;
use crate::registries::Registry;

const ORDER_FILE: &str = include_str!("../configuration-order.txt");

struct Order {
    registry_data: Vec<String>,
    update_tags: Vec<String>,
}

fn parse_order() -> Result<Order> {
    let (mut registry_data, mut update_tags) = (Vec::new(), Vec::new());
    let mut section = "";
    for line in ORDER_FILE.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        match line {
            "[registry_data]" => section = "registry_data",
            "[update_tags]" => section = "update_tags",
            name => match section {
                "registry_data" => registry_data.push(name.to_owned()),
                "update_tags" => update_tags.push(name.to_owned()),
                _ => bail!("configuration-order.txt: {name:?} appears before a section header"),
            },
        }
    }
    Ok(Order { registry_data, update_tags })
}

/// All `*.json` files under `dir`, as `minecraft:<relative path without extension>`.
fn json_names(dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).with_context(|| format!("reading {d:?}"))? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "json") {
                let rel = path.strip_prefix(dir)?.with_extension("");
                let name = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push((format!("minecraft:{name}"), path));
            }
        }
    }
    out.sort();
    Ok(out)
}

pub fn generate(cx: &GenContext, regs: &[Registry]) -> Result<String> {
    let order = parse_order()?;
    let data = cx.reports.parent().context("reports dir has no parent")?.join("data/minecraft");

    // Entry ids per registry key: static registries from the reports, dynamic ones by sort order.
    let mut ids: HashMap<String, HashMap<String, u32>> = HashMap::new();
    for r in regs {
        ids.insert(
            r.key.clone(),
            r.names.iter().enumerate().map(|(i, n)| (n.clone(), i as u32)).collect(),
        );
    }
    let mut dynamic: Vec<(String, Vec<String>)> = Vec::new();
    for key in &order.registry_data {
        let rel = key.strip_prefix("minecraft:").context("registry outside minecraft:")?;
        let names: Vec<String> = json_names(&data.join(rel))?.into_iter().map(|(n, _)| n).collect();
        ensure!(!names.is_empty(), "synchronized registry {key} has no entries on disk");
        ids.insert(
            key.clone(),
            names.iter().enumerate().map(|(i, n)| (n.clone(), i as u32)).collect(),
        );
        dynamic.push((key.clone(), names));
    }

    let mut out = String::new();
    writeln!(out, "{}", cx.header)?;
    writeln!(out, "use crate::configuration::{{SyncRegistry, TagDef, TagRegistry}};\n")?;

    writeln!(
        out,
        "/// Synchronized registries in the order vanilla sends them; entry id = position.\n\
         pub static SYNCHRONIZED_REGISTRIES: [SyncRegistry; {}] = [",
        dynamic.len()
    )?;
    for (key, names) in &dynamic {
        write!(out, "SyncRegistry {{ id: {key:?}, entries: &[")?;
        for n in names {
            write!(out, "{n:?},")?;
        }
        writeln!(out, "] }},")?;
    }
    writeln!(out, "];\n")?;

    writeln!(
        out,
        "/// Tag registries (order unspecified by vanilla); tags are sorted by name.\n\
         pub static TAG_REGISTRIES: [TagRegistry; {}] = [",
        order.update_tags.len()
    )?;
    let mut total_tags = 0;
    for key in &order.update_tags {
        let rel = key.strip_prefix("minecraft:").context("tag registry outside minecraft:")?;
        let reg_ids = ids.get(key).with_context(|| format!("no ids for tag registry {key}"))?;
        let tag_dir = data.join("tags").join(rel);
        let mut raw: BTreeMap<String, serde_json::Value> = BTreeMap::new();
        for (name, path) in json_names(&tag_dir)? {
            raw.insert(name, serde_json::from_slice(&std::fs::read(&path)?)?);
        }
        writeln!(out, "TagRegistry {{ registry: {key:?}, tags: &[")?;
        for name in raw.keys() {
            let resolved = resolve(key, name, &raw, reg_ids, &mut Vec::new())?;
            ensure!(
                resolved.iter().all(|&i| i <= u32::from(u16::MAX)),
                "{key}/{name}: id does not fit u16"
            );
            write!(out, "TagDef {{ name: {name:?}, entries: &[")?;
            for i in &resolved {
                write!(out, "{i},")?;
            }
            writeln!(out, "] }},")?;
            total_tags += 1;
        }
        writeln!(out, "] }},")?;
    }
    writeln!(out, "];")?;
    eprintln!(
        "configuration: {} synchronized registries, {} tag registries, {total_tags} tags",
        dynamic.len(),
        order.update_tags.len()
    );
    Ok(out)
}

fn resolve(
    registry: &str,
    name: &str,
    all: &BTreeMap<String, serde_json::Value>,
    ids: &HashMap<String, u32>,
    stack: &mut Vec<String>,
) -> Result<Vec<u32>> {
    if stack.iter().any(|s| s == name) {
        bail!("{registry}: tag reference cycle through {name}: {stack:?}");
    }
    stack.push(name.to_owned());
    let tag = all.get(name).with_context(|| format!("{registry}: tag {name} does not exist"))?;
    let values = tag
        .get("values")
        .and_then(|v| v.as_array())
        .with_context(|| format!("{registry}/{name}: no values array"))?;

    let mut out: Vec<u32> = Vec::new();
    let push = |id: u32, out: &mut Vec<u32>| {
        if !out.contains(&id) {
            out.push(id);
        }
    };
    for value in values {
        let (id_str, required) = match value {
            serde_json::Value::String(s) => (s.as_str(), true),
            serde_json::Value::Object(o) => (
                o.get("id").and_then(|v| v.as_str()).context("tag entry object without id")?,
                o.get("required").and_then(|v| v.as_bool()).unwrap_or(true),
            ),
            other => bail!("{registry}/{name}: unsupported tag entry {other}"),
        };
        if let Some(reference) = id_str.strip_prefix('#') {
            if !required && !all.contains_key(reference) {
                continue;
            }
            for id in resolve(registry, reference, all, ids, stack)? {
                push(id, &mut out);
            }
        } else {
            match ids.get(id_str) {
                Some(&id) => push(id, &mut out),
                None if !required => {}
                None => bail!("{registry}/{name}: unknown entry {id_str}"),
            }
        }
    }
    stack.pop();
    Ok(out)
}
