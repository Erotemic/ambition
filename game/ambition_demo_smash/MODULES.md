# `ambition_demo_smash` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_demo_smash** — Standalone stocks-based platform-fighter demo.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`bolt`](src/bolt.rs) | The steered bolt, end to end in this game. |
| [`bomb`](src/bomb.rs) | The bomb, end to end in this game. |
| [`capture`](src/capture.rs) | Capture's authored-effect chain, exercised end to end. |
| [`counter`](src/counter.rs) | The counter, end to end in this game. |
| [`dilation`](src/dilation.rs) | Time dilation: a fighter on a slower clock for a while. |
| [`homing`](src/homing.rs) | The homing dash: the fighter is carried at whoever they were pointing at. |
| [`limit`](src/limit.rs) | The game half of the Limit meter: who gains what, and when. |
| [`mark`](src/mark.rs) | The delayed mark, end to end in this game. |
| [`mine`](src/mine.rs) | The remote mine, end to end in this game. |
| [`portal`](src/portal.rs) | The portal recovery, assembled from the portal crate's own parts. |
| [`rules`](src/rules.rs) | The `smash_rules` schema: the demo's own match flow and summon tuning, as data in its pack (`assets/data/smash_rules.ron`). |
| [`select`](src/select.rs) | Pure character-select state for up to four match seats. |
| [`select_screen`](src/select_screen.rs) | Smash character-select presentation and cursor interaction. |
| [`shark_ride`](src/shark_ride.rs) | The pirate's up-special: summon a burning flying shark and ride it. |
| [`sing`](src/sing.rs) | Sing: an area that takes the floor away from whoever stood too close. |
| [`smash_pack`](src/smash_pack.rs) | The Smash demo's content pack. |
| [`stages`](src/stages.rs) | The `smash_stages` schema: Smash's stages, as data in its pack (`assets/data/smash_stages.ron`). |
| [`tether`](src/tether.rs) | The tether reel, end to end in this game. |
| [`text`](src/text.rs) | The lines Smash shows, as data in its pack (`assets/data/smash_text.ron`): the match's cards, the select screen, the pause offer and the launcher row. |

_19 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
