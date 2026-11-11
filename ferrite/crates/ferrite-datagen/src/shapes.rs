//! Per-block-state shape and property facts, extracted by `java/ShapeDump.java`.
//!
//! The vanilla data generator reports block states but not their geometry, and geometry decides
//! how fluids flow, what can support what, and which faces redstone and pistons can use. The
//! extractor asks the game's own registries for those facts; this module validates the table and
//! turns it into static Rust data.

use std::fmt::Write;

use anyhow::{Context, Result, bail, ensure};

use crate::GenContext;

type Box6 = [f64; 6];

struct Row {
    flags: u32,
    light_emission: u32,
    light_dampening: u32,
    piston: u32,
    fluid: u32,
    fluid_amount: u32,
    fluid_flags: u32,
    collision: u32,
    shape: u32,
    occlusion: u32,
    face_occlusion: [u32; 6],
    sturdy: u32,
    destroy_bits: u32,
    needs_tool: bool,
}

fn parse_boxes(line: &str) -> Result<Vec<Box6>> {
    if line == "-" {
        return Ok(Vec::new());
    }
    line.split(';')
        .filter(|s| !s.is_empty())
        .map(|b| {
            let v: Vec<f64> = b
                .split(',')
                .map(|n| n.parse::<f64>().with_context(|| format!("bad number {n:?}")))
                .collect::<Result<_>>()?;
            let arr: Box6 = v.try_into().map_err(|_| anyhow::anyhow!("box needs 6 numbers"))?;
            ensure!(
                arr.iter().all(|x| (-1.0..=2.0).contains(x)),
                "box {arr:?} is implausibly far outside the block cube"
            );
            ensure!(
                arr[0] <= arr[3] && arr[1] <= arr[4] && arr[2] <= arr[5],
                "inverted box {arr:?}"
            );
            Ok(arr)
        })
        .collect()
}

pub fn generate(cx: &GenContext) -> Result<String> {
    let path = cx.reports.parent().unwrap_or(&cx.reports).join("shapes.txt");
    let text = std::fs::read_to_string(&path).with_context(|| {
        format!("reading {path:?} (run without --reports, or supply shapes.txt)")
    })?;
    let mut lines = text.lines().filter(|l| !l.starts_with('#'));

    let header = lines.next().context("empty shapes file")?;
    let shape_count: usize =
        header.strip_prefix("shapes ").context("missing `shapes N`")?.parse()?;
    let mut shapes = Vec::with_capacity(shape_count);
    for _ in 0..shape_count {
        shapes.push(parse_boxes(lines.next().context("truncated shape list")?)?);
    }
    ensure!(shapes[0].is_empty(), "shape 0 must be the empty shape");

    let header = lines.next().context("missing `states N`")?;
    let n: usize = header.strip_prefix("states ").context("missing `states N`")?.parse()?;

    let mut rows = Vec::with_capacity(n);
    for (expect, line) in lines.by_ref().take(n).enumerate() {
        let v: Vec<u32> = line
            .split(' ')
            .map(|x| x.parse::<u32>().with_context(|| format!("bad field {x:?} in {line:?}")))
            .collect::<Result<_>>()?;
        ensure!(v.len() == 21, "state row has {} fields, want 21: {line:?}", v.len());
        ensure!(v[0] as usize == expect, "state ids are not sequential at {expect}");
        let row = Row {
            flags: v[1],
            light_emission: v[2],
            light_dampening: v[3],
            piston: v[4],
            fluid: v[5],
            fluid_amount: v[6],
            fluid_flags: v[7] | (v[8] << 1),
            collision: v[9],
            shape: v[10],
            occlusion: v[11],
            face_occlusion: [v[12], v[13], v[14], v[15], v[16], v[17]],
            sturdy: v[18],
            destroy_bits: v[19],
            needs_tool: v[20] != 0,
        };
        for s in [row.collision, row.shape, row.occlusion].into_iter().chain(row.face_occlusion) {
            ensure!((s as usize) < shapes.len(), "shape id {s} out of range");
        }
        ensure!(row.light_emission <= 15 && row.light_dampening <= 15, "light out of range");
        ensure!(row.fluid_amount <= 8, "fluid amount {} out of range", row.fluid_amount);
        rows.push(row);
    }
    ensure!(rows.len() == n && lines.next().is_none(), "trailing or missing state rows");
    if rows.iter().any(|r| r.flags >= 1 << 16) {
        bail!("flag bits exceed 16");
    }

    let mut out = String::new();
    out.push_str(&cx.header);
    out.push_str("\nuse crate::shapes::{Box6, StateFacts};\n\n");
    let _ = writeln!(out, "pub static SHAPES: [&[Box6]; {}] = [", shapes.len());
    for boxes in &shapes {
        out.push_str("    &[");
        for b in boxes {
            let _ = write!(
                out,
                "[{:?}, {:?}, {:?}, {:?}, {:?}, {:?}], ",
                b[0], b[1], b[2], b[3], b[4], b[5]
            );
        }
        out.push_str("],\n");
    }
    out.push_str("];\n\n");
    let _ = writeln!(out, "pub static STATE_FACTS: [StateFacts; {}] = [", rows.len());
    for r in &rows {
        let _ = writeln!(
            out,
            "    StateFacts::new({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, [{}, {}, {}, {}, {}, {}], {}),",
            r.flags,
            r.light_emission,
            r.light_dampening,
            r.piston,
            r.fluid,
            r.fluid_amount,
            r.fluid_flags,
            r.collision,
            r.shape,
            r.occlusion,
            r.face_occlusion[0],
            r.face_occlusion[1],
            r.face_occlusion[2],
            r.face_occlusion[3],
            r.face_occlusion[4],
            r.face_occlusion[5],
            r.sturdy
        );
    }
    out.push_str("];\n\n");
    out.push_str(
        "/// Per state: `(destroy speed bits as an f32, requires the correct tool for drops)`.\n",
    );
    let _ = writeln!(out, "pub static STATE_DESTROY: [(u32, bool); {}] = [", rows.len());
    for r in &rows {
        let _ = writeln!(out, "    ({}, {}),", r.destroy_bits, r.needs_tool);
    }
    out.push_str("];\n");
    Ok(out)
}
