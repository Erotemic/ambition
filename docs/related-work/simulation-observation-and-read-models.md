# Simulation observation and read models

**Checked 2026-08-07.** Ambition already has a deliberate observation boundary
between authoritative simulation and every consumer. This should be compared to
Bevy's render extraction and mature UI view-model/data-binding systems, because
it is an implemented architectural seam with consequences for rendering,
headless tests, RL, netcode and debugging.

## The Ambition capability that already exists

[`ambition_sim_view`](../../crates/ambition_sim_view/src/lib.rs) calls itself
**the observation boundary**. Its read models are plain-data snapshots rebuilt
from simulation state at the end of the simulation step. The crate's contract is
explicit:

- builders are pure functions of current sim state;
- rows do not retain live `Entity`/asset-handle borrows;
- render, RL observation, netcode confirmation, AI and special-relativity
  presentation can consume the same facts;
- render code depends on the read-model crate instead of querying the live sim
  heart directly.

The current surface includes `FeatureViewIndex`, actor/boss render indexes,
animation/pose snapshots, nameplates, dialogue, attack VFX, shield-ring facts and
`ControlPrompt` — “what does each control do right now” for the controlled
subject.

The camera snapshot is a deliberate exception: it observes simulation state but
resolves on the render clock because viewport/profile/video settings are
presentation facts, not simulation facts.

This is already more than a renderer cache. It is a CQRS-like **read side for a
deterministic game simulation**.

---

## Bevy RenderApp extraction — the closest substrate analogy

Bevy has separate main and render worlds. During `ExtractSchedule`, data is
extracted from `MainWorld` into the render world; the render app then consumes
its own world instead of directly operating on the main app's ECS state.

Sources:

- [`Extract`](https://docs.rs/bevy/latest/bevy/render/struct.Extract.html)
  (Bevy API docs).
- [`ExtractSchedule`](https://docs.rs/bevy/latest/bevy/render/struct.ExtractSchedule.html)
  (Bevy API docs).

### Comparison

This is the strongest precedent for Ambition's boundary and validates its basic
shape. Ambition's move is to **generalize extraction beyond rendering**:

```text
Bevy MainWorld -> RenderApp extraction -> renderer

Ambition authoritative sim -> SimView read models -> renderer
                                             -> RL / headless observation
                                             -> diagnostics / inspector
                                             -> netcode-facing confirmation
                                             -> observer-relative presentation
```

That generalization is useful because otherwise each observer invents a
slightly different query over live components and slowly creates multiple
meanings of “the state of this actor.”

---

## Unreal UMG MVVM — presentation can depend on a view model, not backend objects

Unreal's Model-View-ViewModel plugin lets programmers expose Viewmodels whose
fields are bound to UMG widgets, separating backend/application data from visual
design. Updates to a Viewmodel propagate to bound widgets.

Source: [UMG Viewmodel](https://dev.epicgames.com/documentation/en-us/unreal-engine/umg-viewmodel-for-unreal-engine?lang=en-US)
(Epic, official).

### Comparison

Ambition's `SimView` is not a UI framework, but the architectural lesson is the
same: presentation should consume an intentionally shaped model rather than
reach through to domain implementation objects.

The difference is scope and authority. Unreal MVVM is a UI-facing pattern;
Ambition's read model is intended to be **the engine-wide observation contract**
for multiple non-authoritative consumers. That makes determinism, stable
identity and snapshot semantics more important than widget binding convenience.

---

## Unity UI Toolkit runtime data binding — structured data sources are a product feature

Unity 6's UI Toolkit supports runtime data binding between UI elements and
application data sources, including automatic update propagation for bound
collections/properties.

Sources:

- [Data binding](https://docs.unity3d.com/current/Documentation/Manual/best-practice-guides/ui-toolkit-for-advanced-unity-developers/data-binding.html)
  (Unity, official).
- [Introduction to UI Toolkit](https://docs.unity3d.com/6000.0/Documentation/Manual/ui-systems/introduction-ui-toolkit.html)
  (Unity, official).

### Comparison

This establishes an **ergonomics expectation** Ambition has not yet met. Having
a correct read-model boundary is only half the product. Consumers need easy
ways to:

- subscribe to or diff rows;
- bind UI/overlays to stable fields;
- inspect schema and identity;
- understand when a row was produced;
- distinguish authoritative, predicted, confirmed and presentation-only facts.

The eventual UI binding layer can be Bevy-native, but it should bind to
`SimView` rows rather than encourage arbitrary live simulation queries.

---

## What Ambition already distinguishes

| Question | Common implementation | Ambition's current answer |
|---|---|---|
| What may renderer query? | often gameplay objects/components directly | read-model boundary; render must not query sim heart directly |
| What may headless/RL inspect? | custom environment-specific queries | same observation layer can serve nonvisual consumers |
| What is presentation-only? | frequently mixed into actor state | camera snapshot explicitly resolves on render frame and never feeds sim |
| What identity crosses the seam? | runtime object/entity reference is common | read rows can carry stable semantic/simulation identity instead of live borrows |
| How often is state materialized? | observer-dependent | sim facts rebuild once at a known simulation-tail phase |

## Design work the comparison exposes now

### 1. Define a public observation schema

The current read models are several Rust resources/components. The public engine
needs an inventory saying which facts are guaranteed, which are optional
capabilities, their stable IDs and the tick/epoch they describe. This is
especially important for Python/RL, recording and remote inspection.

### 2. Add change/diff semantics without adding simulation authority

A UI or remote inspector should not have to rescan every row each frame. Add a
read-side revision/change protocol derived during extraction. It must remain
derived state: consumers can use it for efficiency, but simulation must never
branch on it.

### 3. Represent prediction/confirmation explicitly

Once rollback presentation is involved, “this row came from tick 420” is not
quite enough. An observation may be predicted, resimulated, confirmed or
superseded. That status should be part of observation metadata so render/UI/tools
do not invent separate correction bookkeeping.

### 4. Turn the boundary into an enforceable dependency rule everywhere

The render crate already pins the boundary. Apply the same absence/dependency
contract to UI, RL, inspector and presentation crates: if a non-authoritative
consumer needs a sim-heart component directly, that is evidence the read model
is missing a fact.

### 5. Offer bindings/adapters above the read model

Unity/Unreal demonstrate the productivity payoff of view-model binding. Build
thin adapters for Bevy UI/debug overlays rather than putting observer-specific
convenience fields back into simulation state.

## What this comparison changed

`ambition_sim_view` should be documented as an engine-level architectural
feature, not a render-support crate. Bevy's render extraction is the substrate
precedent; Unreal MVVM and Unity data binding show the product experience that a
well-shaped read side can enable.

The differentiating target is **one deterministic observation vocabulary shared
by visual presentation, headless agents, netcode tooling and diagnostics without
letting any observer become simulation authority**.
