//! Replays inventory clicks recorded from the official server (`scripts/record-inventory.py`)
//! against Ferrite's inventory logic and requires the same inventory and cursor after each click.

use ferrite_game::inventory::{Input, Menu, SLOTS};
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
    let slots: Vec<ItemStack> = parts.map(parse_stack).collect();
    assert_eq!(slots.len(), SLOTS, "{line}");
    (carried, slots)
}

#[test]
fn inventory_clicks_match_the_official_server() {
    let text = include_str!("data/inventory.txt");
    let mut menu = Menu::default();
    let (mut scenarios, mut clicks, mut bad) = (0, 0, Vec::new());
    let mut history: Vec<String> = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        match words[0] {
            "scenario" => {
                scenarios += 1;
                history.clear();
            }
            "init" => {
                let (carried, slots) = parse_state(line);
                menu = Menu::default();
                menu.carried = carried;
                menu.slots.copy_from_slice(&slots);
            }
            "click" => {
                let (slot, button, kind): (i32, i32, i32) = (
                    words[1].parse().unwrap(),
                    words[2].parse().unwrap(),
                    words[3].parse().unwrap(),
                );
                history.push(line.to_string());
                menu.click(slot, button, Input::from_id(kind).unwrap(), false);
            }
            "after" => {
                clicks += 1;
                let (carried, slots) = parse_state(line);
                if menu.carried != carried || menu.slots[..] != slots[..] {
                    bad.push(format!(
                        "scenario {scenarios}, after {:?}:\n  vanilla  cursor {:?} slots {:?}\n  ferrite  cursor {:?} slots {:?}",
                        history,
                        carried,
                        slots.iter().enumerate().filter(|(_, s)| s.count > 0).collect::<Vec<_>>(),
                        menu.carried,
                        menu.slots.iter().enumerate().filter(|(_, s)| s.count > 0).collect::<Vec<_>>(),
                    ));
                    // Resynchronise so one difference is reported once.
                    menu.carried = carried;
                    menu.slots.copy_from_slice(&slots);
                }
            }
            _ => {}
        }
    }
    assert!(clicks > 0, "no recorded clicks");
    assert!(
        bad.is_empty(),
        "{} of {clicks} clicks differ:\n{}",
        bad.len(),
        bad.iter().take(6).cloned().collect::<Vec<_>>().join("\n")
    );
}
