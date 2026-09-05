# `ambition_demo_smash` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_demo_smash** — Standalone stocks-based platform-fighter demo.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`bomb`](src/bomb.rs) | A live bomb on the stage: the ruleset half of `smash.drop_bomb`. |
| [`capture`](src/capture.rs) | The Smash ruleset's capture adapter: authored effect keys → typed requests. |
| [`counter`](src/counter.rs) | The counter, assembled from parts the engine already had. |
| [`george_booul_moveset`](src/george_booul_moveset.rs) | George Booul's authored fighter repertoire. |
| [`moveset`](src/moveset.rs) | Shared authored platform-fighter repertoire for demo fighters that do not provide a character-owned table. |
| [`portal`](src/portal.rs) | The portal recovery, assembled from the portal crate's own parts. |
| [`select`](src/select.rs) | Pure character-select state for up to four match seats. |
| [`select_screen`](src/select_screen.rs) | Smash character-select presentation and cursor interaction. |
| [`shark_ride`](src/shark_ride.rs) | The pirate's up-special: summon a burning flying shark and ride it. |
| [`sing`](src/sing.rs) | Sing: an area that takes the floor away from whoever stood too close. |
| [`smash_pack`](src/smash_pack.rs) | Smash demo content pack for George Booul. |

_11 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
