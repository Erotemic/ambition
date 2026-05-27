---
title: "Silksong Comparison Review"
review_date: "2026-05-27"
source_snapshot: "ambition-source-2026-05-26T222032-5-3e93516618a5.tar.gz"
repo_path: "docs/reviews/silksong_comparison.md"
status: "code-grounded gap inventory"
---

# Silksong Comparison Review

Review date: **2026-05-27**

This note tracks Ambition's current movement/combat feel against a Silksong-style reference target. It is a repo-maintained review, not an authoritative description of Silksong internals and not a final design spec.

## Review scope

Reviewed source snapshot:

- `ambition-source-2026-05-26T222032-5-3e93516618a5.tar.gz`

Reference material:

- External movement article: <https://12gramsofcarbon.com/p/the-elegance-of-movement-in-silksong>
- User-provided transcript excerpts about movement buffering and a rich per-hit `HitInstance`-style payload.

The external references are design inspiration. Treat the Silksong items in this document as comparison targets, not as verified claims about Team Cherry's source code.

## Staleness policy for this file

- Prefer stable file paths and type/function names over exact line numbers.
- Keep the `review_date` and `source_snapshot` current whenever this file is refreshed.
- If a feature moves from "missing" to "present", update both the inventory and the open checklist.
- If implementation details become large enough to design directly, move them into a focused RFC/ADR and leave this file as the high-level comparison index.

## Status vocabulary

- **Present**: the current codebase implements this in a recognizable form.
- **Partial**: the current codebase has related behavior, but it is incomplete, narrow, fragmented, or not tuned for the target feel.
- **Missing**: no clear implementation was found in this review.
- **Unknown / audit needed**: the behavior may exist indirectly, but this review did not find enough evidence to call it present.

## Executive summary

Ambition already has a strong custom platformer foundation. The current code includes coyote time, jump and dash buffering, variable-height jump, terminal fall caps, double jump, wall cling/jump/climb, ledge grab/getup options, ledge momentum carry, directional dash, fast fall, glide, free flight, blink/precision blink, dodge roll, shield/parry, directional melee, pogo/down-attack behavior, charged/motion projectiles, explicit hostile hitbox entities, boss damageable volumes, and typed outgoing/incoming damage messages.

The biggest remaining gaps are not basic movement verbs. They are the systems that make a large move set feel forgiving and maintainable:

1. A general-purpose action buffer with explicit expiry/consume semantics.
2. A formal cancel-window matrix for attack, dash, blink, ledge, hitstun, recovery, and tool transitions.
3. Attack, pogo, projectile/tool, blink, and ledge-action buffers beyond the existing jump/dash timers.
4. Apex hang and/or held-jump sustain for a more polished jump arc.
5. Sprint/long-jump or momentum-preserving jump rules.
6. A canonical `HitSpec` / `HitInstance` / `HitResult` pipeline that unifies outgoing and incoming damage.
7. Rich per-hit metadata for raw/final damage, stagger/poise, armor, status/elemental effects, reaction, presentation, resource rewards, and rejection reasons.
8. A hurtbox/hitbox/collider audit so collision forgiveness is deliberate rather than incidental.

## Current movement inventory

| Area | Status | Current code evidence | Notes |
| --- | --- | --- | --- |
| Input snapshot | Present | `crates/ambition_sandbox/src/input/control.rs` | `ControlFrame` is rebuilt each frame from semantic actions. It captures press/hold/release edges but does not itself store gameplay queues. |
| Jump buffer | Present | `crates/ambition_engine/src/movement/tuning.rs`, `movement/player.rs`, `movement/control.rs`, `movement/simulation.rs` | `JUMP_BUFFER` fills `Player::jump_buffer_timer`; simulation consumes it for swim stroke, drop-through cancellation, wall jump, ground/coyote jump, and double jump. |
| Dash buffer | Present | `movement/tuning.rs`, `movement/player.rs`, `movement/control.rs`, `movement/simulation.rs` | `DASH_BUFFER` fills `Player::dash_buffer_timer`; dash and ground dodge consume it when legal. |
| Coyote time | Present | `movement/tuning.rs`, `movement/simulation.rs` | `COYOTE_TIME` refreshes while grounded and is consumed by jump. |
| Variable jump / short hop | Present | `movement/control.rs` | Releasing jump while rising cuts vertical velocity. This is not the same as held-jump sustain. |
| Terminal fall caps | Present | `movement/tuning.rs`, `movement/integration.rs` | Normal fall cap plus separate fast-fall and glide caps. |
| Double jump / air jump | Present | `movement/simulation.rs`, `abilities` | Uses generic jump buffer and `air_jumps_available`. No separately tuned double-jump queue was found. |
| Wall cling / wall climb / wall jump | Present | `movement/integration.rs`, `movement/simulation.rs`, `crates/ambition_engine/tests/wall_*`, `crates/ambition_sandbox/tests/repro_walls.rs` | Wall behavior has explicit state and regression/fuzz coverage. |
| Ledge grab / getup / release / roll / attack / jump | Present | `crates/ambition_engine/src/ledge_grab.rs` | The previous note's `movement/ledge_grab.rs` path is stale; ledge logic currently lives at engine root. |
| Ledge momentum carry | Present | `movement/tuning.rs`, `ledge_grab.rs` | `LedgeMomentumTuning` carries recent airborne approach momentum into quick getups. This is the closest current equivalent to the reference article's edge-spring feel. |
| Pogo / down-air bounce | Present / Partial | `movement/control.rs`, `app/world_flow.rs`, `content/features/ecs/damage.rs` | Pogo exists as a movement bounce and as attack-active contact with pogo-capable world/breakable targets. It is not buffered as its own action. |
| Directional melee | Present | `ambition_engine/src/combat.rs`, `app/world_flow.rs`, `app/sim_systems.rs` | `AttackIntent` and `AttackSpec` cover neutral/forward/back/up/down/aerial/wall-out style attacks. |
| Attack request routing | Present / Partial | `app/sim_systems.rs`, `brain/player.rs` | Melee starts from an `ActorActionMessage::Melee` request; pogo still reads the player input frame directly. There is no melee queue if an attack is already active. |
| Projectiles and motion input | Present / Partial | `ambition_engine/src/projectile/*`, `crates/ambition_sandbox/src/projectile/systems.rs`, `docs/mechanics/projectiles-and-motion-inputs.md` | Fireball charge/release and Hadouken-style motion recognition exist. Cooldown failures are not buffered; `SpawnFailure::Cooldown` is ignored after a failed spawn attempt. |
| Blink / precision blink | Present / Partial | `movement/control.rs`, `movement/blink.rs`, `docs/mechanics/blink.md` | Blink has quick/precision hold behavior and can arm from a held input after cooldown clears. This is useful but is not a formal timed buffer. |
| Fast fall | Present | `movement/integration.rs`, `movement/player.rs` | A committed `fast_falling` state exists and changes fall behavior. |
| Glide / slow fall | Present | `movement/tuning.rs`, `movement/integration.rs`, `movement/player.rs` | Held-jump glide is a traversal option beyond the Silksong reference list. |
| Free flight | Present | `movement/tuning.rs`, `movement/control.rs`, `movement/integration.rs` | Debug/ability-style flight mode exists and intentionally differs from tight platformer motion. |
| Dodge roll | Present | `movement/control.rs`, `movement/tuning.rs`, player combat state | Grounded dash buffer can become a dodge roll when the dodge ability is available. |
| Shield / parry | Present | `movement/control.rs`, `movement/tuning.rs`, player combat state | Shield hold and parry-window timers exist. Incoming damage checks parry/dodge/invulnerability state elsewhere. |
| Body modes / shape changes | Present / Partial | `crates/ambition_sandbox/src/body_mode/*`, `movement/player.rs` | Standing/crouching/crawling/morph-like body state exists. `climbable_contact` is cached but full `BodyMode::Climbing` integration is documented as follow-up in the code. |
| Movement op trace vocabulary | Present | `ambition_engine/src/movement/ops.rs`, trace/replay tests | Movement ops and replay fixtures exist, useful for pinning feel regressions. |

## Movement gaps against the reference target

### General-purpose action buffering

Status: **Partial / missing as a system**

Current code has dedicated jump and dash timers. It does not have a reusable action-buffer abstraction for actions with payloads, expiry, consume/reject reasons, or per-action tuning.

Target shape:

```rust
pub struct ActionBuffer {
    pub action: BufferedAction,
    pub remaining: f32,
    pub payload: ActionPayload,
}

pub enum BufferedAction {
    Jump,
    Dash,
    Attack,
    Pogo,
    Projectile,
    Tool,
    Blink,
    LedgeJump,
    LedgeRoll,
    HarpoonDash,
}
```

Keep jump and dash behavior stable when introducing this. The first implementation can wrap the current `jump_buffer_timer` and `dash_buffer_timer` behind a compatibility API before moving other actions onto it.

### Attack buffer

Status: **Missing**

Attack input is currently edge/request based. `start_attack` returns immediately when an attack is already active, and the request is not retained for the first legal recovery frame. This makes attack inputs easy to drop during active/recovery frames, hitstun, landing transitions, or ledge state changes.

### Pogo buffer

Status: **Missing / partial**

Pogo exists, but pogo intent is immediate. The current code handles dedicated pogo and down+attack pogo attempts, but there is no short-lived pogo intent window if the target becomes valid a frame later.

Pogo should probably be buffered separately from generic attack because it can refresh movement resources and bounce the player.

### Projectile / tool buffer

Status: **Missing / partial**

Projectile systems support charge/release, motion-input recognition, resource checks, cooldowns, and projectile trace events. However, a projectile release or motion press that hits cooldown is not retained. `SpawnFailure::Cooldown` currently has no retry/queue behavior.

If Ambition adds broader "tools", they should probably share a `Tool`/`Projectile` buffer layer rather than each inventing a cooldown edge case.

### Blink buffer

Status: **Partial**

Blink has hold-to-arm behavior: a held blink button can start aiming when cooldown clears. That solves one common missed-input case, but it does not expose the same semantics as a timed buffer with an expiry and a consumed/rejected trace.

Decision needed: keep blink as a hold action, add a timed blink buffer, or support both explicitly.

### Ledge action buffering

Status: **Missing / audit needed**

Ledge actions are rich, but the current ledge code checks inputs while already in the ledge state. This review did not find a general buffer for jump/roll/attack/climb inputs pressed just before the ledge grab becomes active.

### Apex hang

Status: **Missing**

Current jump feel is an impulse plus early-release velocity cut. This review did not find a reduced-gravity apex hang or a gravity-ramp period around the top of a jump.

Target fields could include:

```rust
pub apex_hang_velocity_threshold: f32;
pub apex_hang_gravity_scale: f32;
pub apex_hang_time: f32;
pub apex_hang_ramp_time: f32;
pub jump_cut_gravity_scale: f32;
```

### Held-jump sustain / jump steps

Status: **Missing**

The transcript describes a jump split into holdable steps. Ambition currently uses a single impulse and release cut, not an explicit hold-sustain timer or repeated upward support.

Possible target:

```rust
pub jump_sustain_timer: f32;
pub jump_sustain_max_time: f32;
pub jump_sustain_accel: f32;
```

Alternative implementation: early-jump gravity scaling rather than explicit upward acceleration.

### Sprint jump / long jump / momentum jump

Status: **Missing / design needed**

The reference article highlights sprint-jump and long-jump style options. Current Ambition movement has run acceleration and max speed, but this review did not find a separate sprint tier or jump launch that scales with prior horizontal momentum.

This may be better framed as a momentum-preserving jump than a named long-jump verb.

### Wall coyote / wall grace

Status: **Missing / audit needed**

Wall jump works when the player is still on a wall and the generic jump buffer is live. This review did not find a separate "recent wall contact" timer equivalent to ground coyote time.

### Corner correction / head-bonk forgiveness

Status: **Unknown / audit needed**

There are wall-cling/wall-jump stability tests and collision correction work, but this review did not confirm a dedicated upward corner-correction behavior that nudges the player around tile lips during head collisions.

### Harpoon dash / grapple-like traversal

Status: **Missing**

No harpoon dash, grapple pull, or target-latched dash equivalent was found. Blink/dash cover adjacent design space but do not replace a target-based traversal verb.

### Formal cancel-window matrix

Status: **Missing / partial**

Current systems have local gates, cooldowns, and state checks, but not a central cancel/carry matrix that says which actions can interrupt or queue from which phases.

Examples that should eventually be explicit:

| From state | Potential exits |
| --- | --- |
| Attack startup | none, parry, maybe dodge |
| Attack active | pogo hit, dash cancel, blink cancel |
| Attack recovery | buffered attack, jump, dash, tool |
| Dash | attack, jump, blink, pogo |
| Ledge hang | climb, roll, attack, jump, release |
| Hitstun exit | buffered jump/dash/attack if allowed |

## Current hitbox/hurtbox/collider state

| Area | Status | Current code evidence | Notes |
| --- | --- | --- | --- |
| Player movement collider | Present | `movement/player.rs` | Player uses an explicit AABB body size. Presentation can be larger than gameplay body. |
| Engine combat primitives | Present / small | `ambition_engine/src/combat.rs` | `DamageKind`, `Damage`, `Hitbox`, `Hurtbox`, and `DamageVolume` exist, but they are compact helper types, not a full hit-instance pipeline. |
| Player outgoing damage message | Present / small | `content/features/events.rs`, `content/features/ecs/damage.rs` | `DamageEvent` is the typed path for player slash/projectile/pogo-adjacent feature damage. |
| Incoming player damage message | Present / separate | `content/features/events.rs`, `app/sim_systems.rs`, `app/world_flow.rs` | `PlayerDamageEvent` handles hazards/enemy/boss/projectile damage to the player and carries target-routing fields. |
| Hostile melee hitbox entities | Present | `content/features/ecs/hitbox.rs` | Enemy/boss melee now uses explicit hitbox entities with lifetime and one-hit target sets. |
| Enemy projectile incoming damage | Present / separate | `enemy_projectile/systems.rs` | Enemy projectiles emit `PlayerDamageEvent`, separate from player outgoing projectile `DamageEvent`. |
| Boss damageable volumes | Present | `content/features/boss_attack_geometry.rs`, `content/features/ecs/damage.rs`, `brain::BossAttackState` | Boss damage checks can use dynamic damageable volumes rather than only a gross body AABB. |
| Boss encounter damage outcome | Present / partial | `boss_encounter.rs`, `content/features/ecs/damage.rs` | `record_boss_damage` returns applied/killed information; invulnerable phases can reject damage. This is useful but not a generic `HitResult`. |
| Stagger pressure | Present / coupled | `ambition_engine/src/boss_encounter.rs` | Boss stagger pressure exists but is driven by damage amount, not a separate per-hit stagger value. |
| Hurtbox vs movement collider audit | Needed | multiple paths | Confirm which damage paths use movement AABB, feature AABB, authored hurtboxes, boss animation volumes, or other shapes. |

## Current combat/damage inventory

| Piece | Status | Current code evidence | Notes |
| --- | --- | --- | --- |
| `DamageKind` | Present | `ambition_engine/src/combat.rs` | Broad source classes: slash, pogo, contact, hazard, projectile, environmental, custom. |
| `Damage` | Present / compact | `ambition_engine/src/combat.rs` | Amount, vector knockback, kind, source faction, and hitstop seconds. No raw/final split, stagger, status, presentation, or rejection metadata. |
| `AttackIntent` / `AttackSpec` | Present | `ambition_engine/src/combat.rs` | Directional melee attack specs include damage kind and `can_pogo`. |
| `DamageEvent` | Present / compact | `content/features/events.rs` | Outgoing player-side feature damage uses volume, amount, source, and ignored target keys. |
| `DamageSource` | Present / partial | `content/features/events.rs` | Distinguishes player slash, projectile kind, and pogo bounce. This is a useful seed for `HitSource`, but not enough for full hit resolution. |
| `PlayerDamageEvent` | Present / separate | `content/features/events.rs` | Incoming player damage has mode, source, source/impact positions, knockback, amount, and optional target. |
| Projectile trace events | Present / partial | `projectile/state.rs`, `projectile/systems.rs` | Projectile fire/hit/resource-block events are traced. Cooldown-blocked attempts are not currently represented as retriable buffered intents. |
| Hit rejection result | Missing / partial | scattered | Boss invulnerable phases can return an unapplied outcome, but generic hit rejection reasons are not modeled across all targets. |
| Elemental/status damage | Missing | no central fields found | `DamageKind` is not an element/status system. |
| Resource/economy rewards per hit | Missing / partial | scattered/future | There is player mana/resource infrastructure, but no per-hit reward payload equivalent to `HitReward`. |
| Presentation metadata per hit | Missing / partial | scattered | VFX/SFX/hitstop/flash are mostly emitted by consumers rather than carried with a hit payload. |
| Structured hit analytics | Missing / partial | trace systems exist | Movement/projectile traces exist; no universal hit-instance trace with rejection reasons was found. |

## Combat gaps against a HitInstance-style target

### Canonical `HitSpec` / `HitInstance` / `HitResult` pipeline

Status: **Missing / fragmented**

Current damage behavior is split across engine helper structs, `DamageEvent`, `PlayerDamageEvent`, hostile hitbox entities, projectile state, hazards, boss-specific volume checks, and boss encounter damage outcomes. These pieces work, but there is not one canonical per-hit value that every source produces and every target consumes.

Recommended distinction:

```rust
/// Authored by attacks, projectiles, tools, hazards, and contact damage.
pub struct HitSpec {
    pub source: HitSource,
    pub volume: HitVolume,
    pub damage: HitDamage,
    pub reaction: HitReaction,
    pub flags: HitFlags,
    pub presentation: HitPresentation,
    pub ignored_targets: Vec<HitTargetKey>,
}

/// Created once per resolved source-target overlap.
pub struct HitInstance {
    pub spec: HitSpec,
    pub target: HitTarget,
    pub target_hurtbox: Option<HitboxPartId>,
    pub impact_pos: ae::Vec2,
    pub normal: ae::Vec2,
    pub final_damage: i32,
}

pub enum HitResult {
    Applied(HitApplied),
    Rejected(HitRejectReason),
}
```

Why this matters: one slash, projectile, hazard, boss attack, or tool can overlap multiple targets. Each target may have different faction, invulnerability, armor, boss phase, shield/parry state, hurtbox part, stagger resistance, elemental resistance, reward policy, and presentation rules.

### Raw damage vs final damage

Status: **Missing / partial**

Current messages usually carry a single damage number. Some scaling/rejection happens later, such as player incoming invulnerability or boss encounter outcomes, but there is no central raw/scaled/final breakdown.

Target:

```rust
pub struct HitDamage {
    pub raw_health: i32,
    pub final_health: i32,
    pub stagger: i32,
    pub poise: i32,
    pub kind: DamageKind,
    pub elements: ElementMask,
}
```

### Separate stagger / poise / armor damage

Status: **Missing / partial**

Boss stagger pressure exists, but it is currently based on ordinary damage accumulation. A richer hit payload should let an attack do high stagger but low health damage, or break armor without being the highest DPS move.

### Elemental/status damage metadata

Status: **Missing**

`DamageKind` is a broad source classification. It does not represent elemental or status tags such as fire, poison, silk, shock, bleed, curse, armor break, etc.

Only add these once design actually needs them. The important architectural point is to reserve a place for them in the hit payload instead of scattering special cases through consumers.

### Reaction metadata

Status: **Missing / partial**

Knockback and recoil exist, but in different shapes:

- Engine `Damage` has vector knockback.
- `PlayerDamageEvent` has direction and strength.
- `DamageSource::PlayerSlash` carries horizontal knockback.
- Hostile ECS `Hitbox` carries `knockback_strength`.
- Attack specs carry self impulse, knockback, and `can_pogo`.

Target:

```rust
pub struct HitReaction {
    pub knockback: ae::Vec2,
    pub hitstop_seconds: f32,
    pub hitstun_seconds: f32,
    pub attacker_recoil: ae::Vec2,
    pub pogo: Option<PogoReaction>,
    pub launch: Option<LaunchReaction>,
}
```

### Presentation metadata

Status: **Missing / partial**

VFX, SFX, hit flash, debris bursts, camera shake, and hitstop are mostly emitted by the consumers that happen to apply damage. Engine `Damage` has `hitstop_seconds`, but the sandbox's main damage paths do not yet share one presentation payload.

Target:

```rust
pub struct HitPresentation {
    pub spark_kind: HitSparkKind,
    pub sfx: Option<SoundId>,
    pub camera_shake: Option<CameraShakeSpec>,
    pub flash_seconds: f32,
    pub freeze_seconds: f32,
}
```

### Hit rejection reasons

Status: **Missing / partial**

Generic hit resolution should be able to say why a hit failed or changed shape: wrong faction, invulnerable, already hit by this source, shielded, parried, armor absorbed, boss phase immune, hurtbox disabled, etc.

Target:

```rust
pub enum HitRejectReason {
    FactionRejected,
    Invulnerable,
    AlreadyHitByThisSource,
    Shielded,
    Parried,
    ArmorAbsorbed,
    BossPhaseImmune,
    HurtboxDisabled,
}
```

### Resource and economy metadata

Status: **Missing / partial**

The transcript mentions fields like currency knockback multiplier. Ambition does not need that exact field now, but it should eventually have a per-hit reward/recovery payload for energy gain, combo gain, dash refresh, air-jump refresh, currency modifiers, or quest/flag hooks.

Target:

```rust
pub struct HitReward {
    pub energy_gain: i32,
    pub currency_multiplier: f32,
    pub combo_meter_gain: f32,
    pub refresh_dash: bool,
    pub refresh_air_jump: bool,
}
```

## Recommended implementation order

### Phase 1: Pin current behavior

- Add or extend tests for current jump buffer, dash buffer, coyote time, variable jump, wall jump, double jump, ledge momentum carry, pogo, blink hold behavior, fast fall, glide, dodge, shield/parry, and projectile cooldown behavior.
- Add explicit tests proving currently missing buffers: attack during recovery, projectile release during cooldown, pogo one frame before contact, and ledge action one frame before grab.
- Add damage-path tests for player slash to enemy, projectile to enemy, hostile hitbox to player, enemy projectile to player, hazard to player, boss damageable volume, boss invulnerable phase rejection, and pogo breakable bounce.

### Phase 2: Introduce action buffering without changing feel

- Add an `ActionBuffer` type with `fill`, `tick`, `consume`, `expire`, and trace/debug hooks.
- Wrap the existing jump/dash timers through compatibility methods first.
- Trace `buffer_filled`, `buffer_consumed`, `buffer_expired`, and `buffer_rejected` events so missed-input bugs become inspectable.

### Phase 3: Buffer more actions

- Add attack buffer for cooldown/recovery/landing/hitstun-exit forgiveness.
- Add pogo buffer that preserves pogo/down-air intent separately from generic melee.
- Add projectile/tool buffer for cooldown-edge and release-edge forgiveness.
- Decide whether blink keeps hold-arm only or also gets a formal timed buffer.
- Add ledge action buffers if playtesting shows missed ledge climb/jump/roll/attack inputs.

### Phase 4: Polish jump and traversal feel

- Add apex hang or gravity ramp near jump peak.
- Add held-jump sustain or early-jump gravity scaling.
- Add wall coyote if tests/playtesting show missed wall jumps.
- Audit corner/head-bonk correction.
- Add momentum-preserving jump / sprint-jump / long-jump behavior only after base jump feel is stable.

### Phase 5: Introduce a hit-instance pipeline incrementally

- Add engine-level `HitSpec`, `HitInstance`, `HitResult`, `HitRejectReason`, `HitDamage`, `HitReaction`, and `HitPresentation` types.
- Convert player slash and player projectile producers to emit `HitSpec`, then adapt into existing `DamageEvent` behavior to avoid a flag-day refactor.
- Resolve overlaps into per-target `HitInstance` values.
- Port hostile hitboxes, enemy projectiles, hazards, and boss attack volumes into the same flow.
- Move boss stagger, VFX/SFX, hitstop, shield/parry, invulnerability, and reward logic into structured hit resolution over time.

## Prioritized open checklist

Movement/control:

- [ ] Add `ActionBuffer` design note or RFC.
- [ ] Wrap existing jump/dash buffers with a common buffer API.
- [ ] Add attack buffer.
- [ ] Add pogo buffer.
- [ ] Add projectile/tool buffer.
- [ ] Decide blink buffer vs hold-arm semantics.
- [ ] Add ledge action buffering if playtesting confirms missed inputs.
- [ ] Add apex hang tuning.
- [ ] Add held-jump sustain or early-jump gravity scaling.
- [ ] Add wall coyote timer if needed.
- [ ] Audit corner correction/head-bonk forgiveness.
- [ ] Add momentum-preserving jump / sprint-jump / long-jump rules if desired.

Combat/hits:

- [ ] Add `HitSpec` / `HitInstance` / `HitResult` design note or RFC.
- [ ] Add raw/scaled/final/stagger/poise damage fields.
- [ ] Add unified hit reaction metadata.
- [ ] Add hit presentation metadata.
- [ ] Add hit rejection reasons and hit trace events.
- [ ] Decide how current `DamageEvent` and `PlayerDamageEvent` adapt to/from `HitSpec`.
- [ ] Audit which damage paths use movement AABB, authored feature AABB, boss dynamic damageable volumes, or animation-derived hurtboxes.
- [ ] Move boss stagger pressure from ordinary damage amount to explicit stagger metadata when the new hit payload exists.

Testing/docs:

- [ ] Add tests for all currently-present movement assists listed in this document.
- [ ] Add tests proving currently-missing buffers before implementing them.
- [ ] Add a small combat-hit matrix test suite covering applied and rejected hits.
- [ ] Refresh this file after each major movement/combat refactor.

## Notes for future reviews

- The repo has already moved some code paths compared with earlier notes; for example, ledge logic is currently `crates/ambition_engine/src/ledge_grab.rs`, not `crates/ambition_engine/src/movement/ledge_grab.rs`.
- The current source already has more combat structure than a minimal prototype: explicit hostile hitbox entities, player outgoing `DamageEvent`, player incoming `PlayerDamageEvent`, projectile traces, boss damageable volumes, and boss encounter damage outcomes. Do not describe combat as "no damage system"; describe it as fragmented and not yet unified around a per-hit instance.
- Avoid exact line numbers unless linking to a stable commit. Function/type names and file paths are less likely to make the document stale.
