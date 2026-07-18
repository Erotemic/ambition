# `brain/` — the universal brain seam

> **Read this before touching enemy AI.** This is the navigability map for
> `crate::brain` (~8.7k LOC / 12 files). It tells you *where* each kind of
> behavior lives, *how* a decision flows from perception to the simulation,
> and *what* the key types are. The per-file `//!` docs are authoritative for
> details; this file is the index that points you at the right one.

Companion docs: `docs/systems/actors-brains-and-character-content.md` (overview),
`docs/recipes/extending-brains-and-action-sets.md` (extension recipe).

---

## The one idea: brains are *policy*, ActionSets are *capability*

Every controllable actor in the sandbox — player, NPC, enemy, boss, and
(future) RL agent — carries the same three sibling components:

| Component | Role | Defined in |
|---|---|---|
| `Brain` | the *policy*: decides intent each tick | `mod.rs` |
| `ActionSet` | the *capability*: what this body can actually do | `action_set/mod.rs` |
| `ActorControl(ActorControlFrame)` | last-tick *intent* sink (the brain writes it; the sim reads it) | `mod.rs` (frame type lives in `crate::actor::control`) |

The same `Brain::StateMachine(MeleeBrute(..))` on two different enemies can
look completely different, because the brain only emits *abstract* intent
(`frame.melee_pressed = true`) and each actor's `ActionSet` resolves that into a
*concrete* effect (Swipe vs Lunge vs Bite). A player possessing a goblin keeps
the goblin's ActionSet — "Attack" still resolves to the goblin's leap. This is
the possession / multi-body invariant.

---

## Data flow (one tick)

```
        ┌─────────────────┐   snapshot (read-only world view)
        │  BrainSnapshot  │◄── built per-tick from the actor's ECS components
        └────────┬────────┘    (pos/vel/facing/ground, combat timers, target)
                 │  snapshot.rs
                 ▼
        ┌─────────────────┐   Brain::tick(_with_actions)
        │      Brain      │   match on backend:
        │   (the policy)  │     • Player(slot)      → player.rs
        └────────┬────────┘     • StateMachine(cfg) → state_machine/mod.rs
                 │              dispatch (state_machine/mod.rs) fans out to:
                 │                Patrol/Wanderer/MeleeBrute/Skirmisher/
                 │                Sniper/Shark/StandStill   → state_machine/mod.rs
                 │                BossPattern                → boss_pattern.rs
                 │                Smash                      → smash/ (5-stage)
                 ▼  writes ABSTRACT intent
        ┌─────────────────────────────┐
        │  ActorControlFrame (intent) │  melee_pressed, fire=Some(dir),
        │  stored in ActorControl     │  desired_vel, facing, jump, special…
        └────────┬────────────────────┘
                 │  action_set::resolve(actions, frame, pos)   (emit.rs/mod.rs)
                 ▼  resolves intent → CONCRETE requests
        ┌─────────────────────────────┐
        │  ActorActionMessage stream  │  one per ActionRequest
        │  (request: Melee/Ranged/    │  emitted by emit_brain_action_messages
        │   Special/PlayerProjectile) │  (mod.rs)
        └────────┬────────────────────┘
                 ▼  consumed OUTSIDE brain/ by the sim ("EFFECTS" stage):
        features::ecs::brain_effects        (specials, GNU-ton, sentinel)
        features::ecs::spawn_enemy_projectiles_from_brain_actions (ranged)
        runtime melee windup → active hitbox edge    (melee)
        crate::projectile::update_projectiles        (player projectile tick)
```

The brain half is **pure-ish and deterministic**: same brain + same snapshot →
same frame (modulo the brain's own internal state mutation). That is what lets
`replay_fixture_regression` and future RL training rely on reproducibility — and
why this run must not change behavior.

### Scheduling (lives in `app/plugins.rs`, not here)

`BrainPlugin` (in `mod.rs`) only registers the `ActorActionMessage` channel +
`BrainActionCounter` resource. The per-tick *systems* are scheduled explicitly
in `app/plugins.rs` because they must chain after sandbox input systems:
`tick_player_brains` → (runtime actor tick fills enemy/boss frames) →
`emit_brain_action_messages` → `emit_player_projectile_tick_messages` →
`observe_brain_action_counter`.

---

## Where each kind of AI lives

| Actor kind | Brain backend | Behavior code | Spawned/wired from |
|---|---|---|---|
| **Player** | `Brain::Player(slot)` | `player.rs` — *pure translation* of `PlayerInputFrame` → frame; makes **no** gameplay decisions | player spawn |
| **NPC (peaceful)** | `StateMachine(Patrol{aggressiveness:0})` / `StandStill` | `state_machine/mod.rs` (`tick_patrol`, `tick_stand_still`) | `features/npcs.rs` |
| **Enemy (common)** | `StateMachine(MeleeBrute/Skirmisher/Sniper/Shark/Wanderer)` | `state_machine/mod.rs` (`tick_*` per template) | `features/ecs/brain_builders.rs` (`enemy_default_brain`, archetype-driven from `character_archetypes.ron`) |
| **Enemy (brawler)** | `StateMachine(Smash{..})` | `smash/` 5-stage pipeline | `brain_builders.rs` (`smash_cfg_for_archetype`) |
| **Boss** | `StateMachine(BossPattern{..})` | `boss_pattern.rs` | `features/ecs/spawn_actors.rs`, `bosses.rs` |

> **To improve a specific enemy's behavior**, start at its template's `tick_*`
> function in `state_machine/mod.rs` (or the `smash/` stage for brawlers), and at
> its archetype row in `character_archetypes.ron` + `brain_builders.rs` for tuning.
> The *capability* (which attacks exist) is the `ActionSet` built in
> `brain_builders.rs`; the *policy* (when to use them) is the `tick_*`.

---

## File-by-file (the 12 files)

### Seam / shared
- **`mod.rs`** (1032) — the `Brain` enum + dispatch (`tick`, `tick_with_actions`),
  `ActorControl` component, `BrainPlugin`, the `ActorActionMessage` channel and
  its emitter/observer systems (`emit_brain_action_messages`,
  `emit_player_projectile_tick_messages`, `observe_brain_action_counter`),
  and `BrainActionCounter`. **The public surface** (re-exports at the top).
- **`snapshot.rs`** (234) — `BrainSnapshot`, the read-only per-tick world view
  every backend consumes; `WallContact`. Fields grouped by who fills them
  (actor-self / combat-timers / target / per-template options). Add a field
  here only when a real template consumes it.
- **`action_set/mod.rs`** (1513) — *capability*. `ActionSet` component + every attack
  spec (`SwipeSpec`, `LungeSpec`, `SlamSpec`, `BiteSpec`, `PunchSpec`,
  `RangedActionSpec`, `SpecialActionSpec`, `MoveStyleSpec`, `HeldItemSpec`), the
  `ActionRequest` enum, and `resolve()` — the intent→concrete translation.

### Brain backends
- **`player.rs`** (317) — player input → frame translation. No decisions.
- **`state_machine/mod.rs`** (1680) — the closed set of reusable AI policy
  templates: `StateMachineCfg` enum + `tick_state_machine[_with_actions]`
  dispatch + one `tick_*` fn and a `Cfg`/`State` pair per template
  (StandStill, Patrol, Wanderer, MeleeBrute, Skirmisher, Sniper, Shark; plus the
  thin `BossPattern`/`Smash` forwarders). **This is the main enemy-AI file.**
- **`boss_pattern.rs`** (2169) — boss policy: scripted multi-phase timelines
  (`BossPattern`/`BossPatternStep`) + legacy `Cycle` rhythm. Owns
  `BossMovementProfile`, `BossAttackProfile`, `BossPatternCfg`/`State`/`Context`,
  `BossAttackState`, and `tick_boss_pattern`. `BrainSnapshot`-free by design
  (the boss tick system feeds encounter phase via `BossPatternContext`).

### Smash brawler pipeline (`smash/`)
A 5-stage pure pipeline (each stage a pure fn of the previous output + cfg/state),
so any stage is independently testable and RL-swappable:
- **`smash/mod.rs`** (514) — `SmashCfg`, `SmashState`, `tick_smash` (drives the pipeline).
- **`smash/observation.rs`** (220) — *stage 1*: world → `ObservationFrame`
  (`CrowdingSignal`, `TerrainAwareness`).
- **`smash/mode.rs`** (241) — *stage 2*: `BroadMode` choice with hysteresis
  (Approach/Retreat/Engage/Reposition/Recover/Idle).
- **`smash/action.rs`** (356) — *stage 3*: `SpecificAction` choice, gated by the
  `ActionSet` capability mask.
- **`smash/difficulty.rs`** (247) — *stage 4*: `DifficultyProfile` filter
  (reaction delay, commit probability, aim accuracy).
- **`smash/emit.rs`** (203) — *stage 5*: `SpecificAction` → `ActorControlFrame`.
  The only smash stage that knows the frame schema.

---

## Glossary of key types

- **`Brain`** — enum policy backend (`Player` | `StateMachine(cfg)`). Dispatched
  by `match`, not trait objects, for a single-switch per-tick cost. New backends
  (Remote/Scripted/RlPolicy) extend the enum.
- **`StateMachineCfg`** — closed enum of AI templates; each variant bundles its
  `Cfg` (tuning) + `State` (per-actor runtime) so they can't be mismatched.
- **`BrainSnapshot`** — immutable per-tick input to a brain.
- **`ActorControlFrame`** (in `crate::actor::control`) — the abstract-intent
  output; the *only* thing a brain writes.
- **`ActionSet`** — per-actor capability; `resolve()` turns frame intent into
  `ActionRequest`s.
- **`ActionRequest` / `ActorActionMessage`** — concrete per-tick action requests
  the sim's EFFECTS stage consumes (Melee/Ranged/Special/PlayerProjectileTick).

---

## Enemy-AI improvement ideas

These are **teed up for the owner**, not implemented here — each changes
behavior/feel, which needs a human watching (and would break the zero-behavior-
change contract of the structural run). Each note says where it would plug in.

1. **Flanking / lateral steering for ground enemies.** Today `MeleeBrute` /
   `Patrol` chase is purely `direction_x` toward the target (1-D close).
   Add an approach-angle offset so a pack doesn't stack on one axis.
   *Plug in:* a new `Chase`-intent branch in `state_machine/mod.rs` (`tick_melee_brute`),
   driven by a new `flank_bias` field on `MeleeBruteCfg`; for the brawler path the
   natural home is `smash/mode.rs` `Reposition` mode + `smash/action.rs`.

2. **Group/aggro coordination (don't all commit at once).** The `smash/`
   pipeline already snapshots a `CrowdingSignal` (`smash/observation.rs`) but it
   only nudges spacing. Use it to ration *attack commits* so a crowd telegraphs
   in turns instead of a simultaneous swarm. *Plug in:* gate `Engage` in
   `smash/mode.rs` / commit in `smash/difficulty.rs` on a crowd-derived token;
   for state-machine enemies, add a shared "attack budget" the `tick_*` reads.

3. **Richer windup telegraphs.** Telegraph timing is already owned per-attack by
   the `*Spec` windup→active→recover (`action_set/mod.rs`) — no brain change needed
   to *lengthen* a telegraph (data-only via `character_archetypes.ron`). For a
   *behavioral* telegraph (back-step before a lunge, flash, audio cue), add a
   pre-strike beat: for bosses that's a `BossPatternStep::Telegraph` already
   (`boss_pattern.rs`); for common enemies add a `Windup` micro-state to the
   template's `State` in `state_machine/mod.rs`.

4. **Reaction-delay & feinting for difficulty.** `DifficultyProfile`
   (`smash/difficulty.rs`) already carries `reaction_delay_s` and
   `commit_probability`. Wire `reaction_delay_s` into the observation stage as a
   delayed target sample (it's currently informational), and add an occasional
   "feint" (start a telegraph, abort) via a low-probability branch in
   `smash/action.rs`. State-machine enemies could borrow the same idea by
   sampling a stale `target_pos` in `BrainSnapshot`.

5. **Terrain-aware retreat / ledge awareness.** `TerrainAwareness`
   (`smash/observation.rs`) exists; bias `Retreat`/`Reposition` away from ledges
   and toward cover. *Plug in:* `smash/mode.rs` mode scoring. For state-machine
   enemies, surface a `near_ledge` flag on `BrainSnapshot` (only set it when a
   template consumes it — see the snapshot.rs "add by name" rule) and read it in
   `tick_wanderer` / `tick_melee_brute`.

6. **Skirmisher kiting (maintain a preferred range band).** `Skirmisher`/`Sniper`
   currently fire-and-hold. Add a min/max range band: back-pedal when the target
   closes inside `min`, advance when outside `max`. *Plug in:* `SkirmisherCfg`
   gets `preferred_min`/`preferred_max`; `tick_skirmisher` in `state_machine/mod.rs`
   chooses retreat vs advance vs fire from the band.

7. **Aggro/threat memory (don't instantly forget the player).** Chase currently
   re-evaluates from the live snapshot each tick, so breaking line-of-sight drops
   aggro immediately. Add a decaying `aggro_timer` to the template `State`
   (`state_machine/mod.rs`) so enemies keep pursuing for a short window after losing
   the target. Pairs well with idea 4's stale-sample mechanism.

8. **Boss phase reactions to player state.** `BossPatternContext` (`boss_pattern.rs`)
   already threads `encounter phase` + target pos. Extend the context with a
   couple of player-state reads (e.g. is-charging, low-health) so a boss can pick
   a counter-pattern. *Plug in:* widen `BossPatternContext`, branch in
   `tick_boss_pattern` — keep it boss-specific (do NOT bloat `BrainSnapshot`,
   per the existing design note).

When implementing any of these: add the *capability* (new `*Spec` / `ActionSet`
field) in `action_set/mod.rs`, the *policy* in the relevant `tick_*` / smash stage,
and the *tuning* in `character_archetypes.ron` + `brain_builders.rs`. Verify against
`scripted_gameplay` + `replay_fixture_regression` (a behavior change will and
should move those — regenerate fixtures intentionally, with the owner watching).
