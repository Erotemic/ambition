# `ambition_content_modules` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_content_modules** — Ambition's procedural extension modules.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`apple_rain`](src/apple_rain.rs) | Apple rain: while the boss presses the key, an apple falls every interval of gameplay time, its lane spread across the boss's room by a golden-ratio sequence and moved out from under the boss. |
| [`beam`](src/beam.rs) | Focus Beam: Attack while holding the beam fires a short line of damage along the aim, snapped to the body's horizontal or vertical axis. |
| [`blink`](src/blink.rs) | Blink: Attack while holding the blink moves the body at once up to [`DISTANCE`] along the aim, walls permitting, and strikes where it arrives. |
| [`dive`](src/dive.rs) | Overflow Crash: Attack while holding the dive gauntlet lunges the body up to [`LUNGE`] along the aim, snapped to its larger body axis, walls permitting, and hits everything in the corridor it crossed. |
| [`echo_fan`](src/echo_fan.rs) | The Mockingbird's echo fan: one strike copies a shot across a cone aimed at the boss's target. |
| [`eye_beam`](src/eye_beam.rs) | The Smirking Behemoth's eye beam: during the telegraph the boss locks where its target is; on the first strike tick it fires a short line of fast bubble-laser boxes from its eye toward that point. |
| [`fsm`](src/fsm.rs) | The Flying Spaghetti Monster's conductor: it flies the god and performs its moves. |
| [`gradient_cascade`](src/gradient_cascade.rs) | Gradient cascade: on the first strike tick, minions drop in from the top of the arena, spread evenly about the boss. |
| [`gradient_nova`](src/gradient_nova.rs) | The gradient nova: on the first tick of a strike, sixteen shots burst out of the boss in a full circle, at three speed tiers so the ring tears into layers. |
| [`mark_recall`](src/mark_recall.rs) | Mark / Recall: while the body holds the mark/recall item, Attack puts its mark where it stands, and Blink takes it back to the mark at once and strikes there. |
| [`meteor`](src/meteor.rs) | Meteor: Attack while holding the meteor drops a line of falling rocks on a zone ahead of the body, across its gravity. |
| [`minima_trap`](src/minima_trap.rs) | Minima trap: on the first strike tick a pit of damage opens where the boss's target is, and a crawler appears beside it, on the boss's side. |
| [`mockingbird`](src/mockingbird.rs) | The Mockingbird's conductor: the air chase. |
| [`mode_collapse`](src/mode_collapse.rs) | Mode collapse: during the telegraph the boss locks where its target is; on the first strike tick a ring of shots appears around that point and converges on it. |
| [`overfit_volley`](src/overfit_volley.rs) | Overfit volley: during the telegraph the boss memorises where its target is, once at the start and then every interval of gameplay time; on the first strike tick it fires one bolt at each memorised point. |
| [`overflow_flood`](src/overflow_flood.rs) | Overflow's boundary flood: during the telegraph the boss locks where its target is (the safe lane); on the first strike tick shots fall from above in every column of the boss's room except that lane. |
| [`saddle_point`](src/saddle_point.rs) | Saddle point: on the first strike tick a damage arm appears across the boss, horizontal; every period the arm turns (vertical, then horizontal again), each new arm where the boss is at that moment. |
| [`seismic_stomp`](src/seismic_stomp.rs) | The seismic stomp: on the first tick of a strike, a line of damage boxes stands on the floor under the boss's feet, one under the boss and five each side. |
| [`sentry`](src/sentry.rs) | Sentry: Attack while holding the sentry gauntlet drops a turret at the body. |
| [`shockwave`](src/shockwave.rs) | Shockwave Slam: Attack while holding the shockwave gauntlet slams a damage box around the wielder. |
| [`strike`](src/strike.rs) | The strike rules that several boss techniques share. |
| [`trex`](src/trex.rs) | The Tyrant King's conductor: it walks the T-rex and performs his moves. |
| [`volley`](src/volley.rs) | Volley: Attack while holding the volley fires a fan of bolts along the aim from the body's edge. |
| [`vortex`](src/vortex.rs) | Vortex: Attack while holding the vortex gauntlet opens a singularity ahead of the body along the aim. |
| [`wielded`](src/wielded.rs) | What the wielded abilities share: the descriptor of a stateless entry on the `wielded_use` trigger, and the payment rule. |

_25 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
