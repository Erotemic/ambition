# `ambition_dialog` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_dialog** — Reusable, content-free dialogue runtime.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`bindings`](src/bindings.rs) | Content-free Yarn binding state, presentation cues, and vocabulary installers. |
| [`bridge`](src/bridge.rs) | Yarn↔DialogState bridge. |
| [`content`](src/content.rs) | Runtime dialogue option data consumed by the UI view model. |
| [`context`](src/context.rs) | Who is talking to whom — the identity context of one conversation. |
| [`continuity`](src/continuity.rs) | What ends a conversation that the world keeps running through. |
| [`runtime`](src/runtime.rs) | `DialogState` — the dialogue UI read model. |
| [`speech_sfx`](src/speech_sfx.rs) | Dialogue typewriter SFX selection and throttling. |
| [`systems`](src/systems.rs) | Dialogue Bevy systems: input translation + the typewriter reveal tick. |

_8 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
