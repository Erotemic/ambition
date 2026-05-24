# Sprite rendering surface

> Where do character spritesheets come from, why are there three
> different ways to publish one, and what does "unify the renderer"
> look like as a follow-up?

## The three patterns today

The Ambition sprite renderer (`tools/ambition_sprite2d_renderer/`)
produces every character spritesheet the sandbox ships. It currently
has three publish patterns:

### 1. Python tack-on targets

Live under `tools/.../ambition_sprite2d_renderer/targets/characters/<name>.py`.
Each module exports either a `TARGETS = {...}` dict (multi-variant)
or a single `render()` function. The renderer's CLI subcommand
`publish <target> --dest-root <dir>` walks the module's exports,
renders to `generated/<target>/`, and copies the runtime files
(`<target>_spritesheet.png`, `.yaml`, `.ron`) into `<dir>`.

Examples: `sandbag`, `creator`, `interdimensional_gate`,
`agent_swarm`, every AI-era enemy (`ai_slop`, `hand_saint`,
`helpful_liar`, `spaghetti_event`, `synthetic_friend`,
`puppy_slug_variant2`), `dark_lord`, `mantis_lancer`, …

This is the **canonical pattern**: every new character should land
as a tack-on target so `regen_sprites.sh` and
`publish_catalog_sprites.py` can drive it.

### 2. YAML-adapter rigs

Live under `tools/.../configs/*.yaml` and `tools/.../configs/review/*.yaml`.
Each YAML describes a "rig + palette" reusing one of the adapter
Python modules (`adapters.py`, `targets/characters/toon_side.py`,
`targets/characters/robot_side.py`, `targets/characters/goblin_side.py`,
`targets/characters/ninja_side.py`). The renderer's
`draw-all` + `draw-review` subcommands batch-render every YAML in
the directory into a scratch dir, and `regen_sprites.sh` then copies
the named ones into the runtime sprites dir.

Examples: `architect`, `kernel_guide`, `alice`, `bob`, the twelve
"crypto crew" review NPCs (`craig`, `eve`, `judy`, `mallory`,
`peggy`, `sybil`, `trent`, `trudy`, `victor`, `walter`, `olivia`),
the goblin variants (`goblin_brute_hammer`, `goblin_cave_dagger`,
`goblin_desert_bow`, `goblin_frost_sword`, `goblin_shaman_staff`),
the robot specialists (`robot_archivist`, `robot_caster`, …).

As of Phase 6.C of the character-catalog refactor (2026-05-24), both
adapter rigs and tack-ons surface under the single `[characters]`
category in `list-targets`. The `configs/review/` directory itself
stays — it's an authoring-organization detail, not a runtime
distinction.

### 3. Bespoke one-offs

A few targets have entirely custom publishers that don't fit either
pattern:

- **`gnu_ton_boss`** — publishes into `assets/sprites/gnu_ton_boss/`
  as a subdir with multiple variants (`_body`, `_hands`,
  `_spritesheet`). Driven by the `publish` subcommand but produces
  a multi-file output unlike standard tack-ons.
- **`mockingbird_boss`** — has its own standalone script
  (`tools/.../mockingbird_boss_sprite_generator.py`) with its own
  argparse + install logic, invoked directly from `regen_sprites.sh`.
- **`pirate_heavy`** — renders to `pirate_heavy_<variant>_spritesheet.{png,yaml,ron}`
  for each of three variants (`broadside_bess`, `iron_mary`,
  `salt_annet`). The "base" `pirate_heavy_spritesheet.png` never
  ships, so the catalog's `npc_pirate_heavy` entry falls back to
  the colored-rectangle visual.
- **`robot_heavy`** — renders multiple variants
  (`robot_heavy_bastion`, `robot_heavy_arsenal`, …) but doesn't
  install them. `regen_sprites.sh` skips this target with `[skip]`.
- ~~**`weird_hermit`**~~ — *fixed 2026-05-24*. Publisher now emits
  `weird_hermit_spritesheet.{png,ron,yaml}` with the canonical
  `SheetRow` schema. Catalog entry resolves.

## Why three patterns?

History. Each pattern landed when a particular content type needed
it: the YAML-adapter pipeline came first for review NPCs, then
tack-ons for procedural enemies, then bespoke for the boss
encounter sheets that needed unusual row layouts. The boundaries
calcified.

## Runtime view (catalog-driven)

After Phase 6 of the character-catalog refactor, the runtime sprite
loader is uniform regardless of which publish pattern produced the
sheet:

1. `load_character_sprites_in` (in
   `crates/ambition_sandbox/src/presentation/character_sprites/assets.rs`)
   iterates the embedded `character_catalog.ron`.
2. For each entry, `sheet_for_character_id(cid)` resolves a
   `CharacterSheetSpec`:
   - Hardcoded `*_SHEET` consts for characters that need bespoke
     `collision_scale` / `feet_anchor_y` tuning.
   - Otherwise, the manifest-driven fallback
     (`try_load_spec_for_character_id`) loads the spec from the
     on-disk `<target>_spritesheet.ron` via `record_index`. The
     index scans `assets/sprites/*.ron` plus one level into
     subdirs (`gnu_ton_boss/`, `mockingbird_boss/`).
3. If no spec resolves, the character falls back to the colored-
   rectangle placeholder. The Hall of Characters pedestal stays
   visually empty but doesn't crash.

The runtime doesn't care which publish pattern produced the sheet —
it just consumes the manifest. So the unification problem is
authoring-side: how to drive every target through one canonical
publish flow.

## Unification plan (follow-up work)

The goal: one `Target` trait that every renderer character module
implements, with three methods:

```python
class CharacterTarget:
    name: str
    def render(self, out_dir: Path) -> list[Path]: ...
    def install(self, dest_root: Path) -> list[Path]: ...
    def runtime_filenames(self, dest_root: Path) -> list[Path]: ...
```

`publish_catalog_sprites.py` (added 2026-05-24) is the first step:
a single driver that reads the catalog, derives the renderer target
name from each entry's `spritesheet:` path, and runs
`publish <target>` for each. It logs per-target status and
identifies the stragglers.

Remaining work, in order of value:

1. **Convert `pirate_heavy` and `robot_heavy` to variant-publishing
   targets that ship a flat `<name>_spritesheet.{png,yaml,ron}` for
   each variant.** Today only `pirate_heavy_<variant>_*` files
   ship; nothing under the bare `pirate_heavy` name. The catalog
   has variant-specific entries (npc_pirate_heavy_broadside_bess
   etc.) so what's needed is either to drop the bare-name catalog
   entries or to have the publisher emit a "main" sheet (probably
   the first variant).
2. ~~**Give `weird_hermit` a publisher that installs
   `weird_hermit_spritesheet.png`.**~~ *Landed 2026-05-24.*
   Publisher now emits the canonical `<target>_spritesheet.{png,ron,yaml}`
   filenames AND the runtime's `SheetRow` schema
   (`animation`/`row_index`/`frame_count`/`duration_ms`/etc.).
3. **Lift `mockingbird_boss_sprite_generator.py` into a regular
   tack-on target.** The standalone script predates the unified
   `targets/<name>.py` API; carrying it forward means
   `regen_sprites.sh` needs a special-case invocation that
   `publish_catalog_sprites.py` can't drive.
4. ~~**Document the per-target row layout requirements.** The
   runtime's `CharacterAnim::from_name` expects `idle` / `walk` /
   etc.; sheets that ship only `run` / `attack` / `death` (no
   idle) fall back gracefully now (see
   `try_load_spec_for_character_id`) but render as colored
   rectangles. A linter that warns "this manifest will not produce
   a working sprite" would catch the issue at render time.~~
   *Landed 2026-05-24.* `tackon_sheet.diagnose_idle_coverage`
   prints a stderr warning at publish time when a sheet has
   ≥1 `CharacterAnim` row but no Idle alias. Pinned by
   `test_diagnose_idle_coverage.py` (7 tests, including the
   galwah-pre-rename regression marker).

Each of these is small (1–3 hour). Doing all four would let
`publish_catalog_sprites.py` cover 100% of catalog entries.

## Today's contract

Until the unification lands:

- **New characters** ship as Python tack-ons under
  `targets/characters/<name>.py` (the canonical pattern).
- **Naming convention** is `<target>_spritesheet.{png,yaml,ron}` —
  the catalog's `spritesheet:` field expects this exact form,
  and `record_index()` indexes by `<target>` (the filename root).
- **Subdir publishing** (gnu_ton_boss, mockingbird_boss) is
  tolerated because those sheets are large enough to warrant their
  own folder, but the catalog path must include the subdir prefix:
  `sprites/<target>/<target>_spritesheet.png`.
- **Bespoke publishers** (weird_hermit, robot_heavy) are known
  divergences; their catalog entries render as placeholders.

## Cross-references

- [`character-catalog.md`](character-catalog.md) — the runtime
  consumer of every spritesheet.
- [`docs/recipes/adding-a-character.md`](../recipes/adding-a-character.md)
  — the author-facing how-to that covers the canonical path.
- `tools/ambition_ldtk_tools/ambition_ldtk_tools/publish_catalog_sprites.py`
  — the catalog-driven publish driver introduced 2026-05-24.
