//! The furnace model against ticks recorded from the official server (`scripts/record-furnace.py`).

use ferrite_crafting::Cooker;
use ferrite_game::furnace::Furnace;
use ferrite_protocol::packets::play::ItemStack;

fn item(name: &str) -> i32 {
    ferrite_data::registries::item::by_name[format!("minecraft:{name}").as_str()] as i32
}

fn slot(s: &str) -> ItemStack {
    if s == "-" {
        return ItemStack::default();
    }
    let (name, n) = s.split_once('*').expect("name*count");
    ItemStack { count: n.parse().unwrap(), item: item(name) }
}

fn show(s: &ItemStack) -> String {
    if s.count <= 0 {
        return "-".into();
    }
    let name = ferrite_data::registries::item::names[s.item as usize];
    format!("{}*{}", name.trim_start_matches("minecraft:"), s.count)
}

#[test]
fn furnaces_tick_like_the_official_server() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data");
    let mut text = String::new();
    for file in ["furnace.txt", "furnace2.txt"] {
        text += &std::fs::read_to_string(format!("{dir}/{file}")).expect("the recording");
        text.push('\n');
    }
    let mut cases = 0;
    let mut checks = 0;
    let mut failed = std::collections::HashSet::new();
    let mut current: Option<(Furnace, u32, String)> = None;
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let f: Vec<&str> = line.split(' ').collect();
        match f[0] {
            "S" => {
                let cooker = match f[1] {
                    "furnace" => Cooker::Furnace,
                    "blast_furnace" => Cooker::BlastFurnace,
                    "smoker" => Cooker::Smoker,
                    other => panic!("{other}"),
                };
                let mut fur = Furnace::new(cooker);
                fur.lit = f[2].parse().unwrap();
                fur.lit_total = f[3].parse().unwrap();
                fur.spent = f[4].parse().unwrap();
                fur.total = f[5].parse().unwrap();
                fur.slots = [slot(f[6]), slot(f[7]), slot(f[8])];
                current = Some((fur, 0, line.to_string()));
                cases += 1;
            }
            "T" => {
                let (fur, now, init) = current.as_mut().expect("a case");
                let tick: u32 = f[1].parse().unwrap();
                while *now < tick {
                    fur.tick();
                    *now += 1;
                }
                let got = format!(
                    "{} {} {} {} {} {} {}",
                    fur.lit,
                    fur.lit_total,
                    fur.spent,
                    fur.total,
                    show(&fur.slots[0]),
                    show(&fur.slots[1]),
                    show(&fur.slots[2]),
                );
                let want = f[2..9].join(" ");
                if std::env::var_os("FURNACE_REPORT").is_some() {
                    if got != want && !failed.contains(init) {
                        failed.insert(init.clone());
                        eprintln!("{init}\n  tick {tick}\n    got  {got}\n    want {want}");
                    }
                    continue;
                }
                assert_eq!(got, want, "after {tick} ticks of: {init}");
                let used =
                    fur.used.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(",");
                let used = if used.is_empty() { "-".to_string() } else { used };
                assert_eq!(used, f[9], "recipes used after {tick} ticks of: {init}");
                checks += 1;
            }
            other => panic!("unknown record {other}"),
        }
    }
    assert!(cases >= 600 && checks >= 10000, "{cases} cases, {checks} checks");
}
