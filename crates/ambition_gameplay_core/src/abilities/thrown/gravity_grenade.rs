//! Gravity grenade — a thrown held item that, on its fuse, opens a short-lived
//! up-gravity *well* instead of exploding. Enemies and items caught in it fall
//! UP and float — a crowd-control tool that emerges from the localized-gravity
//! system: the grenade just spawns a [`GravityZone`]; the existing per-actor
//! gravity (`gravity_dir_at`) does the lifting, so no bespoke "lift" code is
//! needed.
//!
//! Reuses the bomb's substrate: a "pure throwable" `HeldItemSpec` (no melee /
//! ranged verb), armed with a fuse the frame it starts moving (thrown), then on
//! expiry it spawns a [`TemporaryZone`] gravity well and despawns. A resting
//! debug grenade never arms until thrown.

use bevy::prelude::*;

use crate::items::pickup::GroundItem;
use crate::physics::{GravityZone, TemporaryZone};
use ambition_engine_core as ae;

/// Held-item id the gravity grenade grants.
pub const GRAVITY_GRENADE_ID: &str = "gravity_grenade";

/// Seconds from being thrown to the well opening.
pub const GRAVITY_GRENADE_FUSE_SECS: f32 = 0.7;
/// How long the up-gravity well lingers once opened.
const WELL_DURATION_SECS: f32 = 3.5;
/// Half-extent of the well region (px).
const WELL_HALF: ae::Vec2 = ae::Vec2::new(110.0, 150.0);

/// Lit fuse on a thrown gravity grenade.
#[derive(Component, Clone, Copy, Debug)]
pub struct GravityGrenadeFuse {
    pub timer: f32,
}

/// Arm a thrown gravity grenade: a moving `gravity_grenade` [`GroundItem`] (just
/// thrown) that isn't armed yet gets a lit fuse. A resting debug grenade stays
/// safe so the player can pick it up.
pub fn arm_thrown_gravity_grenades(
    mut commands: Commands,
    grenades: Query<(Entity, &GroundItem), Without<GravityGrenadeFuse>>,
) {
    for (entity, ground) in &grenades {
        if ground.spec.id == GRAVITY_GRENADE_ID && ground.vel != ae::Vec2::ZERO {
            commands.entity(entity).insert(GravityGrenadeFuse {
                timer: GRAVITY_GRENADE_FUSE_SECS,
            });
        }
    }
}

/// Burn fuses; on expiry open a temporary up-gravity well at the grenade and
/// despawn it.
pub fn tick_gravity_grenade_fuses(
    time: Res<crate::WorldTime>,
    mut commands: Commands,
    mut grenades: Query<(Entity, &GroundItem, &mut GravityGrenadeFuse)>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for (entity, ground, mut fuse) in &mut grenades {
        fuse.timer -= dt;
        if fuse.timer > 0.0 {
            continue;
        }
        commands.spawn((
            GravityZone {
                aabb: ae::Aabb::new(ground.pos, WELL_HALF),
                dir: ae::Vec2::new(0.0, -1.0), // up
            },
            TemporaryZone {
                remaining: WELL_DURATION_SECS,
            },
            Name::new("Gravity well (grenade)"),
        ));
        sfx.write(crate::audio::SfxMessage::Play {
            id: ambition_sfx::ids::PORTAL_POWERUP,
            pos: ground.pos,
        });
        vfx.write(ambition_vfx::vfx::VfxMessage::Explosion {
            pos: ground.pos,
            kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
            scale: 0.7,
        });
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grenade_ground(vel: ae::Vec2) -> GroundItem {
        GroundItem {
            spec: ambition_characters::brain::held_item_by_id(GRAVITY_GRENADE_ID).unwrap(),
            pos: ae::Vec2::new(100.0, 100.0),
            vel,
            half_extent: ae::Vec2::splat(16.0),
        }
    }

    #[test]
    fn a_thrown_grenade_arms_but_a_resting_one_does_not() {
        let mut app = App::new();
        app.add_systems(Update, arm_thrown_gravity_grenades);
        let thrown = app
            .world_mut()
            .spawn(grenade_ground(ae::Vec2::new(60.0, -200.0)))
            .id();
        let resting = app.world_mut().spawn(grenade_ground(ae::Vec2::ZERO)).id();
        app.update();
        assert!(
            app.world().get::<GravityGrenadeFuse>(thrown).is_some(),
            "a thrown grenade arms",
        );
        assert!(
            app.world().get::<GravityGrenadeFuse>(resting).is_none(),
            "a resting grenade stays safe",
        );
    }

    #[test]
    fn fuse_expiry_opens_a_temporary_up_well_and_despawns() {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<ambition_vfx::vfx::VfxMessage>();
        let mut wt = crate::WorldTime::default();
        wt.scaled_dt = GRAVITY_GRENADE_FUSE_SECS + 0.1;
        app.insert_resource(wt);
        app.add_systems(Update, tick_gravity_grenade_fuses);

        let grenade = app
            .world_mut()
            .spawn((
                grenade_ground(ae::Vec2::new(40.0, -120.0)),
                GravityGrenadeFuse {
                    timer: GRAVITY_GRENADE_FUSE_SECS,
                },
            ))
            .id();
        app.update();

        assert!(
            app.world().get::<GroundItem>(grenade).is_none(),
            "the grenade despawns when the well opens",
        );
        let mut q = app.world_mut().query::<(&GravityZone, &TemporaryZone)>();
        let wells: Vec<_> = q.iter(app.world()).collect();
        assert_eq!(wells.len(), 1, "one temporary well opened");
        assert_eq!(
            wells[0].0.dir,
            ae::Vec2::new(0.0, -1.0),
            "the well pulls up"
        );
        assert!(wells[0].1.remaining > 0.0, "the well has a lifetime");
    }
}
