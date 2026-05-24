# ADR 0017: Rust is for behavior, RON is for content, LDtk is for space

## Status

Accepted (2026-05-24). Codifies the architectural posture that the
character-catalog refactor implemented across Phases 1–6 of
[`TODO-character-catalog-and-hall.md`](../../TODO-character-catalog-and-hall.md).

## Decision

Each layer of the project owns a single class of concern:

| Layer | Owns | Examples |
|---|---|---|
| **Rust (engine + Bevy plugins)** | Algorithms, ECS plumbing, physics, brain *variants*, validators, Bevy plugins reusable across Ambition-powered games | `brain::tick_melee_brute`, `step_kinematic`, `CharacterCatalogPlugin`, `BrainDriverPlugin` |
| **RON (data)** | Character catalog, brain CFGs, ActionSet specs, boss encounter scripts, dialogue trees, quest defs, tuning, area specs | `character_catalog.ron`, `boss_encounters/<id>.ron`, `hall_of_characters_area.ron` |
| **LDtk (space)** | Level geometry, entity placement, world composition | `sandbox.ldtk`, `intro.ldtk` — NpcSpawns point at catalog ids |

The rule for "where does this piece of state live?" is whichever
layer it answers to:

- **A new brain template** (e.g. `Scavenger` that prioritizes pickups
  over players) lives in Rust. It's behavior — the algorithm is
  what's new, and the compiler enforces exhaustiveness across the
  brain enum.
- **A new character that uses an existing brain** (e.g. a melee enemy
  with a unique sprite) is a single new row in
  `character_catalog.ron`. It's content — no Rust changes.
- **A new room that puts characters in a layout** is a new level in
  the LDtk file. It's space — no Rust or RON changes.

## Context

Before this ADR the project had a multi-source-of-truth problem
for characters: an `NPC_SPRITE_REGISTRY` table in
`presentation/character_sprites/assets.rs` declared display names
and sprite-spec consts; an LDtk `NpcSpawn.name` field stamped the
runtime label; spawn-time code mapped names to brain archetypes
via a separate `enemy_default_brain` function; and ad-hoc YAML
adapter configs declared yet more characters in
`tools/ambition_sprite2d_renderer/configs/review/*.yaml`. Adding
"Pirate Quartermaster" took rows in three or four tables, in
several languages and config formats, with no compile-time pin
forcing them to stay in sync.

The character-catalog refactor unified all of that under one RON
file (`assets/data/character_catalog.ron`) whose entries are the
sole source of `(display_name, sprite path, default brain preset,
default action-set preset, tier, body kind, tags)` per character.
The Hall of Characters room is the first auto-generated artifact
that consumes the catalog end-to-end — every catalog entry gets a
pedestal + label, computed mechanically from the catalog by
`tools/ambition_ldtk_tools/.../generate_hall_of_characters.py`.

Along the way we surfaced (and committed to) the same posture for
area specs (YAML → RON, Phase 4) and for the relationship between
the LDtk world and runtime gameplay (NpcSpawn carries the
character_id key, not a display name, Phase 2).

## Consequences

**Reusable engine.** A future Ambition-powered game can drop in its
own `character_catalog.ron` and get NPCs working with zero Rust
changes. The same is true for `CharacterCatalogPlugin` consumers —
the only thing the game declares is data.

**One config format.** Project policy: Rust-consumed config is RON
(via `serde` derives + `bevy_common_assets::ron::RonAssetPlugin`).
Python tooling that needs to read the same configs uses the upstream
`python-ron` package (wraps the Rust `ron` crate) so the Python
side parses exactly what Rust does. No YAML, no JSON5, no TOML — the
exception being external schemas already defined by upstream tools
(LDtk is JSON; that stays JSON).

**Compile-time pin on internal consistency.** The catalog validator
runs as a Startup system that panics on any internal reference
error (a character entry pointing at a missing brain preset). A
parallel test pins every renderer-registered character to a catalog
entry. Drift between the catalog and the renderer is loud, not
silent.

**Brain variants stay typed in Rust.** Per the catalog schema, a
brain preset is `BrainPreset::MeleeBrute { aggressiveness: 1.0,
aggro_radius: 220.0, ... }` — variant in Rust, cfg fields in RON.
The compiler still tells you about a missing brain handler.
Adding "a brain that prioritizes ranged attacks against shielded
players" is a Rust patch (new variant + new tick fn). Adding "a
character that uses the existing Skirmisher brain with a longer
fire cooldown" is a one-line RON edit.

**ECS components stay one-purpose.** This ADR doesn't undermine
ADR 0016 (actor unification). The catalog is the *content
manifest* for what to spawn; ADR 0016 governs *how* the spawned
entity is shaped in ECS. They compose: spawn-from-catalog walks
the entry, picks the brain preset + action set preset, and calls
the same `spawn_actor` path that handcrafted spawns use.

## Implementation status (2026-05-24)

Landed:
- Character catalog (`assets/data/character_catalog.ron`, 99 entries).
- `CharacterCatalogPlugin` with Startup validator
  ([`crate::content::character_catalog`]).
- LDtk `NpcSpawn` schema: `character_id` field replaces the legacy
  `name` field across both worlds.
- Hall of Characters room — auto-generated, one pedestal per
  catalog entry.
- Area-spec format: 28 YAML specs migrated to RON.
- Renderer `review_npcs` category merged into `characters`.
- `NPC_SPRITE_REGISTRY` + `npc_sprite_label` deleted; sprite loader
  iterates the catalog via `sheet_for_character_id`.

Deferred for follow-up work:
- `boss_encounters/<id>.ron` per-boss phase schedules — **numeric-
  fields half landed 2026-05-24**: all three authored bosses
  (`gnu_ton`, `mockingbird`, `clockwork_warden`) ship an `<id>.ron`
  that overrides their hardcoded `BossEncounterSpec` constructor
  via `default_boss_profiles`. Remaining: move the per-phase brain
  schedules (a richer schema item) once `BossPattern` has hooks
  for them.
- `SheetRegistry`-driven sprite specs — once the per-character
  `CharacterSheetSpec` consts come from the manifest at startup,
  `sheet_for_character_id` disappears entirely.
- Per-instance brain override semantics — `NpcSpawn.brain_override`
  / `brain_overrides` fields (planned in the original RFC but no
  use case has landed yet).

## Current implications for agents

- **New character or boss** → author RON first, write Rust only when
  the asset needs a bespoke `*_SHEET` tuning or behavior hook. The
  character catalog (`assets/data/character_catalog.ron`) covers the
  common case with zero Rust changes; boss-encounter numeric fields
  (HP, phase thresholds, timings, music ids) live in
  `assets/data/boss_encounters/<id>.ron` and override the hardcoded
  constructor when a matching profile exists.
- **New room or arena** → LDtk file, never Rust. Use the
  `ambition_ldtk_tools` subcommands (`area create`, `intgrid paint`,
  `space_debug_labels`) — never hand-edit the LDtk JSON
  (see [[feedback-ldtk-tools-only]] memory).
- **Tuning a number** (HP, speed, timing, threshold) → check first
  whether it lives in a `.ron` file under `crates/ambition_sandbox/
  assets/data/` (catalog, boss encounters, area specs). If it's in
  RON, edit the RON; don't add a Rust override. The Rust constructor
  is a compile-time fallback for fresh clones, not the authoritative
  source.
- **Hardcoded fallback discipline** — the constructor pattern
  (`BossEncounterSpec::gnu_ton()`) stays around as a fresh-clone
  fallback. RON overrides it when present. Never delete the
  constructor; never delete the RON.
- **Validator hierarchy** — both Python (schema pin) and Rust
  (field-equivalence pin) layers exist. When adding new RON content,
  add a Python schema test first (fires without compile) and a Rust
  pin test second (catches drift against the hardcoded fallback).

## Cross-references

- [`TODO-character-catalog-and-hall.md`](../../TODO-character-catalog-and-hall.md)
  — the overnight run that landed the architecture.
- [`docs/systems/character-catalog.md`](../systems/character-catalog.md)
  — the live system doc.
- [`docs/recipes/adding-a-character.md`](../recipes/adding-a-character.md)
  — author-facing how-to.
- ADR 0003 — data-driven specs and asset loading.
- ADR 0009 — LDtk world composition.
- ADR 0016 — actor unification (this ADR's complement on the ECS side).
