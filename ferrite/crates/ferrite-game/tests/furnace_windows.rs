//! Clicks in furnace windows against the official server (`scripts/record-furnace-windows.py`):
//! the window, the furnace behind it (cooking restarts when the ingredient changes) and the
//! experience paid out when the output is taken.

use ferrite_crafting::Cooker;
use ferrite_game::furnace::Furnace;
use ferrite_game::inventory::{Input, Layout, Menu};
use ferrite_protocol::packets::play::ItemStack;

fn parse_stack(s: &str) -> ItemStack {
    match s.split_once(':') {
        Some((count, item)) => {
            ItemStack { count: count.parse().unwrap(), item: item.parse().unwrap() }
        }
        None => ItemStack::default(),
    }
}

fn parse_state(line: &str) -> (ItemStack, Vec<ItemStack>) {
    let mut parts = line.split_whitespace().skip(1);
    let carried = parse_stack(parts.next().unwrap());
    (carried, parts.map(parse_stack).collect())
}

fn named(s: &str) -> ItemStack {
    if s == "-" {
        return ItemStack::default();
    }
    let (name, n) = s.split_once('*').unwrap();
    ItemStack {
        count: n.parse().unwrap(),
        item: ferrite_data::registries::item::by_name[format!("minecraft:{name}").as_str()] as i32,
    }
}

/// The recorded `furnace ...` line.
struct Recorded {
    lit: u32,
    lit_total: u32,
    spent: u32,
    total: u32,
    slots: [ItemStack; 3],
    used: Vec<(String, u32)>,
    orbs: i32,
}

fn parse_furnace(line: &str) -> Recorded {
    let w: Vec<&str> = line.split_whitespace().collect();
    let used = if w[8] == "-" {
        Vec::new()
    } else {
        w[8].split(',')
            .map(|kv| {
                let (k, v) = kv.rsplit_once('=').unwrap();
                (k.to_string(), v.parse().unwrap())
            })
            .collect()
    };
    Recorded {
        lit: w[1].parse().unwrap(),
        lit_total: w[2].parse().unwrap(),
        spent: w[3].parse().unwrap(),
        total: w[4].parse().unwrap(),
        slots: [named(w[5]), named(w[6]), named(w[7])],
        used,
        orbs: w[9].strip_prefix("orbs=").unwrap().parse().unwrap(),
    }
}

fn load(f: &mut Furnace, r: &Recorded) {
    f.lit = r.lit;
    f.lit_total = r.lit_total;
    f.spent = r.spent;
    f.total = r.total;
    f.slots = r.slots;
    f.used = r.used.iter().cloned().collect();
}

#[test]
fn furnace_window_clicks_match_the_official_server() {
    let text = include_str!("data/furnace_windows.txt");
    let mut menu = Menu::default();
    let mut furnace = Furnace::new(Cooker::Furnace);
    let (mut clicks, mut bad, mut history) = (0, Vec::new(), Vec::<String>::new());
    let mut scenario = String::new();
    let mut first_furnace = true;
    let mut awarded = (0, 0);
    for line in text.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let w: Vec<&str> = line.split_whitespace().collect();
        match w[0] {
            "scenario" => {
                scenario = line.to_string();
                history.clear();
                let cooker = match w[2] {
                    "furnace" => Cooker::Furnace,
                    "blast_furnace" => Cooker::BlastFurnace,
                    _ => Cooker::Smoker,
                };
                menu = Menu::new(Layout::Furnace(cooker));
                furnace = Furnace::new(cooker);
                first_furnace = true;
            }
            "furnace" if first_furnace => {
                first_furnace = false;
                let r = parse_furnace(line);
                load(&mut furnace, &r);
            }
            "init" => {
                let (carried, slots) = parse_state(line);
                assert_eq!(slots.len(), menu.slots.len(), "{scenario}");
                menu.carried = carried;
                menu.slots.copy_from_slice(&slots);
            }
            "click" => {
                history.push(line.to_string());
                let before = menu.slots[2].count;
                menu.click(
                    w[1].parse().unwrap(),
                    w[2].parse().unwrap(),
                    Input::from_id(w[3].parse().unwrap()).unwrap(),
                    false,
                );
                furnace.set_slots([menu.slots[0], menu.slots[1], menu.slots[2]]);
                if menu.slots[2].count < before {
                    // Experience is rounded up with a probability: the recording tells which way.
                    let mut low = furnace.clone();
                    let min = low.take_experience(|| 1.0) as i32;
                    let max = furnace.take_experience(|| 0.0) as i32;
                    awarded = (min, max);
                } else {
                    awarded = (0, 0);
                }
            }
            "after" => {
                clicks += 1;
                let (carried, slots) = parse_state(line);
                if menu.carried != carried || menu.slots[..] != slots[..] {
                    let diff: Vec<String> = (0..slots.len())
                        .filter(|i| slots[*i] != menu.slots[*i])
                        .map(|i| {
                            format!("slot {i}: vanilla {:?} ferrite {:?}", slots[i], menu.slots[i])
                        })
                        .collect();
                    bad.push(format!(
                        "{scenario}, clicks {history:?}: cursor vanilla {carried:?} ferrite {:?}; {}",
                        menu.carried,
                        diff.join("; ")
                    ));
                    menu.carried = carried;
                    menu.slots.copy_from_slice(&slots);
                }
            }
            "furnace" => {
                let r = parse_furnace(line);
                let ours = (
                    furnace.lit,
                    furnace.lit_total,
                    furnace.spent,
                    furnace.total,
                    furnace.slots,
                    {
                        let mut u: Vec<_> =
                            furnace.used.iter().map(|(k, v)| (k.clone(), *v)).collect();
                        u.sort();
                        u
                    },
                );
                let theirs = (r.lit, r.lit_total, r.spent, r.total, r.slots, {
                    let mut u = r.used.clone();
                    u.sort();
                    u
                });
                let xp_ok = (awarded.0..=awarded.1).contains(&r.orbs);
                if ours != theirs || !xp_ok {
                    bad.push(format!(
                        "{scenario}, clicks {history:?}: furnace vanilla {theirs:?} orbs {} ferrite {ours:?} awarded {awarded:?}",
                        r.orbs
                    ));
                }
                load(&mut furnace, &r);
            }
            _ => {}
        }
    }
    assert!(clicks > 100, "{clicks} clicks");
    assert!(bad.is_empty(), "{} of {clicks} clicks differ:\n{}", bad.len(), bad.join("\n"));
}
