---
title: "Silksong Comparison Review"
review_date: "2026-05-26"
source_archive: "ambition-source-2026-05-26T222032-5-3e93516618a5.tar.gz"
repo_path: "docs/reviews/silksong_comparison.md"
status: "working review / gap inventory"
---

NOTE: We do not need to have everything in silksong, this is just a comparison, although we likely do want the things that make it feel good like more input buffering.

# Silksong Comparison Review

Review date: 2026-05-26

This document tracks a comparison between Ambition's current movement/combat feel systems and a Silksong-style reference target. It is intended to be a living repo note, not a final design spec.

## Sources reviewed

- Current source archive: `ambition-source-2026-05-26T222032-5-3e93516618a5.tar.gz`.
- External movement article: <https://12gramsofcarbon.com/p/the-elegance-of-movement-in-silksong>, published 2025-09-08.
- User-provided video transcript excerpts covering movement buffering and a large per-hit `HitInstance` style object.

Caveat: the article and transcript are reference material, not authoritative source-code documentation for Silksong. Treat the Silksong items below as design targets inspired by those analyses, not as exact implementation claims.

## Status vocabulary

- Present: the current codebase already implements this in a recognizable form.
- Partial: the current codebase has related behavior, but it is incomplete, narrow, or split across systems.
- Missing: no clear implementation found in this review.
- Unknown: not enough evidence found during this review.

## Executive summary

Ambition already has a strong custom platformer foundation: coyote time, jump and dash buffering, variable-height jump, terminal fall clamping, double jump, wall/ledge verbs, dash, glide, fast fall, blink, dodge, shield/parry, pogo-like bounce, and animation-aware boss hurtboxes.

The largest gaps versus the Silksong-style target are:

1. A general-purpose action input buffer and cancel-window system.
2. Attack/tool/projectile/blink buffers beyond the existing jump/dash timers.
3. Apex hang and held-jump sustain for a more polished Mario/Silksong jump arc.
4. Sprint/long-jump momentum rules.
5. A canonical `HitSpec` / `HitInstance` / `HitResult` combat pipeline.
6. Rich per-hit metadata for stagger, poise, armor, elemental/status effects, VFX/SFX/camera policy, resource rewards, and rejection reasons.

## Movement reference target

The movement article emphasizes that Silksong gives many ways to cross even a simple platform gap: double jump, long jump, jump plus dash, sprint jump, short jump into wall grab/jump, edge spring, dash back, and downward attack to land faster. The transcript adds the implementation-feel target: coyote time, input buffering, variable jump height, jump steps/held sustain, apex hang, terminal fall clamping, forgiving hitboxes/hurtboxes, and multiple action queues.

## Current movement inventory

| Feature | Status | Current evidence | Notes |
| --- | --- | --- | --- |
| Coyote time | Present | `crates/ambition_engine/src/movement/tuning.rs:56`, `simulation.rs:80-102` | `COYOTE_TIME = 0.120`; refreshed on ground. |
| Jump buffer | Present | `tuning.rs:57`, `control.rs:57`, `simulation.rs:80`, `simulation.rs:106-169` | `JUMP_BUFFER = 0.135`; consumed by swim stroke, wall jump, ground/coyote jump, and double jump. |
| Dash buffer | Present | `tuning.rs:22`, `control.rs:60`, `control.rs:257-274` | `DASH_BUFFER = 0.100`; also used by dodge-roll path when grounded. |
| Variable jump / short hop | Present | `movement/control.rs:202-207` | Early release cuts upward velocity by multiplying it by `0.54` when rising fast enough. |
| Terminal fall clamp | Present | `movement/tuning.rs:12`, `movement/integration.rs:95-99` | Normal fall cap plus separate fast-fall and glide caps. |
| Double jump | Present | `movement/simulation.rs:161-169` | Uses generic jump buffer, not a separate double-jump queue. |
| Wall jump | Present | `movement/simulation.rs:145-153` | Triggered from generic jump buffer while on wall and airborne. |
| Wall cling / wall climb | Present | `movement/integration.rs`, wall tests in `crates/ambition_engine/tests/wall_jump_fuzz.rs` and `crates/ambition_sandbox/tests/repro_walls.rs` | There is dedicated wall-state handling and regression coverage. |
| Ledge grab / getup options | Present | `movement/ledge_grab.rs`, `movement/tuning.rs:77-115` | Includes ledge momentum carry tuning. |
| Ledge momentum carry / boost | Present | `movement/tuning.rs:77-115` | This partially covers the article's "edge spring" feel, but exact spring-off semantics should be reviewed in playtesting. |
| Pogo / downward aerial bounce | Partial / Present | `movement/control.rs:153-198`; combat bounce logic also exists in sandbox world flow | Current pogo is target-dependent and available as dedicated input or down+attack while airborne. |
| Glide / slow fall | Present | `movement/tuning.rs:42-49`, `movement/integration.rs:54-99` | Extra traversal option beyond the reference list. |
| Fast fall | Present | `movement/integration.rs:45-50`, `movement/integration.rs:95-96` | Enables faster downward control. |
| Blink / precision blink | Present | `movement/control.rs:82-148` | Has hold-to-aim behavior and cooldown handling. |
| Dodge roll | Present | `movement/control.rs:210-232` | Uses dash buffer when grounded and dodge is available. |
| Shield / parry window | Present | `movement/control.rs:235-255`, `movement/tuning.rs:70-74` | Movement/combat defensive verb exists. |
| Crouch / alternate body modes | Present / Partial | `crates/ambition_sandbox/src/body_mode/*` | Relevant to collision forgiveness and traversal feel. |
| Input action edge capture | Present | `crates/ambition_sandbox/src/input/control.rs:163-181` | Inputs are mostly read as `just_pressed`, `pressed`, or `just_released`. |
| Stateless per-frame control frame | Present | `input/control.rs:240-242` | Persistent queues are not stored in `ControlFrame`; they belong in simulation/player state. |

## Movement gaps

### 1. General-purpose action input buffering

Status: Missing / Partial.

The code has dedicated timers for jump and dash (`jump_buffer_timer`, `dash_buffer_timer`), but not a generalized action queue. This will become harder to maintain as more Silksong-like verbs are added.

Current evidence:

- `JUMP_BUFFER = 0.135` and `DASH_BUFFER = 0.100` in `movement/tuning.rs`.
- `jump_buffer_timer` and `dash_buffer_timer` are decremented in `movement/simulation.rs:80-81`.
- Jump and dash are filled from `input.jump_pressed` and `input.dash_pressed` in `movement/control.rs:56-60`.

Missing target:

```rust
pub struct ActionBuffer {
    pub action: BufferedAction,
    pub remaining: f32,
    pub payload: ActionPayload,
}

pub enum BufferedAction {
    Jump,
    DoubleJump,
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

Design note: keep jump and dash behavior stable while introducing this. A first migration can wrap the existing timers behind a small buffer API instead of changing all semantics at once.

### 2. Attack buffer

Status: Missing.

Attack input is currently edge-based. `ControlFrame` reads `attack_pressed` using `actions.just_pressed`, the player brain maps it to `out.melee_pressed`, and movement attack handling checks `input.attack_pressed` for that frame. There is no `attack_buffer_timer` equivalent.

Current evidence:

- `attack_pressed: actions.just_pressed(&SandboxAction::Attack)` in `input/control.rs:173`.
- `out.melee_pressed = c.attack_pressed` in `brain/player.rs:96`.
- `handle_attacks` branches directly on `input.attack_pressed` in `movement/control.rs:184`.

Target: buffer attack presses during cooldown, recovery, landing, ledge hang transitions, and hitstun exit where appropriate.

### 3. Pogo buffer

Status: Missing / Partial.

Pogo exists, but the input is immediate. Dedicated `pogo_pressed` and down+attack pogo attempts can be missed if the valid target state arrives a frame later.

Current evidence:

- `pogo_pressed: actions.just_pressed(&SandboxAction::Pogo)` in `input/control.rs:174`.
- Pogo resolves immediately through `try_pogo` in `movement/control.rs:174-190`.

Target: preserve pogo intent separately from generic attack intent, because down-attack and pogo can have different movement consequences.

### 4. Projectile / tool buffer

Status: Missing / Partial.

Projectile charge and motion-input recognition exist, but firing/release is not a general buffered action. If cooldown or animation state rejects the release, the intent can be dropped.

Current evidence:

- Projectile inputs are read as `projectile_pressed`, `projectile_held`, and `projectile_released` in `input/control.rs:179-181`.
- Player brain emits a fire request only on `projectile_released` in `brain/player.rs:103-114`.
- `crates/ambition_sandbox/src/projectile.rs` notes that projectile damage routes through `DamageEvent` and mentions `MotionInputBuffer`, but this is not a general action buffer.

Target: add `projectile_buffer` or a more general `tool_buffer` for cooldown/animation-edge forgiveness.

### 5. Blink buffer

Status: Partial.

Blink has a useful held-input special case: holding blink can arm as soon as cooldown clears. This solves one second-blink failure mode, but it is not the same as a formal timed buffer with rejection/expiry semantics.

Current evidence:

- `handle_blink` allows `input.blink_pressed || (input.blink_held && !player.blink_hold_active)` when cooldown is clear in `movement/control.rs:94-99`.

Target: explicitly model blink as either a hold-to-aim action, a buffered action, or both. Avoid hiding queue semantics inside hold-state logic.

### 6. Ledge action buffering

Status: Missing / Unknown.

Ledge grab and getup are substantial, but this review did not find a general buffer for climb/jump/roll/attack commands entered just before ledge state becomes active.

Target: buffer ledge actions for short windows so near-ledge input feels reliable.

### 7. Apex hang

Status: Missing.

The current jump has initial impulse and early-release cut, but no true reduced-gravity apex hang. Glide and blink grace are separate mechanics and should not be treated as apex hang.

Current evidence:

- Jump impulse is applied in `movement/simulation.rs:154-160`.
- Early release clips velocity in `movement/control.rs:202-207`.
- No `apex`, `hang gravity`, or similar player/tuning fields were found for the player movement path.

Target fields:

```rust
pub apex_hang_velocity_threshold: f32;
pub apex_hang_gravity_scale: f32;
pub apex_hang_time: f32;
pub apex_hang_ramp_time: f32;
pub jump_cut_gravity_scale: f32;
```

### 8. Held-jump sustain / jump steps

Status: Missing.

The transcript describes jump sustain over discrete steps while the button is held. Ambition currently uses a single jump impulse plus early-release velocity cut.

Target fields:

```rust
pub jump_sustain_timer: f32;
pub jump_sustain_max_time: f32;
pub jump_sustain_accel: f32;
```

Potential alternative: implement this as gravity scaling during the first part of the jump rather than explicit upward acceleration.

### 9. Sprint jump / long jump

Status: Missing.

The article's option list includes sprint-jump and long jump. Current movement has horizontal acceleration and max run speed, but this review did not find a separate sprint tier, sprint state, or momentum-scaled jump launch.

Target fields:

```rust
pub sprint_held: bool;
pub sprint_speed: f32;
pub sprint_accel: f32;
pub long_jump_min_speed: f32;
pub long_jump_x_boost: f32;
pub long_jump_y_scale: f32;
```

Design note: this may be better as "momentum-preserving jump" rather than a separate named long-jump move.

### 10. Wall coyote / wall grace

Status: Missing / Unknown.

Generic jump buffering covers wall jump if the player is still on a wall, but this review did not find a dedicated "recent wall contact" grace timer equivalent to ground coyote time.

Target: add a short wall-contact grace window so wall jumps remain reliable when the player loses contact by a few pixels or a few milliseconds.

### 11. Corner correction / head-bonk forgiveness

Status: Unknown.

This review did not confirm a head-bonk correction system that nudges the player around tile corners during upward movement.

Target: add or audit small horizontal correction during upward collision, especially near ledges/platform lips.

### 12. Harpoon dash / grapple-like traversal verb

Status: Missing.

Blink and dash cover nearby design space, but this review did not find a harpoon dash, grapple pull, or target-latched dash mechanic.

Target components:

```rust
pub harpoon_pressed: bool;
pub harpoon_buffer_timer: f32;
pub harpoon_target_probe: HarpoonProbe;
pub harpoon_pull_state: HarpoonPullState;
```

### 13. Formal cancel-window matrix

Status: Missing / Partial.

Current systems contain many action gates, but not a central cancel matrix for movement/combat transitions. This becomes important once attack, tool, dash, blink, ledge, hitstun, and recovery states all need buffering.

Target examples:

| From state | Potential cancels |
| --- | --- |
| Attack startup | none, parry, maybe dodge |
| Attack active | pogo hit, dash cancel, blink cancel |
| Attack recovery | buffered attack, jump, dash, tool |
| Dash | attack, jump, blink, pogo |
| Ledge hang | climb, roll, attack, jump, release |
| Hitstun exit | buffered jump/dash/attack depending on state |

## Hitbox/hurtbox and collision fairness inventory

| Feature | Status | Current evidence | Notes |
| --- | --- | --- | --- |
| Smaller gameplay body than presentation | Present | `movement/player.rs:11-19` | The default player movement AABB is explicit. Presentation may render larger art. |
| Engine-level hurtbox abstraction | Present | `ambition_engine/src/combat.rs:78-100` | `Hurtbox` exists in the engine helper layer. |
| Boss per-animation hurtboxes | Present | `boss_attack_geometry.rs:296-355`, `bosses.rs:625-652`, `character_sprites/registry.rs:144-156` | Boss hurtboxes can follow current animation pose and multi-part metadata. |
| Enemy/boss hostile hitboxes | Present | `content/features/ecs/hitbox.rs:47-90` | Hostile melee hitboxes emit `PlayerDamageEvent`. |
| Unified player/enemy/boss hurtbox model | Partial | Multiple paths exist | Bosses are sophisticated; generic actors/player are less obviously unified. |
| Hurtbox-vs-movement-collider audit | Needed | Multiple systems | Verify which damage paths use movement AABB, authored hurtbox, or animation-derived hurtbox. |

## Combat hit-instance reference target

The transcript describes a Silksong-style `HitInstance` object containing many fields: direction, raw damage, stagger damage, elemental damage, currency/knockback modifiers, and other per-hit metadata. The main design idea is that every damage source creates one rich per-hit payload, and all downstream systems consume that payload instead of each system inventing its own smaller event shape.

## Current combat/damage inventory

| Piece | Status | Current evidence | Notes |
| --- | --- | --- | --- |
| `DamageKind` | Present | `ambition_engine/src/combat.rs:16-25` | Has Slash, Pogo, Contact, Hazard, Projectile, Environmental, Custom. |
| Engine `Damage` payload | Present / Small | `combat.rs:28-34` | Has amount, knockback, kind, source faction, hitstop seconds. |
| Engine `Hitbox` | Present / Small | `combat.rs:59-75` | Short-lived AABB, damage, active time, one-hit-per-target flag. |
| Engine `Hurtbox` | Present | `combat.rs:78-100` | Basic faction-filtered damageable AABB. |
| Engine `DamageVolume` | Present | `combat.rs:106+` | Persistent hazards and damaging areas. |
| Sandbox `DamageEvent` | Present / Small | `content/features/events.rs:242-269` | Player outgoing damage path: volume, damage amount, source, ignored targets. |
| Sandbox `PlayerDamageEvent` | Present / Separate | `content/features/events.rs:65-95` | Incoming damage to player path: mode, source, positions, knockback, strength, amount, target. |
| Hostile ECS hitbox | Present / Small | `content/features/ecs/hitbox.rs:47-90` | Owner/source/anchor/half extent/damage/knockback strength. |
| Boss stagger | Present / Coupled | `boss_encounter.rs` and damage consumers | Stagger exists, but pressure appears tied to damage amount rather than per-hit stagger metadata. |
| Elemental/status damage | Missing | No central per-hit fields found | `DamageKind` is broad category, not an elemental/status system. |
| Hit rejection/result object | Missing | Damage consumers mutate state directly | No generic `HitResult` with applied/rejected reason was found. |

## Combat gaps

### 14. Canonical `HitSpec` / `HitInstance` / `HitResult` pipeline

Status: Missing / Fragmented.

Current damage behavior is split across engine helpers, player outgoing damage messages, player incoming damage messages, hostile hitboxes, projectiles, hazards, and boss-specific consumers. The code works, but there is no single per-hit truth object.

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

Why this matters: one slash, projectile, or hazard can overlap multiple targets. Each target may have different faction, invulnerability, armor, boss phase, shield/parry state, hurtbox part, stagger resistance, elemental resistance, and reward policy.

### 15. Raw damage vs final damage

Status: Missing / Partial.

Current events mostly carry a single damage number. Some player incoming difficulty/invulnerability rules happen later, but there is no central raw/scaled/final damage breakdown.

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

### 16. Separate stagger / poise / armor damage

Status: Missing / Partial.

Boss stagger exists, but the hit payload does not carry independent stagger or poise values. That prevents attacks from doing low health damage but high stagger damage, or vice versa.

Target: add per-hit `stagger`, `poise`, and `armor_break` metadata, with defaults derived from health damage for backward compatibility.

### 17. Elemental/status damage metadata

Status: Missing.

`DamageKind` describes broad source class, not elemental/status payloads. Add tags or bitmasks for fire, poison, silk, shock, curse, bleed, etc. only when design needs them.

Target:

```rust
bitflags::bitflags! {
    pub struct ElementMask: u32 {
        const FIRE = 1 << 0;
        const POISON = 1 << 1;
        const SHOCK = 1 << 2;
        const SILK = 1 << 3;
    }
}
```

### 18. Reaction metadata

Status: Missing / Partial.

Knockback exists in multiple forms, but reaction is not unified. Incoming player damage has `knockback_dir` and `strength`; engine `Damage` has a vector; player slash source has `knock_x`; hostile hitbox has `knockback_strength`.

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

### 19. Presentation metadata

Status: Missing / Partial.

VFX, SFX, hit flash, camera shake, and hitstop are mostly hardcoded by consumers rather than carried by the hit. Engine `Damage` has `hitstop_seconds`, but sandbox hit paths do not appear to use one unified presentation payload.

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

### 20. Hit rejection reasons

Status: Missing.

A target may ignore or reduce a hit because of invulnerability, armor, faction, one-hit-per-target, boss phase, shield, parry, damage immunity, or cooldown. Today these decisions are split across consumers and are hard to inspect uniformly.

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

### 21. Resource and economy metadata

Status: Missing / Unknown.

The transcript mentions fields like currency knockback multiplier. Ambition does not need that exact concept immediately, but the equivalent place would be a `HitReward` or `HitEconomy` component on the hit.

Target examples:

```rust
pub struct HitReward {
    pub energy_gain: i32,
    pub currency_multiplier: f32,
    pub combo_meter_gain: f32,
    pub refresh_dash: bool,
    pub refresh_air_jump: bool,
}
```

### 22. Debug/analytics trace metadata

Status: Missing / Partial.

Ambition has movement op/combo tracking, debug tooling, and gameplay effects, but not a unified hit trace that records every hit instance and every rejection reason.

Target: emit structured trace events from `HitResult` creation:

```rust
pub struct HitTraceEvent {
    pub frame: u64,
    pub source_label: String,
    pub target_label: String,
    pub result: HitResultSummary,
}
```

## Recommended implementation order

### Phase 1: Document and test current behavior

- Add tests for current jump buffer, dash buffer, coyote time, variable jump, wall jump, double jump, pogo, blink hold behavior, fast fall, glide, and ledge momentum carry.
- Add tests that prove attack/projectile/pogo inputs are currently not buffered.
- Add tests around current damage paths: player slash to enemy, projectile to enemy, hostile hitbox to player, hazard to player, boss hitbox to player.

### Phase 2: Introduce action buffering without changing semantics

- Add an `ActionBuffer` type and wire jump/dash through it behind compatibility methods.
- Keep existing tuning values and timer behavior stable.
- Add trace/debug output for "buffer filled", "buffer consumed", and "buffer expired".

### Phase 3: Buffer more actions

- Add attack buffer.
- Add pogo buffer that preserves pogo/down-attack intent.
- Add projectile/tool buffer for cooldown and release-edge forgiveness.
- Decide whether blink uses a formal buffer, held-arm behavior, or both.
- Add ledge action buffers if playtesting shows missed ledge intents.

### Phase 4: Add jump polish

- Add apex hang gravity scaling.
- Add held-jump sustain or equivalent early-jump gravity scaling.
- Add wall-coyote time and corner/head-bonk correction if tests/playtesting show misses.
- Add sprint/long-jump momentum rules once base jump feel is stable.

### Phase 5: Introduce hit-instance pipeline

- Add engine-level `HitSpec`, `HitInstance`, `HitResult`, `HitRejectReason`, `HitDamage`, `HitReaction`, and `HitPresentation` types.
- Convert player slash and projectile producers to emit `HitSpec` while preserving existing `DamageEvent` behavior via adapter.
- Resolve overlaps into `HitInstance` values and convert them into current effects.
- Port hostile hitbox/player damage path onto the same flow.
- Move boss stagger, VFX/SFX, hitstop, shield/parry, invulnerability, and reward logic into structured hit resolution over time.

## Prioritized open checklist

- [ ] Add `ActionBuffer` design note or RFC.
- [ ] Add attack buffer.
- [ ] Add pogo buffer.
- [ ] Add projectile/tool buffer.
- [ ] Decide blink buffer vs hold-arm semantics.
- [ ] Add ledge action buffering if needed.
- [ ] Add apex hang tuning.
- [ ] Add held-jump sustain or early-jump gravity scaling.
- [ ] Add wall coyote timer.
- [ ] Audit corner correction/head-bonk forgiveness.
- [ ] Add sprint/long-jump momentum rule.
- [ ] Add `HitSpec` / `HitInstance` / `HitResult` design note or RFC.
- [ ] Add raw/final/stagger/poise damage fields.
- [ ] Add unified hit reaction metadata.
- [ ] Add hit presentation metadata.
- [ ] Add hit rejection reasons and hit trace events.
- [ ] Audit which damage paths use movement AABB, authored hurtbox, or animation-derived hurtbox.
- [ ] Add tests for all currently-present movement assists.
- [ ] Add tests proving currently-missing buffers before implementing them.

## Notes for future reviews

- Keep this file as a gap inventory. Implementation details should move into separate RFCs or issue-specific docs when they become actionable.
- Re-run this review after any major movement/combat refactor and update `review_date` plus the source archive or commit hash.
- Prefer adding exact source links or line references when this repo is hosted in a stable remote; line numbers in this file are based on the reviewed archive and can drift.
