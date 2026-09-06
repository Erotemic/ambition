# Deterministic simulation, rollback, and replay

**Checked 2026-08-07.** This page compares rollback as an engine contract rather
than as a networking feature checkbox.

## The Ambition question

Ambition's current architecture already treats rollback as a property of the
real simulation schedule:

- authoritative state is registered into the `bevy_ggrs` rollback envelope;
- headless scenarios run the same schedule as visible hosts;
- damage, transition, action and contact families have real rewind/resimulation
  coverage;
- prepared-content identity participates in the rollback session contract;
- confirmed external effects are quarantined from speculative frames.

The important remaining work is not "add rollback":

1. Task 2 still lacks a concise **declarative scenario vocabulary**; scenarios
   are Rust test code today.
2. Task 12 lacks cross-tick causal joins, so an outcome at N cannot yet point to
   the fact at N-k that caused it.
3. Host budgets are measured but not enforced.
4. Production online transport remains acceptance-driven rather than an engine
   centerpiece.
5. PA6 still must make participant frame-policy changes deterministic.

See [`simulation-authority-and-determinism.md`](../planning/engine/simulation-authority-and-determinism.md)
and [`netcode.md`](../planning/engine/netcode.md). The current successor work
focuses on explicit deterministic phases, domain-owned rollback participation,
and real consumer/scenario evidence rather than reopening the retired master
campaign.

---

## Photon Quantum — the closest full-system comparison

Photon Quantum describes itself as a deterministic ECS for online multiplayer.
Its core uses predict/rollback, keeps simulation logic separate from Unity view
code, and supplies deterministic libraries including physics. Clients exchange
input, simulate locally, restore state, and re-simulate mispredictions.

Source: [Quantum 3 Intro](https://doc.photonengine.com/quantum/v3/quantum-intro)
(Photon, official). The page calls out a "server-managed predict/rollback
simulation core".

Quantum explicitly distinguishes verified and predicted frames, and says it
always rolls back and re-simulates frames as part of its architecture.

Source: [Quantum Frames](https://doc.photonengine.com/quantum/v3/manual/frames)
(Photon, official).

Its simulation-to-view Events account for a frame being simulated multiple
times. Event identity includes event data/id/tick for duplicate detection, and
`synced` events can wait for server-confirmed input.

Source: [Quantum ECS Events & Callbacks](https://doc.photonengine.com/quantum/v3/manual/quantum-ecs/game-events)
(Photon, official).

### Lesson for Ambition

Quantum is the most useful competitive bar because it treats determinism,
rollback, simulation/view separation and event confirmation as one coherent
product rather than a plugin added after gameplay exists.

Ambition can still be distinct in two ways:

- **rollback is useful even without online multiplayer.** The same machinery
  powers headless regression, replay, deterministic diagnosis and local testing;
- **simulation is platformer-specialized.** Ambition can make movement, contact,
  action and lifecycle causality first-class instead of providing only a general
  deterministic ECS substrate.

The comparison also validates the existing confirmed-effect quarantine: a view
must not blindly manifest every speculative simulation event.

---

## Unity Netcode for Entities — authoritative snapshots plus client replay

Unity's Netcode for Entities prediction model lets a client use local inputs to
simulate without waiting for the server, then reconcile against authoritative
network state. Its prediction documentation includes smoothing for correcting
visible error.

Source: [Prediction in Netcode for Entities](https://docs.unity3d.com/Packages/com.unity.netcode%401.6/manual/prediction.html)
(Unity, official).

### Lesson for Ambition

Unity demonstrates the mainstream value of prediction/reconciliation, but
Ambition should avoid allowing "network prediction" to become a separate
simulation path. The headless, replay, local and online hosts should continue to
share authoritative rules and rollback participation.

---

## Unreal Network Prediction — a subsystem, not the organizing model

Unreal provides a Network Prediction plugin described by Epic as a generalized
framework for prediction-friendly gameplay systems. Epic currently labels the
feature Beta and advises caution when shipping with it.

Source: [Network Prediction](https://dev.epicgames.com/documentation/unreal-engine/API/PluginIndex/NetworkPrediction?lang=en-US)
(Epic, official; re-check status before depending on version-specific details).

### Lesson for Ambition

Ambition should not copy a "prediction-capable subsystem beside ordinary
runtime" shape. The competitive point is that platformer authors get one
simulation contract whose rollback properties are continuously tested, even if
they never ship online play.

---

## Bevy ecosystem — powerful substrate, policy intentionally external

Bevy supplies ECS scheduling, state, assets, scenes and a broad plugin model, but
its core does not prescribe one rollback networking contract. Ambition currently
uses `bevy_ggrs` / GGRS as the rollback authority and layers platformer-specific
registration, session identity, harnesses and causal tooling above it.

Source: [Bevy](https://bevy.org/) (Bevy, official) for the core engine substrate.
`bevy_ggrs` itself is ecosystem integration rather than a Bevy-core guarantee.

---

## Competitive design frontier

| Problem | Strong prior art | Ambition bar |
|---|---|---|
| Deterministic rollback core | Photon Quantum | make participation mechanically reviewable and keep one real schedule |
| Predicted vs confirmed presentation | Quantum events; Unity reconciliation | confirmed-effect quarantine plus causal identity for speculative/corrected effects |
| Scenario reproduction | engine/project test tooling | concise declarative scenario -> normal step + rewind + checksum + facts |
| Network transport | Quantum integrated service; engine packages elsewhere | keep transport replaceable/acceptance-driven; do not make it simulation authority |
| Content compatibility | deterministic engines generally require matching data | prepared-content fingerprint is explicit in the session contract |
| Debugging a desync | checksums/profilers/logs | localize to semantic movement/contact/action/lifecycle causes |

## What still needs to be designed

### 1. A declarative scenario language that is smaller than a test framework

Task 2 already states the right ingredients: composition/provider, room/setup,
participant input frames, tick count, selected observations, optional rollback
window and expected invariant/checksum.

The related-work constraint is: **do not turn this into a second scripting
runtime.** A scenario should declaratively feed the real engine schedule and ask
for facts; custom setup can remain a bounded hook where data is insufficient.
The same scenario must run in ordinary headless stepping and forced
rewind/resimulation.

A useful author-facing acceptance test is one file that can reproduce a bug,
produce the mismatch tick, and then hand that tick directly to the causal
explainer.

### 2. Causal identity across speculative and corrected history

Quantum's duplicate/confirmed-event machinery makes a hidden requirement
visible: rollback-facing presentation needs identity across repeated simulation.
Ambition should connect external-effect facts to the causal action/contact/damage
facts that emitted them so tooling can say both:

- "this sound/VFX was delayed because its source tick was speculative"; and
- "this earlier speculative effect was superseded by this corrected cause."

This belongs in the causal/effect contract, not in each audio/VFX consumer.

### 3. Prediction workload budgets

A deterministic engine can still fail operationally if a correction causes too
much resimulation. Representative workloads should measure and eventually gate:

- normal fixed-step cost;
- snapshot size/copy cost;
- rollback depth and re-simulated tick count;
- collision/cast work per corrected tick;
- allocation behavior inside resimulation;
- confirmed-effect backlog size.

Prediction culling or similar optimization should be considered only when a real
workload proves it necessary; correctness coverage comes first.

### 4. Keep transport downstream of the simulation contract

Production transport is explicitly deferred until a customer requires it. That
is a feature, not an embarrassment: GGRS/session semantics should define what an
adapter must deliver, while matchmaking, relay/server authority and transport
choice remain replaceable. Photon Quantum chooses a vertically integrated
service; Ambition can distinguish itself through a portable simulation contract.

## What this changed

Photon Quantum raises the competitive bar: "rollback-capable" is too weak a
claim. The meaningful bar is an engine where determinism, predicted/confirmed
history, view-side effects, replay and tooling form one contract.

Ambition is already unusually close on the simulation side. The highest-value
remaining design work is the **declarative scenario -> rollback mismatch ->
causal explanation** loop, because that turns determinism from an implementation
property into an author-facing debugging capability.
