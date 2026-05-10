# Factions

Living index of every faction the sandbox has authored content for —
each row links a faction's identity to its leader sprite, music cue,
LDtk lair (if any), runtime hookups, and theme direction. Use this as
the source of truth for "what's already represented" before adding
new lairs or NPCs so we don't end up with two factions claiming the
same theme.

When adding a faction:

1. Pick a row in **Active** (in-game today) or **Roadmap** (sprite +
   music ready, lair pending).
2. The display name on the LDtk `NpcSpawn` field must match the key
   in `NPC_SPRITE_REGISTRY` (`crates/ambition_sandbox/src/character_sprites/assets.rs`).
3. The level's `music_track` field is the cue id from
   `crates/ambition_sandbox/assets/ambition/sandbox.ron`'s
   `music_tracks` list.
4. Update this table in the same patch that adds the lair.

## Active factions (lair authored, in-game)

| Faction        | Theme                                  | Leader (LDtk name)               | Sprite asset                                  | Dialog id                  | Music cue                       | LDtk lair                |
| -------------- | -------------------------------------- | -------------------------------- | --------------------------------------------- | -------------------------- | ------------------------------- | ------------------------ |
| Iron Resolve   | Military bombast — tower of bureaucracy | `General`                        | `absurd_general_spritesheet.png`              | `military_general`         | `fighters_guild_oath_of_steel`  | `military_tower`         |
| Goblin Cantina | Rowdy training pit — vault tables, duck shelves | `Fretjaw, Cantina Chieftain` | `goblin_cantina_chieftain_spritesheet.png` | `goblin_cantina_chieftain` | `first_goblin_tune_v2_radio`    | `goblin_cantina_lair`    |
| Pulse Voyagers | Sky/water dais — hop stepping stones to the captain | `Captain Pulse`           | `pulse_voyager_captain_spritesheet.png`       | `pulse_voyager_captain`    | `pulse_drift_voyage`            | `pulse_voyager_outpost`  |
| Tech-Bros Basement | Satirical office — descend through ledges to the boardroom | `Chadwick Disruptor III` | `tech_bro_disruptor_spritesheet.png`        | `tech_bros_disruptor`      | `tech_bros_disruption`          | `tech_bros_basement`     |

Notes:

- **Iron Resolve** and the **General** share the same faction. The
  generator archetype id is `absurd_general` (kept in code as
  `ABSURD_GENERAL_SHEET`); the in-game display name is just `General`
  to keep the dialog from feeling like a one-note joke. There is no
  separate "Iron Resolve" lair — `military_tower` is its lair.
- **Goblin Cantina** uses the radio-mix of the first-goblin-tune
  cue. The adaptive boss-encounter version of the same cue still
  drives the `mob_lab` encounter; the cantina is a peaceful lair.

## Roadmap factions (sprite + music ready, lair pending)

These have a faction-leader sheet under
`tools/ambition_sprite2d_renderer/generated/factions/` and a
matching music cue in the radio, but no LDtk lair yet. The
`tools/ambition_sprite2d_renderer/generated/factions/faction_lineup_manifest.yaml`
is the upstream manifest these come from.

| Faction              | Theme hook                                          | Leader (manifest name) | Sprite asset                                  | Music cue                |
| -------------------- | --------------------------------------------------- | ---------------------- | --------------------------------------------- | ------------------------ |
| Lofi Drift Hub       | Neutral hub, radio desk, tutorial NPCs              | Mx. Drift, Radio Host  | `lofi_radio_host_spritesheet.png`             | `long_lofi_drift`        |
| Dinosaur Liberators  | Rebel garage with banners, fossils, overdriven speakers | Sauria Freebird     | `dinosaur_liberator_frontbot_spritesheet.png` | `dinosaur_liberators`    |
| Watershed Advocates  | Watershed HQ, gentle but politically forceful       | Solace Reed            | `watershed_advocate_spritesheet.png`          | `env_advocacy_solace`    |
| Glasswood Canopy     | Luminous arboretum and high-branch elder court      | Old-Light Warden       | `glasswood_warden_spritesheet.png`            | `glasswood`              |
| Moonlit Canal        | Nocturne canal dock with reflective platforms       | Canal Keeper Noct      | `moonlit_canal_keeper_spritesheet.png`        | `moonlit_canal`          |

## Hub NPCs (not factions, but live in `central_hub_*`)

These existed in LDtk before any faction work — they get
faction-appropriate sheets through the same `NPC_SPRITE_REGISTRY`
table so their static `NpcTerminal` placeholders don't feel like
placeholders anymore.

| LDtk name              | Sprite asset                            | Dialog id          |
| ---------------------- | --------------------------------------- | ------------------ |
| Architect NPC          | `architect_spritesheet.png`             | `architect_intro`  |
| Kernel Guide NPC       | `kernel_guide_spritesheet.png`          | `hub_guide`        |
| Vault Keeper NPC       | `vault_keeper_spritesheet.png`          | `vault_keeper`     |
| Merchant Prototype NPC | `merchant_prototype_spritesheet.png`    | `merchant_seed`    |
