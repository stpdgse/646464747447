//! Replays 2x2 crafting grids recorded from the official server (`scripts/record-crafting.py`):
//! the result slot must be the same item and count.

use ferrite_crafting::{Stack, craft};

#[test]
fn crafting_results_match_the_official_server() {
    let text = include_str!("data/crafting.txt");
    let (mut n, mut bad) = (0, Vec::new());
    for line in text.lines().filter(|l| l.starts_with("grid ")) {
        let (grid, result) = line["grid ".len()..].split_once(" -> ").unwrap();
        let cells: Vec<Stack> = grid
            .split_whitespace()
            .map(|c| match c.parse::<i32>() {
                Ok(item) => Stack { item, count: 1 },
                Err(_) => Stack::default(),
            })
            .collect();
        if cells.len() != 4 {
            continue; // a truncated final line
        }
        let want = result
            .split_once(':')
            .map(|(count, item)| (item.parse::<i32>().unwrap(), count.parse::<i32>().unwrap()));
        let got = craft(&cells, 2, 2).map(|(_, s)| (s.item, s.count));
        n += 1;
        if got != want {
            bad.push(format!("{grid}: vanilla {want:?}, ferrite {got:?}"));
        }
    }
    assert!(n > 300, "only {n} recorded grids");
    assert!(
        bad.is_empty(),
        "{} of {n} grids differ:\n{}",
        bad.len(),
        bad.iter().take(15).cloned().collect::<Vec<_>>().join("\n")
    );
}
