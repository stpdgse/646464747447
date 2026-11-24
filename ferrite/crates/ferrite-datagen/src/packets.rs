use std::collections::BTreeMap;
use std::fmt::Write;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::GenContext;
use crate::util::{const_name, order_by_id, strip_namespace};

#[derive(Deserialize)]
struct PacketEntry {
    protocol_id: i64,
}

type Report = BTreeMap<String, BTreeMap<String, BTreeMap<String, PacketEntry>>>;

const STATES: [(&str, &str); 5] = [
    ("handshake", "Handshake"),
    ("status", "Status"),
    ("login", "Login"),
    ("configuration", "Configuration"),
    ("play", "Play"),
];
const DIRECTIONS: [(&str, &str); 2] =
    [("serverbound", "Serverbound"), ("clientbound", "Clientbound")];

pub fn generate(cx: &GenContext) -> Result<String> {
    let path = cx.reports.join("packets.json");
    let report: Report = serde_json::from_slice(&std::fs::read(&path)?)
        .with_context(|| format!("parsing {path:?}"))?;

    let mut out = String::new();
    writeln!(out, "{}", cx.header)?;
    writeln!(out, "use crate::{{Direction, State}};\n")?;

    let mut name_arms = String::new();
    let mut count_arms = String::new();

    for (state, state_variant) in STATES {
        writeln!(out, "pub mod {state} {{")?;
        for (dir, dir_variant) in DIRECTIONS {
            let entries = report
                .get(state)
                .and_then(|s| s.get(dir))
                .map(|d| d.iter().map(|(k, v)| (v.protocol_id, k.clone())).collect::<Vec<_>>())
                .unwrap_or_default();
            let ordered = order_by_id(&format!("packets {state}/{dir}"), &entries)?;

            writeln!(out, "pub mod {dir} {{")?;
            for (id, key) in ordered.iter().enumerate() {
                writeln!(out, "pub const {}: i32 = {id};", const_name(key))?;
            }
            writeln!(out, "pub const COUNT: usize = {};", ordered.len())?;
            write!(out, "pub static NAMES: [&str; {}] = [", ordered.len())?;
            for key in &ordered {
                write!(out, "{:?},", strip_namespace(key))?;
            }
            writeln!(out, "];\n}}")?;

            writeln!(
                name_arms,
                "(State::{state_variant}, Direction::{dir_variant}) => {state}::{dir}::NAMES.get(id as usize).copied(),"
            )?;
            writeln!(
                count_arms,
                "(State::{state_variant}, Direction::{dir_variant}) => {state}::{dir}::COUNT,"
            )?;
        }
        writeln!(out, "}}\n")?;
    }

    writeln!(
        out,
        "pub fn packet_name(state: State, direction: Direction, id: i32) -> Option<&'static str> {{\n\
         if id < 0 {{ return None; }}\n\
         match (state, direction) {{\n{name_arms}}}\n}}\n"
    )?;
    writeln!(
        out,
        "pub fn packet_count(state: State, direction: Direction) -> usize {{\n\
         match (state, direction) {{\n{count_arms}}}\n}}"
    )?;
    Ok(out)
}
