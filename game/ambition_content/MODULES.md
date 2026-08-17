# `ambition_content` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_content** — THE named Ambition game content — everything that names this game's specific world: quests, bosses, items, dialogue, banter, the intro, the enemy roster, music cues, and the cross-content validator.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`alice_moveset`](src/alice_moveset.rs) | **Alice's repertoire** — the cryptographer, and the one who SENDS. |
| [`audio_registries`](src/audio_registries.rs) | Ambition's authored audio registries — CONTENT data, evicted from the engine core (R3.2: the engine ships no tracks and no cues). |
| [`authored`](src/authored/mod.rs) | **Every character this provider AUTHORS, one file each.** |
| [`banter`](src/banter.rs) | Ambition's authored combat-banter lines. |
| [`bob_moveset`](src/bob_moveset.rs) | **Bob's repertoire** — the engineer, and the one who RECEIVES. |
| [`bosses`](src/bosses/mod.rs) | Named Ambition boss content registration. |
| [`carl_stargan_moveset`](src/carl_stargan_moveset.rs) | **Carl Stargan's repertoire** — cosmic perspective, as a fighter. |
| [`cellular_automaton_moveset`](src/cellular_automaton_moveset.rs) | **The Perfect Cellular Automaton's signature move**, authored as data. |
| [`character_catalog`](src/character_catalog.rs) | Ambition's character-catalog DATA + the curated playable cast — CONTENT, evicted from the engine core (R3.2, violations #3 and #10). |
| [`content_validation`](src/content_validation.rs) | Cross-content validation for authored sandbox data. |
| [`dialogue`](src/dialogue/mod.rs) | Named Ambition dialogue / cutscene content registration. |
| [`dormancy`](src/dormancy.rs) | **Which of Ambition's named cast may stop thinking — declared, per actor.** |
| [`duel_arena`](src/duel_arena.rs) | Spectator-duel CONTENT — the PCA-vs-robot exhibition fight (R3.3: room mechanics split by kind; this one is a `RoomLoaded` consumer). |
| [`encounters`](src/encounters.rs) | Content encounter customers on the GENERIC lifecycle (E13). |
| [`falling_sand`](src/falling_sand.rs) | Falling-sand room PRESENTATION + `bevy_falling_sand` bridge for water/oil — CONTENT (a self-gating room plugin: feature-gated, visible-binary only, active only while its authored room is; R3.3 room-mechanics-by-kind). |
| [`falling_sand_sim`](src/falling_sand_sim.rs) | Falling-sand room SIMULATION — the deterministic, headless-safe half. |
| [`goblin_moveset`](src/goblin_moveset.rs) | **The goblin's repertoire** — the third character in the game to state its own moves, and the first ENEMY to. |
| [`input_techniques`](src/input_techniques.rs) | Ambition-owned motion-input technique registrations. |
| [`intro`](src/intro/mod.rs) | Intro sequence story content. |
| [`items`](src/items/mod.rs) | Named Ambition item-roster / default-inventory registration. |
| [`music`](src/music.rs) | Ambition's authored music-cue catalog + encounter bindings. |
| [`ninja_shadow_oni_leader_moveset`](src/ninja_shadow_oni_leader_moveset.rs) | **The Shadow Oni Leader's repertoire** — the counter-puncher, written from his own barks. |
| [`emmy_noether_moveset`](src/emmy_noether_moveset.rs) | **Emmy Ethereal's repertoire** — a theorem, as a fighter. |
| [`oiler_moveset`](src/oiler_moveset.rs) | **Oiler's repertoire** — the maintenance mechanic, as a fighter. |
| [`pack`](src/pack.rs) | Ambition's own content pack — the compile that IS the load path. |
| [`patent_clerk_moveset`](src/patent_clerk_moveset.rs) | **The Patent Clerk's repertoire** — the heavyweight, written from the row's own `gameplay_description` rather than from taste. |
| [`pirate_admiral_moveset`](src/pirate_admiral_moveset.rs) | **The Pirate Admiral's repertoire** — a cutlass, and the reach that comes with carrying one. |
| [`player_robot_lineage`](src/player_robot_lineage.rs) | **The player robot's incarnations, emitted from one source.** |
| [`player_robot_moveset`](src/player_robot_moveset.rs) | **The player robot's canonical move repertoire** — the moves that ARE the protagonist, wherever it is seated. |
| [`plugin`](src/plugin.rs) | [`AmbitionContentPlugin`] — named Ambition game-content registration. |
| [`portal`](src/portal/mod.rs) | Ambition-specific portal adapters. |
| [`presentation`](src/presentation/mod.rs) | Content-owned presentation plugins — named Ambition looks layered onto the reusable renderer's PUBLIC seams. |
| [`projectiles`](src/projectiles.rs) | Ambition-owned projectile visual registrations. |
| [`provider`](src/provider.rs) | Reusable Ambition gameplay provider. |
| [`quest`](src/quest.rs) | Ambition's authored quests + their completion payouts. |
| [`quests`](src/quests/mod.rs) | Named Ambition quest content registration. |
| [`worlds`](src/worlds.rs) | Ambition's LDtk WORLD payload + its `WorldManifest` — CONTENT, evicted from the engine core (R3.2, the #1 violation: the engine shipped the game's worlds). |
| [`yarn_vocabulary`](src/yarn_vocabulary.rs) | Yarn command + function + markup registrations — the "vocabulary" that authored `.yarn` content can invoke at runtime. |

_38 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
