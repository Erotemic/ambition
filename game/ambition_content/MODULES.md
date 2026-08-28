# `ambition_content` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_content** — THE named Ambition game content — everything that names this game's specific world: quests, bosses, items, dialogue, banter, the intro, the enemy roster, music cues, and the cross-content validator.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`actor_moveset`](src/actor_moveset.rs) | The Actor — the sword archetype's table, with four specials of her own. |
| [`alice_moveset`](src/alice_moveset.rs) | Alice's repertoire — the cryptographer, and the one who SENDS. |
| [`archetype_moveset`](src/archetype_moveset.rs) | One fighter borrowing another's TIMINGS, under its own name. |
| [`audio_registries`](src/audio_registries.rs) | Ambition's authored audio registries — CONTENT data, evicted from the engine core (R3.2: the engine ships no tracks and no cues). |
| [`author_moveset`](src/author_moveset.rs) | The Author — the sword archetype's table, wielded with a pen. |
| [`authored`](src/authored/mod.rs) | Every character this provider AUTHORS, one file each. |
| [`authored_movesets`](src/authored_movesets.rs) | Every moveset THIS CRATE authors, in one list. |
| [`banter`](src/banter.rs) | Ambition's authored combat-banter lines. |
| [`bob_moveset`](src/bob_moveset.rs) | Bob's repertoire — the engineer, and the one who RECEIVES. |
| [`bosses`](src/bosses/mod.rs) | Named Ambition boss content registration. |
| [`carl_stargan_moveset`](src/carl_stargan_moveset.rs) | Carl Stargan moveset. |
| [`cellular_automaton_moveset`](src/cellular_automaton_moveset.rs) | The Perfect Cellular Automaton's signature move, authored as data. |
| [`character_catalog`](src/character_catalog.rs) | Ambition's character-catalog DATA + the curated playable cast — CONTENT, evicted from the engine core (R3.2, violations #3 and #10). |
| [`content_validation`](src/content_validation.rs) | Cross-content validation for authored sandbox data. |
| [`dialogue`](src/dialogue/mod.rs) | Named Ambition dialogue / cutscene content registration. |
| [`dormancy`](src/dormancy.rs) | Dormancy policy for Ambition-authored actors. |
| [`duel_arena`](src/duel_arena.rs) | Spectator-duel CONTENT — the PCA-vs-robot exhibition fight (R3.3: room mechanics split by kind; this one is a `RoomLoaded` consumer). |
| [`emmy_noether_moveset`](src/emmy_noether_moveset.rs) | Emmy Ethereal's authored Smash repertoire. |
| [`encounters`](src/encounters.rs) | Content encounter customers on the GENERIC lifecycle (E13). |
| [`falling_sand`](src/falling_sand.rs) | Falling-sand room PRESENTATION + `bevy_falling_sand` bridge for water/oil — CONTENT (a self-gating room plugin: feature-gated, visible-binary only, active only while its authored room is; R3.3 room-mechanics-by-kind). |
| [`falling_sand_sim`](src/falling_sand_sim.rs) | Falling-sand room SIMULATION — the deterministic, headless-safe half. |
| [`goblin_moveset`](src/goblin_moveset.rs) | Goblin-authored platform-fighter repertoire. |
| [`input_techniques`](src/input_techniques.rs) | Ambition-owned motion-input technique registrations. |
| [`intro`](src/intro/mod.rs) | Intro sequence story content. |
| [`items`](src/items/mod.rs) | Named Ambition item-roster / default-inventory registration. |
| [`medic_moveset`](src/medic_moveset.rs) | The Medic — the brawler archetype's normals, under her own name, and four specials that are hers. |
| [`music`](src/music.rs) | Ambition's authored music-cue catalog + encounter bindings. |
| [`ninja_shadow_oni_leader_moveset`](src/ninja_shadow_oni_leader_moveset.rs) | Shadow Oni Leader moveset. |
| [`officer_moveset`](src/officer_moveset.rs) | The Officer — the brawler archetype's table, under his own name, plus the one move that is his. |
| [`oiler_moveset`](src/oiler_moveset.rs) | Oiler's authored Smash repertoire. |
| [`pack`](src/pack.rs) | Ambition's own content pack — the compile that IS the load path. |
| [`patent_clerk_moveset`](src/patent_clerk_moveset.rs) | Patent Clerk's authored Smash repertoire. |
| [`pirate_admiral_moveset`](src/pirate_admiral_moveset.rs) | Pirate Admiral's authored Smash repertoire. |
| [`player_robot_lineage`](src/player_robot_lineage.rs) | Player Robot incarnations generated from shared source. |
| [`player_robot_moveset`](src/player_robot_moveset.rs) | The player robot's canonical move repertoire — the moves that ARE the protagonist, wherever it is seated. |
| [`plugin`](src/plugin.rs) | Ambition game-content registration. |
| [`pointed_polygon_moveset`](src/pointed_polygon_moveset.rs) | Pointed Polygon — sword archetype repertoire. |
| [`portal`](src/portal/mod.rs) | Ambition-specific portal adapters. |
| [`presentation`](src/presentation/mod.rs) | Content-owned presentation plugins — named Ambition looks layered onto the reusable renderer's PUBLIC seams. |
| [`projectile_polygon_moveset`](src/projectile_polygon_moveset.rs) | Projectile Polygon — ranged beast-biped fundamentals repertoire. |
| [`projectiles`](src/projectiles.rs) | Ambition-owned projectile visual registrations. |
| [`provider`](src/provider.rs) | Reusable Ambition gameplay provider. |
| [`pugnacious_polygon_moveset`](src/pugnacious_polygon_moveset.rs) | Pugnacious Polygon — brawler archetype repertoire. |
| [`quest`](src/quest.rs) | Ambition's authored quests + their completion payouts. |
| [`quests`](src/quests/mod.rs) | Named Ambition quest content registration. |
| [`special_slots`](src/special_slots.rs) | Replacing one special in a table a fighter BORROWED. |
| [`worlds`](src/worlds.rs) | Ambition's LDtk WORLD payload + its `WorldManifest` — CONTENT, evicted from the engine core (R3.2, the #1 violation: the engine shipped the game's worlds). |
| [`yarn_vocabulary`](src/yarn_vocabulary.rs) | Yarn command, function, and markup registrations available to authored `.yarn` content. |

_48 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
