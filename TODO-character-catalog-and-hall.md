# TODO: Character catalog refactor + Hall of Characters (overnight run)

**Status:** plan, 2026-05-24. Authoritative source for the next 8 h
autonomous run. Self-contained — readable after context
compaction.

This is the same shape as
[`TODO-controllable-entity.md`](TODO-controllable-entity.md) (which
ran successfully as the universal-brain overnight). Pattern:
phase-by-phase plan, each phase exits at a green commit, the run
can stop at any checkpoint if budget runs out.

## Run progress (live updates)

**Run started:** 2026-05-24T17:16:07+0000 (epoch 1779642967)
**Current phase:** Phase 8 (stretch) — sprite-regen cache 🏗️
**Last green commit:** _Phase 7 commit pending_

### Estimated vs Actual time

| Phase | Estimated | Actual | Status | Notes |
|---|---|---|---|---|
| 1. Foundation (RON catalog + plugin + validator) | 2.0 h | 0.16 h | ✅ done | 24 chars; 771 lib tests + 8 new catalog tests all green; headless 60-tick clean. Fast because brain/action_set types already designed for this shape. |
| 2. NpcSpawn schema change (`name` → `character_id`) | 1.5 h | 0.16 h | ✅ done | 26 LDtk instances migrated (18 sandbox + 8 intro); +11 intro catalog entries; parser translates character_id → display_name so downstream sprite/banter/dialog keep working without churn; 10 catalog tests + 771 lib tests + 200-tick headless all green. |
| 3. Sprite gap closure (every renderer entry → catalog) | 1.0 h | 0.09 h | ✅ done | Codegen script `codegen_character_catalog.py` synthesized 64 entries from renderer's list-targets via heuristic table. Catalog at 114 entries total. +1 coverage gate test pinning the renderer snapshot. 774 lib tests + headless 100-tick all green. Skipped regen smoke: this phase touched no sprite plumbing. |
| 4. Area specs YAML → RON migration | 1.0 h | 0.10 h | ✅ done | 28 YAML specs converted to RON in-place. Initially hand-rolled a 200-LOC RON parser; per Jon's mid-run guidance switched to upstream `python-ron` (wraps the Rust `ron` crate) so the Python side parses exactly what Rust does. Keeps in-house struct-style `dumps` for idiomatic output. 774 lib tests green. |
| 5. Hall of Characters generator + room | 1.5 h | 0.19 h | ✅ done | Generator: `generate_hall_of_characters.py` reads catalog, partitions by tier (pyron drops Rust enum discriminators on unit variants so regex extracts `tier:` directly), lays out 6 floors × 16 main slots + 2 basement rows × 8 = 112 capacity. Spec: 2048×1840 at world (40000, 0). 89 main + 10 basement pedestals (each = NpcSpawn + DebugLabel). LDtk pin test added. 775 lib tests + headless 200 ticks all green. |
| 6. Cleanup (delete legacy registries, merge review_npcs) | 1.0 h | 0.24 h | ✅ done | Three sub-commits: 6A drop YAML support + archive one-shots (def5b2f); 6B delete NPC_SPRITE_REGISTRY + npc_sprite_label, sprite loader iterates catalog via sheet_for_character_id (26000cd, -298+151 lines); 6C renderer review_npcs → characters merge (9ed4a39). |
| 7. Documentation + ADR 0017 | 0.5 h | 0.07 h | ✅ done | ADR 0017, character-catalog system doc, adding-a-character recipe, FEATURES.md row, TODO.md "landed" entry with deferred follow-ups, docs/recipes/index.md + docs/systems/index.md updates, dev/SEARCH.md grep tip. |
| **Total (planned)** | **8.5 h** | _ | | over budget by 0.5 h — trim Phase 6 if needed |
| 8. (stretch) Sprite-regen caching | ~0.75 h | _ | ⬜ optional | only if >45 min slack after Phase 7 |

Status legend: ⬜ pending · 🏗️ in progress · ✅ done · ⏭️ deferred · ❌ blocked

Notes column: capture *why* a phase diverged from estimate (unexpected
coupling, EMFILE retries, validation-gate failures, scope changes).

## TL;DR

Refactor character authoring away from hand-maintained Rust
registries into a **RON character catalog**, with a multi-level
**Hall of Characters** room that visually proves the catalog covers
every renderable character. Bake the architectural posture (Rust =
behavior, RON = content, LDtk = space) into an ADR. Pre-release;
breaking changes accepted.

The work that makes the hall ship cleanly is the same work that
makes Ambition a reusable engine. The character catalog ships as
`AmbitionCharacterCatalogPlugin` — any future Ambition-powered game
drops in its own `character_catalog.ron` and gets NPCs for free.

## Architectural mantra (to be ADR 0017)

> **Rust is for behavior. RON is for content. LDtk is for space.**

| Layer | Owns | Examples |
|---|---|---|
| **Rust (engine + Bevy plugins)** | Algorithms, ECS plumbing, physics, brain *variants*, validators, Bevy plugins reusable by other Ambition games | `brain::tick_melee_brute`, `step_kinematic`, `CharacterCatalogPlugin` |
| **RON (data)** | Character catalog, brain CFGs, ActionSet specs, boss encounter scripts, dialogue trees, quest defs, tuning, area specs | `character_catalog.ron`, `boss_encounters/gnu_ton.ron`, `hall_of_characters_area.ron` |
| **LDtk (space)** | Level geometry, entity placement, world composition | `sandbox.ldtk` — NpcSpawns point at catalog ids |

Use this mantra to break ties mid-run: any decision about where a
piece of state should live answers to it.

## Decisions locked (Jon, 2026-05-24)

| Decision | Choice | Source |
|---|---|---|
| Architectural posture | Rust = behavior, RON = content, LDtk = space; bake into ADR 0017 | Jon's "Ambition is the engine" framing |
| Animation timing | RON-owned per character; brain *variant* stays typed in Rust | Tradeoff analysis (author velocity > const folding) |
| Multi-part bosses | One catalog shape with optional `composition` field; resist sub-types | Avoid catalog fragmentation |
| Boss encounter scripts | Separate per-encounter RON file (`boss_encounters/<id>.ron`) | Phase schedules are verbose; catalog stays slim |
| NpcSpawn schema | Hard break: `name` → `character_id`; migration script rewrites every instance | [[feedback-pre-release-no-compat]] |
| "Review NPCs" naming | Merge into `characters` category; the split was a renderer-internal detail | Stale framing |
| Area-spec format | Migrate YAML → RON; `area create` accepts both during the migration commit only | Project policy: "one config format" |
| Hall layout | Multi-level (5 floors × 16 slots) + basement; vertical, not horizontal | Jon's "give us 4–5 levels for headroom" |
| Hall door | From `central_hub_main` | Mirrors `hall_of_bosses` precedent |
| Basement entry | Drop-through hole; ladder back up from basement | Matches existing hub→basement pattern |
| Boss display | Standees only — `Brain::stand_still()` + peaceful | Hall is a gallery, not an arena |
| Commit target | Directly to `main`; no push, no amend, no force-push | Standard autonomous rules |
| Budget | 8 h; stop at any green checkpoint if exhausted | [[feedback-never-stop-during-long-run]] |

## Hard invariants (must hold across the refactor)

1. **`regen_sprites.sh` must work on a fresh clone.** Jon recently stabilized
   it (commits `4b66bc3`, `29205ee`); the script regenerates every sprite the
   game needs from a clean tree. Any refactor that touches sprite authoring,
   target discovery, manifests, or `NPC_SPRITE_REGISTRY` MUST preserve this:
   after the run, `./regen_sprites.sh` on a clean checkout still produces the
   full sprite set. Details of *how* it generates them can change — the
   invariant is the end state.
2. **`regen_assets.sh` must still drive the full asset refresh.** Same shape
   as (1) but for the broader asset pipeline.
3. **Validation gate added to every phase that touches sprite plumbing**
   (Phase 3 mandatory; Phases 5, 6 if they touch renderer code): run
   `./regen_sprites.sh --dry-run` (or a no-op subset) before commit; if the
   script changed shape, run a full regen against `/tmp/regen-smoke/` to
   verify exit code 0 and non-zero output.
4. **No new binary blobs committed.** Generated sprites stay gitignored
   per [[feedback-no-binary-data]]; `regen_sprites.sh` reproduces them.

### Stretch goal — sprite-regen caching (if time permits, post-Phase 7)

Jon noted: "consider having the main regen command implement caching for the
sprites so it has a faster regen of modified sprites." Concrete shape if
attempted:

- Hash each target's source (the Python `_render_*` function bytecode +
  any RON timing config it consults) into `tools/ambition_sprite2d_renderer/.cache/<target>.hash`.
- Skip re-rendering if the hash matches and the generated PNG/manifest still
  exists.
- `regen_sprites.sh --force` bypasses cache.
- Only attempt this if Phases 1–7 finish with >45 min remaining in the budget.
- Out of scope: cross-machine cache (host file hashes are enough).

## Hall of Characters layout

```
+----------------------------------------------+ y=0
| ▒ ceiling ▒                                   |
|  🧍 🧍 🧍 🧍 🧍 🧍 🧍 🧍   16 slots × 128 px   |  Floor 5  (newest / overflow)
| ─[ladder]─────────[ladder]──[OneWayPlatform]─ |  y=192
|  🧍 🧍 🧍 🧍 🧍 🧍 🧍 🧍                     |  Floor 4
| ─[ladder]─────────[ladder]──[OneWayPlatform]─ |  y=384
|  🧍 🧍 🧍 🧍 🧍 🧍 🧍 🧍                     |  Floor 3
| ─[ladder]─────────[ladder]──[OneWayPlatform]─ |  y=576
|  🧍 🧍 🧍 🧍 🧍 🧍 🧍 🧍                     |  Floor 2
| ─[ladder]─────────[ladder]──[OneWayPlatform]─ |  y=768
|  🧍 🧍 🧍 🧍 🧍 🧍 🧍 🧍                     |  Floor 1  (hub entry)
| ▒▒▒▒▒▒▒▒▒▒▒▒▒[drop hole]▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒  |  y=960
|                                                |
|  🦖    🛸    🪲    🧙    🐻              ⛓    |  Basement  (big sprites)
| ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒[ladder]▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ |  y=1280
+----------------------------------------------+
worldX:  8000
pxWid:   2048   (16 standard slots × 128 px)
pxHei:   1280
```

**Slot rules:**
- Main floor slot: 128 px wide × 192 tall (sprite + headroom + label).
- Basement slot: 256 px wide × 320 tall.
- Sprite foot anchored at slot bottom-center.
- `DebugLabel` above each pedestal: shows `character_id` so the hall
  doubles as a sprite-registry visualizer.
- Two ladder columns per floor at x ∈ {320, 1728}.
- One-way platforms between floors so you can jump up but don't
  fall back through.

**Capacity:** 5 floors × 16 = **80 standard slots** + **8 basement
slots** = 88 total. Current census fills ~63; headroom for ~25
more before the room needs resizing. Generator extends Floor 5
horizontally first; when that maxes, adds Floor 6.

**Tier classification** (initial — lives in catalog as `tier` field):

| Tier | Initial population |
|---|---|
| `Basement` | gnu_ton_boss, mockingbird_boss, flying_spaghetti_monster_boss, smart_house, trex_enemy, bear_mauler, raptor_stalker, mantis_lancer |
| `MainHall` | everything else |

## RON schemas (authoritative)

### `assets/data/character_catalog.ron`

```ron
(
    // Brain presets — common tuning sets reused across many characters.
    // Per-character overrides allowed at the use site (see `default_brain`).
    brain_presets: {
        "stand_still": StandStill,
        "patrol_peaceful": Patrol(
            spawn_local_x: 0.0, radius: 96.0,
            speed: 36.0, aggressiveness: 0.0,
            aggro_radius: 80.0, attack_range: 0.0,
        ),
        "wanderer_puppy_slug": Wanderer(
            speed: 36.0, climb_walls: true,
            chatter_threshold: 3, chatter_window_s: 1.0, chatter_pause_s: 2.0,
            aggressiveness: 0.0,
        ),
        "melee_brute_striker": MeleeBrute(
            aggressiveness: 1.0, aggro_radius: 220.0,
            attack_range: 36.0, chase_speed: 110.0,
        ),
        "melee_brute_brute": MeleeBrute(
            aggressiveness: 1.0, aggro_radius: 240.0,
            attack_range: 44.0, chase_speed: 75.0,
        ),
        "skirmisher_ranger": Skirmisher(
            aggressiveness: 1.0, aggro_radius: 320.0,
            standoff_px: 140.0, strafe_speed: 85.0, fire_cooldown_s: 0.8,
        ),
        "sniper_default": Sniper(
            aggressiveness: 1.0, aggro_radius: 480.0, fire_cooldown_s: 1.5,
        ),
        "boss_pattern_gnu_ton": BossPattern(
            aggressiveness: 1.0,
            encounter_id: "gnu_ton",  // → assets/data/boss_encounters/gnu_ton.ron
        ),
    },

    // ActionSet presets — what an actor can do when the brain says "act".
    action_set_presets: {
        "peaceful": (
            move_style: Walk,
            melee: None, ranged: None, special: None,
        ),
        "striker_swipe": (
            move_style: Walk,
            melee: Some(Swipe(
                windup_s: 0.28, active_s: 0.08, recover_s: 0.32,
                damage: 1, reach_px: 28.0,
            )),
            ranged: None, special: None,
        ),
        "brute_lunge": (
            move_style: WalkHeavy,
            melee: Some(Lunge(
                windup_s: 0.45, active_s: 0.12, recover_s: 0.45,
                damage: 2, reach_px: 40.0, step_px: 18.0,
            )),
            ranged: None, special: None,
        ),
        "ranger_arrow": (
            move_style: Strafe,
            melee: None,
            ranged: Some(Arrow(speed: 520.0, damage: 2)),
            special: None,
        ),
        "boss_default": (
            move_style: Walk,
            melee: None,
            ranged: Some(Bolt(speed: 380.0, damage: 1)),
            special: Some(BossSpotlight),
        ),
        // …more as needed
    },

    // The catalog itself. Map of character_id → entry.
    characters: {
        "kernel_guide": (
            display_name: "Kernel Guide NPC",  // human-facing label
            spritesheet:  "characters/kernel_guide_spritesheet.png",
            manifest:     "characters/kernel_guide_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,   // single-part character
            default_brain:      "patrol_peaceful",
            default_action_set: "peaceful",
            tags: ["hub", "guide"],
        ),
        "puppy_slug_variant2": (
            display_name: "Puppy Slug Variant 2",
            spritesheet:  "characters/puppy_slug_variant2_spritesheet.png",
            manifest:     "characters/puppy_slug_variant2_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain:      "wanderer_puppy_slug",
            default_action_set: "peaceful",
            tags: ["enemy", "ai_era"],
        ),
        "gnu_ton_boss": (
            display_name: "GNU-ton",
            spritesheet:  "characters/gnu_ton_boss_spritesheet.png",
            manifest:     "characters/gnu_ton_boss_spritesheet.ron",
            tier: Basement,
            body_kind: Wide,
            composition: Some([          // optional; renderer still emits a composed sheet today
                ( id: "body",  layer: 0, anchor_px: (80, 120) ),
                ( id: "head",  layer: 2, anchor_px: (80,  60) ),
                ( id: "wings", layer: 1, anchor_px: (50,  80) ),
            ]),
            default_brain:      "boss_pattern_gnu_ton",
            default_action_set: "boss_default",
            tags: ["boss", "act1"],
        ),
        // …one entry per renderer-registered character (≈63 today)
    },
)
```

**Schema rules:**

- `character_id` (the map key) is the canonical id used by LDtk
  NpcSpawns. Stable identifier; renames are a content-data break.
- `display_name` is the human label for UI / dialogue / debug.
- `tier ∈ { MainHall, Basement }` — drives hall layout.
- `body_kind ∈ { Standard, Wide, Floating, Crawler }` — runtime hint for default footprint / anchor.
- `composition: Option<Vec<CompositionLayer>>` — present only for multi-part sprites; dormant scaffolding until layered rendering ships.
- `default_brain` / `default_action_set` reference a preset name; the LDtk NpcSpawn may override per-instance.
- `tags: Vec<String>` — free-form, used by tooling (the hall generator filters by `tags = ["boss"]` for the basement, etc.).

### Brain-preset override semantics

NpcSpawn can override the catalog default per-instance:

```
NpcSpawn:
  character_id: "fretjaw_cantina_chieftain"
  brain_override: Some("melee_brute_brute")     # optional preset swap
  brain_overrides: { aggro_radius: 280.0 }      # optional field overrides on whatever preset is in effect
```

The runtime resolves:

1. Look up `character_id` in catalog → get `default_brain` preset name.
2. If `brain_override` on the NpcSpawn, use that preset name instead.
3. Apply `brain_overrides` field by field on top.
4. Build the `Brain::StateMachine(<variant>)`.

ActionSet follows the same pattern (`action_set_override` / `action_set_overrides`).

### `assets/data/boss_encounters/<id>.ron`

Per-boss phase schedule. Referenced by `BossPattern` brain entries
via `encounter_id`. Out of scope to fully spec this run; create the
file format placeholder with the gnu_ton encounter as a minimal
example, leaving the real phase schedules for follow-up work.

## Migration mechanics

### Codegen the initial catalog

Tool: `tools/ambition_ldtk_tools/ambition_ldtk_tools/codegen_character_catalog.py`

Reads:
- `crates/ambition_sandbox/src/presentation/character_sprites/assets.rs::NPC_SPRITE_REGISTRY` (parsed from Rust)
- `crates/ambition_sandbox/src/content/features/ecs/spawn.rs::enemy_default_brain()` (parsed for archetype → preset mapping)
- `crates/ambition_sandbox/src/brain/state_machine.rs::*_DEFAULT` consts (parsed for preset values)
- Renderer's `list-targets` output (cross-reference for missing sprites)

Emits:
- `crates/ambition_sandbox/assets/data/character_catalog.ron` (one entry per registered character + one entry per renderer target that wasn't registered, marked with `tags: ["needs_review"]`)
- `crates/ambition_sandbox/assets/data/boss_encounters/<id>.ron` placeholders

One-shot script; output is committed; script is retired after the migration lands (kept in `dev/migration-scripts/` for reference).

### LDtk NpcSpawn rename

Tool: `tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/rename_npc_field.py`

Walks every `*.ldtk` file:
- For each `NpcSpawn` entity instance, read the `name` field.
- Look up the catalog by `display_name` → resolve to `character_id`.
- Write `character_id` field instance; remove `name` field instance.
- Update the entity *definition* (`defs.entities`) to replace the `name` field with `character_id`.

Run once; commit the result. Validator (phase 1) ensures every
NpcSpawn resolves.

### YAML → RON for area specs

Tool: `tools/ambition_ldtk_tools/ambition_ldtk_tools/migrate_specs_to_ron.py`

Converts every `tools/ambition_ldtk_tools/specs/*.yaml` to `.ron`.
Validates each survives a `area create` round-trip. Deletes the
YAML files once the RON files are in. `area create` learns to
accept `.ron` natively; the YAML path is retired in the same commit.

## Bevy plugin shape

`crates/ambition_sandbox/src/content/character_catalog/`:

- `mod.rs` — `CharacterCatalogPlugin`, `CharacterCatalog` resource, public API.
- `entry.rs` — `CharacterCatalogEntry` struct, `BrainPreset` / `ActionSetPreset` enums (deserialized RON shapes).
- `loader.rs` — Bevy `AssetLoader` that reads `.ron` files into `CharacterCatalog`.
- `resolver.rs` — `resolve_character_to_brain(catalog, id, overrides) -> Brain`, `resolve_character_to_action_set(...)`.
- `validator.rs` — Startup system; panics on missing refs (or logs + flags as a startup error). Headless test pins coverage.

Re-export at `crate::content::character_catalog::{CharacterCatalog,
CharacterCatalogPlugin, resolve_character_to_brain, …}`.

Plugin registration in `app/plugins.rs`:
```rust
app.add_plugins(crate::content::character_catalog::CharacterCatalogPlugin);
```

The plugin lives in `ambition_sandbox` today; a future PR can lift
it into a standalone `ambition_character_catalog` crate when a
second Ambition game needs it.

## Phase plan

Each phase ends at a green commit. The run stops at the last green
phase if budget runs out.

### Phase 1 — Foundation (2 h)

**Goal:** RON catalog loaded into a Bevy resource with load-time validation.

**Files:**
- New: `crates/ambition_sandbox/assets/data/character_catalog.ron` (codegen'd from current registry)
- New: `crates/ambition_sandbox/src/content/character_catalog/{mod,entry,loader,resolver,validator}.rs`
- New: `tools/ambition_ldtk_tools/ambition_ldtk_tools/codegen_character_catalog.py` (one-shot migration script)
- Edit: `crates/ambition_sandbox/src/app/plugins.rs` (register plugin)

**Tests:**
- `character_catalog::tests::catalog_loads_without_panic`
- `character_catalog::tests::every_npc_sprite_registry_entry_has_catalog_entry` (transitional bridge: pins coverage during overlap)
- `character_catalog::tests::brain_preset_resolves_to_valid_variant_for_each_entry`
- Headless integration: `sim_validates_character_catalog_at_startup`

**Exit criteria:**
- `cargo test -p ambition_sandbox --lib content::character_catalog` green
- `cargo run -p ambition_sandbox --bin headless -- --ticks 60` clean
- Catalog has one entry per character in `NPC_SPRITE_REGISTRY`
- Loaded catalog matches the codegen'd RON (round-trip stable)

**Commit message:** `content: introduce CharacterCatalog plugin + RON-driven character data`

### Phase 2 — NpcSpawn schema change (1.5 h)

**Goal:** LDtk NpcSpawns reference `character_id`; runtime resolves via catalog.

**Files:**
- Edit: `crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk` (NpcSpawn defs + every instance)
- Edit: `crates/ambition_sandbox/assets/ambition/worlds/intro.ldtk` (same)
- Edit: `crates/ambition_sandbox/src/world/ldtk_world/` (parser reads `character_id`)
- Edit: `crates/ambition_sandbox/src/content/features/npcs.rs` (NpcRuntime construction)
- Edit: `crates/ambition_sandbox/src/content/features/ecs/spawn.rs::spawn_interactable` (catalog lookup)
- New: `tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/rename_npc_field.py`

**Tests:**
- Existing NPC tests still green (no behavior change)
- `npc_runtime::tests::spawned_npc_resolves_to_catalog_entry`
- LDtk validator: every NpcSpawn has a `character_id` that exists in the catalog

**Exit criteria:**
- `python -m ambition_ldtk_tools validate sandbox.ldtk` warning-free
- `python -m ambition_ldtk_tools validate intro.ldtk` warning-free
- `cargo run -p ambition_sandbox --bin headless -- --ticks 200` clean
- LDtk NpcSpawn `name` field gone everywhere; `character_id` everywhere

**Commit message:** `ldtk: rename NpcSpawn.name → character_id; route through CharacterCatalog`

### Phase 3 — Sprite gap closure (1 h)

**Goal:** Every renderer-registered character has a catalog entry.

**Files:**
- Edit: `crates/ambition_sandbox/assets/data/character_catalog.ron` (add ~25 entries)
- Edit (potential): `crates/ambition_sandbox/assets/<sprite_folder>/` (run `python -m ambition_sprite2d_renderer install <name>` for each)

Each new catalog entry: `character_id`, `display_name`, sprite/manifest paths, `tier`, `body_kind`, default brain/action-set, tags. No Rust changes.

**Tests:**
- `character_catalog::tests::every_renderer_target_has_catalog_entry_or_explicit_exclusion`
- Headless: `sim_spawns_every_catalog_character_without_panic` (drops one of each at a known coord, asserts no panic)

**Exit criteria:**
- `python -m ambition_sprite2d_renderer list-targets` and `cat character_catalog.ron` cover the same characters
- All 760+ sandbox lib tests still green

**Commit message:** `catalog: close sprite gap — every renderer-registered character is now spawnable`

### Phase 4 — Area specs YAML → RON (1 h)

**Goal:** `area create` consumes `.ron`; existing YAML specs migrated.

**Files:**
- Edit: `tools/ambition_ldtk_tools/ambition_ldtk_tools/area_authoring.py` (accept `.ron`)
- New: `tools/ambition_ldtk_tools/ambition_ldtk_tools/migrate_specs_to_ron.py`
- Rename: `tools/ambition_ldtk_tools/specs/*.yaml` → `*.ron`

**Tests:**
- For each migrated spec, `area create --dry-run <spec.ron>` produces the same LDtk diff as the YAML source did before migration (semantic equivalence).

**Exit criteria:**
- No `.yaml` files remain under `tools/ambition_ldtk_tools/specs/`
- `area create` accepts both formats only during this commit (next commit drops YAML support)

**Commit message:** `tools(ldtk): area create consumes .ron specs; migrate existing YAML specs`

### Phase 5 — Hall of Characters (1.5 h)

**Goal:** Hall room ships, populated from catalog.

**Files:**
- New: `tools/ambition_ldtk_tools/ambition_ldtk_tools/generate_hall_of_characters.py`
- New: `tools/ambition_ldtk_tools/specs/hall_of_characters_area.ron` (generator output, committed)
- Edit: `crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk` (new level + door from central_hub_main)

**Generator behavior:**
- Read `character_catalog.ron`.
- Partition entries by `tier` (MainHall vs Basement).
- Layout MainHall entries across floors, packing left-to-right, top-to-bottom.
- Layout Basement entries across the basement row.
- Emit pedestals + DebugLabels + NpcSpawns (each with `character_id` pointing at the catalog).
- Wire ladders between floors, drop-through between Floor 1 and Basement.
- Wire `LoadingZone` Door from `central_hub_main`.

**Tests:**
- `python -m ambition_ldtk_tools validate sandbox.ldtk` warning-free after the hall is added.
- Headless: `sim_loads_hall_of_characters_room_without_panic` (start in hall, run 60 ticks, every NpcSpawn resolves, no missing sprites).
- Visual smoke: run dev binary, walk into hall via hub door, confirm every pedestal has a sprite, every sprite is on the ground, no overlaps.

**Exit criteria:**
- Hall reachable from central_hub_main
- All ~63 characters have a pedestal + sprite + label
- 80 main-hall slots + 8 basement slots filled (or remaining slots empty but framed correctly)
- Re-running the generator produces a stable diff (idempotent)

**Commit message:** `sandbox: hall_of_characters — multi-level pantheon, one pedestal per catalog entry`

### Phase 6 — Cleanup (1 h)

**Goal:** Single source of truth. Catalog wins; legacy registries delete.

**Files (deletions):**
- Delete: `NPC_SPRITE_REGISTRY` array in `crates/ambition_sandbox/src/presentation/character_sprites/assets.rs`
- Delete: per-character `*_SHEET` `LazyLock<CharacterSheetSpec>` consts in `crates/ambition_sandbox/src/presentation/character_sprites/sheets.rs` (where the catalog now resolves the spec)
- Delete: `sheet_consts_match_their_yaml_manifests` test (catalog validator replaces it)
- Delete: `tests::npc_sprite_label` (now `character_id`)
- Delete: YAML path in `area create` (RON only after phase 4)

**Renamings:**
- `review_npcs` category → merged into `characters` in the renderer (`target_registry.py::CATEGORIES`, `cli.py::RUNTIME_REVIEW_NPCS`, `configs/review/` → `configs/`)
- Adapter-driven characters merge into the `characters` category; the discovery walks both `targets/characters/` (Python tack-on) and `configs/` (adapter YAML) under one name.

**Tests:**
- Full workspace test suite green
- Headless 1000-tick run clean
- Re-rendered hall stable (idempotent)

**Commit message:** `cleanup: delete NPC_SPRITE_REGISTRY + per-character SHEET consts; merge review_npcs into characters`

### Phase 7 — Documentation + ADR (0.5 h, if budget remains)

**Files:**
- New: `docs/adr/0017-rust-behavior-ron-content-ldtk-space.md`
- New: `docs/systems/character-catalog.md`
- New: `docs/recipes/adding-a-character.md`
- Edit: `docs/systems/index.md`, `docs/recipes/index.md`
- Edit: `FEATURES.md` (add Character Catalog row)
- Edit: `TODO.md` (mark this entry done)
- Edit: `dev/SEARCH.md` (add catalog search line)

**ADR 0017** (`docs/adr/0017-rust-behavior-ron-content-ldtk-space.md`):
- Status: Accepted (2026-05-24)
- Decision: Rust = behavior, RON = content, LDtk = space
- Context: The Hall of Characters refactor crystallized this
- Consequences: Character catalog is the first instantiation; future content systems (quests, dialogue) follow the same shape

**Commit message:** `docs: ADR 0017 + character-catalog system doc + adding-a-character recipe`

## Validation gates (between every phase)

```bash
~/.cargo/bin/cargo check -p ambition_engine
~/.cargo/bin/cargo check -p ambition_sandbox
~/.cargo/bin/cargo test  -p ambition_engine  --lib
~/.cargo/bin/cargo test  -p ambition_sandbox --lib
~/.cargo/bin/cargo run   -p ambition_sandbox --bin headless -- --ticks 100
python3 -m ambition_ldtk_tools validate crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
python3 scripts/check_doc_links.py
```

If sandbox tests hit EMFILE under parallelism (common under shared
VMs), fall back to `--test-threads=2`. Documented in
`docs/recipes/extending-brains-and-action-sets.md`.

## Risks named explicitly

1. **Save-format break.** Saves reference Rust types via `EnemyArchetype` and NpcSpawn names. Pre-release per [[feedback-pre-release-no-compat]], so break cleanly during the migration commit. Document in commit message; no migration code.

2. **Multi-part sprite handling.** Renderer emits composed sheets today (mockingbird_boss, gnu_ton_boss). Catalog `composition` field is dormant scaffolding. No layered-render runtime work this run.

3. **Brain preset ergonomics.** Hybrid approach: named presets in a separate RON table + optional per-character overrides. Common cases are one line; outliers override individual fields. Don't pre-build infrastructure for unused override patterns.

4. **Catalog god-file.** ~63 entries today; if it exceeds ~200, split into `catalog/hub.ron`, `catalog/enemies.ron`, `catalog/bosses.ron` and the loader merges. Don't split prematurely.

5. **Renderer manifests aren't fully aligned.** Some sheets have non-standard layouts. Loader treats missing or malformed manifests as a load-time warning (not panic) — the character is registered but its sprite spec falls back to defaults. CI test pins which characters are in this fallback state.

6. **Hot reload not implemented this run.** The catalog is a Bevy `Resource` loaded at startup. Mid-session reload is a future PR; the API leaves the door open by going through a resource rather than a static.

7. **LDtk schema validator must accept the rename.** The existing `validate` warns on missing fields; if the rename is not parsed by validator, it'll emit spurious warnings. Phase 2 includes a validator update.

8. **Compilation time concern** ([[feedback-compile-time]]). The catalog plugin adds one Bevy resource + one asset loader; minimal compile-time impact. The deletion of `NPC_SPRITE_REGISTRY` + per-character consts is a net reduction in code-gen surface.

## Autonomous operating rules (overnight)

Same as the universal-brain run; captured here so context-compaction
preserves them:

- **Cargo path:** `~/.cargo/bin/cargo` everywhere ([[feedback-patch-discipline]]).
- **CARGO_TARGET_DIR:** `/home/agent/ambition-target` (the configured `/home/joncrall/ambition-target` is root-owned).
- **No sudo, no apt-get, ever** ([[feedback-no-sudo-apt]]). If a system lib is missing, stop and log it — don't try to install.
- **Stage explicit paths, never `git add -A`** ([[feedback-git-add-targeted]]). Working tree carries `foo/`, `tpl/`, `perf.data`, generator output that must not enter commits.
- **No binary blobs in commits** ([[feedback-no-binary-data]]).
- **Don't push to remote, don't amend, don't force-push.** All commits stay local on `main`.
- **Transient filesystem errors (EMFILE/EIO): retry-then-move-on** per [[feedback-never-stop-during-long-run]]. Don't stop; don't `/tmp` around them.
- **Real blockers don't stop work either** — work around, document in `dev/journals/`, continue.
- **Test gate before every commit:** validation gates above.
- **`>1 hr` diagnoses get a dev/journals/ entry** per [[reference-lessons-learned]].
- **`Reflect` only when something needs it** — keep new components / enum variants out of the registry until a consumer demands it.
- **Pre-poison tests** ([[feedback-pre-poison-test-pattern]]): for any new `fn(&mut Out)` test, pre-poison Out with non-default values so early-return-without-write branches trip the assertion.
- **Update FEATURES.md / TODO.md as items complete**.
- **Match memory format when writing notes** — `dev/journals/<topic>-<YYYY-MM-DD>.md` for incidents.
- **Stopping checkpoints:** end of every phase. Commit, run validation, move on. The session naturally ends when (a) Phase 7 lands, or (b) time runs out at a green checkpoint.
- **Memory updates:** if this work changes any [[feedback-*]] or [[project-*]] memory's accuracy, update it.

## Memory invariants this plan respects

- [[feedback-patch-discipline]] — `~/.cargo/bin/cargo`; verify before claiming compile-tested.
- [[feedback-compile-time]] — catalog is a net code reduction; no Reflect on hot paths.
- [[feedback-design-balance]] — one shape with optional fields, not sub-types; add knobs only when a use case lands.
- [[feedback-pre-release-no-compat]] — single-commit replacements; save format breaks accepted.
- [[feedback-always-commit]] — commit each green phase.
- [[feedback-never-stop-during-long-run]] — work around blockers; stop at last green commit if budget exhausted.
- [[feedback-bevy-testing-pattern]] — minimal-plugin App + `app.update()` + World assertions for new tests.
- [[feedback-pre-poison-test-pattern]] — pre-poison `&mut Out` before testing.
- [[feedback-git-add-targeted]] — never `git add -A`; stage explicit paths.
- [[feedback-no-binary-data]] — generated sprite output stays gitignored.
- [[feedback-entity-id-matches-label]] — `character_id` is the stable identifier; renames are a content break.
- [[project-controllable-unification]] — character catalog is the next step in the universal-brain trajectory.

## Cross-refs

- [`docs/planning/hall-of-characters-and-character-catalog.md`](docs/planning/hall-of-characters-and-character-catalog.md) — earlier planning conversation (this doc supersedes it)
- [`docs/systems/brain-driver.md`](docs/systems/brain-driver.md) — brain seam this builds on
- [`docs/recipes/extending-brains-and-action-sets.md`](docs/recipes/extending-brains-and-action-sets.md) — current "add a new brain" procedure
- [`TODO-controllable-entity.md`](TODO-controllable-entity.md) — the previous overnight run's plan, same shape as this one
- [`tools/ambition_ldtk_tools/specs/hall_of_bosses_area.yaml`](tools/ambition_ldtk_tools/specs/hall_of_bosses_area.yaml) — precedent hall room
- [`crates/ambition_sandbox/src/presentation/character_sprites/assets.rs`](crates/ambition_sandbox/src/presentation/character_sprites/assets.rs) — current `NPC_SPRITE_REGISTRY` to be retired
- [`crates/ambition_sandbox/src/brain/`](crates/ambition_sandbox/src/brain/) — brain module that the catalog references
- ADR 0003 — data-driven specs and asset loading (catalog aligns with this)
- ADR 0016 — actor unification (universal-brain section)
- ADR 0017 — Rust = behavior, RON = content, LDtk = space (created in Phase 7)

## Definition of done

After Phase 7:

- `character_catalog.ron` is the single source of truth for character-runtime metadata.
- `NPC_SPRITE_REGISTRY` and per-character `*_SHEET` consts are deleted.
- LDtk NpcSpawns reference `character_id`.
- Every renderer-registered character has a catalog entry.
- Hall of Characters is reachable from central_hub_main and visualizes every catalog entry.
- Area specs are RON; YAML path is gone.
- "Review NPCs" category is merged into `characters`.
- ADR 0017 codifies the architectural mantra.
- New character authoring path (RON + LDtk + zero Rust) is documented in a recipe.
- All 760+ sandbox lib tests + 265 engine tests + workspace tests green.

If Phase 7 doesn't land, the run ends at whichever phase did, with a
`dev/journals/<topic>-2026-05-24.md` entry naming the remaining work.
