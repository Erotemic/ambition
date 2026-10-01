# `ambition_content_modules` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_content_modules** — Ambition's procedural extension modules.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`echo_fan`](src/echo_fan.rs) | The Mockingbird's echo fan: one strike copies a shot across a cone aimed at the boss's target. |
| [`eye_beam`](src/eye_beam.rs) | The Smirking Behemoth's eye beam: during the telegraph the boss locks where its target is; on the first strike tick it fires a short line of fast bubble-laser boxes from its eye toward that point. |
| [`gradient_nova`](src/gradient_nova.rs) | The gradient nova: on the first tick of a strike, sixteen shots burst out of the boss in a full circle, at three speed tiers so the ring tears into layers. |
| [`mode_collapse`](src/mode_collapse.rs) | Mode collapse: during the telegraph the boss locks where its target is; on the first strike tick a ring of shots appears around that point and converges on it. |
| [`seismic_stomp`](src/seismic_stomp.rs) | The seismic stomp: on the first tick of a strike, a line of damage boxes stands on the floor under the boss's feet, one under the boss and five each side. |
| [`strike`](src/strike.rs) | The strike rules that several boss techniques share. |

_6 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
