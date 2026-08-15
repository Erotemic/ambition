# D115 K2/K3/K4 — the discharged case files (2026-08-14)

⚠ **EVIDENCE, NOT AUTHORITY.** Closed case files for D115's ownership carve (K3),
typed path references (K2) and contact completeness (K4), moved out of the live
ledger under its own rule: *when a row closes, remove its historical case file,
preserve useful history in the archive, and continue.* ⛔ do not reconstruct a
deleted representation because this file names it. What is still OPEN on D115 —
K5 and K6 — is stated in
[`../../../planning/queue-72h-2026-08-08.md`](../../../planning/queue-72h-2026-08-08.md).

- ▢ **D115 — Ambition-first LDtk authoring + moving-platform architecture.**

Moving platforms already author from LDtk, so do not rebuild the feature from
scratch. Start by re-measuring the current path from `MovingPlatform` /
`KinematicPath` authored entities to prepared `MovingPlatformSpec` and runtime
`MovingPlatformSet`. Then take the smallest slice that materially improves both
Ambition authoring and reusable engine architecture.

Current target docs:

- [`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md)
- [`engine/kinematic-world-objects.md`](engine/kinematic-world-objects.md)

Prefer typed/native LDtk references and explicit validated motion semantics over
string linkage and optional-field precedence. Preserve current ride/ledge/portal
behavior while moving authoritative kinematic-world state toward a coherent
world/simulation owner. This is the first Engine-1.0 vertical slice because
Ambition needs it directly.

✔ **THE OWNERSHIP CARVE IS CLOSED (2026-08-14), and both halves turned out to be
adoption rather than design.** Step 2: the "construction → presentation seam"
this row was going to invent already existed — every other room feature is drawn
reactively by a render family discovering its own population — so the platform
joined it and `MovingPlatformVisual`, its spawn/sync pair, the commit-time spawn,
the app dressing call and a rollback waiver were all DELETED (plus two
`SessionDressingSetup` fields the compiler then reported dead). Step 4: the
"explicit dynamic-geometry query" already existed as `CollisionWorld`, with three
non-adopters — and one was a LIVE DEFECT (the blink preview resolved against a
world without the ECS overlay or portal carves, so the reticle could disagree
with the blink). The portal host's genuinely different need got a name,
`hostable_surfaces()`. No consumer composes a collision world by hand now.

✔ **K2 IS CLOSED (2026-08-14), and the row's own description of it was wrong in
the most useful way.** The typed reference this row asked for ALREADY EXISTED:
`RoomBindings::sweep` resolves `CharacterBrain::Patrol { path_id }` through
`ambition_binding`'s typed `Ref<KinematicPathId>` / `Resolver`, names the
declaring entity and suggests the nearest spelling. Building typed references
from scratch would have been the expensive mistake.

⭐ **what was actually broken was AGREEMENT ABOUT WHICH SPELLINGS RESOLVE — four
sites deciding it as three different rules**: `KinematicPathSpec::resolution_aliases`
(id, name, name-slug — and self-declared "the ONE authority"), `room_spec_paths`
(id and name **only** — and the table the spawn road actually resolves against),
`content_validation::patrol_name_slug` (plain name slug, and nothing else once an
`id` exists), and conversion's `path_lookup_id` (which derives the id the others
index). ⛔ **this was a LIVE DEFECT in shipped content, not a tidiness argument.**
Sandbox `basement_enemies` authors its path with no `id`, named
`enemy patrol path A`, so conversion derives `enemy_patrol_a` while the placement
says `Patrol:enemy_patrol_path_a`. The sweep resolved it. The content validator
resolved it. The runtime table held only `enemy_patrol_a` / `enemy patrol path A`,
found nothing, and built `ActorMotionPath(None)` — silently. **That room is the
"one body under three controller roles" gallery**, so the exhibit that exists to
prove controllers change behaviour had a patroller that did not patrol.

⛔⛔ **AND THE SENTENCE AFTER THE MEASUREMENT WAS WRONG AGAIN.**
`validate_patrol_brain_paths`'s doc recorded *"zero patrol warnings, so the
mismatch is gone"*. The number was right; the conclusion was not. The warnings
stopped because the brain had been rewritten to the spelling **that validator
derives** — silencing the validator while breaking the runtime. Corrected in
place. This is the fourth time this exact failure shape has been recorded here.

⇒ one alias rule now: `kinematic_path_lookup` generates the runtime table from
`resolution_aliases`, `room_spec_paths` delegates, `patrol_name_slug` is DELETED,
and the sweep additionally covers `InteractionKindSpec::Npc { patrol_path_id }`
and `HazardSpec.path_id`, which shrugged against the same table and were unswept.

✔ **K4 CLOSED 2026-08-14, and three of its four items were already done.**
Passenger carry is unified — a moving solid publishes displacement on
`Block::velocity` and all three sweeps read it off the block they are attached
to, with zero player-marker keying. One-way is explicit and shared —
`BlockKind::OneWay`, `one_way_landing_from_previous_feet`,
`surface_supports_body_at_rest`, drop-through suppression, all gravity-relative
in `collision_semantics` and consumed by both sweeps.

⭐ **the real gap was LEDGE, and it was a FORK — exactly the acceptance line this
plan sets ("no actor-family special case is needed to ride it") being false.**
`integrate_home_body` took `&[MovingPlatformState]` to carry a ledge hang;
`integrate_actor_body` was never handed the platform set. But `ledge_grab` is
kernel state on `AxisManeuverState` **with no player marker**, so an enemy or NPC
could latch onto a moving platform's ledge, be left behind by it, and be dragged
through a wall that the player is knocked off by. ⇒ the carrier was already in
the collision world both roads pass to `step_motion` — a `Block` whose `velocity`
is `last_delta` and whose previous pose is `aabb.translated(-velocity)` — so the
rule became the same sentence as the grounded ride, at ONE kernel site, for every
body, and the parameter that made it player-only DISAPPEARED. `ledge_platform_carry`,
`LedgePlatformCarry`, `matches_ledge_contact{,_in_frame}` and
`ledge_contact_matches_platform` are deleted. The trimming residue is gone too:
`resolve_kinematic_path` is the one reference rule, adopted by both remaining
shruggers. ⭐ the pin's falsifier is the good part — the carrier is authored
`Solid` rather than the `BlinkWall` a platform really becomes, so the knock-off
assertion passes **only if the carrier is excluded by identity**; a kind filter
cannot save it.

⛔ **CRUSH IS A REFUSED WIRE, NOT A MISSING ONE — do not "fix" it.** The canon is
unambiguous (Celeste/SMB: the solid pushes actors, unfittable actors die), but
Ambition's model is inverted: solids expose velocity and bodies carry themselves.
Every resolve site is gated on `is_contact_range_snap` — **Jon's no-artificial-
pushout rule** — so a closing solid leaves a body embedded rather than ejecting
it, and `kinematic/tests.rs:147` *asserts the absence of ejection*. There is no
`ResetCause::Crushed` and that is a decision, not an oversight.

⇒ **four findings handed forward, none absorbed:**

1. ⇒ **promoted to D126.1, REPRODUCED and measured RED 2026-08-14** (16.0px apart
   between the two block orders). ⚠ the live function is `resolve_axis_repair`,
   not `resolve_axis` — that name died with this row's own item 2.
2. **A moving platform cannot be authored one-way.** `as_collision_block`
   hardcodes `BlinkWall{Soft}` on *blink* grounds — a blink concern deciding a
   contact policy. Small slice, but it needs a field on
   `MovingPlatformSpec`/`State`, which is snapshot state (rollback schema
   re-baseline) and possibly RON-authored.
3. **`step_kinematic` has NO production caller** — every ECS actor goes through
   `ae::step_motion` — while several comments still claim enemies route through
   it. Stale duplicated authority.
4. **`ActorControlFrame::drop_through` is a DEAD FIELD**, never mapped in
   `to_input_state()`, so an AI cannot request a drop-through the way its own doc
   says it can.

⇒ what remains on this row: **K5** authoring polish, **K6** a second kinematic
customer.

