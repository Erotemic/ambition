# Expressive move capabilities — the engine surface a platform fighter needs

**Authority for:** which semantic capability owns each mechanical family, and
what a move is allowed to own itself. **Not** a status page for Smash content —
that stays [`../demos/smash-parity-inventory.md`](../demos/smash-parity-inventory.md).
The execution order lives in
[`../demos/campaigns/expressive-moves-2026-09-05.md`](../demos/campaigns/expressive-moves-2026-09-05.md).

Direction set by Jon, 2026-09-05, after reading the tree and the moves already
built. It **revises** the earlier "twenty new systems" answer in one major way,
and the revision is the whole point of this page.

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

⛔ **DO NOT CREATE `ActionGraph`, `Resource`, `TargetQuery`, `CaptureConstraint`
AND FRIENDS AS A NEW SET OF PEER ENGINE AUTHORITIES.** Ambition already has most
of the right authorities. The missing capability is a way for a move to
**compose and coordinate the authorities that exist, over time**.

⇒ The test for whether a new move justifies an engine change: it must introduce
a genuinely new **semantic primitive**, not merely a combination the existing
primitives cannot be sequenced into.

## Why the moves already built are the evidence

Each of the recent multi-day moves probed a different boundary, and each one
came out the same way — the primitive existed and the coordination did not.

* **Pirate Up-B** composes summon + mount + movement + recovery policy. Mount
  owns the ride relationship; the move does not.
* **Actor Down-B** uses the existing move-hold machinery to enter a `BodyMode`;
  movement and body semantics own what submerged means.
* **Flyline** establishes the wire only; the movement kernel owns the swing and
  the winch. ⭐ This is the template the rest should copy.
* **Author's blink** is already a generic teleport technique with authored
  destination policy, wall clamping, ledge assist, intangibility and
  presentation parameters.
* **Grounded command grabs** already reduce to an authored
  `smash.capture_attempt` into the generic `CapturedBy` authority.
* **Projectile Polygon's charge ball** already has continuous charge scaling,
  visual tiers, charge storage, interruption banking and special-button
  charging.
* **Projectile flight**, by contrast, is still mostly a ballistic description.
  ✔ Verified 2026-09-05 against `ProjectileFlight`
  (`crates/ambition_characters/src/brain/action_set/mod.rs:560`): `gravity`,
  `bounces`, `bounce_on_world_contact`, `max_lifetime`, `half_extent`,
  `boomerang_return_s`, splash half-extent. Nothing else. ⓘ Its own doc says the
  boomerang *"needs no reference to the thrower, which is what keeps a returning
  shot inside the projectile stepper's pure signature"* — the same fact this
  page records as "owner-relative return is not built yet", stated from the
  other side by the code that chose not to build it.

## The orchestration gate has fired

[`authored-gameplay-logic-and-orchestration.md`](authored-gameplay-logic-and-orchestration.md)
holds O3 shut until a real authored feature needs **both** a trigger
representation its current format handles poorly **and** semantic effects that
already have owning domains — and requires the first implementation to state who
owns per-occurrence memory and how it participates in rollback.

**PK-Thunder-style behaviour is that customer**, and it satisfies all three
clauses rather than two:

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

⭐ **Not one of those operations belongs to the sequencer.** Projectile spawning
belongs to projectile authority; input routing to control authority; trajectory
to projectile authority; fighter movement to movement authority; collision to
the relevant contact authority. The move owns exactly one thing:

> **what happens next, based on what happened before.**

## `TechniqueFlow` — and deliberately not `ActionGraph`

⛔ **THE NAME IS PART OF THE DESIGN.** `ActionGraph` reads as a universal
gameplay execution model, which is precisely what this page's own parent
refuses under "No universal sequencer". Scope the name to what it is: a
**move-scoped** flow. `TechniqueFlow` or `MoveFlow`.

The node vocabulary starts this narrow, and the omissions are the design:

```text
PreparedTechniqueFlow
    emit    semantic effect/request
    wait    for a semantic signal or condition
    branch  on a semantic condition
    finish
```

A node may transition to another node, so repeated-input moves are expressible.
⛔ **Initially NO variables, arithmetic, arbitrary expressions, arbitrary ECS
queries, general scripting, behaviour trees, or a global blackboard.** Every one
of those is how a move-scoped flow becomes the universal sequencer this
architecture spent two pages refusing.

### Who owns per-occurrence memory — O3's mandatory clause

**`MovePlayback`**, which is already the natural occurrence owner. ✔ Verified
2026-09-05 against `crates/ambition_combat/src/moveset/mod.rs`: it already
carries per-use deterministic state — `t` (move time), `landed_hit` /
`connected_hit` / `blocked_hit` / `hit_targets` (contact outcomes), `aim` and
`aimed_stick` (aim latches), `charge` (charge state), `looped_s` (loop state),
and `instance` (the stable move-use identity). It is rollback-carried today.

Extend that concept with only:

```text
current flow node
trigger/input latches
issued-event bookkeeping
timeouts
local symbolic slots
```

### What the flow cursor costs in the rollback wire — measured 2026-09-05

⛔⛔ **CORRECTED 2026-09-05 — THE FIRST VERSION OF THIS SECTION WAS WRONG, and
it was wrong in the direction that would have made somebody build the wrong
thing.** It said `MovePlayback`'s snapshot is a PROJECTION rather than a clone,
so a flow cursor added to the struct but not to the projection *"would silently
reset on every rewind"*. It would not.

`rollback_component_resolved` records itself as **"bevy_ggrs clone snapshot +
canonical authored-reference checksum projection"** — the whole component is
cloned and restored, and `encode_ref` is the CHECKSUM, not the restore. ⇒ New
fields come back from a rewind for free.

⚠ **AND I QUOTED A FUNCTION NOTHING CALLS.** The claim rested on
`MovePlayback::resumed(spec, facing, t, landed_hit)` and its doc about what "the
blob carries". That function has **no callers anywhere in `crates` or `game`** —
it is dead `pub` API, which is why no `dead_code` warning names it. I read a
signature and a doc comment and inferred a live mechanism from both.

⇒ **THE REAL BILL, which is smaller and differently shaped:**

1. the cursor, latches and timeouts join `MovePlayback` — **restored
   automatically**, because the snapshot is a clone;
2. they join `encode_ref` — not to be restored, but so a peer DIVERGENCE on them
   is detected. Omitting this does not reset anything; it means two peers whose
   flows are on different nodes agree on the checksum until the difference
   becomes visible in the world, which is the worst moment to find out;
3. that changes the wire fingerprint, so the schema version bumps and the
   readable baseline records it — including the version line in its header,
   which is part of the fingerprint by design.

ⓘ The registry's history still says this is the normal price and for the right
reason: **v144** added two contact facts to this exact projection and **v145**
added the instance, because *"two peers agreeing on the move id and its clock
could still disagree about whether this is the first jab or the second"*. A
cursor is the same class of fact — the correction is about WHY it must be
encoded, not whether.

### Slots, and why a move must not hold an `Entity`

```text
spawn projectile -> slot "thunder"
wait projectile.hit_owner("thunder")
```

⭐ The projectile retains a **stable semantic occurrence identity**; the move
holds a symbol, not a handle. That is what makes rollback, inspection, remote
detonation and later entity replacement clean rather than a table of stale
`Entity` values that a rewind invalidates.

⭐⭐ **AND THE CODEBASE ALREADY REFUSES THE ALTERNATIVE, which upgrades this from
a preference to a constraint.** `MovePlayback`'s own snapshot doc states
*"a blob cannot carry an `Entity` (N3.1 decision 2), and it does not have to"* —
so a slot holding a handle is not merely fragile, it cannot be stored in the one
place the flow's memory is allowed to live. ⇒ Symbols are the only design that
fits the rollback contract that was already there.

⇒ For something that outlives the move — C4, a planted mine — **the entity keeps
the occurrence identity**. The completed `MovePlayback` does not own it.

## The capability inventory

Jon's assessment of where the tree stands, 2026-09-05. ⓘ Recorded as **his
reading**, with the rows spot-checked against the code marked ✔; the rest are
unverified here and should be re-derived before anyone builds against them.

| Mechanical family | Representative moves | What Ambition already has | Reusable capability still needed | Authority |
|---|---|---|---|---|
| **Move orchestration / conditional phases** | PK Thunder, Sonic Blade, multi-stage specials, counter follow-ups | ✔ `MoveSpec`, timeline events, repeat, charge, contact outcome, `EffectRef` | **Move-scoped `TechniqueFlow`**, correlated signals, waits/branches | Moveset/playback owns only flow occurrence |
| **Temporary locomotion changes** | parasol/glide, hover, Spin Dash, homing dash, cart, Withdraw | unified movement, `MotionModel`, `BodyMode`, `WireState`, `motion_scale` | scoped movement **controllers/modifiers** requested through movement authority | Movement |
| **Deterministic targeting** | Homing Attack, auto-reticle, nearest-fighter teleport, guided missile | strong deterministic actor targeting; distance + stable `SimId` tie-breaking | reusable **semantic spatial selection**, without another persistent `ActorTarget` | Target/query policy; caller stores the identity |
| **Tether / spatial link** | ledge tether, fishing rod, whip grab, reel, swing | grapple traversal, flyline wire, capture reach volumes | shared tether constraint ONLY when real reeling/swinging needs it | Movement/spatial constraint |
| **Capture / command grab** | Flying Slam, Inhale, Egg Lay, Ridley drag | `CapturedBy` is already the sole capture relation; grounded command grab is authoring-only | aerial eligibility, targeted hit-grab request, cargo movement, richer escape/release | Capture |
| **Carry / drag** | DK cargo, Ridley Rush, Bowser carry | capture relation and pose constraint | explicit capture↔movement contract letting the captor move while the victim stays constrained | Capture + Movement |
| **Guided/steerable projectiles** | PK Thunder, Nikita, Din's Fire | unified projectile entities; ballistic `ProjectileFlight`; bouncing and returning shots | projectile-owned **guidance/control policy** and an input-control lease | Projectile |
| **Homing projectiles** | missiles, auto-target shots | deterministic target machinery exists elsewhere | target acquisition/lock/retarget policy inside the projectile owner | Projectile |
| **Owner-relative return** | boomerangs, crowns | ✔ analytic boomerang, deliberately owner-free | true owner-relative return/catch for a MOVING owner | Projectile |
| **Projectile interception** | reflector, cape redirect, absorber | parry already re-owns a projectile: rewrites allegiance, reverses/boosts velocity | generalise that into projectile-owned reflect/redirect/consume responses | Projectile |
| **Pocket / storage** | Pocket, Oil Panic | ownership and projectile identity foundations | immutable semantic stored payload; later respawn through projectile authority | Fighter state + projectile |
| **Persistent traps** | C4, mine, Lloid, planted bomb | bomb/item/projectile lifetimes | stable owner/source/tag identity, lookup of owned occurrences, remote trigger | Owning entity domain |
| **Attach to target/terrain** | C4, Crash Bomber, Pikmin latch, Corrin pin | capture/mount relationships, collision/world coordinates | generic stable attachment/anchor substrate, once multiple real customers overlap | Relationship/world owner |
| **Reactive defense** | counters, Revenge, Witch Time | parry already supplies "qualifying contact is denied" | authorable **successful-defense consequence** correlated back to the move | Combat |
| **Control impairment** | Sing, Disable, stun, sleep | hitstun/hard locks cover some cases | explicit status state ONLY where semantics differ: sleep/wake, mash escape | Combat/status owner |
| **Local time alteration** | Witch Time | `ProperTimeScale` exists; move/hurtbox clocks consume entity proper time | audit/finish propagation through locomotion and other body systems | Time + consuming domains |
| **Timed stat modifiers** | Deep Breathing, Revenge buff, Monado arts | some independent move/character tuning; armor/invuln | scoped modifiers applied by the authority of the affected quantity | ⛔ do not create a stat-writing god system |
| **Character meters/resources** | Limit, MP, fuel, feathers, ink, ammo, durability | `ResourceMeter`, `BodyMana`, other budgets | character-owned rollback resource state + semantic spend/gain/threshold ops | Character/provider |
| **Conditional move variants** | Limit specials, KO Punch, Arsène | move gates and repertoire resolution | state-conditioned **move binding/resolution** in one action-selection authority | Moveset/action resolution |
| **Transformations/forms** | Stone, alternate forms, stance swaps | some body modes; no universal form system, intentionally | eventually one `ResolvedForm` authority changing moveset/body/art/hurtboxes atomically | Character identity/form |
| **Summoned attack actor** | Phantom, temporary monster, turret | generic summoning; the shark is summon + mount | owner relation, lifecycle, command policy, attribution | Actor/summon |
| **True secondary/puppet actor** | Luma, Ice Climbers, Pikmin | pieces of summon/brain/control | participant-to-multiple-actor ownership/control model | Future dedicated capability |
| **Stage actuator** | spring, trampoline, hydrant | world/items/projectiles can spawn entities | reusable contact-triggered actuator: bounce/push/launch | World/entity + movement reaction |
| **Stage construction** | blocks, walls, platforms, destructibles | world geometry authority | dynamic authoritative world construction with stable identity | World |
| **Area force / field** | windbox, vacuum, gravity pull, poison | wind/force behaviour partly exists | persistent field emitter, once a move needs lifetime/periodic application | Combat/world |
| **Input delegation** | PK Thunder, Nikita, menu specials | semantic input/control structure is already strong | explicit routing/**lease** of selected input channels to another controlled entity | Input/control |
| **Input grammar** | command inputs, mash escapes, repeated directional specials | motion buffer; charge/repeat semantics | promote a reusable gesture recognizer as customers appear | Input |
| **Random/selected variant** | turnips, Judge, spell menus | deterministic engine foundations | rollback-safe selection/cycle/RNG policy | Character/move |
| **Self-interaction** | PK Thunder hitting its owner, recoil shots | ownership/friendly-hit filtering | authored self-contact eligibility, NOT a global weakening of owner exclusion | Projectile/combat |
| **Generated/held item** | vegetables, banana, gyro, bombs | held-item/custody infrastructure is substantial | mostly content; extend generation/inventory only with real customers | Item |
| **Recovery route semantics** | teleport, mount recovery, tether recovery, glide | burst, teleport and sustained-authority routes exist | tether/glide route shapes only when the recovery planner must reason about them | Recovery planning |
| **Sustained move presentation** | charge ball, attached aura, targeting line | point VFX events, procedural flyline visuals | move-relative sustained VFX with lifetime/attachment and stable source transforms | Presentation |
| **Per-action spawn transforms** | cannon muzzle, hand projectile, mouth beam | Projectile Polygon exposes the deficiency directly | **per-action authored muzzle/origin transform** | Projectile spawn authoring |

⭐ **A lot of apparent "mechanics" disappear once these exist.** That is the
argument for building the coordination rung before building mechanics.

## Four families that are much cheaper than they look

### Counters — do not build a Counter subsystem

Parry has already solved the hard semantic question: a qualifying attack reaches
a defensive state, the defender does not take the normal hit, and the contact
resolves deterministically. What is missing is approximately one line:

```text
successful defensive interception -> emit authored semantic effect
```

Then everything else is composition — an ordinary counter starts a retaliation
move; a Revenge-like effect increases a character resource; a Witch-Time-like
effect applies `ProperTimeScale` to the attacker.

### Ground command grabs — architecturally done

Authored `smash.capture_attempt` → Smash adapter → typed
`CaptureAttemptRequested` → generic capture authority. **No new command-grab
system.** The recent implementation proves the road.

### Tethers — four different mechanics wearing one word

⛔ Keeping these apart is what stops "tether" becoming an oversized abstraction.

* a **grounded long-range tether grab** barely needs tether physics at all —
  `CaptureAttemptRequested` already takes an authored reach volume, and the
  parity audit found the capture path does not intrinsically require hand-sized
  reach. Author the reach, add the line presentation. ⓘ The reach ceiling guard
  `no_authored_grab_reaches_further_than_the_stage_allows` anticipates exactly
  this and says so in place: raise the constant in the commit that authors the
  tether.
* an **aerial** tether grab needs capture eligibility generalised past the
  current grounded restriction;
* a **ledge tether** is a traversal/ledge-acquisition problem;
* only a tether that **reels, swings, drags or stays taut** justifies extracting
  shared constraint machinery from flyline/grapple.

### Reflectors — a projectile-domain operation, not a collision feature

Parry already changes `ProjectileOwner`, `ProjectileAllegiance` and velocity
while leaving presentation provenance separate. Turn that parry-specific code
into a projectile-domain semantic operation rather than implementing reflector
collision independently.

⛔ **AND KEEP THESE SIX AXES INDEPENDENT** — a reflected Nikita-like missile is
the case that proves "owner" cannot mean all of them at once:

```text
combat owner | allegiance/team | damage attribution
visual identity | trajectory | player control owner
```

## Movement needs the most careful abstraction

This is where flyline, trapdoor, blink, parasol, Homing Attack, Spin Dash, PK
Thunder 2, carts, burrows, glides and carries all push at once.

⛔ **NO TECHNIQUE SYSTEM MAY INSERT VELOCITY.** The rule stands:

> **The movement authority is the final writer of body motion.**

Moves instead request richer motion behaviour, decomposed as:

```text
Base MotionModel        long-lived character physics policy
BodyMode                structural body state / collision posture
Scoped Motion Controller   temporary EXCLUSIVE move-driven locomotion
Scoped Motion Modifiers    composable gravity/steering/speed-cap/terminal changes
```

* **Parasol**: the move requests a glide modifier (gravity scale, terminal fall
  speed, horizontal steering factor, open impulse); the movement kernel applies
  it; the move opens and closes it from input and flow.
* **Homing Attack**: the move selects a target, requests
  `GuidedDash(target, speed/accel/turn policy)`, movement integrates it and
  reports contact / obstruction / timeout, and the move branches.
* **PK Thunder 2**: the projectile reports a self-contact vector, the move
  requests a temporary directed launch controller, movement applies it.
* **Flyline** already reached this architecture, which is why it is the
  template rather than a special case.

## Projectiles get richer inside the projectile authority

⛔ **DO NOT ANSWER THE NEXT TEN MOVES BY GROWING `ProjectileFlight` INTO A BAG
OF BOOLEANS** — `is_homing`, `is_steerable`, `is_remote`, `is_returning`,
`is_sticky`, `is_orbiting`. Split behaviour into orthogonal policies instead:

```text
trajectory / guidance | world-contact response | body-contact response
lifetime | interception response | control source | ownership/allegiance
```

The state stays inside the projectile domain. That expresses ballistic,
bouncing, homing, player-steered, owner-relative return, stop-and-return,
falling-after-control-ends, sticky, remote-detonated, attached, orbiting,
target-seeking, destructible, reflectable and absorbable — **without a
fighter-specific system ever touching projectile position.**

## Status, resources and forms — built conservatively

**Status.** Do not answer Sing with a generic status scripting framework. Use
existing mechanics wherever the semantics genuinely match — Disable-style "you
cannot act for this duration" may already be expressible through the combat and
control lock machinery. **Sleep** is richer (cannot act, specific pose,
duration, mash reduction, wake on damage, stack/refresh rule) and earns an
explicit state once a real move needs it.

⭐ **Witch Time is the interesting one, because it is a completeness test rather
than a mechanism.** `ProperTimeScale` exists and move/hurtbox timelines already
advance in owner proper time — but the main home-body movement integration is
still passed the world `scaled_dt` while several combat timelines explicitly use
`entity_dt(ProperTimeScale)`. ⇒ A Witch-Time vertical slice should be run as a
**proper-time completeness audit**: locomotion, animation, move playback,
recovery clocks and status clocks must all agree about the victim's local time.
That is worth far more than a second slow-motion mechanism. ⚠ Jon's reading;
the `scaled_dt` / `entity_dt` split is the specific claim to re-derive first,
because the whole slice is scoped by whether it is still true.

**Resources start character-local.** `ResourceMeter` and `BodyMana` exist; do
not turn `BodyMana` into "all Smash resources". A Limit-style gauge is a meter
plus a full-state timeout, with `gain` / `spend` / `set` / `query full` /
`query fraction`, and its effects compose into a movement modifier and
conditional special variants. Hero MP, Inkling ink, ROB fuel, Banjo feathers and
Robin durability differ enough that one string-keyed global resource manager
would hide the semantics. ⇒ Implement the FIRST as a character-owned rollback
component over the existing meter math; extract a shared capability only if the
second and third expose a genuinely identical lifecycle.

**Forms stay deferred, with the shape written down.** When a concrete customer
arrives, one form authority resolves the whole bundle atomically:

```text
ResolvedForm -> moveset -> body tuning -> hurtbox/body shape
             -> movement traits -> available abilities -> presentation identity
```

⛔ The wrong implementation has the transformation move separately changing
movement parameters, replacing move tables, swapping art, changing collision and
toggling components through independent systems, with no single answer to *what
form is this fighter in?*

**Summons split into three levels**, and only the first two are proven: the
shark is summon + mount. A Phantom-like summoned attacker needs a `SummonedBy`
relation, lifetime, damageability, targeting and command policy. A Luma / Ice
Climbers / Olimar model is substantially harder because one player owns a
**multi-actor participant** — do not build it for one temporary shark, but keep
it on the horizon, because it is what will eventually test the assumption that
one participant equals one controllable body.

## Authoring and discovery — O4, with teeth

This is directly related to the observation that **agents do not reach for
capabilities unless told they exist**.

`EffectRef` is a good open seam with poor discoverability, and
`ParamSchemaRegistry` intentionally accepts an unknown effect key. It can say
*the parameters for `smash.teleport` are malformed*; it cannot say
*`smash.teleprot` does not exist.*

⇒ Add an **installed technique descriptor catalog** — for preparation,
inspection and tooling only, ⛔ never the runtime reducer. Per technique: key,
owning capability/domain, documentation, parameter schema, where it may be used
(event / sustained window / on-hit), signals it may produce, examples.

Then make the Smash provider's content-finalization pass strict —
**every authored `EffectRef` must resolve to an installed semantic technique** —
and expose `smash_tool techniques`, `smash_tool technique <key>`,
`smash_tool mechanics <domain>`.

⭐ This is O4's *"authoring and agent tooling should not require searching Rust
registration topology to learn what exists"*, and it makes it materially harder
for an agent to decide "I guess I need to write a new system" before discovering
that capture, teleport, stored charge and mount already exist.

## The diagnostic question when a move feels wrong

> First ask whether the **semantic mechanic** is wrong, or whether the
> simulation is right and the authored spatial/timing/presentation contract is
> too weak.

The charge ball is the worked example. Its mechanical pieces are all present —
special-button charge, arbitrary hold, continuous fraction, continuous
damage/speed/size scaling, stepped visual tiers, storage, interrupted-charge
banking, stored charge on the same move, and a full charge that stays loaded
instead of auto-firing. The source itself names the two things actually missing:
**per-action muzzle location** (the charge VFX is drawn where the generic
projectile road launches, not at the character's cannon) and **sustained charge
presentation** (a timeline VFX event is a point event; charging is a
variable-duration state, and the current art is built to disguise that).

⇒ Those two are the engine changes to make — not another charge mechanism.
**Blink is likely the same story**: the teleport technique already supports aim
latching over startup, an upward default, destination collision resolution,
ledge assist, "behind nearest foe" targeting, intangibility and authored
departure/arrival effects. If Author still does not read like Mewtwo, diagnose
missing teleport POLICY parameters or presentation sequencing before writing a
second teleport.
