//! Per-frame tick for breakable feature entities: respawn countdown
//! and the stand-to-break collapse trigger.

use super::util::player_is_standing_on;
use super::BREAK_ON_STAND_SECONDS;
use super::*;
use ambition_sfx::SfxWriter;

/// Tick ECS-owned breakable timers and stand-to-break triggers.
pub fn update_ecs_breakables(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    player_body_q: Query<
        &ambition_engine_core::BodyKinematics,
        With<ambition_platformer_primitives::markers::PlayerEntity>,
    >,
    mut banner: ResMut<GameplayBanner>,
    mut breakables: Query<
        (
            Entity,
            &FeatureName,
            &CenteredAabb,
            &mut BreakableFeature,
            Option<&mut RespawnTimer>,
            Option<&mut StandTimer>,
        ),
        With<FeatureSimEntity>,
    >,
    mut sfx: SfxWriter,
    mut vfx: MessageWriter<VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
) {
    // Sim clock: breakable respawn / stand-to-break should freeze in
    // bullet-time alongside the player and enemies (ADR 0010).
    let dt = world_time.sim_dt();
    for (entity, name, aabb, mut feature, respawn_timer, stand_timer) in &mut breakables {
        if feature.broken() {
            if let Some(mut timer) = respawn_timer {
                timer.0 = (timer.0 - dt).max(0.0);
                if timer.0 <= 0.0 {
                    feature.breakable.state = ambition_interaction::BreakableState::Intact;
                    feature.breakable.health.reset();
                    commands.entity(entity).remove::<RespawnTimer>();
                    banner.show(format!("{} respawned", name.0.as_str()), 2.6);
                    vfx.write(VfxMessage::Burst {
                        pos: aabb.center,
                        count: 16,
                        speed: 230.0,
                        color: [0.84, 0.95, 1.0, 0.82],
                        kind: ParticleKind::Spark,
                    });
                }
            }
            continue;
        }

        let breaks_on_stand = feature.breakable.collision.blocks_movement()
            && feature.breakable.trigger.allows_stand();
        let Some(mut stand_timer) = stand_timer else {
            continue;
        };
        // Iterate every player so any player standing on a
        // collision-blocking breakable triggers the collapse. Single-
        // player behavior preserved because there's one entity in the
        // iterator today. OVERNIGHT-TODO #17.8 (iterate-all-players
        // "no targeting" B-bucket).
        let any_player_standing = breaks_on_stand
            && player_body_q
                .iter()
                .any(|kin| player_is_standing_on(kin.aabb(), aabb.aabb()));
        if any_player_standing {
            stand_timer.0 += dt;
            if stand_timer.0 >= BREAK_ON_STAND_SECONDS {
                let damage = feature.breakable.health.current.max(1);
                let broke = feature.breakable.apply_damage(damage);
                if broke {
                    begin_ecs_breakable_respawn(&mut commands, entity, &feature.breakable);
                    stand_timer.0 = 0.0;
                    banner.show(format!("{} collapsed under weight", name.0.as_str()), 2.6);
                    emit_breakable_destroyed(aabb.center, &mut sfx, &mut vfx, &mut debris);
                }
            }
        } else {
            stand_timer.0 = (stand_timer.0 - dt * 2.0).max(0.0);
        }
    }
}

#[cfg(test)]
mod breakable_tests {
    //! Stand-to-break collapse as a minimal-App harness: a player standing
    //! on a Solid/OnStand breakable accumulates its StandTimer and, once
    //! past BREAK_ON_STAND_SECONDS, collapses it; standing elsewhere does
    //! not. Drives sim time via a fixed WorldTime::scaled_dt.
    use super::*;
    use ambition_engine_core::BodyBaseSize;
    use ambition_engine_core::BodyKinematics;
    use ambition_interaction::{Breakable, BreakableCollision, BreakableTrigger};
    use ambition_platformer_primitives::markers::PlayerEntity;
    use ambition_time::WorldTime;
    use ambition_vfx::vfx::DebrisBurstMessage;
    use bevy::prelude::{App, Entity, Update};

    const THRESHOLD: f32 = BREAK_ON_STAND_SECONDS;

    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.insert_resource(WorldTime {
            raw_dt: 0.1,
            scaled_dt: 0.1,
        });
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_message::<DebrisBurstMessage>();
        app.add_systems(Update, update_ecs_breakables);
        app
    }

    fn stand_breakable(app: &mut App, center: ae::Vec2, stand: f32) -> Entity {
        let mut b = Breakable::new("brk", 1);
        b.collision = BreakableCollision::Solid;
        b.trigger = BreakableTrigger::OnStand;
        app.world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureName::new("Crate"),
                CenteredAabb::from_center_size(center, ae::Vec2::new(24.0, 24.0)),
                BreakableFeature::new(b),
                StandTimer(stand),
            ))
            .id()
    }

    fn player_at(app: &mut App, pos: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                PlayerEntity,
                BodyKinematics {
                    pos,
                    size: ae::Vec2::new(28.0, 46.0),
                    facing: 1.0,
                    ..Default::default()
                },
                BodyBaseSize {
                    base_size: ae::Vec2::new(28.0, 46.0),
                },
            ))
            .id()
    }

    #[test]
    fn standing_past_the_threshold_collapses_the_breakable() {
        let mut app = app();
        let center = ae::Vec2::new(64.0, 100.0); // top edge = 88
        let brk = stand_breakable(&mut app, center, THRESHOLD - 0.05);
        player_at(&mut app, ae::Vec2::new(64.0, 65.0)); // player bottom = 88 -> standing
        app.update();
        assert!(
            app.world().get::<BreakableFeature>(brk).unwrap().broken(),
            "standing past BREAK_ON_STAND_SECONDS collapses the platform"
        );
    }

    #[test]
    fn not_standing_does_not_collapse() {
        let mut app = app();
        let brk = stand_breakable(&mut app, ae::Vec2::new(64.0, 100.0), THRESHOLD - 0.05);
        player_at(&mut app, ae::Vec2::new(2000.0, 2000.0)); // far away
        app.update();
        assert!(
            !app.world().get::<BreakableFeature>(brk).unwrap().broken(),
            "no player standing -> the stand timer decays, no collapse"
        );
    }
}

/// Schedule a broken breakable for respawn if its policy allows.
///
/// Called from both `apply_feature_hit_events` (typed damage path) and
/// `update_ecs_breakables` (stand-to-break path), so it lives here as a
/// `pub(super)` helper rather than duplicating the policy check.
pub fn begin_ecs_breakable_respawn(
    commands: &mut Commands,
    entity: Entity,
    breakable: &ambition_interaction::Breakable,
) {
    if let ambition_entity_catalog::placements::HazardRespawn::AfterSeconds(seconds) =
        breakable.respawn
    {
        commands.entity(entity).insert(RespawnTimer(seconds));
    }
}

/// Common VFX/SFX/debris emission when a breakable is destroyed by any path.
pub fn emit_breakable_destroyed(
    pos: ae::Vec2,
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    debris: &mut MessageWriter<DebrisBurstMessage>,
) {
    vfx.write(VfxMessage::Burst {
        pos,
        count: 16,
        speed: 230.0,
        color: [0.84, 0.95, 1.0, 0.82],
        kind: ParticleKind::Spark,
    });
    debris.write(DebrisBurstMessage {
        pos,
        cue: PhysicsDebrisCue::Breakable,
    });
    sfx.write(SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_CRATE_BREAK,
        pos,
    });
}
