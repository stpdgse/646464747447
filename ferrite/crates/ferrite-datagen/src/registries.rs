use std::collections::{BTreeMap, HashSet};
use std::fmt::Write;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::GenContext;
use crate::util::{const_name, module_name, order_by_id};

#[derive(Deserialize)]
struct RawRegistry {
    #[serde(default)]
    default: Option<String>,
    protocol_id: u32,
    entries: serde_json::Map<String, serde_json::Value>,
}

/// A registry with entries ordered by protocol id.
pub struct Registry {
    pub key: String,
    pub module: String,
    pub protocol_id: u32,
    pub default: Option<String>,
    pub names: Vec<String>,
}

pub fn load(cx: &GenContext) -> Result<Vec<Registry>> {
    let path = cx.reports.join("registries.json");
    let raw: BTreeMap<String, RawRegistry> = serde_json::from_slice(&std::fs::read(&path)?)
        .with_context(|| format!("parsing {path:?}"))?;

    let mut regs = Vec::new();
    let mut modules = HashSet::new();
    for (key, reg) in raw {
        let module = module_name(&key)?;
        if !modules.insert(module.clone()) {
            bail!("registry module name collision for {key}");
        }
        let items: Vec<(i64, String)> = reg
            .entries
            .iter()
            .map(|(name, v)| {
                let id = v
                    .get("protocol_id")
                    .and_then(|x| x.as_i64())
                    .with_context(|| format!("{key}/{name} has no protocol_id"))?;
                Ok((id, name.clone()))
            })
            .collect::<Result<_>>()?;
        let names = order_by_id(&key, &items)?;
        regs.push(Registry {
            key,
            module,
            protocol_id: reg.protocol_id,
            default: reg.default,
            names,
        });
    }
    regs.sort_by_key(|r| r.protocol_id);
    Ok(regs)
}

pub fn generate(cx: &GenContext, regs: &[Registry]) -> Result<String> {
    let mut out = String::new();
    writeln!(out, "{}", cx.header)?;
    writeln!(out, "use crate::registries::Registry;\n")?;

    for r in regs {
        let n = r.names.len();
        writeln!(out, "pub mod {} {{", r.module)?;
        writeln!(out, "use super::Registry;")?;
        writeln!(out, "pub const count: usize = {n};")?;

        let mut seen = HashSet::new();
        for (id, name) in r.names.iter().enumerate() {
            let c = const_name(name);
            if !seen.insert(c.clone()) {
                bail!("{}: constant name collision for {name} ({c})", r.key);
            }
            writeln!(out, "pub const {c}: u32 = {id};")?;
        }

        write!(out, "pub static names: [&str; {n}] = [")?;
        for name in &r.names {
            write!(out, "{name:?},")?;
        }
        writeln!(out, "];")?;

        let mut map = phf_codegen::Map::new();
        for (id, name) in r.names.iter().enumerate() {
            map.entry(name.as_str(), &id.to_string());
        }
        writeln!(out, "pub static by_name: phf::Map<&'static str, u32> = {};", map.build())?;

        let default = match &r.default {
            Some(d) => format!("Some({d:?})"),
            None => "None".to_string(),
        };
        writeln!(
            out,
            "pub static registry: Registry = Registry {{ name: {:?}, protocol_id: {}, entries: &names, by_name: &by_name, default: {default} }};",
            r.key, r.protocol_id
        )?;
        writeln!(out, "}}\n")?;
    }

    writeln!(out, "/// Every registry, ordered by registry protocol id.")?;
    writeln!(out, "pub static ALL: [&Registry; {}] = [", regs.len())?;
    for r in regs {
        writeln!(out, "&{}::registry,", r.module)?;
    }
    writeln!(out, "];")?;
    Ok(out)
}
