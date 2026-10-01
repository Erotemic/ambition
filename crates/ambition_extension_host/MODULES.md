# `ambition_extension_host` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_extension_host** — The Bevy host for procedural extension modules (fast-iteration I4).

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`admission`](src/admission.rs) | Admission: match each module's requirements against the ports this composition installed, and fix one serial entry order. |
| [`exec`](src/exec.rs) | Serial invocation: triggers in, staged state and requests out. |
| [`reload`](src/reload.rs) | Replacing a loaded module while the game runs. |
| [`store`](src/store.rs) | The host-owned store for module state attached to a body. |

_4 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
