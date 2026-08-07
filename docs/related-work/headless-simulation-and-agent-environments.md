# Headless simulation and agent environments

**Checked 2026-08-07.** Ambition already exposes its real platformer simulation as
a programmatically stepped environment. That should be compared directly with
standard reinforcement-learning environment APIs and game-engine ML tooling,
because the overlap is no longer hypothetical.

## The Ambition capability that already exists

[`ambition_sim_harness`](../../crates/ambition_sim_harness/src/lib.rs) exposes:

```text
Platformer2dSimHarness::build
Platformer2dSimHarness::step
Platformer2dSimHarness::reset_episode
AgentAction
AgentObservation
reward shaping
random-policy / fuzz driver
```

The important source-level property is that this harness **composes below the
product shell but above the real simulation**. A caller supplies the game
composition; the harness installs the same headless foundation and simulation
host used by the engine. It can run render-frame, fixed-step or GGRS-backed
stepping, and fixed modes use Bevy's `TimeUpdateStrategy::ManualDuration` so one
programmatic update advances a controlled deterministic duration rather than
whatever wall time elapsed.

The crate explicitly names RL agents, fuzz harnesses, scripted replay tools and
future Python bindings as consumers of the same seam.

That is already a product-shaped engine interface.

---

## Gymnasium `Env` — the de facto small interface for a stepped environment

Gymnasium environments expose `reset()` and `step(action)`. A step returns the
next observation, reward, `terminated`, `truncated` and an information payload.
Environments also publish explicit action and observation spaces.

Sources:

- [`Env` API](https://gymnasium.farama.org/api/env/)
  (Farama Foundation, official).
- [Basic usage](https://gymnasium.farama.org/introduction/basic_usage/)
  (Farama Foundation, official).

### Comparison

The mapping is already close:

| Gymnasium | Ambition today |
|---|---|
| `reset()` | `reset_episode()` |
| `step(action)` | `step(AgentAction)` |
| observation | `AgentObservation` |
| reward | reward helper / caller policy |
| termination | episode/lifecycle facts available to harness |
| truncation | not yet as explicit in the public seam |
| action/observation spaces | Rust types imply shape, but no standard space metadata yet |
| `info` | engine facts are available, but no compact standard step-info object yet |

Gymnasium therefore exposes concrete API work Ambition can adopt without
changing its architecture: make episode termination versus external truncation
explicit, publish machine-readable action/observation specifications, and
provide an `info` channel for stable diagnostics/metrics that are not learning
observations.

---

## Unity ML-Agents — game-engine simulation as an ML environment

Unity ML-Agents defines Agents that observe an environment, receive actions and
accumulate rewards. Its environment-design guidance covers observation/action
spaces, rewards, teams and multi-agent scenarios, and its low-level Python API
returns decision/terminal steps with observations, rewards and stable agent
identifiers.

Sources:

- [Agents](https://unity-technologies.github.io/ml-agents/Learning-Environment-Design-Agents/)
  (Unity Technologies, official project docs).
- [Designing a learning environment](https://unity-technologies.github.io/ml-agents/Learning-Environment-Design/)
  (Unity Technologies, official project docs).
- [Python low-level API](https://unity-technologies.github.io/ml-agents/Python-LLAPI/)
  (Unity Technologies, official project docs).

### Comparison

ML-Agents is the most useful **game-engine product bar**. It demonstrates that
“headless/RL support” is not just removing rendering. A serious environment
surface needs:

- declared observations and actions;
- agent identity through episodes;
- multi-agent decision cadence;
- deterministic or controllable stepping;
- reset semantics;
- reward/termination semantics;
- an external language/process bridge;
- scalable parallel execution.

Ambition already has unusually strong foundations for several of these because
participant seats, stable `SimId`, deterministic stepping, rollback and
`SimView` are engine concepts rather than ML-specific wrappers.

---

## What is distinctive about Ambition's current seam

### It is not a second toy simulation

The harness builds the same engine foundation and caller-supplied game
composition. That is the most important design choice. A training environment
that approximates the game would drift; Ambition's goal is one simulation with
optional presentation.

### Rollback is a harness mode, not only a multiplayer feature

The harness can start GGRS sync-test sessions. That means the same programmatic
API can be used to stress deterministic rollback behavior, not merely train an
agent. This is a strong bridge between RL, fuzzing and netcode validation.

### The observation boundary already exists independently

[`ambition_sim_view`](../../crates/ambition_sim_view/src/lib.rs) rebuilds plain
data read models from authoritative simulation state. The RL harness therefore
has a path toward consuming the same observations as render/debug/netcode
surfaces instead of growing private ECS queries.

### Local multi-seat semantics are real engine state

The runtime code already had to solve a subtle problem: headless hosts do not
have physical devices, so seat topology must be declared before a rollback
session or a second participant can author inputs nobody routes. That is exactly
the kind of bug a mature multi-agent API has to prevent by construction.

## Design work the comparison exposes now

### 1. Make the harness Gymnasium-complete at the semantic level

Without requiring Python yet, define one engine-neutral step result:

```text
StepResult {
    observation,
    reward_facts,
    terminated,
    truncated,
    info,
    tick,
}
```

Keep reward policy pluggable; the engine should publish facts from which a task
can compute reward rather than baking one universal reward function into the
simulation.

### 2. Publish action and observation schemas

A typed Rust struct is sufficient for Rust callers but not for tooling,
recording, Python or remote workers. Add stable schema/spec objects describing
shape, ranges, optionality and semantic IDs. They should be derivable from the
same action/control vocabulary the game exposes, not duplicated ML metadata.

### 3. Make multi-agent stepping participant-native

Do not add an ML-specific “agent slot” beside participant/control authority.
Expose actions keyed by the same stable seat/participant identity the simulation
uses and observations keyed by the controlled/observed subject. That lets RL,
local multiplayer, bots and network input share topology instead of translating
between parallel concepts.

### 4. Add vectorized/batched execution without changing sim semantics

The useful scale target is many independent `Platformer2dSimHarness` instances,
possibly one per worker thread/process, with batched action/observation transfer.
The simulation itself should remain single-instance deterministic code. Parallel
training belongs around simulations, not inside the authoritative state of one.

### 5. Build the Python bridge over the stable facade

A future Python binding should bind the public harness/action/observation schema,
not arbitrary Bevy components. That makes Python an SDK consumer and an API
quality test in the same way the external Rust consumer is today.

## What this comparison changed

Headless simulation should be listed among Ambition's **existing competitive
surfaces**. The engine is already close enough to Gymnasium's conceptual API that
its missing pieces can be named precisely, while Unity ML-Agents establishes a
clear product bar for multi-agent ergonomics and external tooling.

The differentiating target is:

> the same deterministic, rollback-capable platformer simulation is the game,
> the test fixture, the fuzz target and the learning environment; only the
> participant input source and observation consumer change.
