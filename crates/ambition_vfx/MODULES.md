# `ambition_vfx` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_vfx** — Reusable effect vocabulary + executor.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`fx`](src/fx.rs) | Authored visual effects are named rows. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_vfx`. |
| [`vfx`](src/vfx.rs) | The visual-effects MESSAGE vocabulary — the presentation-neutral data a simulation system emits to ask for a cue, with NO renderer attached. |

_3 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
