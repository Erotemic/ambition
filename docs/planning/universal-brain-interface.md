# Universal Brain Interface — design doc

**Status (2026-05-24):** PARTIALLY LANDED via overnight Chunks 1–4f
+ a late-session stability polish round.

What's live: every controllable entity (player / NPC / enemy /
boss) carries `Brain` + `ActionSet` + `ActorControl` sibling
components. `crates/ambition_sandbox/src/brain/` defines the Brain
enum + 7 templates (StandStill/Patrol/Wanderer/MeleeBrute/
Skirmisher/Sniper/BossPattern) + ActionSet (Swipe/Lunge/Slam/Bite/
PunchWeak/Rock/Arrow/Pistol/Bolt/BubbleShield/BossSpotlight). The
`emit_brain_action_messages` resolver writes one
`ActorActionMessage` per tick per resolved request; today the
stream is observed by `BrainActionCounter` only — no combat
spawner reads from it yet. Hostile NPC flip swaps brain +
ActionSet together. Tests: 762 sandbox lib + 265 engine = 1091
green.

What remains (daytime): EFFECTS-stage consumer flip (one melee
variant at a time, overlap-then-delete per the stale-component
benchmark); `update_player` consume the ActorControl frame instead
of `PlayerInputFrame`; narrow `ActorControlFrame::fire` to
`Option<Vec2>` once ActionSet owns speed. (`ae::Player` decomposition
✅ landed 2026-05-28 — the player entity carries 18 cluster
components; `PlayerMovementAuthority` / `PlayerBody` /
`ae::Player` are deleted.)

See [`../../TODO-controllable-entity.md`](../../TODO-controllable-entity.md)
for the multi-chunk plan, `dev/journals/ae-player-field-usage-
2026-05-24.md` for the field-usage audit + landed inventory, and
[`../recipes/extending-brains-and-action-sets.md`](../recipes/extending-brains-and-action-sets.md)
for the daytime EFFECTS-flip procedure.

**Original design (2026-05-23):** Continuation of the
controllable-entity unification arc that already merged
`EnemyRuntime` + `BossRuntime` behind `ActorControlFrame`
(commits `155171c`, `66c8b0b`, 2026-05-21). This doc proposes
extending the seam to cover NPCs and players, and discusses the
performance + ergonomics tradeoffs.

This doc is intended to be reviewable by an outside reader
(specifically ChatGPT): it tries to motivate the design without
assuming prior context, and the "Open questions" section names the
parts I'm uncertain about so a reviewer can engage productively.

## TL;DR

Every controllable entity in Ambition — player, NPC, enemy, boss,
RL agent, remote co-op player — should write the **same**
`ActorControlFrame` each tick via a swappable **brain backend**.
The sim integration step is identical across all entity kinds. The
only thing that varies is which brain backend fills the frame:

| Brain backend     | Used for                                              |
| ----------------- | ----------------------------------------------------- |
| `Player(slot)`    | A human at a controller. Copies inputs into the frame. |
| `Remote(peer_id)` | A networked co-op player. Reads frames off the wire.   |
| `RlPolicy(net)`   | An RL agent. Runs inference and writes the frame.      |
| `StateMachine(p)` | Current NPC/enemy AI — patrol / chase / attack / idle. |
| `Scripted(track)` | A cutscene puppet. Plays back authored input.          |

Two consequences:

1. **`aggressiveness: f32` (or similar gate) is the sole knob for
   whether an actor initiates attacks.** Today the NPC vs Enemy
   class boundary doubles as the attack gate. Post-refactor, a
   peaceful kernel guide and a hostile goblin both live in the
   same `Actor` data type — the goblin's state-machine brain has
   `aggressiveness = 0.8`, the guide's has `aggressiveness = 0.0`.
2. **Playing as a goblin is the same operation as adding a second
   human player.** Both swap a `Brain::StateMachine` for a
   `Brain::Player`; the sim doesn't notice.

## Why this exists

### Today's bifurcation

The sandbox currently has three actor data types with overlapping
responsibilities:

- **`NpcRuntime`** ([`content/features/npcs.rs`](../../crates/ambition_sandbox/src/features/npcs.rs))
  — peaceful patrol/stand-still loop. Hardcoded `update(world,
  target_pos, dt)`. Goes hostile via `hostile_from_npc(npc) ->
  EnemyRuntime` once strike count crosses a threshold.
- **`EnemyRuntime`** ([`content/features/enemies.rs`](../../crates/ambition_sandbox/src/features/enemies.rs))
  — `EnemyArchetype` + runtime integration state. Movement intent and
  attack requests now come from the actor/brain/action pipeline; stable
  per-actor variation lives in
  [`variation.rs`](../../crates/ambition_sandbox/src/mechanics/combat/variation.rs).
- **`BossRuntime`** ([`content/features/bosses.rs`](../../crates/ambition_sandbox/src/features/bosses.rs))
  — boss-pattern state machine. Movement was migrated onto
  `ActorControlFrame` in `66c8b0b`; attack patterns still run as a
  layered driver in the EFFECTS stage.

The player is its own thing entirely
([`player/components.rs::PlayerEntity`](../../crates/ambition_sandbox/src/player/components.rs)
+ the 18 cluster components), with its own input frame
(`PlayerInputFrame`) and update path
(`update_player_*_with_clusters`). It does **not** write into
`ActorControlFrame` today.

### Symptoms

- **Behavior duplication.** Locomotion-along-tangent,
  facing-direction, hit-flash, knockback, on-ground-check — every
  one of these exists in 2–3 places (player / NPC / enemy). When
  we changed `surface_normal` rotation rendering for the puppy
  slug, we had to wire it through `EnemyRuntime`,
  `ActorRuntime::feature_view`, `FeatureView`, and `sync_visuals`,
  but the player's pipeline wouldn't have needed any of that — its
  Transform write is in a different system.
- **Attack-gating coupled to entity class.** "Peaceful pirate
  heavy in the cove" today requires either authoring as
  `NpcSpawn` (gets NPC barks but no enemy archetype tuning) or as
  `EnemySpawn` with `aggro_radius = 0` and a sentinel
  `EnemyArchetype::attacks_player() == false` (introduced
  2026-05-23 in `53b1825` as a stop-gap). Both are workarounds for
  the absence of a clean `aggressiveness` knob.
- **Player as a special case.** Multiplayer, co-op, and "play as a
  goblin" all require the player to be just-another-actor — but
  it's not. The per-player audit in
  [`player-singleton-audit.md`](player-singleton-audit.md) has
  been chipping away at this since 2026-05-19; it has gotten the
  *data* per-player (`PlayerSlot`, `PlayerInputFrame`,
  `PrimaryPlayer`), but not the *behavior*. Behavior unification
  is the brain-interface piece.
- **RL training is unspawnable.** A goblin's "AI" is the
  state-machine in `EnemyRuntime::update`. To put an RL policy in
  control of a goblin, you'd have to fork `EnemyRuntime` or wrap
  the whole thing in a shim. With a brain interface, you swap
  `Brain::StateMachine` for `Brain::RlPolicy` on the same `Actor`.

### What we already have

The good news is that the **seam already exists** for enemies and
bosses:

```rust
// crates/ambition_sandbox/src/actor_control.rs
pub struct ActorControlFrame {
    pub desired_vel: Vec2,
    pub drop_through: bool,
    pub facing: f32,
    pub melee_pressed: bool,
    pub fire: Option<ActorFireRequest>,
}
```

`EnemyRuntime::build_control_frame` fills it. `step_kinematic`
consumes it. The brain → sim boundary is clean *for enemies*. The
work is to (a) extend the frame to cover the player's action
vocabulary, (b) write NPC and player brain backends that emit the
frame, and (c) collapse the actor data types into one.

## Vision

### Conceptual surface

```
                ┌────────────────────────────────┐
                │           Brain                │
                │  trait Brain {                 │
                │    fn tick(                    │
                │      sim: &SimSnapshot,        │
                │      actor: &ActorState,       │
                │      out: &mut ControlFrame,   │
                │    );                          │
                │  }                             │
                └────────────────────────────────┘
                             │
   ┌────────────┬────────────┼────────────┬────────────────┐
   │            │            │            │                │
 Player    Remote (peer)  StateMachine  Scripted        RlPolicy
 (controller   (network    (current      (cutscene       (inference
  inputs)       streamed     enemy AI)    track playback) over a net)
                inputs)
                             │
                             ▼
                  ┌─────────────────────┐
                  │   ActorControlFrame │
                  │  (desired_vel,      │
                  │   facing, attack,   │
                  │   fire, …)          │
                  └─────────────────────┘
                             │
                             ▼
                  ┌─────────────────────┐
                  │ step_kinematic +    │
                  │ EFFECTS stage       │
                  │ (gated by           │
                  │  aggressiveness)    │
                  └─────────────────────┘
```

Sample composition:

- **Kernel guide**: `Actor { aggressiveness: 0.0, brain:
  Brain::StateMachine(StateMachineCfg::Patrol { path: kernel_loop
  }) }`.
- **Goblin**: `Actor { aggressiveness: 0.7, brain:
  Brain::StateMachine(StateMachineCfg::EnemyArchetype(
  MediumStriker)) }`.
- **Player 1**: `Actor { aggressiveness: 1.0, brain:
  Brain::Player(PlayerSlot(0)) }`.
- **Player 2 (co-op)**: `Actor { aggressiveness: 1.0, brain:
  Brain::Player(PlayerSlot(1)) }`.
- **RL-driven test agent**: `Actor { aggressiveness: 1.0, brain:
  Brain::RlPolicy(policy_id) }`.
- **Cutscene-puppet pirate**: `Actor { aggressiveness: 0.0, brain:
  Brain::Scripted(intro_raid_track) }`.

### Aggressiveness is the sole attack gate

Today three independent gates can suppress attacks:

- `EnemyArchetype::attacks_player()` (the stop-gap)
- `aggro_radius == 0` (gates AI Chase mode)
- `attack_range == 0` (gates AI Attack mode)
- The choreography evaluator's internal `has_slot` /
  `MELEE_ENGAGE_DISTANCE` checks

Post-refactor: **one** gate at the EFFECTS-stage entry point:

```rust
if frame.melee_pressed
    && self.aggressiveness > 0.0
    && self.attack_cooldown <= 0.0
{
    // fire windup
}
```

Behavior at `aggressiveness ∈ (0, 1)` is design-space worth
exploring: timid actors that swing rarely, frenzied ones that
swing constantly. For the first cut, treat it as a boolean
(>0 → hostile).

## Brain backends

Concrete signatures and where they live.

### `Brain::Player(slot)`

```rust
fn tick_player(slot: PlayerSlot, inputs: &PlayerInputFrame, out: &mut ControlFrame) {
    out.desired_vel = inputs.move_axis() * MAX_PLAYER_VEL;
    out.facing = inputs.facing();
    out.melee_pressed = inputs.attack_pressed;
    out.fire = inputs.aim().map(|d| ActorFireRequest { dir: d, speed: ... });
}
```

The player brain reads `PlayerInputFrame` (already per-player from
the singleton audit) and writes the frame. **No game-logic changes
in this backend**; it's purely a translation. All the player's
existing movement quirks (coyote frames, jump buffer, dash window)
get moved into the player brain's internal state — or, better,
into the `step_kinematic` integration so they apply uniformly to
every `Brain::Player` actor regardless of who controls it.

### `Brain::Remote(peer_id)`

```rust
fn tick_remote(peer_id: PeerId, net: &NetInput, out: &mut ControlFrame) {
    if let Some(remote_frame) = net.latest_input_for(peer_id) {
        *out = remote_frame.control_frame.clone();
    }
    // else: hold last frame (or zero), depending on stall policy.
}
```

This is what makes co-op cheap. A second player on the same couch
uses `Brain::Player(PlayerSlot(1))`; a remote co-op partner uses
`Brain::Remote(peer)`. Both feed the same sim.

### `Brain::StateMachine(cfg)`

The current enemy AI, lifted as a brain backend. `cfg` carries the
data table that today lives in `EnemyArchetype` (patrol speed,
aggro radius, attack range, choreography). The AI evaluator stays
pure (`evaluate_character_ai_output`), the choreography stays pure
(`evaluate_choreography`), the brain just composes them.

NPC patrol (kernel guide etc.) is a state machine too — same
backend, different cfg. The "happy NPC default" cfg is
`StateMachineCfg::StandStill`.

### `Brain::Scripted(track)`

Plays back a recorded sequence of control frames. Used by
cutscenes (the intro raid puppet shoves) and by trace replay /
gold-master testing.

### `Brain::RlPolicy(policy_id)`

Calls into the RL inference layer. **Batched**: every tick, the
sim collects all `Brain::RlPolicy` actors, runs inference for all
of them in one batch, and writes back to their frames. This
amortizes the per-call overhead and lets the policy run on a
single forward pass over a stacked observation tensor.

The headless sim already has shapes for this
(`SandboxSim::observation`, `AgentAction`, `AgentObservation` per
the `TODO.md` "PyO3 binding" entry). The brain interface gives
those a place to land.

## Current state of the seam

A quick inventory so a reviewer can verify the proposal against
the code:

| Component                 | Shape today                                                                                      | What changes                                                            |
| ------------------------- | ------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------- |
| `EnemyRuntime`            | Carries archetype, health, AI state, `ActorControlFrame` is built per tick.                       | Becomes the **state-machine brain backend's** internal state.            |
| `BossRuntime`             | Like enemy; movement uses `ActorControlFrame`; attack patterns run in EFFECTS as a driver.        | Same as enemy. Attack-pattern driver becomes a `StateMachineCfg` variant. |
| `NpcRuntime`              | Hardcoded patrol/stand-still loop; no `ActorControlFrame`.                                        | Becomes another `StateMachineCfg` variant; loses its bespoke update fn.   |
| `PlayerEntity` + `update_player_*_with_clusters` | Reads `Res<ControlFrame>` / `PlayerInputFrame`; mutates the 18 player cluster components through `PlayerClustersMut`; not on `ActorControlFrame`. | Player path moves onto `ActorControlFrame` via `Brain::Player(slot)`.    |
| `EnemySpawn` / `NpcSpawn` LDtk entities | Two distinct entity defs with disjoint field sets.                                       | Collapse to one `ActorSpawn` with a `brain: BrainCfgRef` field.          |
| `ActorRuntime` enum       | `Peaceful(NpcRuntime)` / `Hostile(EnemyRuntime)`.                                                  | Becomes one `Actor` struct with a `Brain` field.                         |

The path is incremental: the seam is in the right place, the
dispatch shape is in the right place; what changes is who **fills**
the frame.

## Performance

The user's concern (verbatim, paraphrased): the elegant interface
is "everything is a brain," but trait-object dispatch every tick
per actor could be a real perf hit. They want the conceptual
surface without the cost. Options, in order of preference:

### 1. Enum-dispatch, not trait objects

```rust
pub enum Brain {
    Player(PlayerSlot),
    Remote(PeerId),
    StateMachine(StateMachineCfg),
    Scripted(ScriptedTrack),
    RlPolicy(PolicyId),
}

impl Brain {
    pub fn tick(&mut self, sim: &SimSnapshot, actor: &ActorState, out: &mut ControlFrame) {
        match self {
            Self::Player(slot) => tick_player(*slot, sim.input(*slot), out),
            Self::Remote(peer) => tick_remote(*peer, sim.net(), out),
            Self::StateMachine(cfg) => tick_state_machine(cfg, sim, actor, out),
            Self::Scripted(track) => tick_scripted(track, sim, out),
            Self::RlPolicy(id) => out.write_zero(), // batched separately
        }
    }
}
```

The match is a single switch, monomorphic, branch-predictor
friendly. No vtable, no heap allocation per actor. This is the
**default recommendation**.

Cost: adding a new brain backend touches the enum. Acceptable —
brain kinds are a small, stable set (~5 today, probably ≤10 ever).

### 2. Batch by backend variant

```rust
for actor in actors.iter_mut().filter(|a| matches!(a.brain, Brain::StateMachine(_))) {
    let Brain::StateMachine(cfg) = &mut actor.brain else { unreachable!() };
    tick_state_machine(cfg, sim, &actor.state, &mut actor.control);
}
for actor in actors.iter_mut().filter(|a| matches!(a.brain, Brain::Player(_))) { ... }
// ...
```

The inner loop becomes monomorphic per-backend. Cache locality
improves because every state-machine brain's internal state is
laid out contiguously (Bevy ECS already does this via Query). The
cost is that you walk the actor list once per backend variant —
~5 passes instead of 1. For 100s of actors this is invisible; for
10000s it matters.

For RL specifically, this is **required**: the policy network
runs one batched forward pass over all RL-controlled actors'
observations, which means collecting observations and writing
back frames are explicit batched steps, not a per-actor virtual
call.

### 3. Avoid `dyn Brain` entirely

Trait objects (`Box<dyn Brain>`) would let third parties register
custom backends, but:

- Per-tick virtual call cost (~5–10 ns/call × N actors × 60 Hz)
- Heap allocation per actor at spawn
- Loses the "all backends are in this enum, easy to grep" property

Verdict: don't. If someone wants a custom backend, they extend the
enum. The conceptual surface ("anyone can plug in a brain") holds
without the trait object — the abstraction is **the
ActorControlFrame**, not the trait.

### Profile targets

Before / after numbers worth collecting (none of these are
expected to regress, but the doc should say what to measure):

- `cargo bench -p ambition_sandbox actor_tick` — N actors of each
  brain kind, per-frame ms. Goal: same shape today and post-
  refactor.
- Headless step time (`SandboxSim::step` ×10000 ticks) for the
  current 6-goblin arena vs. a converted-to-brains version.
- RL batched-inference path: 32 RL agents × 60 Hz; ensure the
  per-step cost is ≤ 1.2× the sum of individual inference calls
  (i.e. the batching is paying off).

## Migration plan

Five chunks, each independently mergeable. Each is "behavior-
preserving" — the goal is to get to the universal brain without a
"big bang" rewrite. The last chunk deletes the stop-gap fields.

### Chunk 1 — Extend `ActorControlFrame` to the player's action vocabulary

Add `jump_pressed`, `dash_pressed`, `interact_pressed`,
`drop_through_pressed`, etc. to `ActorControlFrame`. Keep them
zero for the existing enemy/boss code paths (no behavior change).
This is the prerequisite for the player brain to write into the
frame.

### Chunk 2 — Introduce the `Brain` enum + `Actor` struct, behind a feature flag or as a parallel type

```rust
pub struct Actor {
    pub id: String,
    pub pos: Vec2,
    pub vel: Vec2,
    pub size: Vec2,
    pub aggressiveness: f32,
    pub brain: Brain,
    pub control: ActorControlFrame,
    pub state: ActorState,  // health, hit_flash, surface_normal, etc.
}
```

`Actor` is a new type. Don't migrate anyone yet. Add a `Brain::StateMachine` wrapper around the existing `EnemyRuntime::build_control_frame` logic.

### Chunk 3 — Move NPCs onto `Brain::StateMachine`

Convert `NpcRuntime` to fill an `ActorControlFrame` via
`StateMachineCfg::Patrol` / `StandStill`. Delete the bespoke
`NpcRuntime::update` once the brain backend covers every existing
NPC's behavior. **First time the gate** `aggressiveness > 0`
controls anything — kernel guide gets `aggressiveness = 0`, all
existing peaceful NPCs ditto.

### Chunk 4 — Move the player onto `Brain::Player`

Refactor `update_player_*_with_clusters` into a
`tick_player(slot, inputs, out)` function. The player's existing
per-tick state lives in `ActorState`. The Bevy systems that today
write to the player cluster components / `PlayerCombatState` now
read from `Actor` instead.

Beware: per the [stale-component benchmark candidate](../../dev/benchmark-candidates/bevy-ecs-stale-component-after-sync-removal-2026-05-15.md),
removing one of the player's sync systems without thinking about
who reads which component is exactly the bug class to avoid. Add
the new sync-write system **before** removing the old one;
overlap for one PR; then delete the old.

### Chunk 5 — Collapse `NpcSpawn` and `EnemySpawn` LDtk entities

Add a new `ActorSpawn` entity def with fields
`{ name, brain: BrainCfgRef, aggressiveness, ... }`. Migrate
existing levels one at a time. Once all are migrated, delete the
old defs. Remove `EnemyArchetype::attacks_player()` and the
`NpcRuntime`/`EnemyRuntime` dispatch in `ActorRuntime` —
everything is `Actor` now.

### Chunk 6 — Add `Brain::RlPolicy` + batched inference path

Ship the headless trainer's brain backend. PyO3 binding from
`TODO.md` slots into this.

## Open questions

These are the parts the design wasn't sure about up front.
Several were resolved by the Chunk 1–4f implementation; the
remaining unresolved questions land here for daytime work.

1. ~~**Where does `aggressiveness` actually live?**~~ **Resolved
   (2026-05-24):** Inside `Brain::StateMachine`'s cfg per
   template. Patrol, Wanderer, MeleeBrute, Skirmisher, Sniper,
   BossPattern each carry an `aggressiveness: f32`. `Brain::
   is_hostile()` aggregates and short-circuits via match.

2. ~~**Does the player brain still own input-edge state, or does
   the `ActorControlFrame` carry rising/falling edges?**~~
   **Resolved (2026-05-24):** Edges land on the frame (Chunk 1
   added `jump_pressed/held/released`, `dash_pressed`,
   `interact_pressed`, `shield_held`, `special_pressed`).
   `ActorControlFrame::clear_edges()` helps integrators that
   consume across multiple stages.

3. **Cutscene puppets vs. story scripts.** Still open. Suggestion
   from 2026-05-24 plan: `Scripted` is a brain backend;
   cross-actor choreography (whole cutscenes) is a separate
   Director system that temporarily swaps each participant's
   brain. No concrete consumer yet.

4. **Determinism for RL training.** Still open at the policy
   level. Sim-layer determinism work continues (no `Instant::now()`,
   no `HashMap` order leaks into observations). The brain ABI
   test `brain_tick_is_deterministic_given_same_snapshot` pins
   the local guarantee.

5. ~~**Brain swapping mid-game.**~~ **Resolved (2026-05-24):**
   Internal state transition for the NPC peaceful→hostile flip,
   *and* the damage handler swaps both the brain template and
   the ActionSet (`MeleeBrute::STRIKER_DEFAULT` + Swipe) so the
   parallel shape is internally consistent. This keeps brains
   stateful but contained — `Scripted` and `RlPolicy` get their
   own stateful sub-types when they land.

6. **Save-game shape.** Still open. Today's brains aren't
   serialized; daytime work picks the savable shape per backend.
   The plan in `TODO-controllable-entity.md` Chunk 3 notes save
   version bumps come with this work.

## Out of scope (explicitly deferred)

- Per-`aggressiveness` value smoothing (timid vs. frenzied
  attackers). First cut treats it as a boolean.
- The visual / animation layer rebinding. Sprites continue to
  resolve from the existing `npc_sprite_label` table — the
  display-name-keyed lookup doesn't care about brain backends.
- The combat-slot board's `actor_id` keying. Slots are allocated
  by id; the brain swap shouldn't disturb a slot's actor mapping
  because the id is the `Actor`'s, not the brain's.
- Networking architecture. `Brain::Remote` is a placeholder —
  the actual transport is a separate doc.

## Related docs

- [`docs/systems/character-ai-refactor.md`](../systems/character-ai-refactor.md)
  — current state of the enemy+boss seam (the foundation this
  builds on).
- [`docs/planning/player-singleton-audit.md`](player-singleton-audit.md)
  — the per-player work that's already done (data) and what
  remains (behavior).
- [`docs/systems/input-and-control-frame.md`](../systems/input-and-control-frame.md)
  — player input plumbing as it stands.
- [`dev/benchmark-candidates/bevy-ecs-stale-component-after-sync-removal-2026-05-15.md`](../../dev/benchmark-candidates/bevy-ecs-stale-component-after-sync-removal-2026-05-15.md)
  — the failure mode to avoid when removing the old player sync
  systems in Chunk 4.
- [`TODO.md`](../../TODO.md) — the "Universal brain interface"
  entry in the Proposed section is the work item this doc
  expands.
