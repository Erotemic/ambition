//! Hazard tick: patrol motion, contact damage, and the impact SFX/VFX
//! published to the presentation/audio buses.

use super::util::hazard_sfx_id;
use super::*;

/// Tick ECS-authored hazards and publish player damage through Bevy messages.
pub fn update_ecs_hazards(
    world_time: Res<WorldTime>,
    mut sfx: MessageWriter<ambition_sfx::SfxMessage>,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut hit_events: MessageWriter<HitEvent>,
    // `Without<FeatureSimEntity>` keeps this read of the player's published
    // `CenteredAabb` (§A6) provably disjoint from the mutable hazard query.
    player: Query<
        (
            Entity,
            &ambition_engine_core::BodyKinematics,
            Option<&ae::SweepSample>,
            &CenteredAabb,
            &ambition_engine_core::BodyOffense,
            &ambition_engine_core::BodyMotionFacts,
            &ambition_engine_core::BodyShieldState,
            &ambition_characters::actor::BodyCombat,
            // The victim's per-tick resolved frame (ADR 0024): the knockback
            // side is a fact of the VICTIM's own frame.
            &ambition_platformer_primitives::frame_env::ResolvedMotionFrame,
        ),
        (
            With<ambition_platformer_primitives::markers::PlayerEntity>,
            Without<FeatureSimEntity>,
        ),
    >,
    // Every OTHER body with a published footprint burns too (fable review
    // 2026-07-02 §A4): hazards are relational-agnostic world danger — an NPC
    // in lava takes the hit, a boss can be lured into spikes. Deliberately NOT
    // faction-gated (unified-actors guardrail 4). `Without<HazardFeature>`
    // keeps this read provably disjoint from the mutable hazard query.
    actor_victims: Query<
        (
            Entity,
            // `Option`: every real body carries kinematics (→ swept), but a bare
            // headless/test hurtbox without it falls back to the discrete check.
            Option<&ambition_engine_core::BodyKinematics>,
            Option<&ae::SweepSample>,
            &CenteredAabb,
            &ambition_engine_core::BodyOffense,
            &ambition_engine_core::BodyMotionFacts,
            &ambition_engine_core::BodyShieldState,
            &ambition_characters::actor::BodyCombat,
            &ambition_characters::actor::BodyHealth,
        ),
        (
            With<FeatureSimEntity>,
            Without<ambition_platformer_primitives::markers::PlayerEntity>,
            Without<HazardFeature>,
        ),
    >,
    mut hazards: Query<
        (&FeatureName, &mut CenteredAabb, &mut HazardFeature),
        With<FeatureSimEntity>,
    >,
) {
    // Sim clock: patrolling damage volumes must slow in bullet-time
    // so the player can route around them. ADR 0010.
    let dt = world_time.sim_dt();
    if player.is_empty() {
        // No players yet (pre-spawn); tick hazard motion but skip the
        // damage check so the patrol path still advances.
        for (_name, mut aabb, mut feature) in &mut hazards {
            let hazard = &mut feature.hazard;
            hazard.update(dt);
            aabb.center = hazard.pos;
            aabb.half_size = hazard.size * 0.5;
        }
        return;
    }
    for (_name, mut aabb, mut feature) in &mut hazards {
        let hazard = &mut feature.hazard;
        hazard.update(dt);
        aabb.center = hazard.pos;
        aabb.half_size = hazard.size * 0.5;
        if !hazard.active() {
            continue;
        }
        // Iterate every player so each overlapping player takes damage
        // independently — a future co-op build wants hazards to bite
        // every player in the volume, not implicitly the primary one.
        // OVERNIGHT-TODO #17.8 (B-bucket iterate-all-players for
        // hazard hits). Single-player behavior preserved because the
        // iterator has exactly one entity today.
        for (player_entity, kin, sweep, hurtbox, offense, facts, shield, combat, resolved_frame) in
            &player
        {
            // CC2 (the sweep law): a hazard touch is path-dependent — a fast body
            // (dash, Sanic run) must not tunnel through a thin spike between
            // frames. The path is the §3.1 SweepSample — the kernel's TRUE
            // integrated segment, which excludes teleports (blink/respawn/
            // portal) by construction, so a blink OVER spikes is not a graze.
            // Bodies without a sample keep the historical `vel·dt`
            // approximation (delete the fallback when every mover writes one).
            let delta = sweep.map(|s| s.delta()).unwrap_or(kin.vel * dt);
            if !crate::util::body_vulnerable(offense, facts.dodge_rolling, shield, combat)
                || !ae::cast::aabb_path_contacts(
                    hurtbox.center,
                    hurtbox.half_size,
                    delta,
                    hazard.aabb(),
                )
            {
                continue;
            }
            let pos = kin.pos;
            // Knockback side in the victim's LOCAL frame (§B11), from its own
            // per-tick resolved frame.
            let side = resolved_frame.basis().side;
            let knockback_dir = (pos - hazard.pos).dot(side).signum();
            vfx.write(VfxMessage::Impact { pos });
            vfx.write(VfxMessage::Burst {
                pos,
                count: 14,
                speed: 300.0,
                color: [1.0, 0.34, 0.28, 0.88],
                kind: ParticleKind::Shard,
            });
            debris.write(DebrisBurstMessage {
                pos,
                cue: PhysicsDebrisCue::Impact,
            });
            sfx.write(ambition_sfx::SfxMessage::Play {
                id: hazard_sfx_id(&hazard.name),
                pos,
            });
            hit_events.write(HitEvent {
                volume: hazard.aabb().into(),
                damage: hazard.volume.damage.amount.max(1),
                source: HitSource::Hazard,
                attacker: None,
                // Hazards iterate every overlapping player; tag the
                // event with the player who actually overlapped so
                // the reader lands the hit on the right one.
                target: HitTarget::Player(player_entity),
                mode: hazard.mode,
                knockback: Some(HitKnockback {
                    dir: knockback_dir,
                    strength: 1.0,
                    source_pos: hazard.pos,
                    impact_pos: pos,
                    launch_dir: None,
                }),
                ignored_targets: Vec::new(),
            });
        }
        // Non-player bodies: same hazard, same rule, pre-resolved victim.
        // Knockback is left to the victim consumer (actor knockback rides the
        // resolver, not the event — see §A2).
        for (victim, kin, sweep, hurtbox, offense, facts, shield, combat, health) in &actor_victims
        {
            // CC2: every body sweeps the same way (relativity principle) — an
            // actor lured onto spikes at speed can't tunnel them either. The
            // §3.1 sample (the true integrated segment) wins; a body without
            // one keeps the historical `vel·dt` approximation; a bare hurtbox
            // stays discrete.
            let delta = sweep
                .map(|s| s.delta())
                .or_else(|| kin.map(|k| k.vel * dt))
                .unwrap_or(ae::Vec2::ZERO);
            if health.current() <= 0
                || !crate::util::body_vulnerable(offense, facts.dodge_rolling, shield, combat)
                || !ae::cast::aabb_path_contacts(
                    hurtbox.center,
                    hurtbox.half_size,
                    delta,
                    hazard.aabb(),
                )
            {
                continue;
            }
            let pos = hurtbox.center;
            vfx.write(VfxMessage::Impact { pos });
            sfx.write(ambition_sfx::SfxMessage::Play {
                id: hazard_sfx_id(&hazard.name),
                pos,
            });
            hit_events.write(HitEvent {
                volume: hazard.aabb().into(),
                damage: hazard.volume.damage.amount.max(1),
                source: HitSource::Hazard,
                attacker: None,
                target: HitTarget::Actor(victim),
                mode: hazard.mode,
                knockback: None,
                ignored_targets: Vec::new(),
            });
        }
    }
}

#[cfg(test)]
mod tests;
