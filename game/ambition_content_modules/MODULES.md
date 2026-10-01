# `ambition_content_modules` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_content_modules** — Ambition's procedural extension modules.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`echo_fan`](src/echo_fan.rs) | The Mockingbird's echo fan: one strike copies a shot across a cone aimed at the boss's target. |
| [`eye_beam`](src/eye_beam.rs) | The Smirking Behemoth's eye beam: during the telegraph the boss locks where its target is; on the first strike tick it fires a short line of fast bubble-laser boxes from its eye toward that point. |
| [`locked_strike`](src/locked_strike.rs) | The "lock the target during the telegraph, fire once per strike" rule that several boss techniques share. |
| [`mode_collapse`](src/mode_collapse.rs) | Mode collapse: during the telegraph the boss locks where its target is; on the first strike tick a ring of shots appears around that point and converges on it. |

_4 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
