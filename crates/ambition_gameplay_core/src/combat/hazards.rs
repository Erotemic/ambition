//! Hazard tick: patrol motion, contact damage, and the impact SFX/VFX
//! published to the presentation/audio buses.

use super::util::hazard_sfx_id;
use super::*;

/// Tick ECS-authored hazards and publish player damage through Bevy messages.
pub fn update_ecs_hazards(
    world_time: Res<WorldTime>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut hit_events: MessageWriter<HitEvent>,
    player: Query<
        (
            Entity,
            &crate::player::BodyKinematics,
            &crate::player::PlayerOffense,
            &crate::player::PlayerDodgeState,
            &crate::player::PlayerShieldState,
            &crate::player::PlayerCombatState,
        ),
        With<crate::player::PlayerEntity>,
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
        for (player_entity, kin, offense, dodge, shield, combat) in &player {
            let dodge_rolling = dodge.roll_timer > 0.0;
            let player_vulnerable =
                !offense.invincible && !dodge_rolling && !shield.parrying() && combat.vulnerable();
            if !player_vulnerable || !hazard.aabb().strict_intersects(kin.aabb()) {
                continue;
            }
            let pos = kin.pos;
            let knockback_dir = (pos.x - hazard.pos.x).signum();
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
            sfx.write(crate::audio::SfxMessage::Play {
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
                }),
                ignored_targets: Vec::new(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{
        BodyKinematics, PlayerBaseSize, PlayerCombatState, PlayerDodgeState, PlayerEntity,
        PlayerOffense, PlayerShieldState,
    };
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
            PlayerBaseSize {
                base_size: ae::Vec2::new(28.0, 46.0),
            },
            PlayerOffense::default(),
            PlayerDodgeState::default(),
            PlayerShieldState::default(),
            PlayerCombatState::default(),
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
        app.insert_resource(crate::WorldTime::default());
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
}
