//! The command tree the server sends to players of each permission level, as the official
//! server builds it (`java/CommandDump.java`). Ferrite sends these bytes as they are and decodes
//! the largest to parse commands.

use std::fmt::Write;

use anyhow::{Context, Result, ensure};

use crate::GenContext;

pub fn generate(cx: &GenContext) -> Result<String> {
    let path = cx.reports.parent().unwrap_or(&cx.reports).join("commands.txt");
    let text = std::fs::read_to_string(&path).with_context(|| {
        format!("reading {path:?} (run without --reports, or supply commands.txt)")
    })?;
    let mut trees: Vec<Vec<u8>> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let f: Vec<&str> = line.split(' ').collect();
        ensure!(f.len() == 3 && f[0] == "level" && f[1].parse::<usize>()? == i, "bad row {i}");
        ensure!(f[2].len() % 2 == 0, "odd hex length");
        let bytes = (0..f[2].len() / 2)
            .map(|k| u8::from_str_radix(&f[2][2 * k..2 * k + 2], 16))
            .collect::<Result<Vec<u8>, _>>()?;
        trees.push(bytes);
    }
    ensure!(trees.len() == 5, "expected 5 permission levels, got {}", trees.len());
    let mut out = String::new();
    out.push_str(&cx.header);
    out.push_str(
        "\n/// The body of the `commands` packet for permission levels 0 to 4 (player, moderator,\n/// gamemaster, admin, owner), as the dedicated server sends it.\n",
    );
    out.push_str("pub static COMMAND_TREES: [&[u8]; 5] = [\n");
    for bytes in &trees {
        out.push_str("    b\"");
        for b in bytes {
            let _ = write!(out, "\\x{b:02x}");
        }
        out.push_str("\",\n");
    }
    out.push_str("];\n");
    Ok(out)
}
