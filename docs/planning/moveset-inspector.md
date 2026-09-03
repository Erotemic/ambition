# Combat Inspection and Moveset Observatory

> ⛔ **NOTHING IN THE REPOSITORY LINKS TO THIS PAGE (measured 2026-09-03), and
> that makes it unreachable by the documented route.**
> [`README.md`](README.md) says a reader arrives at a focused plan through *"the
> focused engine, demo, game, or campaign document linked by the selected queue
> row"* — so a plan with no queue or tracks row has no entrance. A sweep of all
> 278 non-archive documents found exactly three with zero inbound references, and
> the other two are CLOSED receipts that do not need one. This page is **OPEN**,
> with M3 still outstanding.
>
> ⇒ The tooling it describes is not lost — `scripts/render_take_diagnostic.py`
> and two other scripts name the inspector in prose — but a person following the
> planning docs never meets the plan, only the code. ⚠ The fix is a row in
> `queue.md` or `tracks.md`, which belong to another agent's hot set; reported
> there rather than taken here.

Status: **OPEN** — M1, M2, M4, M5, M6 and M7 closed. What remains is M3's
art/geometry AGREEMENT measurements, which need the render to expose its camera
transform. Ten of ten exit criteria hold; see the table.

## Purpose

Build a canonical inspection surface for authored combat behavior so a human or LLM can answer:

* what move was authored;
* what the runtime actually resolved;
* where the actor and target were;
* where attack and damageable geometry existed;
* what contacted what;
* what consequences the contact produced;
* what animation, projectile, summon, audio, and VFX accompanied the move;
* what follow-up actions were available;
* and how a change altered those results.

`tools/ambition_moveset_inspector` is the first major frontend for this capability.

The architectural goal is larger than one viewer:

> Ambition should expose enough deterministic semantic combat state that an agent can author a mechanic, execute a representative scenario, inspect its consequences numerically and visually, compare it against a previous result, and refine it without requiring a human playtest for every iteration.

This is an Engine 1.0 authoring and observability capability.

It is particularly important for LLM-first development. The preferred loop is:

```text
discover
→ author
→ execute representative scenario
→ inspect runtime consequences
→ compare against intent/baseline
→ refine
→ validate in the actual game
```

The inspector must use real runtime behavior rather than implement a simplified combat model of its own.

---

# Current problem

The existing moveset inspector contains useful pieces, but its current architecture limits its value as an authoring instrument.

## Inspection coverage depends too much on pre-recorded takes

The resolved moveset export can discover the broader character roster, but Engine Takes are populated from the corpus that has actually been recorded.

A character should not become inspectable only after somebody has generated a bulk take for it.

Full-corpus recording is still useful. A roughly half-hour regeneration is acceptable for an overnight workflow, especially when run on a GPU host.

The problem is the dependency:

```text
inspectable character
    !=
character with an existing cached take
```

The roster and move inventory must come from prepared runtime content. Recorded artifacts are caches and evidence.

---

## Subject and target are not semantically clear

Several current capture paths construct symmetric or same-character matchups and distinguish participants primarily through seat or presentation styling.

That is insufficient for authoring inspection.

Every scenario needs an explicit semantic distinction among:

* subject under inspection;
* target;
* subject-owned projectile/summon;
* target-owned projectile/summon;
* other world entities.

The inspector should never require an observer to infer which fighter is the move authoring subject.

---

## Attack geometry is only half of the interaction

The engine already resolves runtime hurt/damageable geometry through concepts such as `ResolvedHurtboxes` and `DamageableVolumes`.

The inspection trace must expose both:

* attack volumes;
* damageable/hurt volumes.

An authored combat tool that shows only the attack side cannot explain whether apparent contact is real, why an attack missed, or whether art and mechanics agree.

---

## Structured traces and rendered captures are not yet one observation

Current headless/structured traces and rendered captures can originate from different executions.

That creates avoidable ambiguity in:

* coordinate systems;
* exact simulation state;
* frame correspondence;
* transient VFX/projectile timing.

The long-term inspection artifact should derive all evidence for one scenario from the same semantic execution or from executions proven equivalent by a shared deterministic scenario description.

---

## The tool shows motion better than causality

The existing inspector can show useful frame and animation information, but the authoring problem is causal:

> Why did this move hit here?
> Why did it miss?
> Why did the target launch that way?
> Why did this follow-up fail?
> Was the VFX aligned with the actual active frame?

The inspector must make those answers first-class.

---

# Architectural principles

## 1. The runtime is authoritative

Do not implement an alternate combat evaluator inside the inspector.

The observatory should construct deterministic scenarios, run the real Ambition systems, and publish semantic read models from them.

The same rule applies to:

* hitboxes;
* hurtboxes;
* movement;
* attack timing;
* contact;
* damage;
* launch;
* status effects;
* projectile behavior;
* VFX requests;
* action availability.

If the runtime cannot expose an important fact cleanly, improve the runtime's semantic observability rather than duplicating its logic in tooling.

---

## 2. Coarse visualization is acceptable; inaccurate geometry is not

The first priority is correct geometry and consequences.

A diagnostic canvas with simple body silhouettes and faithful runtime volumes is more valuable than a polished GPU render with approximate or independently reconstructed geometry.

High-quality rendering can remain relatively slow and can be generated:

* per-character during active iteration;
* as a bulk overnight corpus;
* on a GPU-capable host.

Do not distort the inspection architecture around making the full visual corpus cheap to regenerate.

---

## 3. Cached artifacts are evidence, not authority

The complete character and moveset inventory comes from the prepared engine/content model.

Generated takes, filmstrips, screenshots, and reports are caches keyed by the exact scenario and source/build identity.

A missing cache entry must not make a fighter disappear from the inspector.

---

## 4. Inspection scenarios are semantic objects

The primary unit is not a screenshot or a recorded take.

It is a deterministic `CombatScenario`.

Conceptually:

```text
subject
target
move or action
starting posture/state
spacing
damage state
charge
target behavior
environment
simulation horizon
```

A scenario produces one observation artifact.

---

## 5. Subject and target are explicit

Every trace and visualization should classify relevant entities by semantic role.

At minimum:

```text
Subject
Target
SubjectOwned
TargetOwned
Other
```

These roles should appear in structured data and presentation.

---

## 6. Runtime geometry is the comparison standard

The art pipeline should be compared to combat geometry.

Combat geometry should not be inferred from art in the inspector.

If future authoring workflows support art-derived geometry, the final resolved runtime volumes remain what the inspector displays and measures.

---

## 7. Machine-readable inspection is a first-class product

The browser is one consumer.

A CLI or library caller must be able to request a scenario and receive compact structured results without opening an interactive UI.

This is required for LLM-first authoring, regression analysis, and CI.

---

# Core data model

## CombatScenario

Introduce or evolve the existing move-exercise vocabulary toward one shared scenario description.

Representative fields:

```text
subject character
target character / passive target profile

requested move
activation mode

subject posture
target posture

subject position
target position
spacing/orientation

subject damage
target damage

subject velocity
target velocity

charge amount

target behavior
    passive
    shielding
    airborne
    captured
    scripted movement
    real fighter behavior

environment
    flat-ground
    edge
    airborne
    custom authored fixture

horizon
```

Do not attempt to cover every combat state in the first implementation.

The structure should be extensible without making each tool invent its own scenario vocabulary.

---

# Two activation modes

The observatory needs to distinguish two different authoring questions.

## Input-path inspection

Exercise:

```text
physical/logical input
→ gesture interpretation
→ action/repertoire resolution
→ move acceptance/rejection
→ behavior
```

Use this to inspect whether the authored move is actually reachable from normal controls.

---

## Direct resolved-move inspection

Exercise a selected prepared move through a sanctioned diagnostic activation path.

Use this when the question is:

> What does this authored move do?

This mode may bypass input mapping, but it must not bypass the real move behavior/runtime systems.

The report must clearly state which activation mode was used.

Do not mutate internal playback state ad hoc from the tool.

If no sanctioned runtime seam exists, add a narrow diagnostic move-attempt interface shared with the real runtime.

---

# Required observation model

For every simulated frame, publish enough semantic information to reconstruct the combat interaction.

## Identity

* subject and target character IDs;
* stable simulation IDs where applicable;
* semantic scenario role;
* move/action ID;
* authored/provider provenance where useful.

## Body state

* world position;
* velocity;
* facing;
* grounded/airborne state;
* relevant posture;
* current animation/pose identity.

## Move state

* move clock;
* move phase;
* startup / active / recovery classification;
* charge;
* landed-hit state;
* active cancel/follow-up policy.

## Attack geometry

For each resolved attack volume:

* owner;
* attack/hitbox identifier;
* shape;
* local geometry;
* world-space geometry;
* active interval;
* attack properties required to interpret consequences.

## Damageable geometry

For subject and target:

* resolved `DamageableVolumes`;
* world-space shape;
* source:

  * move override;
  * pose profile;
  * default body;
  * other runtime source;
* intangible/invulnerable state where relevant.

## Contact

When an interaction occurs:

* attack volume;
* damageable volume;
* simulation tick;
* contact/intersection position or region;
* owning entities.

## Consequence

As available from existing runtime semantics:

* damage;
* knockback/launch vector;
* resulting target velocity;
* hitlag;
* hitstun/recoil;
* invulnerability;
* status/effect application;
* subject recoil or momentum change.

## Spawn/effect events

Timestamp and attribute:

* projectile creation;
* summon creation;
* VFX request;
* SFX request;
* gameplay impulse;
* screen/presentation effect where semantically related.

## Control/action result

For input-path scenarios:

* requested input;
* interpreted action;
* selected move;
* acceptance/rejection;
* rejection reason where available.

---

# Milestone M1 — Geometry truth

This is the highest-priority slice.

## Goal

Make any supported prepared fighter/move inspectable with runtime-faithful attack and damageable geometry.

## Requirements

The inspector must enumerate characters and moves from prepared content rather than the cached take population.

For a selected move/scenario it must provide:

* explicit subject;
* explicit target;
* subject and target transforms;
* facing;
* move clock and phase;
* resolved attack volumes;
* resolved target damageable/hurt volumes;
* world-space geometry;
* contacts.

The diagnostic rendering may remain coarse.

## Default target

Provide a passive deterministic target suitable for isolated move inspection.

Use a real combat/damageable body through the normal runtime systems.

A live CPU opponent should be optional rather than the default because independent opponent decisions add noise to move inspection.

Any real fighter should still be selectable as a target.

## Acceptance

From a fresh generated-artifact directory:

1. the inspector lists every resolved supported fighter;
2. selecting a fighter exposes every supported resolved move, independent of cached takes;
3. the user can inspect a move without running a full-grid regeneration;
4. subject and target cannot reasonably be confused;
5. every active attack volume is drawn from runtime-resolved geometry;
6. every target damageable volume is drawn from runtime-resolved geometry;
7. overlap/contact in the visualization agrees with the combat runtime.

Representative validation should include:

* ordinary melee attack;
* moving attack;
* multihit;
* posture-dependent hurtbox;
* projectile attack;
* summon-assisted move.

---

# Milestone M2 — Geometry measurements

**CLOSED.** `scripts/moveset_report.py` derives every metric below from the
runtime's published observation and writes `report.json` + `summary.md`; the
before/after half of M7 is the same tool's `--against`. ⛔ Its standing rule:
`overlap_ticks` (this script measuring boxes) and `contacts` (the runtime's
hit-once memory) are separate lines and the summary warns when the first is
nonzero and the second is zero. Measured on the admiral's jab at 38px: 5 ticks of
overlap, ONE resolved contact.

Once the geometry is trustworthy, derive useful quantitative measurements.

Do not create theoretical authored measurements when resolved runtime measurement is available.

Representative metrics:

* startup ticks;
* active ticks;
* recovery ticks;
* first active frame;
* first contact frame;
* last contact frame;
* maximum reach from subject body origin;
* horizontal and vertical attack extents;
* attack-volume area over time;
* target overlap duration;
* subject travel before first active frame;
* subject travel during active frames;
* target displacement after contact;
* launch velocity;
* projectile/summon spawn position and timing.

The structured report should make these directly accessible to an agent.

## Acceptance

An agent should be able to answer questions such as:

* How many pixels beyond the body does this attack reach?
* How far does the subject travel before the first active frame?
* For how many ticks can this move intersect a stationary opponent?
* How high above the fighter is the attack actually active?
* Which part of a multihit is failing to catch the target trajectory?

without manually deriving the answer from Rust source.

---

# Milestone M3 — Art/geometry agreement

**The overlay half is CLOSED** (M1.5): `moveset_render` forces the production
combat overlay on, so one PNG carries the real art, the real target, the real
VFX and the real `CombatGeometryView` volumes from one execution. Verified by
pixels rather than by a flag — an overlay-on and an overlay-off render of the
same action tick differ, and the strike volume is visible over the art at the
tick the manifest says it is live.

**The layer toggles are DONE.** `moveset_render --overlay
on|off|art,hurtboxes,strikes` drives them independently, through one definition
of the gates (`dev_tools::force_combat_overlay` takes a `CombatOverlayLayers`),
and the manifest records which were on — so a PNG with no cyan on it can be told
apart from a body with no hurtbox.

⭐ MEASURED IN PIXELS, on one tick of the admiral's jab at 38px:

| render | cyan px | red px |
|---|---|---|
| `--overlay hurtboxes` | 815 | 33 |
| `--overlay strikes` | 12 | 445 |

`on` and `hurtboxes` differ in 16376 pixels. ⛔ AND THE OBSERVATION IS UNCHANGED
— both manifests record the same one live strike on that tick. A layer toggle
changes what is DRAWN, never what is measured, which is the only reason it is
safe to turn one off.

⛔ The `Combat` debug PRESET turns on the COMBINED gate
(`show_feature_hitboxes`), which draws both halves whatever the per-layer fields
say — so asking for one layer means clearing it. That is why the toggles could
not be had by setting two booleans.

**Still open**: `trajectories`, `contact markers` and `VFX` have no independent
gate in the overlay, and the agreement MEASUREMENTS (visual weapon tip versus
attack extent, a volume mostly inside the body, VFX centre versus contact point)
need the render to publish its camera transform, which it does not. That
transform is the one architectural gap left in this whole program.

## Goal

Overlay authoritative runtime geometry onto the rendered character/game frame.

The intended presentation is:

```text
actual character art
actual target art
actual VFX/projectiles
+
runtime attack volumes
runtime damageable volumes
contact markers
trajectories
labels
```

All visible evidence must correspond to the same scenario and frame.

## Required toggles

At minimum:

* art;
* hitboxes;
* hurtboxes;
* body origins;
* trajectories;
* contact markers;
* projectiles/summons;
* VFX;
* semantic labels.

## Measurements

Support useful art/mechanics agreement questions such as:

* visual weapon tip versus attack extent;
* attack volume mostly inside the character body;
* visible attack with no corresponding active volume;
* active volume appearing before the visual action;
* VFX center versus actual contact;
* projectile sprite versus projectile collision volume.

Do not automatically “correct” authored geometry from those measurements. The observatory exposes evidence.

---

# Milestone M4 — Key-frame filmstrip

**CLOSED for the diagnostic sheet.** `render_take_diagnostic.py --select key`
(the default) picks the ticks that mean something — opening pose, last startup,
first live volume, first contact, max reach, spawns, last active, end of recovery
— and labels each cell with why it was chosen. It reuses `moveset_report.measure`
rather than deriving "first contact" a second way. ⛔ The failure it removes: a
jab is live for five of a hundred and fifty ticks, so an even strip of twelve
usually misses every one of them and shows a fighter standing still.

Long animation sequences are expensive for both humans and LLMs to inspect frame by frame.

Generate a compact filmstrip from semantically important frames.

Candidate selection:

* initial pose;
* last startup frame;
* first active frame;
* first contact;
* maximum reach;
* major projectile/summon event;
* last active frame;
* first follow-up/cancel opportunity;
* final recovery frame.

Annotate each frame with:

* tick;
* move phase;
* subject/target identity;
* important geometry;
* important event.

The full frame corpus can still be available when needed.

---

# Milestone M5 — Consequence tracing

**The derivable half is DONE.** `moveset_report.py` publishes a
`consequence_chain`: for each runtime-resolved contact, what the victim's own
published state did across it. Measured on the admiral's forward smash —

```text
tick 47 (0.7833s): strike smash_forward/w1/v0 → seat1 (target)
    damage taken: 0 → 22
    hitstun: 0 → 0.336
    hitlag: 0 → 0.105
    velocity: [0.0, 0.0] → [193.6, -81.3]
    displacement over the next 12 ticks: 0 → 15.252
```

⛔ Every number is DIFFERENCED from what the runtime published, never recomputed
from a knockback formula — a second implementation of the launch rule is exactly
what this program exists to remove.

⛔⛔ AND IT SAYS WHAT IT CANNOT ANSWER. This is WHAT changed, not WHY. The
resolution vocabulary — ignored / blocked / armored / wallet-shielded / damaged —
is the engine's own and travels on `ambition_damage::BodyHitResolved` behind the
`causal` feature, alongside `BodyReactionApplied`. Consuming those is the
remaining M5 slice, and the road is already built: the monolith's `causal.rs`
turns both into facts with a cause chain, `clear_message_on_rollback` keeps them
honest across a rewind, and `ambition_platformer2d::causal` is the SDK surface —
a recorder installs `CausalPlugin`, sets `RecordingPolicy::only([domains::DAMAGE,
domains::MOVESET])`, and exports `log.facts()` joined to the take's frames by sim
tick. ⚠ The take does not record an absolute `sim_tick` per frame yet; that join
key is the one missing piece.

Status effects, VFX and SFX are untouched.

## Goal

Make the inspector explain what contact actually did.

Add synchronized inspection for:

* damage;
* launch/knockback;
* hitlag;
* hitstun/recoil;
* velocity;
* status effects;
* impulses;
* projectiles;
* summons;
* VFX;
* SFX.

The agent should be able to select a contact and see the causal consequence chain.

Example:

```text
tick 12
  hitbox "sword_sweetspot"
  intersects target torso

  damage: 12
  target velocity: ...
  hitlag: ...
  hitstun: ...

tick 12
  vfx: slash_large
  sfx: sword_heavy

tick 13
  target displacement: ...
```

---

# Milestone M6 — Move-chain and combo laboratory

**The empirical A → B probe is DONE.** `moveset_takes --chain VERB --chain-at
TICK` drives a second verb through the same press table
(`move_exercise::chained_frame`, so a chain presses exactly what a single take
presses up to the hand-off), and the report answers the plan's questions from the
recording.

⛔ **THE SCHEDULE STAYS A PURE FUNCTION OF THE ACTION TICK.** A probe that waited
for A to connect before pressing B would make the press depend on the outcome it
is measuring. `--chain-at` is an INPUT; sweep it.

Measured on the admiral's jab into itself, at 38px, against a passive target:

| requested | accepted | note |
|---|---|---|
| 8 | never | the press landed inside A's own playback and the engine played nothing |
| 14 | 18 | **buffered for 4 ticks** — the request is not the acceptance |
| 18 | 18 | immediate |
| 22 | 22 | immediate |

In every accepted case B's box overlapped the target and the runtime resolved NO
hit, which the report says in those words rather than calling it a combo.

⛔ **"THE ENGINE NEVER PLAYED IT" IS AN ANSWER, NOT A MISSING SECTION.** A report
that omitted the chain when B did not come out would leave a reader thinking the
probe had not run.

**The authored action graph is DONE too.** `moveset_export` resolves every
`Cancelable` window's rule into the moves it admits FOR THAT FIGHTER
(`cancel_into_resolved` beside the authored `cancel_into`), and the Fighter view
shows both with the window's frame range and condition.

⛔⛔ **THE EXPORTER RESOLVES IT, NOT THE BROWSER.**
`MovesetContract::cancel_targets` matches on `cancel_names_for` — the same
verb-class list the trigger road matches on, which moved to the catalog beside
`CANCEL_CLASS_NAMES` so there is one copy rather than two that must agree.

⛔⛔ **AND THE NAMESPACE HAD TO BE TOTAL, NOT A FALL-THROUGH.** The first version
treated everything that was not a special as attack-family, and a real export
showed what that costs: the admiral's `ranged` cancel resolved into **23 moves,
including every grab, pummel, throw and the taunt**. On the trigger road a grab
passes `[grab]`, a capture action passes its own FULL verb
(`capture_throw_forward`, which is not `base_verb_of` of it), and taunt and
`ranged` pass one name each — so `cancel_names_for` now answers for every arm and
returns EMPTY to mean "this verb answers to its own name". Found by looking at
exported data, not by reading the code.

⚠ The plan's original instruction — *do not begin here; this depends on
trustworthy single-move inspection* — was right, and is why this closed last.

## Authored action graph

For the selected move, expose:

* authored cancel rules;
* timing windows;
* resolved follow-up repertoire for this character and posture.

Broad rules such as `any_attack` should be resolvable into actual candidate moves.

## Empirical A → B probe

Run a deterministic two-action scenario and report:

* contact tick of A;
* earliest requested B input;
* B acceptance tick;
* B first active tick;
* target hitstun end;
* target position/velocity;
* whether B's geometry reaches;
* B contact tick if successful.

Prefer reporting hard facts rather than prematurely classifying every sequence as a "true combo."

Ruleset-specific combo classification can be added later.

---

# Milestone M7 — Agent-native artifact and diff loop

**CLOSED except the render half.** `moveset_report.py --out DIR` writes the
bundle:

```text
report.json     the machine-readable authority, with provenance
summary.md      the causal read, for a person or a model
trace.jsonl     one line per tick, for the question the report did not anticipate
filmstrip.svg   the key-frame sheet, from the one tool that draws one
```

Provenance names the source recording and its mtime plus all three schema
versions, so a report derived from a recording made before a tuning change
cannot read as current. `--against` is the before/after diff, and it REFUSES to
present two different scenarios as one change.

⚠ Still open: `render/` — the GPU frames are written by `moveset_render` into
its own directory and are not yet gathered into the bundle.

Every scenario should produce a reusable artifact bundle.

Representative layout:

```text
inspection/
    report.json
    summary.md
    trace.jsonl
    filmstrip.png

    frames/
        ...

    render/
        ...
```

`report.json` is the machine-readable authority.

`summary.md` gives a concise causal interpretation suitable for an agent or reviewer.

The browser should consume the same artifacts rather than maintain an independent data model.

---

## Semantic cache identity

Artifacts must record enough provenance to prevent stale results from appearing current.

At minimum:

* subject character;
* target;
* scenario parameters;
* resolved move identity;
* prepared-content fingerprint;
* relevant source/build identity;
* inspector/schema version;
* render configuration when visual artifacts are involved.

---

## Before/after comparison

Support comparing two compatible inspection artifacts.

Report changes such as:

```text
startup         13 → 11
active frames   unchanged
recovery        19 → 16
maximum reach   +23 px
first contact   16 → 14
launch speed    +8%
hurtbox         unchanged
VFX timing      12 → 11
```

Visual diffing should eventually support:

* before/after geometry paths;
* contact locations;
* subject trajectory;
* target trajectory.

This is one of the highest-value capabilities for LLM-first authoring.

---

# Browser organization

The primary workspace should be organized around:

```text
Fighter
→ Move
→ Scenario
```

rather than around recorded artifact types.

Example:

```text
Subject:  Pirate Admiral
Move:     Up Special — Call the Shark
Scenario: Passive target
Target:   Sandbag
```

## Primary pane

A synchronized inspection viewport showing:

* subject;
* target;
* art where available;
* runtime geometry;
* timeline;
* key events.

## Supporting panes

* move metadata;
* numerical measurements;
* cancel/follow-up graph;
* event/consequence trace;
* provenance;
* before/after comparison.

"Engine Take" should be an implementation/cache concept, not a primary author-facing mental model.

---

# Rendering and corpus policy

Bulk high-quality rendering does not need to be interactive.

A roughly half-hour full-grid redraw is acceptable for:

* overnight agent campaigns;
* CI/regression snapshots;
* release checkpoints;
* GPU-host generation.

The fast authoring loop should support:

* one move;
* one character;
* one scenario;
* coarse faithful geometry.

If high-quality iteration benefits substantially from a GPU host, use the GPU host rather than redesigning the system around weak local rendering hardware.

Do not optimize the full-grid GPU path before the semantic inspection path is trustworthy.

---

# Integration with existing planning authorities

## `authoring-and-tools.md`

This plan is a primary consumer of the agent-first authoring loop.

That document should link here for combat authoring/inspection rather than duplicate the mechanics-specific design.

---

## `inspection-diagnostics-and-workbench.md`

The moveset observatory should use generic engine inspection facilities where appropriate:

* pause/step;
* deterministic scenario execution;
* structured query surfaces;
* trace capture;
* geometry overlays.

Do not introduce combat-only infrastructure when the generic inspection capability is reusable.

---

## `sprite-renderer.md`

The existing melee hitbox-agreement question should resolve toward:

> display and measure the authoritative runtime geometry over the rendered art.

The sprite tool is not an independent authority for combat volumes.

---

## Smash parity / training tooling

The Smash live hitbox/hurtbox overlay requirement should use this engine capability.

Do not create an unrelated Smash-specific geometry implementation.

---

# Relationship to broader Engine 1.0 goals

This program is not only a Smash tool.

It exercises several capabilities required for a credible LLM-first 2D engine:

* deterministic scenario construction;
* semantic runtime observability;
* machine-readable inspection;
* visual verification;
* authored/runtime agreement;
* action and consequence tracing;
* before/after behavioral diffing.

Combat is a strong first customer because geometry, timing, animation, state, and effects interact densely and errors are visually obvious.

The same inspection architecture should later inform:

* NPC actions;
* projectiles;
* traps;
* moving hazards;
* environmental interactions;
* scripted encounter mechanics.

Do not generalize those surfaces until the combat observatory proves the abstractions.

---

# Initial queue slice

The first implementation slice should be narrow and high leverage.

## D-COMBAT-INSPECT — runtime-faithful move geometry

1. Enumerate all prepared supported fighters and moves independently of cached takes.
2. Introduce explicit subject and target roles.
3. Add a deterministic passive-target move scenario.
4. Publish per-frame:

   * transforms;
   * facing;
   * move tick/phase;
   * resolved attack volumes;
   * resolved damageable/hurt volumes;
   * contact pairs.
5. Draw those volumes faithfully in `ambition_moveset_inspector`.
6. Make one-character / one-move regeneration straightforward.
7. Preserve full-grid generation as an offline/overnight operation.
8. Validate representative:

   * melee;
   * moving attack;
   * multihit;
   * posture-dependent hurtbox;
   * projectile;
   * summon-assisted move.

Do not spend this first slice on:

* bulk-render optimization;
* combo classification;
* polished UI transitions;
* general-purpose scene editing;
* a separate collision model;
* visual scripting;
* reproducing Godot-style human-first authoring workflows.

---

# Exit criteria

All ten hold. Status, so what remains is legible as the incremental work it is:

| # | criterion | status |
|---|---|---|
| 1 | every fighter/move selectable independently of cached artifacts | ✔ 21 grid fighters offered with one recorded; an unrecorded fighter exposes all 26 of its moves |
| 2 | a deterministic scenario inspects subject and target through the real runtime | ✔ `--target`, `--target-behavior passive`, `--spacing` |
| 3 | attack and damageable geometry published from runtime authority | ✔ `CombatGeometryView` only; guarded by an absence contract |
| 4 | contacts and consequences machine-readable | ✔ contacts, damage, hitstun, hitlag, launch, and — with `--features causal` — the engine's own resolution (`outcome: damaged, raw_damage: 4, source: Melee`) |
| 5 | rendered evidence aligned with the same semantic scenario | ✔ one execution, overlay + shutter-time observation |
| 6 | agents generate and consume a compact artifact noninteractively | ✔ `moveset_takes --verbs`, `moveset_report.py`, SVG sheets |
| 7 | before/after behavioural comparison | ✔ `--against` |
| 8 | the browser consumes the same semantic artifacts | ✔ it draws the recorded observation; it derives no geometry |
| 9 | representative move-chain inspection | ✔ both halves: the authored cancel graph, resolved per fighter, and the empirical A→B probe with the buffered-acceptance case measured |
| 10 | the major remaining work is UX/coverage/performance, not observability | ✔ — and MORE true than the note below claimed; see the 2026-09-03 re-measurement |

⭐ **RE-MEASURED 2026-09-03 — M3's remaining work is smaller than "the render
does not publish the camera transform" suggests, and the difference is
architectural versus incremental.**

The transform's components ARE published, per view, and are already consumed by
production render systems:

* `ambition_sim_view::CameraViewState` is a per-view COMPONENT carrying
  `center_world`, `visible_view`, `orthographic_scale`, `zoom_multiplier` and
  `target_world` (`crates/ambition_sim_view/src/camera_snapshot.rs:1574`);
* `CameraViewport` publishes the view rectangle in logical pixels (`:844`) as an
  observer fact beside it;
* and they are not diagnostics-only — `rendering/knockout.rs` places a beat
  against *"the published camera rect, never a second copy resolved here"*, and
  `rendering/nameplates.rs` reads `target_world`.

⇒ **What is missing is a COMPOSITION, not a publication.** A grep for
`world_to_screen` / `screen_from_world` finds nothing: no helper turns
`center_world` + `orthographic_scale` + viewport px into a pixel mapping, so each
consumer re-derives the part it needs. That is a small helper plus a decision
about who owns it — incremental work, which is exactly what criterion 10 asserts
— rather than a render-side publish the inspector has to wait for.
⚠ It does NOT follow that the agreement measurement is free: composing the
mapping is the easy half, and deciding what "agreement" means in pixels (which
anchor, what tolerance, at which zoom) is the half this page still owes.

This architecture program can leave active planning when:

1. every supported fighter/move can be selected independently of cached artifacts;
2. a deterministic scenario can inspect subject and target through the real combat runtime;
3. attack and damageable geometry are published from runtime authority;
4. contacts and consequences are machine-readable;
5. rendered evidence can be aligned with the same semantic scenario;
6. agents can generate and consume a compact inspection artifact noninteractively;
7. before/after behavioral comparison exists for authored changes;
8. the browser consumes the same semantic artifacts rather than defining a second behavior model;
9. representative move-chain inspection exists;
10. the major remaining work is incremental UX, coverage, or performance rather than architectural observability.

The end state is not a prettier moveset browser.

The end state is a combat observatory that makes authored mechanics empirically inspectable.


# Existing implementation map

This program must extend the existing inspection stack rather than construct a parallel combat harness.

The important existing pieces are listed below. Treat them as the starting architecture.

## 1. Prepared roster and moveset authority

**Existing tool:**

```text
game/ambition_app_tools/src/bin/moveset_export.rs
```

`moveset_export` already boots the composed host and exports what the prepared game resolves, rather than parsing authored Rust/files directly.

It already owns the browser-facing schema for:

* character inventory;
* Smash grid membership;
* resolved repertoire;
* moves;
* move windows;
* authored attack-volume metadata;
* cancel information;
* sprite-sheet atlas information.

### Required use

The inspector's fighter/move inventory must come from `moveset_export`.

Do not derive the available fighter list from recorded takes.

The current browser function:

```text
tools/ambition_moveset_inspector/web/app.js
    takeFighters()
```

derives the Engine Takes fighter picker from `TAKES.takes`.

That is the immediate cause of the partial-roster behavior.

Change the browser so:

```text
prepared bundle = what exists
take/render cache = what has already been generated
```

Missing cached evidence should be shown as missing evidence, not as a missing fighter.

---

# 2. One existing authority for "perform this move"

**Existing module:**

```text
crates/ambition_sim_harness/src/move_exercise.rs
```

This already owns the shared move-driving behavior used by both:

```text
moveset_takes
moveset_render
```

It contains measured rules for:

* tilt versus Smash stick magnitude;
* aerial preparation;
* facing/turnaround settling;
* press versus hold edges;
* charge/release timing;
* settling between moves;
* determining the intended prepared move;
* detecting whether the engine actually performed that move.

There is already an architectural guard in:

```text
scripts/check_absence_contracts.py
```

under:

```text
the-two-move-drivers-do-not-author-their-own-presses
```

preventing the two drivers from inventing separate move-input schedules.

### Required use

Do not add move-driving logic inside the inspector, Python server, browser, or another binary.

Evolve `move_exercise` when additional scenario preparation is necessary.

A reasonable progression is for today's:

```text
Verb
```

to eventually participate in a richer scenario description containing:

```text
subject
target
posture
spacing
target state
move exercise
```

Do not require a general `CombatScenario` framework before M1. Extend the existing harness incrementally.

---

# 3. Existing deterministic headless take recorder

**Existing tool:**

```text
game/ambition_app_tools/src/bin/moveset_takes.rs
```

This already:

* boots the real composed Smash host in `NoWindow`;
* seats a real match;
* drives `move_exercise`;
* records every simulation tick;
* records stable `SimId` identity;
* records body position/velocity/facing;
* records move identity;
* records resolved gesture;
* records pose/clip information;
* records projectiles;
* records exact runtime strike shapes;
* canonicalizes output order for stable comparison;
* records whether a projectile/strike belongs to the subject.

This should remain the cheap full-tick recorder.

### Important current debt

`moveset_takes::sample` currently reconstructs much of its combat observation by directly querying:

```text
BodyKinematics
Hitbox
MovePlayback
...
```

The engine now has a better semantic observation boundary.

Refactor the combat-geometry portion of this sampler to consume:

```text
ambition_sim_view::CombatGeometryView
```

rather than maintaining a second geometry extraction algorithm.

The take recorder may still join view entities against simulation identity/presentation facts such as:

```text
SimId
MatchSeat
WornCharacter
```

when stable artifact identity requires them.

The rule is:

> geometry and combat state come from the semantic sim-view; artifact identity/provenance may be joined at the tool boundary.

---

# 4. Existing authoritative combat geometry read model

**Existing module:**

```text
crates/ambition_sim_view/src/combat_geometry_view.rs
```

**Existing resource:**

```text
CombatGeometryView
```

This is the default source for M1 geometry.

It already publishes:

## Per combat body

```text
collision
hurtboxes
damage_taken
facing
hitstun_s
hitlag_s
landing_lag_s
jump_squat_s
velocity
grounded
wall contact
move state
```

The move state contains:

```text
move id
authored phase/window
elapsed time
duration
attack-facing
landed-hit
```

## Per live strike

```text
exact CombatVolume
owner
strike entity
body-tracking versus world-anchored
```

The strike volume is already resolved into world space using the same `Hitbox::world_volume` semantics used by combat.

The effective hurtboxes already preserve the runtime's three-way damageable-volume rule.

### Required use

Do not independently resolve hitbox/hurtbox geometry inside `moveset_takes`, JavaScript, or Python.

Serialize this view.

If the observatory needs additional semantic facts, extend the read model narrowly.

Examples that may justify extensions later:

* stable semantic identity suitable for persisted artifacts;
* hurtbox provenance such as move override versus pose/default;
* explicit contact/resolution facts.

Do not make the inspector reach back into private combat ECS merely because one field is missing.

---

# 5. Existing coarse diagnostic renderers

There are already two useful non-game visualization paths.

## Browser canvas

```text
tools/ambition_moveset_inspector/web/app.js
    drawTake()
    drawHitboxShape()
```

It already draws:

* body positions;
* sprite-derived art when available;
* runtime-recorded hitboxes;
* projectiles;
* platforms.

Extend this renderer for M1 rather than creating another browser visualization.

Add:

* cyan effective hurtboxes;
* explicit `SUBJECT` / `TARGET` labels;
* stable role styling;
* target-owned versus subject-owned strikes/projectiles;
* move phase/tick readout.

## Headless SVG diagnostic renderer

```text
scripts/render_take_diagnostic.py
```

This already produces no-GPU SVG contact sheets from `moveset_takes`.

Extend it to consume the same schema.

This is a useful LLM/CI artifact path because it requires no render device and can create small geometry-faithful filmstrips quickly.

Do not create a second Python/SVG geometry implementation with a different schema.

---

# 6. Existing real GPU renderer

**Existing tool:**

```text
game/ambition_app_tools/src/bin/moveset_render.rs
```

It already:

* boots the real visible Ambition composition using:

```text
VisibleRenderMode::OffscreenGpu
```

* uses the same `move_exercise` as `moveset_takes`;
* renders one character + verb on demand;
* records action tick and simulation tick;
* records the engine's semantic pose/clip decision;
* checks that the requested move actually occurred;
* reports the actual WGPU adapter;
* can run through hardware or software rendering.

Do not replace this binary.

Extend it.

---

# 7. Existing deterministic GPU capture session

**Existing module:**

```text
crates/ambition_render/src/capture.rs
```

**Existing type:**

```text
DeterministicCaptureSession
```

This already solves an important problem the observatory must not solve again:

> service GPU readback without advancing simulation time.

It:

* captures the current simulation tick;
* pumps rendering at zero-duration time;
* verifies that the simulation tick did not advance;
* refuses a frame if it did;
* restores the canonical simulation period afterward.

### Required use

Any rendered inspection evidence must continue through this capture path.

When semantic geometry is associated with a rendered frame, sample the semantic observation **before the shutter**, as `moveset_render` already does for move/pose state.

Do not query semantic simulation state after the zero-time render pump and label it as the shutter state.

---

# 8. Existing real-render geometry renderer

**Existing renderer:**

```text
crates/ambition_render/src/rendering/debug_viz.rs
    draw_combat_geometry_view
```

This already draws `CombatGeometryView` over the game's actual rendered presentation.

It includes:

* coarse collision envelope;
* effective hurtboxes;
* exact live strike volumes;
* move phase/timing visualization;
* velocity/launch information;
* hitlag/hitstun/landing-lag readout;
* facing versus committed attack-facing.

The production developer overlay already uses this semantic view.

### M1/M3 implication

Before building browser-side world-to-camera overlay machinery, investigate whether `moveset_render` can enable the existing combat debug visualization during its OffscreenGPU capture.

If so, this is the preferred first implementation of:

> actual art + actual runtime combat geometry in one image.

That eliminates the current problem where the GPU screenshot and diagnostic boxes come from different runs/coordinate systems.

Machine-readable geometry must still be serialized separately; the debug render does not replace structured inspection.

---

# 9. Existing on-demand inspector server

**Existing server:**

```text
tools/ambition_moveset_inspector/ambition_moveset_inspector/server.py
```

The existing endpoint:

```text
GET /api/render
```

already invokes `moveset_render` for:

```text
character + verb + frame count + stride
```

and caches results by the requested move.

It already:

* locates debug/release binaries;
* detects stale renders;
* reports unavailable render capability as structured JSON;
* refuses mismatched move captures;
* exposes rendered frames to the browser.

### Required use

Do not introduce a separate rendering service for M1/M3.

Extend this endpoint or add a sibling inspection endpoint only when the returned schema genuinely differs.

The eventual server shape may become:

```text
/api/inspect
/api/render
/api/status
```

where inspection is semantic and rendering is an optional artifact of the same scenario.

Do not require that refactor for the first geometry slice.

---

# 10. Existing causal hit instrumentation

Do not infer actual combat outcomes from geometry when the runtime already announces them.

Relevant existing facts include:

```text
ambition_combat::hitbox::LandedBodyHit
ambition_combat::hitbox::ResolvedBodyHit
```

and, under the `causal` feature:

```text
ambition_damage::BodyHitResolved
ambition_damage::BodyReactionApplied
```

`BodyHitResolved` already exposes the engine's decision vocabulary, including whether a hit was:

```text
ignored
blocked
armored
wallet-shielded
damaged
...
```

`BodyReactionApplied` publishes the resulting reaction/launch information.

The `ambition_app_tools` package already forwards the `causal` feature:

```text
cargo ... -p ambition_app_tools --features causal
```

### Required use

For the consequence-tracing milestone, consume these existing causal facts before introducing new inspector-specific hit events.

Keep two concepts distinct:

```text
geometric overlap
runtime-resolved hit
```

The inspector may measure the first from geometry.

It must use runtime causal events for claims about the second.

---

# 11. Current subject/target problem

`moveset_takes::reseat` currently constructs:

```text
smash_roster([character, character])
```

and `moveset_render` similarly uses the selected character for both participants.

The current take schema then relies heavily on:

```text
subject seat == 0
```

and `subject_owned` booleans.

That is why the resulting evidence is visually ambiguous.

### Required change

Introduce explicit scenario roles at the recording boundary.

At minimum every observed body should resolve to one of:

```text
subject
target
subject_owned
target_owned
other
```

Do not make the browser derive these roles from character names.

The first slice may still use an ordinary real opponent if passive-target composition requires additional work.

However, the subject and target must already be structurally distinct.

Then introduce an explicit target argument, for example conceptually:

```text
--character npc_pirate_admiral
--target <prepared target>
```

The eventual passive target should be created through existing match/body policy rather than by a tool system that freezes or mutates arbitrary combat components.

---

# Concrete M1 implementation sequence

**M1 is CLOSED.** Receipts below; the standing prohibitions each step established
are the part worth keeping.

| step | what was wrong | what closed it | guard |
|---|---|---|---|
| M1.1 | the fighter picker read `TAKES.takes`, so a fighter existed only once recorded | `takeRoster()` / `takeSlotsFor()` enumerate `moveset_bundle.json`; the cache is shown as status | `check_takes_discovery.mjs` |
| M1.2 | `smash_roster([c, c])` plus `seat == 0`: one character twice, told apart by a convention nothing wrote down | `ScenarioRoles` — every body, strike and shot carries `subject`/`target`/`subject_owned`/`target_owned`/`other`; the take names both fighters and their `SimId`s | `a_seated_scenario_serializes_roles_identities_and_both_geometries` |
| M1.3 | `moveset_takes::sample` queried `Hitbox` and called `world_volume` itself, and had no hurtboxes at all | `ambition_sim_harness::combat_observation` serializes `CombatGeometryView`; the recorder resolves nothing | absence contract `the-recorders-do-not-resolve-their-own-combat-geometry` |
| M1.4 | attack volumes only, in one browser canvas | cyan hurtboxes, role labels, phase readout, real shapes — in `app.js` AND `render_take_diagnostic.py` | `test_render_take_diagnostic.py`, `check_draw_path.mjs` |
| M1.5 | the GPU screenshot and the diagnostic boxes came from different runs | `moveset_render` forces the production combat overlay on (`--combat-overlay`, default on): one execution, real art, real volumes | `force_combat_overlay` is one function, shared with `capture_scene` |
| M1.6 | the PNG did not say what it showed | every shot carries its `observation`, sampled BEFORE the shutter | manifest field, same schema as the take |

Two things were added that the sequence did not name and M1 could not be checked
without:

- **spacing.** The match places seats far enough apart that no ordinary move
  reaches, so no take could ever exhibit a contact. `move_exercise::approach`
  walks the subject to `--spacing PX` through the ordinary control frame (it does
  not teleport), and the take records the gap it ASKED for beside the gap it
  REACHED. Measured: the admiral's jab at 33px connects on tick 3 for 4 damage
  and 0.088s of hitstun; the forward smash on tick 47 for 22 and 0.336s.
- **contacts.** `CombatGeometryView` now carries each strike's `HitboxHits` — the
  resolver's own hit-once memory, which is sim truth under rollback — so the
  artifact says what CONNECTED rather than leaving a reader to conclude it from
  overlapping rectangles.

⛔ **THE STANDING PROHIBITION FROM M1.** Geometric overlap and a resolved hit are
two facts and must never be merged. A strike volume inside a hurtbox is not a
hit: the victim may be intangible, on the same team, shielded, or already struck
by that strike. `moveset_report.py` reports `overlap_ticks` and `contacts` as
separate lines and warns when the first is nonzero and the second is zero.

<details>
<summary>The original sequence, as specified</summary>

## M1.1 — Fix discovery before changing simulation

Change the Engine Takes fighter selector to enumerate from:

```text
moveset_bundle.json
```

rather than `TAKES.takes`.

Continue showing:

```text
recorded / missing / stale
```

as cache status.

Acceptance:

> Every prepared Smash fighter is visible immediately even when only two have recorded takes.

---

## M1.2 — Add explicit subject/target roles to the take schema

Extend the take/scenario metadata to identify:

```text
subject character
target character
subject stable id
target stable id
```

and classify bodies/projectiles/strikes by role.

Do not rely on visual styling to establish identity.

Acceptance:

> A screenshot, JSON frame, or diagnostic SVG can be read without knowing seat conventions.

---

## M1.3 — Route take geometry through `CombatGeometryView`

Refactor the combat-geometry portion of:

```text
moveset_takes::sample
```

to serialize:

```text
CombatGeometryView.bodies
CombatGeometryView.strikes
```

Join the view's entity identities to stable `SimId` and scenario roles at the tool boundary.

Keep the existing projectile sampling until an equivalent semantic view owns projectile observation.

Acceptance:

> Headless output contains effective hurtboxes and exact live strike shapes from the same semantic view the production debug renderer uses.

Add a contract/test proving the take recorder no longer directly reconstructs `Hitbox::world_volume` or damageable-volume fallback semantics independently.

---

## M1.4 — Extend the existing diagnostic canvas

Update:

```text
web/app.js::drawTake
scripts/render_take_diagnostic.py
```

to draw:

* subject;
* target;
* hurtboxes;
* strikes;
* projectiles;
* role labels;
* move phase.

Suggested default semantics:

```text
subject body          clearly labelled
target body           clearly labelled
subject strike        strong red
target strike         subdued red
hurtboxes             cyan
collision envelope    optional orange
projectiles           owner-labelled
```

Color choices are presentation policy; semantic roles must also exist as text/data.

Acceptance:

> The coarse view alone is sufficient to inspect hit/hurt geometry faithfully.

---

## M1.5 — Put the same geometry on the real GPU capture

Investigate enabling:

```text
draw_combat_geometry_view
```

inside the existing `moveset_render` OffscreenGPU composition.

Prefer that over implementing a browser-side transform from a separately simulated take onto a PNG.

The resulting capture should be one simulation execution containing:

```text
actual rendered character art
actual rendered target
actual VFX/projectiles
actual CombatGeometryView overlay
```

If the existing debug renderer cannot be enabled cleanly in this composition, identify the missing plugin/resource seam and fix that seam.

Do not create a second combat-geometry drawing algorithm.

---

## M1.6 — Serialize shutter-time semantic geometry from `moveset_render`

Even if the PNG already contains debug geometry, write the corresponding semantic data into the render artifact.

Sample it before calling:

```text
DeterministicCaptureSession::capture
```

for the same reason current move/pose state is sampled before the shutter.

This lets the browser and LLM know exactly what the image represents.

---

</details>

# Tests and policy guards to extend

The existing tool already has useful contract tests:

```text
tools/ambition_moveset_inspector/check_browser_acceptance.mjs
tools/ambition_moveset_inspector/check_bundle_contract.mjs
tools/ambition_moveset_inspector/check_draw_path.mjs

scripts/tests/test_moveset_inspector_renderer.py
scripts/check_absence_contracts.py
```

Extend these rather than replacing them.

Required regression tests for M1 — **all ten are in place**:

| # | requirement | where it lives |
|---|---|---|
| 1 | N fighters produce N selectable fighters with zero takes | `check_takes_discovery.mjs` |
| 2 | two recordings do not limit the picker to two | `check_takes_discovery.mjs` |
| 3 | subject/target ids and roles survive serialization | `combat_observation::tests::a_seated_scenario_serializes_roles_identities_and_both_geometries` |
| 4 | a published empty `DamageableVolumes` produces no hurtbox | `the_artifact_distinguishes_intangible_from_a_coarse_fallback` (`ambition_sim_harness/tests/combat_observation_it.rs:131`) |
| 5 | unpublished damageable geometry falls back to the coarse box | same test — both bodies, both answers, in one fixture |
| 6 | circle/OBB/convex strike geometry survives serialization | `every_volume_shape_survives_serialization`, `test_a_strike_is_drawn_in_its_real_shape` |
| 7 | target-owned strikes are not attributed to the subject | `a_strike_belongs_to_its_owners_side_not_to_the_owner` |
| 8 | a rendered frame and its semantic manifest name the same action tick | structural: the observation is a field OF the shot row, beside `action_tick` |
| 9 | a stale render stays visibly stale | `test_a_cache_older_than_the_binary_is_not_served`, `test_a_cache_with_no_provenance_is_not_served_as_current` |
| 10 | both drivers use `move_exercise` alone | absence contract `the-two-move-drivers-do-not-author-their-own-presses` |

⭐ #8 is deliberately a STRUCTURE rather than a test. The observation is written
into the shot object that carries `action_tick`, sampled before that shot's
shutter, so there is no second value that could disagree — and a test asserting
two fields of one JSON literal match would be testing the literal.

---

# Explicit non-goals for the first implementation

Do not:

* create a new combat simulator;
* create a second hitbox resolver;
* create a second hurtbox resolver;
* create a third move-input driver;
* replace `moveset_export`;
* replace `moveset_takes`;
* replace `moveset_render`;
* replace `DeterministicCaptureSession`;
* build a new web server;
* optimize the 27-minute full-grid run;
* introduce a persistent renderer daemon before measuring the one-move workflow;
* build combo analysis before single-move geometry is trustworthy.

The first milestone is predominantly a **convergence task over machinery that already exists**.

