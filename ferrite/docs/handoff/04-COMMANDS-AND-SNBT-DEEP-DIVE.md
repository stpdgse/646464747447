# Commands, SNBT, blocks, items: everything learned

## Brigadier port (`ferrite-commands`)
- `reader.rs`: StringReader with UTF-16 positions; vanilla replaces Brigadier built-in messages
  with translation keys (`parsing.int.expected`, `argument.integer.low`, numeric args...).
  `SyntaxError.cursor: i64` (-1 = no position). `Message`/`Arg` (Str/Int/Long/Float/Double/Msg).
- Dispatcher parse algorithm: relevant nodes, potentials sorted, best exception selection (see
  `dispatch.rs`); redirects are marked with `>` in the oracle output.
- Command tree packet is per permission level; literal siblings sorted for reproducibility
  (vanilla uses hash order for some, e.g. gamerule; sorting only literals; argument order kept
  because `teleport` argument order matters, an earlier "sort everything" broke 4 oracle cases).
- Error display = two `system_chat` messages each wrapped red: the message, then gray context
  with "..." (cursor>10), pre-text (only if non-empty), red underlined remainder,
  `command.context.here` red italic.
- Chat commands send `disguised_chat` (chat_type = registry index + 1: emote_command 2,
  msg_incoming 3, msg_outgoing 4, say_command 5); `msg` sends outgoing then incoming.
- Selector rules found by oracle: `level`, `gamemode`, non-empty `advancements`, `type=player`
  make a selector players-only; tags tracked in `type_tags` for duplicate check; a tag sets
  `type_limited_inversely`; team negation inapplicable; entity-anchor error cursor at word start;
  score-holder "too many" cursor not reset to 0.

## 26.x time system
World clocks (`minecraft:overworld`, `the_end`), timelines with markers (day 1000, noon 6000,
night 13000, midnight 18000, roll_village_siege 18000, wake_up_from_sleep 0; day period 24000).
`time set <marker>` jumps to the next occurrence; `already_at_time_marker` if already there;
clock update still broadcast on marker failure. `set_time` packets carry clock updates (rate 0
when paused); the periodic 20-tick sync carries none. `/tick`: TickingState/TickingStep packets;
run loop uses `recv_timeout`.

## Teleport details
PlayerPosition relative flag bits X1 Y2 Z4 YROT8 XROT16 DX32 DY64 DZ128 (per relative axis: pos bit
+ delta bit); no rotation given -> rot bits 0x18; server-side position keeps raw offsets for
relative axes until client ack; `PlayerLookAt` for facing (eye height f32 1.62); bounds +-30,000,000.

## Furnace state machine (26.3)
Ignition rescales progress with f32 math; speed multiplier from fuel (blast/smoker 2.0, other 1.0);
`>=` completion check; setItem reset rule; unlit decay by 2; XP via `RecipesUsed`; a furnace placed by
command starts with cookTime 0 and finishes an item on its first tick.

## Block state / predicate args (`blockstate.rs`)
id via identifier read; unknown -> `argument.block.id.invalid [full id]` with cursor reset to the START
of the id (a bug found late: we had it at the end). `[` property loop errors: unknown/duplicate/novalue/
invalid/unclosed with exact cursors; `#tag` disallowed for setblock-style (`argument.block.tag.disallowed`)
but allowed in predicates; `{..}` = block-entity NBT via the SNBT parser (`nbt::read_compound`).

## NBT args (`nbt.rs`)
`read_tag` = `TagParser.parseAsArgument`: consumes leading whitespace + one value, error cursor =
reader cursor + SNBT cursor. `read_compound` adds `argument.nbt.expected.compound` at the cursor AFTER
the value when it is not a compound. Selector `nbt=` supports `!` negation and repeats.

## SNBT parser (`ferrite-nbt/src/parse.rs`), full rules
Values: every token skips leading whitespace (Java `Character.isWhitespace`: space, \t \n \v \f \r,
U+001C-001F, U+1680, U+2000-200A except 2007, U+2028/9, U+205F, U+3000; NOT NBSP, 2007, 202F).
Even `1 .5`, `- 1`, `0 x1`, `1 b` are single numbers.
Numbers:
- float: `sign? (int . frac? exp? | . frac exp? | int exp) [fFdD]?`; leading zeros allowed (`00.5`);
  digits may contain `_` between digits (trailing/leading `_` makes that rule fail); overflow to
  infinity fails the float alternative (and registers `infinity_not_allowed` at the end); `-0.0` -> `0.0`.
- integer: decimal numeral is `0` (not followed, after whitespace, by a valid digit run) or `[1-9][0-9_]*`;
  `0x`/`0X` hex (cut: no digits => error `expected_hex_numeral`, no decimal fallback), `0b`/`0B` binary
  (soft). Then optional marker `u|U` (unsigned) or `s|S` (signed) and optional type suffix `b s i l`.
  Reading order: marker+type, type only, nothing. Radix != 10 defaults to UNSIGNED semantics,
  decimal to signed. Negative + unsigned => `expected_non_negative_number`. Negative hex/bin untyped
  and unmarked => silent total stop (no function-call fallback).
- conversion messages mimic Java: byte/short signed => parseInt first (`For input string: "X"`
  [+ ` under radix R`]) then `Value out of range. Value:"X" Radix:R`; unsigned byte/short => parseInt then
  `out of range: N`; unsigned int/long => `String value X exceeds range of unsigned int|long.`;
  int/long signed => `For input string: "X"`. Typed failures are loud errors at the end of the literal;
  an untyped overflow is a silent stop.
Strings: `'`/`"`; escapes `\b \s \t \n \f \r \\ \' \"`, `\xHH`, `\uHHHH`, `\UHHHHHHHH` (>10FFFF =>
`invalid_codepoint U+%08X`), `\N{NAME}` (Unicode name, exact, via `unicode_names2` + exact-name check);
whitespace is skipped after the backslash; unterminated => `invalid_string_contents` at end. Lone
surrogates become U+FFFD (Rust `String` limitation; vanilla keeps them: KNOWN DIVERGENCE).
Unquoted strings: `[0-9A-Za-z_\-.+]`; as VALUES must start with `[A-Za-z_]`; `true`/`false`
case-insensitive => byte.
Compound: keys quoted or unquoted (any name chars); empty key => `empty_key`; duplicate keys last wins;
trailing comma ok. Lists: mixed types allowed (`List::Mixed`, written wrapped as `{"": x}` on the wire);
typed arrays `[B;`,`[I;`,`[L;` (elements checked and converted only at the closing `]`; `invalid_array_element_type`
etc. are reported after the `]`; B accepts b-suffix, I accepts b/s/i, L all).
Built-ins: `bool(x)` (any numeric or true/false), `uuid("...")` (Java `UUID.fromString` leniency).
Function call alternative: any unquoted-name `name(args)`; names starting with a non-letter =>
`invalid_unquoted_start` (after the call); unknown => `no_such_operation name/arity` at the end;
once `(` is read the call is committed EXCEPT a quirk: for `bool`/`uuid` with exactly one acceptable
argument that then fails (missing `)`, junk, failing 2nd arg) vanilla returns the built-in's value with
the cursor just after the NAME. Reproduced deliberately.
Error selection: furthest position wins, FIRST registered at that position wins. The registration
order in the parser is carefully arranged to reproduce this (float alternative before integer, number
before function call, `LIT[...]` notes for the next expected token such as `.`, `e|E`, `f|F`, `d|D`,
`x|X`, `u|U`, `b|B`, `+`, `0`, `,`, `]`, `}`). If a new mismatch appears, add/reorder a `note` call.

## Item stacks (`item.rs`, `components.rs`)
`id[comp=value,!comp,...]`. Unknown id: `argument.item.id.invalid` at id start. Component names read like
identifiers (empty name => `minecraft:`); unknown => `arguments.item.component.unknown` at the NAME start
(`[!x]`: after `!`). Transient components (no persistent codec) are unknown: `creative_slot_lock`,
`additional_trade_cost`, `map_post_processing`. Repeated (set or remove twice) =>
`arguments.item.component.repeated` with cursor -1. Missing `=` => `parsing.expected "="`. Value = SNBT
(errors propagate with absolute cursor), then codec check: failure =>
`arguments.item.component.malformed [name, reason]` with cursor at the VALUE start. After the list:
`parsing.expected "]"`; trailing comma allowed; whitespace everywhere.
Validated so far: max_stack_size [1;99] "Value must be within range [1;99]: N", max_damage (positive), damage /
repair_cost (non-negative), unbreakable (`Not a map: <snbt>`), enchantment_glint_override (any number),
custom_data (compound or string that parses to a compound; errors below), rarity.
Codec messages come from DataFixerUpper: e.g. `Not a number`, `Not a string`, `Failed to parse either.
First: ...; Second: ...`, `No key X in MapLike[{...}]`, `... missed input: {...}`, registry misses
(`Failed to get element minecraft:nope from registry minecraft:enchantment`). Porting ~110 codecs by
hand is the long tail; plan in 06.
custom_data string case: the second alternative parses the string as SNBT and the message is the
RENDERED English exception: `Expected literal : at position 2: {a<--[HERE]`, `Expected compound tag, got 1`,
`Unexpected trailing data at position 5: {a:1}<--[HERE]`, `Failed to parse number: ... at position N: ...`.
Needs: en_us strings (now generated: `ferrite_data::lang::{en_us,render}`) + Brigadier context formatting
(`...` prefix if cursor>10, last 10 chars before cursor, `<--[HERE]`), then `message at position N: context`.

## Command output facts
Chat/system messages are components; `ferrite-text` encodes bare string for plain text, compound otherwise,
`{"": x}` wraps non-compound elements of mixed lists; click_event/hover_event use the flat layout.
