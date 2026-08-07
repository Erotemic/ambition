# Diagnostics, causality, and frame inspection

**Checked 2026-08-07.** General engines have excellent profilers and runtime
inspectors. Ambition should use them where possible. The comparison here asks a
different question: can the engine explain *why a platformer outcome happened*?

## The Ambition question

Task 12's intended surface combines two different diagnostic layers:

1. **performance/host telemetry** — fixed-step time, allocations, casts,
   snapshots, rollback cost, construction, rendering, startup and artifact size;
2. **semantic causality** — input, action phase, velocity contributions,
   casts/contacts, damage, lifecycle, rollback correction and presentation facts.

Ambition already has an important primitive:
`ambition_causal::CausalLog::explain(tick, &SubjectKey) -> Explanation`, with a
text renderer. The selected-tick question is therefore real, not aspirational.
The measured limitation is that explanation is tick-scoped: a result at tick N
cannot yet join directly to a cause at N-k.

Budgets are also measured but not enforced, and measurements do not yet cover all
supported hosts.

---

## Unreal Insights — excellent trace/timeline infrastructure

Unreal Insights captures high-rate telemetry events, records trace sessions and
analyzes/visualizes them. Timing Insights provides per-frame CPU/GPU tracks,
spike inspection, caller/callee hierarchy, task execution paths and counters.

Sources:

- [Unreal Insights](https://dev.epicgames.com/documentation/unreal-engine/unreal-insights-in-unreal-engine?lang=en-US)
  (Epic, official).
- [Timing Insights](https://dev.epicgames.com/documentation/unreal-engine/timing-insights-in-unreal-engine-5?lang=en-US)
  (Epic, official). It can "trace the execution path of Tasks".

### Lesson for Ambition

Do not build a competing general profiler. Bevy tracing/diagnostics and platform
profilers should remain the substrate for *where time went*.

Ambition's differentiated layer should answer questions Timing Insights cannot
answer from timing hierarchy alone, for example:

- why did this body become airborne on tick 1842?
- which sweep rejected the floor and under which filter?
- what action owned this velocity contribution?
- why was damage eligible after armor changed?
- which earlier fact caused this transition?
- was this presentation event speculative, confirmed, canceled or replayed?

That is semantic domain causality, not another flame graph.

---

## Unity Profiler — broad instrumentation and device comparison

Unity's Profiler records performance across CPU, rendering, memory, audio and
other modules. It is instrumentation-based, supports custom markers, and can
compare behavior across devices.

Source: [Profiler introduction](https://docs.unity3d.com/6000.0/Documentation/Manual/profiler-introduction.html)
(Unity, official).

### Lesson for Ambition

Unity sets a useful bar for **host-specific measurement**. Ambition's current
"desktop measured, budgets unenforced" state is not enough for a cross-platform
engine claim. The right response is not a standalone profiler UI; it is stable
metrics and workload fixtures that can feed Bevy tooling, CI gates and optional
visual inspectors.

---

## Godot debugger/profiler — integrated, approachable runtime inspection

Godot's profiler lives in the editor debugger, measures frame/physics/script
costs, and lets the user select a frame. Godot also exposes runtime collision and
navigation visualization and remote debugging.

Sources:

- [The Profiler](https://docs.godotengine.org/en/stable/tutorials/scripting/debug/the_profiler.html)
  (Godot, official).
- [Overview of debugging tools](https://docs.godotengine.org/en/4.4/tutorials/scripting/debug/overview_of_debugging_tools.html)
  (Godot, official).

### Lesson for Ambition

Godot demonstrates why the eventual semantic inspector should be easy to reach
from a running game. It does **not** imply Ambition needs an editor shell. A
textual explanation and lightweight Bevy-native overlays can deliver most of the
value while preserving headless parity.

---

## Competitive design frontier

| Diagnostic question | General-engine strength | Ambition opportunity |
|---|---|---|
| Where did CPU/GPU time go? | Unreal/Unity/Godot profilers | integrate; expose stable Ambition markers/metrics |
| What existed at this frame? | inspectors, debug views | expose selected authoritative + presentation facts |
| Why did this contact/action/damage happen? | mostly project-specific logging | typed causal facts with subject and provenance |
| What caused this fact several ticks later? | trace correlation can be hand-built | first-class cross-tick causal edges |
| What changed because of rollback? | networking debuggers/project tooling | predicted/confirmed/corrected history over the same causal graph |
| Is performance acceptable? | profiler interpretation | representative workload budgets enforced per supported host |
| Can CI explain a failure without graphics? | weaker editor-first workflow | same textual explanation in headless tests and visible hosts |

## What still needs to be designed

### 1. Cross-tick causal edges

The next primitive should be a small relation, not a giant event schema. A fact
needs enough identity to reference an earlier fact that materially caused it.
For example:

```text
fact N:   grounded=false
caused_by -> fact N-1: floor sweep rejected one-way candidate
caused_by -> fact N-3: drop-through gesture opened exclusion window
```

The graph should support many-to-one and one-to-many causes, survive
rewind/resimulation, and remain queryable without forcing every subsystem into a
single enum. Typed domain facts can stay domain-owned while sharing subject,
tick and causal-edge vocabulary.

### 2. Correction-aware history

A rollback inspector needs to distinguish at least:

- facts from current corrected history;
- facts that existed only in a speculative history;
- confirmation/cancellation of external effects;
- the tick range that was re-simulated;
- the earliest semantic divergence if two runs mismatch.

This is where the rollback localizer and `ambition_causal` should become
composable instruments without being merged into one storage system merely for
convenience.

### 3. Stable metric names and host budgets

Define metrics as engine-facing contracts before defining dashboards. A CI
fixture should be able to say that a representative scenario exceeded, for
example, fixed-step or rollback-work budget on a named host profile. Thresholds
must come from measurements and should be versioned with the workload.

### 4. Text first, overlays second

The most differentiated interface can be deliberately boring:

```text
ambition explain <scenario> --tick 1842 --subject actor:7
```

The same `Explanation` can later power an in-game overlay, Bevy inspector panel,
HTML report, or trace annotation. The source of truth stays a value that headless
tests can assert, not widget state.

### 5. Author-facing "why not?" queries

A high-leverage extension beyond ordinary profiling is negative explanation:

- why was action `dash` unavailable?
- why did this prompt disappear?
- why was this collider filtered out?
- why did this provider recipe fail preparation?

These queries force eligibility/filtering/validation to expose structured reasons
instead of boolean gates. They would make Ambition materially easier to author
against without requiring a Unity-style editor.

## What this changed

The comparison makes the differentiation boundary sharper:

- **compete on semantic explanation and reproducibility;**
- **integrate for general profiling and visualization.**

A smaller engine can plausibly beat editor-first systems at answering a
platformer-specific "why" because Ambition already owns the deterministic action,
movement, contact, lifecycle and prepared-content contracts. Cross-tick causal
edges and correction-aware history are the missing connective tissue.
