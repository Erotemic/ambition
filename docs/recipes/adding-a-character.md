# Adding a character

> Goal: spawnable in the sandbox via LDtk, with a sprite, a brain,
> and an action set. No Rust changes for the common case.

## The 60-second path (reuses existing brain + sprite)

If the new character behaves like an existing one — a Pirate Cove
goblin variant, a hub NPC with a fresh sprite — the recipe is two
files:

### 1. Add the catalog entry

Edit `crates/ambition_sandbox/assets/data/character_catalog.ron`.
Pick a stable `character_id` (snake_case, `npc_` prefix for NPCs).
Reference an existing brain preset and action-set preset.

```ron
"npc_dock_inspector": (
    display_name: "Dock Inspector",
    spritesheet: "sprites/dock_inspector_spritesheet.png",
    manifest: "sprites/dock_inspector_spritesheet.ron",
    tier: MainHall,
    body_kind: Standard,
    composition: None,
    default_brain: "patrol_peaceful",
    default_action_set: "peaceful",
    tags: ["hub", "dock"],
),
```

### 2. Place a spawner in LDtk

In `sandbox.ldtk` (or whichever level), add an `NpcSpawn` entity
and set its `character_id` field to your new id. That's it — the
runtime queries the catalog for the brain / action set / display
name, and the sprite loader finds the spritesheet via the
catalog's path.

### 3. Generate the sprite (if it's new art)

The sprite generator lives in
`tools/ambition_sprite2d_renderer/`. The shortest path:

```bash
cd tools/ambition_sprite2d_renderer
python -m ambition_sprite2d_renderer install dock_inspector
```

(`install` renders the sheet + writes it under
`crates/ambition_sandbox/assets/sprites/`. `regen_sprites.sh` does
the same in bulk for every registered target.)

### 4. Verify

```bash
~/.cargo/bin/cargo test -p ambition_sandbox --lib content::character_catalog
~/.cargo/bin/cargo run -p ambition_sandbox --bin headless -- --ticks 100
```

The Startup validator flags any catalog inconsistency immediately;
the headless smoke confirms the new character spawns without panic.

## The longer path (new brain template)

A character that needs a brain none of `StandStill` / `Patrol` /
`Wanderer` / `MeleeBrute` / `Skirmisher` / `Sniper` / `BossPattern`
covers needs a Rust patch — but only the brain side. The catalog
flow above still applies.

See [`docs/recipes/extending-brains-and-action-sets.md`](extending-brains-and-action-sets.md)
for the full brain-extension recipe. The short version:

1. Add a variant to `StateMachineCfg` in `crates/ambition_sandbox/src/brain/state_machine.rs`.
2. Add a `tick_<your_brain>` function and dispatch on the enum.
3. Add a `BrainPreset::<YourBrain>` mirror to
   `crates/ambition_sandbox/src/content/character_catalog/entry.rs`.
4. Extend `brain_from_preset` in `resolver.rs` to construct your
   variant from the preset.
5. Register a preset in `character_catalog.ron` (`brain_presets:
   { "your_brain_tuning": YourBrain(...), ... }`).
6. Point your character entry at it via `default_brain:
   "your_brain_tuning"`.

The same shape applies for ActionSet additions (new melee / ranged
variant).

## The Hall of Characters

The Hall of Characters room (`hall_of_characters` in `sandbox.ldtk`)
is auto-generated from the catalog by
`tools/ambition_ldtk_tools/.../generate_hall_of_characters.py`. Re-
running the generator after your edit places a pedestal + label for
the new character automatically:

```bash
PYTHONPATH=tools/ambition_ldtk_tools \
    python -m ambition_ldtk_tools.generate_hall_of_characters
PYTHONPATH=tools/ambition_ldtk_tools \
    python -m ambition_ldtk_tools.area_authoring \
      tools/ambition_ldtk_tools/specs/hall_of_characters_area.ron \
      --replace-existing
```

Walk into the hall from the hub door (in `central_hub_main` at
x=1357, y=880) to confirm visually that the sprite is wired in.

## Common authoring pitfalls

**character_id naming.** Use `snake_case` with `npc_` prefix for NPCs,
no prefix for base characters (`player`, `goblin`, `robot`,
`sandbag`). Boss ids end in `_boss` for readability
(`npc_gnu_ton_boss`).

**Sprite path.** The catalog stores `spritesheet: "sprites/<name>_spritesheet.png"`
— the `sprites/` prefix is part of the path. The loader strips it
when registering with the asset manifest.

**Tier choice.** `Basement` is for visually-big sprites that get a
256 px-wide pedestal in the Hall of Characters. Use it for bosses
(`gnu_ton_boss`, `mockingbird_boss`, `flying_spaghetti_monster_boss`)
and large enemies (`trex_enemy`, `bear_mauler`). Everything else is
`MainHall`.

**Brain preset choice.** Default is `patrol_peaceful` for hub NPCs,
`melee_brute_striker` for melee enemies, `skirmisher_ranger` for
ranged enemies. See `character_catalog.ron` for the full list of
named presets.

**Sheet const wiring.** Until the Phase-7+ migration lands, the
sprite spec (frame size, atlas layout) comes from a hardcoded
`*_SHEET` const in `presentation/character_sprites/sheets.rs`. If
your character uses a fresh art layout, add a row to
`sheet_for_character_id` in `presentation/character_sprites/assets.rs`
to point at the matching const. If your character reuses an
existing layout (most pirates share `PIRATE_SHEET`, most ninjas
share `NINJA_SHEET`), add your `character_id` to the existing
`match` arm.

**Validator failure?** The Startup panic message lists every error.
Common ones:
- `character 'X' default_brain 'Y' not found in brain_presets`
  → either typo `Y` or add it to `brain_presets:`.
- LDtk validate error about character_id → the LDtk file refers to
  a `character_id` that isn't in the catalog. Add the catalog entry
  or fix the LDtk field value.

## Where to read next

- [`docs/systems/character-catalog.md`](../systems/character-catalog.md)
  — the system overview.
- [`docs/adr/0017-rust-behavior-ron-content-ldtk-space.md`](../adr/0017-rust-behavior-ron-content-ldtk-space.md)
  — why this architecture.
- [`docs/recipes/extending-brains-and-action-sets.md`](extending-brains-and-action-sets.md)
  — the brain-variant extension recipe.
- [`docs/systems/brain-driver.md`](../systems/brain-driver.md) — the
  universal brain seam that the catalog feeds into.
