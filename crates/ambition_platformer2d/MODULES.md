# `ambition_platformer2d` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d** — Public facade for Ambition-derived platformer games.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`app`](src/app.rs) | High-level platformer application composition. |
| [`autopilot`](src/autopilot.rs) | A seat that navigation drives: the player's own body goes to a goal by the room's surface graph, with the input a person would give. |
| [`demo_shell`](src/demo_shell.rs) | The launcher conventions a demo's own standalone shell speaks. |
| [`game_assets`](src/game_assets/mod.rs) | Install the asset resources required by generic platformer presentation. |
| [`prelude`](src/prelude.rs) | Curated imports for games built on the Ambition engine facade. |
| [`rollback`](src/rollback.rs) | Supported rollback session entry points. |
| [`scripted_input`](src/scripted_input.rs) | Drive scripted local input through the production participant pipeline. |
| [`session`](src/session.rs) | Canonical live session-world surface. |

_8 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
