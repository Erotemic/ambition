# Platformer navigation and reachability — Engine 1.0 program

**State:** OPEN — problem and customers are clear; graph representation and dynamic-path semantics are not.

## Goal

Let the engine answer **whether and how a particular body can get somewhere** in
a 2D platformer world.

This is not a generic 3D navmesh project. Reachability depends on authored
geometry, jumps/drops, ledges, ladders, portals, moving platforms, gates, room
transitions and the body's current capabilities/properties.

## Consumers

- NPC pursuit and autonomous movement;
- Ambition open-world population and persistent actors;
- boss/fighter reasoning where long-range traversal matters;
- LDtk authoring validation and LLM spatial reasoning;
- capability-gated world design;
- multi-room route planning and soft exploration hints.

## Required query shape

Eventually an agent or AI should be able to ask:

```text
Can body B reach target T now?
Which traversal transitions make that possible?
Which capability blocks the route?
What routes change if gate G opens?
Can this NPC patrol between these regions?
Where would a moving platform create a new route?
```

## ⭐ It already has a MEASURED, BLOCKED consumer — start there

**Super Smash Siblings' CPU is failing for exactly the reason this plan exists,
and the diagnosis is already recorded** (D72 / `engine/fighter-brain.md`,
2026-08-14): the rollout horizon is ~12 ticks (0.2s) while the fall from a
platform to the blast floor is ~24 ticks (0.4s), **so a deeper search cannot see
the cost of a ledge exit and increasingly picks apparently-free self-KO
trajectories.** Two rigs agree that a duelist loses all three stocks to itself at
0% damage, and the A/B is stark — depth 0 survives 47.8s, depth 12 survives 7.4s.

⛔ **and the shortcut was already tried and REMOVED, which is the useful part.** A
terminal value of *"airborne + below the lip + outside the span ⇒ already dead"*
was implemented, measured, and deleted **because it is not body-generic**: air
movement, jumps, flight, wall interaction, ledge grab, recovery attacks, impulses,
portals and grapples each falsify it. ⭐ **that is precisely a reachability
question wearing a fighting game's clothes** — *can THIS body, with ITS
capabilities, still get back?* — and the recorded conclusion names this plan as
where the real answer comes from.

⇒ so the first slice should be shaped by a consumer that exists and is measurably
broken, not by the general case. **"Is this body's position recoverable under its
own capabilities?" is a smaller and sharper question than "plan a route",** and it
is the one already blocking work.

## Architecture direction

Prefer a derived traversal/reachability representation over hand-authored
waypoint lore. The representation should reference authoritative world geometry
and capability requirements rather than duplicate them.

⭐ **and the pieces that make "reference rather than duplicate" concrete now
exist** — this section used to be a principle with no handles:

- **authoritative geometry** is `CollisionWorld`, which answers exactly four
  questions (`solids`, `carves_only`, `hostable_surfaces`, `base`) and has no
  non-adopters left. ⛔ a reachability graph that builds its own block list is the
  duplication this section forbids.
- **time-dependent routes** have a handle: a moving solid publishes its
  displacement on `Block::velocity`, and `MovingPlatformState` carries
  `previous_aabb()`. ⚠ note the trap that one-way landing already hit — comparing a
  body's PREVIOUS coordinate against a solid's CURRENT face is a **mixed frame**
  for geometry that moves.
- **capability requirements** are the body's own authored kit; the population that
  could not build a body from its own definition went 14 → 7 → **0**, so there is
  no fallback path to special-case.

⭐ **the reporting contract is settled and should be copied, not re-litigated:**
the reusable mechanics layer reports **what physically happened** and game policy
decides what it means. `FrameEvents` already does this for contacts, and D126
extended it to *"no legal position exists"* via `AxisConstraintConflict` — with
nothing reading the conflict, deliberately. ⇒ **reachability should answer
*"which capability blocks the route"* in the same register: report the blocking
fact, and let the brain, the authoring validator or the LLM decide what to do
about it.**

Separate **route existence/planning** from **low-level movement execution**. A
brain may choose a route while the ordinary body movement kernel still performs
jumps, climbs or portal traversal.

## Candidate crate / Bevy ecosystem value

This is one of the strongest candidates for an eventually independent Bevy
plugin/crate because platformer navigation is a general gap and can plausibly be
specified without Ambition content. A mature crate should accept world/traversal
adapters rather than import Ambition room or character catalogs.

Do not publish until the Ambition implementation survives at least moving
platforms, portals and capability-dependent routes.

## Open design questions — deliberately unresolved

- Graph of discrete traversal opportunities, sampled reachability field, or a
  hybrid?
- How are continuous jump arcs represented without exploding graph size?
- How are moving-platform/time-dependent routes represented and costed?
- How frequently does dynamic world change invalidate navigation data?
- Are portals ordinary graph edges or a separate routing layer?
- How should navigation span unloaded rooms?
- Does background AI plan exact routes or only region-level intentions?
- How is route risk/danger represented without mixing game-specific utility into
  the navigation core?
