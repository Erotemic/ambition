//! Vortex — a player-wielded **crowd-control** gauntlet: fire a singularity at a
//! point and it drags nearby enemies toward it for a moment. Distinct from
//! every other wielded attack (which deal damage / teleport): the vortex deals
//! *no* damage — it **gathers** a scattered group so the player can follow up
//! with an AOE (`crate::abilities::ranged::shockwave` / `crate::abilities::ranged::beam`) or a volley. Pull-then-slam.
//!
//! Distinct from the gravity grenade too: that spawns a *directional*
//! `GravityZone` (up-lift); the vortex is a *point* attractor — it lerps each
//! enemy's position toward the singularity center, clamped by the normal
//! collision step (`step_kinematic` resolves any wall the pull pushes into).
//! Bosses share the unified `BodyKinematics` now, but the faction guard below
//! (`ActorFaction::Boss != Enemy`) keeps them immune; only grounded/aerial mobs
//! (and peaceful NPCs, harmlessly) match the `Enemy` faction and get pulled.

use ambition_characters::brain::ActorControl;
use bevy::prelude::*;

use crate::actor::BodyMana;
use crate::features::{ActorFaction, BodyKinematics, FeatureSimEntity, HeldItem};
use ambition_engine_core as ae;
use ambition_platformer_primitives::lifecycle::{
    SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer_primitives::markers::ControlledSubject;

/// Held-item id of the vortex gauntlet.
pub const VORTEX_ID: &str = "vortex";

/// Mana the vortex spends per cast (out of 100). A utility, gated like the rest.
const VORTEX_MANA_COST: f32 = 22.0;

/// How far in front of the player (along aim) the singularity spawns.
const VORTEX_RANGE: f32 = 200.0;
/// Radius (px) within which enemies get dragged toward the center.
const VORTEX_RADIUS: f32 = 220.0;
/// Pull rate (1/s) — the fraction of the remaining gap closed per second
/// (`lerp` factor `rate * dt`). Higher = a snappier gather. Feel-tune.
const VORTEX_PULL_RATE: f32 = 5.0;
/// How long (s) the singularity persists pulling.
const VORTEX_LIFETIME_S: f32 = 0.9;

/// A live vortex singularity: pulls enemies toward `center` until `remaining_s`
/// hits zero.
#[derive(Component, Debug, Clone, Copy)]
pub struct VortexWell {
    pub center: ae::Vec2,
    pub remaining_s: f32,
}

/// `Attack` while holding the vortex gauntlet spawns a [`VortexWell`] at a point
/// ahead of the player along the aim. Plain Attack only — `Shield + Attack`
/// drops the item (the id is `UseSystem`, excluded from throw-on-plain-Attack).
pub fn fire_vortex_system(
    // Ability ORIGIN = the controlled subject, not a `PrimaryPlayer` filter.
    controlled: Res<ControlledSubject>,
    mut bodies: Query<(
        &ActorControl,
        &BodyKinematics,
        &crate::physics::ResolvedMotionFrame,
        &HeldItem,
        &mut BodyMana,
        Option<&SessionScopedEntity>,
    )>,
    mut commands: Commands,
    mut sfx: ambition_sfx::SfxWriter,
) {
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((control, kin, resolved_frame, held, mut mana, owner)) = bodies.get_mut(subject) else {
        return;
    };
    let c = control.0;
    if !c.melee_pressed || c.shield_held {
        return;
    }
    if held.spec.id != VORTEX_ID {
        return;
    }
    if !mana.meter.try_spend(VORTEX_MANA_COST) {
        return;
    }
    // The body's per-tick resolved frame (ADR 0024 frame law).
    let gravity_dir = resolved_frame.down();
    let aim =
        crate::items::pickup::ability_aim_world(&c, kin.facing, gravity_dir).normalize_or_zero();
    if aim == ae::Vec2::ZERO {
        return;
    }
    let center = kin.pos + aim * VORTEX_RANGE;
    commands.spawn_session_scoped(
        SessionSpawnScope::new(owner.map(|owner| owner.0)),
        (
            VortexWell {
                center,
                remaining_s: VORTEX_LIFETIME_S,
            },
            Name::new("Vortex singularity"),
        ),
    );
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: ambition_sfx::ids::PLAYER_BLINK,
        pos: center,
    });
}

/// Drag every Enemy-faction actor within [`VORTEX_RADIUS`] of each live well
/// toward its center (a position lerp; the actor's own `step_kinematic` next
/// tick resolves any wall it's pulled into), then age the wells out. Runs on
/// `scaled_dt` so bullet-time slows the gather with everything else.
pub fn update_vortex_wells(
    world_time: Res<ambition_time::WorldTime>,
    mut commands: Commands,
    mut wells: Query<(Entity, &mut VortexWell)>,
    mut actors: Query<(&mut BodyKinematics, &ActorFaction), With<FeatureSimEntity>>,
) {
    let dt = world_time.scaled_dt;
    if dt <= 0.0 {
        return;
    }
    let factor = (VORTEX_PULL_RATE * dt).min(1.0);
    for (entity, mut well) in &mut wells {
        for (mut kin, faction) in &mut actors {
            if *faction != ActorFaction::Enemy {
                continue;
            }
            if kin.pos.distance(well.center) <= VORTEX_RADIUS {
                // The well is an external kinematic constraint (ADR 0024 authority):
                // it carries the body toward the center by this tick's pull delta.
                let delta = kin.pos.lerp(well.center, factor) - kin.pos;
                ae::movement::carry_body(&mut kin, delta);
            }
        }
        well.remaining_s -= dt;
        if well.remaining_s <= 0.0 {
            if let Ok(mut ec) = commands.get_entity(entity) {
                ec.despawn();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::test_support::spawn_primary_player_holding;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.insert_resource(ambition_time::WorldTime {
            raw_dt: 0.016,
            scaled_dt: 0.016,
        });
        app.add_systems(Update, (fire_vortex_system, update_vortex_wells).chain());
        app
    }

    fn spawn_enemy(app: &mut App, pos: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                FeatureSimEntity,
                BodyKinematics {
                    pos,
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                ActorFaction::Enemy,
            ))
            .id()
    }

    #[test]
    fn attack_with_vortex_spawns_a_well_and_pulls_a_nearby_enemy_inward() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, VORTEX_ID);
        // Player at (100,100), facing +x → well at (300,100). Enemy just inside
        // the radius, off to the side, should be dragged toward the center.
        let enemy = spawn_enemy(&mut app, ae::Vec2::new(420.0, 100.0));
        let start_dist = ae::Vec2::new(420.0, 100.0).distance(ae::Vec2::new(300.0, 100.0));
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        // A well exists.
        let well_count = app
            .world_mut()
            .query::<&VortexWell>()
            .iter(app.world())
            .count();
        assert_eq!(well_count, 1, "one vortex well spawned");
        // The enemy moved closer to the well center.
        let new_pos = app.world().get::<BodyKinematics>(enemy).unwrap().pos;
        let new_dist = new_pos.distance(ae::Vec2::new(300.0, 100.0));
        assert!(
            new_dist < start_dist,
            "enemy should be pulled toward the singularity: {start_dist} -> {new_dist}"
        );
    }

    #[test]
    fn vortex_ignores_a_far_enemy_and_expires() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, VORTEX_ID);
        // Far away (well at 300,100; enemy at 900 — outside the 220 radius).
        let far = spawn_enemy(&mut app, ae::Vec2::new(900.0, 100.0));
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = false;
        let far_pos = app.world().get::<BodyKinematics>(far).unwrap().pos;
        assert_eq!(
            far_pos.x, 900.0,
            "an enemy outside the radius is not pulled"
        );
        // Age it out: lifetime 0.9s at 0.016/tick → ~57 ticks. Run plenty.
        for _ in 0..70 {
            app.update();
        }
        let well_count = app
            .world_mut()
            .query::<&VortexWell>()
            .iter(app.world())
            .count();
        assert_eq!(well_count, 0, "the well expires and despawns");
    }
}
