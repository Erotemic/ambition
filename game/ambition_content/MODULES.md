# `ambition_content` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_content** — THE named Ambition game content — everything that names this game's specific world: quests, bosses, items, dialogue, banter, the intro, the enemy roster, music cues, and the cross-content validator.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`audio_registries`](src/audio_registries.rs) | Ambition's authored audio registries — CONTENT data, evicted from the engine core (R3.2: the engine ships no tracks and no cues). |
| [`banter`](src/banter.rs) | Ambition's authored combat-banter lines. |
| [`bosses`](src/bosses/mod.rs) | Named Ambition boss content registration. |
| [`character_catalog`](src/character_catalog.rs) | Ambition's character-catalog DATA + the curated playable cast — CONTENT, evicted from the engine core (R3.2, violations #3 and #10). |
| [`content_validation`](src/content_validation.rs) | Cross-content validation for authored sandbox data. |
| [`dialogue`](src/dialogue/mod.rs) | Named Ambition dialogue / cutscene content registration. |
| [`duel_arena`](src/duel_arena.rs) | Spectator-duel CONTENT — the PCA-vs-robot exhibition fight (R3.3: room mechanics split by kind; this one is a `RoomLoaded` consumer). |
| [`encounters`](src/encounters.rs) | Content encounter customers on the GENERIC lifecycle (E13). |
| [`enemy_roster`](src/enemy_roster.rs) | THE Ambition hostile-archetype roster — named, authored game data. |
| [`falling_sand`](src/falling_sand.rs) | Falling-sand room PRESENTATION + `bevy_falling_sand` bridge for water/oil — CONTENT (a self-gating room plugin: feature-gated, visible-binary only, active only while its authored room is; R3.3 room-mechanics-by-kind). |
| [`falling_sand_sim`](src/falling_sand_sim.rs) | Falling-sand room SIMULATION — the deterministic, headless-safe half. |
| [`input_techniques`](src/input_techniques.rs) | Ambition-owned motion-input technique registrations. |
| [`intro`](src/intro/mod.rs) | Intro sequence story content. |
| [`items`](src/items/mod.rs) | Named Ambition item-roster / default-inventory registration. |
| [`music`](src/music.rs) | Ambition's authored music-cue catalog + encounter bindings. |
| [`plugin`](src/plugin.rs) | [`AmbitionContentPlugin`] — named Ambition game-content registration. |
| [`portal`](src/portal/mod.rs) | Ambition-specific portal adapters. |
| [`presentation`](src/presentation/mod.rs) | Content-owned presentation plugins — named Ambition looks layered onto the reusable renderer's PUBLIC seams. |
| [`projectiles`](src/projectiles.rs) | Ambition-owned projectile visual registrations. |
| [`provider`](src/provider.rs) | Reusable Ambition gameplay provider. |
| [`quest`](src/quest.rs) | Ambition's authored quests + their completion payouts. |
| [`quests`](src/quests/mod.rs) | Named Ambition quest content registration. |
| [`vanity_card`](src/vanity_card.rs) | Ambition's startup vanity card: the authored "I MADE THIS" comic beat. |
| [`worlds`](src/worlds.rs) | Ambition's LDtk WORLD payload + its `WorldManifest` — CONTENT, evicted from the engine core (R3.2, the #1 violation: the engine shipped the game's worlds). |

_24 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
