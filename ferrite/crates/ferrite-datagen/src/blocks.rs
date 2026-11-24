use std::collections::HashMap;
use std::fmt::Write;

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;

use crate::GenContext;
use crate::registries::Registry;

#[derive(Deserialize)]
struct RawBlock {
    #[serde(default)]
    properties: serde_json::Map<String, serde_json::Value>,
    states: Vec<RawState>,
}

#[derive(Deserialize)]
struct RawState {
    id: u32,
    #[serde(default)]
    default: bool,
    #[serde(default)]
    properties: serde_json::Map<String, serde_json::Value>,
}

struct Block {
    name: String,
    first: u32,
    count: u32,
    default: u32,
    props: Vec<(String, Vec<String>)>,
}

pub fn generate(cx: &GenContext, regs: &[Registry]) -> Result<String> {
    let path = cx.reports.join("blocks.json");
    let raw: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&std::fs::read(&path)?)?;

    let block_reg = regs
        .iter()
        .find(|r| r.key == "minecraft:block")
        .context("minecraft:block registry missing")?;
    ensure!(
        block_reg.names.len() == raw.len(),
        "blocks.json has {} blocks but the registry has {}",
        raw.len(),
        block_reg.names.len()
    );

    let mut blocks = Vec::with_capacity(raw.len());
    for name in &block_reg.names {
        let value = raw.get(name).with_context(|| format!("{name} missing from blocks.json"))?;
        let block: RawBlock =
            serde_json::from_value(value.clone()).with_context(|| format!("parsing {name}"))?;
        blocks.push(validate(name, block)?);
    }

    // Blocks must tile the state space in registry order with no gaps.
    let mut next = 0u32;
    for b in &blocks {
        ensure!(b.first == next, "{}: states start at {} but expected {next}", b.name, b.first);
        next += b.count;
    }
    let state_count = next;
    ensure!(state_count <= u32::from(u16::MAX), "state ids no longer fit in u16");

    let mut out = String::new();
    writeln!(out, "{}", cx.header)?;
    writeln!(out, "use crate::blocks::{{BlockInfo, Property}};\n")?;
    writeln!(out, "pub const STATE_COUNT: usize = {state_count};\n")?;

    // Deduplicated value lists.
    let mut value_lists: HashMap<Vec<String>, usize> = HashMap::new();
    let mut ordered_lists: Vec<Vec<String>> = Vec::new();
    for b in &blocks {
        for (_, values) in &b.props {
            if !value_lists.contains_key(values) {
                value_lists.insert(values.clone(), ordered_lists.len());
                ordered_lists.push(values.clone());
            }
        }
    }
    for (i, values) in ordered_lists.iter().enumerate() {
        write!(out, "static V{i}: [&str; {}] = [", values.len())?;
        for v in values {
            write!(out, "{v:?},")?;
        }
        writeln!(out, "];")?;
    }

    writeln!(out, "\npub static BLOCKS: [BlockInfo; {}] = [", blocks.len())?;
    for (id, b) in blocks.iter().enumerate() {
        write!(
            out,
            "BlockInfo {{ name: {:?}, id: {id}, first_state: {}, state_count: {}, default_state: {}, properties: &[",
            b.name, b.first, b.count, b.default
        )?;
        for (pname, values) in &b.props {
            write!(out, "Property {{ name: {pname:?}, values: &V{} }},", value_lists[values])?;
        }
        writeln!(out, "] }},")?;
    }
    writeln!(out, "];\n")?;

    write!(out, "pub static STATE_TO_BLOCK: [u16; {state_count}] = [")?;
    for (id, b) in blocks.iter().enumerate() {
        for _ in 0..b.count {
            write!(out, "{id},")?;
        }
    }
    writeln!(out, "];\n")?;

    // Keys are stored without the `minecraft:` prefix; all blocks are vanilla.
    let mut map = phf_codegen::Map::new();
    for (id, b) in blocks.iter().enumerate() {
        let short = b
            .name
            .strip_prefix("minecraft:")
            .with_context(|| format!("{} is not in the minecraft namespace", b.name))?;
        map.entry(short, &id.to_string());
    }
    writeln!(out, "pub static BY_NAME: phf::Map<&'static str, u16> = {};", map.build())?;
    Ok(out)
}

/// Checks the structural assumptions behind `BlockInfo::state_id`: contiguous ids, properties
/// sorted by name, last property varying fastest, value order as listed.
fn validate(name: &str, b: RawBlock) -> Result<Block> {
    // State ids are numbered over the properties *sorted by name* (vanilla's StateDefinition
    // keeps them in a sorted map); the report lists them in class-declaration order instead.
    let mut declared: Vec<_> = b.properties.iter().collect();
    declared.sort_by(|a, b| a.0.cmp(b.0));
    let props: Vec<(String, Vec<String>)> = declared
        .into_iter()
        .map(|(k, v)| {
            let values = v
                .as_array()
                .with_context(|| format!("{name}.{k}: values are not an array"))?
                .iter()
                .map(|x| x.as_str().map(str::to_owned).context("non-string property value"))
                .collect::<Result<Vec<_>>>()?;
            ensure!(!values.is_empty(), "{name}.{k} has no values");
            ensure!(values.len() <= 255, "{name}.{k} has too many values");
            Ok((k.clone(), values))
        })
        .collect::<Result<_>>()?;

    let expected: u64 = props.iter().map(|(_, v)| v.len() as u64).product();
    ensure!(
        b.states.len() as u64 == expected,
        "{name}: {} states but properties imply {expected}",
        b.states.len()
    );
    ensure!(!b.states.is_empty(), "{name} has no states");

    let first = b.states.iter().map(|s| s.id).min().expect("non-empty");
    let mut default = None;
    let mut seen = vec![false; b.states.len()];
    for s in &b.states {
        let offset = s.id.checked_sub(first).context("state id underflow")? as usize;
        match seen.get_mut(offset) {
            Some(slot) if !*slot => *slot = true,
            _ => bail!("{name}: state ids are not contiguous (id {})", s.id),
        }
        // Decode the offset as mixed radix (last property fastest) and compare.
        let mut rest = offset;
        for (pname, values) in props.iter().rev() {
            let idx = rest % values.len();
            rest /= values.len();
            let actual = s
                .properties
                .get(pname)
                .and_then(|v| v.as_str())
                .with_context(|| format!("{name}: state {} lacks property {pname}", s.id))?;
            ensure!(
                values[idx] == actual,
                "{name}: state {} has {pname}={actual} but layout predicts {}",
                s.id,
                values[idx]
            );
        }
        if s.default {
            ensure!(default.replace(s.id).is_none(), "{name}: multiple default states");
        }
    }
    Ok(Block {
        name: name.to_owned(),
        first,
        count: b.states.len() as u32,
        default: default.with_context(|| format!("{name}: no default state"))?,
        props,
    })
}
