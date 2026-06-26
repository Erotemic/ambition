# Character barks + Hall-of-Characters dialogue

Status: **scaffold landed** (mechanism + 3 exemplars). Remaining work is
content authoring + deleting the legacy fallback. This doc is the handoff for
the content-writing pass.

## What this is

A character's **voice** now lives on its **identity** — the catalog row — not
scattered across hardcoded Rust tables. Every system that spawns a character (a
room placement, the peaceful→hostile flip, the Hall gallery) reads the same
lines by `character_id`. See the design note in the conversation that produced
this; the short version:

- **Barks** = one-line speech bubbles, authored in `character_catalog.ron` as
  pools keyed by occasion.
- **Hall dialogue** = the branching Yarn conversation shown when you Inspect a
  pedestal in the Hall of Characters.

## Schema (already in place)

`crates/ambition_characters/src/actor/character_catalog/entry.rs`:

```rust
pub enum BarkSituation { OnHit, Provoked, Idle, Hall }

pub struct CharacterBarks {
    pub on_hit: Vec<String>,    // struck in combat (rotates with strike count)
    pub provoked: Vec<String>,  // peaceful NPC turns hostile (usually one line)
    pub idle: Vec<String>,      // ambient muttering while idling
    pub hall: Vec<String>,      // Hall-of-Characters gallery line (fun/self-aware)
}
```

`CharacterCatalogEntry` gained two fields (both `#[serde(default)]`):

```ron
barks: (
    on_hit:   ["...", "..."],
    provoked: ["..."],
    idle:     ["..."],
    hall:     ["..."],
),
hall_dialogue_id: Some("hall_<character_id>"),
```

All pools default empty (silent). See the **three exemplars** for the exact RON
shape: `npc_pirate_admiral`, `stochastic_parrot`, `npc_architect` in
`crates/ambition_gameplay_core/assets/data/character_catalog.ron`.

## How firing works (already wired)

| Situation | Fired by | Read site |
|-----------|----------|-----------|
| `OnHit` (peaceful NPC) | `apply_actor_hit` | `npcs::npc_hit_bark_line` |
| `OnHit` (hostile enemy/boss) | `apply_actor_hit` | resolves `character_id_for_display_name` → catalog |
| `Provoked` | `apply_actor_hit` (strike threshold) | `npcs::npc_hostile_bark_line` |
| `Idle` | `tick_npc_idle_barks` (not in hall) | `npcs::npc_ambient_bark_line` |
| `Hall` | `tick_npc_idle_barks` (active area `hall_of_characters`) | `npcs::npc_ambient_bark_line` |

All read sites are **catalog-first with a TEMP legacy fallback** (see below).
Pickers live in `character_roster.rs`:
`bark_line_for_character_id(id, situation, rotation)` and
`hall_dialogue_id_for_character_id(id)`.

Hall dialogue: `hall_dialogue_id` → the hall generator stamps it onto each
pedestal's `NpcSpawn.dialogue_id` → Inspect starts that Yarn node.
`known_dialogue_ids()` folds the catalog ids in, so the LDtk validator accepts
authored `hall_<id>` nodes with no second list to maintain.

## The content-writing job

### 1. Populate `barks` for every character

Migrate the existing lines, then add Hall lines. Sources to migrate FROM:

- Peaceful NPCs: `crates/ambition_gameplay_core/src/features/npcs.rs`
  - `npc_hit_barks()` → each arm's lines → that character's `barks.on_hit`
  - `npc_hostile_bark()` → `barks.provoked`
  - `npc_idle_bark_line()` (the parrot) → `barks.idle`
  - Note: arms match fuzzy `key.contains()/name.contains()`; map each to the
    real `character_id`(s). Some arms cover several ids (the pirates).
- Enemies/bosses: `CombatBanterRegistry` installers, keyed by **display name**:
  - `crates/ambition_content/src/banter.rs` (`install_pirate_banter`, …)
  - `crates/ambition_content/src/intro/banter.rs`
  - `crates/ambition_content/src/bosses/banter.rs`
  - Map display name → `character_id` (use `character_id_for_display_name`) →
    `barks.on_hit` / `barks.idle`.

Then **add `hall` lines** for every character — the fun, fourth-wall,
"I'm-on-a-plinth" gallery beat. This is the genuinely new content.

### 2. Author Hall dialogue

For each character that should have an Inspect conversation:

- Add `hall_dialogue_id: Some("hall_<character_id>")` to its catalog row.
- Add a matching `title: hall_<character_id>` node to
  `crates/ambition_gameplay_core/assets/dialogue/sandbox/hall.yarn`
  (follow the three exemplars; keep them short — a line or three, optional
  branch). **Every `hall_dialogue_id` MUST have a node** or the dialogue will
  start an empty/unknown node.

### 3. Delete the TEMP fallback (same PR as full population)

Once `barks` covers every character, remove the bridge — search for the
`TEMP fallback` comments:

- `npcs.rs`: delete `npc_hit_barks`, `npc_hostile_bark`, `npc_idle_bark_line`
  and the fallback branches in `npc_hit_bark_line` / `npc_hostile_bark_line` /
  `npc_ambient_bark_line`.
- `actor_hit.rs`: drop the `.or_else(|| combat_banter…)` fallback and the
  `combat_banter` param threading.
- `ambition_content` banter installers + `CombatBanterRegistry` (and its
  resource insertion) once nothing reads it.

### 4. Regenerate the hall + re-embed

The hall pedestals' `dialogue_id` only update after regeneration. **The
committed `specs/hall_of_characters_area.ron` is currently stale** (it predates
~10 catalog additions); regenerating will catch all of that up, not just the
dialogue ids — review the diff with that in mind.

```bash
PYTHONPATH=tools/ambition_ldtk_tools \
  python3 -m ambition_ldtk_tools.generate_hall_of_characters
# then re-embed the spec into sandbox.ldtk via the normal world build/import
# step, so the embedded hall pedestals carry the new dialogue_ids.
```

## Invariants / gotchas

- The hall area id is the const `HALL_OF_CHARACTERS_AREA = "hall_of_characters"`
  in `features/ecs/actors/update.rs`, matching the generator. Keep them in sync.
- `barks` is Rust-authored RON; the embedded-catalog parse test
  (`character_roster::tests::catalog_loads_without_panic`) pins it parses.
- The Python generator regexes `hall_dialogue_id: Some("…")` out of each entry
  block, bounded by the next catalog key — a very large `barks` list is fine.
- Pins to keep green: `exemplar_barks_resolve_from_catalog`,
  `exemplar_hall_dialogue_ids_resolve_and_are_known` (character_roster),
  `character_barks_pick_rotates_and_empty_is_none` (ambition_characters),
  `test_generate_hall_of_characters.py`.
