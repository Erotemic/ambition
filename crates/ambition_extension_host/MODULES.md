# `ambition_extension_host` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_extension_host** — The Bevy host for procedural extension modules (fast-iteration I4).

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`admission`](src/admission.rs) | Admission: match each module's requirements against the ports this composition installed, and fix one serial entry order. |
| [`exec`](src/exec.rs) | Serial invocation: triggers in, staged state and requests out. |
| [`inspect`](src/inspect.rs) | Text views of the extension host, for a developer or an agent that has no access to the `World` (fast-iteration I7 item 5). |
| [`native_build`](src/native_build.rs) | Native code identity is read once, before simulation or preparation. |
| [`observer_delivery`](src/observer_delivery.rs) | Transient routing policy for typed native observers. |
| [`reload`](src/reload.rs) | Replacing a loaded module while the game runs. |
| [`save`](src/save.rs) | The records a save road keeps, as save rows, and their adoption on a load. |
| [`store`](src/store.rs) | The host-owned stores for module state. |

_8 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
