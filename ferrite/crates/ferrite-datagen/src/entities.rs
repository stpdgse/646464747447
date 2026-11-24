//! Entity facts the reports leave out (hitbox, category, tracking range, default attributes),
//! extracted by `java/EntityDump.java`.

use std::fmt::Write;

use anyhow::{Context, Result, bail, ensure};

use crate::GenContext;

pub fn generate(cx: &GenContext) -> Result<String> {
    let path = cx.reports.parent().unwrap_or(&cx.reports).join("entities.txt");
    let text = std::fs::read_to_string(&path).with_context(|| {
        format!("reading {path:?} (run without --reports, or supply entities.txt)")
    })?;
    let mut lines = text.lines();
    let n: usize = lines
        .next()
        .and_then(|l| l.strip_prefix("entities "))
        .context("missing `entities N`")?
        .parse()?;
    let mut out = String::new();
    out.push_str(&cx.header);
    out.push_str("\nuse crate::entities::{Attributes, EntityInfo, MobCategory};\n\n");
    let _ = writeln!(out, "pub static ENTITIES: [EntityInfo; {n}] = [");
    let mut count = 0;
    for line in lines {
        let f: Vec<&str> = line.split(' ').collect();
        ensure!(f.len() == 16, "entity row has {} fields: {line:?}", f.len());
        let category = match f[4] {
            "monster" => "Monster",
            "creature" => "Creature",
            "ambient" => "Ambient",
            "axolotls" => "Axolotls",
            "underground_water_creature" => "UndergroundWaterCreature",
            "water_creature" => "WaterCreature",
            "water_ambient" => "WaterAmbient",
            "misc" => "Misc",
            other => bail!("unknown mob category {other}"),
        };
        let attr = |s: &str| -> String {
            if s == "-" { "None".to_string() } else { format!("Some({s}f64)") }
        };
        let attributes = if f[9] == "1" {
            format!(
                "Some(Attributes {{ max_health: {}, movement_speed: {}, attack_damage: {}, follow_range: {}, knockback_resistance: {}, armor: {} }})",
                attr(f[10]),
                attr(f[11]),
                attr(f[12]),
                attr(f[13]),
                attr(f[14]),
                attr(f[15])
            )
        } else {
            "None".to_string()
        };
        let _ = writeln!(
            out,
            "    EntityInfo {{ name: {:?}, width: {}f32, height: {}f32, eye_height: {}f32, category: MobCategory::{category}, tracking_range: {}, update_interval: {}, fire_immune: {}, summonable: {}, attributes: {attributes} }},",
            f[0],
            f[1],
            f[2],
            f[3],
            f[5],
            f[6],
            f[7] == "1",
            f[8] == "1"
        );
        count += 1;
    }
    ensure!(count == n, "{count} entity rows, expected {n}");
    out.push_str("];\n");
    Ok(out)
}
