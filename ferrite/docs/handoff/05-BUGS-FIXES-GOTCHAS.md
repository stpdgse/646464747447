# Bugs, fixes, gotchas (chronological-ish)

- **Disk full** repeatedly: delete `target/miri target/release target/doc target/criterion
  target/debug/incremental`, and `target/debug/build` entries older than ~3 h.
- **Text-replace scripts silently missed** after rustfmt reflowed code (is_supported list, selector
  value types, anchor error). Always re-read the actual text before patching; assert the old text
  exists (`assert old in s`). One replace truncated commands.rs's tail; restore with
  `git show HEAD:./crates/...`.
- **Oracle server stalls with no players** (pause-when-empty) => `pause-when-empty-seconds=-1`.
  Whitelist / allow-flight kicks => set properties.
- **ferrite-interact crashed on set_player_inventory in smaller windows**: slot mapping fixed.
- Parse differential test initially skipped types wrongly; replaced `is_supported` with explicit
  `unsupported()` errors; the wide corpus then found the selector rules (see 04).
- Pushed with a failing `time set abc` replay test once (time not implemented yet); later leftover
  state from the previous scenario => explicit `rcon time set 54` at each scenario start.
- Run loop hang in replay at `tick rate 10000`: collect with a deadline; `recv_timeout` loop.
- **CI datagen non-determinism**: gamerule children come in JVM hash order. Fix: sort only
  literal siblings in CommandDump (`TreeMap` for literals, argument nodes keep order).
- Clippy: boxed large enum variants (`Arg::Text(Box<Component>)`, `Failure(Box<Component>)`,
  `Value::Selector(Box<Selector>)`, `ScoreHolder::Selector(Box<..>)`), collapsible_if (use let-chains),
  `enum_variant_names`, `useless_format`. Always run `cargo clippy --workspace --all-targets -- -D warnings`.
- Block id error cursor: vanilla resets to the id start (we had end) - fixed.
- SNBT: dozens of grammar details discovered by fuzzing (see 04); biggest conceptual ones:
  whitespace skipping inside numbers, `bool(`/`uuid(` quirk, hex cut, silent stops vs loud errors,
  furthest-then-first error choice, `Hard` vs `Stop` vs `Soft` number results.
- `Tag::String` cannot hold lone surrogates (vanilla can): the fuzz oracle normalises them to U+FFFD.
- `unicode_names2::character` is forgiving about spacing; we double check with `name(ch)` equality.
- phf `phf_map!` macro needs the `macros` feature; the language table is a sorted slice + binary search.
- datagen `zip::ZipArchive::file_names()` yields `Result<Cow<str>>` in this zip version.
- The stop-hook complains about uncommitted changes: commit+push after each unit of work.
- `java` prints `Picked up JAVA_TOOL_OPTIONS...` on stderr; filter with `grep -v Picked`.
- Local `cd` is reset between Bash calls in the cloud harness; use absolute paths.
