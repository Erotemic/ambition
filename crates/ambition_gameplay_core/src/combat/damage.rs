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

use ambition_characters::actor::BodyCombat;
use ambition_characters::actor::BodyHealth;
use crate::actor::{BodyDodgeState, BodyOffense, BodyShieldState};
use crate::actor::{PlayerEntity, PrimaryPlayer, PrimaryPlayerOnly};
use ambition_sfx::SfxMessage;
use crate::dev::dev_tools::EditableMovementTuning;
use crate::features::{self, GameplayBanner, HitEvent as FeatureHitEvent};
use crate::player::{PlayerAnimState, PlayerSafetyState};
use ambition_time::ClockState;
use crate::time::feel::SandboxFeelTuning;
use ambition_engine_core::RoomGeometry;
use crate::{
    remember_safe_player_position, ActorDiedMessage, MovingPlatformSet, SafePositionContext,
    SandboxSimState,
};

/// THE one "can this body take a hit right now?" rule, shared by every damage
/// EMITTER that needs an early-out (hazards, enemy hitboxes, boss volumes,
/// body-contact, enemy projectiles). Fable review 2026-07-02 §A5: this
/// predicate was copy-pasted at five emit sites and had already drifted
/// (the projectile site dropped the parry term). i-frames / dodge-roll /
/// parry / invincibility gate a PLAYER-side victim; the actor-side victim
/// consumer applies its own (shield-directional) rule at consume time.
pub fn body_vulnerable(
    offense: &BodyOffense,
    dodge: &BodyDodgeState,
    shield: &BodyShieldState,
    combat: &BodyCombat,
) -> bool {
    !offense.invincible && dodge.roll_timer <= 0.0 && !shield.parrying() && combat.vulnerable()
}

/// Whether a held shield blocks a hit coming from `hit_pos`: you can only guard
/// the local side you face (a hit from behind still lands). A facing of exactly
/// 0 (neutral) guards either side. Pure so the directional rule is unit-tested
/// directly.
pub fn shield_blocks_hit(
    shield_held: bool,
    facing: f32,
    player_pos: ae::Vec2,
    hit_pos: ae::Vec2,
    gravity_dir: ae::Vec2,
) -> bool {
    if !shield_held {
        return false;
    }
    if facing == 0.0 {
        return true;
    }
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let local_side_delta = frame.to_local(hit_pos - player_pos).x;
    // Same local-side sign => the hit is on the side the controlled body faces.
    local_side_delta.signum() == facing.signum()
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
    banner: &mut GameplayBanner,
    player_health: Option<&mut BodyHealth>,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
    cause: crate::DeathCause,
    anim: &mut PlayerAnimState,
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
    banner.show("PLAYER DOWN: respawned at room start with full HP", 2.4);
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
    banner: &mut GameplayBanner,
    mut player_health: Option<&mut BodyHealth>,
    damage_events: &[features::HitEvent],
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    difficulty_multiplier: f32,
    anim: &mut PlayerAnimState,
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
            banner.show("blocked", 1.0);
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
                banner,
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
            features::HitMode::SafeRespawn => {
                safe_respawn_player(
                    sfx, vfx, clusters, clock, safety, combat, tuning, feel, impact_pos,
                );
            }
            features::HitMode::Knockback => {
                // Getting hit knocks you off a ledge grab — you fall with the
                // knockback instead of hanging there immune.
                clusters.ledge.knock_off_on_hit();
                apply_player_knockback(sfx, vfx, clusters, combat, tuning, feel, &damage);
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
    knockback: Option<&features::HitKnockback>,
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
    frame.to_world(ae::Vec2::new(dir * knock_x * strength, -knock_y * strength))
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
    knockback: Option<&features::HitKnockback>,
    feel: SandboxFeelTuning,
) {
    let strength = knockback.map(|k| k.strength.max(0.0)).unwrap_or(0.0);
    *vel = resolved_body_knockback_velocity(
        body_pos, body_facing, gravity_dir, boss_hit, knockback, feel,
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
    damage: &features::HitEvent,
) {
    let boss_hit = matches!(
        damage.source,
        features::HitSource::BossBody | features::HitSource::BossAttack
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
    user_settings: Res<crate::persistence::settings::UserSettings>,
    feature_ecs_overlay: Res<features::FeatureEcsWorldOverlay>,
    mut sim_state: ResMut<SandboxSimState>,
    mut clock: ResMut<ClockState>,
    mut banner: ResMut<GameplayBanner>,
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
    friendly_fire: Option<Res<features::FriendlyFire>>,
    attacker_factions: Query<&crate::combat::components::ActorFaction>,
    mut player_q: Query<
        (
            Entity,
            ae::BodyClusterQueryData,
            Option<&mut BodyHealth>,
            &mut PlayerAnimState,
            &mut BodyCombat,
            &mut PlayerSafetyState,
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
        crate::persistence::settings::AssistMode::Off => 1.0,
        crate::persistence::settings::AssistMode::On => 0.5,
    };
    let difficulty_multiplier = user_settings.gameplay.difficulty.damage_taken_multiplier()
        * user_settings.gameplay.player_damage_multiplier
        * assist_factor;
    let tuning = editable_tuning.as_engine();
    let feel = *feel_tuning;
    let safe_world =
        features::world_with_sandbox_solids(&world.0, &moving_platforms.0, &feature_ecs_overlay);

    // Resolve every event to a concrete target entity once: events
    // with `HitTarget::Player(e)` route to that player; events with
    // `HitTarget::Volume` (legacy "iterates-and-takes-primary") fall
    // back to the primary player. Events that never resolve (no
    // primary, e.g. headless pre-spawn) are silently dropped.
    let resolved: Vec<(Entity, FeatureHitEvent)> = events
        .into_iter()
        .filter_map(|e| {
            let target = match e.target {
                features::HitTarget::Player(entity) => Some(entity),
                features::HitTarget::Volume => primary,
                // Pre-resolved non-player actor victim + orb-match are not player
                // hits — the actor / breakable consumers own them.
                features::HitTarget::Actor(_) | features::HitTarget::OrbMatch => None,
            };
            target.map(|t| (t, e))
        })
        .collect();

    for (player_entity, mut cluster_item, player_health, mut anim, mut combat, mut safety) in
        &mut player_q
    {
        let target_events: Vec<FeatureHitEvent> = resolved
            .iter()
            .filter(|(t, _)| *t == player_entity)
            .map(|(_, e)| e.clone())
            .collect();
        let damaged_this_frame = !target_events.is_empty();

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
            &mut banner,
            player_health.map(|h| h.into_inner()),
            &target_events,
            tuning,
            feel,
            difficulty_multiplier,
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
            &mut combat, None, false, 1.0, pos, pos, DOWN, 5, 1.0, false, TEST_FEEL,
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
            let knockback = features::HitKnockback {
                dir: 0.0,
                strength: 1.0,
                source_pos,
                impact_pos: victim_pos,
            };
            let vel = resolved_body_knockback_velocity(
                victim_pos,
                1.0,
                gravity_dir,
                false,
                Some(&knockback),
                feel,
            );
            let local_vel = ae::Vec2::new(vel.dot(frame.side), vel.dot(frame.down));
            assert!(
                (local_vel - local_expected).length() < 1e-3,
                "knockback should resolve in local side/down for {gravity_dir:?}: {local_vel:?}"
            );
        }
    }
}
