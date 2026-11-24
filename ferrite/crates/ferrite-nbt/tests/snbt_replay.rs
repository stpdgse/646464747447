//! Replays answers recorded from the official SNBT parser (`vanilla_snbt.rs` writes them with
//! `FERRITE_SNBT_RECORD`), so this runs without the server jar.

mod common;

use common::{expected_lines, unescape};

#[test]
fn recorded_vanilla_answers_are_reproduced() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/snbt.txt"))
        .expect("recorded corpus");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len() % 3, 0, "corpus is three lines per case");
    let mut bad = 0;
    for case in lines.chunks(3) {
        let input = unescape(case[0]);
        let want = expected_lines(&input);
        for k in 0..2 {
            if want[k] != case[1 + k] {
                bad += 1;
                if bad <= 10 {
                    println!("{input:?}\n  vanilla {}\n  ferrite {}", case[1 + k], want[k]);
                }
            }
        }
    }
    assert_eq!(bad, 0, "{bad} answers differ from the recorded ones");
    assert!(lines.len() / 3 >= 3000);
}
