//! Monitor boxes — Sanic's power-up crates, pure content on two engine seams.
//!
//! A monitor is an LDtk-authored NAMED solid block (`monitor_*`); the demo
//! identifies each by its authored block name — the same durable-identity
//! discipline as Mary-O's `GeoId` bonks, minus the contact seam, because a
//! monitor breaks on Sanic's verbs: land on it while FALLING or touch it while
//! ROLLING. (A riding body never sweeps against solid blocks — code smell #13
//! — so an un-rolled runner passes through; that is survivable here and the
//! smell entry tracks the real fix.)
//!
//! A broken monitor is a mid-run World SUBTRACTION done the established way:
//! its name joins the collision overlay's per-frame `removed_block_names`
//! (the immutable authored base is never edited), so it stops colliding and —
//! via the render reconcile — stops drawing. Re-arms on room (re)load.
//!
//! Grants:
//! - `monitor_super`  → wear the Super Sanic form (`WornCharacter` swap, the
//!   same authority the D-toggle uses).
//! - `monitor_speed`  → SPEED SHOES: a timed multiplier on the body's OWN
//!   `MomentumParams` (top speed + ground accel), restored exactly on expiry.

use bevy::prelude::*;

use ambition::actors::actor::PrimaryPlayer;
use ambition::actors::features::FeatureEcsWorldOverlay;
use ambition::actors::rooms::RoomLoaded;
use ambition::engine_core as ae;
use ambition::platformer::lifecycle::SessionWorldRef;

use crate::{SPEEDWAY_ROOM_ID, SUPER_SANIC_CHARACTER_ID};

/// Authored block-name prefix that marks a block as a monitor.
pub const MONITOR_PREFIX: &str = "monitor_";
/// The two authored monitors (block names in the LDtk file).
pub const SUPER_MONITOR: &str = "monitor_super";
pub const SPEED_MONITOR: &str = "monitor_speed";

/// How long the speed shoes last (sim seconds) and what they multiply.
const SPEED_SHOES_SECONDS: f32 = 8.0;
const SPEED_SHOES_TOP_SPEED_FACTOR: f32 = 1.4;
const SPEED_SHOES_ACCEL_FACTOR: f32 = 1.5;

/// Vertical tolerance (px) for "feet on the monitor's lid".
const STOMP_BAND: f32 = 16.0;

/// Which monitors are broken this run. A Vec, not a HashSet: the overlay
/// contribution iterates it every frame and the sim determinism contract bans
/// std-hash iteration order.
#[derive(Resource, Default)]
pub struct SpentMonitors(pub Vec<String>);

impl SpentMonitors {
    fn is_broken(&self, name: &str) -> bool {
        self.0.iter().any(|broken| broken == name)
    }
}

/// The timed speed-shoes grant riding on the player body. Carries the saved
/// authored params so expiry restores EXACTLY what the catalog authored.
#[derive(Component, Debug)]
pub struct SpeedShoes {
    pub remaining: f32,
    saved_top_speed: f32,
    saved_ground_accel: f32,
}

/// **The break.** A falling player whose feet land on a monitor's lid, or a
/// rolling player overlapping it, breaks it once: burst + cue + the grant.
pub fn break_monitor_boxes(
    mut commands: Commands,
    mut spent: ResMut<SpentMonitors>,
    geometry: SessionWorldRef<ae::RoomGeometry>,
    mut vfx: MessageWriter<ambition::vfx::VfxMessage>,
    mut sfx: ambition::sfx::SfxWriter,
    mut players: Query<
        (
            Entity,
            &ae::BodyKinematics,
            &mut ambition::characters::actor::WornCharacter,
            &mut ae::MotionModel,
            Option<&crate::ball_dash::Rolling>,
            Option<&SpeedShoes>,
        ),
        With<PrimaryPlayer>,
    >,
) {
    let Ok((entity, kin, mut worn, mut model, rolling, shoes)) = players.single_mut() else {
        return;
    };
    let rolling = rolling.is_some();
    let falling = kin.vel.y > 0.0;
    if !rolling && !falling {
        return;
    }
    let p = kin.aabb();
    for block in &geometry.0.blocks {
        if !block.name.starts_with(MONITOR_PREFIX) || spent.is_broken(&block.name) {
            continue;
        }
        let b = block.aabb;
        let overlap_x = p.min.x < b.max.x && p.max.x > b.min.x;
        let overlap_y = p.min.y < b.max.y && p.max.y > b.min.y;
        let feet = p.max.y;
        let stomp =
            falling && overlap_x && feet >= b.min.y - STOMP_BAND && feet <= b.min.y + STOMP_BAND;
        let roll = rolling && overlap_x && overlap_y;
        if !stomp && !roll {
            continue;
        }
        spent.0.push(block.name.clone());
        let center = (b.min + b.max) * 0.5;
        vfx.write(ambition::vfx::VfxMessage::Burst {
            pos: center,
            count: 16,
            speed: 170.0,
            color: [0.55, 0.75, 0.95, 1.0],
            kind: ambition::vfx::ParticleKind::Shard,
        });
        sfx.write(ambition::sfx::SfxMessage::Jump { pos: center });
        match block.name.as_str() {
            SUPER_MONITOR => {
                // The transformation grant reuses the ONE identity authority
                // (WornCharacter), exactly like the D-toggle.
                *worn = ambition::characters::actor::WornCharacter::new(SUPER_SANIC_CHARACTER_ID);
            }
            SPEED_MONITOR => {
                // Never stack: a second pair of shoes while one is live would
                // save the already-multiplied params and "restore" them.
                if shoes.is_none() {
                    if let ae::MotionModel::SurfaceMomentum(momentum) = &mut *model {
                        commands.entity(entity).insert(SpeedShoes {
                            remaining: SPEED_SHOES_SECONDS,
                            saved_top_speed: momentum.params.top_speed,
                            saved_ground_accel: momentum.params.ground_accel,
                        });
                        momentum.params.top_speed *= SPEED_SHOES_TOP_SPEED_FACTOR;
                        momentum.params.ground_accel *= SPEED_SHOES_ACCEL_FACTOR;
                    }
                }
            }
            other => {
                // An authored monitor with no grant is a level-authoring bug;
                // break it visibly but loudly note the miss in debug builds.
                debug_assert!(false, "monitor block '{other}' has no authored grant");
            }
        }
    }
}

/// Count the shoes down on the SIM clock and restore the authored params
/// exactly on expiry.
pub fn tick_speed_shoes(
    mut commands: Commands,
    time: Res<ambition::time::WorldTime>,
    mut bodies: Query<(Entity, &mut ae::MotionModel, &mut SpeedShoes)>,
) {
    for (entity, mut model, mut shoes) in &mut bodies {
        shoes.remaining -= time.scaled_dt;
        if shoes.remaining > 0.0 {
            continue;
        }
        if let ae::MotionModel::SurfaceMomentum(momentum) = &mut *model {
            momentum.params.top_speed = shoes.saved_top_speed;
            momentum.params.ground_accel = shoes.saved_ground_accel;
        }
        commands.entity(entity).remove::<SpeedShoes>();
    }
}

/// Contribute each broken monitor's authored NAME to the collision overlay's
/// per-frame `removed_block_names` — the engine's immutable-base subtraction
/// seam. Runs AFTER the overlay rebuild clears the list (its clean-slate
/// contract), the same slot Mary-O's bricks take.
pub fn contribute_broken_monitors_to_overlay(
    spent: Res<SpentMonitors>,
    mut overlay: ResMut<FeatureEcsWorldOverlay>,
) {
    overlay.removed_block_names.extend(spent.0.iter().cloned());
}

/// Re-arm every monitor when the speedway (re)loads.
pub fn rearm_monitors_on_room_loaded(
    mut rooms: MessageReader<RoomLoaded>,
    mut spent: ResMut<SpentMonitors>,
) {
    for message in rooms.read() {
        if message.room_id == SPEEDWAY_ROOM_ID {
            spent.0.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_broken_monitor_is_subtracted_from_the_collision_overlay() {
        let mut app = App::new();
        app.init_resource::<FeatureEcsWorldOverlay>();
        app.insert_resource(SpentMonitors(vec![SUPER_MONITOR.to_string()]));
        app.add_systems(Update, contribute_broken_monitors_to_overlay);
        app.update();
        let removed = &app
            .world()
            .resource::<FeatureEcsWorldOverlay>()
            .removed_block_names;
        assert!(
            removed.contains(&SUPER_MONITOR.to_string()),
            "broken monitors are named in removed_block_names: {removed:?}"
        );
    }

    #[test]
    fn a_reload_rearms_the_monitors() {
        let mut app = App::new();
        app.insert_resource(SpentMonitors(vec![SPEED_MONITOR.to_string()]));
        app.add_message::<RoomLoaded>();
        app.add_systems(Update, rearm_monitors_on_room_loaded);
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<RoomLoaded>>()
            .write(RoomLoaded {
                room_id: SPEEDWAY_ROOM_ID.to_string(),
            });
        app.update();
        assert!(
            app.world().resource::<SpentMonitors>().0.is_empty(),
            "a level (re)load restocks the monitors"
        );
    }

    #[test]
    fn expired_speed_shoes_restore_the_authored_params() {
        let mut app = App::new();
        app.insert_resource(ambition::time::WorldTime {
            scaled_dt: 10.0,
            ..Default::default()
        });
        app.add_systems(Update, tick_speed_shoes);
        let params = ae::MomentumParams {
            top_speed: 1200.0,
            ground_accel: 900.0,
            ..Default::default()
        };
        let mut boosted = params;
        boosted.top_speed *= SPEED_SHOES_TOP_SPEED_FACTOR;
        boosted.ground_accel *= SPEED_SHOES_ACCEL_FACTOR;
        let body = app
            .world_mut()
            .spawn((
                ae::MotionModel::surface_momentum(boosted),
                SpeedShoes {
                    remaining: 1.0,
                    saved_top_speed: params.top_speed,
                    saved_ground_accel: params.ground_accel,
                },
            ))
            .id();
        app.update();
        let ae::MotionModel::SurfaceMomentum(momentum) =
            app.world().get::<ae::MotionModel>(body).unwrap()
        else {
            panic!("body keeps its momentum policy");
        };
        assert_eq!(momentum.params.top_speed, 1200.0, "top speed restored");
        assert_eq!(momentum.params.ground_accel, 900.0, "accel restored");
        assert!(
            app.world().get::<SpeedShoes>(body).is_none(),
            "expired shoes come off"
        );
    }
}
