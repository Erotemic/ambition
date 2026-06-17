# Runtime invariant not enforced at the loader (Idle row crash)

**Date:** 2026-05-24
**Tags:** `rust-module-refactor`, `runtime-invariants`, `bevy-asset-load`,
`fail-loud-vs-fail-soft`

## Mistake

During Phase 9.A of the character-catalog refactor, an agent (me)
added a manifest-driven fallback in the sprite-spec loader:

```rust
pub fn try_load_spec_for_character_id(character_id: &str) -> Option<CharacterSheetSpec> {
    let index = record_index();
    let record = index
        .get(character_id)
        .or_else(|| character_id.strip_prefix("npc_").and_then(|s| index.get(s)))?;
    Some(spec_from_record(record, &DEFAULT_TUNING))
}
```

The function loads any manifest under `assets/sprites/` and converts
its `SheetRecord` into a `CharacterSheetSpec` for the catalog
loader's fallback path.

Every unit test passed. Headless ran clean. The agent shipped the
commit.

The user then walked their game into the Hall of Characters and got
a panic:

```
thread 'Compute Task Pool (0)' panicked at
    crates/.../character_sprites/sheets.rs:759:14:
character sprite sheet must define an Idle row
```

`flat_index` in `sheets.rs` falls back to `Idle` for any animation
that doesn't have its own row, then `expect`s the Idle row exists.
A generator that emits only `walk` / `attack` / `death` (no idle
equivalent) produces a `CharacterSheetSpec` with zero Idle rows.
The catalog loader stored it. The renderer hit the first frame and
crashed.

## The invariant the agent missed

`CharacterSheetSpec` carries an implicit invariant: the rows MUST
contain a `CharacterAnim::Idle` (or an alias that maps to it). The
type system does not enforce this — `rows: Vec<(CharacterAnim,
AnimRow)>` allows any subset. The runtime relies on the invariant
holding because the atlas indexer falls back to Idle.

The agent should have validated the invariant at load time and
returned `None` for specs that violate it (caller falls back to the
colored-rectangle placeholder, no panic).

## Pre-mistake context

The agent had:
- The `CharacterAnim::from_name` source (which filters non-recognized
  row names out of the spec).
- The `flat_index` source (which unwraps the Idle row lookup).
- A bunch of working sprite manifests in `assets/sprites/` that all
  happened to include an idle row.
- A few generator targets the agent had recently touched
  (`ai_era_enemies.py`, `galwah.py`, `weird_hermit.py`) — some of
  these emit rows the agent could have spot-checked.

The compiler did not complain. The unit tests did not complain.
The headless smoke did not complain (it doesn't enter the Hall).

The mistake was "trust the manifest data" without enforcing the
runtime invariant at the load boundary.

## Repair shape (what the agent should produce)

```rust
let spec = spec_from_record(record, &DEFAULT_TUNING);
if spec.rows.iter().any(|(anim, _)| *anim == CharacterAnim::Idle) {
    Some(spec)
} else {
    bevy::log::warn!(
        target: "ambition::character_sprites",
        "character_sprites: skip spec for '{character_id}' \
         (manifest has no Idle row; rows = {:?})",
        spec.rows.iter().map(|(a, _)| a).collect::<Vec<_>>(),
    );
    None
}
```

Plus a paired test:

```rust
#[test]
fn every_catalog_sprite_spec_has_idle_row_if_loaded() {
    use crate::presentation::character_sprites::sheet_for_character_id;
    let data = load_embedded();
    for cid in data.characters.keys() {
        let Some(spec) = sheet_for_character_id(cid) else { continue };
        assert!(
            spec.rows.iter().any(|(anim, _)| matches!(anim, CharacterAnim::Idle)),
            "spec for '{cid}' lacks Idle row",
        );
    }
}
```

The test catches the regression in CI without needing to walk the
Hall.

## Why this is a good benchmark question

The agent has to:
1. Recognize that `expect("...")` in a deep render path is an
   invariant the loader must enforce.
2. Find the invariant — it's an implicit consequence of
   `resolve_anim` falling back to Idle, two function-hops away from
   the `flat_index` call site.
3. Decide where the enforcement belongs (load time, not panic time).
4. Write a test that exercises the invariant from the catalog side,
   not from a hypothetical render call.

Without seeing the panic, the agent has to know:
- Which functions assume Idle row presence.
- That the loader's job is to fail loud or fail silent — not pass
  bad data to the renderer.

## Compact question

> The runtime's `CharacterSheetSpec::flat_index` resolves an
> animation by falling back to `Idle` for any anim without its own
> row, then unwraps the Idle row's position. Your new loader,
> `try_load_spec_for_character_id`, returns a spec built from any
> RON manifest under `assets/sprites/`. Some published generators
> emit row names like `walk` / `attack` / `death` without an idle
> equivalent.
>
> Augment the loader so a misshaped manifest doesn't crash the
> renderer at first frame. Constraints: callers must keep working
> when they get a `None` (they already do — colored-rectangle
> fallback). The fix must be tested from the loader side, not the
> renderer side.

## Validation

```bash
~/.cargo/bin/cargo test -p ambition_gameplay_core --lib \
    every_catalog_sprite_spec_has_idle_row_if_loaded
```

Should fail before the fix lands, pass after.

## Post-mortem (2026-05-24, second-half wins)

The fix shipped in Phase 9.B (catalog-loader filter) and was extended
in 9.R with a third defensive layer:

1. **Publish-time (9.R, 2026-05-24).**
   `tackon_sheet.diagnose_idle_coverage` prints a stderr warning during
   `regen_sprites.sh` when a sheet has ≥1 `CharacterAnim` row but no
   Idle alias. The renderer author sees the issue at sheet-emit time,
   before the catalog even loads the manifest.
2. **Load-time (9.A/9.B, 2026-05-24).**
   `try_load_spec_for_character_id` returns `None` for manifests
   that lack an Idle row; the catalog logs the skipped id in the
   one-line startup INFO census so the placeholder fallback is
   diagnosable.
3. **Test-time (9.B, 2026-05-24).**
   `every_catalog_sprite_spec_has_idle_row_if_loaded` trips at
   `cargo test` time on any catalog entry whose manifest loads but
   doesn't define an Idle alias.

The recipe doc at `docs/recipes/adding-a-character.md` lists all
three layers under "**Idle row is mandatory**" so future authors
know where each catches the omission.

This benchmark candidate stays in the candidates dir as a frozen
record of the mistake; the layered fix is the answer the benchmark
question is looking for.
