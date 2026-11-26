# Crushing, compression and motion authority

**Status:** Q187 is resolved at the architectural level (Jon, 2026-11-26).
The design below follows from the ruling. Two things are **not** decided
and need design work before C4: the numerical model (how authority and
resistance are measured and compared) and the deterministic fallback for
constraints that no response can reconcile. Nothing below is built except
what [the current state](#the-current-state) names. Work: queue CRUSH.

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

**C1 — the obstruction fact.** Turn each conflict into an `Obstruction`
{ body, axis, overconstraint, driver source, driver velocity, other
source }, with the driver chosen by relative motion (the claimant whose
surface velocity closes on the body). Published per tick, in identity
order. Tests: a still pinch has no driver; a moving block over a floor
names the block; the same pinch built across a static portal pair names
the far-side solid and produces the same numbers as the local pinch.

**C2 — policy data.** The three policy tables, with content loading and
validation. Defaults reproduce today exactly (no response). Tests:
merge order (body over surface over game); a policy that names an
unknown surface or body is refused at load.

**C3 — the driver stalls.** Movers gain the proposal step and the
accepted prefix. `Yielding` and `Limited` movers stall; riders are
carried by the accepted delta. Tests: a yielding lift stops on a body
under it, with the rider carried by the accepted delta and the body
untouched; a limited platform pushes until its limit, then holds; a
stalled mover resumes when the body leaves; all three under a rewind.

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
