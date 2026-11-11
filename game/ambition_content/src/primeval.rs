//! Swallows fetch coconuts from the primeval trail and release them over prey.

use ambition_characters::control::{ActorControl, DrivingParticipant};
use ambition_combat::{components::ActorTarget, hand::RepertoireQuery, held_items::HeldItem};
use ambition_content_schemas::rules::SwallowFetch;
use ambition_held_items::{
    release_carried_item, take_ground_item, GroundItem, ItemCustody, Release, SettledItem,
    SupportedCarry,
};
use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
use ambition_platformer2d_shared_tangle::schedule::{
    ActorDecisionSet, GameplayGated, PlayerInputSet, SimScheduleExt,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;
use std::collections::BTreeSet;

pub const COCONUT: &str = "coconut";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Laden {
    Carry(ae::Vec2),
    LetGo,
}

/// Steer the load above the predicted target position before release.
pub fn laden(
    fetch: &SwallowFetch,
    load: ae::Vec2,
    target: ae::Vec2,
    target_vel: ae::Vec2,
) -> Laden {
    let over = ae::Vec2::new(
        target.x + target_vel.x * fetch.lead_seconds,
        target.y - fetch.carry_over,
    );
    if (load.x - over.x).abs() <= fetch.let_go_within && target.y - load.y >= fetch.let_go_over {
        return Laden::LetGo;
    }
    Laden::Carry((over - load).normalize_or_zero() * fetch.laden_speed)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum FetchPhase {
    #[default]
    Approach,
    Strain,
    Wait,
    Depart,
}

/// A committed coconut and a bounded attempt. Stable identity survives restore.
#[derive(Component, Clone, Debug, Default)]
struct FetchFlight {
    goal: Option<String>,
    phase: FetchPhase,
    elapsed: f32,
}

impl FetchFlight {
    fn depart(&mut self) {
        self.phase = FetchPhase::Depart;
        self.elapsed = 0.0;
    }

    fn checksum(&self) -> u64 {
        let mut sum = 0xcbf29ce484222325u64;
        for byte in self
            .goal
            .as_deref()
            .unwrap_or("")
            .bytes()
            .chain([self.phase as u8])
            .chain(self.elapsed.to_bits().to_le_bytes())
        {
            sum = (sum ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
        sum
    }
}

struct Bird {
    entity: Entity,
    id: SimId,
    kin: ae::BodyKinematics,
    capacity: f32,
    available: bool,
    alive: bool,
    held: Option<String>,
    target: Option<Entity>,
    support: Option<SupportedCarry>,
    flight: FetchFlight,
}

struct Nut {
    entity: Entity,
    id: String,
    pos: ae::Vec2,
}

fn flight_control(control: &mut ActorControl, velocity: ae::Vec2) {
    control.0.velocity_target = ae::WorldVec2(velocity);
    control.0.melee_pressed = false;
    control.0.melee_held = false;
    control.0.fire = None;
}

/// Intent uses the normal control gate. Custody and release use the item domain.
#[allow(clippy::type_complexity)]
fn swallows_carry_coconuts(
    mut commands: Commands,
    bodies: Query<(
        Entity,
        &WornCharacter,
        &BodyHealth,
        &ae::BodyKinematics,
        &SimId,
        Option<&ActorTarget>,
        Option<&HeldItem>,
        Option<&SupportedCarry>,
        Option<&FetchFlight>,
        Has<DrivingParticipant>,
    )>,
    mut controls: Query<(&mut ActorControl, RepertoireQuery)>,
    mut items: Query<(Entity, &GroundItem, &ae::BodyKinematics, &mut ItemCustody, &ambition_held_items::HeldMaterialBasis)>,
    release_world: ambition_held_items::ItemReleaseWorld,
    at_rest: Query<(), With<SettledItem>>,
    ids: Query<&SimId>,
    hunted: Query<&ae::BodyKinematics>,
    rooms: LiveRooms,
    time: Res<ambition_time::WorldTime>,
    tuning: Res<crate::tuning::AmbitionTuning>,
) {
    let fetch = &tuning.0.swallow_fetch;
    let dt = time.sim_dt();
    let mut birds: Vec<_> = bodies
        .iter()
        .filter_map(
            |(entity, worn, health, kin, id, target, held, support, flight, driven)| {
                Some(Bird {
                    entity,
                    id: id.clone(),
                    kin: *kin,
                    capacity: *fetch.capacity.get(worn.id())?,
                    available: health.alive() && !driven,
                    alive: health.alive(),
                    held: held.map(|held| held.id().to_owned()),
                    target: target.and_then(|target| target.entity),
                    support: support.copied(),
                    flight: flight.cloned().unwrap_or_default(),
                })
            },
        )
        .collect();
    birds.sort_by(|a, b| a.id.cmp(&b.id));
    let nuts: Vec<_> = items
        .iter()
        .filter_map(|(entity, ground, item_body, custody, _)| {
            (ground.spec.id == COCONUT && custody.in_world() && at_rest.contains(entity))
                .then(|| {
                    ids.get(entity).ok().map(|id| Nut {
                        entity,
                        id: id.as_str().to_owned(),
                        pos: item_body.pos,
                    })
                })
                .flatten()
        })
        .collect();
    let mut busy = BTreeSet::new();
    let mut taken = BTreeSet::new();

    // An existing pair makes one decision for both control frames.
    for i in 0..birds.len() {
        let Some(link) = birds[i].support else {
            continue;
        };
        let leader = birds[i].entity;
        let helper = birds.iter().position(|bird| bird.entity == link.supporter);
        busy.insert(leader);
        busy.insert(link.supporter);
        let anchor = helper
            .filter(|j| rooms.of(leader) == rooms.of(birds[*j].entity))
            .map_or(
                birds[i].kin.pos + ae::Vec2::Y * (birds[i].kin.size().y * 0.5 + 8.0),
                |j| SupportedCarry::anchor(&birds[i].kin, &birds[j].kin, ae::Vec2::Y),
            );
        let intact = helper.is_some_and(|j| {
            birds[i].available
                && birds[j].available
                && birds[i].held.as_deref() == Some(COCONUT)
                && birds[j].held.is_none()
                && birds[i].capacity + birds[j].capacity >= 1.0
                && rooms.of(leader) == rooms.of(link.supporter)
                && birds[i].kin.pos.distance(birds[j].kin.pos)
                    <= fetch.half_span * 4.0 + fetch.join_radius
        });
        let prey = birds[i].target.and_then(|target| hunted.get(target).ok());
        let decision = prey.map_or(Laden::Carry(ae::Vec2::ZERO), |prey| {
            laden(fetch, anchor, prey.pos, prey.vel)
        });
        if !intact || decision == Laden::LetGo {
            if let Ok((_, mut repertoire)) = controls.get_mut(leader) {
                match release_carried_item(
                    &mut commands,
                    leader,
                    &mut repertoire,
                    &mut items,
                    &birds[i].kin,
                    &release_world,
                    anchor,
                    ae::Vec2::new(birds[i].kin.vel.x * 0.25, 0.0),
                    if intact {
                        Release::Throw
                    } else {
                        Release::Drop
                    },
                ) {
                    Err(refusal) => { warn!(?refusal, "supported item release refused"); continue; }
                    Ok(false) => ambition_held_items::empty_hand(&mut commands, leader, &mut repertoire),
                    Ok(true) => {}
                }
            }
            commands.entity(leader).remove::<SupportedCarry>();
            birds[i].flight.depart();
            if let Some(j) = helper {
                birds[j].flight.depart();
            }
        } else if let (Some(j), Laden::Carry(velocity)) = (helper, decision) {
            let centre = (birds[i].kin.pos + birds[j].kin.pos) * 0.5;
            for (index, side) in [(i, -1.0), (j, 1.0)] {
                let formation = centre + ae::Vec2::X * (side * fetch.half_span);
                let corrected =
                    velocity + (formation - birds[index].kin.pos) * fetch.formation_gain;
                if let Ok((mut control, _)) = controls.get_mut(birds[index].entity) {
                    flight_control(&mut control, corrected.clamp_length_max(fetch.fetch_speed));
                }
            }
        }
    }

    // Commit nearby arrivals to the same load before either can take custody.
    for i in 0..birds.len() {
        if busy.contains(&birds[i].entity) || !birds[i].available || birds[i].held.is_some() {
            continue;
        }
        birds[i].flight.elapsed += dt;
        if birds[i].flight.phase == FetchPhase::Depart {
            if birds[i].flight.elapsed < fetch.retry_seconds {
                continue;
            }
            birds[i].flight = FetchFlight::default();
        }
        let valid = |nut: &&Nut| {
            rooms.of(nut.entity) == rooms.of(birds[i].entity)
                && nut.pos.distance(birds[i].kin.pos) <= fetch.fetch_reach
        };
        let committed = nuts
            .iter()
            .filter(valid)
            .find(|nut| birds[i].flight.goal.as_deref() == Some(nut.id.as_str()));
        if committed.is_none() {
            let partner_goal = birds
                .iter()
                .filter(|other| {
                    other.entity != birds[i].entity
                        && other.available
                        && other.held.is_none()
                        && other.flight.phase != FetchPhase::Depart
                        && rooms.of(other.entity) == rooms.of(birds[i].entity)
                        && other.kin.pos.distance(birds[i].kin.pos) <= fetch.partner_reach
                })
                .find_map(|other| {
                    nuts.iter()
                        .filter(valid)
                        .find(|nut| other.flight.goal.as_deref() == Some(nut.id.as_str()))
                });
            let goal = partner_goal.or_else(|| {
                nuts.iter().filter(valid).min_by(|a, b| {
                    a.pos
                        .distance_squared(birds[i].kin.pos)
                        .total_cmp(&b.pos.distance_squared(birds[i].kin.pos))
                        .then(a.id.cmp(&b.id))
                })
            });
            birds[i].flight = FetchFlight {
                goal: goal.map(|nut| nut.id.clone()),
                ..default()
            };
        }
    }

    for i in 0..birds.len() {
        let bird = &birds[i];
        let entity = bird.entity;
        if busy.contains(&entity) {
            continue;
        }
        if bird.held.as_deref() == Some(COCONUT) {
            if bird.alive && !bird.available {
                continue;
            }
            let from = bird.kin.pos + ae::Vec2::new(0.0, bird.kin.size().y * 0.5 + 8.0);
            let decision = bird
                .target
                .and_then(|target| hunted.get(target).ok())
                .map(|prey| laden(fetch, from, prey.pos, prey.vel));
            if let Ok((mut control, mut repertoire)) = controls.get_mut(entity) {
                if !bird.alive || bird.capacity < 1.0 || decision == Some(Laden::LetGo) {
                    match release_carried_item(
                        &mut commands,
                        entity,
                        &mut repertoire,
                        &mut items,
                        &bird.kin,
                        &release_world,
                        from,
                        ae::Vec2::new(bird.kin.vel.x * 0.25, 0.0),
                        if bird.alive && bird.capacity >= 1.0 {
                            Release::Throw
                        } else {
                            Release::Drop
                        },
                    ) {
                        Err(refusal) => { warn!(?refusal, "item release refused"); continue; }
                        Ok(false) => ambition_held_items::empty_hand(&mut commands, entity, &mut repertoire),
                        Ok(true) => {}
                    }
                    birds[i].flight.depart();
                } else if let Some(Laden::Carry(velocity)) = decision {
                    flight_control(&mut control, velocity);
                }
            }
            continue;
        }
        if !bird.available || bird.held.is_some() {
            continue;
        }
        if bird.flight.phase == FetchPhase::Depart {
            if bird.flight.elapsed < fetch.attempt_seconds {
                let away = bird
                    .flight
                    .goal
                    .as_ref()
                    .and_then(|id| nuts.iter().find(|nut| &nut.id == id))
                    .map_or(1.0, |nut| {
                        if bird.kin.pos.x < nut.pos.x {
                            -1.0
                        } else {
                            1.0
                        }
                    });
                if let Ok((mut control, _)) = controls.get_mut(entity) {
                    flight_control(
                        &mut control,
                        ae::Vec2::new(away, -1.0).normalize() * fetch.fetch_speed,
                    );
                }
            }
            continue;
        }
        let Some(nut) = nuts.iter().find(|nut| {
            Some(nut.id.as_str()) == bird.flight.goal.as_deref() && !taken.contains(&nut.entity)
        }) else {
            continue;
        };
        let approach = nut.pos - ae::Vec2::Y * (bird.kin.size().y * 0.5 + 4.0);
        let arrived = bird.kin.pos.distance(approach) <= fetch.join_radius;
        let partner = birds
            .iter()
            .enumerate()
            .find(|(j, other)| {
                *j != i
                    && other.available
                    && other.held.is_none()
                    && !busy.contains(&other.entity)
                    && other.flight.phase != FetchPhase::Depart
                    && other.flight.goal == bird.flight.goal
                    && rooms.of(other.entity) == rooms.of(entity)
                    && other.kin.pos.distance(bird.kin.pos) <= fetch.partner_reach
                    && other.capacity + bird.capacity >= 1.0
            })
            .map(|(j, _)| j);
        let joined = partner.filter(|j| birds[*j].kin.pos.distance(approach) <= fetch.join_radius);
        if arrived && (bird.capacity >= 1.0 || joined.is_some()) {
            let Ok((_, mut repertoire)) = controls.get_mut(entity) else {
                continue;
            };
            let Ok((item, ground, item_body, mut custody, _)) = items.get_mut(nut.entity) else {
                continue;
            };
            if !custody.in_world() {
                continue;
            }
            take_ground_item(
                &mut commands,
                entity,
                &mut repertoire,
                item,
                ground,
                item_body,
                &mut custody,
                &bird.kin,
            );
            taken.insert(item);
            busy.insert(entity);
            if bird.capacity < 1.0 {
                let supporter = birds[joined.unwrap()].entity;
                commands.entity(entity).insert(SupportedCarry { supporter });
                busy.insert(supporter);
            }
            continue;
        }
        let mut aim = approach;
        if arrived && bird.capacity < 1.0 {
            match birds[i].flight.phase {
                FetchPhase::Approach => {
                    birds[i].flight.phase = FetchPhase::Strain;
                    birds[i].flight.elapsed = 0.0;
                }
                FetchPhase::Strain if birds[i].flight.elapsed >= fetch.attempt_seconds => {
                    if partner.is_some() {
                        birds[i].flight.phase = FetchPhase::Wait;
                        birds[i].flight.elapsed = 0.0;
                    } else {
                        birds[i].flight.depart();
                    }
                }
                FetchPhase::Wait
                    if partner.is_none() || birds[i].flight.elapsed >= fetch.wait_seconds =>
                {
                    birds[i].flight.depart()
                }
                _ => {}
            }
            if birds[i].flight.phase == FetchPhase::Strain {
                aim.y -= 8.0 * (birds[i].flight.elapsed * 12.0).sin().abs();
            }
        } else if birds[i].flight.elapsed
            > fetch.wait_seconds + fetch.fetch_reach / fetch.fetch_speed.max(1.0)
        {
            birds[i].flight.depart();
        }
        if let Ok((mut control, _)) = controls.get_mut(entity) {
            flight_control(
                &mut control,
                ((aim - birds[i].kin.pos) * fetch.formation_gain)
                    .clamp_length_max(fetch.fetch_speed),
            );
        }
    }
    for bird in birds {
        commands.entity(bird.entity).insert(bird.flight);
    }
}

pub fn register_rollback_state(
    registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar,
) {
    registrar.require_rollback::<FetchFlight>("ambition_content::primeval", "entity:swallow_fetch");
    registrar.rollback_component_clone_checksum::<FetchFlight>(
        "ambition_content::primeval",
        "content.swallow_fetch",
        "goal, phase, elapsed",
        FetchFlight::checksum,
    );
}

pub struct PrimevalPlugin;

impl Plugin for PrimevalPlugin {
    fn build(&self, app: &mut App) {
        use ambition_platformer2d::runtime::ScopedSystemsAppExt as _;
        register_rollback_state(
            &mut ambition_platformer2d_runtime::rollback::SchemaRollbackRegistrar::new(app),
        );
        let sim = SimScheduleExt::sim_schedule(app);
        app.add_scoped_systems(
            sim,
            crate::AMBITION_RULES,
            swallows_carry_coconuts
                .in_set(GameplayGated)
                .after(ActorDecisionSet::Publish)
                .before(PlayerInputSet::ControlGate),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (App, Entity, Entity) {
        use ambition_characters::brain::{HeldItemSpec, HeldUseBehavior};
        let mut app = App::new();
        app.insert_resource(crate::tuning::AmbitionTuning(crate::tuning::shipped()));
        app.insert_resource(ambition_time::WorldTime::new(1.0 / 60.0, 1.0 / 60.0));
        app.add_systems(Update, swallows_carry_coconuts);
        let bird = spawn_bird(&mut app, "first", ae::Vec2::new(100.0, 286.0));
        let nut = app
            .world_mut()
            .spawn((
                GroundItem::at_rest(
                    HeldItemSpec {
                        id: COCONUT.into(),
                        melee: None,
                        ranged: None,
                        use_behavior: HeldUseBehavior::Auto,
                    },
                    ae::Vec2::new(100.0, 300.0),
                    ae::Vec2::new(9.0, 7.5),
                ),
                SettledItem::default(),
                SimId::placement("nut"),
            ))
            .id();
        (app, bird, nut)
    }

    fn spawn_bird(app: &mut App, id: &str, pos: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                WornCharacter::new("european_swallow"),
                BodyHealth::new(ambition_characters::actor::Health::new(1)),
                ae::BodyKinematics {
                    pos,
                    canonical_size: ae::Vec2::new(24.0, 20.0),
                    ..default()
                },
                SimId::placement(id),
                ActorControl::default(),
                ambition_characters::brain::action_set::IdentityKit::default(),
                ambition_characters::brain::ActionSet::default(),
                ambition_combat::moveset::ActorMoveset::default(),
            ))
            .id()
    }

    #[test]
    fn a_lone_european_attempts_the_lift_then_leaves_the_coconut() {
        let (mut app, bird, nut) = fixture();
        app.update();
        assert_eq!(
            app.world().get::<FetchFlight>(bird).unwrap().phase,
            FetchPhase::Strain
        );
        for _ in 0..70 {
            app.update();
        }
        assert_eq!(
            app.world().get::<FetchFlight>(bird).unwrap().phase,
            FetchPhase::Depart
        );
        assert!(app.world().get::<ItemCustody>(nut).unwrap().in_world());
        assert_eq!(
            app.world().get::<ambition_platformer2d::engine_core::BodyKinematics>(nut).unwrap().pos,
            ae::Vec2::new(100.0, 300.0)
        );
        assert!(app.world().get::<HeldItem>(bird).is_none());
        assert!(
            app.world()
                .get::<ActorControl>(bird)
                .unwrap()
                .0
                .velocity_target
                .0
                .y
                < 0.0
        );
    }

    #[test]
    fn a_european_waits_for_an_arriving_partner_and_both_release_one_load() {
        let (mut app, first, nut) = fixture();
        let second = spawn_bird(&mut app, "second", ae::Vec2::new(350.0, 200.0));
        for _ in 0..65 {
            app.update();
        }
        assert_eq!(
            app.world().get::<FetchFlight>(first).unwrap().phase,
            FetchPhase::Wait
        );
        assert!(app.world().get::<ItemCustody>(nut).unwrap().in_world());
        app.world_mut()
            .get_mut::<ae::BodyKinematics>(second)
            .unwrap()
            .pos = ae::Vec2::new(115.0, 280.0);
        app.update();
        assert!(app.world().get::<ItemCustody>(nut).unwrap().held_by(first));
        assert_eq!(
            app.world().get::<SupportedCarry>(first).unwrap().supporter,
            second
        );
        assert!(app.world().get::<HeldItem>(second).is_none());
        let target = app
            .world_mut()
            .spawn(ae::BodyKinematics {
                pos: ae::Vec2::new(400.0, 400.0),
                ..default()
            })
            .id();
        app.world_mut().entity_mut(first).insert(ActorTarget {
            entity: Some(target),
            pos: ae::Vec2::new(400.0, 400.0),
        });
        app.update();
        for body in [first, second] {
            let control = app.world().get::<ActorControl>(body).unwrap();
            assert!(control.0.velocity_target.0.x > 0.0 && control.0.velocity_target.0.y < 0.0);
            assert!(!control.0.melee_held);
        }
        app.world_mut()
            .get_mut::<ae::BodyKinematics>(first)
            .unwrap()
            .pos = ae::Vec2::new(376.0, 220.0);
        app.world_mut()
            .get_mut::<ae::BodyKinematics>(second)
            .unwrap()
            .pos = ae::Vec2::new(424.0, 220.0);
        app.init_resource::<ambition_sprite_sheet::SpriteContent>();
        {
            let world = app.world_mut();
            let mut positions =
                bevy::ecs::system::SystemState::<ambition_held_items::ItemWorldPos>::new(world);
            let actual = positions
                .get(world)
                .expect("the shared position query is valid")
                .of(
                    world.get::<ItemCustody>(nut).unwrap(),
                    world.get::<ae::BodyKinematics>(nut).unwrap(),
                );
            assert_eq!(actual, ae::Vec2::new(400.0, 238.0));
            assert_eq!(
                world.get::<ambition_platformer2d::engine_core::BodyKinematics>(nut).unwrap().pos,
                ae::Vec2::new(100.0, 300.0)
            );
        }
        app.update();
        assert!(app.world().get::<ItemCustody>(nut).unwrap().in_world());
        assert_eq!(
            app.world().get::<ambition_platformer2d::engine_core::BodyKinematics>(nut).unwrap().pos,
            ae::Vec2::new(400.0, 238.0)
        );
        assert!(app.world().get::<SupportedCarry>(first).is_none());
        assert!(app.world().get::<HeldItem>(first).is_none());
        assert_eq!(
            app.world()
                .get::<ambition_held_items::ReleasedAs>(nut)
                .unwrap()
                .0,
            Release::Throw
        );
        assert_eq!(
            app.world().get::<FetchFlight>(second).unwrap().phase,
            FetchPhase::Depart
        );
    }

    #[test]
    fn a_lost_partner_releases_the_existing_item() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
        for loss in ["missing", "dead", "driven", "other_room", "separated"] {
            let (mut app, first, nut) = fixture();
            let second = spawn_bird(&mut app, "second", ae::Vec2::new(115.0, 280.0));
            app.update();
            assert!(app.world().get::<SupportedCarry>(first).is_some());
            match loss {
                "missing" => {
                    app.world_mut().despawn(second);
                }
                "dead" => {
                    let mut health = ambition_characters::actor::Health::new(1);
                    health.current = 0;
                    app.world_mut()
                        .entity_mut(second)
                        .insert(BodyHealth::new(health));
                }
                "driven" => {
                    app.world_mut()
                        .entity_mut(second)
                        .insert(DrivingParticipant(
                            ambition_characters::control::PlayerSlot::PRIMARY,
                        ));
                }
                "other_room" => {
                    app.world_mut()
                        .entity_mut(second)
                        .insert(InRoomInstance(LiveRoomInstance::from_ordinal(1)));
                }
                "separated" => {
                    app.world_mut()
                        .get_mut::<ae::BodyKinematics>(second)
                        .unwrap()
                        .pos
                        .x += 400.0;
                }
                _ => unreachable!(),
            }
            app.update();
            assert!(
                app.world().get::<ItemCustody>(nut).unwrap().in_world(),
                "{loss}"
            );
            assert!(app.world().get::<SupportedCarry>(first).is_none(), "{loss}");
            assert!(app.world().get::<HeldItem>(first).is_none(), "{loss}");
        }
    }

    #[test]
    fn a_blocked_partner_does_not_make_a_wait_permanent() {
        let (mut app, first, nut) = fixture();
        spawn_bird(&mut app, "second", ae::Vec2::new(350.0, 200.0));
        for _ in 0..320 {
            app.update();
        }
        assert_eq!(
            app.world().get::<FetchFlight>(first).unwrap().phase,
            FetchPhase::Depart
        );
        assert!(app.world().get::<ItemCustody>(nut).unwrap().in_world());
    }

    fn fetch() -> SwallowFetch {
        crate::tuning::shipped().swallow_fetch
    }

    /// A swallow beside its target climbs to the place over it; over it, it
    /// lets go; and it does not let go from so low that the coconut arrives
    /// slower than it hurts at.
    #[test]
    fn a_laden_swallow_goes_over_its_target_and_lets_go_there() {
        let target = ae::Vec2::new(500.0, 400.0);
        let Laden::Carry(velocity) = laden(
            &fetch(),
            ae::Vec2::new(200.0, 380.0),
            target,
            ae::Vec2::ZERO,
        ) else {
            panic!("a swallow beside its target does not let go");
        };
        assert!(
            velocity.x > 0.0 && velocity.y < 0.0,
            "it flies toward the target and up, not {velocity:?}"
        );
        assert!((velocity.length() - fetch().laden_speed).abs() < 1e-3);
        assert_eq!(
            laden(
                &fetch(),
                ae::Vec2::new(505.0, 400.0 - fetch().carry_over),
                target,
                ae::Vec2::ZERO
            ),
            Laden::LetGo
        );
        assert!(matches!(
            laden(
                &fetch(),
                ae::Vec2::new(505.0, 400.0 - fetch().let_go_over + 20.0),
                target,
                ae::Vec2::ZERO
            ),
            Laden::Carry(_)
        ));
    }

    /// The coconut is let go where a walking target will be, not where it is.
    #[test]
    fn it_leads_a_target_that_moves() {
        let target = ae::Vec2::new(500.0, 400.0);
        let walking = ae::Vec2::new(300.0, 0.0);
        let over_now = ae::Vec2::new(500.0, 400.0 - fetch().carry_over);
        assert!(
            matches!(laden(&fetch(), over_now, target, walking), Laden::Carry(velocity) if velocity.x > 0.0)
        );
        assert_eq!(
            laden(
                &fetch(),
                over_now + ae::Vec2::new(300.0 * fetch().lead_seconds, 0.0),
                target,
                walking
            ),
            Laden::LetGo
        );
    }

    /// The fall from where it lets go is long enough to hurt: the thrown-item
    /// rule's bar for a coconut is a speed, and a fall of `let_go_over` at the
    /// gravity a loose item falls with is faster.
    #[test]
    fn a_coconut_let_go_arrives_faster_than_it_hurts_at() {
        let bar = crate::tuning::shipped()
            .thrown_impacts
            .get(COCONUT)
            .expect("a coconut hurts what it strikes")
            .min_speed;
        let arrives = (2.0 * ambition_held_items::GROUND_ITEM_GRAVITY * fetch().let_go_over).sqrt();
        assert!(
            arrives > bar,
            "a coconut that falls {} arrives at {arrives}, under its bar of {bar}",
            fetch().let_go_over
        );
    }
}
