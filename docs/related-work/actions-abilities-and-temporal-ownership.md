# Actions, abilities, and temporal ownership

**Checked 2026-08-07.** This comparison asks where a semantic action stops being
"input" and becomes authoritative gameplay state: eligibility, startup, active
windows, recovery, costs, cooldowns, cancellation, prediction, and effects.

## The Ambition question

Ambition already has a good invocation seam:

- `ControlFrame` / `SlotControls` carry compact participant intent;
- `ActorActionScheme` and the shared resolver map slots to semantic actions;
- `ControlPrompt` uses the same eligibility seam;
- combat `MovePlayback` owns a real temporal action lifecycle for melee;
- movement maneuvers already own their own temporal state where appropriate.

The roadmap therefore says **do not mint a second ability framework just to
rename what exists**. The open question is narrower: what shared temporal
ownership is needed by the *next* action family that cannot fit `MovePlayback`
or a movement maneuver without duplication?

See [the competitive roadmap, Task 3](../planning/engine/competitive-2d-platformer-engine-roadmap.md#task-3--finish-participant-action-routing-and-temporal-action-ownership)
and [`character-actions.md`](../planning/engine/character-actions.md).

---

## Unreal Gameplay Ability System — the strongest general-purpose prior art

Unreal's Gameplay Ability System (GAS) is explicitly a framework for attributes,
abilities and interactions owned/triggered by Actors. Gameplay Abilities may be
active or passive, coordinate animation/VFX/audio, and use asynchronous Ability
Tasks. Gameplay Effects modify attributes.

Source: [Gameplay Ability System](https://dev.epicgames.com/documentation/unreal-engine/gameplay-ability-system-for-unreal-engine?lang=en-US)
(Epic, official).

A `UGameplayAbility` defines what an ability does, its costs and conditions, and
may run multi-stage asynchronous work. The system also supports replication and
client-side prediction.

Source: [Using Gameplay Abilities](https://dev.epicgames.com/documentation/unreal-engine/using-gameplay-abilities-in-unreal-engine?lang=en-US)
(Epic, official).

GAS prediction uses `FPredictionKey` to correlate predicted client actions and
side effects with server acceptance/rejection. Epic's own documentation also
calls out cases that do not fit the correlation model cleanly, which is useful:
prediction is not "turn it on", it is a contract about which side effects can be
matched and undone.

Source: [FPredictionKey](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Plugins/GameplayAbilities/FPredictionKey)
(Epic, official). The page describes it as an ID for "predictive actions and side
effects".

### Lesson for Ambition

GAS is worth copying at the **lifecycle vocabulary** level:

- eligibility/preconditions;
- cost commitment;
- activation;
- asynchronous stages;
- cancellation;
- cooldown;
- effect/presentation boundaries;
- explicit prediction identity where prediction exists.

It is not evidence that Ambition needs one universal `Ability` object, an effect
DSL, or a second actor component that becomes the new owner of movesets,
movement maneuvers and interactions. GAS solves for a very broad Unreal project
space. Ambition's narrower platformer specialization lets ownership stay with the
subsystem that has the best semantics.

---

## Unity Input System — temporal input recognition, not gameplay ability ownership

Unity Interactions model input patterns such as press, hold, tap, slow tap and
multi-tap. Interactions have phases including waiting, started, performed and
canceled, and they drive Input Actions.

Source: [Interactions](https://docs.unity3d.com/Packages/com.unity.inputsystem%401.13/manual/Interactions.html)
(Unity, official).

That is useful prior art for **gesture recognition**, especially progress UI for
holds. It is deliberately the wrong layer for authoritative attack startup,
recovery, armor, costs or rollback state. Those facts depend on the controlled
body and gameplay state, not merely how a physical control was actuated.

Unity's Animator state-machine facilities are similarly presentation-oriented:
they are good at animation transitions and callbacks but do not by themselves
establish a gameplay ability contract.

Source: [StateMachineBehaviour](https://docs.unity3d.com/6000.0/Documentation/Manual/StateMachineBehaviours.html)
(Unity, official).

### Lesson for Ambition

Keep **slot-level gestures** and **body-level action state** separate. A hold/tap
interaction may decide which semantic action is requested; it should not become
the owner of the action's authoritative timeline.

---

## Godot — input actions and animation graphs are intentionally compositional pieces

Godot's `InputMap` names and binds input actions, while
`AnimationNodeStateMachine` is a graph of animation nodes whose transitions can
be driven at runtime.

Sources:

- [InputMap](https://docs.godotengine.org/en/stable/classes/class_inputmap.html)
  (Godot, official).
- [AnimationNodeStateMachine](https://docs.godotengine.org/en/stable/classes/class_animationnodestatemachine.html)
  (Godot, official).

The official facilities cited here are pieces from which a game can build an
ability lifecycle, not one generalized gameplay-ability authority. That keeps
small games simple, but leaves cross-game lifecycle conventions to project code.

---

## Competitive design frontier

| Problem | Strong prior art | Ambition bar |
|---|---|---|
| Semantic invocation | Unity/Godot actions; Unreal Input Actions | keep the landed slot-to-action resolver as the one entry seam |
| Temporal ability lifecycle | Unreal GAS | introduce shared lifecycle state only when two real families need the same semantics |
| Gesture recognition | Unity Interactions | participant-side press/hold/tap; gameplay remains body-authoritative |
| Costs/cooldowns/eligibility | Unreal GAS | one eligibility source for behavior and prompts; rollback-safe commitment |
| Specialized execution | GAS Ability Tasks / project code | movesets, movement laws, interactions and provider effects remain specialized executors |
| Prediction | Unreal prediction keys | deterministic rollback is the default simulation model; side-effect confirmation still needs explicit identity/ownership |

## What still needs to be designed

### 1. The minimum shared temporal record

Do not start by designing `UniversalAbility`. Start by finding the smallest state
that the next non-melee action family needs *in common* with existing temporal
owners. Candidate facts include:

- action identity and owning subject;
- start tick and phase;
- committed cost/cooldown facts;
- cancellation/abort reason;
- completion tick;
- optional causal id for confirmed/predicted external effects.

The burden of proof is on each field: if `MovePlayback` or a movement-maneuver
state already owns it correctly, the shared layer should reference or project
that fact rather than duplicate it.

### 2. Cost and cooldown transaction timing

Ambition needs one rule for when a cost becomes authoritative relative to
eligibility, action start, rollback, cancellation and provider effects. GAS shows
why this is a first-class lifecycle concern; Ambition can distinguish itself by
making the rule deterministic and inspectable instead of callback-order
convention.

### 3. Confirmed versus speculative presentation

Rollback means an action can be simulated more than once. External effects such
as audio, haptics and non-authoritative VFX need a semantic relation to the
action fact that caused them and a confirmation policy. This should integrate
with the existing confirmed-frame external-effect quarantine, not create an
ability-specific networking side channel.

### 4. Provider extension without effect-language lock-in

Provider-authored actions need stable IDs and registration, but the execution
body should remain normal Rust/domain code. The competitive advantage is an
open lifecycle contract without forcing every mechanic through a universal
effect graph.

## What this changed

Unreal GAS raises the bar for lifecycle completeness, but it does **not** justify
replacing Ambition's landed action seam. The sharper target is:

> GAS-quality lifecycle semantics where those semantics are truly shared, with
> Ambition's specialized executors and deterministic rollback remaining the
> authority.

That gives Ambition a path to be both smaller and more explainable than a broad
ability framework while still matching the parts authors actually depend on:
eligibility, costs, cancellation, cooldowns, prediction/confirmation and tooling.
