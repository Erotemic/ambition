//! Gravity grenade: a thrown held item that, when its fuse ends, opens a
//! short-lived up-gravity well instead of exploding. Enemies and items in it
//! fall up and float. The grenade only spawns a [`GravityZone`]; the per-actor
//! gravity (`gravity_dir_at`) does the lifting.
//!
//! Uses the bomb's substrate: a pure-throwable `HeldItemSpec` (no melee or
//! ranged verb), armed when thrown. On expiry it spawns a [`TemporaryZone`]
//! gravity well and despawns. A grenade that was never thrown does not arm.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::{
    SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};

use ambition_held_items::{GroundItem, Release, ReleasedAs};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::gravity::{GravityZone, TemporaryZone};

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

/// Arm a thrown gravity grenade: a `gravity_grenade` [`GroundItem`] with
/// [`ReleasedAs`] throw gets a lit fuse. A grenade nobody threw stays safe to
/// pick up.
///
/// Same rule as the bomb: arming reads [`ReleasedAs`] (the release decision),
/// not velocity, so falling does not arm it. Disarming is here too, chained
/// before the ticker, so a caught grenade cannot detonate in custody.
pub fn arm_thrown_gravity_grenades(
    mut commands: Commands,
    grenades: Query<(
        Entity,
        &GroundItem,
        Option<&ReleasedAs>,
        Has<GravityGrenadeFuse>,
    )>,
) {
    for (entity, ground, released, armed) in &grenades {
        if ground.spec.id != GRAVITY_GRENADE_ID {
            continue;
        }
        let thrown = matches!(released, Some(ReleasedAs(Release::Throw)));
        match (thrown, armed) {
            (true, false) => {
                commands.entity(entity).insert(GravityGrenadeFuse {
                    timer: GRAVITY_GRENADE_FUSE_SECS,
                });
            }
            (false, true) => {
                commands.entity(entity).remove::<GravityGrenadeFuse>();
            }
            _ => {}
        }
    }
}

/// Open one temporary up-gravity well. The only way a grenade's well enters
/// the world.
///
/// One place, like `deploy_sentry` and `open_vortex_well`, so tests can build
/// the real entity; it otherwise exists only after a fuse burns down.
///
/// Not the same kind of thing as an authored gravity column. That is room
/// geometry rebuilt on room load; this one is spawned mid-match, counts
/// `remaining` down, and despawns itself. So `TemporaryZone` is the rollback
/// anchor (see the shared-tangle registration for why `GravityZone` is not).
///
/// `sim_id` is the well's identity, minted under the grenade that burned
/// down (`SimId::spawned(grenade, counter)`); `None` only for fixtures. An
/// anchored entity without an identity rewinds anonymously (S4).
pub fn open_temporary_gravity_well(
    commands: &mut Commands,
    scope: SessionSpawnScope,
    center: ae::Vec2,
    sim_id: Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
) -> Entity {
    let mut spawned = commands.spawn_session_scoped(
        scope,
        (
            GravityZone {
                aabb: ae::Aabb::new(center, WELL_HALF),
                dir: ae::Vec2::new(0.0, -1.0), // up
            },
            TemporaryZone {
                remaining: WELL_DURATION_SECS,
            },
            Name::new("Gravity well (grenade)"),
        ),
    );
    if let Some(sim_id) = sim_id {
        spawned.insert(sim_id);
    }
    spawned.id()
}

/// Burn fuses; on expiry open a temporary up-gravity well at the grenade and
/// despawn it.
pub fn tick_gravity_grenade_fuses(
    time: Res<ambition_time::WorldTime>,
    mut commands: Commands,
    mut grenades: Query<(
        Entity,
        &GroundItem,
        &mut GravityGrenadeFuse,
        Option<&SessionScopedEntity>,
        // The grenade's identity and counter: its well is minted under it.
        Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
        Option<&mut ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>,
    )>,
    mut sfx: ambition_sfx::BodySfxWriter,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for (entity, ground, mut fuse, owner, grenade_id, mut counter) in &mut grenades {
        fuse.timer -= dt;
        if fuse.timer > 0.0 {
            continue;
        }
        // Refuse to open an unnameable well (ADR 0030). The grenade still
        // despawns: its fuse has expired, and skipping cleanup would retry
        // every tick forever.
        let (Some(grenade), Some(counter)) = (grenade_id, counter.as_deref_mut()) else {
            warn!(
                "a gravity grenade's well was refused: the grenade carries no \
                 SimId or no SimIdCounter, so the well could not be named"
            );
            commands.entity(entity).despawn();
            continue;
        };
        let well_id = Some(ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(
            grenade,
            counter.next(),
        ));
        open_temporary_gravity_well(
            &mut commands,
            SessionSpawnScope::new(owner.map(|owner| owner.0)),
            ground.pos,
            well_id,
        );
        sfx.write_for(
            entity,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_POWERUP,
                pos: ground.pos,
            },
        );
        vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
            pos: ground.pos,
            fx: ambition_vfx::fx::ids::CLASSIC_BURST,
            scale: 0.7,
            pose: ambition_vfx::FxPose::UPRIGHT,
        });
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grenade_ground(vel: ae::Vec2) -> GroundItem {
        GroundItem::released(
            ambition_characters::brain::held_item_by_id(GRAVITY_GRENADE_ID).unwrap(),
            ae::Vec2::new(100.0, 100.0),
            vel,
            ae::Vec2::splat(16.0),
        )
    }

    /// Arming reads the release, not the velocity.
    #[test]
    fn a_thrown_grenade_arms_but_a_grenade_nobody_threw_does_not() {
        let mut app = App::new();
        app.add_systems(Update, arm_thrown_gravity_grenades);
        let thrown = app
            .world_mut()
            .spawn((
                grenade_ground(ae::Vec2::new(60.0, -200.0)),
                ReleasedAs(Release::Throw),
            ))
            .id();
        // Falling, not thrown: the velocity of a second of free fall.
        let falling = app
            .world_mut()
            .spawn(grenade_ground(ae::Vec2::new(0.0, 980.0)))
            .id();
        app.update();
        assert!(
            app.world().get::<GravityGrenadeFuse>(thrown).is_some(),
            "a thrown grenade arms",
        );
        assert!(
            app.world().get::<GravityGrenadeFuse>(falling).is_none(),
            "a grenade armed itself by falling — gravity is being read as a throw",
        );
    }

    /// Catching one puts the fuse out: taking custody removes `ReleasedAs`,
    /// and the arming system (chained before the ticker) removes the fuse.
    #[test]
    fn catching_a_live_grenade_puts_the_fuse_out() {
        let mut app = App::new();
        app.add_systems(Update, arm_thrown_gravity_grenades);
        let caught = app
            .world_mut()
            .spawn((
                grenade_ground(ae::Vec2::ZERO),
                GravityGrenadeFuse { timer: 0.01 },
            ))
            .id();
        app.update();
        assert!(
            app.world().get::<GravityGrenadeFuse>(caught).is_none(),
            "the fuse survived the catch, so it is still counting down in custody",
        );
    }

    #[test]
    fn fuse_expiry_opens_a_temporary_up_well_and_despawns() {
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<ambition_vfx::vfx::VfxMessage>();
        let mut wt = ambition_time::WorldTime::default();
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
                // The well mints under the grenade, so a grenade with no
                // identity opens nothing (ADR 0030). Production grenades
                // always have one.
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement("test_grenade"),
                ambition_platformer2d_shared_tangle::sim_id::SimIdCounter::default(),
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
