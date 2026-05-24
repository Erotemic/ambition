# Hall of Characters + RON-driven character catalog — planning doc

**Status (2026-05-24):** planning only. No code changes yet. Jon
needs to think over the tradeoffs before sequencing.

## TL;DR

Two intertwined pieces of work:

1. **Hall of Characters** — a sandbox room with one idle-NPC pedestal
   per character sprite in the game, plus a basement for big
   characters that don't fit the main floor. Used for stress-testing
   many actors at once, verifying every sprite has a working
   spawn-time wiring, and later as a place to give each NPC its real
   brain and watch them interact.

2. **RON-driven character catalog** — refactor away from the
   hand-maintained Rust `NPC_SPRITE_REGISTRY` (and the parallel
   `*_SHEET` `LazyLock` consts) toward a single
   `crates/ambition_sandbox/assets/data/character_catalog.ron` plus
   per-renderer-target manifests, with load-time validation. The goal
   is "authoring a new character touches RON + LDtk only — no Rust."

The two pieces support each other: the hall exposes every missing
sprite registration, and the catalog is what makes hall extension
zero-cost.

## Pass 1 — Hall of Characters (existing authoring path)

### Decisions already locked

| Decision | Choice | Why |
|---|---|---|
| Door placement | From `central_hub_main` | Mirrors hall_of_bosses; single hop from hub |
| Basement entry | Drop-through hole in floor | Matches existing hub→basement pattern; no new auth needed |
| Boss treatment | Standees only — `Brain::stand_still()` + peaceful | Hall is a gallery, not an arena; combat lives in real boss arenas |
| Authoring | Python generator script | Re-runnable when sprites are added |
| Sprite-gap policy | Hand-complete `NPC_SPRITE_REGISTRY` in this pass | Catches every missing wiring before the catalog refactor |

### Layout proposal

One LDtk level, two floors:

```
+-----------------------------------------------------------+ y=0
| ▒ ceiling ▒                                                |
|                                                            |
|  🧍 🧍 🧍 🧍 🧍 …  main hall (small/medium NPCs)  … 🧍 🧍   |  y=64..480
|  [128px pedestal × N]                                      |
| ▒▒▒▒▒▒▒▒▒▒▒▒▒▒ floor ▒▒▒▒[drop hole]▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒  |  y=512
|                                                            |
|  🦖   🛸     🪲     🧙       basement (big/boss sprites)   |  y=560..960
|  [256px pedestal × M]                                      |
| ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ floor ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ |  y=992
+-----------------------------------------------------------+
worldX: 8000 (east of tech_bros_basement at ~6000)
pxWid: ~7700 (auto-sized from slot count)
pxHei: 1024
```

**Slot rules:**

- **Main hall slot:** 128 px wide × 96 tall pedestal. Sprite foot at
  slot bottom-center. ~12 px gap between slots.
- **Basement slot:** 256 px wide × 240 tall pedestal. Big/boss
  sprites here.
- Single ground Y per tier — foot anchor lines up across the row.
- `DebugLabel` above each pedestal with the character id → the hall
  doubles as a sprite-name registry check.

**Tier classification** (initial draft; lives in the generator
script):

| Tier | Characters |
|---|---|
| Basement (big) | gnu_ton_boss, mockingbird_boss, flying_spaghetti_monster_boss, smart_house, trex_enemy, bear_mauler, raptor_stalker, mantis_lancer |
| Main hall (everyone else) | All other registered character targets + review_npcs |

Roughly 65 main-hall slots × 128 px ≈ 8300 px (call it 8400 wide).
8 basement slots × 256 px ≈ 2050 px. Room is **~8400 × 1024**.

### The extensibility hinge: generator script

Hand-authoring 65+ `NpcSpawn` entries in YAML is fragile.
Authoring-by-generator is the model:

**New script:** `tools/ambition_ldtk_tools/ambition_ldtk_tools/generate_hall_of_characters.py`

**Inputs:**

- Renderer's registered targets (via
  `ambition_sprite2d_renderer.target_registry.discover_all_targets()`)
- Sandbox's `NPC_SPRITE_REGISTRY` (parsed from the Rust file, or
  read from a side-by-side JSON snapshot if we prefer not to parse
  Rust)
- A small classification table inside the script: `BASEMENT_TIER = {...}`

**Output:** `tools/ambition_ldtk_tools/specs/hall_of_characters_area.yaml`

**Apply step:** `python -m ambition_ldtk_tools area create
specs/hall_of_characters_area.yaml --apply`

Re-run the generator any time `NPC_SPRITE_REGISTRY` grows.

### Sprite-gap pass

The ~25 characters that render but aren't spawnable get registry
rows in this PR. Mechanical but ~25 sheet-spec consts × ~10 lines
each = ~250 lines of `sheets.rs` additions and a parallel ~25 rows
in `assets.rs::NPC_SPRITE_REGISTRY`. The `sheet_consts_match_their_yaml_manifests` test catches drift.

**Note:** boss multi-part sprites (`mockingbird_boss`,
`gnu_ton_boss`) have custom install paths — these may need special
handling beyond the standard registry pattern. Check the existing
boss-sprite loaders before bulk-adding.

### Validation checkpoints

1. After the generator runs: `python -m ambition_ldtk_tools validate sandbox.ldtk` warns-free.
2. After commit: `cargo test -p ambition_sandbox --lib presentation::character_sprites` green.
3. Smoke: `cargo run -p ambition_sandbox --bin headless -- --ticks 60 --start-room hall_of_characters` clean.
4. Visual: load the room in the dev binary, eyeball that every sprite renders, none floats, none overlaps, basement has headroom.

## Pass 2 — RON-driven character catalog (separate PR)

Pass 1 leaves us with a complete `NPC_SPRITE_REGISTRY`. Pass 2 moves
authoring out of Rust entirely.

### Target shape

**One RON file (or a small set, split by zone):**

```ron
// crates/ambition_sandbox/assets/data/character_catalog.ron
(
    characters: {
        "kernel_guide": (
            display_name: "Kernel Guide NPC",
            spritesheet: "characters/kernel_guide_spritesheet.png",
            manifest: "characters/kernel_guide_spritesheet.ron",  // renderer-emitted
            tier: MainHall,
            default_brain: "patrol_peaceful",
            default_action_set: "peaceful",
            tags: ["hub", "guide"],
        ),
        "puppy_slug_variant2": (
            display_name: "Puppy Slug Variant 2",
            spritesheet: "characters/puppy_slug_variant2_spritesheet.png",
            manifest: "characters/puppy_slug_variant2_spritesheet.ron",
            tier: MainHall,
            default_brain: "wanderer_puppy_slug",
            default_action_set: "peaceful",
            tags: ["enemy", "ai_era"],
        ),
        "gnu_ton_boss": (
            display_name: "GNU-ton Boss",
            spritesheet: "characters/gnu_ton_boss_spritesheet.png",
            manifest: "characters/gnu_ton_boss_spritesheet.ron",
            tier: Basement,
            default_brain: "boss_pattern_gnu_ton",
            default_action_set: "boss_default",
            tags: ["boss", "act1"],
        ),
    },
    brain_presets: {
        "patrol_peaceful":    Patrol(spawn_local_x: 0.0, radius: 96.0, aggressiveness: 0.0),
        "stand_still":        StandStill,
        "wanderer_puppy_slug": Wanderer(WandererCfg::PUPPY_SLUG_DEFAULT_OVERRIDE),
        "melee_brute_striker": MeleeBrute(MeleeBruteCfg::STRIKER_DEFAULT_OVERRIDE),
        "boss_pattern_gnu_ton": BossPattern(encounter_id: "gnu_ton"),
    },
    action_set_presets: {
        "peaceful":      (melee: None, ranged: None, special: None, move_style: Walk),
        "striker_swipe": (melee: Some(Swipe(STRIKER_DEFAULT)), move_style: Walk),
        "boss_default":  (ranged: Some(Bolt(speed: 380.0, damage: 1)), special: Some(BossSpotlight), move_style: Walk),
    },
)
```

The renderer's `*_spritesheet.ron` (already emitted today) becomes
the **source of truth for sheet dimensions** (frame width/height,
label width, animation rows). The character catalog points at it;
the runtime loads it and builds a `CharacterSheetSpec` at startup
rather than at compile time.

**LDtk side:**

```
NpcSpawn:
  character_id: "kernel_guide"        # one new string field
  # display_name, sprite, brain, action_set all come from catalog
  # remaining LDtk fields stay (patrol_radius, path_id, dialogue_id, prompt)
```

**Authoring a new character becomes:**

1. Render it (already automatic via renderer's `TARGETS` discovery).
2. Add one row to `character_catalog.ron`.
3. Author the LDtk `NpcSpawn` with `character_id`.

No Rust touched.

### Tradeoff matrix

| Aspect | Current Rust-driven | Proposed RON-driven |
|---|---|---|
| Authoring a new character | 3+ Rust edits + LDtk | 1 RON row + LDtk |
| Authoring needs Rust knowledge | Yes | No |
| Compile-time guarantees | Strong — typed sheet consts | Weaker — string ids, runtime parse |
| Drift between renderer ↔ sandbox | Pinning test catches it; tedious to keep in sync | Catalog points directly at renderer manifest |
| Validation | `cargo test` | Build.rs or load-time validator + CI test |
| Hot reload | Recompile sandbox | Catalog reloadable at runtime (huge for iteration) |
| Save-game compatibility | Saves reference Rust types (e.g. `EnemyArchetype`) | Saves reference string ids — needs stability promise |
| Brain customization per character | Implicit (NPC fields) or hardcoded (enemy match arms) | Per-character `brain_id` + override params; LDtk-authorable |
| Discoverability | grep `NPC_SPRITE_REGISTRY` | Read one RON file |
| AI/non-coder contribution | Hard — Rust + LDtk | Easy — RON + LDtk |
| Migration cost | n/a | ~1 week of focused work |

### Risks named explicitly

1. **Save-format break.** Saves reference Rust types today; string-id
   saves need a stability promise (no silent renames). Mitigation:
   we're pre-release per [[feedback-pre-release-no-compat]], so the
   answer is "break it cleanly during the migration commit."

2. **Build-time vs load-time validation.** Two options:
   - **`build.rs` codegen** — read catalog at build time, emit
     `pub static CATALOG: &[Character] = &[…]` Rust. Catches missing
     references during `cargo build`. Disables hot-reload.
   - **Load-time validator** — Bevy startup system that panics or
     logs on missing refs. Hot-reload friendly. Headless test pins
     the validator so CI catches breakage.

   **Recommendation:** load-time + a CI test. Catches drift while
   keeping RON hot-reload (huge for content iteration). The test is
   `headless --start-room <each_authored_room> --ticks 0` and the
   validator panics if any `character_id` in any LDtk level is
   missing from the catalog.

3. **Brain presets are the hardest piece.** Brain templates carry
   typed cfg/state structs. Going to RON means either:
   - Per-template variant with all knobs spelled out (verbose, fully
     data-driven).
   - Named-constant references like `MeleeBruteCfg::STRIKER_DEFAULT`
     which still requires Rust changes.

   **Recommendation:** RON variants accept partial cfg structs with
   field-level defaults filled in by the loader from the matching
   `*_DEFAULT` const. Common cases stay one line; outliers can
   override individual fields.

4. **The catalog could become a 1000-line god-file.** Mitigation:
   split by tier or zone (`hub_characters.ron`,
   `pirate_characters.ron`, `boss_characters.ron`); the loader
   merges them. Same shape, easier to review.

5. **Renderer manifests aren't fully aligned.** Some sheets
   (`mockingbird_boss`, `gnu_ton_boss`) have non-standard layouts.
   The catalog's `manifest` field needs to gracefully handle
   "multi-part sprite" cases — likely with a small enum tagging
   the layout style, or an explicit override section.

6. **Re-rendering when manifest schema changes.** If we tighten the
   renderer's RON schema, every character's manifest needs
   re-rendering. Mitigation: `python -m ambition_sprite2d_renderer
   regenerate-all` already exists for exactly this.

### Migration sequence (Pass 2)

1. **Codegen the initial catalog** from `NPC_SPRITE_REGISTRY`. One-time
   script that reads the Rust + the renderer manifests and emits the
   RON.
2. **Add `CharacterCatalog` resource** + RON loader + load-time
   validator. Behind a feature flag so it doesn't disturb the
   existing path.
3. **Migrate one zone** (e.g. `central_hub_main`'s NpcSpawns) to
   `character_id` lookup. The LDtk `name` field stays as a fallback
   during overlap.
4. **Flip the rest of the LDtk levels** zone-by-zone. CI test pins
   each zone's coverage.
5. **Delete `NPC_SPRITE_REGISTRY`** and the per-character
   `*_SHEET` `LazyLock` consts. The `sheet_consts_match_their_yaml_manifests`
   test gets retired in favor of the catalog validator.
6. **Add the LDtk authoring docs** for new contributors: "to add a
   character, write a RON row + an `NpcSpawn`."

### Sequencing decision (Jon, 2026-05-24)

**Pass 1 first** is the recommendation. The hall ships visible
value immediately and forces us to confront the full registration
gap before committing to a refactor. Pass 2 then has a complete,
known input.

**Jon's actual call (2026-05-24):** planning doc only for now —
think it over before sequencing.

## Naming: "review NPCs" is stale

Jon flagged that "review NPCs" doesn't match his mental model.
Tracing the term:

- It originated when the YAML+rig pipeline was used to render
  "concept-art-style" NPCs for design **review** before promoting
  them to runtime sprites.
- Today many of these (alice, bob, kernel_guide, architect,
  vault_keeper, merchant_prototype, etc.) **do** ship to the
  runtime via `RUNTIME_REVIEW_NPCS` in the renderer CLI.
- So the name now describes *how the sprite is authored* (YAML +
  toon rig) more than *what it's for*. The "review" framing is
  legacy.

**Rename candidates:**

| Name | Captures | Cost |
|---|---|---|
| `rigged_characters` | Authoring goes through an animation rig adapter | Renaming touch — CATEGORIES const, CLI bulk paths, `RUNTIME_REVIEW_NPCS`, directory move `configs/review/` → `configs/rigged/` |
| `toon_characters` / `toon_npcs` | They all use the toon rig today | Same touch surface; tighter coupling to the current rig |
| `composed_characters` | They're composed from YAML palette + rig | Same touch surface; more abstract |
| (no separate category — merge into `characters`) | Drops the artificial split entirely; the renderer's category is just `characters` regardless of authoring path | Bigger refactor but the cleanest end state — the consumer doesn't care how a sheet was authored |

**Recommendation:** **merge into `characters`** as part of Pass 2.
The "tack-on Python vs YAML+rig" distinction is an implementation
detail of the renderer, not something downstream consumers (the
sandbox, the hall, anyone authoring a level) should know about. The
category column in `list-targets` becomes one row per character,
and the renderer's discovery decides at import time which authoring
shape to use.

If a single-step rename is preferred before the merge, **`rigged_characters`**
is the closest neutral name.

## RON vs YAML for tooling specs (Jon's other concern)

Jon: "Why are we using YAML? I thought we switched to RON for
things that Rust consumes?"

Strictly accurate answer: **area-spec YAMLs aren't consumed by
Rust.** They're consumed by Python (`ambition_ldtk_tools area
create …`) which then writes into the LDtk JSON. The runtime
reads the LDtk JSON, not the YAML. So the "RON for Rust" rule
isn't technically violated.

But the spirit of the rule is "**one config format across the
project**," and right now we have:

| File | Format | Consumed by |
|---|---|---|
| Brain configs (in code today, RON-target in Pass 2) | Rust→RON | Rust runtime |
| Player tuning, abilities | RON | Rust runtime |
| Settings, save data | RON | Rust runtime |
| Sprite-renderer adapter configs (`configs/*.yaml`) | YAML | Python renderer |
| Area-spec authoring (`specs/*_area.yaml`) | YAML | Python LDtk tool |
| LDtk world files (`*.ldtk`) | JSON | Both (Python tool writes, Rust runtime reads) |

The YAMLs are pre-policy artifacts. They predate the
"RON-everywhere" line. Three options:

1. **Keep YAML for tool-only specs.** Cheapest. The
   "RON for Rust" rule remains literally true. Two formats in the
   tree.
2. **Migrate area specs + adapter configs to RON.** Aligns with the
   spirit of the rule. Mechanical but touches dozens of files. Cost
   is upfront; benefit is one less format to remember.
3. **Use RON for new specs, leave existing YAMLs in place, migrate
   opportunistically.** Pragmatic but leaves a permanent two-format
   surface.

**Recommendation:** **(2) — migrate to RON.** The friction is
real and the existing YAMLs are small enough that a one-time
sweep is cheap. The Hall of Characters area spec should land as
`hall_of_characters_area.ron` and `ambition_ldtk_tools area
create` should accept either format during a brief overlap window.

Adapter configs (`configs/*.yaml` in the renderer) can move on
the same schedule. Python reads RON fine via `rtoml` or the
existing `python-ron` shims in the project.

**If migration is too noisy:** option (3) is the fallback. New
area specs land as `.ron`; existing `.yaml`s stay until a
follow-up sweep.

## Open questions for Jon

1. **"Review NPCs" rename.** Merge into `characters` (clean end
   state) vs. rename to `rigged_characters` (single-step,
   preserves the split)?

2. **YAML→RON for area specs.** Full migration now, opportunistic,
   or keep YAML at the tool boundary?

3. **Catalog scope creep.** Should the catalog also carry
   *animation timing* (windup / active / recover per attack) or do
   those stay in Rust attack-spec defaults? Today's brain templates
   couple cfg + state; the catalog mostly references presets. If
   per-character animation tweaks are wanted, the catalog grows.

4. **Multi-part bosses (mockingbird, gnu_ton).** Continue treating
   them as special-case loaders, or extend the catalog schema to
   describe multi-part composition declaratively (head + body +
   wings)?

5. **Brain RON ergonomics.** Verbose per-character cfg vs.
   named-preset references? The latter is more Rusty-feeling, the
   former is more data-driven.

6. **Catalog file split.** One big `character_catalog.ron` or
   per-zone splits (`hub_characters.ron`, `pirate_characters.ron`,
   etc.)?

## Cross-refs

- [`docs/systems/brain-driver.md`](../systems/brain-driver.md) — brain seam this builds on
- [`docs/recipes/extending-brains-and-action-sets.md`](../recipes/extending-brains-and-action-sets.md) — current "add a new brain" procedure
- [`tools/ambition_ldtk_tools/specs/hall_of_bosses_area.ron`](../../tools/ambition_ldtk_tools/specs/hall_of_bosses_area.ron) — precedent hall room (post-Phase-4 YAML→RON migration)
- [`crates/ambition_sandbox/src/presentation/character_sprites/assets.rs`](../../crates/ambition_sandbox/src/presentation/character_sprites/assets.rs) — current registry
- [[feedback-pre-release-no-compat]] — save-format break policy
- [[feedback-design-balance]] — narrow types beat wide generic ones; add knobs only when use cases land
- [[feedback-entity-id-matches-label]] — string id stability matters
- ADR 0003 — data-driven specs and asset loading (this aligns with that direction)
