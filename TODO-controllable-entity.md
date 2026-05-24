# TODO: Controllable-entity unification (Player / Enemy / NPC)

**Status:** plan draft, 2026-05-24. Awaiting Jon's review before overnight execution.

**Source design doc:** [docs/planning/universal-brain-interface.md](docs/planning/universal-brain-interface.md)
**Source TODO entry:** [TODO.md](TODO.md) "Universal brain interface" (Proposed section, L230-235)
**Companion docs:** [docs/systems/character-ai-refactor.md](docs/systems/character-ai-refactor.md), [docs/planning/player-singleton-audit.md](docs/planning/player-singleton-audit.md)

## Decisions captured (Jon, 2026-05-24)

| Decision               | Choice                                                                                                                    |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Overnight scope        | Chunks 1–4 (extend frame, Brain enum, NPC migration, **full player decomposition including `ae::Player` teardown**)        |
| Unified actor shape    | **Decomposed ECS components** (no `Actor` struct; sibling components on the actor entity)                                  |
| Aggressiveness location| **Inside `Brain::StateMachine` cfg** — every enemy/NPC is some kind of state-machine brain (simple or reused)             |
| Brain reuse model      | **Few brain templates, many ActionSets.** Same brain policy across many enemies; the *concrete action* per state varies per entity (leap vs swipe vs throw). Brains shrink to a small reusable set. |
| NPC → hostile flip     | **Internal state transition** within one brain (brain persists; sub-state changes)                                         |
| Crate ownership        | **Sandbox** — `Brain` enum + every backend live in `ambition_sandbox`; engine stays narrow (just `ActorControlFrame` + `step_kinematic`) |
| Build/test gate        | Every commit compiles clean + `cargo test -p ambition_engine --lib` + `cargo test -p ambition_sandbox --lib` pass before moving on |
| Pre-release compat tax | None ([[feedback-pre-release-no-compat]]) — single-commit replacements over bridge patterns; save format may break        |
| Commit target          | **Directly to `main`** (current branch). One commit per green sub-step. No push to remote; no amend.                        |
| Chunk 4 posture        | **Race** — start Chunk 4 immediately after Chunk 3 lands; stop at the last green commit per the risk gate if budget runs out |

## Brain templates × ActionSet decomposition (2026-05-24 refinement)

Jon's framing: **brains are reusable policy templates; ActionSets are per-entity capabilities.** Two enemies can share the same `MeleeBrute` brain (same state graph: idle → aggro → attack → recover → idle, same transition rules, same aggressiveness) and look completely different in the world because their `ActionSet` resolves the abstract action differently — one leaps, one swipes, one stabs. The brain doesn't model telegraphs separately; telegraphs are part of an attack's animation, owned by the attack spec in the ActionSet.

```
Brain (policy)               Frame (abstract intent)        ActionSet (capability)         Concrete effect
─────────────                ───────────────────────        ──────────────────────         ───────────────
MeleeBrute(cfg)  ─────────►  frame.melee_pressed = true ──► Goblin.attack = Leap     ────► spawn leap arc + hitbox
                                                            Pirate.attack = Swipe    ────► spawn swing hitbox
                                                            Shark.attack = Bite      ────► spawn jaw hitbox

Skirmisher(cfg)  ─────────►  frame.desired_vel = vec        Goblin.move = Hop        ────► hop animation + step_kinematic
                             frame.fire = Some(dir)         Pirate.move = Walk       ────► walk anim + step_kinematic
                                                            Goblin.fire = Rock       ────► spawn rock projectile
                                                            Pirate.fire = Pistol     ────► spawn pistol shot
```

**Implications:**

- `StateMachineCfg` shrinks to a small set of *brain templates* (`StandStill`, `Patrol`, `MeleeBrute`, `Skirmisher`, `Sniper`, `BossPattern(id)`, …) — not one variant per enemy archetype. Chunk 2's transitional `StateMachineCfg::Enemy(EnemyArchetype)` is replaced by mapping each `EnemyArchetype` to a `(BrainTemplate, ActionSet)` pair during Chunk 3 migration.
- `ActorControlFrame` carries abstract intent. The `fire` field becomes a direction-only request; **speed + projectile kind move into ActionSet** so the brain doesn't bake per-entity values. The frame may also gain `attack_axis: Vec2` for direction-of-attack so the ActionSet knows where to swing.
- **`ActionSet` is a sibling component** on the actor entity. Resolved by the EFFECTS stage when the frame says "attack" / "fire". Today this logic is scattered in `attack_choreography.rs` + projectile spawners — formalizing it as ActionSet is mostly factoring, not new behavior.
- **Possession is now cheap.** Player possesses a goblin → swap `Brain::StateMachine(MeleeBrute)` for `Brain::Player(slot)` on the goblin entity; keep the goblin's `ActionSet`. Player presses Attack → frame.melee_pressed=true → goblin's ActionSet resolves it as a leap. Player inherits the goblin moveset automatically.
- **Multi-player with different bodies is the same operation.** Two players can pilot entities with different ActionSets — Player 1 in a standard body, Player 2 in a fast/fragile skirmisher body — without forking the player input pipeline.
- **Bosses fit the same model.** Boss attack patterns become a `Brain::StateMachine(BossPattern(id))` policy with a richer ActionSet (multi-stage hitboxes, choreographed shots). Each attack's windup is part of its animation, not a separate telegraph spec. The boss runtime's bespoke pattern-state-machine collapses into the same dispatch.

**Cost flagged:** ActionSet's shape needs care to avoid becoming another god-struct. Plan: model it as a small struct of `Option<ActionSpec>` per abstract verb (`melee_attack`, `ranged`, `move_style`, `special`) where `ActionSpec` is itself an enum of concrete shapes (`Leap`, `Swipe`, `Stab`, `Throw`, …) and each variant carries its own animation timing (windup → active → recover). Adding a new attack style adds an enum variant + a resolver, not a new component.

**Effect on Chunks 1–4:**

- Chunk 1 (frame): `fire: Option<Vec2>` instead of `Option<ActorFireRequest>`; add `attack_axis: Vec2`.
- Chunk 2 (Brain): `StateMachineCfg` = template enum (small set). `ActionSet` introduced as a sibling component with placeholder variants matching today's enemy taxonomy.
- Chunk 3 (NPC): NPCs get `ActionSet::peaceful()` (no attack); hostile transition flips brain sub-state but the ActionSet is what determines whether the goblin's "attack" is a punch or a shove.
- Chunk 4 (player): player entity carries an ActionSet describing the player's full moveset (attack types, dash, projectile spawn). Brain::Player writes intent into the frame; the ActionSet resolves it — same code path enemies use. This is what makes "play as a goblin" a one-line swap.

## Design risks flagged before execution

Jon asked for risks across **performance, maintainability, extensibility**. Captured honestly:

### Performance

- **Brain dispatch:** enum + match is monomorphic + branch-predictor friendly. No concern. Don't drift to `Box<dyn Brain>`.
- **ECS-component decomposition of player adds archetype/query cost.** Today `update_player` mutates one `ae::Player`. Post-Chunk 4, a player tick reads/writes ~8–12 components per actor. For 1 player this is invisible. For 4 local + RL agents (Chunk 6's future), worth benchmarking — the cost is component lookups, not Brain dispatch. *Action:* before/after `cargo bench` on a representative scene **if** a baseline bench exists; otherwise capture wall-clock per `app.update()` in the headless harness.
- **Compile times** ([[feedback-compile-time]] — sandbox is already ~10min). Adding ~10 new components + the Brain enum + StateMachineCfg variants will grow type-checking time. Estimated +5–15% on full rebuilds; incremental should stay fast if we avoid `Reflect` on the new types and keep them out of `lib.rs` re-exports. *Action:* land Brain in its own module; no `Reflect` until something needs it.
- **Save state growth** from internal hostility transitions — brains carry both peaceful and hostile sub-state. Marginal; no concern.

- **"Unbrained" actor optimization (considered, deferred).** Jon raised: could simple actors (puppy slug) skip the brain abstraction and run direct hardcoded behavior for perf? Honest answer:

  | Option | Saves | Costs |
  | ------ | ----- | ----- |
  | Unbrained (parallel ECS path) | ~1-2 ns/tick of enum match arm dispatch | Two implementations of slug behavior; possession requires extra "promote to brained" step; another ECS system in the schedule; drift risk |
  | Lazy `BrainSnapshot` | Snapshot construction cost for brains that don't read all fields | Trivial: each brain pulls what it needs |
  | Batch by brain variant via marker components | Same as unbrained, but monomorphic; preserves possession | Bevy compile-time cost; doesn't change the surface |

  **Decision: don't add an unbrained path.** Brain dispatch is an enum match, not a virtual call — the cost is already near zero. The real risk if perf ever matters is `BrainSnapshot` construction, mitigated by making it lazy (per-field opt-in). If profiling later shows brain dispatch is hot, the right fix is "batch by brain variant via marker components" — it keeps possession + multi-player + RL working without forking the code path. Reserve unbrained as a documented escape hatch only.

### Maintainability

- **No uniform attack gate.** Dropped per Jon (2026-05-24) — aggressiveness lives in the brain; each backend gates its own attacks. Accepted cost: no single point to add "global passive mode" or "RL: suppress all attacks." When such a knob is needed, options are a sim-wide gate component or a `Brain::is_hostile()` convention used by debug tooling. Not blocking.

- **StateMachineCfg variant size is bounded by brain templates, not enemies** (per Jon's brain-template + ActionSet refinement). Brains stay small (`StandStill`, `Patrol`, `MeleeBrute`, `Skirmisher`, `Sniper`, `BossPattern(id)`); per-entity variety lives in `ActionSet`. The variant-explosion risk shifts to `ActionSet` — guarded by the small-Option-struct-with-enum-fields shape described in the decomposition section.

- **Player decomposition broad-churn risk.** `ae::Player` is read in ~50+ call sites (queries, dev panels, save/load, HUD, sprite sync, FX). Chunk 4 means touching each. Overlap-then-delete discipline is mandatory per [the stale-component journal](dev/benchmark-candidates/bevy-ecs-stale-component-after-sync-removal-2026-05-15.md). *Action:* in Chunk 4, **add** new components alongside `ae::Player` first, **migrate** readers one subsystem at a time, **delete** `ae::Player` last. Each step a commit. If we run out of overnight budget, the migration freezes at whatever subsystem we're on — game still works because both shapes coexist.

- **Active S-blocker:** "Wall-clipping bugs in the intro" (TODO.md L44). The bug is almost certainly in `ae::Player`'s collision/movement state. Decomposing that during the overnight could either fix it (state isolation often surfaces bugs) or make it temporarily harder to repro. *Action:* before Chunk 4, run `cargo test -p ambition_sandbox --test repro_walls` and capture the current pass/fail snapshot so we know if the decomposition regresses anything.

### Extensibility

- **Closed Brain enum:** new backends require touching the enum. Accepted per design doc; mod/plugin authors can't add a brain without forking. Fine for now (brain set is small + stable). Escape hatch if needed later: `Brain::Custom(Arc<dyn BrainFn>)` — defer until a concrete consumer wants it.

- **Internal-transition brain state.** Brains accumulate state across mode changes (peaceful conversation flags persist after going hostile). Likely fine for game feel ("Bob remembers you wronged him"). Worth noting as a property, not a bug.

- **No `Brain::Remote` / `Brain::Scripted` / `Brain::RlPolicy` overnight.** Scoped out:
  - `Brain::Remote` needs a transport layer — out of scope.
  - `Brain::Scripted` needs a recorder. The existing `gameplay-trace-recorder` records inputs, not control frames; building the bridge is a separate session.
  - `Brain::RlPolicy` needs PyO3 + batched inference plumbing. TODO.md "PyO3 binding for SandboxSim" is the prerequisite.

  Plan adds enum variants as `unimplemented!()` placeholders **only if** they're needed for an exhaustive `match`. Otherwise enum is just `Player | StateMachine` overnight, extended later.

- **Save-game shape.** Internal transitions mean save format changes for NPCs in Chunk 3. Pre-release rule says break it. Existing saves of intro slice progress will not load. *Action:* document that overnight commits invalidate saved games; bump any save version field.

## Chunks

### Chunk 1 — Extend `ActorControlFrame` to player verbs

**Goal:** add the player's action surface to the frame. Zero behavior change. Prerequisite for all later chunks.

**Files touched:**
- [crates/ambition_engine/src/actor_control.rs](crates/ambition_engine/src/actor_control.rs) — add fields

**New `ActorControlFrame` fields (proposed names):**
- `jump_pressed: bool` — rising edge "wants to jump this tick"
- `jump_held: bool` — sustain for variable-height jumps
- `dash_pressed: bool` — rising edge
- `interact_pressed: bool` — rising edge (E/F/RB binding from [[feedback-interact-binding]])
- `shield_held: bool` — bubble-shield / parry sustain
- `special_pressed: bool` — abstract "use special" — ActionSet resolves to which special
- `attack_axis: Vec2` — direction the attack should resolve in (e.g. up-tilt vs down-air). Zero = use facing.
- *(`drop_through` already exists)*

**Changed field (per Jon's ActionSet refinement):**
- `fire: Option<Vec2>` (just direction) — was `Option<ActorFireRequest { dir, speed }>`. Speed + projectile kind move into ActionSet so the brain emits abstract "fire this way" and the entity's capability decides what flies.

**Edge semantics decision (captured from doc open-question #2):** edges live on the frame, not in brains. Reason: keeps brains stateless w.r.t. input; enemies set the bool true the tick they want the action; `step_kinematic` and EFFECTS consume them as rising edges. Player brain converts `PlayerInputFrame`'s edge state into these.

**Backwards-touch:** the `fire` field change touches every existing brain that fires (enemies, bosses). Their archetype data already carries speed; the migration is "move the speed read from the brain to the ActionSet-equivalent at the spawn site." Done in this chunk so enemy/boss code paths stay green. Verify that all four current `fire =` writers compile against the new shape.

**Tests added:**
- `extended_frame_defaults_neutral` — every new field starts false/None.

**Exit criteria:** existing engine + sandbox tests pass; enemy/boss callers compile unchanged (only new fields, never break old).

**Commit message shape:** `engine: extend ActorControlFrame to cover player verbs (no behavior change)`

---

### Chunk 2 — Introduce `Brain` enum + sibling components in sandbox

**Goal:** land the data shape without migrating any actor onto it. Compile-time-only change; everything still runs through `EnemyRuntime` / `NpcRuntime` / `update_player`.

**New module:** `crates/ambition_sandbox/src/brain/` with:
- `mod.rs` — `Brain` enum + `Brain::tick()` dispatch
- `state_machine.rs` — `StateMachineCfg`, `StateMachineState`, `tick_state_machine()`
- `player_brain.rs` — `tick_player_brain(slot, &PlayerInputFrame, &mut ActorControlFrame)` (function only; no actor uses it yet)

**Components (siblings on the actor entity):**
- `ActorBrain(Brain)` — the brain instance (policy)
- `ActorBrainState(BrainState)` — per-brain runtime state (StateMachine sub-state, scripted cursor, etc.)
- `ActorControl(ActorControlFrame)` — last-tick output of the brain
- `ActionSet(ActionSet)` — per-entity capability (how abstract intent resolves to concrete behavior)

Aggressiveness is **not** a sibling component — it lives inside the brain (Jon's call). Brain exposes `is_hostile(&self, state: &BrainState) -> bool` so debug tooling has a uniform query.

**`Brain` enum (Chunk 2 minimum):**
```rust
pub enum Brain {
    Player(PlayerSlot),
    StateMachine(StateMachineCfg),
    // Remote/Scripted/RlPolicy variants added in future chunks
}

impl Brain {
    pub fn tick(&mut self, snapshot: &BrainSnapshot, state: &mut BrainState, out: &mut ActorControlFrame) { ... }
    pub fn is_hostile(&self, state: &BrainState) -> bool { ... }
}
```

**`StateMachineCfg` — small set of brain templates (per Jon's refinement):**
```rust
pub enum StateMachineCfg {
    StandStill,
    Patrol(PatrolCfg),               // fixed waypoint loop
    Wanderer(WandererCfg),           // move forward; on wall, climb-if-able else reverse; pause on rapid chatter
    MeleeBrute(MeleeBruteCfg),       // approach + melee + recover
    Skirmisher(SkirmisherCfg),       // strafe + ranged harass
    Sniper(SniperCfg),               // hold position + long-range fire
    BossPattern(BossEncounterId),    // scripted multi-phase
}
```

Per-entity variety lives in `ActionSet`, not in new `StateMachineCfg` variants. New enemies pick an existing brain template + a per-entity `ActionSet`.

`WandererCfg` (drafted from Jon's puppy-slug spec):
```rust
pub struct WandererCfg {
    pub speed: f32,
    pub climb_walls: bool,                // true: try to climb on wall hit; false: reverse immediately
    pub reverse_chatter_threshold: u8,    // reversals within window before pausing
    pub reverse_chatter_window: f32,      // seconds
    pub chatter_pause_seconds: f32,
}
```

Brain state for wanderer (tiny — fits in a sliding window):
```rust
pub struct WandererState {
    pub recent_reversal_times: ArrayVec<f32, 4>,  // sim-time stamps for the chatter check
    pub pause_until: f32,
}
```

**`ActionSet` — per-entity capability:**
```rust
pub struct ActionSet {
    pub melee_attack: Option<MeleeActionSpec>,  // Leap / Swipe / Stab / Lunge / Bite / ...
    pub ranged: Option<RangedActionSpec>,       // Rock / Pistol / Arrow / Fireball / ...
    pub move_style: MoveStyleSpec,              // Walk / Hop / Float / Slither / ...
    pub special: Option<SpecialActionSpec>,     // entity-specific signature move (boss/player)
}
```

Each `*ActionSpec` is itself an enum of concrete shapes; **each variant owns its full animation timing (windup → active → recover)**. Adding a new attack style is an enum-variant addition + a resolver in the EFFECTS stage, never a new component.

**No separate telegraph spec.** Per Jon (2026-05-24): telegraphs aren't a code concept — they're part of an attack's animation. The windup phase of `Swipe`, `Lunge`, `Slam`, etc. *is* the telegraph. This already matches `attack_choreography.rs`'s phased model; no `TelegraphSpec` field on the ActionSet.

**Existing taxonomy migration map** (drafted, validated in Chunk 3):
| `EnemyArchetype` (today)   | Brain template       | aggressiveness | ActionSet (sketch)                                       |
| -------------------------- | -------------------- | -------------- | -------------------------------------------------------- |
| `Striker`                  | `MeleeBrute`         | hostile        | `melee=Swipe (windup→swing→recover), move=Walk`          |
| `Brute`                    | `MeleeBrute`         | hostile        | `melee=Lunge(heavy, slow windup), move=Walk(slow)`       |
| `FastFall`                 | `MeleeBrute`         | hostile        | `melee=Slam (hop-windup→slam→recover), move=Hop`         |
| `Ranger`                   | `Skirmisher`         | hostile        | `ranged=Arrow (draw→loose), move=Strafe`                 |
| `Sandbag`                  | `StandStill`         | peaceful       | `melee=Punch(weak)` when struck (counter-attack)         |
| `PuppySlug`                | `Wanderer { climb_walls=true, chatter_threshold=3, window=1.0s, pause=2.0s }` | **peaceful (aggressiveness=0)** | `move=Slither` only — no melee, no ranged |
| `PirateHeavy` (peaceful)   | `Patrol`             | peaceful       | (no attack actions when aggressiveness gate closed)      |

If the migration map reveals a brain template that doesn't fit any existing template cleanly, **add the template**, don't widen an existing one.

**Tests added:**
- `brain_player_tick_translates_input_frame` — feed a `PlayerInputFrame`, assert frame fields.
- `brain_state_machine_stand_still_emits_neutral`
- `brain_state_machine_patrol_steers_toward_waypoint`
- `brain_state_machine_wanderer_reverses_on_wall` — synthetic wall hit; assert facing flips and `desired_vel.x` sign flips next tick.
- `brain_state_machine_wanderer_climbs_when_able` — wall hit with `climb_walls=true` and a climbable surface; assert brain switches to climb mode (not reverse).
- `brain_state_machine_wanderer_pauses_on_chatter` — feed N reversals within the window; assert next tick emits `desired_vel = ZERO` until the pause expires.
- `brain_state_machine_melee_brute_winds_up_then_attacks` — verify the attack spec's windup→active→recover phasing runs from a single `melee_pressed=true` brain output (no separate telegraph step).
- `actionset_resolves_attack_to_concrete_spec` — two ActionSets resolve `frame.melee_pressed=true` into distinct effect requests (Swipe vs Leap), each carrying its own windup timing.
- `brain_is_hostile_reports_per_cfg` — Wanderer + StandStill + peaceful Patrol all return `false`; MeleeBrute + Skirmisher + Sniper return `true`.

**Exit criteria:** new module compiles; no actor wired in; sandbox still runs identically.

**Commit message shape:** `sandbox: add Brain enum + ActionSet + sibling components (no actors migrated yet)`

---

### Chunk 3 — Move `NpcRuntime` onto `Brain::StateMachine`

**Goal:** every NPC actor entity carries `ActorBrain` + `ActorBrainState` + `ActorControl`. The bespoke `NpcRuntime::update` is deleted. First time `Brain::is_hostile()` gates anything in the sim.

**Files touched:**
- [crates/ambition_sandbox/src/content/features/npcs.rs](crates/ambition_sandbox/src/content/features/npcs.rs) — port behavior into `StateMachineCfg::Patrol` + `StandStill`; delete `NpcRuntime::update`
- [crates/ambition_sandbox/src/content/features/components.rs](crates/ambition_sandbox/src/content/features/components.rs) — adjust `ActorRuntime` variant for the new shape, or remove if no longer needed
- [crates/ambition_sandbox/src/content/features/ecs/actors.rs](crates/ambition_sandbox/src/content/features/ecs/actors.rs) — system that previously ticked `NpcRuntime` now ticks `Brain` via the new components
- LDtk hostile-flip path in `features::apply_save` — internal `StateMachineCfg::Patrol` → hostile sub-state transition; no longer replaces the runtime

**Save format:**
- Bump save version.
- NPC state schema changes: stores brain sub-state (peaceful/hostile + conversation flags) instead of "is this an NPC or an Enemy."

**Tests added:**
- `npc_patrol_via_brain_matches_old_behavior` — golden test against a current trace replay or hand-built waypoint case.
- `npc_goes_hostile_via_internal_transition` — strike threshold flips internal sub-state; `Brain::is_hostile()` returns true after.
- `npc_attack_gate_blocks_pre_hostile` — pre-transition, `melee_pressed` does not fire windup.

**Exit criteria:** all NPC behavior in the intro slice still works in a manual run (or headless replay). `cargo test -p ambition_sandbox --lib` green. Trace tests for NPC encounters green.

**Compatibility break:** old saves invalid. Documented in commit.

**Commit message shape:** `sandbox: migrate NpcRuntime onto Brain::StateMachine; delete bespoke NPC update path`

---

### Chunk 4 — Player onto `Brain::Player` + decompose `ae::Player`

**Goal:** player is just another actor with `Brain::Player(PlayerSlot)`. `ae::Player`'s 50+ fields move to ECS components on the player entity. `update_player` becomes a thin orchestrator that builds `BrainSnapshot`, calls `Brain::tick`, and runs the integration.

**This chunk is large.** Sub-step it as separate commits; each commit must compile and pass tests.

**Sub-steps (each its own commit):**

1. **4a — Audit `ae::Player` field reads.** Generate `dev/journals/ae-player-field-usage-2026-05-24.md` listing every read/write site and which subsystem owns it. No code change.

2. **4b — Carve out player movement state.** New components: `PlayerVelocity`, `PlayerFacing`, `PlayerGrounded`, `PlayerJumpState { coyote_remaining, buffer_remaining, holding, variable_h_left }`, `PlayerDashState`, `PlayerLedgeState`. Add the components alongside `ae::Player`; one sync system mirrors `ae::Player` → components. Readers stay on `ae::Player`. *No deletions yet.*

3. **4c — Carve out player combat/ability state.** New components: `PlayerAttackState` (overlap with the existing `ActivePlayerAttack`? reconcile), `PlayerBubbleShield`, `PlayerDodgeRoll`, `PlayerProjectileCharge`, `PlayerWaterContact`, `PlayerMorphBall`. Same overlap-then-delete pattern.

4. **4d — Write `tick_player_brain` so it builds the control frame from input.** Wire `Brain::Player` so it actually fires per tick. *But the integration is still `update_player`.* No behavior change yet; the frame is built and discarded.

5. **4e — Migrate `update_player` to consume the frame** instead of reading raw input. `update_player` reads `ActorControl(ActorControlFrame)` and the per-player components. `ae::Player` is now a *read-only mirror* maintained by a sync system in the opposite direction.

6. **4f — Flip the polarity.** Components become the source of truth; `ae::Player` is removed where unused. Each call site moves from `ae::Player` reads to component reads. One commit per readers cluster: HUD, FX, save/load, dev panel.

7. **4g — Delete `ae::Player`.** Once no reader remains, drop the type, drop `PlayerMovementAuthority`, drop `write_player_ecs_components` if it still exists.

8. **4h — Move `update_player`'s `step_kinematic` call onto the same integration enemies/bosses use.** Now the player passes through the same collision code path as every other actor.

**Tests added (and updated):**
- `player_brain_tick_translates_input_to_frame` — input frame in, control frame out, field-by-field assertion.
- `player_movement_components_round_trip` — after sub-step 4b, set components, tick, assert positions match pre-decomposition.
- Update `repro_walls`, `trace_*`, `encounter_*` tests for new component reads where they touched `ae::Player` directly.

**Exit criteria:** every existing test green at HEAD of Chunk 4. Manual smoke test (or headless replay of intro): walk, jump, dash, attack, fireball, bubble shield, dodge roll, ledge grab, morph ball, water — all work.

**Risk gate:** if 4f bogs down (broad call-site churn), STOP at last green commit. Game still runs with the dual-shape mirror in place; finishing the polarity flip is a follow-up.

**Commit message shapes:**
- `sandbox: audit ae::Player field reads (no code change)`
- `sandbox: add PlayerVelocity/PlayerFacing/PlayerGrounded/... components alongside ae::Player`
- … (one per sub-step)

---

## Validation gates between chunks

After every chunk:

```bash
~/.cargo/bin/cargo check -p ambition_engine
~/.cargo/bin/cargo check -p ambition_sandbox
~/.cargo/bin/cargo test  -p ambition_engine  --lib
~/.cargo/bin/cargo test  -p ambition_sandbox --lib
~/.cargo/bin/cargo test  -p ambition_sandbox --test repro_walls  # after Chunk 4 starts
```

If any fail, do not start the next chunk — fix or revert.

After Chunk 3 (NPC behavior):

```bash
~/.cargo/bin/cargo test -p ambition_sandbox --test trace_intro_npc_smoke   # if it exists; else manually replay an NPC encounter trace
```

After Chunk 4 (player behavior): replay an intro trace if one exists. Otherwise, document that overnight ended with components in place but no manual play verification.

## Out of scope overnight (deferred follow-ups)

- **Chunk 5 — LDtk entity-def collapse** (`NpcSpawn` + `EnemySpawn` → `ActorSpawn`). Authoring impact; needs daytime LDtk session.
- **Chunk 6 — `Brain::RlPolicy` + batched inference.** Depends on PyO3 binding from TODO.md.
- **`Brain::Remote` networking.** Out of scope.
- **`Brain::Scripted` recorded playback.** Out of scope until trace recorder unifies on frames vs inputs.
- **`StateMachineCfg::Enemy(EnemyArchetype)` collapse** into a data table (the long-deferred Step B from `character-ai-refactor.md`). Transitional cfg lives until that lands.
- **Per-`aggressiveness` value smoothing** (timid vs frenzied attackers). Boolean gate (>0 → hostile) overnight.
- **Multi-player input wiring** (OVERNIGHT-TODO #17.5). Player brain reads `PlayerInputFrame` which is already per-player; multi-player works mechanically post-Chunk 4 even though no second-player input source exists yet.

## Open design questions captured (answered post-review by Jon)

1. ~~Where does `aggressiveness` live?~~ → Inside the brain.
2. ~~Edge detection on frame vs in brain?~~ → On the frame (Chunk 1 fields are edges).
3. **Cutscene puppets:** out of scope (`Brain::Scripted` deferred). When it lands, it's a brain variant; cross-actor choreography is a separate Director layer that temporarily swaps brains.
4. **Determinism for RL:** sim-layer responsibility, not brain-layer. Brains must not read `Instant::now()` or iterate `HashMap`.
5. ~~Brain swap vs internal transition for NPC→hostile?~~ → Internal transition.
6. **Save-game shape for brains:** brains serialize sub-state; save version bumps in Chunk 3.

## Autonomous operating rules (overnight)

Captured explicitly so they don't drift mid-session:

- **Cargo path:** `~/.cargo/bin/cargo` everywhere ([[feedback-patch-discipline]]).
- **No sudo, no apt-get, ever** ([[feedback-no-sudo-apt]]). If a system lib is missing, stop and log it — don't try to install.
- **Stage explicit paths, never `git add -A`** ([[feedback-git-add-targeted]]). Working tree carries `foo/`, `tpl/`, `perf.data`, generator output that must not enter commits.
- **No binary blobs in commits** ([[feedback-no-binary-data]]).
- **Don't push to remote, don't amend, don't force-push.** All commits stay local on `main` until Jon reviews.
- **Transient filesystem errors (EMFILE/EIO): retry-then-move-on** per [[feedback-never-stop-during-long-run]]. Don't stop for them; don't `/tmp` around them.
- **Real blockers don't stop work either** — work around, document in the plan or a `dev/journals/` entry, continue. The only thing that ends the session is reaching the last green commit of the last attempted chunk.
- **Test gate before every commit:** `~/.cargo/bin/cargo check -p ambition_engine`, `~/.cargo/bin/cargo check -p ambition_sandbox`, `~/.cargo/bin/cargo test -p ambition_engine --lib`, `~/.cargo/bin/cargo test -p ambition_sandbox --lib`. If sandbox tests fail to *link* due to environment, fall back to engine tests + `cargo check -p ambition_sandbox` + headless harness invocation, and log the gap.
- **`>1hr` diagnoses get a dev/journals/ entry** per [[reference-lessons-learned]]. Title with date.
- **`Reflect` only when something needs it** — keep new components / enum variants out of the registry until a consumer demands it ([[feedback-compile-time]]).
- **Update FEATURES.md / TODO.md as items complete** per [[feedback-todo-features-discipline]]. Mark the TODO.md "Universal brain interface" entry as `[~]` while in progress; move resolved bits to FEATURES.md when chunks land.
- **Match memory format when writing notes** — `dev/journals/<topic>-<YYYY-MM-DD>.md` for incidents; `dev/benchmark-candidates/` for transferable refactor lessons.
- **Stopping checkpoints:** end of every chunk and end of every sub-step. Run the test gate, commit, move on. The session naturally ends when (a) Chunk 4's last sub-step lands, or (b) time runs out at a green checkpoint.

## Memory invariants this plan respects

- [[feedback-patch-discipline]] — `~/.cargo/bin/cargo`; verify before claiming compile-tested.
- [[feedback-compile-time]] — no Reflect on hot paths; new components in their own modules.
- [[feedback-design-balance]] — narrow specific types; add knobs only when a use case lands.
- [[feedback-pre-release-no-compat]] — single-commit replacements; save format breaks accepted.
- [[feedback-always-commit]] — commit each green sub-step.
- [[feedback-never-stop-during-long-run]] — overnight; work around blockers; if Chunk 4 sub-step bogs down, stop at last green commit, don't ask for input.
- [[feedback-bevy-testing-pattern]] — minimal-plugin App + `app.update()` + World assertions for new tests.
- [[reference-lessons-learned]] — if any change costs >1hr to diagnose, draft a `dev/journals/` entry.
- [[feedback-git-add-targeted]] — never `git add -A`; stage explicit paths.
- [[feedback-no-binary-data]] — none expected.
