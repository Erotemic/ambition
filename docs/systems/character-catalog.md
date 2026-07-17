# Character catalog

> Where do you go to add a new spawnable character to the sandbox?
> Here. Just the catalog file. No Rust changes for the common case.
> No LDtk schema changes either.

The character catalog (`crates/ambition_actors/assets/data/character_catalog.ron`)
is the single source of truth for every character the sandbox can
spawn — players, NPCs, enemies, bosses. Each catalog entry pairs a
stable `character_id` with:

- a human-facing display name,
- the on-disk sprite path,
- a default brain preset (one of `StandStill`, `Patrol`, `Wanderer`,
  `MeleeBrute`, `Skirmisher`, `Sniper`, `BossPattern`),
- a default action-set preset (move style + optional melee / ranged /
  special),
- a tier (`MainHall` or `Basement` — drives Hall of Characters layout),
- a body-kind hint (`Standard`, `Wide`, `Floating`, `Crawler`),
- optional composition layers (multi-part sprite scaffolding),
- a free-form tag list (used by tooling — the hall generator filters
  basement entries via `tags = ["boss"]`).

## Architectural posture (ADR 0017)

> **Rust = behavior. RON = content. LDtk = space.**

Brain *variants* (the `MeleeBrute` algorithm) stay typed in Rust so
the compiler enforces exhaustiveness. Brain *cfg values* (aggro
radius, attack range) live in RON so a new character can reuse the
same brain with different tuning by editing one row.

## File layout

```ron
(
    brain_presets: {
        "stand_still": StandStill,
        "patrol_peaceful": Patrol(
            spawn_local_x: 0.0, radius: 64.0,
            speed: 28.0, aggressiveness: 0.0,
            aggro_radius: 80.0, attack_range: 0.0,
        ),
        "melee_brute_striker": MeleeBrute(
            aggressiveness: 1.0,
            aggro_radius: 220.0,
            attack_range: 36.0,
            chase_speed: 110.0,
        ),
        // ...
    },
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
        // ...
    },
    characters: {
        "npc_kernel_guide": (
            display_name: "Kernel Guide NPC",
            spritesheet: "sprites/kernel_guide_spritesheet.png",
            manifest: "sprites/kernel_guide_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "patrol_peaceful",
            default_action_set: "peaceful",
            tags: ["hub", "guide"],
        ),
        // ...
    },
)
```

## Runtime shape

The Bevy plugin is `ambition_characters::actor::character_catalog::CharacterCatalogPlugin`; sandbox wires it through `crates/ambition_actors/src/character_roster.rs`.
At app build it:

1. Parses the embedded RON via `include_str!` (synchronous; no
   asset round-trip).
2. Inserts the result as a `CharacterCatalog` resource.
3. Registers a Startup system (`validate_catalog_on_startup`) that
   panics with the joined error list if any catalog entry references
   a missing brain or action-set preset.

The pre-release stance is fail-loud: a typo in the catalog file is a
fatal startup error, not a silent fallback.

## Public API

The catalog resource exposes the runtime queries spawn paths need:

```rust
pub struct CharacterCatalog(pub CharacterCatalogData);

impl CharacterCatalog {
    pub fn get(&self, character_id: &str) -> Option<&CharacterCatalogEntry>;
    pub fn iter(&self) -> impl Iterator<Item = (&String, &CharacterCatalogEntry)>;
    pub fn len(&self) -> usize;
    pub fn build_default_brain(&self, character_id: &str, spawn_world_x: f32) -> Option<Brain>;
    pub fn build_default_action_set(&self, character_id: &str) -> Option<ActionSet>;
}
```

`build_default_brain` adds `spawn_world_x` to `Patrol.spawn_local_x`
to derive the patrol center for `Patrol`-brain characters; for non-
patrol brains the argument is ignored.

A non-Bevy lookup path (`crates/ambition_actors/src/character_roster.rs::EMBEDDED_CATALOG`
+ `display_name_for_character_id`) is available for code that
doesn't have `Res<>` access — the LDtk parser uses it to translate
`NpcSpawn.character_id` into a display name for `Authored.name`.

## How the LDtk parser uses the catalog

LDtk `NpcSpawn` entities carry a
`character_id` field. `convert_npc_spawn` reads it, asks the
embedded catalog for the matching `display_name`, and stamps that on
`Authored.name`. Downstream consumers (sprite lookup, combat banter,
dialog UI) still match on display name today — the catalog drives
*authoring* without forcing every consumer to change shape.

## NPC brain authority (explicit, authorable, runtime-switchable)

An NPC's brain is chosen ONLY by its explicit selection or the character
catalog default — never by inspecting the resulting `Brain`. There is no
"basic brain" classification, no `is_hostile` gate, and no `patrol_radius == 0`
sentinel. The pieces (see
`crates/ambition_characters/src/actor/character_catalog/binding.rs`):

- **`default_brain` (catalog)** — the character's *canonical* behaviour: what it
  normally does when spawned without an override. A talkable NPC placed as an
  `NpcSpawn` uses its default unless the placement overrides it; a character
  whose default is hostile (a fighter) spawns hostile unless overridden.
- **`brain_override` (NpcSpawn placement)** — an optional preset name that sets
  *this instance's* initial brain regardless of the character default. Absent /
  empty means "use the catalog default". A stable preset id is authored (never a
  serialized runtime `Brain`), resolved at spawn by `resolve_initial_brain`
  (precedence: override → catalog default → clear error). Namespace rule: a
  fully-qualified preset (`provider::name`) is used exactly; a raw name resolves
  ONLY within the character's own provider namespace (no silent cross-provider
  fallback). `patrol_radius` PARAMETERIZES a *selected* patrol preset's lane; it
  never selects the brain. A `patrol_path_id` is a **separate** movement
  attachment (`ActorMotionPath`), not a brain-build parameter.
- **`AuthoredBrainContext` (spawn)** — a companion component capturing the actor's
  authored home (spawn anchor + patrol radius). Retained through runtime switches
  and snapshot-safe, so a `RestoreDefault` / reconcile rebuilds a patrol brain
  around where it was AUTHORED, not wherever it wandered.
- **`BrainCommand::UsePreset` (runtime)** — replace an actor's current autonomous
  behaviour with a fresh brain built from a preset, recording the override on its
  `BrainBinding`. The one deterministic, snapshot-safe switch path
  (`crates/ambition_actors/src/features/brain_command.rs`); ordinary gameplay
  never edits the `Brain` component directly. A body under temporary control
  (player possession / mounted) is not switched.
- **`BrainCommand::RestoreDefault` (runtime)** — rebuild the character's canonical
  default brain (a FRESH default; no suspended brain is resumed) and clear the
  override.
- **Disposition is a separate authority.** Brain choice and allegiance are
  distinct. Turning an NPC hostile ("fight me") goes through the existing
  challenge/provocation path (`<<challenge>>` → `ActorStimulus::Challenged` →
  `provoke_actor_in_place`), which installs a hostile brain and flips
  `ActorDisposition`. That path is NON-catalog, so it marks the actor's
  `BrainBinding` as `External`, keeping brain and binding consistent (reconcile
  leaves an externally-owned brain to that authority instead of rebuilding the
  catalog default over it). A dialogue outcome pairs `<<challenge>>` with
  `<<use_brain "attack_preset">>`; "you are free" is `<<restore_brain>>` (→
  `RestoreDefault`).

Every actor spawned from a catalog row carries a `BrainBinding` (its default
preset + current selection: default / override / external) plus an
`AuthoredBrainContext`; both are snapshot-registered. A rewind past a runtime
switch reconciles the live `Brain` back to the binding, comparing *authored
configuration* (not just the variant label, so `wanderer_slow` and `wanderer_fast`
are told apart) and skipping player/mounted/external actors.

Hall of Characters pedestals are ordinary, complete NPC instances that stand
still because each carries an EXPLICIT `brain_override: "stand_still"` — not a
room flag or inferred behaviour. That is why peaceful wanderers (puppy slugs)
hold still on their pedestal while keeping full identity/body/dialogue and the
ability to be switched to another brain later.

## How sprite loading uses the catalog

`load_character_sprites_in` iterates `EMBEDDED_CATALOG.characters`.
For each entry it asks `sheet_for_character_id(cid)` for the
hardcoded `CharacterSheetSpec` const (`PIRATE_SHEET`,
`KERNEL_GUIDE_SHEET`, etc.). Entries without a wired sheet const are
silently skipped — the affected character spawns with the colored-
rectangle fallback.

`sheet_for_character_id` is the only remaining mapping table from
character_id to per-character data. A future cleanup can drive the
sheet spec from the on-disk manifest (`<name>_spritesheet.ron`) at
startup, at which point this function and its match arms disappear.

## Validators / tests

The module ships a tight set of pins:

- `catalog_loads_without_panic` — embedded RON parses cleanly.
- `embedded_catalog_passes_validator` — no internal reference errors.
- `every_renderer_target_has_catalog_entry_or_explicit_exclusion` —
  every renderer-registered character (~87 today) has a catalog
  entry.
- `brain_preset_resolves_to_valid_variant_for_each_entry` — every
  entry's `default_brain` produces a runtime `Brain` value.
- `action_set_preset_resolves_for_each_entry` — same for `default_action_set`.
- `display_name_resolves_for_every_catalog_entry` — the LDtk parser's
  display-name lookup round-trips.
- `validator_reports_missing_brain_preset` — pre-poisoned negative
  test confirming the validator flags broken references.
- `embedded_ldtk_hall_of_characters_has_expected_pedestals` — the
  Hall room references >=80 catalog ids and every reference resolves.

## Authoring path: see [the recipe](../recipes/adding-a-character.md).

## Cross-references

- [`docs/adr/0017-rust-behavior-ron-content-ldtk-space.md`](../adr/0017-rust-behavior-ron-content-ldtk-space.md)
  — the codified architectural posture.
- [`docs/systems/brain-driver.md`](brain-driver.md) — the brain
  seam the catalog feeds into.
- [`docs/recipes/extending-brains-and-action-sets.md`](../recipes/extending-brains-and-action-sets.md)
  — how to add a new brain *variant* (the Rust side).
- `crates/ambition_characters/src/actor/character_catalog/`
  — the live source.
