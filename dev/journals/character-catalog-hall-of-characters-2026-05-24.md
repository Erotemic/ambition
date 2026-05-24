# Character catalog refactor + Hall of Characters

**Date:** 2026-05-24
**Duration:** ~3.5 h wall-clock for plan (1.10 h) + bonus follow-ups (2.4 h)
**Plan doc:** [`TODO-character-catalog-and-hall.md`](../../TODO-character-catalog-and-hall.md)
**ADR:** [`docs/adr/0017-rust-behavior-ron-content-ldtk-space.md`](../../docs/adr/0017-rust-behavior-ron-content-ldtk-space.md)
**System doc:** [`docs/systems/character-catalog.md`](../../docs/systems/character-catalog.md)
**Final state:** 100% Hall sprite coverage (97/97), 0 sandbox.ldtk validator warnings (down from 185), 3 intentional intro.ldtk warnings.

## What landed

Eight planned phases plus fourteen bonus follow-up sprints in
response to mid-run QA from Jon. Final commit count: 44. Final line
count delta: many thousands of additions (most are auto-generated
sprite manifests and the regenerated Hall LDtk level).

### Headline results

- **97/97 Hall pedestals render** real sprites (was 24/99 at the
  start of the run, before any catalog work).
- **0 sandbox.ldtk validator warnings** (was 185, mostly false
  positives from the validator not recognizing IntGrid Collision
  cells as walls; cleaned up by extending the validator + filling
  one genuinely-missing floor row in goblin_encounter via the new
  `intgrid paint` tool).
- **3 intro.ldtk warnings** remain — all are genuine spatial-
  authoring decisions that need human judgment (pirate_sky_arena
  top edge, two mid-air doors in drain_alley / under_town_pipes).
- **779 sandbox lib tests + 265 engine tests + 22 Python ldtk_tools
  tests pass.** Added ~25 tests this run.
- **42 commits**, all green.

Single sentence: every spawnable character in the sandbox now lives
in one RON file (`character_catalog.ron`), with a Bevy plugin that
loads it at startup, validates internal references, and ships a
multi-level Hall of Characters room that visually proves every
catalog entry is wired in.

## Lessons learned

Listed in rough order of "took the longest to figure out" → "noticed
in passing":

### 1. pyron drops Rust enum discriminators on unit variants

The upstream `python-ron` package (which wraps the Rust `ron` crate)
parses the catalog's `tier: MainHall` as `{}` — the discriminator
name is gone. Stuct variants (`Patrol(spawn_local_x: ...)`) come back
as just the payload dict, also without the variant name.

**Workaround:** regex over the raw RON text to extract the variant
name when it matters. Used in `generate_hall_of_characters.py` to
partition entries by tier.

**Long-term:** if the Python side needs to round-trip RON cleanly
(serialize back), we either author specifically-string-typed enums
in Rust or write our own RON parser. For now the regex is fine —
the data is read-only on the Python side.

### 2. Boss subdirs need explicit recursive scanning

`record_index()` originally scanned `assets/sprites/*_spritesheet.ron`
flat. `gnu_ton_boss` and `mockingbird_boss` publish into their own
subdirs because their generated PNGs are huge (768×576 and 576×216
respectively). Without an explicit `read_dir` walk into the subdirs,
the manifest-driven sprite-spec loader never sees them — the catalog
points at a file that's "logically there" but the in-memory index
doesn't know about it.

**Fix:** one-level subdir scan in `record_index()`. Documented in
the system doc so the convention is explicit.

### 3. "Idle row" is a runtime-fatal invariant for the actor renderer

`flat_index` (sheets.rs) falls back to `Idle` for any animation that
doesn't have its own row. Without an Idle row, the spec has no
fallback, and the renderer panics on the first frame of the first
NPC. This crash is what Jon hit walking into the Hall mid-build.

The runtime parser via `CharacterAnim::from_name` is selective about
which row names map to which enum variants. A sheet that ships only
`run` / `attack` / `death` (e.g. galwah's `walk`/`duel`/`theorem`/
`death`/`turn`) deserializes a spec with zero rows that match Idle.

**Fix:** validate Idle presence in `try_load_spec_for_character_id`
and return None when it's missing. Caller falls back to the colored
rectangle. Also widened `CharacterAnim::from_name`'s Idle aliases
(`rest`, `front_idle`, `side_idle`, `side_walk` → Walk) to cover
the most common idle-equivalent generator outputs.

**Bigger lesson:** the renderer→runtime contract here is implicit.
A generator that omits `idle` produces a sheet the runtime can't
animate. A linter that warns "this manifest will not produce a
working sprite" at render time would prevent the issue from
re-occurring.

### 4. The three renderer publish patterns are a leaky abstraction

  - Python tack-on targets (`targets/characters/<name>.py`) —
    canonical, well-understood.
  - YAML-adapter rigs (`configs/*.yaml`) — drives review NPCs.
  - Bespoke one-offs (mockingbird boss generator, robot_heavy
    variant-only publisher, weird_hermit non-conventional names,
    pirate_heavy variant-only).

A catalog-driven publish driver (`publish_catalog_sprites.py`) was
the first artifact to surface the divergence. Of 97 catalog entries,
8 failed `publish <target>` with various reasons:
  - 5 are `pirate_heavy_*` variant publishes (the variants ship; just
    the bare name doesn't).
  - 2 are faction-leader configs that the catalog-driver invokes by
    bare name but the renderer expects via `draw-factions`.
  - 1 is `weird_hermit` (filename + schema both diverge).

Documented in [`docs/systems/sprite-rendering-surface.md`](../../docs/systems/sprite-rendering-surface.md)
with four ranked follow-ups.

### 5. Estimates pad for risks that don't materialize

Plan estimated 8.5h for the 7 main phases; actual was 0.94h. Reasons:
  - Brain and ActionSet vocabulary already designed for the catalog
    shape (from the universal-brain run a week earlier).
  - LDtk runtime spine already in place; entity parsing was a
    one-field change, not a parser rewrite.
  - The renderer already had `list-targets` and stable target
    naming, so the codegen was a script over existing data.

The estimates assumed a worst case (unexpected coupling, EMFILE
retries, validation-gate failures). None of those materialized.
**Calibration:** estimates for tightly-scoped refactors that build
on existing foundations can be 5-10x lower than estimated. Big
unknowns (the SheetRegistry-driven sprite-spec follow-up) still
take real time.

### 6. The Hall is a stress-test surface; visual feedback is dense

Walking the Hall after a refactor catches issues the unit tests
miss: render-size aspect-ratio overlaps (bear_mauler on top of
smart_house), missing platforms under basement bosses, character_id
typos that compile cleanly but display the wrong sprite.

**Worth investing in:** an LDtk validator pass for spawn overlap
landed in this sprint. A linter that compares NpcSpawn's rendered
size (derived from manifest + tuning) against the slot it sits in
would catch the next class of issue.

## Process notes

  - The progress table in the TODO doc was useful for self-pacing
    and for Jon's mid-run check-ins. The "Estimated vs Actual"
    column made it obvious which phases were faster than expected
    (all of them — see lesson 5).
  - Jon's mid-run interjections caught issues I would have missed
    on my own: the npc_pirate_heavy / npc_robot_heavy rollups, the
    boss placeholder regression after Phase 6, the missing
    basement platforms. The conversation latency was fine — a 5-10
    minute round-trip with explicit findings.

## Cross-refs

- [`TODO-character-catalog-and-hall.md`](../../TODO-character-catalog-and-hall.md) — the run's plan + progress table.
- [`docs/adr/0017-rust-behavior-ron-content-ldtk-space.md`](../../docs/adr/0017-rust-behavior-ron-content-ldtk-space.md) — codified architectural posture.
- [`docs/systems/character-catalog.md`](../../docs/systems/character-catalog.md) — live system overview.
- [`docs/systems/sprite-rendering-surface.md`](../../docs/systems/sprite-rendering-surface.md) — renderer-side divergence + cleanup plan.
- [`docs/recipes/adding-a-character.md`](../../docs/recipes/adding-a-character.md) — author-facing how-to.
