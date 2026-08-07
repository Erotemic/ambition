# Diagnostics, causality, and frame inspection

**Checked 2026-08-07.** General engines have excellent profilers and runtime
inspectors. Ambition should use them where possible. The source now supports a
more specific comparison too: `ambition_causal` is already an observer-only,
typed semantic trace whose execution identity survives rollback resimulation.
The competitive question is therefore not whether Ambition needs "debug logs";
it is how far to take **semantic explanation** beyond ordinary profiling.

## The Ambition capability that already exists

[`ambition_causal`](../../crates/ambition_causal/src/lib.rs) opens with the
question it is built to answer:

> Why did this actor change on this tick?

The implementation is materially stronger than a text log:

- domains publish typed `CausalFact`s; nothing parses strings back into facts;
- simulation code must never read the observer log, preserving rollback safety;
- `RecordingPolicy` bounds/selects instrumentation by domain;
- the ring reports dropped facts instead of silently pretending a truncated
  explanation is complete;
- `ExecutionKey` distinguishes session generation, original vs. resimulated
  execution, and rollback attempt;
- `CausalLog::explanations` returns each execution of one tick separately rather
  than merging contradictory histories;
- `FactId` lets one fact name another as its cause;
- `dump()` gives deterministic CI text ordered by tick and fact id;
- the thread-local sink explicitly counts off-thread facts that it could not
  collect, instead of silently losing them under Bevy's scheduler.

See [`log.rs`](../../crates/ambition_causal/src/log.rs) and
[`fact.rs`](../../crates/ambition_causal/src/fact.rs).

There is one important boundary to state precisely. `FactId` is log-global, so a
publisher can name an earlier fact. But `Explanation::chain_to` currently walks
only the facts already selected into that one-tick `Explanation`. A cause from
an earlier tick is therefore not yet traversable through the public explanation
value. The missing capability is **cross-tick explanation/query**, not basic
rollback labeling.

---

## Whyline — direct prior art for "why did?" and "why didn't?"

Andrew Ko and Brad Myers' Whyline work is unusually close to Ambition's desired
*author-facing* diagnostic experience. Instead of asking a programmer to infer a
cause from execution traces, the tool offers questions about observable output,
including both **why did** and **why didn't** questions, and computes
explanations from program execution.

Sources:

- [Debugging Reinvented: Asking and Answering Why and Why Not Questions about Program Behavior](https://faculty.washington.edu/ajko/papers/Ko2008JavaWhyline.pdf)
  (Ko and Myers, CHI 2008, author-hosted paper).
- [Designing the Whyline](https://www.cs.cmu.edu/afs/cs.cmu.edu/Web/People/marmalade/papers/Ko2004Whyline.pdf)
  (Ko and Myers, CHI 2004, institution-hosted paper).

### Lesson for Ambition

This is stronger prior art than a general profiler for the roadmap's proposed
`ambition explain` surface. Ambition should explicitly borrow the interrogation
model:

```text
why did actor:7 become airborne?
why did dash start?
why didn't dash start?
why was collider:ledge rejected?
why didn't route:boss_room commit?
```

The important technical consequence is that **negative explanation must be
recordable**. If eligibility and filtering return only booleans, "why not?" is
impossible without re-running domain logic in a debugger. Rejection/eligibility
seams should instead be able to expose stable reason codes and the facts they
depended on.

Whyline operates at program/debugger semantics. Ambition has the opportunity to
be more domain-specific: the explanation vocabulary can be action, movement,
contact, damage, lifecycle, loading, content preparation, rollback and
presentation rather than variables and statements.

---

## rr — deterministic replay makes temporal debugging practical

`rr` records execution and replays it deterministically; its debugger supports
reverse execution by restoring checkpoints and replaying forward to an earlier
point.

Sources:

- [`rr`: lightweight recording & deterministic debugging](https://rr-project.org/)
  (rr project, official).
- [Lightweight User-Space Record And Replay](https://arxiv.org/abs/1610.02144)
  (O'Callahan et al., primary paper).

### Lesson for Ambition

Ambition does not need to reproduce an instruction-level reverse debugger. Its
simulation already has the more useful domain boundary: fixed ticks,
deterministic inputs, rollback attempts and stable semantic identities.

The rr comparison raises the product bar from "we can replay" to **time-travel
investigation**:

1. choose a divergent or surprising tick;
2. inspect authoritative facts at that tick;
3. walk semantic causes backward across earlier ticks;
4. compare original and resimulated attempts;
5. reproduce the same chain headlessly from the scenario/input artifact.

That would be a platformer-specific reverse debugger over domain facts rather
than machine instructions.

---

## OpenTelemetry traces — explicit context makes causal structure composable

OpenTelemetry traces consist of spans joined by trace/span context. Parent-child
relationships capture nested operations, while trace context is propagated so
work performed in different components can still belong to one end-to-end
trace.

Sources:

- [Traces](https://opentelemetry.io/docs/concepts/signals/traces/)
  (OpenTelemetry, official).
- [OpenTelemetry documentation](https://opentelemetry.io/docs/)
  (OpenTelemetry, official).

### Lesson for Ambition

Do not copy the service/span vocabulary into gameplay. Copy two structural ideas:

1. **identity is carried with the fact**, not reconstructed by timestamp/text
   correlation;
2. **causal context crosses subsystem boundaries** without requiring every
   subsystem to share one closed event enum.

Ambition's open `FactDetail::kind`, stable subject keys and optional `cause`
already point this way. The next step is a query model that can traverse causes
across ticks and subjects while preserving execution identity.

A useful difference from ordinary distributed traces is that rollback gives
Ambition **alternative executions of the same logical tick**. Any causal graph
must therefore key edges by the exact execution/generation/attempt, not just by
`SimTick`.

---

## Unreal Insights, Unity Profiler, and Godot debugger — integrate for general profiling

Unreal Insights captures trace sessions and exposes timing/task/counter views;
Unity Profiler provides CPU, rendering, memory and other instrumentation; Godot
integrates frame/physics/script profiling and runtime debug visualization into
its debugger.

Sources:

- [Unreal Insights](https://dev.epicgames.com/documentation/unreal-engine/unreal-insights-in-unreal-engine?lang=en-US)
  (Epic, official).
- [Unity Profiler introduction](https://docs.unity3d.com/6000.0/Documentation/Manual/profiler-introduction.html)
  (Unity, official).
- [Godot profiler](https://docs.godotengine.org/en/stable/tutorials/scripting/debug/the_profiler.html)
  (Godot, official).

### Lesson for Ambition

These systems remain the right comparison for **where time and memory went**.
Ambition should emit stable tracing/diagnostic markers and workload metrics into
existing tooling rather than creating another flame graph.

The semantic layer should answer the orthogonal questions those tools do not
know from timing hierarchy alone: why this move started, why this contact was
accepted, why this transition was refused, which speculative execution emitted a
fact, and what earlier game-domain fact caused the current state.

---

## Comparison matrix

| Question | Mature prior art | Ambition today | Next bar |
|---|---|---|---|
| Where did CPU/GPU time go? | Unreal Insights, Unity/Godot profilers | Bevy/general diagnostics substrate | stable markers + representative host budgets |
| What happened on this tick? | frame inspectors, logs | typed `CausalFact`s + deterministic dump | selected-tick inspector over causal/read-model facts |
| Was this original or rollback replay? | networking/project-specific tools | `ExecutionKey { generation, execution, attempt }` | correction/divergence UI over attempts |
| What caused this fact? | tracing parents, debugger slicing | `FactId` + optional `cause` | graph query across ticks/subjects/executions |
| Why did this outcome happen? | Whyline is direct research prior art | same-tick subject explanation | domain-level backward semantic explanation |
| Why did this outcome *not* happen? | Whyline | mostly boolean eligibility/filtering today | structured rejections and negative queries |
| Can I go backward from a failure? | rr reverse execution | deterministic sim/rollback + per-tick facts | scenario-driven semantic time travel |
| Can CI consume the same evidence? | traces/log artifacts | deterministic textual dump | assertion/query API + minimized repro artifact |

## Design work the source exposes now

### 1. Cross-tick, execution-qualified causal references

Do not replace `FactId` with a giant event hierarchy. Add the minimum query
identity needed to resolve an edge across retained history while proving it
refers to the intended execution. A useful public reference likely needs fact id
plus enough execution identity to reject stale/cross-history edges explicitly.

The resulting query should support many-to-one and one-to-many explanations and
be honest when the bounded ring dropped an earlier cause.

### 2. First-class negative/rejection facts

Eligibility, filtering, preparation and commit seams should expose structured
reasons when they decline an operation. Candidate domains already exist:

- action unavailable / cooldown / capability missing;
- collider ignored by one-way/filter/frame rule;
- interaction candidate lost arbitration;
- loading barrier not ready or superseded;
- provider/content preparation rejected a row/reference;
- presentation effect quarantined until confirmation.

These reasons should be normal domain values first and causal facts second. The
simulation must not branch on observer data.

### 3. Correction-aware comparison, not just correction labeling

The source already distinguishes attempts. Build the next tool around that fact:
for one subject/tick, compare original and resimulated explanations and identify
the earliest semantic divergence. This is stronger and more actionable than
simply coloring a timeline "resimulated".

### 4. Thread-safe ECS recording with loss accounting

The thread-local sink deliberately exposes `facts_lost_offthread`. Finish the
Bevy-side recorder contract so multithreaded domains can publish without either
serializing the simulation or silently dropping data. Keep loss counters and
recording policy visible in every explanation artifact.

### 5. Stable semantic metric and query vocabulary

A tool ecosystem can only grow around names that do not change every refactor.
Promote subject identity, execution identity, rejection reason, causal reference
and high-value metric names as compatibility surfaces; keep domain-specific fact
payloads open.

### 6. Text/query API before a bespoke UI

The highest-leverage interface remains deliberately boring:

```text
ambition explain <scenario> --tick 1842 --subject actor:7
ambition why-not <scenario> --tick 1842 --subject actor:7 --action dash
ambition diff-execution <scenario> --tick 1842 --subject actor:7
```

A Bevy overlay, HTML report or trace annotation can render the same values later.
Headless tests and CI should never require a widget to obtain the explanation.

## What this changed

The source changes the diagnosis from the earlier planning prose:
**rollback-aware semantic recording is already present.** Ambition is not
starting from printf debugging, and it does not need another general profiler.

The sharper differentiation target is a domain-level combination of Whyline's
interrogative debugging, rr's temporal reproducibility and tracing-style explicit
causal context:

> explain why or why-not a deterministic platformer outcome happened, across
> ticks and rollback attempts, using the same typed facts available to headless
> tests.

Cross-tick graph queries and structured rejection facts are the two biggest
missing pieces.
