//! Vortex: a player-wielded crowd-control gauntlet. It fires a singularity at
//! a point that drags nearby enemies toward it for a moment. It deals no
//! damage; it gathers a group for a follow-up AOE (`crate::ranged::shockwave`
//! / `crate::ranged::beam`) or a volley.
//!
//! Unlike the gravity grenade's directional `GravityZone`, the vortex is a
//! point attractor: it lerps each enemy's position toward the center, and the
//! normal collision step (`step_motion`) resolves any wall the pull pushes
//! into. The faction guard (`ActorFaction::Boss != Enemy`) keeps bosses immune;
//! only `Enemy`-faction bodies are pulled.

use ambition_characters::control::ActorControl;
use bevy::prelude::*;

use ambition_combat::held_items::HeldItem;
use ambition_combat::components::ActorFaction;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::body_clusters::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_platformer2d_shared_tangle::lifecycle::{
    SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// Held-item id of the vortex gauntlet.
pub const VORTEX_ID: &str = "vortex";

/// Mana per cast (out of 100).
const VORTEX_MANA_COST: f32 = 22.0;

/// How far in front of the player (along aim) the singularity spawns.
const VORTEX_RANGE: f32 = 200.0;
/// Radius (px) within which enemies get dragged toward the center.
const VORTEX_RADIUS: f32 = 220.0;
/// Pull rate (1/s): the fraction of the remaining gap closed per second
/// (`lerp` factor `rate * dt`). Higher gathers faster.
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

/// `Attack` while holding the vortex gauntlet spawns a [`VortexWell`] ahead of
/// the player along the aim. Plain Attack only; `Shield + Attack` drops the
/// item (the id is `UseSystem`, excluded from throw-on-plain-Attack).
pub fn fire_vortex_system(
    // Every driven body, not only the primary seat's `ControlledSubject`, so a
    // possessed body or a second seat can cast.
    driven: ambition_held_items::DrivenBodies,
    mut bodies: Query<(
        &ActorControl,
        &BodyKinematics,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        &HeldItem,
        Option<&mut ambition_platformer2d_core::resources::ActorResources>,
        Option<&SessionScopedEntity>,
        // The caster's identity and mint stream. `Option` because fixtures
        // carry neither; production bodies get them from `ensure_sim_id` or
        // their spawn site.
        Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
        Option<&mut ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>,
    )>,
    mut commands: Commands,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    for subject in driven.entities() {
        let Ok((
            control,
            kin,
            resolved_frame,
            held,
            mut mana,
            owner,
            caster_id,
            mut caster_counter,
        )) = bodies.get_mut(subject)
        else {
            continue;
        };
        let c = control.0;
        if !c.melee_pressed || c.shield_held {
            continue;
        }
        if held.spec.id != VORTEX_ID {
            continue;
        }
        // Refuse before spending (ADR 0030): a refusal after `try_spend`
        // would take mana and open nothing.
        let (Some(caster), Some(counter)) = (caster_id, caster_counter.as_mut()) else {
            warn!(
                "a vortex cast was refused: the caster carries no SimId or no \
                 SimIdCounter, so the well could not be named"
            );
            continue;
        };
        // N3.1: a dynamically spawned sim entity is `SimId::spawned(caster,
        // counter.next())`. The counter lives on the caster, so casters never
        // share a stream; taking a number is snapshot state.
        let id = Some(ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(
            caster,
            counter.next(),
        ));
        if !crate::mana::spend(mana.as_deref_mut(), VORTEX_MANA_COST) {
            continue;
        }
        // The body's per-tick resolved frame (ADR 0024 frame law).
        let gravity_dir = resolved_frame.down();
        let aim = ambition_held_items::ability_aim_world(&c, kin.facing, gravity_dir)
            .normalize_or_zero();
        if aim == ae::Vec2::ZERO {
            continue;
        }
        let center = kin.pos + aim * VORTEX_RANGE;
        open_vortex_well(
            &mut commands,
            SessionSpawnScope::new(owner.map(|owner| owner.0)),
            center,
            id,
        );
        sfx.write_for(
            subject,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::PLAYER_BLINK,
                pos: center,
            },
        );
    }
}

/// Open one singularity. The only way a vortex well enters the world.
///
/// One place, like `deploy_sentry`, so tests can spawn the entity the way
/// production does. An archetype spawned only inside a system that needs a
/// held gauntlet, mana, and an aim would be unreachable by coverage sweeps.
///
/// `id` is `Option`: a well cast by a named caster gets `SimId::spawned`; a
/// fixture well has no caster. It never decides the order (see
/// [`update_vortex_wells`]).
///
/// `remaining_s` is authoritative simulation state: the well pulls every body
/// in radius while it counts down. The component and entity anchor are
/// declared in the actor crate's `register_rollback_state`.
pub fn open_vortex_well(
    commands: &mut Commands,
    scope: SessionSpawnScope,
    center: ae::Vec2,
    id: Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
) -> Entity {
    let mut well = commands.spawn_session_scoped(
        scope,
        (
            VortexWell {
                center,
                remaining_s: VORTEX_LIFETIME_S,
            },
            Name::new("Vortex singularity"),
        ),
    );
    if let Some(id) = id {
        well.insert(id);
    }
    well.id()
}

/// Drag every Enemy-faction actor within [`VORTEX_RADIUS`] of each live well
/// toward its center (a position lerp; the actor's `step_motion` next tick
/// resolves walls), then age the wells out. Runs on `scaled_dt`, so
/// bullet-time slows the gather.
///
/// Overlapping wells do not commute. Each well lerps a fraction `f` toward
/// its own center, so A-then-B ends `f²·(B−A)` away from B-then-A (about
/// 1.3px per tick for wells 200px apart at 60 Hz). Rollback registration does
/// not fix the order, so wells are sorted by their own state (center,
/// remaining life), then by identity. Wells that tie on state are the same
/// pull, and fixtures without ids stay repeatable.
pub fn update_vortex_wells(
    world_time: Res<ambition_time::WorldTime>,
    mut commands: Commands,
    mut wells: Query<(Entity, &mut VortexWell)>,
    // Tie-break authority, read separately so a well with no id still
    // applies.
    ids: Query<&SimId>,
    mut actors: Query<
        (
            &mut BodyKinematics,
            Option<&mut ae::SweepSample>,
            &ActorFaction,
            Option<&ambition_characters::actor::BodyHealth>,
            // A body out of play is not a target.
            bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
            // Whether a participant drives this body, which decides its
            // effective side. See the filter below.
            Option<&ambition_characters::control::DrivingParticipant>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let dt = world_time.scaled_dt;
    if dt <= 0.0 {
        return;
    }
    let factor = (VORTEX_PULL_RATE * dt).min(1.0);
    let mut order: Vec<(ae::Vec2, f32, Option<SimId>, Entity)> = wells
        .iter()
        .map(|(entity, well)| {
            (
                well.center,
                well.remaining_s,
                ids.get(entity).ok().cloned(),
                entity,
            )
        })
        .collect();
    order.sort_by(|a, b| {
        a.0.x
            .total_cmp(&b.0.x)
            .then_with(|| a.0.y.total_cmp(&b.0.y))
            .then_with(|| a.1.total_cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    for (_, _, _, entity) in order {
        let Ok((entity, mut well)) = wells.get_mut(entity) else {
            continue;
        };
        for (mut kin, mut sweep, faction, health, out_of_play, driver) in &mut actors {
            // Use the effective faction, not the authored one: a possessed NPC
            // keeps `ActorFaction::Enemy` and moves its side through the
            // driver, so the authored field would pull the player's own body.
            // Not widened past the `Enemy` class (see `update_sentries`).
            // A dead enemy is an intangible corpse; the well does not drag it.
            if ambition_combat::targeting::effective_faction(*faction, driver)
                != ActorFaction::Enemy
                || ambition_combat::util::body_is_untouchable(health, out_of_play)
            {
                continue;
            }
            if kin.pos.distance(well.center) <= VORTEX_RADIUS {
                // The well is an external kinematic constraint (ADR 0024): it
                // moves the body toward the center by this tick's pull delta.
                let delta = kin.pos.lerp(well.center, factor) - kin.pos;
                ae::movement::carry_body(&mut kin, sweep.as_deref_mut(), delta);
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
    use crate::test_support::spawn_primary_player_holding;

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

    /// Same allegiance rule as the sentry: a possessed NPC keeps
    /// `ActorFaction::Enemy` (`targeting::effective_faction` gives its side),
    /// so the well must not drag the body a participant drives.
    #[test]
    fn a_well_does_not_drag_the_body_a_player_is_driving() {
        use ambition_characters::control::{DrivingParticipant, PlayerSlot};

        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, VORTEX_ID);
        // Authored Enemy, but driven, so effectively on the player's side.
        let driven = spawn_enemy(&mut app, ae::Vec2::new(420.0, 100.0));
        app.world_mut()
            .entity_mut(driven)
            .insert(DrivingParticipant(PlayerSlot(1)));
        let start = ae::Vec2::new(420.0, 100.0);

        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        for _ in 0..10 {
            app.update();
        }

        assert_eq!(
            app.world().get::<BodyKinematics>(driven).unwrap().pos,
            start,
            "the well pulled a body a second participant is driving — its \
             authored `Enemy` is not the side it is fighting on"
        );
    }

    #[test]
    fn attack_with_vortex_spawns_a_well_and_pulls_a_nearby_enemy_inward() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, VORTEX_ID);
        // Player at (100,100), facing +x, so the well is at (300,100). An
        // enemy just inside the radius should be dragged toward the center.
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
    fn vortex_does_not_pull_a_dead_enemy() {
        // A dead enemy is an intangible corpse: the well must not drag it
        // (dead enemies linger). Removing the `body_is_untouchable` skip in
        // `update_vortex_wells` makes this fail.
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, VORTEX_ID);
        // A dead enemy just inside the radius (well spawns at 300,100).
        let corpse = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                BodyKinematics {
                    pos: ae::Vec2::new(420.0, 100.0),
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                ActorFaction::Enemy,
                ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health {
                    current: 0,
                    max: 3,
                    invulnerable: Default::default(),
                }),
            ))
            .id();
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        let pos = app.world().get::<BodyKinematics>(corpse).unwrap().pos;
        assert_eq!(
            pos,
            ae::Vec2::new(420.0, 100.0),
            "a dead enemy corpse must not be pulled by the vortex"
        );
    }

    /// Two wells do not commute: A-then-B ends `f²·(B−A)` away from
    /// B-then-A. This test reverses only the spawn order (same wells, enemy,
    /// and tick) and compares where the body ends.
    #[test]
    fn two_overlapping_wells_pull_a_body_to_the_same_place_in_either_order() {
        fn resting_place(order: [ae::Vec2; 2]) -> ae::Vec2 {
            let mut app = App::new();
            app.add_message::<ambition_sfx::OwnedSfxMessage>();
            app.insert_resource(ambition_time::WorldTime {
                raw_dt: 0.016,
                scaled_dt: 0.016,
            });
            app.add_systems(Update, update_vortex_wells);
            let enemy = spawn_enemy(&mut app, ae::Vec2::ZERO);
            for (n, center) in order.iter().enumerate() {
                let mut commands = app.world_mut().commands();
                open_vortex_well(
                    &mut commands,
                    ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
                    *center,
                    Some(SimId::spawned(&SimId::player_slot(0), n as u64)),
                );
                app.world_mut().flush();
            }
            app.update();
            app.world().get::<BodyKinematics>(enemy).unwrap().pos
        }

        let a = ae::Vec2::new(200.0, 0.0);
        let b = ae::Vec2::new(0.0, 200.0);
        let forwards = resting_place([a, b]);
        let backwards = resting_place([b, a]);
        assert_eq!(
            forwards, backwards,
            "reversing the order two wells were opened in moved the body they \
             both pull ({forwards:?} vs {backwards:?}) — a resimulated tick \
             composes the same two wells the other way round"
        );
    }

    #[test]
    fn vortex_ignores_a_far_enemy_and_expires() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, VORTEX_ID);
        // Far away (well at 300,100; enemy at 900, outside the 220 radius).
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
        // Age it out: lifetime 0.9s at 0.016/tick is about 57 ticks.
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

    /// A second driven body opens its own well; a couch's second seat can
    /// cast.
    #[test]
    fn two_driven_bodies_each_open_their_own_well() {
        use crate::test_support::spawn_seated_body_holding;
        let mut app = test_app();
        app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));
        let a = spawn_seated_body_holding(
            &mut app,
            VORTEX_ID,
            0,
            "seat_a",
            ae::Vec2::new(100.0, 100.0),
        );
        let b = spawn_seated_body_holding(
            &mut app,
            VORTEX_ID,
            1,
            "seat_b",
            ae::Vec2::new(900.0, 100.0),
        );
        for body in [a, b] {
            app.world_mut()
                .get_mut::<ActorControl>(body)
                .unwrap()
                .0
                .melee_pressed = true;
        }
        app.update();

        // Each well opens VORTEX_RANGE ahead of its own caster, so distinct
        // centers show two bodies cast.
        let centers: Vec<f32> = app
            .world_mut()
            .query::<&VortexWell>()
            .iter(app.world())
            .map(|well| well.center.x)
            .collect();
        assert_eq!(
            centers.len(),
            2,
            "one well per casting seat; got {centers:?}"
        );
        assert!(
            centers.iter().any(|&x| x < 500.0) && centers.iter().any(|&x| x > 900.0),
            "each well should open ahead of its OWN caster; got {centers:?}"
        );
    }
}
