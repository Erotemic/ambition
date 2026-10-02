# Source-text guards: which ones fail silently

**State:** standing rules for guards that scan source text. The sweep found one
real blind guard (`scripts/tests/test_text_spawns_resolve_a_font.py`), and it is
fixed.

## The question

A guard blinded into a phantom finding is self-limiting: somebody chases it. A
guard blinded into "no offenders" prints a clean result forever and is quoted as
evidence. So ask of each guard: **if the scan underneath it matched nothing,
would this test still pass?**

## Instrument

```bash
python3 scripts/measure_source_text_guard_exposure.py               # Python guards
python3 scripts/measure_source_text_guard_exposure.py --exposed-only
python3 scripts/measure_source_text_guard_exposure.py --rust        # Rust guards
```

Verdicts: `exposed` (a ceiling or emptiness check with no floor), `equality`
(a derived set against a hand-written one), `floored` (an anti-vacuity
assertion or a positive control that requires the checker to fail),
`cross-checked` (two independent inputs), `unclassified` (read them).
"Exposed" is a place to look, not a finding. Most exposed guards anchor on
text a formatter cannot touch. The classifier's known false positives (a floor
on a count variable, a floor in a sibling test, `assert checker.main() == 1`
against a fixture) are pinned in `scripts/tests/test_source_text_guard_exposure.py`.

## Remedies, strongest first

1. **A second input of a different kind.** Derive one set from source text and
   one from another world (a directory listing, a runtime registry dump), and
   assert each difference is empty. The `*_it_sync` guards do this: `mod
   <name>;` against the directory listing. Neither side can be emptied by a
   spelling change, because the other side is not made of spellings. Where a
   fact has a runtime owner, read the runtime owner (for example
   `RollbackRegistry::schema_dump()`), not a source scan.
2. **A spelling the language makes canonical.** Anchor on a spelling that is
   the only legal one: a `#[non_exhaustive]` type's constructor
   (`GroundItem::`), a single public constructor, or an exhaustive match.
3. **Rust's own shape, not a list of names.** An associated function is
   `Type::snake_case(`; a method is `value.snake_case(`. A hand-kept list of
   blessed constructor names goes blind when someone uses another. For example,
   `bevy_ui` declares `pub struct Text(pub String)`, so `Text("Play".into())`
   is legal and a `Text::(new|default)` pattern cannot see it.
4. **An anti-vacuity floor.** It does not stop the guard going blind. It makes
   the blindness loud.

## What a floor cannot see

A floor proves the scan found something. It cannot prove the scan looked
everywhere. A scan can match plenty and still miss part of its population (a
glob that covers `crates/` but not `game/`, one of several spellings of "this
file registers"). Only a second derivation from a different source catches
that.

Many scripts strip `#[cfg(test)]` before scanning. That is correct for a census
of production users or content state. It is a blind spot for a guard whose
question includes test code. Decide which question the guard asks before you
reuse the stripping helper.
