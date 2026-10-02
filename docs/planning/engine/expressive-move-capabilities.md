# Expressive move capabilities — the engine surface a platform fighter needs

**Scope:** which semantic capability owns each mechanical family, and what a
move can own itself. It is not a status page for Smash content; that is
[`../demos/smash-parity-inventory.md`](../demos/smash-parity-inventory.md).
Admission of authored techniques is in
[authored technique admission](authored-technique-admission.md). Work is in
[`../queue.md`](../queue.md).

Standing lessons:

1. Geometry origin, damage eligibility and attack attribution are separate facts.
2. A capability that is compiled but not installed and used by content is not
   accepted.
3. Query or entity order is not arbitration.
4. A move whose mechanic the player cannot read is not complete.
5. Reusable capability mechanisms go below game-specific balance and policy.
6. Many apparent engine gaps are existing capabilities with no authored
   customer. Start new move work from this page and the parity inventory.

## The rule

> **A complex move may coordinate many authorities, but it must not become the
> authority for their state.**
>
> The move owns its sequence, branches, latches, and references to semantic
> occurrences. Movement owns motion. Capture owns capture. Projectiles own
> projectile state. Resources own resources. Character state owns forms. Input
> owns control routing. World capabilities own persistent world entities.
>
> Rust extends the available semantic vocabulary. Authored flow composes that
> vocabulary.

Do not create `ActionGraph`, `Resource`, `TargetQuery`, `CaptureConstraint` or
similar types as new peer engine authorities. Ambition has most of the right
authorities. What a move needs is a way to compose and coordinate them over
time.

A new move justifies an engine change only if it adds a new **semantic
primitive**, not a combination that the existing primitives can be sequenced
into.

## Evidence from the moves that exist

Each of these moves probed a different boundary. In each, the primitive
existed and only the coordination was missing.

* **Pirate Up-B** composes summon, mount, movement and recovery policy. Mount
  owns the ride relationship.
* **Actor Down-B** uses move-hold machinery to enter a `BodyMode`; movement and
  body semantics own what "submerged" means.
* **Flyline** sets up the wire only; the movement kernel owns the swing and the
  winch. This is the template.
* **Director's blink** is a generic teleport technique with authored
  destination policy, wall clamping, ledge assist, intangibility and
  presentation parameters.
* **Grounded command grabs** are an authored `smash.capture_attempt` into the
  generic `CapturedBy` authority.
* **Projectile Polygon's charge ball** has continuous charge scaling, visual
  tiers, charge storage, interruption banking and special-button charging.
* **Projectile flight** is still mostly a ballistic description:
  `ProjectileFlight` (`crates/ambition_characters/src/brain/action_set/mod.rs`)
  has gravity, bounces, world-contact bounce, lifetime, half extent, an
  analytic boomerang return and splash extent. The boomerang needs no reference
  to the thrower, which keeps the projectile stepper's signature pure.

## `TechniqueFlow` — and deliberately not `ActionGraph`

The name is part of the design. `ActionGraph` reads as a universal gameplay
execution model, which [authored orchestration](authored-gameplay-logic-and-orchestration.md)
refuses. `TechniqueFlow` is a move-scoped flow. Its customer is a
PK-Thunder-style move:

```text
start move
spawn projectile as "thunder"
route steering input to "thunder"
restrict fighter control
wait until one of:
    thunder hits fighter
    thunder hits something else
    thunder expires
if thunder hits fighter:
    release projectile control
    request directed fighter launch
else:
    release projectile control
finish
```

No operation in that list belongs to the sequencer. Each belongs to the
projectile, control, movement or contact authority. The move owns one thing:
**what happens next, based on what happened before.**

```text
PreparedTechniqueFlow
    emit    semantic effect/request
    wait    for a semantic signal or condition
    branch  on a semantic condition
    finish
```

No variables, arithmetic, arbitrary expressions, arbitrary ECS queries, general
scripting, behavior trees or global blackboard. Each of those turns a
move-scoped flow into a universal sequencer.

### One contact decision per occurrence

`MovePlayback`'s `landed_hit`, `connected_hit` and `blocked_hit` are
per-occurrence latches. Nothing clears them until the move ends, and
`contact()` derives `overlapped` from all three. So after the first contact,
every later `Wait { on: Overlapped }` in the same flow is already satisfied.
`a_second_wait_on_contact_passes_on_the_first_hit`
(`ambition_combat::moveset::tests`) pins this. If per-beat contact is ever
added, that test fails, and `FlowSignal`'s doc and `TechniqueFlow::problems()`
must change with it.

This is a boundary, not a gap. Each multi-hit shape has its own road:

| Shape | Road | Why |
|---|---|---|
| An unconditional multi-hit | Windows on one timeline | A combo is one move, not two moves and a cancel |
| A confirmed follow-up that the player presses | `Cancelable` + `CancelCondition::{OnHit, OnWhiff, OnBlock}` | It is a read, and a read needs an input |
| An automatic consequence that nobody presses | `TechniqueFlow` | The one decision that neither of the above can make |

Witnesses: `a_follow_up_press_walks_the_authored_jab_chain` (engine seam);
`every_cancel_target_resolves_and_a_confirm_is_authored` (Pugnacious Polygon's
jab confirms into `polygon_brawler_jab2` on hit).

### Who owns per-occurrence memory

**`MovePlayback`**, the occurrence owner. It carries per-use deterministic
state: move time, contact outcomes and `hit_targets`, aim latches, charge, loop
state and `instance` (the stable move-use identity). It is rollback state.
Extend it only with: the current flow node, trigger and input latches,
issued-event bookkeeping, timeouts and local symbolic slots.

### Cost in the rollback wire

The `MovePlayback` snapshot is a clone; `encode_ref` is the checksum
projection, not the restore. So a new field comes back from a rewind without
extra work. A new flow field must still join `encode_ref`, so that a peer
divergence on it is detected before it shows in the world. That changes the
wire fingerprint: the schema version advances and the readable baseline
records it.

### Slots, and why a move must not hold an `Entity`

```text
spawn projectile -> slot "thunder"
wait projectile.hit_owner("thunder")
```

The projectile keeps a stable semantic occurrence identity; the move holds a
symbol, not a handle. A snapshot blob cannot carry an `Entity`, so a slot that
holds a handle cannot be stored where the flow's memory lives. For something
that outlives the move (a planted mine), the entity keeps the occurrence
identity; the completed `MovePlayback` does not own it.

### A "sole writer" claim must include the rollback codec

A rollback codec writes nearly every registered field, and a grep for
assignments cannot see it, because decode builds a struct literal. A "sole
writer" comment on a field of a rollback-registered type is not correct until
you read its `snapshot_impls`. Find the affected types with
`git grep -l 'impl SnapshotState'`.

- Write "the only writer on a live entity" when the other writer is a
  constructor or a restore.
- Write "the only writer outside the codec" when the codec rebuilds the field.
- Do not make such a field private. `snapshot_impls` is usually a sibling
  module; use `pub(crate)`.
- Messages are not affected. `clear_message_on_rollback` clears the buffer, and
  no codec constructs a message.

## Capability inventory

Rows marked "shipped" are in source. Re-derive the other rows before you build
against them.

| Mechanical family | Representative moves | What exists | Reusable capability still needed | Authority |
|---|---|---|---|---|
| Move orchestration | PK Thunder, Sonic Blade, multi-stage specials | `MoveSpec`, timeline events, repeat, charge, contact outcome, `EffectRef`, `TechniqueFlow` | Correlated signals as customers appear | Moveset playback owns only the flow occurrence |
| Temporary locomotion changes | parasol, hover, Spin Dash, homing dash, cart | unified movement, `MotionModel`, `BodyMode`, `WireState`, `motion_scale` | Scoped movement controllers and modifiers requested through movement | Movement |
| Deterministic targeting | Homing Attack, auto-reticle, nearest-fighter teleport | deterministic actor targeting; distance plus stable `SimId` tie-break | Reusable semantic spatial selection, without another persistent `ActorTarget` | Target policy; the caller stores the identity |
| Tether or spatial link | ledge tether, fishing rod, whip grab | grapple, flyline wire, capture reach volumes; the ground tether is shipped | Ledge tether (mostly `TeleportParams::ledge_assist` plus a visible line); shared constraint only when reeling or swinging needs it | Movement and spatial constraint |
| Capture or command grab | Flying Slam, Inhale, Egg Lay | `CapturedBy` is the only capture relation; grounded command grab is authoring only | Aerial eligibility, targeted hit-grab, cargo movement, richer escape | Capture |
| Carry or drag | DK cargo, Bowser carry | Shipped: a carry is a hold that does not zero the captor's locomotion (`restrict_captor_control`; a flag on `SmashHoldState`) | - | Capture + movement |
| Guided projectiles | PK Thunder, Nikita | Shipped: `ActorControlFrame::steer_axis()` publishes what the player holds; the caster stays rooted and keeps the seat. Steering is not possession | - | Projectile |
| Homing projectiles | missiles | target machinery elsewhere | Acquisition, lock and retarget inside the projectile owner | Projectile |
| Owner-relative return | boomerangs | analytic boomerang, owner-free | Return and catch for a moving owner | Projectile |
| Projectile interception | reflector, absorber | Shipped: `ProjectileInterception::{Reflect, Consume}`; the reflector uses the same `parrying()` window as the melee seam | - | Projectile |
| Pocket or storage | Pocket | ownership and projectile identity | Immutable stored payload; respawn through projectile authority | Fighter state + projectile |
| Persistent traps | C4, mine | Shipped: the trap names its owner by `MatchSeat` (rollback state) | General occurrence identity for steerable projectiles | Owning entity domain |
| Attach to target or terrain | C4 stick, Pikmin latch | capture and mount relationships | Stable attachment substrate, once several customers overlap | Relationship or world owner |
| Reactive defense | counters, Revenge | Shipped: `ParriedBodyHit` + `smash.counter`; `answer_a_parry_with_the_authored_counter` dispatches an arbitrary technique. A counter is a trigger | - | Combat |
| Control impairment | Sing, sleep | Shipped: `sleep_timer` as a named cause in the hard lock, plus `smash.sleep` | Mash escape | Combat or status owner |
| Local time alteration | Witch Time | `ProperTimeScale`; move and hurtbox clocks use entity proper time | A proper-time completeness audit (locomotion, animation, playback, recovery and status clocks) | Time + consuming domains |
| Timed stat modifiers | Deep Breathing, Monado | some tuning, armor, invulnerability | Scoped modifiers applied by the owner of the affected quantity; no stat-writing god system | Owner of the quantity |
| Character meters | Limit, MP, fuel, ammo | `ResourceMeter`, `BodyMana`, other budgets | Content-defined resources with prepared handles and atomic costs | Character + [resource owner](composable-actor-resources.md) |
| Conditional move variants | Limit specials, KO Punch | move gates, repertoire resolution | State-conditioned move binding in one action-selection authority | Moveset resolution |
| Transformations or forms | Stone, stance swaps | some body modes | One `ResolvedForm` authority that changes moveset, body, art and hurtboxes together | Character form |
| Summoned attack actor | Phantom, turret | generic summoning; the shark is summon + mount | Owner relation, lifetime, command policy, attribution | Actor or summon |
| Secondary or puppet actor | Luma, Ice Climbers | parts of summon, brain and control | Participant-to-many-actors ownership and control | Future capability |
| Stage actuator | spring, trampoline | world, items and projectiles spawn entities | Contact-triggered bounce, push or launch | World + movement reaction |
| Stage construction | blocks, walls | world geometry authority | Dynamic world construction with stable identity | World |
| Area force or field | windbox, vacuum | `VolumeReaction::Windbox` (with a `WindboxWithDamage` validation error); the move-scoped gust is shipped (`ambition_entity_catalog::authoring::gust`) | A persistent field emitter | Combat or world |
| Input delegation | PK Thunder, Nikita | `DrivingParticipant` (rollback state) names the driver; `ActorControl` is the per-tick frame; `ControlClaims` names temporary controllers by `SimId`; `causal::seat_of` attributes a driven body's hits to its seat | A per-seat driver that a move can operate. A move feeds `project_driving_participant`; it never writes `DrivingParticipant` | Input and control |
| Input grammar | command inputs, mash escapes | motion buffer; charge and repeat | A gesture recognizer as customers appear | Input |
| Random or selected variant | turnips, Judge | deterministic foundations | Rollback-safe selection and RNG policy | Character or move |
| Self-interaction | PK Thunder hitting its owner | Shipped: the bolt owns a `clear_of_caster` latch; the eligibility is the move's, not a global rule. A spawn point is inside the spawner | - | Projectile + combat |
| Generated or held item | vegetables, bombs | held-item custody | Mostly content | Item |
| Recovery route semantics | teleport, tether, glide | burst, teleport and sustained routes | Tether and glide routes only when the recovery planner must reason about them | Recovery planning |
| Sustained move presentation | charge ball, aura, targeting line | point VFX events, procedural flyline visuals | Move-relative sustained VFX with lifetime and attachment | Presentation |
| Per-action spawn transforms | cannon muzzle, mouth beam | - | Per-action authored muzzle or origin transform | Projectile spawn authoring |

## Families that are cheaper than they look

### Counters

Parry answers the hard question: a qualifying attack reaches a defensive
state, the defender does not take the normal hit, and contact resolves
deterministically. A counter adds one step:

```text
successful defensive interception -> emit authored semantic effect
```

A retaliation move, a Revenge-like resource gain or a Witch-Time-like
`ProperTimeScale` on the attacker are all composition.

### Ground command grabs

Authored `smash.capture_attempt` -> Smash adapter -> typed
`CaptureAttemptRequested` -> generic capture authority. No command-grab system.

### Tethers — four mechanics with one name

* A **grounded long-range tether grab** needs little tether physics:
  `CaptureAttemptRequested` takes an authored reach volume. Author the reach and
  add the line. Raise the ceiling in
  `no_authored_grab_reaches_further_than_the_stage_allows` in the change that
  authors the tether.
* An **aerial** tether grab needs capture eligibility past the grounded
  restriction.
* A **ledge tether** is a traversal and ledge-acquisition problem.
* Only a tether that **reels, swings, drags or stays taut** justifies shared
  constraint machinery from flyline and grapple.

### Reflectors

Reflection is a projectile-domain operation (`ProjectileInterception`), not a
collision feature. Keep these six axes independent; a reflected Nikita-like
missile shows that "owner" cannot mean all of them:

```text
combat owner | allegiance/team | damage attribution
visual identity | trajectory | player control owner
```

## Movement

> **The movement authority is the final writer of body motion.** No technique
> system inserts velocity.

Moves request motion behavior:

```text
Base MotionModel           long-lived character physics policy
BodyMode                   structural body state / collision posture
Scoped Motion Controller   temporary EXCLUSIVE move-driven locomotion
Scoped Motion Modifiers    composable gravity/steering/speed-cap/terminal changes
```

* **Parasol:** the move requests a glide modifier (gravity scale, terminal fall
  speed, steering factor, open impulse); movement applies it; the move opens and
  closes it.
* **Homing Attack:** the move selects a target and requests
  `GuidedDash(target, policy)`; movement integrates it and reports contact,
  obstruction or timeout; the move branches.
* **PK Thunder 2:** the projectile reports a self-contact vector; the move
  requests a directed launch controller; movement applies it.
* **Flyline** already has this shape.

## Projectiles

Do not grow `ProjectileFlight` into a bag of booleans (`is_homing`,
`is_steerable`, `is_returning`, ...). Split behavior into orthogonal policies
inside the projectile domain:

```text
trajectory / guidance | world-contact response | body-contact response
lifetime | interception response | control source | ownership/allegiance
```

No fighter-specific system writes projectile position.

## Status, resources and forms

**Status.** Do not answer Sing with a status scripting framework. Use existing
lock machinery where the semantics match. Sleep has its own named cause.

**Witch Time** is a completeness test: run it as a proper-time audit so that
locomotion, animation, move playback, recovery clocks and status clocks agree
about the victim's local time. First re-derive whether home-body movement still
integrates with the world `scaled_dt` while combat timelines use
`entity_dt(ProperTimeScale)`.

**Resources** have a focused owner: [composable actor
resources](composable-actor-resources.md) (stable content identity -> validated
composition -> prepared handle -> dense actor-local state). Do not turn
`BodyMana` into all resources or replace it with a string-keyed global manager.
Fill, decay, stock and efficiency policy stays with the capability or ruleset
that owns the rule.

**Forms** wait for a customer. When one arrives, one authority resolves the
whole bundle at once:

```text
ResolvedForm -> moveset -> body tuning -> hurtbox/body shape
             -> movement traits -> available abilities -> presentation identity
```

Independent systems that each change one part, with no single answer to "what
form is this fighter in?", is the wrong implementation.

**Summons** have three levels. Summon plus mount is proven (the shark). A
Phantom-like attacker needs a `SummonedBy` relation, lifetime, damageability,
targeting and command policy. A multi-actor participant (Luma, Ice Climbers) is
much harder; do not build it for one temporary summon.

## Authoring and discovery

Agents do not use capabilities that they cannot find. Installed-support
admission ([authored technique admission](authored-technique-admission.md))
refuses an authored `EffectRef` that names no installed technique. The
remaining work is discovery: an installed technique descriptor catalog for
preparation, inspection and tooling only (never the runtime reducer), with key,
owning domain, documentation, parameter schema, where it can be used, signals it
can produce and examples; and tool commands that list techniques and
mechanics by domain.

## The diagnostic question when a move feels wrong

> First ask whether the **semantic mechanic** is wrong, or whether the
> simulation is right and the authored spatial, timing or presentation contract
> is too weak.

The charge ball is the example. All its mechanical parts exist. The two missing
parts are **per-action muzzle location** (the charge VFX is drawn where the
generic projectile road launches) and **sustained charge presentation** (a
timeline VFX event is a point event). Those are the engine changes to make.
Blink is likely the same: diagnose missing teleport policy parameters or
presentation sequencing before you write a second teleport.

## Admission prerequisites for new authored mechanics

New move content goes through installed-support admission and the flow bounds
of A11/A12 ([frontier](actor-monolith-work-frontier.md)).

Keep content decisions in the Smash inventory and maintainer rulings. A general
form, limit meter, controllable projectile, counter or multi-beat command
capture needs a real customer and one transition owner; it does not justify a
universal ability service. A flow `Wait` reads the current occurrence's latched
signals unless a scoped per-beat fact is implemented and tested.

Contact work distinguishes geometric contact from damage and projectile
consumption. A new counter or reflector states its precedence against the same
accepted contact; it does not add another independent overlap test.
