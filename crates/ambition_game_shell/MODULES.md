# `ambition_game_shell` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_game_shell** — Top-level game-shell routing without game-specific route names or rendering.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`basic_presentation`](src/basic_presentation.rs) | Plain Bevy UI reference presentation for launchers and shell sequences. |
| [`experience`](src/experience.rs) | Registered top-level experiences: the launcher-visible identity of a provider. |
| [`id`](src/id.rs) | Stable identifiers for shell routes, experiences, holds, and sequence segments. |
| [`input`](src/input.rs) | Narrow neutral shell action adapter shared by startup, launcher, loading, and gameplay-to-home presentation. |
| [`launcher`](src/launcher.rs) | Host-provided launch catalog and the cursor used by the minimal `ambition_menu` adapter. |
| [`plugin`](src/plugin.rs) | Bevy plugins that drive shell routing, sequences, and launcher commands. |
| [`preparation`](src/preparation.rs) | Provider-authored fresh preparation plans and exact prepared-session identity. |
| [`router`](src/router.rs) | Host-relative top-level route lifecycle, pending loads, focus, and scoped cleanup. |
| [`sequence`](src/sequence.rs) | Neutral ordered presentation-sequence data and runtime. |
| [`session`](src/session.rs) | Shell-to-gameplay-session lifecycle bridge. |

_10 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
