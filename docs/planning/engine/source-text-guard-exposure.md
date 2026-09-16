# Source-text guards — which ones fail silently when their spelling changes

**Two source-text guards were found blind on 2026-09-10 and neither was found by
looking; a third was found on 2026-09-16 with a working floor in place, and it is
the case this page had already predicted it could not see.** One anchored on a contiguous `commands.insert_resource(` that
`rustfmt` had wrapped; one recognised a drop by the literal `GroundItem {` and
went blind when the type was sealed. A third — the writer census itself —
recognised construction by a hand-kept list of blessed method names and lost six
sites to the same seal. This page is the sweep those three argue for.

Measured 2026-09-10, re-measured 2026-09-16 (population +20%, conclusions
unchanged). Re-derive:

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

Re-measured **2026-09-16**, six days on. The instrument has not changed since the
original sweep (last commit `318aa23e8`, 2026-09-10), so every number below moved
because the TREE did — three agents adding guards — and not because the measure
did. That separation is the only reason these two rows can be compared at all.

| verdict | 2026-09-10 | 2026-09-16 | meaning |
|---|---|---|---|
| exposed | 103 | **121** | a CEILING or an emptiness check, no floor — an empty scan reads as clean |
| equality | 345 | **418** | compares a derived set against a hand-written one; an empty scan fails LOUDLY |
| floored | 174 | **205** | an anti-vacuity assertion, or a fixture positive control requiring the checker to FAIL |
| unclassified | 63 | **79** | no assertion this sweep recognises — read them |
| **files / functions** | 121 / 685 | **137 / 823** | |

⭐ **THE SHAPE HELD WHILE THE POPULATION GREW A FIFTH.** Every category rose
roughly in proportion, and the shortlist below is STILL one file. A sweep whose
conclusion survives 138 new test functions written by people who had not read it
is measuring the codebase rather than the day it was run.

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

## The remedies, strongest first

⭐⭐ **A SECOND INPUT OF A DIFFERENT KIND beats everything below it**, and the
sweep found the example rather than being scoped to look for it: the `*_it_sync`
guards derive one set from SOURCE TEXT (`mod <name>;`) and one from a DIRECTORY
LISTING, then assert each difference is empty. Blind either side and the other is
still full, so it reddens. **A floor says "I saw N things"; cross-evidence says
"two independent worlds agree"** — and only the second survives the instrument
going blind, because the two inputs are not made of the same stuff. Worked twice
more since: the planning-citation guard that read a rollback schema row as a
misspelled condition id was narrowed by consulting
`RollbackRegistry::schema_dump()` alongside the condition catalog, both published
by the composed app, so neither can drift into a hand-kept list.

⭐ Failing that, where a guard can be repointed at a spelling **the language makes canonical**,
that is worth more than widening its pattern. `the_death_drop_table_is_complete`
now watches `GroundItem::` rather than `GroundItem {`: because `GroundItem` is
`#[non_exhaustive]`, no crate outside `ambition_held_items` can spell a
construction any other way, so the anchor is closed rather than open. The same
move is available wherever a sealed type, a single public constructor, or an
enum's exhaustive match makes one spelling the only legal one.

Failing THAT, the rule is Rust's own shape rather than a list of
names: **an associated function is `Type::snake_case(`, a method is
`value.snake_case(`.** That is what replaced the blessed-name lists in both
`measure_state_writers.py` and the text-font guard.

⚠ And only when none of the three is available does an **anti-vacuity floor**
come next — it does not stop the guard going blind, it makes the blindness
loud.

⛔⛤ **AND ON 2026-09-16 THE FIRST BULLET OF "WHAT THIS CANNOT SEE" WAS
REPRODUCED, WITH A FLOOR IN PLACE AND WORKING.** This page already warns that *"a
floor on a different collection than the ceiling protects nothing, and this sweep
cannot tell the two apart"*. Here is that case, measured:

`rollback_schema_usage` censused the rollback registration NAMES out of source
text. Its floor was `assert len(current["stable_schema_names"]) > 100` — real,
in the right file, and passing on 423 names. Meanwhile the runtime registry held
**493** rows, of which **73 were invisible to the scan and 21 fed the peer
checksum**. Three causes, none of them a formatter: the glob covered `crates/`
and 47 registrations live under `game/`; 26 names are colon-form against a
dot-requiring pattern; and the marker it followed is one of FOUR spellings of
"this file registers", including an extension trait that names no registrar at
all.

⇒ **A FLOOR PROVES THE SCAN FOUND SOMETHING. IT CANNOT PROVE THE SCAN LOOKED
EVERYWHERE**, and those are different properties. 423 is a fine number; it is
simply not a claim about 493. Nothing internal to the guard could have caught
this, because every part of it agreed with itself — what caught it was an
INDEPENDENT instrument over the same subject, the checked-in runtime dump, and
the only reason to compare them was noticing the repo kept the fact twice.

⚠ **SO THE EXPOSURE AXIS HAS A SECOND DIMENSION THIS SWEEP DOES NOT SCORE.** The
sweep asks *"if the scan matched nothing, would this test still pass?"* — a
question about the scan's OUTPUT being empty. This case's scan matched plenty and
its POPULATION was wrong, which no floor, control or equality check on the
guard's own output can see. The remedy is not a better floor; it is a second
derivation from a different source, and where a fact has a runtime owner, a
source scan should not be the one holding it. (`stable_schema_names` was deleted
rather than widened, for exactly that reason.)

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

## The Rust half, read by hand

The Python sweep classifies by AST, which the Rust half has none of here, so
`--rust` reads `assert!`/`assert_eq!` invocations textually and asks the same
question. That half matters: it is where one of the two real cases lived
(`the_death_drop_table_is_complete`).

```bash
python3 scripts/measure_source_text_guard_exposure.py --rust
```

**MEASURED: 12 files, ZERO exposed.** 4 floored, 1 equality against a
hand-written table, 5 cross-checked, 2 not guards at all.

⛔⛔ **AND THE INSTRUMENT CORRECTED THE HAND READING THAT PRECEDED IT.** Reading
these twelve by eye, I grouped `app_it_sync.rs` with its three siblings as one
"equality" shape. They are not the same: only `app_it_sync.rs` carries an
`assert_eq!`, and it is a DUPLICATE-`mod` check, not the comparison I credited.
All five are safe for a different reason than I wrote down — which is the
argument for the instrument over the prose.

**Nothing further found, and the reason is structural rather than lucky.**

| file | anchor | why an empty scan is not silent |
|---|---|---|
| `time/time_control/tests.rs` | `.decay_reaction_timers(`, `let dt = world_time.sim_dt()` | `assert!(scaled.len() >= 2, "the scan is broken, not the code")` |
| `audio/tests.rs` | `"fundsp"` and `"audio"` in a manifest | `assert!(ids.len() >= 2)` |
| `dev_tools/runtime_census.rs` | a system-name census | `assert!(found.len() >= 19)` |
| `content/falling_sand/tests.rs` | banned identifiers | `assert!(source.contains("SpawnParticleSignal"))` runs before the absence checks |
| `features/ecs/damage_drops/tests.rs` | `GroundItem::`, `PickupFeature::new(` | `assert_eq!(defined, guarded)` against a hand-written table |
| `app_it_sync.rs` and three siblings | `mod <name>;` | compares source text against a DIRECTORY LISTING |
| `sprite_sheet/build.rs`, `baked_sheet_rons.rs` | — | build-time bakers, not guards |

⭐⭐ **THE `*_it_sync` SHAPE IS THE ONE WORTH COPYING, and the shape classifier
called all five of them EXPOSED before it could see why.** One set comes from
SOURCE TEXT (`mod <name>;`), one from a DIRECTORY LISTING, and each difference is
asserted empty. Blind the text side and the disk side is still full, so `missing`
reddens; blind the disk side and `orphaned` reddens. **Neither can be silently
emptied by a spelling change, because the other is not made of spellings.** That
is a stronger anti-vacuity than any count floor, and reporting it as an exposure
would have sent someone to "fix" the best guard shape in the tree. `--rust` now
reports it as `cross-checked`, pinned by an arm.

⇒ An equality is only self-limiting when the expectation does not come from the
same scan as the subject. Cross-evidence is that condition made structural.

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
