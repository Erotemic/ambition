//! Victim-side combat damage resolution: shield blocks, knockback, hitstun,
//! safe-respawn, and death-respawn for the controlled body, plus the
//! `apply_player_hit_events` system that drives them off the victim-side
//! `HitEvent` channel.
//!
//! This is runtime (sim) logic — it emits `SfxMessage`/`VfxMessage` *facts* but
//! holds no render dependency (`VfxMessage` is `ambition_vfx`, not
//! `ambition_render`). It lived in `ambition_app` only because it was authored
//! beside the room/attack glue; it belongs here in the combat runtime, where the
//! state it mutates already lives.
//!
//! Player-centrism note: the bodies still name the controlled actor "player"
//! because the component vocabulary (`BodyCombat`, `ae::BodyClustersMut`)
//! does. The relativity-principle fix is the actor-unification rename of those
//! types, tracked separately; this drain only relocates and de-render-couples.

use bevy::prelude::{Entity, MessageReader, MessageWriter, Query, Res, ResMut};

use ambition_engine_core as ae;
use ambition_vfx::vfx::VfxMessage;
use ambition_world::collision::MovingPlatformSet;

use crate::actor::BodyAnimFacts;
use crate::actor::PrimaryPlayerOnly;
use crate::avatar::PlayerSafetyState;
use crate::combat::events::{GameplayBannerRequested, HitEvent as FeatureHitEvent, HitTarget};
use crate::time::feel::SandboxFeelTuning;
use crate::time::time_control::{ClockRequester, ClockResetRequest};
use crate::{
    remember_safe_player_position, ActorDiedMessage, SafePositionContext, SandboxSimState,
};
use ambition_characters::actor::BodyCombat;
use ambition_characters::actor::BodyHealth;
use ambition_characters::equipment::WornEquipment;
use ambition_dev_tools::dev_tools::EditableMovementTuning;
use ambition_engine_core::RoomGeometry;
use ambition_sfx::SfxMessage;

// `body_vulnerable` / `shield_blocks_hit` moved to `crate::combat::util`
// (E2): they are the shared victim-gate predicates every damage EMITTER
// reads — combat vocabulary, not victim-side application code.
pub use crate::combat::util::{body_vulnerable, shield_blocks_hit};

// `scaled_knockback` moved to `crate::combat::util` (E2): the CM1
// knockback-scaling LAW is combat model vocabulary.
pub use crate::combat::util::scaled_knockback;

/// THE directional-influence law (CM2): the victim's held control rotates its
/// OWN knockback launch, by at most `max_angle` radians. Pure and
/// frame-agnostic — `launch` is the resolved world-frame launch velocity;
/// `di_input_local` is the victim's `ActorControl.locomotion` (local `x` = side,
/// `y` = gravity-down, magnitude a `[0,1]` throttle); `gravity_dir` places that
/// local intent into the world frame. The rotation turns `launch` TOWARD the
/// held direction, weighted by how PERPENDICULAR the input is to the launch
/// (you cannot DI along your own launch line) and by the throttle — classic
/// smash DI. PARITY: `max_angle == 0.0` (or a null input) returns `launch`
/// unchanged, so DI is inert until a game authors a budget. Frame-agnostic
/// because `launch` and the world-frame input rotate together under any gravity,
/// so the victim-local trajectory conjugates (the C4 law).
pub fn di_adjust(
    launch: ae::Vec2,
    di_input_local: ae::Vec2,
    gravity_dir: ae::Vec2,
    max_angle: f32,
) -> ae::Vec2 {
    if max_angle <= 0.0 {
        return launch;
    }
    let speed = launch.length();
    if speed < 1e-6 {
        return launch;
    }
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let di_world = frame.to_world(di_input_local);
    let di_mag = di_world.length();
    if di_mag < 1e-6 {
        return launch;
    }
    let throttle = di_mag.min(1.0);
    let launch_dir = launch / speed;
    let di_dir = di_world / di_mag;
    // Signed sine of the angle FROM launch TO the held direction: its magnitude
    // is the perpendicular fraction, its sign the way to rotate.
    let cross = launch_dir.x * di_dir.y - launch_dir.y * di_dir.x;
    let rot = (max_angle * cross.abs() * throttle).min(max_angle) * cross.signum();
    let (s, c) = rot.sin_cos();
    ae::Vec2::new(launch.x * c - launch.y * s, launch.x * s + launch.y * c)
}

/// Per-body feel values for [`resolve_body_hit`] — how hard the hit reads on
/// THIS body (blink length, i-frame window), not whether it lands. The player
/// and actors pass different numbers; the rule is one.
#[derive(Clone, Copy, Debug)]
pub struct BodyHitFeel {
    /// Damage-blink armed on a damaging hit.
    pub hit_flash: f32,
    /// Post-hit i-frame window armed on a damaging hit.
    pub damage_invuln_time: f32,
    /// Damage-blink armed on a BLOCKED hit (0.0 leaves the flash untouched).
    pub block_hit_flash: f32,
    /// Guard i-frame on a blocked hit: the timer is raised to at least this.
    pub block_invuln_floor: f32,
}

/// What [`resolve_body_hit`] decided about one hit on one body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BodyHitResolution {
    /// The hit doesn't register at all: the body is inside its post-hit
    /// i-frame window, or already dead. No state was touched — the caller
    /// plays no feedback and records no landed hit.
    Ignored,
    /// The body's raised shield consumed the hit: no damage, but the hit DID
    /// register (guard i-frame armed; the caller plays block feedback).
    Blocked,
    /// A worn armor equipment row absorbed the hit (A3
    /// `OnHit::ConsumeAsArmor`): no HP damage, the row was spent (removed or
    /// downgraded), and the SAME brief i-frames a damaging hit arms are armed.
    /// The hit registered, but it never reaches HP or the death path.
    Armored,
    /// The hit landed. `damage` is the post-multiplier amount applied; `died`
    /// is whether it killed the body.
    Damaged { damage: i32, died: bool },
}

/// THE one victim-side hit resolver every body shares (fable review 2026-07-02
/// §A2): consume-time i-frame gate → directional shield block → scaled damage →
/// death flag → hit-flash/i-frame arming. The player and actor consumers both
/// call this; what stays outside is genuine per-body POLICY (difficulty
/// multiplier choice, death→respawn vs death→drops, peaceful-actor barks,
/// knockback style — the last is step 6 of the §A2 plan).
///
/// MECHANICS contract:
/// - i-frames are consumed HERE, at consume time, for every body — no emitter
///   decides them (an emitter may still read `body_vulnerable` to early-out or
///   mute feedback, but the event must flow).
/// - `damage_multiplier` is a policy hook (player: difficulty × assist;
///   actors: 1.0). A landed hit always deals at least 1 damage.
/// - `never_dies` bodies (training dummies) take no health damage at all.
/// - `health: None` (headless test bodies) resolves as damaged-but-undying.
#[allow(clippy::too_many_arguments)]
pub fn resolve_body_hit(
    combat: &mut BodyCombat,
    mut health: Option<&mut BodyHealth>,
    armor: Option<&mut WornEquipment>,
    shield_active: bool,
    facing: f32,
    body_pos: ae::Vec2,
    impact_pos: ae::Vec2,
    gravity_dir: ae::Vec2,
    raw_damage: i32,
    damage_multiplier: f32,
    never_dies: bool,
    feel: BodyHitFeel,
) -> BodyHitResolution {
    if !combat.vulnerable() {
        return BodyHitResolution::Ignored;
    }
    if let Some(health) = health.as_deref() {
        if !health.alive() {
            return BodyHitResolution::Ignored;
        }
    }
    if shield_blocks_hit(shield_active, facing, body_pos, impact_pos, gravity_dir) {
        if feel.block_hit_flash > 0.0 {
            combat.hit_flash = feel.block_hit_flash;
        }
        combat.damage_invuln_timer = combat.damage_invuln_timer.max(feel.block_invuln_floor);
        return BodyHitResolution::Blocked;
    }
    // A3 armor-on-hit (shield beats armor beats damage): a worn armor row spends
    // itself BEFORE the hit reaches HP. The wearer takes zero HP damage and gets
    // the same brief i-frames a damaging hit arms. Generic — any body carrying a
    // `WornEquipment` with a `ConsumeAsArmor` row gets this; the player is the
    // only wirer today. `never_dies` bodies still spend armor (a downgrade is a
    // state change worth honoring), then reach the no-death path below anyway.
    if let Some(armor) = armor {
        if armor.consume_armor().is_some() {
            combat.hit_flash = feel.hit_flash;
            combat.damage_invuln_timer = feel.damage_invuln_time;
            return BodyHitResolution::Armored;
        }
    }
    combat.hit_flash = feel.hit_flash;
    combat.damage_invuln_timer = feel.damage_invuln_time;
    let damage = ((raw_damage as f32) * damage_multiplier).round() as i32;
    let damage = damage.max(1);
    let died = if never_dies {
        false
    } else {
        health
            .as_deref_mut()
            .map(|health| health.damage(damage))
            .unwrap_or(false)
    };
    BodyHitResolution::Damaged { damage, died }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn death_respawn_player(
    world: &ae::World,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    died: &mut MessageWriter<ActorDiedMessage>,
    clusters: &mut ae::BodyClustersMut<'_>,
    sim_state: &mut SandboxSimState,
    clock_resets: &mut MessageWriter<ClockResetRequest>,
    safety: &mut PlayerSafetyState,
    banner_requests: &mut MessageWriter<GameplayBannerRequested>,
    player_health: Option<&mut BodyHealth>,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
    cause: crate::DeathCause,
    anim: &mut BodyAnimFacts,
    combat: &mut BodyCombat,
) {
    let to = world.spawn;
    ae::reset_body_clusters(clusters, world.spawn);
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning.air_jumps,
    );
    clusters.mana.meter.refill_full();
    safety.last_safe_pos = world.spawn;
    clock_resets.write(ClockResetRequest::sim_clock(
        ClockRequester::Engine,
        "death_respawn",
    ));
    sim_state.room_transition_cooldown = 0.0;
    anim.reset();
    combat.reset();
    if let Some(health) = player_health {
        health.reset();
    }
    combat.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    combat.hit_flash = feel.reset_flash_time.max(0.35);
    banner_requests.write(GameplayBannerRequested::new(
        "PLAYER DOWN: respawned at room start with full HP",
        2.4,
    ));
    sfx.write(SfxMessage::Death { pos: from });
    vfx.write(VfxMessage::ResetEffects { from, to });
    died.write(ActorDiedMessage { pos: from, cause });
}

/// Resolve this frame's hits against one body. Returns **true when the body was
/// Class-B remapped** (`collision-and-ccd.md` §3.2) — a death respawn or a
/// hazard safe-respawn both teleport it. Knockback does not: it writes velocity,
/// which is Class-A's business. The caller owns the entity id, so the caller
/// records into `ClassBRemapLog`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_player_damage_events(
    world: &ae::World,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    died: &mut MessageWriter<ActorDiedMessage>,
    clusters: &mut ae::BodyClustersMut<'_>,
    sim_state: &mut SandboxSimState,
    clock_resets: &mut MessageWriter<ClockResetRequest>,
    safety: &mut PlayerSafetyState,
    banner_requests: &mut MessageWriter<GameplayBannerRequested>,
    mut player_health: Option<&mut BodyHealth>,
    // A3: the player's worn equipment, so an armor row can absorb this hit before
    // it reaches HP. `None` for a player wearing nothing.
    armor: Option<&mut WornEquipment>,
    damage_events: &[FeatureHitEvent],
    tuning: ae::MovementTuning,
    // The body's frame down direction, resolved by the environment.
    gravity_dir: ae::Vec2,
    feel: SandboxFeelTuning,
    difficulty_multiplier: f32,
    // The controlled body's held locomotion (local frame) for DI (CM2).
    di_input_local: ae::Vec2,
    anim: &mut BodyAnimFacts,
    combat: &mut BodyCombat,
) -> bool {
    let Some(damage) = damage_events.first().cloned() else {
        return false;
    };
    // Consume-time vulnerability (§A2): invincibility (debug toggle),
    // dodge-roll i-frames, and an active parry drop the event before any state
    // mutates. The post-hit i-frame window is consumed inside the resolver —
    // the SAME rule for every body; emitters no longer decide it.
    if !body_vulnerable(clusters.offense, clusters.dodge, clusters.shield, combat) {
        return false;
    }
    let impact_pos = damage
        .knockback
        .as_ref()
        .map(|k| k.impact_pos)
        .unwrap_or_else(|| damage.volume.center());
    // THE shared mechanics: shield block (the body's RESOLVED guard —
    // `resolve_shield` is ability-gated and dash-blocked, invariant I3),
    // difficulty-scaled damage, death flag, hit-flash + i-frame arming.
    // Difficulty is player POLICY: easy halves incoming damage, hard doubles
    // it, plus the fine-grained gameplay multiplier and assist factor.
    let resolution = resolve_body_hit(
        combat,
        player_health.as_deref_mut(),
        armor,
        clusters.shield.active,
        clusters.kinematics.facing,
        clusters.kinematics.pos,
        impact_pos,
        gravity_dir,
        damage.damage,
        difficulty_multiplier,
        false,
        BodyHitFeel {
            hit_flash: 0.20,
            damage_invuln_time: feel.knockback_invulnerability_time,
            block_hit_flash: 0.0,
            block_invuln_floor: 0.12,
        },
    );
    match resolution {
        BodyHitResolution::Ignored => false,
        BodyHitResolution::Blocked => {
            sfx.write(SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_ROCK_HIT,
                pos: clusters.kinematics.pos,
            });
            banner_requests.write(GameplayBannerRequested::new("blocked", 1.0));
            false
        }
        // A3: a worn armor row (mushroom-analog) absorbed the hit. No HP change,
        // no respawn/teleport (so no Class-B remap), just the spent powerup and
        // the brief i-frames the resolver already armed.
        BodyHitResolution::Armored => {
            sfx.write(SfxMessage::Play {
                id: ambition_sfx::ids::PLAYER_DAMAGE,
                pos: impact_pos,
            });
            banner_requests.write(GameplayBannerRequested::new("POWERUP LOST", 1.4));
            false
        }
        BodyHitResolution::Damaged { died: true, .. } => {
            // Attribution for the death fact: the killing hit's source category
            // plus its attacker entity when the source carries one.
            let cause = crate::DeathCause {
                source: damage.source.clone(),
                attacker: damage.attacker,
            };
            death_respawn_player(
                world,
                sfx,
                vfx,
                died,
                clusters,
                sim_state,
                clock_resets,
                safety,
                banner_requests,
                player_health,
                tuning,
                feel,
                impact_pos,
                cause,
                anim,
                combat,
            );
            true
        }
        BodyHitResolution::Damaged { died: false, .. } => match damage.mode {
            crate::combat::HitMode::SafeRespawn => {
                safe_respawn_player(
                    sfx,
                    vfx,
                    clusters,
                    clock_resets,
                    safety,
                    combat,
                    tuning,
                    feel,
                    impact_pos,
                );
                true
            }
            crate::combat::HitMode::Knockback => {
                // Getting hit knocks you off a ledge grab — you fall with the
                // knockback instead of hanging there immune.
                clusters.ledge.knock_off_on_hit();
                apply_player_knockback(
                    sfx,
                    vfx,
                    clusters,
                    combat,
                    tuning,
                    gravity_dir,
                    feel,
                    &damage,
                    di_input_local,
                );
                false
            }
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn safe_respawn_player(
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::BodyClustersMut<'_>,
    clock_resets: &mut MessageWriter<ClockResetRequest>,
    safety: &PlayerSafetyState,
    combat: &mut BodyCombat,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
) {
    let to = safety.last_safe_pos;
    ae::reset_body_clusters(clusters, to);
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning.air_jumps,
    );
    combat.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    combat.hitstun_timer = 0.0;
    combat.recoil_lock_timer = 0.0;
    combat.hitstop_timer = 0.0;
    combat.hit_flash = feel.reset_flash_time;
    clock_resets.write(ClockResetRequest::sim_clock(
        ClockRequester::Engine,
        "safe_respawn",
    ));
    sfx.write(SfxMessage::Reset { pos: to });
    vfx.write(VfxMessage::ResetEffects { from, to });
}

/// THE feel-tuned, frame-agnostic knockback velocity for ANY struck body
/// (§A2 step 6): side away from the hit's source (falling back to the stored
/// event dir, then away from facing), scaled by the per-source feel values and
/// the hit's strength, launched with a rise against the body's gravity.
pub(crate) fn resolved_body_knockback_velocity(
    victim_pos: ae::Vec2,
    victim_facing: f32,
    gravity_dir: ae::Vec2,
    boss_hit: bool,
    knockback: Option<&crate::combat::HitKnockback>,
    // The victim's held control (local frame), for directional influence (CM2).
    // `ZERO` == no DI intent; the effect is also inert unless `feel.di_max_angle`
    // is nonzero, so this is parity-free by construction.
    di_input_local: ae::Vec2,
    feel: SandboxFeelTuning,
) -> ae::Vec2 {
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let side_from_source = knockback.map(|k| (victim_pos - k.source_pos).dot(frame.side));
    let knockback_dir = side_from_source
        .filter(|d| d.abs() > 0.001)
        .or_else(|| knockback.map(|k| k.dir))
        .unwrap_or(0.0);
    let dir = if knockback_dir.abs() <= 0.001 {
        -victim_facing
    } else {
        knockback_dir.signum()
    };
    let strength = knockback.map(|k| k.strength.max(0.0)).unwrap_or(0.0);
    let knock_x = if boss_hit {
        feel.boss_knockback_x
    } else {
        feel.enemy_knockback_x
    };
    let knock_y = if boss_hit {
        feel.boss_knockback_y
    } else {
        feel.enemy_knockback_y
    };
    // CM1: a volume-authored launch DIRECTION (smash-style fixed angles)
    // replaces the default feel diagonal while preserving its SPEED — the
    // authored vector is normalized (direction only), `x` mirrored by the
    // away-from-source side sign, `y` positive = up against gravity. The
    // magnitude invariant: an authored direction matching the default
    // diagonal reproduces today's launch bit-for-bit in spirit — same
    // frame, same `|v|` (`hypot(knock_x, knock_y) * strength`).
    let authored = knockback
        .and_then(|k| k.launch_dir)
        .filter(|ld| ld.length_squared() > 1e-6);
    let local = match authored {
        Some(ld) => {
            let n = ld.normalize();
            let speed = ae::Vec2::new(knock_x, knock_y).length() * strength;
            ae::Vec2::new(dir * n.x * speed, -n.y * speed)
        }
        None => ae::Vec2::new(dir * knock_x * strength, -knock_y * strength),
    };
    let launch = frame.to_world(local);
    // CM2: the victim's held input rotates its own launch, bounded by the
    // authored DI budget. Inert at `di_max_angle == 0` (Ambition today).
    di_adjust(launch, di_input_local, gravity_dir, feel.di_max_angle)
}

/// The ONE post-hit launch + stagger arming for ANY struck body (§A2 steps
/// 6–7), called by the player's knockback path and the actor damage consumer:
/// SET the resolved knockback velocity, then arm hitstun (strength- and
/// source-scaled), the fixed recoil throw, and the hitstop beat. hit_flash +
/// damage_invuln_timer are armed by `resolve_body_hit` before this runs —
/// this owns only the launch + control-lock timers.
pub(crate) fn apply_body_hit_reaction(
    vel: &mut ae::Vec2,
    combat: &mut BodyCombat,
    body_pos: ae::Vec2,
    body_facing: f32,
    gravity_dir: ae::Vec2,
    boss_hit: bool,
    knockback: Option<&crate::combat::HitKnockback>,
    // The struck body's held control (local frame) for DI (CM2). `ZERO` = none.
    di_input_local: ae::Vec2,
    feel: SandboxFeelTuning,
) {
    let strength = knockback.map(|k| k.strength.max(0.0)).unwrap_or(0.0);
    *vel = resolved_body_knockback_velocity(
        body_pos,
        body_facing,
        gravity_dir,
        boss_hit,
        knockback,
        di_input_local,
        feel,
    );
    combat.hitstun_timer = if boss_hit {
        feel.boss_hitstun_time
    } else {
        feel.enemy_hitstun_time
    } * strength.max(0.35);
    // Brief hard control-lock at the front of the hitstun window: the body is
    // thrown with no authority, then regains the attack verb the instant it
    // clears (while still in hitstun + i-frames). Fixed-length — the recoil is a
    // readable beat, not something that scales with how hard the hit was.
    combat.recoil_lock_timer = feel.knockback_recoil_lock_time;
    combat.hitstop_timer = feel.player_damage_hitstop_time;
}

pub(crate) fn apply_player_knockback(
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::BodyClustersMut<'_>,
    combat: &mut BodyCombat,
    tuning: ae::MovementTuning,
    // The body's frame down direction, resolved by the environment.
    gravity_dir: ae::Vec2,
    feel: SandboxFeelTuning,
    damage: &FeatureHitEvent,
    // The controlled body's held locomotion (local frame) for DI (CM2).
    di_input_local: ae::Vec2,
) {
    let boss_hit = matches!(
        damage.source,
        crate::combat::HitSource::BossBody | crate::combat::HitSource::BossAttack
    );
    let knockback = damage.knockback.as_ref();
    let impact_pos = knockback
        .map(|k| k.impact_pos)
        .unwrap_or_else(|| damage.volume.center());
    let pos = clusters.kinematics.pos;
    let facing = clusters.kinematics.facing;
    apply_body_hit_reaction(
        &mut clusters.kinematics.vel,
        combat,
        pos,
        facing,
        gravity_dir,
        boss_hit,
        knockback,
        di_input_local,
        feel,
    );
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning.air_jumps,
    );
    sfx.write(SfxMessage::Hit { pos: impact_pos });
    vfx.write(VfxMessage::Impact { pos: impact_pos });
}

/// Resolve this tick's victim-side `HitEvent`s and remember the last safe-spawn
/// position.
///
/// Reads `MessageReader<HitEvent>` and filters to victim-side sources (hazard /
/// enemy / boss); attacker-side hits (player slash, player projectile, pogo) are
/// consumed by `apply_feature_hit_events` separately. Routes the first event
/// through `handle_player_damage_events` — which can knock back, hitstun,
/// hazard-respawn, or fully kill the player — and writes resulting sfx / vfx /
/// died messages directly to their `MessageWriter`s. Then runs
/// `remember_safe_player_position` to update `safety.last_safe_pos` when the
/// player wasn't damaged this frame, isn't blinking, isn't in hitstun, and isn't
/// mid-room-transition.
#[allow(clippy::too_many_arguments)]
pub fn apply_player_hit_events(
    // Bundled into one tuple param to stay under Bevy's 16-system-param ceiling
    // (S3e's relational `relations` + `attacker_factions` pushed this to 17).
    // `class_b` is the §3.2 transit ledger — death and hazard respawn are both
    // Class-B remaps, and this system is where the victim's entity id is known.
    (world, moving_platforms, mut class_b): (
        Res<RoomGeometry>,
        Res<MovingPlatformSet>,
        Option<ResMut<ambition_platformer_primitives::class_b::ClassBRemapLog>>,
    ),
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    user_settings: Res<ambition_persistence::settings::UserSettings>,
    feature_ecs_overlay: Res<crate::world::overlay::FeatureEcsWorldOverlay>,
    mut sim_state: ResMut<SandboxSimState>,
    mut clock_resets: MessageWriter<ClockResetRequest>,
    mut banner_requests: MessageWriter<GameplayBannerRequested>,
    mut hit_events: MessageReader<FeatureHitEvent>,
    mut died_writer: MessageWriter<ActorDiedMessage>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
    // SLOT-0 BY DESIGN: the safe-position memory this feeds is slot 0's respawn
    // point. Damage ROUTING itself is body-generic (it runs off factions and the
    // grudge); only "where does the local player wake up" is primary-scoped.
    primary_q: Query<Entity, crate::actor::PrimaryPlayerOnly>,
    // Friendly-fire policy (the DAMAGE side) + a faction lookup for the hit's
    // attacker: damage is physical, so any DIFFERENT-faction attacker's hit lands
    // on the player — including a duel's stray that the observer walked into. Only
    // a same-faction attacker (co-op ally) is spared, unless friendly fire is on.
    // Whether an actor AIMS at the player is the separate targeting concern.
    friendly_fire: Option<Res<crate::combat::targeting::FriendlyFire>>,
    attacker_factions: Query<&crate::combat::components::ActorFaction>,
    mut player_q: Query<
        (
            Entity,
            ae::BodyClusterQueryData,
            Option<&mut BodyHealth>,
            // A3: the player's worn equipment (mushroom/flower). `Option` so a
            // player wearing nothing still resolves — the common case.
            Option<&mut WornEquipment>,
            &mut BodyAnimFacts,
            &mut BodyCombat,
            &mut PlayerSafetyState,
            // The controlled body's held input, for directional influence (CM2).
            // `Option` so a headless player with no brain still resolves (→ ZERO,
            // no DI). Inert unless `feel.di_max_angle` is authored nonzero.
            Option<&ambition_characters::brain::ActorControl>,
            // The victim's per-tick resolved frame (shield side + knockback
            // launch are frame-relative facts of the VICTIM's body).
            &crate::physics::ResolvedMotionFrame,
        ),
        // SLOT-0 BY DESIGN: this is the PLAYER-VICTIM path — hitstop, the death
        // banner, the safe-position rewind. Actor-vs-actor damage runs through
        // `apply_actor_hit_events` on the same `HitEvent` stream; the two differ
        // only in the feel/save consequences the local human is owed.
        PrimaryPlayerOnly,
    >,
) {
    let primary = primary_q.single().ok();
    // Drain only victim-side hits — attacker-side hits flow to
    // `apply_feature_hit_events`. The two consumers read the same
    // `HitEvent` channel from independent `MessageReader` positions
    // so both see every event but each filters by source-direction.
    let friendly_fire = friendly_fire.map(|r| *r).unwrap_or_default();
    let events: Vec<FeatureHitEvent> = hit_events
        .read()
        .filter(|e| !e.source.is_attacker_side())
        // Friendly-fire gate: a same-faction attacker (co-op ally) doesn't damage
        // the player unless friendly fire is on; any different-faction hit lands
        // (the observer takes a duel's strays). Hits with no entity attacker
        // (hazards, string-owned enemy projectiles) are environmental and always apply.
        .filter(
            |e| match e.attacker.and_then(|a| attacker_factions.get(a).ok()) {
                Some(faction) => crate::combat::targeting::can_damage(
                    *faction,
                    crate::combat::components::ActorFaction::Player,
                    friendly_fire,
                ),
                None => true,
            },
        )
        .cloned()
        .collect();

    let assist_factor = match user_settings.gameplay.assist {
        ambition_persistence::settings::AssistMode::Off => 1.0,
        ambition_persistence::settings::AssistMode::On => 0.5,
    };
    let difficulty_multiplier = user_settings.gameplay.difficulty.damage_taken_multiplier()
        * user_settings.gameplay.player_damage_multiplier
        * assist_factor;
    let tuning = editable_tuning.as_engine();
    let feel = *feel_tuning;
    let safe_world = ambition_world::collision::world_with_sandbox_solids(
        &world.0,
        &moving_platforms.0,
        &feature_ecs_overlay,
    );

    // Resolve every event to a concrete target entity once: events
    // with `HitTarget::Player(e)` route to that player; events with
    // `HitTarget::Volume` (legacy "iterates-and-takes-primary") fall
    // back to the primary player. Events that never resolve (no
    // primary, e.g. headless pre-spawn) are silently dropped.
    let resolved: Vec<(Entity, FeatureHitEvent)> = events
        .into_iter()
        .filter_map(|e| {
            let target = match e.target {
                HitTarget::Player(entity) => Some(entity),
                HitTarget::Volume => primary,
                // Pre-resolved non-player actor victim + orb-match are not player
                // hits — the actor / breakable consumers own them.
                HitTarget::Actor(_) | HitTarget::OrbMatch => None,
            };
            target.map(|t| (t, e))
        })
        .collect();

    for (
        player_entity,
        mut cluster_item,
        player_health,
        worn,
        mut anim,
        mut combat,
        mut safety,
        control,
        resolved_frame,
    ) in &mut player_q
    {
        let target_events: Vec<FeatureHitEvent> = resolved
            .iter()
            .filter(|(t, _)| *t == player_entity)
            .map(|(_, e)| e.clone())
            .collect();
        let damaged_this_frame = !target_events.is_empty();
        // The victim's held locomotion (local frame) drives DI (CM2).
        let di_input_local = control.map(|c| c.0.locomotion).unwrap_or(ae::Vec2::ZERO);

        let mut clusters = cluster_item.as_clusters_mut();
        // The victim's per-tick resolved frame direction (shield side +
        // knockback launch are frame-relative) — the same value its own
        // movement integrated under this tick.
        let victim_gravity_dir = resolved_frame.down();
        let remapped = handle_player_damage_events(
            &world.0,
            &mut sfx_writer,
            &mut vfx_writer,
            &mut died_writer,
            &mut clusters,
            &mut sim_state,
            &mut clock_resets,
            &mut safety,
            &mut banner_requests,
            player_health.map(|h| h.into_inner()),
            worn.map(|w| w.into_inner()),
            &target_events,
            tuning,
            victim_gravity_dir,
            feel,
            difficulty_multiplier,
            di_input_local,
            &mut anim,
            &mut combat,
        );
        // Class-B transit authority (`collision-and-ccd.md` §3.2). Death and the
        // hazard safe-respawn both teleport the victim; recorded here because
        // this is where the entity id lives.
        if remapped {
            if let Some(log) = class_b.as_mut() {
                log.record(
                    player_entity,
                    ambition_platformer_primitives::class_b::ClassBRemap::DeathOrReset,
                );
            }
        }

        let ctx = SafePositionContext {
            damaged_this_frame,
            in_hitstun: combat.hitstun_timer > 0.0,
            feature_requested_reset: false,
            blink_grace_active: clusters.blink.grace_timer > 0.0,
            room_transitioning: sim_state.room_transition_cooldown > 0.0,
        };
        remember_safe_player_position(&mut safety, &clusters, &safe_world, ctx);
    }
}

#[cfg(test)]
mod tests;
