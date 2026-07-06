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
            &crate::actor::BodyKinematics,
            &CenteredAabb,
            &crate::actor::BodyOffense,
            &crate::actor::BodyDodgeState,
            &crate::actor::BodyShieldState,
            &ambition_characters::actor::BodyCombat,
        ),
        (With<crate::actor::PlayerEntity>, Without<FeatureSimEntity>),
    >,
    gravity: crate::physics::GravityCtx,
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
            Option<&crate::actor::BodyKinematics>,
            &CenteredAabb,
            &crate::actor::BodyOffense,
            &crate::actor::BodyDodgeState,
            &crate::actor::BodyShieldState,
            &ambition_characters::actor::BodyCombat,
            &ambition_characters::actor::BodyHealth,
        ),
        (
            With<FeatureSimEntity>,
            Without<crate::actor::PlayerEntity>,
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
        for (player_entity, kin, hurtbox, offense, dodge, shield, combat) in &player {
            // CC2 (the sweep law): a hazard touch is path-dependent — a fast body
            // (blink, dash, Sanic run) must not tunnel through a thin spike
            // between frames. Route through the ONE swept trigger primitive
            // instead of a discrete endpoint overlap. Parity for slow/standing
            // bodies (it subsumes the old `strict_intersects`).
            if !crate::combat::damage::body_vulnerable(offense, dodge, shield, combat)
                || !ae::cast::aabb_path_contacts(
                    hurtbox.center,
                    hurtbox.half_size,
                    kin.vel * dt,
                    hazard.aabb(),
                )
            {
                continue;
            }
            let pos = kin.pos;
            // Knockback side in the victim's LOCAL frame (§B11).
            let side = ae::AccelerationFrame::new(gravity.dir_at(pos)).side;
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
        for (victim, kin, hurtbox, offense, dodge, shield, combat, health) in &actor_victims {
            // CC2: every body sweeps the same way (relativity principle) — an
            // actor lured onto spikes at speed can't tunnel them either.
            let delta = kin.map(|k| k.vel * dt).unwrap_or(ae::Vec2::ZERO);
            if health.current() <= 0
                || !crate::combat::damage::body_vulnerable(offense, dodge, shield, combat)
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
mod tests {
    use super::*;
    use crate::actor::BodyKinematics;
    use crate::actor::PlayerEntity;
    use crate::actor::{BodyBaseSize, BodyDodgeState, BodyOffense, BodyShieldState};
    use ambition_characters::actor::BodyCombat;
    use bevy::prelude::{App, MessageReader, ResMut, Resource, Update};

    #[derive(Resource, Default)]
    struct HitLog(Vec<HitSource>);

    fn record_hits(mut reader: MessageReader<HitEvent>, mut log: ResMut<HitLog>) {
        for e in reader.read() {
            log.0.push(e.source.clone());
        }
    }

    fn spawn_player(app: &mut App, pos: ae::Vec2) {
        app.world_mut().spawn((
            PlayerEntity,
            BodyKinematics {
                pos,
                size: ae::Vec2::new(28.0, 46.0),
                facing: 1.0,
                ..Default::default()
            },
            // The published combat footprint every body carries (§A6).
            ae::CenteredAabb::from_center_size(pos, ae::Vec2::new(28.0, 46.0)),
            BodyBaseSize {
                base_size: ae::Vec2::new(28.0, 46.0),
            },
            BodyOffense::default(),
            BodyDodgeState::default(),
            BodyShieldState::default(),
            BodyCombat::default(),
        ));
    }

    fn spawn_hazard(app: &mut App, id: &str, pos: ae::Vec2) {
        let aabb = ae::Aabb::new(pos, ae::Vec2::new(16.0, 16.0));
        let hazard =
            HazardRuntime::new(id, id, aabb, crate::combat::DamageVolume::new(id, aabb, 1));
        app.world_mut().spawn((
            FeatureSimEntity,
            FeatureName::new(id),
            CenteredAabb::from_center_size(pos, ae::Vec2::new(32.0, 32.0)),
            HazardFeature::new(hazard),
        ));
    }

    fn app_with_hazard_system() -> App {
        let mut app = App::new();
        app.insert_resource(ambition_time::WorldTime::default());
        app.init_resource::<HitLog>();
        app.add_message::<HitEvent>();
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_message::<DebrisBurstMessage>();
        app.add_systems(Update, (update_ecs_hazards, record_hits).chain());
        app
    }

    #[test]
    fn player_touching_a_hazard_emits_a_hazard_hit() {
        let mut app = app_with_hazard_system();
        let pos = ae::Vec2::new(100.0, 100.0);
        spawn_player(&mut app, pos);
        spawn_hazard(&mut app, "spikes", pos);
        app.update();
        assert!(
            app.world()
                .resource::<HitLog>()
                .0
                .iter()
                .any(|s| matches!(s, HitSource::Hazard)),
            "overlapping a hazard should emit a HitSource::Hazard hit"
        );
    }

    #[test]
    fn player_clear_of_a_hazard_takes_no_hit() {
        let mut app = app_with_hazard_system();
        spawn_player(&mut app, ae::Vec2::new(100.0, 100.0));
        spawn_hazard(&mut app, "spikes", ae::Vec2::new(900.0, 900.0));
        app.update();
        assert!(
            app.world().resource::<HitLog>().0.is_empty(),
            "a hazard the player is clear of should not emit a hit"
        );
    }

    /// Fable review 2026-07-02 §A4: hazards are world danger for EVERY body —
    /// an NPC standing in the spikes takes the same hit a player would
    /// (previously the damage query was player-scoped and NPCs were immune).
    #[test]
    fn a_non_player_body_touching_a_hazard_takes_the_hit_too() {
        let mut app = app_with_hazard_system();
        // A player far away (the system requires at least one player to run
        // its damage pass).
        spawn_player(&mut app, ae::Vec2::new(900.0, 900.0));
        let pos = ae::Vec2::new(100.0, 100.0);
        let victim = app
            .world_mut()
            .spawn((
                crate::features::FeatureSimEntity,
                ae::CenteredAabb::from_center_size(pos, ae::Vec2::new(24.0, 40.0)),
                BodyOffense::default(),
                BodyDodgeState::default(),
                BodyShieldState::default(),
                BodyCombat::default(),
                ambition_characters::actor::BodyHealth::new(
                    ambition_characters::actor::Health::new(5),
                ),
            ))
            .id();
        spawn_hazard(&mut app, "spikes", pos);
        app.update();
        let world = app.world_mut();
        let mut reader = world
            .resource_mut::<bevy::prelude::Messages<HitEvent>>()
            .get_cursor();
        let world = app.world();
        let hits: Vec<_> = reader
            .read(world.resource::<bevy::prelude::Messages<HitEvent>>())
            .collect();
        assert!(
            hits.iter().any(|e| matches!(e.source, HitSource::Hazard)
                && matches!(e.target, HitTarget::Actor(v) if v == victim)),
            "an overlapping non-player body should take a pre-resolved hazard hit; got {hits:?}"
        );
    }

    /// CC2 (the sweep law): a body leaps ACROSS a hazard in one frame, ending
    /// CLEAR of it. The old discrete endpoint overlap missed the tunnel; the
    /// swept path catches it. This is the tunneling class §7.6 retires.
    #[test]
    fn a_fast_body_cannot_tunnel_through_a_hazard_between_frames() {
        let mut app = app_with_hazard_system();
        {
            let mut wt = app.world_mut().resource_mut::<ambition_time::WorldTime>();
            wt.scaled_dt = 0.1;
            wt.raw_dt = 0.1;
        }
        let end = ae::Vec2::new(160.0, 100.0);
        // vel * dt = 200 px this frame → the path started at x = -40, crossing
        // the hazard at x = 100, and ENDED clear at x = 160.
        app.world_mut().spawn((
            PlayerEntity,
            BodyKinematics {
                pos: end,
                vel: ae::Vec2::new(2000.0, 0.0),
                size: ae::Vec2::new(28.0, 46.0),
                facing: 1.0,
                ..Default::default()
            },
            ae::CenteredAabb::from_center_size(end, ae::Vec2::new(28.0, 46.0)),
            BodyBaseSize {
                base_size: ae::Vec2::new(28.0, 46.0),
            },
            BodyOffense::default(),
            BodyDodgeState::default(),
            BodyShieldState::default(),
            BodyCombat::default(),
        ));
        spawn_hazard(&mut app, "spikes", ae::Vec2::new(100.0, 100.0));
        // Sanity: at the END position the player is CLEAR of the hazard, so a
        // discrete check would emit nothing — any hit here is the swept catch.
        assert!(
            !ae::Aabb::new(end, ae::Vec2::new(14.0, 23.0)).strict_intersects(ae::Aabb::new(
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(16.0, 16.0)
            )),
            "test setup: the end position must be clear of the hazard"
        );
        app.update();
        assert!(
            app.world()
                .resource::<HitLog>()
                .0
                .iter()
                .any(|s| matches!(s, HitSource::Hazard)),
            "a body whose path crossed the hazard should take the hit (no tunneling)"
        );
    }
}
