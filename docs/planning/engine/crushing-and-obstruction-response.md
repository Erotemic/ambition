# Crushing, compression and motion authority

**Status:** Q187 is resolved at the architectural level (Jon, 2026-11-26).
The design below follows from the ruling. Two things are **not** decided
and need design work before C4: the numerical model (how authority and
resistance are measured and compared) and the deterministic fallback for
constraints that no response can reconcile. C0–C3 and C5 landed
2026-11-26 (receipts under [Stages](#stages)); C4 (which must first make a
full step report a press closing on a standing body, found by C0) and C6
are open. Work: queue CRUSH.

## The ruling

Crushing and obstruction response is one engine system, not a portal
policy.

- **Displacement** when a body has room to escape.
- **Directional squish and flattening** of compressible bodies, which can
  damage or kill.
- **Bodies differ** in compressibility, durability and material failure.
  Some survive being flattened; incompressible bodies resist deformation.
- **Moving surfaces differ** in motion authority: drive strength, mass,
  momentum and resistance to obstruction. A surface stalls when it meets
  enough resistance. A high-authority surface displaces, crushes or ejects
  what obstructs it instead of stopping.
- **Velocity alone does not decide priority.** A slow hydraulic press can
  exert great force.
- **A portal on a moving train does not stop the train** because an
  ordinary object straddles the portal: the object is displaced, crushed,
  destroyed or ejected. A small force-limited platform may stop.
- **Constraints transmitted through portals** take part in the same
  resolution as ordinary local constraints. Closing the portal is not the
  default response.
- **Configurable by game, body and surface.** Mary-O keeps its behaviour:
  when a moving one-way platform raises the player into a ceiling, the
  player falls through the platform. Ambition generally crushes.
- **Not a gate on portal work.** M1, M2 and the other portal work proceed
  without the whole system ([portals are moving
  frames](portals-are-moving-frames.md)).

## The current state

- **The kernel reports a crush and resolves none.** When the solids that
  claim a body admit no position on an axis, the sweep returns an
  `AxisConstraintConflict` (`ambition_platformer2d_core::collision_semantics`):
  the axis, the two bounds (`min_center > max_center`), the two claiming
  sources, and `overconstraint()` in pixels. It rides out on
  `FrameEvents::constraint_conflicts` (`movement/events.rs`), and the
  kernel keeps the body where no surface accepts it
  (`movement/collision.rs::resolve_material_axis_repair`). Nothing consumes
  the events: this was deliberate ([kinematic world
  objects](kinematic-world-objects.md): "Crush is reported, not resolved").
- **Platforms never hear back.** `MovingPlatformState::update` advances a
  platform by its motion law alone and records `last_delta`; riders are
  carried by it. No law takes a refusal, so nothing can stall.
- **A portal host moves first and the portal follows**
  (`portal_world/hosts.rs::refresh_hosted_portal_frames`). Nothing checks
  material that is part-way through the portal
  ([driven aperture motion](portals-are-moving-frames.md#driven-aperture-motion-is-admitted-before-the-host-commits)).
- **A body's size is already a material.** `BodyKinematics::material`
  (`BodyMaterial`) carries the body's scale and orientation, and every
  collision reader uses it. A squash along one axis is a change of
  material scale, not a new shape system.
- **Mary-O's one-way behaviour** (a moving one-way platform raises the
  player into a ceiling, and the player falls through the platform) is
  reported by the maintainer. No test pins it,
  and its mechanism has not been read; C0 does both before anything
  changes.

## Terms

- **Obstruction:** a body that a moving surface's proposed motion would
  put in a place no claiming surface admits. The kernel's
  `AxisConstraintConflict` is one obstruction on one axis.
- **Driver:** the claiming source that moved into the body this step.
  The driver is identified by relative motion (which surface closed on the
  body), but that only says *who* pushes; how hard it pushes is its
  authority, which is not its speed. A conflict between two still
  surfaces has no driver: the body moved itself into it, which is the
  body's own motion refusal, not a crush.
- **Motion authority** (per surface): drive strength (the force its motion
  law can apply), mass and momentum (what it carries into the contact),
  and its resistance to being obstructed. A press is slow and strong; a
  conveyor plank is fast and weak.
- **Resistance** (per body): its compressibility, durability and failure
  response, and its mass.
- **Response:** what gives way. One of: the driver stalls (keeps the
  valid part of its move), the body is displaced along a free direction,
  the body compresses, is damaged, is destroyed or is ejected, or the body
  passes through a support that yields (Mary-O's one-way).

## How the existing parts fit

- **Movement models.** Every movement model ends in the same kernel sweep
  and axis repair, which already reports the conflict. The response reads
  the conflict, not the model, so a walker, a ball, a crawler and a flyer
  are crushed by the same rule. A model that has its own collision shape
  (a crawler's adhesive body) reports through the same conflict.
- **One-way surfaces.** A one-way support claims a body only from above
  its face. Whether a one-way support may give way under compression
  (Mary-O) or holds and crushes (Ambition) is a policy flag, decided in
  the response, not by removing the one-way's claim in the sweep.
- **Moving geometry.** Platforms, lifts, ferries, planks and changing
  supports (`shape_before`) are the drivers. Each motion law gains a
  proposal and an accepted prefix (stage C3); riders are carried by the
  accepted motion, so carry never disagrees with what happened.
- **Body collision shapes.** A body's material (`BodyMaterial`: scale and
  orientation) is its collision shape for every reader. Directional squish
  is a change of material scale along the crush axis, which every reader
  already honours; flattening is the end of that range.
- **Portal frames.** A solid met through an aperture is a claimant in the
  same axis repair (to be witnessed in C1). A moving portal's host is the
  driver of the constraints met through it, with the host's authority:
  the train's portal crushes or ejects; a light platform's portal stalls.

## Still to design (before C4)

- **The numerical model.** The units of authority and resistance, how
  momentum and drive strength combine against mass and durability, and
  how a stall's accepted prefix is computed for one driver against several
  bodies. It must be integer- or fixed-order-deterministic for rollback.
- **The deterministic fallback.** When no response reconciles the
  constraints (two unstoppable drivers, an incompressible body with no
  escape and no failure response), the outcome must still be one defined
  state on every peer. Candidates to evaluate: eject along the least
  overconstrained free axis, destroy, or hold the body and report; the
  choice is design work, with a test that hits the case.

## Design

### One resolution, wherever the constraint comes from

The obstruction is read from the kernel's own conflict, which already
names its sources. The axis repair reads its claimants through
`material_axis_hits`, which takes the body's material cells from
`MaterialCast` (`.cells(admission)`), the same cast that carries a body
across a portal ([accepted motion across portal
charts](collision-and-ccd.md#accepted-motion-across-portal-charts)).
The premise, read from source and not yet witnessed, is that a solid
met through an aperture is a claimant in that set, so it produces the
same conflict, with a source that names the far-side surface. C1's
portal test proves or refutes it; if it is false, C1 routes the mapped
claimants into the same set rather than adding a portal-side resolver.
The resolver below never asks whether a source is
portal-carried. What differs for a portal is only *who the driver is*:
the host's motion carries the aperture, so the driver of a constraint
met through a moving portal is the host. That is how the host's
authority reaches the body (stage C5).

### Policy, at three levels

A small table of data, merged most specific first: body, then surface,
then game.

```text
GameCrushPolicy      default_authority, default_resistance, one_way_yields: bool
SurfaceAuthority     Yielding | Limited { max_overconstraint_px, max_force } | Unstoppable
BodyResistance       { compress: Option<Compressibility>, failure: FailureResponse }
Compressibility      { axes, min_scale (e.g. 0.35), recover_per_s, survivable: bool }
FailureResponse      Damage { per_px, per_tick_cap } | Destroy | Displace | Rigid
```

- `Yielding` (a cardboard lift, an elevator with a safety stop): the
  driver stalls on any obstruction.
- Authority is a force and a mass, not a speed: a slow press is
  `Unstoppable` or a high `Limited`; a fast light plank is `Yielding`.
- `Limited`: the driver pushes until the overconstraint or its force
  limit is reached, then stalls.
- `Unstoppable` (the train, a crusher): the driver keeps its schedule,
  and the body takes the whole response.
- A body with `Compressibility` squashes along the crush axis before it
  fails: its material scale on that axis falls toward `min_scale`, and
  recovers at `recover_per_s` when the pressure lifts. While squashed it
  is a state other systems can read (presentation, abilities, a
  "flattened" pose). If `survivable`, reaching `min_scale` is not death.
- `FailureResponse` applies when the body cannot (further) compress:
  damage per pixel of overconstraint (capped per tick), destruction, or
  ejection. `Rigid` is an incompressible body: it does not deform.
- Before any of these, a body with room to escape is **displaced** along
  the free direction (out of the pinch, or off the driver's leading edge).
- `one_way_yields` is a game rule. Mary-O sets it: when a moving one-way
  platform raises a body into a ceiling, the body falls through the
  platform rather than being crushed. Ambition leaves it off.

The **response** is chosen by comparing authority against resistance (a displacement first, when the body has room). The table is the shape of the rule; its numbers are the open numerical model:

| Driver | Body can compress | Body cannot compress |
| --- | --- | --- |
| `Yielding` | driver stalls | driver stalls |
| `Limited` under its limit | body compresses | body's failure response |
| `Limited` at its limit | driver stalls | driver stalls |
| `Unstoppable` | body compresses, then fails | body's failure response |

### The motion law takes a refusal

Stalling requires that a platform's motion law can be told no. Each
mover gains a proposal step: it proposes its delta, the obstruction pass
evaluates the bodies its proposal would overconstrain, and the mover
commits either the whole delta or the accepted prefix (the largest
fraction of the move that admits every body under its authority). The
accepted motion is the one riders are carried by and the one
`last_delta` records, so carry stays consistent with what actually
happened. A mover with no obstruction pays only the proposal: the pass
looks at bodies in contact with it, which the contact snapshot already
lists.

### Rollback and determinism

All resolution state is simulation state: the accepted prefix, a body's
compression and its recovery clock, damage dealt. Each is rewound with
its owner (the platform set, the body's material, its health). The pass
iterates bodies and movers in identity order; nothing reads a
presentation-only value.

## Stages

Each stage lands on its own, with its tests. Stages C0–C2 change no
behaviour.

**C0 — pin what exists.** Witnesses for today's behaviour, so later
stages are measured against it:

- a body between a descending block and a floor reports one Y conflict
  whose sources are the block and the floor, with `overconstraint()`
  equal to the overlap;
- Mary-O: a moving one-way platform raises the player into a ceiling and
  the player falls through the platform (find and name the mechanism;
  this becomes `one_way_yields`);
- a body walking into a still wall reports a motion refusal and no
  conflict (no driver, no crush).

*Receipt (2026-11-26, core `movement/tests/crush.rs`).* The pins found the
plan's first claim false for a full step:
`a_press_closing_on_a_standing_body_passes_through_it_unreported`. A press
descending 6 px a tick onto a body standing on a floor is met by the
gravity sweep as a head contact each tick; the body stays where it stands,
the overlap grows by the press's step (4 px, then 10, … 28 after five
ticks), and no step reports a conflict. Only the one-axis sweep called with
the gap already closed reports one (C1's witness). Mary-O's rule holds as
described: `a_one_way_that_raises_a_body_into_a_ceiling_drops_it_through`
(the lift raises the body to the ceiling, rises past its feet, and the body
falls through it; no conflict on any tick). The wall:
`a_body_walking_into_a_still_wall_reports_no_conflict`.

**C1 — the obstruction fact.** Turn each conflict into an `Obstruction`
{ body, axis, overconstraint, driver source, driver velocity, other
source }, with the driver chosen by relative motion (the claimant whose
surface velocity closes on the body). Published per tick, in identity
order. Tests: a still pinch has no driver; a moving block over a floor
names the block; the same pinch built across a static portal pair names
the far-side solid and produces the same numbers as the local pinch.

*Receipt (2026-11-26, core).* `AxisConstraintConflict` now records
`min_motion` and `max_motion`, each claimant's displacement this tick in the
root chart (from `step_in_root`, so a far-side claimant reads through its
portal), and `obstruction()` returns the `Obstruction` with `driver()`
choosing the claimant that closes on the body (stillness below 1e-4 is no
driver; a tie goes to the min side). Witness:
`a_pinch_names_the_claimant_that_closes_on_the_body` in
`movement/tests/wall_collision.rs`. Still open in C1: per-tick publication in
identity order, and the across-a-portal pinch witness.

*Receipt (2026-11-26, the rest).* Published per tick: the runtime's
`TickObstructions` resource, rebuilt after bodies step from every body's
reported conflicts, ordered by `SimId` (bodies without one last)
(`obstructions_are_published_in_identity_order`). Across a portal:
`a_pinch_across_a_portal_pair_is_the_local_pinch` builds the pinch with a
mouth where the ceiling was and a solid over its exit, and the far-side
solid claims the body with the local pinch's numbers; the rising platform
drives. **Still open, found by C0:** a full step does not report the
commonest crush, a press closing on a standing body (above). The step meets
the press as a head contact and lets it pass through, so the published list
is empty for it. C1 is complete for the conflicts the step reports; making
the step report this one (the press's head claim and the floor's support
claim as one empty interval) is the first piece of C4, since it is where the
body's response will act.

**C2 — policy data.** The three policy tables, with content loading and
validation. Defaults reproduce today exactly (no response). Tests:
merge order (body over surface over game); a policy that names an
unknown surface or body is refused at load.

*C2 receipt (2026-11-26, core `crush.rs`).* `SurfaceAuthority`
(Yielding, Limited, Unstoppable), `BodyResistance` (compressibility and a
`FailureResponse`), `GameCrushPolicy` and `CrushPolicy` (the game's rule and
the surfaces and bodies it names), with `authority`/`resistance` merging
own over named over game, and `validate` refusing an unknown surface or body
or a number out of range. Defaults reproduce today: unstoppable surfaces,
rigid bodies, `one_way_yields`. A moving platform's spec and state carry an
optional own `authority`. Loading the tables from content waits for C6,
which authors the first ones (the types are serde; the core has no RON).
Tests: `the_default_policy_changes_nothing`, `the_most_specific_level_wins`,
`a_policy_naming_an_unknown_surface_or_body_is_refused`.

**C3 — the driver stalls.** Movers gain the proposal step and the
accepted prefix. `Yielding` and `Limited` movers stall; riders are
carried by the accepted delta. Tests: a yielding lift stops on a body
under it, with the rider carried by the accepted delta and the body
untouched; a limited platform pushes until its limit, then holds; a
stalled mover resumes when the body leaves; all three under a rewind.

*C3 receipt (2026-11-26, world `platforms/obstruction.rs`,
`advance_moving_platforms`).* A platform whose authority allows a stall
proposes its advance: it reads the bodies its leading face would push, each
body's clearance against the room's solids (and a one-way only when pushed
onto its top along gravity), and keeps the part of its move that pushes no
body past its allowance (0 for yielding, the limit for limited, measured as
a depth so a limit is not spent again each tick). The accepted move is what
the platform records (`last_delta`), so riders are carried by it. An
unstoppable platform, the default, advances as before and the room builds
nothing for it. Pairwise and box-shaped: a body pushed into a body is not
chained. Tests in `platforms/simulation/tests.rs`:
`a_yielding_press_stops_on_a_body_under_it`,
`an_unstoppable_press_keeps_its_schedule`,
`a_limited_press_pushes_to_its_limit_then_holds`,
`a_stalled_press_resumes_when_the_body_leaves_and_a_rewind_replays_it`.

**C4 — the body gives way.** Compression (material scale on the crush
axis, recovery, the squashed state), damage, destruction, displacement,
and `one_way_yields`. Ambition's defaults turn on physical crushing.
Needs the numerical model and the fallback (above) designed first.
Tests: a body with room is displaced out of a closing gap; a slow press
with high authority crushes where a fast weak plank stalls (velocity is
not authority); an unstoppable block flattens a compressible body to
`min_scale` and it recovers once released; an incompressible body with no
escape meets the deterministic fallback, the same on a rewind; a rigid body takes damage per pixel up
to its cap and dies at zero health through the ordinary death path; a
destroyable prop is destroyed; Mary-O's C0 witness still passes with its
game policy.

**C5 — portal hosts.** `refresh_hosted_portal_frames` becomes a
proposal: the host's motion is admitted through C3, with the host as
the driver of constraints met through its aperture. Host, aperture and
material commit together, or the accepted prefix of all three. Tests:
a yielding host carrying a portal stalls while a body stands in the
portal and the far side would overlap a solid; an unstoppable host
crushes or damages that body by its resistance, exactly as the same
pinch without a portal would; the aperture never separates from its
host; the portal is not closed by default.

*C5 receipt (2026-11-26, runtime `portal_world/carried.rs`).* Before
platforms advance, the portal layer publishes `CarriedObstructions`: for
each body straddling a portal on a moving face, the part of it behind the
mouth's plane, mapped to the exit's chart, and the exit room's solids. The
host's proposal (C3) admits its move against them: the pair's map of the
host's `-delta` pushes that part, and a host that may stall keeps the part
of its move that part has room for. Host, aperture and part commit together
because the aperture follows the host's recorded (accepted) delta in
`refresh_hosted_portal_frames`. The portal is not closed. Test:
`a_yielding_host_stalls_on_a_body_whose_far_side_meets_a_solid` (a yielding
host keeps 2 px of its 4, moves on once the body leaves, and an
unstoppable host keeps its schedule; the aperture stays on its host).
Pairwise: only the straddled portal's host drives; an exit host's own motion
is not read. The unstoppable host crushing or damaging the body is C4's.

**C6 — content.** Author authority and resistance for shipped movers
and characters: the train (unstoppable), lifts (yielding), Scout
(rigid, damage), soft creatures (compressible, survivable flattened
pose). Presentation for the squashed state.

## Independence from portal work

M1, M2, SUP1's portal cases and P5/P6 proceed without this. Only the
admission half of driven aperture motion (C5) waits, and only on C1
and C3, not on C4 or C6.

## Not in scope

A general rigid-body solver, stacking forces through chains of bodies
(a body pushed into a body pushed into a wall is resolved pairwise per
step), and deformable geometry beyond a body's material scale.
