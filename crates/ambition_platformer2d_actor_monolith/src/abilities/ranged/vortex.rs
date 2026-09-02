//! Vortex — a player-wielded crowd-control gauntlet: fire a singularity at a
//! point and it drags nearby enemies toward it for a moment. Distinct from
//! every other wielded attack (which deal damage / teleport): the vortex deals
//! *no* damage — it gathers a scattered group so the player can follow up
//! with an AOE (`crate::abilities::ranged::shockwave` / `crate::abilities::ranged::beam`) or a volley. Pull-then-slam.
//!
//! Distinct from the gravity grenade too: that spawns a *directional*
//! `GravityZone` (up-lift); the vortex is a *point* attractor — it lerps each
//! enemy's position toward the singularity center, clamped by the normal
//! collision step (`step_motion` resolves any wall the pull pushes into).
//! Bosses share the unified `BodyKinematics` now, but the faction guard below
//! (`ActorFaction::Boss != Enemy`) keeps them immune; only grounded/aerial mobs
//! (and peaceful NPCs, harmlessly) match the `Enemy` faction and get pulled.

use ambition_characters::control::ActorControl;
use bevy::prelude::*;

use ambition_combat::held_items::HeldItem;
use ambition_combat::components::ActorFaction;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::body_clusters::BodyKinematics;
use ambition_platformer2d_core::BodyMana;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_platformer2d_shared_tangle::lifecycle::{
    SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

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
    // ⭐ EVERY DRIVEN BODY, not the one the primary seat happens to hold.
    // `ControlledSubject` is singular by construction, so a possessed body or a
    // second seat holding the same item simply never fired.
    driven: crate::items::pickup::DrivenBodies,
    mut bodies: Query<(
        &ActorControl,
        &BodyKinematics,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        &HeldItem,
        &mut BodyMana,
        Option<&SessionScopedEntity>,
        // The caster's identity and its own mint stream. `Option` because a
        // fixture body carries neither; production bodies are named by
        // `ensure_sim_id` or by their spawn site.
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
        if !mana.meter.try_spend(VORTEX_MANA_COST) {
            continue;
        }
        // The body's per-tick resolved frame (ADR 0024 frame law).
        let gravity_dir = resolved_frame.down();
        let aim = crate::items::pickup::ability_aim_world(&c, kin.facing, gravity_dir)
            .normalize_or_zero();
        if aim == ae::Vec2::ZERO {
            continue;
        }
        let center = kin.pos + aim * VORTEX_RANGE;
        // `SimId::spawned(caster, counter.next())` — N3.1's rule for a dynamically
        // spawned sim entity. The counter lives on the CASTER so two casters never
        // share a stream; taking a number is itself snapshot state.
        let id = match (caster_id, caster_counter.as_mut()) {
            (Some(caster), Some(counter)) => Some(
                ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(caster, counter.next()),
            ),
            _ => None,
        };
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

/// Open one singularity. THE seam a vortex well comes into the world through.
///
/// ⭐ ONE PLACE, for the same reason `deploy_sentry` is one place — and for a
/// second reason the sentry taught: an archetype spawned only from inside a
/// system that needs a held gauntlet, spent mana and an aim vector is an
/// archetype no coverage sweep can reach, so the state it carries is registered
/// on trust. A named seam is what lets a test bring the entity into a booted
/// world the way production does.
///
/// ⭐ `id` IS `Option` AND THAT IS NOT A HEDGE. A well minted under a caster the
/// sim can name gets `SimId::spawned`; a fixture well has no caster to mint
/// under. Which one it is never decides the ORDER — see [`update_vortex_wells`],
/// whose sort reads the well's own state first and identity only as the final
/// tie-break, so an unidentified population is still composed repeatably.
///
/// ⛔⛔ ITS `remaining_s` IS AUTHORITATIVE SIMULATION STATE. The well pulls every
/// body in radius for as long as it counts down, so a rewind that kept a
/// future's well keeps a pull the authoritative timeline never applied. Both the
/// component and the entity anchor are declared in the actor crate's
/// `register_rollback_state`.
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
/// toward its center (a position lerp; the actor's own `step_motion` next
/// tick resolves any wall it's pulled into), then age the wells out. Runs on
/// `scaled_dt` so bullet-time slows the gather with everything else.
///
/// ⛔⛔ TWO OVERLAPPING WELLS DO NOT COMMUTE, and this composed them in Bevy
/// query order. Each well lerps the body a fraction `f` of the way to its own
/// centre, so applying A then B lands `f²·(B−A)` away from B then A. At 60 Hz
/// `f ≈ 0.08`, so two wells 200px apart differ by ~1.28px in ONE TICK from the
/// iteration order alone — three orders of magnitude past a rounding difference,
/// and archetype order is exactly what a resimulated tick can present
/// differently. Two live wells is an ordinary arrangement: they last 0.9s and
/// nothing stops a caster opening a second.
///
/// ⚠ THE ANCHOR AND CODEC DID NOT FIX THIS. Registering the well made it REWIND
/// correctly; it said nothing about the order two rewound wells are applied in.
/// Those are three separate facts — what bytes rewind, whether the entity
/// rewinds, and whether several of them compose the same way twice.
///
/// ⭐ THE ORDER IS THE WELL'S OWN STATE, THEN ITS IDENTITY. Centre and remaining
/// life determine the transformation completely, so two wells that tie on them
/// ARE the same pull and may be applied either way round; identity is the final
/// tie-break for the pair that somehow ties on both. Reading state first is what
/// keeps an unidentified fixture population repeatable too.
pub fn update_vortex_wells(
    world_time: Res<ambition_time::WorldTime>,
    mut commands: Commands,
    mut wells: Query<(Entity, &mut VortexWell)>,
    // The final tie-break's authority, read separately so a well with no id
    // still competes — it just cannot break a tie WITH one.
    ids: Query<&SimId>,
    mut actors: Query<
        (
            &mut BodyKinematics,
            &ActorFaction,
            Option<&ambition_characters::actor::BodyHealth>,
            // The world's hands are off this body — it is not a target either.
            bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
            // Whether a participant is driving this body, which is what decides
            // its EFFECTIVE side. See the filter below.
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
        for (mut kin, faction, health, out_of_play, driver) in &mut actors {
            // ⛔⛔ THE EFFECTIVE FACTION, NOT THE AUTHORED ONE. Possession keeps a
            // possessed NPC's `ActorFaction::Enemy` on purpose and moves its
            // allegiance through the driving relationship, so a raw `!= Enemy`
            // test had the well dragging the body the player is currently
            // driving into its own singularity.
            //
            // ⚠ Deliberately not widened past the `Enemy` class — see the same
            // note on `update_sentries`. This repair reads the allegiance from
            // the right field; which classes a vortex engages is a separate
            // design question.
            //
            // Structural tangibility gate: a dead enemy is an intangible corpse
            // — the well does not drag it.
            if ambition_combat::targeting::effective_faction(*faction, driver)
                != ActorFaction::Enemy
                || ambition_combat::util::body_is_untouchable(health, out_of_play)
            {
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

    /// ⛔⛔ SAME ALLEGIANCE DEFECT AS THE SENTRY. A possessed NPC keeps
    /// `ActorFaction::Enemy` on purpose — `targeting::effective_faction` is how
    /// possession moves the side without a faction overwrite/restore path — so a
    /// raw `!= Enemy` test had the well dragging the body a participant is
    /// currently driving into its own singularity.
    #[test]
    fn a_well_does_not_drag_the_body_a_player_is_driving() {
        use ambition_characters::control::{DrivingParticipant, PlayerSlot};

        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, VORTEX_ID);
        // Authored Enemy, but DRIVEN — so effectively on the player's side.
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
    fn vortex_does_not_pull_a_dead_enemy() {
        // A dead enemy is an intangible corpse: the well must not drag it.
        // (Enemies die and linger with a body, so this is reachable.) Poison:
        // drop the `body_is_corpse` skip in `update_vortex_wells` and the corpse
        // is pulled toward the singularity.
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, VORTEX_ID);
        // A DEAD enemy just inside the radius (well spawns at 300,100).
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

    /// ⛔⛔ TWO WELLS DO NOT COMMUTE, AND THE ORDER WAS THE ARCHETYPE'S. Each
    /// well lerps the body a fraction `f` toward its OWN centre, so A-then-B ends
    /// `f²·(B−A)` away from B-then-A. This arm reverses only the SPAWN order —
    /// same wells, same enemy, same tick — and reads where the body finished.
    ///
    /// ⚠ THE ANCHOR AND CODEC ADDED LAST PASS DO NOT COVER THIS. They make a well
    /// rewind; they say nothing about how two rewound wells compose.
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

    /// ⭐⭐ A SECOND DRIVEN BODY OPENS ITS OWN WELL.
    /// Same singular-`ControlledSubject` defect as the volley: this ability read
    /// one entity, so a couch's second seat could not cast at all.
    #[test]
    fn two_driven_bodies_each_open_their_own_well() {
        use crate::abilities::test_support::spawn_seated_body_holding;
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

        // Each well opens VORTEX_RANGE ahead of ITS OWN caster, so the centres
        // are what says two bodies cast — not one body casting twice.
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
