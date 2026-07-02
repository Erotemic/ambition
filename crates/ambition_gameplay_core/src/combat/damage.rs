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

use crate::actor::BodyCombat;
use crate::actor::BodyHealth;
use crate::actor::{BodyDodgeState, BodyOffense, BodyShieldState};
use crate::actor::{PlayerEntity, PrimaryPlayer, PrimaryPlayerOnly};
use crate::audio::SfxMessage;
use crate::dev::dev_tools::EditableMovementTuning;
use crate::features::{self, GameplayBanner, HitEvent as FeatureHitEvent};
use crate::player::{PlayerAnimState, PlayerInputFrame, PlayerSafetyState};
use crate::time::clock_state::ClockState;
use crate::time::feel::SandboxFeelTuning;
use crate::{
    remember_safe_player_position, ActorDiedMessage, MovingPlatformSet, RoomGeometry,
    SafePositionContext, SandboxSimState,
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
    let Some(mut damage) = damage_events.first().cloned() else {
        return;
    };
    // Invincibility (debug toggle): drop the damage event entirely
    // before any state mutates so testing systems that consume HP
    // (boss phases, encounter pacing, music) can run uninterrupted.
    if clusters.offense.invincible {
        return;
    }
    // Shield block: a held shield fully negates a hit coming from the side the
    // player faces (you can't guard your back). Costs nothing but a short guard
    // i-frame; a defensive verb to complement the offensive/movement abilities.
    let guard_impact = damage
        .knockback
        .as_ref()
        .map(|k| k.impact_pos)
        .unwrap_or_else(|| damage.volume.center());
    // The body's RESOLVED guard (`resolve_shield`: ability-gated, dash-blocked),
    // not the raw held input — the body enforces, the controller only attempts
    // (invariant I3; fable review §A2). The raw-input form let a body with no
    // shield ability block, and let a guard hold through a dash.
    if shield_blocks_hit(
        clusters.shield.active,
        clusters.kinematics.facing,
        clusters.kinematics.pos,
        guard_impact,
        tuning.gravity_dir,
    ) {
        sfx.write(SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_ROCK_HIT,
            pos: clusters.kinematics.pos,
        });
        combat.damage_invuln_timer = combat.damage_invuln_timer.max(0.12);
        banner.show("blocked", 1.0);
        return;
    }
    // Difficulty / assist scaling. Easy halves incoming damage, hard
    // doubles it; the menu setting also exposes a fine-grained
    // gameplay damage multiplier. The minimum is one HP so a damage
    // event always lands somewhere.
    let scaled = ((damage.damage as f32) * difficulty_multiplier).round() as i32;
    damage.damage = scaled.max(1);
    let died_from_damage = if let Some(health) = player_health.as_deref_mut() {
        health.damage(damage.damage)
    } else {
        false
    };
    let impact_pos = damage
        .knockback
        .as_ref()
        .map(|k| k.impact_pos)
        .unwrap_or_else(|| damage.volume.center());
    if died_from_damage {
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
        return;
    }
    match damage.mode {
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

fn resolved_player_knockback_velocity(
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
    let strength = knockback.map(|k| k.strength.max(0.0)).unwrap_or(0.0);
    clusters.kinematics.vel = resolved_player_knockback_velocity(
        clusters.kinematics.pos,
        clusters.kinematics.facing,
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
    combat.hitstun_timer = if boss_hit {
        feel.boss_hitstun_time
    } else {
        feel.enemy_hitstun_time
    } * strength.max(0.35);
    // Brief hard control-lock at the front of the hitstun window: the player is
    // thrown with no authority, then regains the attack verb the instant it
    // clears (while still in hitstun + i-frames). Fixed-length — the recoil is a
    // readable beat, not something that scales with how hard the hit was.
    combat.recoil_lock_timer = feel.knockback_recoil_lock_time;
    combat.damage_invuln_timer = feel.knockback_invulnerability_time;
    combat.hitstop_timer = feel.player_damage_hitstop_time;
    combat.hit_flash = 0.20;
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
            &PlayerInputFrame,
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

    for (player_entity, input, mut cluster_item, player_health, mut anim, mut combat, mut safety) in
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
            let vel = resolved_player_knockback_velocity(
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
