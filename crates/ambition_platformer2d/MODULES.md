# `ambition_platformer2d` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d** — Public facade for Ambition-derived platformer games.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`app`](src/app.rs) | **Standing up a game.** The engine owns composition ordering; the consumer states policy. |
| [`game_assets`](src/game_assets.rs) | **The asset install a visible game needs before anything draws.** |
| [`prelude`](src/prelude.rs) | Curated imports for games built on the Ambition engine facade. |
| [`rollback`](src/rollback.rs) | **Rollback, as a supported promise.** |
| [`session_world`](src/session_world.rs) | Canonical live session-world surface. |

_5 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
