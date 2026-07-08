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

use bevy::prelude::{Entity, MessageReader, MessageWriter, Query, Res, ResMut, With};

use ambition_engine_core as ae;
use ambition_vfx::vfx::VfxMessage;

use crate::actor::{BodyDodgeState, BodyOffense, BodyShieldState};
use crate::actor::{PlayerEntity, PrimaryPlayer, PrimaryPlayerOnly};
use crate::combat::events::{GameplayBannerRequested, HitEvent as FeatureHitEvent, HitTarget};
use ambition_dev_tools::dev_tools::EditableMovementTuning;
use crate::player::{BodyAnimFacts, PlayerSafetyState};
use crate::time::feel::SandboxFeelTuning;
use crate::{
    remember_safe_player_position, ActorDiedMessage, MovingPlatformSet, SafePositionContext,
    SandboxSimState,
};
use ambition_characters::actor::BodyCombat;
use ambition_characters::actor::BodyHealth;
use ambition_engine_core::RoomGeometry;
use ambition_sfx::SfxMessage;
use ambition_time::ClockState;

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
    clock: &mut ClockState,
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
        tuning,
    );
    clusters.mana.meter.refill_full();
    safety.last_safe_pos = world.spawn;
    clock.time_scale = 1.0;
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_player_damage_events(
    world: &ae::World,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    died: &mut MessageWriter<ActorDiedMessage>,
    clusters: &mut ae::BodyClustersMut<'_>,
    sim_state: &mut SandboxSimState,
    clock: &mut ClockState,
    safety: &mut PlayerSafetyState,
    banner_requests: &mut MessageWriter<GameplayBannerRequested>,
    mut player_health: Option<&mut BodyHealth>,
    damage_events: &[FeatureHitEvent],
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    difficulty_multiplier: f32,
    // The controlled body's held locomotion (local frame) for DI (CM2).
    di_input_local: ae::Vec2,
    anim: &mut BodyAnimFacts,
    combat: &mut BodyCombat,
) {
    let Some(damage) = damage_events.first().cloned() else {
        return;
    };
    // Consume-time vulnerability (§A2): invincibility (debug toggle),
    // dodge-roll i-frames, and an active parry drop the event before any state
    // mutates. The post-hit i-frame window is consumed inside the resolver —
    // the SAME rule for every body; emitters no longer decide it.
    if !body_vulnerable(clusters.offense, clusters.dodge, clusters.shield, combat) {
        return;
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
        clusters.shield.active,
        clusters.kinematics.facing,
        clusters.kinematics.pos,
        impact_pos,
        tuning.gravity_dir,
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
        BodyHitResolution::Ignored => {}
        BodyHitResolution::Blocked => {
            sfx.write(SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_ROCK_HIT,
                pos: clusters.kinematics.pos,
            });
            banner_requests.write(GameplayBannerRequested::new("blocked", 1.0));
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
                clock,
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
        }
        BodyHitResolution::Damaged { died: false, .. } => match damage.mode {
            crate::combat::HitMode::SafeRespawn => {
                safe_respawn_player(
                    sfx, vfx, clusters, clock, safety, combat, tuning, feel, impact_pos,
                );
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
                    feel,
                    &damage,
                    di_input_local,
                );
            }
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn safe_respawn_player(
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::BodyClustersMut<'_>,
    clock: &mut ClockState,
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
        tuning,
    );
    combat.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    combat.hitstun_timer = 0.0;
    combat.recoil_lock_timer = 0.0;
    combat.hitstop_timer = 0.0;
    combat.hit_flash = feel.reset_flash_time;
    clock.time_scale = 1.0;
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
        tuning.gravity_dir,
        boss_hit,
        knockback,
        di_input_local,
        feel,
    );
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning,
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
    (world, moving_platforms): (Res<RoomGeometry>, Res<MovingPlatformSet>),
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    user_settings: Res<ambition_persistence::settings::UserSettings>,
    feature_ecs_overlay: Res<crate::world::overlay::FeatureEcsWorldOverlay>,
    mut sim_state: ResMut<SandboxSimState>,
    mut clock: ResMut<ClockState>,
    mut banner_requests: MessageWriter<GameplayBannerRequested>,
    mut hit_events: MessageReader<FeatureHitEvent>,
    mut died_writer: MessageWriter<ActorDiedMessage>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
    primary_q: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
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
            &mut BodyAnimFacts,
            &mut BodyCombat,
            &mut PlayerSafetyState,
            // The controlled body's held input, for directional influence (CM2).
            // `Option` so a headless player with no brain still resolves (→ ZERO,
            // no DI). Inert unless `feel.di_max_angle` is authored nonzero.
            Option<&ambition_characters::brain::ActorControl>,
        ),
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
    let safe_world = crate::world::overlay_rebuild::world_with_sandbox_solids(
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
        mut anim,
        mut combat,
        mut safety,
        control,
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
        handle_player_damage_events(
            &world.0,
            &mut sfx_writer,
            &mut vfx_writer,
            &mut died_writer,
            &mut clusters,
            &mut sim_state,
            &mut clock,
            &mut safety,
            &mut banner_requests,
            player_health.map(|h| h.into_inner()),
            &target_events,
            tuning,
            feel,
            difficulty_multiplier,
            di_input_local,
            &mut anim,
            &mut combat,
        );

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
mod tests {
    use super::*;

    #[test]
    fn shield_blocks_only_hits_from_the_faced_side() {
        let player = ae::Vec2::new(100.0, 200.0);
        let down = ae::Vec2::new(0.0, 1.0);
        // Controlled body facing local-right (+1) under normal gravity.
        assert!(
            shield_blocks_hit(true, 1.0, player, player + ae::Vec2::new(50.0, 0.0), down),
            "guards a hit from local right"
        );
        assert!(
            !shield_blocks_hit(true, 1.0, player, player + ae::Vec2::new(-50.0, 0.0), down),
            "a hit from behind (local left) lands"
        );
        // Facing local-left (-1) flips it.
        assert!(
            shield_blocks_hit(true, -1.0, player, player + ae::Vec2::new(-50.0, 0.0), down),
            "guards a hit from local left"
        );
        assert!(
            !shield_blocks_hit(true, -1.0, player, player + ae::Vec2::new(50.0, 0.0), down),
            "a hit from behind (local right) lands"
        );
        // No shield held -> never blocks; neutral facing -> guards either side.
        assert!(
            !shield_blocks_hit(false, 1.0, player, player + ae::Vec2::new(50.0, 0.0), down),
            "no shield, no block"
        );
        assert!(
            shield_blocks_hit(true, 0.0, player, player + ae::Vec2::new(-50.0, 0.0), down),
            "neutral facing guards either side"
        );
    }

    #[test]
    fn shield_side_test_uses_the_controlled_body_frame() {
        let player = ae::Vec2::new(100.0, 200.0);
        let right_gravity = ae::Vec2::new(1.0, 0.0);
        // With right gravity, local-right is world-up.
        assert!(
            shield_blocks_hit(
                true,
                1.0,
                player,
                player + ae::Vec2::new(0.0, -50.0),
                right_gravity,
            ),
            "facing local-right should guard the world-up side under right gravity"
        );
        assert!(
            !shield_blocks_hit(
                true,
                1.0,
                player,
                player + ae::Vec2::new(0.0, 50.0),
                right_gravity,
            ),
            "world-down is behind a body facing local-right under right gravity"
        );
    }

    fn test_health(hp: i32) -> BodyHealth {
        BodyHealth::new(ambition_characters::actor::Health::new(hp))
    }

    const TEST_FEEL: BodyHitFeel = BodyHitFeel {
        hit_flash: 0.16,
        damage_invuln_time: 0.2,
        block_hit_flash: 0.16,
        block_invuln_floor: 0.2,
    };

    const DOWN: ae::Vec2 = ae::Vec2::new(0.0, 1.0);

    #[test]
    fn resolver_ignores_a_hit_inside_the_i_frame_window() {
        let mut combat = BodyCombat {
            damage_invuln_timer: 0.1,
            hit_flash: 0.5, // pre-poison: an Ignored hit must not touch state
            ..Default::default()
        };
        let mut health = test_health(5);
        let pos = ae::Vec2::new(100.0, 200.0);
        let res = resolve_body_hit(
            &mut combat,
            Some(&mut health),
            false,
            1.0,
            pos,
            pos + ae::Vec2::new(50.0, 0.0),
            DOWN,
            3,
            1.0,
            false,
            TEST_FEEL,
        );
        assert_eq!(res, BodyHitResolution::Ignored);
        assert_eq!(health.current(), 5, "ignored hit deals no damage");
        assert_eq!(combat.hit_flash, 0.5, "ignored hit arms nothing");
    }

    #[test]
    fn resolver_ignores_a_hit_on_a_dead_body() {
        let mut combat = BodyCombat::default();
        let mut health = test_health(5);
        health.damage(5);
        let pos = ae::Vec2::new(100.0, 200.0);
        let res = resolve_body_hit(
            &mut combat,
            Some(&mut health),
            false,
            1.0,
            pos,
            pos + ae::Vec2::new(50.0, 0.0),
            DOWN,
            3,
            1.0,
            false,
            TEST_FEEL,
        );
        assert_eq!(res, BodyHitResolution::Ignored);
    }

    #[test]
    fn resolver_shield_blocks_a_faced_hit_and_arms_the_guard_i_frame() {
        let mut combat = BodyCombat::default();
        let mut health = test_health(5);
        let pos = ae::Vec2::new(100.0, 200.0);
        let res = resolve_body_hit(
            &mut combat,
            Some(&mut health),
            true,
            1.0,
            pos,
            pos + ae::Vec2::new(50.0, 0.0),
            DOWN,
            3,
            1.0,
            false,
            TEST_FEEL,
        );
        assert_eq!(res, BodyHitResolution::Blocked);
        assert_eq!(health.current(), 5, "a blocked hit deals no damage");
        assert!(
            combat.damage_invuln_timer >= TEST_FEEL.block_invuln_floor,
            "block arms the guard i-frame"
        );
        assert_eq!(combat.hit_flash, TEST_FEEL.block_hit_flash);
        // A hit from BEHIND the guard still lands.
        let mut combat = BodyCombat::default();
        let res = resolve_body_hit(
            &mut combat,
            Some(&mut health),
            true,
            1.0,
            pos,
            pos + ae::Vec2::new(-50.0, 0.0),
            DOWN,
            3,
            1.0,
            false,
            TEST_FEEL,
        );
        assert_eq!(
            res,
            BodyHitResolution::Damaged {
                damage: 3,
                died: false
            }
        );
    }

    #[test]
    fn resolver_scales_damage_arms_feel_and_floors_at_one() {
        let mut combat = BodyCombat::default();
        let mut health = test_health(10);
        let pos = ae::Vec2::new(0.0, 0.0);
        let res = resolve_body_hit(
            &mut combat,
            Some(&mut health),
            false,
            1.0,
            pos,
            pos,
            DOWN,
            3,
            2.0,
            false,
            TEST_FEEL,
        );
        assert_eq!(
            res,
            BodyHitResolution::Damaged {
                damage: 6,
                died: false
            }
        );
        assert_eq!(health.current(), 4);
        assert_eq!(combat.hit_flash, TEST_FEEL.hit_flash);
        assert_eq!(combat.damage_invuln_timer, TEST_FEEL.damage_invuln_time);
        // A landed hit always deals at least 1 (assist can't zero it out).
        let mut combat = BodyCombat::default();
        let res = resolve_body_hit(
            &mut combat,
            Some(&mut health),
            false,
            1.0,
            pos,
            pos,
            DOWN,
            1,
            0.1,
            false,
            TEST_FEEL,
        );
        assert_eq!(
            res,
            BodyHitResolution::Damaged {
                damage: 1,
                died: false
            }
        );
    }

    #[test]
    fn resolver_reports_death_and_never_dies_takes_no_damage() {
        let mut combat = BodyCombat::default();
        let mut health = test_health(2);
        let pos = ae::Vec2::new(0.0, 0.0);
        let res = resolve_body_hit(
            &mut combat,
            Some(&mut health),
            false,
            1.0,
            pos,
            pos,
            DOWN,
            5,
            1.0,
            false,
            TEST_FEEL,
        );
        assert_eq!(
            res,
            BodyHitResolution::Damaged {
                damage: 5,
                died: true
            }
        );
        assert!(!health.alive());
        // A `never_dies` body (training dummy) registers the hit but its HP
        // never moves.
        let mut combat = BodyCombat::default();
        let mut health = test_health(2);
        let res = resolve_body_hit(
            &mut combat,
            Some(&mut health),
            false,
            1.0,
            pos,
            pos,
            DOWN,
            5,
            1.0,
            true,
            TEST_FEEL,
        );
        assert_eq!(
            res,
            BodyHitResolution::Damaged {
                damage: 5,
                died: false
            }
        );
        assert_eq!(health.current(), 2);
        // A headless body with no health component is damaged-but-undying.
        let mut combat = BodyCombat::default();
        let res = resolve_body_hit(
            &mut combat,
            None,
            false,
            1.0,
            pos,
            pos,
            DOWN,
            5,
            1.0,
            false,
            TEST_FEEL,
        );
        assert_eq!(
            res,
            BodyHitResolution::Damaged {
                damage: 5,
                died: false
            }
        );
    }

    #[test]
    fn knockback_impulse_is_frame_equivalent() {
        let feel = SandboxFeelTuning::default();
        let local_expected = ae::Vec2::new(feel.enemy_knockback_x, -feel.enemy_knockback_y);
        let victim_pos = ae::Vec2::new(100.0, 200.0);
        for gravity_dir in [
            ae::Vec2::new(0.0, 1.0),
            ae::Vec2::new(1.0, 0.0),
            ae::Vec2::new(0.0, -1.0),
            ae::Vec2::new(-1.0, 0.0),
        ] {
            let frame = ae::AccelerationFrame::new(gravity_dir);
            let source_pos = victim_pos - frame.side * 40.0;
            let knockback = crate::combat::HitKnockback {
                dir: 0.0,
                strength: 1.0,
                source_pos,
                impact_pos: victim_pos,
                launch_dir: None,
            };
            let vel = resolved_body_knockback_velocity(
                victim_pos,
                1.0,
                gravity_dir,
                false,
                Some(&knockback),
                ae::Vec2::ZERO,
                feel,
            );
            let local_vel = ae::Vec2::new(vel.dot(frame.side), vel.dot(frame.down));
            assert!(
                (local_vel - local_expected).length() < 1e-3,
                "knockback should resolve in local side/down for {gravity_dir:?}: {local_vel:?}"
            );
        }
    }

    // --- CM1: knockback scaling (the smash-percent axis) ---

    #[test]
    fn scaled_knockback_is_parity_at_zero_growth() {
        // growth == 0 returns the flat base for ANY damage/weight — the
        // byte-parity pin that keeps every un-authored volume unchanged.
        for dmg in [0, 5, 50, 999] {
            for w in [0.5, 1.0, 4.0] {
                assert_eq!(scaled_knockback(7.5, 0.0, dmg, w), 7.5);
            }
        }
    }

    #[test]
    fn scaled_knockback_grows_with_damage_and_divides_by_weight() {
        // base + growth * damage / weight.
        assert_eq!(scaled_knockback(10.0, 2.0, 0, 1.0), 10.0);
        assert_eq!(scaled_knockback(10.0, 2.0, 30, 1.0), 70.0);
        // Twice the weight -> half the growth contribution.
        assert_eq!(scaled_knockback(10.0, 2.0, 30, 2.0), 40.0);
        // Monotonic in accumulated damage.
        assert!(scaled_knockback(10.0, 2.0, 60, 1.0) > scaled_knockback(10.0, 2.0, 30, 1.0));
        // Degenerate weight falls back to the reference body (never divides by 0).
        assert_eq!(scaled_knockback(10.0, 2.0, 10, 0.0), 30.0);
    }

    #[test]
    fn scaled_knockback_conjugates_under_rotated_gravity() {
        // C4: a growth-scaled hit under rotated gravity produces the conjugated
        // trajectory — the scalar scaling is frame-agnostic, so the resolved
        // velocity stays identical in the victim's local frame under every
        // gravity, exactly like the flat case.
        let feel = SandboxFeelTuning::default();
        let strength = scaled_knockback(1.0, 0.05, 80, 1.25); // == 4.2
        let local_expected = ae::Vec2::new(
            feel.enemy_knockback_x * strength,
            -feel.enemy_knockback_y * strength,
        );
        let victim_pos = ae::Vec2::new(100.0, 200.0);
        for gravity_dir in [
            ae::Vec2::new(0.0, 1.0),
            ae::Vec2::new(1.0, 0.0),
            ae::Vec2::new(0.0, -1.0),
            ae::Vec2::new(-1.0, 0.0),
        ] {
            let frame = ae::AccelerationFrame::new(gravity_dir);
            let source_pos = victim_pos - frame.side * 40.0;
            let knockback = crate::combat::HitKnockback {
                dir: 0.0,
                strength,
                source_pos,
                impact_pos: victim_pos,
                launch_dir: None,
            };
            let vel = resolved_body_knockback_velocity(
                victim_pos,
                1.0,
                gravity_dir,
                false,
                Some(&knockback),
                ae::Vec2::ZERO,
                feel,
            );
            let local_vel = ae::Vec2::new(vel.dot(frame.side), vel.dot(frame.down));
            assert!(
                (local_vel - local_expected).length() < 1e-3,
                "growth-scaled knockback must conjugate for {gravity_dir:?}: {local_vel:?}"
            );
        }
    }

    // --- CM1: the authored launch DIRECTION (smash-style fixed angles) ---

    #[test]
    fn authored_launch_dir_sets_the_angle_and_keeps_the_default_speed() {
        let feel = SandboxFeelTuning::default();
        let victim_pos = ae::Vec2::new(100.0, 200.0);
        let down = ae::Vec2::new(0.0, 1.0);
        let source_pos = victim_pos - ae::Vec2::new(40.0, 0.0); // hit from local left
        let default_speed = ae::Vec2::new(feel.enemy_knockback_x, feel.enemy_knockback_y).length();

        // A pure up-launcher: (0, 1) launches straight against gravity.
        let up = crate::combat::HitKnockback {
            dir: 0.0,
            strength: 1.0,
            source_pos,
            impact_pos: victim_pos,
            launch_dir: Some(ae::Vec2::new(0.0, 1.0)),
        };
        let vel = resolved_body_knockback_velocity(
            victim_pos,
            1.0,
            down,
            false,
            Some(&up),
            ae::Vec2::ZERO,
            feel,
        );
        assert!(
            vel.x.abs() < 1e-3 && vel.y < 0.0,
            "a (0,1) launcher throws straight up (world -y): {vel:?}"
        );
        assert!(
            (vel.length() - default_speed).abs() < 1e-3,
            "the authored angle keeps the feel-tuned SPEED: |{vel:?}| vs {default_speed}"
        );

        // The lateral component mirrors to point AWAY from the source: hit
        // from the left ⇒ positive local x ⇒ world +x.
        let diag = crate::combat::HitKnockback {
            dir: 0.0,
            strength: 1.0,
            source_pos,
            impact_pos: victim_pos,
            launch_dir: Some(ae::Vec2::new(1.0, 1.0)),
        };
        let vel = resolved_body_knockback_velocity(
            victim_pos,
            1.0,
            down,
            false,
            Some(&diag),
            ae::Vec2::ZERO,
            feel,
        );
        assert!(
            vel.x > 0.0 && vel.y < 0.0,
            "a (1,1) launcher throws up-and-away from the source: {vel:?}"
        );
        // Mirrored source ⇒ mirrored lateral, same rise.
        let mirrored = crate::combat::HitKnockback {
            source_pos: victim_pos + ae::Vec2::new(40.0, 0.0),
            ..diag
        };
        let mvel = resolved_body_knockback_velocity(
            victim_pos,
            1.0,
            down,
            false,
            Some(&mirrored),
            ae::Vec2::ZERO,
            feel,
        );
        assert!(
            (mvel.x + vel.x).abs() < 1e-3 && (mvel.y - vel.y).abs() < 1e-3,
            "the authored angle mirrors with the away-from-source side: {vel:?} vs {mvel:?}"
        );
    }

    #[test]
    fn authored_launch_dir_conjugates_under_rotated_gravity() {
        // C4: the authored angle is a LOCAL-frame fact, so the resolved
        // velocity is identical in the victim's side/down frame under every
        // gravity — the same conjugation invariant the flat + growth paths pin.
        let feel = SandboxFeelTuning::default();
        let victim_pos = ae::Vec2::new(100.0, 200.0);
        let speed = ae::Vec2::new(feel.enemy_knockback_x, feel.enemy_knockback_y).length();
        let n = ae::Vec2::new(0.6, 0.8); // already unit-length
        let local_expected = ae::Vec2::new(n.x * speed, -n.y * speed);
        for gravity_dir in [
            ae::Vec2::new(0.0, 1.0),
            ae::Vec2::new(1.0, 0.0),
            ae::Vec2::new(0.0, -1.0),
            ae::Vec2::new(-1.0, 0.0),
        ] {
            let frame = ae::AccelerationFrame::new(gravity_dir);
            let source_pos = victim_pos - frame.side * 40.0;
            let knockback = crate::combat::HitKnockback {
                dir: 0.0,
                strength: 1.0,
                source_pos,
                impact_pos: victim_pos,
                launch_dir: Some(n),
            };
            let vel = resolved_body_knockback_velocity(
                victim_pos,
                1.0,
                gravity_dir,
                false,
                Some(&knockback),
                ae::Vec2::ZERO,
                feel,
            );
            let local_vel = ae::Vec2::new(vel.dot(frame.side), vel.dot(frame.down));
            assert!(
                (local_vel - local_expected).length() < 1e-3,
                "authored launch must conjugate for {gravity_dir:?}: {local_vel:?}"
            );
        }
    }

    #[test]
    fn zero_length_launch_dir_falls_back_to_the_default_diagonal() {
        // A degenerate authored vector (bad data) must not NaN the launch —
        // it reads as un-authored.
        let feel = SandboxFeelTuning::default();
        let victim_pos = ae::Vec2::new(100.0, 200.0);
        let down = ae::Vec2::new(0.0, 1.0);
        let source_pos = victim_pos - ae::Vec2::new(40.0, 0.0);
        let base = crate::combat::HitKnockback {
            dir: 0.0,
            strength: 1.0,
            source_pos,
            impact_pos: victim_pos,
            launch_dir: None,
        };
        let degenerate = crate::combat::HitKnockback {
            launch_dir: Some(ae::Vec2::ZERO),
            ..base
        };
        let expected = resolved_body_knockback_velocity(
            victim_pos,
            1.0,
            down,
            false,
            Some(&base),
            ae::Vec2::ZERO,
            feel,
        );
        let got = resolved_body_knockback_velocity(
            victim_pos,
            1.0,
            down,
            false,
            Some(&degenerate),
            ae::Vec2::ZERO,
            feel,
        );
        assert_eq!(expected, got);
    }

    #[test]
    fn death_policy_gates_the_meter_kill() {
        use crate::combat::DeathPolicy;
        // HpDepleted (default) kills at the meter's max; Unbounded (smash
        // percent) never does — its death comes from the blast-zone gate.
        assert!(DeathPolicy::default().kills_at_max());
        assert!(DeathPolicy::HpDepleted.kills_at_max());
        assert!(!DeathPolicy::Unbounded.kills_at_max());
    }

    #[test]
    fn damage_taken_is_the_accumulated_meter() {
        let mut h = test_health(20);
        assert_eq!(h.damage_taken(), 0);
        h.damage(7);
        assert_eq!(h.damage_taken(), 7);
        h.damage(100); // clamps at the pool max
        assert_eq!(h.damage_taken(), 20);
    }

    // --- CM2: directional influence ---

    #[test]
    fn di_is_inert_at_zero_budget_or_null_input() {
        let launch = ae::Vec2::new(300.0, -400.0);
        let down = ae::Vec2::new(0.0, 1.0);
        // Zero budget -> no DI, whatever the input.
        assert_eq!(
            di_adjust(launch, ae::Vec2::new(1.0, 0.0), down, 0.0),
            launch
        );
        // Null input -> no DI, even with a budget.
        assert_eq!(di_adjust(launch, ae::Vec2::ZERO, down, 0.35), launch);
        // Zero-length launch (no knockback) is left alone.
        assert_eq!(
            di_adjust(ae::Vec2::ZERO, ae::Vec2::new(1.0, 0.0), down, 0.35),
            ae::Vec2::ZERO
        );
    }

    #[test]
    fn di_rotates_toward_held_input_bounded_by_the_budget() {
        let down = ae::Vec2::new(0.0, 1.0);
        // Launch straight "up" (world -y); hold fully perpendicular (local +x =
        // world +x). Speed is preserved and the vector rotates by exactly the
        // budget (perpendicular input, full throttle).
        let launch = ae::Vec2::new(0.0, -100.0);
        let max = 0.30_f32;
        let out = di_adjust(launch, ae::Vec2::new(1.0, 0.0), down, max);
        assert!((out.length() - 100.0).abs() < 1e-3, "DI preserves speed");
        let ang = (out.x / out.length()).asin(); // angle off vertical toward +x
        assert!(
            (ang - max).abs() < 1e-3,
            "rotates by the full budget: {ang}"
        );
        // Holding INTO the launch line (parallel) cannot DI — no rotation.
        let parallel = di_adjust(launch, ae::Vec2::new(0.0, -1.0), down, max);
        assert!(
            (parallel - launch).length() < 1e-3,
            "cannot DI along the launch"
        );
    }

    #[test]
    fn di_conjugates_under_rotated_gravity() {
        // C4: the SAME local input under rotated gravity yields the conjugated
        // launch — DI is frame-agnostic, so the victim-local outgoing angle is
        // identical under every gravity.
        let max = 0.28_f32;
        let di_local = ae::Vec2::new(1.0, 0.0); // hold local-side
        let local_launch = ae::Vec2::new(0.0, -100.0); // straight up, body-local
        let mut expected_local: Option<ae::Vec2> = None;
        for gravity_dir in [
            ae::Vec2::new(0.0, 1.0),
            ae::Vec2::new(1.0, 0.0),
            ae::Vec2::new(0.0, -1.0),
            ae::Vec2::new(-1.0, 0.0),
        ] {
            let frame = ae::AccelerationFrame::new(gravity_dir);
            let launch_world = frame.to_world(local_launch);
            let out = di_adjust(launch_world, di_local, gravity_dir, max);
            let out_local = ae::Vec2::new(out.dot(frame.side), out.dot(frame.down));
            match expected_local {
                None => expected_local = Some(out_local),
                Some(e) => assert!(
                    (out_local - e).length() < 1e-3,
                    "DI must conjugate for {gravity_dir:?}: {out_local:?} vs {e:?}"
                ),
            }
        }
    }
}
