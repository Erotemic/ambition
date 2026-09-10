# Source-text guards — which ones fail silently when their spelling changes

**Two source-text guards were found blind on 2026-09-10 and neither was found by
looking.** One anchored on a contiguous `commands.insert_resource(` that
`rustfmt` had wrapped; one recognised a drop by the literal `GroundItem {` and
went blind when the type was sealed. A third — the writer census itself —
recognised construction by a hand-kept list of blessed method names and lost six
sites to the same seal. This page is the sweep those three argue for.

Measured 2026-09-10. Re-derive:

```bash
python3 scripts/measure_source_text_guard_exposure.py
python3 scripts/measure_source_text_guard_exposure.py --exposed-only
```

## The question, and why it is this one

⭐⭐ **THE AXIS IS DIRECTION, NOT LIKELIHOOD.** A guard blinded into reporting a
PHANTOM is self-limiting — somebody chases it, which is exactly what happened
with the `rustfmt` case. A guard blinded into reporting **"no offenders"** prints
a clean bill of health forever and is quoted as evidence. So the sweep asks one
mechanical question per test: **if the scan underneath it matched nothing, would
this test still pass?**

## What it found

121 guard files under `scripts/tests/` read source text; 685 test functions in
them.

| verdict | tests | meaning |
|---|---|---|
| exposed | 103 | a CEILING or an emptiness check, no floor — an empty scan reads as clean |
| equality | 345 | compares a derived set against a hand-written one; an empty scan fails LOUDLY |
| floored | 174 | an anti-vacuity assertion, or a fixture positive control requiring the checker to FAIL |
| unclassified | 63 | no assertion this sweep recognises — read them |

⛔ **"Exposed" is a place to look, not a finding.** Most of the 103 anchor on
something a formatter cannot touch. Intersecting exposure with a **rigid join** —
a literal joining tokens with `.`, `::`, `(` or `{` and no whitespace slack, the
shape all three of the real cases had — and discarding files whose siblings carry
a floor leaves **one** file, and it is fixture-driven.

## The one real case, named and reproduced

**`scripts/tests/test_text_spawns_resolve_a_font.py`** anchored on
`\bText(2d)?\s*::\s*(new|default)\b` — a hand-kept list of two constructor names,
with no anti-vacuity floor anywhere in the file.

⛔⛔ `bevy_ui-0.19.1` declares **`pub struct Text(pub String);`** — in the
registry copy of the dependency (widget text module, line 111), not anywhere in
this repo. So `Text("Play".into())` is legal, is unfonted, and was
invisible. MEASURED by appending one such spawn to a copy of
`crates/ambition_menu/src/render/bevy_ui/spawn.rs`:

| pattern | verdict on an unfonted `Text("Play".into())` spawn |
|---|---|
| `Text::(new\|default)` — as written | **PASSED — silently blind** |
| `\bText(?:2d)?\s*(?:::\s*[a-z_]\w*\s*)?\(` — the shape | caught |

Three further live spellings were blind to the old pattern and are not to the
new one: `Text::from(..)`, `Text::from_section(..)`, and any helper spelled
`Text::some_name(..)`. Both patterns find the same 5 constructions in the two
guarded files today, so the widening added no false positives. The guard now
also carries a floor (5 measured, floor 3) and a regression arm asserting the
tuple-struct spelling is seen and that `TextFont(`, `TextColor(` and
`TextLayout::new_with_justify(` are not.

## The remedy that beats a cleverer regex

⭐ Where a guard can be repointed at a spelling **the language makes canonical**,
that is worth more than widening its pattern. `the_death_drop_table_is_complete`
now watches `GroundItem::` rather than `GroundItem {`: because `GroundItem` is
`#[non_exhaustive]`, no crate outside `ambition_held_items` can spell a
construction any other way, so the anchor is closed rather than open. The same
move is available wherever a sealed type, a single public constructor, or an
enum's exhaustive match makes one spelling the only legal one.

Where it is not available, the rule is Rust's own shape rather than a list of
names: **an associated function is `Type::snake_case(`, a method is
`value.snake_case(`.** That is what replaced the blessed-name lists in both
`measure_state_writers.py` and the text-font guard.

## Anti-vacuity, stated plainly

**One real case, and it is fixed.** Everything else the mechanical pass raised
was the classifier being wrong, three times, always in the same direction —
calling a well-floored guard exposed:

- a floor asserted on a count variable (`assert count >= 10`);
- a floor living in a **sibling test** rather than the same function;
- a positive control spelled `assert checker.main() == 1` against a constructed
  fixture — the strongest control shape in this repo.

Each correction is an arm in `scripts/tests/test_source_text_guard_exposure.py`,
because a sweep whose false-positive rate is unpinned produces a list nobody
reads. The tree's guards are much better floored than the raw 103 suggests.

## The axis this sweep can only point at

⚠ **33 scripts strip `#[cfg(test)]` before scanning**, and stripping is *correct*
for most of them — `probe_dead_public_fns.py` should not count a test as a user,
and `measure_state_writers.py` should not report a fixture's capture buffer as
content state. It is a blind spot only for a guard that inherited the helper
without inheriting the question, and no text-level property distinguishes the
two. The evidence that the distinction is real: the writer census strips test
modules by design and its production count of `GroundItem` minting sites was
exactly right, while sealing the type revealed **13 further assembly sites in
test code** it could never have seen.

⚠ **Rust guards that read source text are not in this population** — 12 files
under `crates/` and `game/` read `.rs` text, including
`the_death_drop_table_is_complete`, which is where one of the two real cases was.
The sweep is Python-only because it classifies by Python AST.
