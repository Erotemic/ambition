# `ambition_dialog` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_dialog** — Reusable dialogue runtime (E1c carve out of `ambition_platformer2d_actor_monolith`).

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`bindings`](src/bindings.rs) | Generic Yarn binding machinery — the reusable half of the old `dialog/yarn_bindings.rs`. |
| [`bridge`](src/bridge.rs) | Yarn↔DialogState bridge. |
| [`content`](src/content.rs) | Dialogue content types — minimal post-Yarn migration. |
| [`context`](src/context.rs) | **Who is talking to whom** — the identity context of one conversation. |
| [`runtime`](src/runtime.rs) | `DialogState` — the dialogue UI read model. |
| [`speech_sfx`](src/speech_sfx.rs) | Dialogue typewriter SFX selection and throttling. |
| [`systems`](src/systems.rs) | Dialogue Bevy systems: input translation + the typewriter reveal tick. |

_7 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
