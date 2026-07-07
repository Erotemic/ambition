# Factions

Living index of faction/lair content that is represented in the sandbox. Use this before adding new faction lairs or NPCs so two factions do not accidentally claim the same theme.

## Authoring rules

1. Character identity comes from `crates/ambition_actors/assets/data/character_catalog.ron`.
2. LDtk `NpcSpawn.character_id` should reference a catalog id; display names are resolved through the character roster adapter.
3. Sprite metadata lives under `crates/ambition_actors/assets/sprites/` and the character catalog points at the relevant sheet/manifest.
4. Music cue ids should match the generated-audio catalog / asset manager entries.
5. Update this table in the same patch that adds or materially changes a faction lair.

## Active factions (lair authored, in-game)

| Faction | Theme | Leader / character id | Sprite asset | Dialog id | Music cue | LDtk lair |
|---|---|---|---|---|---|---|
| Iron Resolve | Military bombast — tower of bureaucracy | General / absurd-general lineage | `absurd_general_spritesheet.png` | `military_general` | `fighters_guild_oath_of_steel` | `military_tower` |
| Goblin Cantina | Rowdy training pit — vault tables, duck shelves | Fretjaw, Cantina Chieftain | `goblin_cantina_chieftain_spritesheet.png` | `goblin_cantina_chieftain` | `first_goblin_tune_v2_radio` | `goblin_cantina_lair` |
| Pulse Voyagers | Sky/water dais — hop stepping stones to the captain | Captain Pulse | `pulse_voyager_captain_spritesheet.png` | `pulse_voyager_captain` | `pulse_drift_voyage` | `pulse_voyager_outpost` |
| Tech-Bros Basement | Satirical office — descend through ledges to the boardroom | Chadwick Disruptor III | `tech_bro_disruptor_spritesheet.png` | `tech_bros_disruptor` | `tech_bros_disruption` | `tech_bros_basement` |

## Roadmap factions (sprite/music concept exists, lair pending)

Faction target specs live under `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/configs/factions/` or the current sprite target/config directories. When a roadmap faction becomes active, publish the sprite metadata into `crates/ambition_actors/assets/sprites/`, add or update the character catalog entry, and author the LDtk lair.

| Faction | Theme hook | Leader concept | Music cue |
|---|---|---|---|
| Lofi Drift Hub | Neutral hub, radio desk, tutorial NPCs | Mx. Drift, Radio Host | `long_lofi_drift` |
| Dinosaur Liberators | Rebel garage with banners, fossils, overdriven speakers | Sauria Freebird | `dinosaur_liberators` |
| Watershed Advocates | Watershed HQ, gentle but politically forceful | Solace Reed | `env_advocacy_solace` |
| Glasswood Canopy | Luminous arboretum and high-branch elder court | Old-Light Warden | `glasswood` |
| Moonlit Canal | Nocturne canal dock with reflective platforms | Canal Keeper Noct | `moonlit_canal` |

## Hub NPCs

Hub NPCs are not necessarily factions. They use the same character catalog + sprite manifest flow, so static `NpcTerminal` placeholders can be replaced without inventing a parallel registry.

| LDtk / catalog identity | Sprite asset | Dialog id |
|---|---|---|
| Architect NPC | `architect_spritesheet.png` | `architect_intro` |
| Kernel Guide NPC | `kernel_guide_spritesheet.png` | `hub_guide` |
| Vault Keeper NPC | `vault_keeper_spritesheet.png` | `vault_keeper` |
| Merchant Prototype NPC | `merchant_prototype_spritesheet.png` | `merchant_seed` |
