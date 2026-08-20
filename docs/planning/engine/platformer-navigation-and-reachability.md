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

## ✔ The first slice LANDED, and the query now has a consumer (2026-08-15)

⭐ **`probe_recovery` shipped with ZERO consumers, and that WAS the defect** (it
has one now — the fighter brain, below) — a
reusable query with no adopter decays and its claims go stale. It has one now.

`RecoveryLens` (`ambition_characters/src/brain/fighter/recovery.rs`) lowers a
`Perceived` view into a real `ae::World` plus the body's **own** `AbilitySet` and
`MovementTuning` into an `ae::BodyClusterScratch`, and `refine_by_rollout`
overrules the shadow **in both directions**.

⭐ **the reprieve is the half that fixed the measured defect**, and it is not the
half anyone expected: the shadow line `Hold`s after `commit_ticks`, so near a
ledge it condemned **every** verb — the veto emptied, and choice fell through to
*"dies latest"* rather than *"lives"*. The condemnation was never the whole bug.

✔ **and it was a deletion.** `WorldView::reachable` and `SolidKind::blocks_path`
— hand-rolled straight-line reachability with **zero production consumers** — are
gone. ⭐ that is exactly the duplication the section below forbids, and it existed
in the tree while this plan was being written.

⭐ **the body-generic proof is REAL, and the falsifier was executed:** two bodies
at an identical position, with identical geometry, gravity and unspent air-jump
*count*, differing only in `AbilitySet::double_jump`, reach opposite verdicts —
because the kernel gates on verb **and** budget. Poisoning the production path so
the lens builds its scratch body with a default `AbilitySet` turns **exactly that
test red and no other**. ⇒ the verdict comes from the body's capabilities, not
from the stage's shape.

### ⭐ The negative is now BOUNDED, and that was the important correction

⛔ **`NoSupportFound` never meant "this body cannot recover"** — it meant *"the
positions a small fixed steering policy reached, within this horizon, found no
support."* The policy presses **only `side ∈ {0, -1, +1}` plus jump**; dash,
blink, flight, wall verbs, ledge grab and recovery attacks are never explored.

The outcome is therefore `NoSupportFoundBy { search, reset }`, carrying the
policy and horizon that failed, and `RecoveryOutlook::bounded_by()` reads it.
⭐ **a positive returns `None`** — finding a route proves one exists, while
failing to find one is only ever a claim about the searcher.

⭐ **and the falsifier is a two-policy comparison, which is why it is convincing.**
Same body, same position, same horizon, same world: under the default policy the
body does not recover; under a policy that presses **blink**, it does. ⛔ that
test could not be red before the split, because the comparison was unexpressible
— it pins the current bounded answer and demonstrates it is wrong about the body
in the same breath. Poisoned by making the blink policy stop pressing blink:
exactly that test reddens, so the positive comes from the verb and not from the
fixture drifting onto the shelf.

⇒ **sound for the shipped fighter, which owns neither dash nor blink** — and the
first thing to re-check the day a fighter gains one.

⚠ **three further honest limits, recorded rather than hidden:**

- **blast margins are probed at zero** while Smash's stage authors 120px on all
  three axes, so the probe is conservative by exactly that. A body genuinely
  recoverable inside the real margin is condemned. Fixing it means carrying
  margins on `StageView`.
- ✔ **the ledge-transition case is now PINNED (2026-08-15)** — it was the common
  one in play. The two sites disagreed by `half_width − EDGE_OVERLAP_SLOP`, 11px
  for the shipped fighter: the shadow tested the body's **centre** against
  `ground_span` while the kernel used the **footprint**, so a probe started
  half-overlapping the platform it was leaving and the first "stand still" effort
  landed it straight back on. ⭐ fixed by making the capture point ask the
  kernel's own question (`spans_overlap_for_support`), **not** by nudging the
  start position — a magic epsilon beside a body claim is the smell, not the fix.
  Probe frequency is non-increasing, since footprint-supported is a strict
  superset of centre-supported for any body wider than `2 × EDGE_OVERLAP_SLOP`.
- **cost is unmeasured.** At most one probe per modelled movement verb per
  decision (≤4), only for a verb whose line left the ground, each 3 efforts
  capped at 2.0s — worst case ~1440 kernel steps per decision, decisions every 5
  ticks. The existing budget bench rolls no movement lines, so it does not price
  the lens and needs extending.

⚠ **one design premise nothing enforces:** the lens reprieves a line on the
assumption the brain *will* switch to full recovery effort at its next decision.
That is true today (it re-decides every `interval()` ticks) but unenforced — if
`Situation::Recovery` ever fails to offer `Recover`, the reprieve becomes a
promise nothing keeps.

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
