# Extending brains and ActionSets

A practical recipe for daytime work on the universal-brain
interface. See [`../systems/brain-driver.md`](../systems/brain-driver.md)
for the overview and
[`../../TODO-controllable-entity.md`](../../TODO-controllable-entity.md)
for the multi-chunk plan.

## Three places work usually lands

1. **Brain template** (`crates/ambition_sandbox/src/brain/state_machine.rs`)
   — when an actor needs a new *policy* (the state graph + transition
   rules). New variant of `StateMachineCfg` + a `tick_<template>`
   function. Per-actor state lives in a sibling struct (`*State`).
2. **ActionSet spec** (`crates/ambition_sandbox/src/brain/action_set.rs`)
   — when an actor needs a new *capability* (the concrete effect a
   melee/ranged/special action resolves to). New variant of
   `MeleeActionSpec` / `RangedActionSpec` / `SpecialActionSpec`.
3. **Per-archetype mapping** (`crates/ambition_sandbox/src/content/features/ecs/spawn.rs`)
   — when an existing enemy archetype should resolve its melee /
   ranged through a different spec. Update
   `enemy_default_action_set` (or the player's
   `default_player_action_set`).

## Adding a new brain template

A brain template is a reusable AI policy. Two enemies sharing the
template share state-machine code but can still look different
because their ActionSets resolve abstract intent differently.

### Step 1 — add the variant + cfg + state

```rust
// crates/ambition_sandbox/src/brain/state_machine.rs

pub enum StateMachineCfg {
    // existing variants ...
    Charger {
        cfg: ChargerCfg,
        state: ChargerState,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct ChargerCfg {
    pub aggressiveness: f32,
    pub aggro_radius: f32,
    pub charge_speed: f32,
    pub windup_s: f32,
    pub recover_s: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ChargerState {
    pub windup_remaining: f32,
    pub charging: bool,
    pub recover_remaining: f32,
}
```

### Step 2 — add the tick fn + dispatch arm

```rust
fn tick_charger(cfg: &ChargerCfg, state: &mut ChargerState, snap: &BrainSnapshot, out: &mut ae::ActorControlFrame) {
    *out = ae::ActorControlFrame::neutral();
    let to_target = snap.target_pos - snap.actor_pos;
    let dist = to_target.length();
    if dist > cfg.aggro_radius || !snap.target_alive {
        return;
    }
    // (rest of charger logic — windup → charge → recover loop)
}

pub fn tick_state_machine(...) {
    // existing arms ...
    StateMachineCfg::Charger { cfg, state } => tick_charger(cfg, state, snapshot, out),
}
```

### Step 3 — extend `is_hostile` for the variant

```rust
impl StateMachineCfg {
    pub fn is_hostile(&self) -> bool {
        match self {
            // existing arms ...
            Self::Charger { cfg, .. } => cfg.aggressiveness > 0.0,
        }
    }
}
```

### Step 4 — re-export from `brain/mod.rs`

```rust
pub use state_machine::{
    // existing exports ...
    ChargerCfg, ChargerState,
};
```

### Step 5 — write tests

Per [[feedback-bevy-testing-pattern]] use `BrainSnapshot::idle()`
overrides. Cover the basic state transitions:

```rust
#[test]
fn charger_windups_then_charges_when_in_range() { ... }

#[test]
fn charger_holds_when_target_dead() { ... }
```

### Step 6 — wire it into the right archetype spawn

```rust
// crates/ambition_sandbox/src/content/features/ecs/spawn.rs
fn enemy_default_brain(enemy: &EnemyRuntime) -> Brain {
    match enemy.archetype {
        EnemyArchetype::ChargerBeast => Brain::StateMachine(StateMachineCfg::Charger {
            cfg: ChargerCfg { ... },
            state: ChargerState::default(),
        }),
        // ...
    }
}
```

## Adding a new MeleeActionSpec / RangedActionSpec

An ActionSpec is the concrete attack an actor performs when its
brain says `melee_pressed = true`. Each variant owns its windup →
active → recover animation timing.

### Step 1 — extend the enum + add a spec struct

```rust
// crates/ambition_sandbox/src/brain/action_set.rs

pub enum MeleeActionSpec {
    // existing variants ...
    Headbutt(HeadbuttSpec),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadbuttSpec {
    pub windup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub damage: i32,
    pub reach_px: f32,
    pub knockback_strength: f32,
}

impl HeadbuttSpec {
    pub const RAM_DEFAULT: Self = Self {
        windup_s: 0.20,
        active_s: 0.06,
        recover_s: 0.40,
        damage: 2,
        reach_px: 26.0,
        knockback_strength: 280.0,
    };
}
```

### Step 2 — re-export from `brain/mod.rs`

```rust
pub use action_set::{
    // existing exports ...
    HeadbuttSpec,
};
```

### Step 3 — wire archetypes to the new spec

```rust
fn enemy_default_action_set(enemy: &EnemyRuntime) -> ActionSet {
    match enemy.archetype {
        EnemyArchetype::HornedGoat => ActionSet {
            melee: Some(MeleeActionSpec::Headbutt(HeadbuttSpec::RAM_DEFAULT)),
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        },
        // ...
    }
}
```

### Step 4 — write resolver tests

```rust
#[test]
fn headbutt_action_carries_knockback_strength() {
    let actions = ActionSet { melee: Some(MeleeActionSpec::Headbutt(HeadbuttSpec::RAM_DEFAULT)), ..Default::default() };
    let mut frame = ae::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
    match reqs[0] {
        ActionRequest::Melee { spec: MeleeActionSpec::Headbutt(spec), .. } => {
            assert!(spec.knockback_strength > 0.0);
        }
        _ => panic!(),
    }
}
```

### Step 5 — daytime work: wire EFFECTS-stage consumer

The resolver writes `ActorActionMessage { actor, request:
ActionRequest::Melee { spec, .. } }` into the channel. The
EFFECTS-stage spawn system needs an arm for each new spec
variant that translates it into a real hitbox / particle / SFX:

```rust
fn spawn_melee_hitboxes(mut messages: MessageReader<ActorActionMessage>, ...) {
    for msg in messages.read() {
        if let ActionRequest::Melee { spec, origin, facing, attack_axis } = msg.request {
            match spec {
                MeleeActionSpec::Headbutt(s) => spawn_headbutt(origin, facing, s, ...),
                // existing arms ...
            }
        }
    }
}
```

## Adding a brain backend (e.g. `Brain::Scripted`)

For a new top-level backend (not a state-machine template) — e.g.
`Scripted` for cutscene puppets, `Remote` for networked co-op,
`RlPolicy` for RL-driven agents.

### Step 1 — extend the Brain enum + dispatch

```rust
// crates/ambition_sandbox/src/brain/mod.rs

pub enum Brain {
    Player(PlayerSlot),
    StateMachine(StateMachineCfg),
    Scripted(ScriptedCfg),  // new
}

impl Brain {
    pub fn tick(&mut self, snapshot: &BrainSnapshot, out: &mut ae::ActorControlFrame) {
        match self {
            // existing arms ...
            Brain::Scripted(cfg) => scripted::tick_scripted_brain(cfg, snapshot, out),
        }
    }

    pub fn is_hostile(&self) -> bool {
        match self {
            // existing arms ...
            Brain::Scripted(cfg) => cfg.is_hostile(),
        }
    }
}
```

### Step 2 — implement the backend in a new submodule

```rust
// crates/ambition_sandbox/src/brain/scripted.rs
//
// Cursor + recorded sequence of (frame, dt) pairs the brain plays back.

pub struct ScriptedCfg { ... }

pub fn tick_scripted_brain(cfg: &mut ScriptedCfg, snap: &BrainSnapshot, out: &mut ae::ActorControlFrame) {
    ...
}
```

### Step 3 — declare the submodule

```rust
// crates/ambition_sandbox/src/brain/mod.rs
pub mod scripted;
```

### Step 4 — write tests (same patterns as state_machine tests).

## Common pitfalls

- **`#[derive(Component)]` is required** on any brain-side type
  spawned as a sibling component (ActionSet learned this the hard
  way).
- **`Brain` is `#[derive(Clone, Debug)]`, not `Copy`** because
  some templates carry `String` (BossPattern.encounter_id) or
  `Vec<f32>` (Wanderer.recent_reversals). Don't try to make it
  Copy — clone explicitly where needed.
- **Snapshot construction is per-actor per-tick.** Don't allocate
  on the heap inside the snapshot building.
- **The resolver returns a `Vec<ActionRequest>`, not Option.**
  Brain ticks emitting both `melee_pressed = true` and `fire =
  Some(dir)` get two requests; the resolver doesn't deduplicate.
  EFFECTS-stage consumers handle multi-action ticks.
- **No `TelegraphSpec`.** Telegraphs are part of an attack spec's
  windup phase. Don't add separate telegraph state to brain
  templates.
- **Aggressiveness lives in the brain, not the actor.** There is
  no `ActorAggression` sibling component. Query
  `brain.is_hostile()` if you need the answer.

## Validation gates

After every change:

```bash
~/.cargo/bin/cargo check -p ambition_engine
~/.cargo/bin/cargo check -p ambition_sandbox
~/.cargo/bin/cargo test  -p ambition_engine  --lib
~/.cargo/bin/cargo test  -p ambition_sandbox --lib brain::
~/.cargo/bin/cargo test  -p ambition_sandbox --lib
~/.cargo/bin/cargo run   -p ambition_sandbox --bin headless -- --ticks 30
```
