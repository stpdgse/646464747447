//! Timelines (the repeating schedules of the world clocks) and their named time markers, as the
//! data generator wrote them: `/time set noon` jumps to the next `minecraft:noon`.

use std::fmt::Write;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::GenContext;

pub fn generate(cx: &GenContext) -> Result<String> {
    let dir = cx.reports.parent().unwrap_or(&cx.reports).join("data/minecraft/timeline");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .with_context(|| format!("reading {dir:?}"))?
        .collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    let mut out = String::new();
    out.push_str(&cx.header);
    out.push_str("\nuse crate::timelines::{TimeMarker, Timeline};\n\n");
    let _ = writeln!(out, "pub static TIMELINES: [Timeline; {}] = [", entries.len());
    for e in entries {
        let name = e.file_name().to_string_lossy().trim_end_matches(".json").to_string();
        let json: Value = serde_json::from_slice(&std::fs::read(e.path())?)?;
        let clock = json["clock"].as_str().context("timeline clock")?;
        let period = match json.get("period_ticks").and_then(Value::as_i64) {
            Some(p) => format!("Some({p})"),
            None => "None".to_string(),
        };
        let mut markers = String::new();
        if let Some(m) = json.get("time_markers").and_then(Value::as_object) {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            for k in keys {
                let (ticks, show) = match &m[k] {
                    Value::Number(n) => (n.as_i64().context("marker ticks")?, false),
                    Value::Object(o) => (
                        o["ticks"].as_i64().context("marker ticks")?,
                        o.get("show_in_commands").and_then(Value::as_bool).unwrap_or(false),
                    ),
                    other => anyhow::bail!("marker {k}: {other}"),
                };
                let _ = write!(
                    markers,
                    "TimeMarker {{ name: {k:?}, ticks: {ticks}, show_in_commands: {show} }}, "
                );
            }
        }
        let _ = writeln!(
            out,
            "    Timeline {{ name: \"minecraft:{name}\", clock: {clock:?}, period_ticks: {period}, markers: &[{markers}] }},"
        );
    }
    out.push_str("];\n");
    Ok(out)
}
