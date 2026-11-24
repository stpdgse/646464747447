//! Replays inventory clicks inside container windows recorded from the official server
//! (`scripts/record-windows.py`).

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

#[test]
fn window_clicks_match_the_official_server() {
    let text = include_str!("data/windows.txt");
    let mut menu = Menu::default();
    let (mut clicks, mut bad, mut history) = (0, Vec::new(), Vec::<String>::new());
    let mut scenario = String::new();
    for line in text.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let w: Vec<&str> = line.split_whitespace().collect();
        match w[0] {
            "scenario" => {
                scenario = line.to_string();
                history.clear();
                menu = Menu::new(match w[2] {
                    "crafting" => Layout::Crafting,
                    "chest3" => Layout::Generic(3),
                    _ => Layout::Generic(6),
                });
            }
            "init" => {
                let (carried, slots) = parse_state(line);
                assert_eq!(slots.len(), menu.slots.len(), "{scenario}");
                menu.carried = carried;
                menu.slots.copy_from_slice(&slots);
            }
            "click" => {
                history.push(line.to_string());
                menu.click(
                    w[1].parse().unwrap(),
                    w[2].parse().unwrap(),
                    Input::from_id(w[3].parse().unwrap()).unwrap(),
                    false,
                );
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
            _ => {}
        }
    }
    assert!(clicks > 0);
    assert!(
        bad.is_empty(),
        "{} of {clicks} clicks differ:\n{}",
        bad.len(),
        bad.iter().take(5).cloned().collect::<Vec<_>>().join("\n")
    );
}
