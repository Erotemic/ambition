//! Ambition plays itself, for unattended profiling: `AMBITION_AUTOPLAY=ambition`.
//!
//! The policy is one fixed itinerary, so two runs load the same rooms in the
//! same order and their profiles compare. From the hub the player walks into
//! the hall of characters and across it, walks out and to the primeval trail,
//! flies through the trail over its dinosaurs and across the tyrant's hall,
//! and flies back to the hub; then again.
//!
//! Each stop is a room. Doors and available dimension gates share one room
//! route. A gate trip uses its terminal, dials through the host intent ledger,
//! and enters the open ring. In each room the engine's autopilot takes the
//! body to the door the route leaves by (on foot by the room's surface graph,
//! or in the air by its flight map), with the input a person would give, and
//! the policy presses Interact there.
//!
//! A cutscene that plays on a room's entry (a level's `entry_cutscene`) holds
//! the body until its lines are dismissed: the policy presses Confirm, as a
//! person would.
//!
//! The default route does not fight or collect items. `castle_godwin` selects
//! a combat route through the castle's gate, halls, muster, and boss fight.

use bevy::prelude::*;

use ambition_platformer2d::autopilot::{Autopilot, AutopilotOrder, AutopilotStatus, NavGoal};
use ambition_platformer2d::engine_core as ae;
use ae::AabbExt;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::world::rooms::{
    GateFacts, GateNetworkCatalog, GateNetworkState, GatePortalPhases, GateRingPositions,
    GateTerminalAction, GateTerminalIntent, LiveRoomSpecs, LoadingZone, LoadingZoneActivation,
    RoomHop, RoomSet, RoomSpec,
};
use ambition_platformer2d::platformer::host_intents::HostIntentWriter;
use bevy::ecs::system::SystemParam;

/// This game's id in the hub, the value of `AMBITION_AUTOPLAY` that selects
/// this policy.
pub const AUTOPLAY_GAME: &str = "ambition";
pub const CASTLE_AUTOPLAY: &str = "castle_godwin";

use ambition_content_schemas::autoplay::{Stop, Way};

#[derive(Resource)]
pub(crate) struct Profiles(pub ambition_content_schemas::autoplay::Profiles);

pub(super) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    let profiles = ambition_content_schemas::autoplay::lowered_profiles(&pack).cloned().unwrap_or_default();
    app.insert_resource(Profiles(profiles)).init_resource::<Itinerary>()
        .add_systems(Update, play_ambition
            .in_set(ambition_platformer2d::scripted_input::AutoplayPolicy)
            // Read the completed frame before the next simulation advance.
            .before(ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhase::CoreSimulation)
            .run_if(|mode: Option<Res<ambition_platformer2d::scripted_input::Autoplay>>, profiles: Res<Profiles>|
                mode.is_some_and(|mode| profiles.0.contains_key(&mode.0))));
}

/// Frames between two presses of Interact at a door, or of Confirm in a
/// cutscene.
const PRESS_EVERY: u32 = 20;
/// Frames between route attempts while no route is available.
const RETRY_ROUTE_FRAMES: u32 = 60;
/// How far from the far wall a crossing ends (px).
const CROSS_INSET: f32 = 96.0;
/// How high over its arrival a flight across a room goes (px).
const CROSS_LIFT: f32 = 64.0;

/// Where the itinerary is.
#[derive(Resource, Default)]
pub struct Itinerary {
    stop: usize,
    /// The room the body is in, as far as this policy has seen.
    room: Option<String>,
    /// Where the body came into it.
    arrival: ae::Vec2,
    /// What the body goes to in this room.
    goal: Option<Goal>,
    frames_in_room: u32,
    frames_since_press: u32,
    pending_dial: Option<(String, u64)>,
    pub completed: bool,
    frames: u64,
    equipped: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Goal {
    /// The zone this room is left by on the way to the stop.
    Leave(RoomHop),
    /// Across the stop's room, to this point.
    Cross(ae::Vec2),
    /// Nothing left to do in this room: the stop is done.
    Done,
    /// No available route yet. A closed gate can become available later.
    Waiting,
}

/// The live gate facts and the same request channel that the terminal uses.
#[derive(SystemParam)]
pub struct GateTravel<'w, 's> {
    catalog: Res<'w, GateNetworkCatalog>,
    state: Res<'w, GateNetworkState>,
    positions: Res<'w, GateRingPositions>,
    phases: Res<'w, GatePortalPhases>,
    save: Res<'w, ambition_platformer2d::persistence::save::AmbitionGameSave>,
    intents: HostIntentWriter<'w, 's, GateTerminalIntent>,
    terminals: Query<'w, 's, (Entity, &'static ambition_platformer2d::encounter::switches::SwitchFeature,
        &'static ae::geometry::CenteredAabb)>,
}

impl GateTravel<'_, '_> {
    fn route(&self, rooms: &RoomSet, from: usize, to: usize) -> Option<Vec<RoomHop>> {
        let data = self.save.data();
        let switch_on = |id: &str| data.switch(id);
        let flag = |id: &str| data.flag(id);
        let zone_exists = |room: &str, zone: &str| rooms.has_zone(room, zone);
        let distance = |a: &str, b: &str| self.positions.distance(a, b);
        self.catalog.route(rooms, &self.state, from, to, &GateFacts {
            switch_on: &switch_on, flag: &flag, zone_exists: &zone_exists, distance: &distance,
        })
    }

    fn ask(&mut self, subject: &ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId, ring: &str, action: GateTerminalAction, it: &mut Itinerary) {
        if action == GateTerminalAction::Close || it.frames_since_press >= PRESS_EVERY {
            it.frames_since_press = 0;
            if matches!(action, GateTerminalAction::Dial(_)) {
                it.pending_dial = Some((ring.to_owned(), self.state.next_serial));
            }
            self.intents.write(GateTerminalIntent { subject: subject.clone(), ring: ring.to_owned(), action });
        }
    }
}

/// The policy: this frame's order to the autopilot.
fn play_ambition(
    mut order: ResMut<AutopilotOrder>,
    mut itinerary: ResMut<Itinerary>,
    mode: Res<ambition_platformer2d::scripted_input::Autoplay>,
    profiles: Res<Profiles>,
    items: ambition_items::ItemCatalogRead,
    autopilot: Option<Res<Autopilot>>,
    rooms: Option<LiveRoomSpecs>,
    players: Query<(Entity, &ae::BodyKinematics, Option<&ae::SweepSample>, Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>, &ambition_characters::control::DrivingParticipant, &ambition_platformer2d_shared_tangle::sim_id::SimId), With<PrimaryPlayer>>,
    cutscene: Option<Res<ambition_cutscene::ActiveCutscene>>,
    mut gates: GateTravel,
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    boss_records: Query<(&ambition_boss_encounter::BossConfig, Option<&ambition_extension_host::BodyRecords>)>,
    mut equips: HostIntentWriter<ambition_items::ItemEquipRequested>,
    encounters: Query<(Entity, &ambition_platformer2d::encounter::Encounter,
        &ambition_platformer2d::encounter::EncounterLifecycle)>,
    actors: Query<(Entity, &ae::BodyKinematics,
        &ambition_platformer2d::characters::actor::BodyHealth,
        &ambition_platformer2d::characters::actor::ActorFaction,
        Option<&ambition_boss_encounter::BossEncounter>,
        &ambition_platformer2d_shared_tangle::sim_id::SimId)>,
) {
    let Ok((entity, kin, sweep, in_room, driver, subject)) = players.single() else {
        *order = AutopilotOrder::Idle;
        return;
    };
    let subject = ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(subject.clone(), in_room.map(|room| room.0));
    let Some(specs) = rooms.as_ref() else {
        *order = AutopilotOrder::Idle;
        return;
    };
    let Some(room) = specs.spec_of(entity) else {
        *order = AutopilotOrder::Idle;
        return;
    };
    let rooms = specs.rooms();
    let it = &mut *itinerary;
    let Some(profile) = profiles.0.get(&mode.0) else { return; };
    if profiles.is_changed() || mode.is_changed() { *it = Itinerary::default(); }
    let stops = &profile.stops;
    let combat = profile.fight_at_stops;
    it.frames += 1;
    if !it.equipped {
        if let Some(id) = &profile.equip {
            if let Some(item) = items.get().item_by_dialog_id(id) {
                equips.write(ambition_items::ItemEquipRequested { participant: driver.0, item });
            } else { error!("autoplay {} requests missing equipment {id}", mode.0); }
        }
        it.equipped = true;
    }
    if it.completed {
        *order = AutopilotOrder::Idle;
        return;
    }
    if specs.set_is_changed() || gates.catalog.is_changed() {
        it.goal = None;
    }
    it.frames_since_press = it.frames_since_press.saturating_add(1);
    if let Some((ring, serial)) = it.pending_dial.clone() {
        if let Some(report) = gates.state.report_for(&subject, &ring).filter(|report| report.serial >= serial) {
            it.pending_dial = None;
            if report.outcome.is_err() {
                eprintln!("[autoplay] dial refused ring={ring} outcome={:?}; planning again", report.outcome);
                gates.ask(&subject, &ring, GateTerminalAction::Close, it);
                it.goal = None;
                *order = AutopilotOrder::Idle;
                return;
            }
        }
    }
    if cutscene.is_some_and(|cutscene| cutscene.is_playing()) {
        *order = if it.frames_since_press >= PRESS_EVERY {
            it.frames_since_press = 0;
            AutopilotOrder::Press(ControlFrame { confirm_pressed: true, ..ControlFrame::default() })
        } else {
            AutopilotOrder::Idle
        };
        return;
    }
    it.frames_in_room = it.frames_in_room.saturating_add(1);
    if it.room.as_deref() != Some(room.id.as_str()) {
        it.room = Some(room.id.clone());
        it.arrival = kin.pos;
        it.frames_in_room = 0;
        it.goal = None;
        it.pending_dial = None;
    }
    if combat && stops.iter().any(|stop| stop.room == room.id) {
        let target = actors.iter().filter(|(actor, target, health, faction, boss, _)|
            specs.live().of(*actor) == specs.live().of(entity)
                && health.alive() && faction.is_hostile_side()
                && target.pos.x >= 0.0 && target.pos.x < room.world.size.x
                && target.pos.y >= 0.0 && target.pos.y < room.world.size.y
                && boss.is_none_or(|boss| boss.encounter_phase() != ambition_boss_encounter::BossEncounterPhase::Dormant
                    || boss_records.get(*actor).is_ok_and(|(config, _)| config.wakes_after.is_none())))
            .min_by(|(_, ka, _, _, _, a), (_, kb, _, _, _, b)| kin.pos.distance_squared(ka.pos)
                .total_cmp(&kin.pos.distance_squared(kb.pos)).then_with(|| a.cmp(b)));
        if let Some((target_entity, target, ..)) = target {
            if let Ok((config, records)) = boss_records.get(target_entity) {
                use ambition_content_modules::inugami::{Conductor, INUGAMI_ID};
                if config.behavior.id == INUGAMI_ID && records.and_then(|records| records.get(&Conductor::KEY))
                    .and_then(|record| Conductor::from_record(record).ok()).is_some_and(|c| c.warded()) {
                    if let Some(zone) = room.camera_zones.iter().find(|zone| zone.id == "castle_godwin_throne_final_camera") {
                        *order = go_to(zone.aabb.center(), Way::Fly);
                        return;
                    }
                }
            }
            let delta = target.pos - kin.pos;
            let clear = collision.room(in_room).and_then(|room| room.solids())
                .is_some_and(|walls| ae::cast::raycast_solids(&*walls, kin.pos, delta.normalize_or_zero(), delta.length(), false).is_none());
            if delta.length() <= 110.0 && clear {
                let aim = delta.normalize_or_zero();
                let fire = it.frames % 90 == 0;
                *order = AutopilotOrder::Press(ControlFrame {
                    axis_x: if delta.x.abs() > 50.0 { delta.x.signum() * 0.5 } else { delta.x.signum() * 0.05 },
                    axis_y: (delta.y / 80.0).clamp(-1.0, 1.0),
                    aim_x: aim.x, aim_y: aim.y,
                    projectile_pressed: fire, projectile_held: it.frames % 90 < 60,
                    projectile_released: it.frames % 90 == 60,
                    attack_pressed: delta.length() < 95.0 && it.frames % 24 == 0,
                    attack_from_aim_stick: true, attack_aim_x: aim.x, attack_aim_y: aim.y,
                    ..default()
                });
            } else {
                *order = go_to(target.pos, Way::Fly);
            }
            return;
        }
        for trigger in &room.encounter_triggers {
            let phase = encounters.iter().find(|(encounter, id, _)| id.id == trigger.id
                && specs.live().of(*encounter) == specs.live().of(entity)).map(|(_, _, life)| life.phase());
            use ambition_platformer2d::encounter::EncounterPhase;
            match phase {
                Some(EncounterPhase::Inactive | EncounterPhase::Failed) => {
                    *order = go_to(trigger.min + trigger.size * 0.5, Way::Fly);
                    return;
                }
                Some(EncounterPhase::Starting { .. } | EncounterPhase::Active) => {
                    *order = AutopilotOrder::Idle;
                    return;
                }
                _ => {}
            }
        }
        if !profile.repeat && stops.last().is_some_and(|stop| stop.room == room.id) {
            let bosses: Vec<_> = actors.iter().filter(|(actor, _, _, _, boss, _)|
                boss.is_some() && specs.live().of(*actor) == specs.live().of(entity)).collect();
            if !bosses.is_empty() && bosses.iter().all(|(_, _, health, _, _, _)| !health.alive()) {
                it.completed = true;
                info!("[autoplay] {} complete frames={}", mode.0, it.frames);
                *order = AutopilotOrder::Idle;
                return;
            }
        }
    }
    let status = autopilot.as_deref().map(Autopilot::status);
    if it.goal == Some(Goal::Waiting) && it.frames_in_room % RETRY_ROUTE_FRAMES == 0 {
        it.goal = None;
    }
    if it.goal == Some(Goal::Done) || (matches!(it.goal, Some(Goal::Cross(_))) && status == Some(AutopilotStatus::Arrived)) {
        if profile.repeat || it.stop + 1 < stops.len() {
            it.advance(stops.len());
        } else if !combat {
            it.completed = true;
            *order = AutopilotOrder::Idle;
            return;
        }
    }
    if it.goal.is_none() {
        it.goal = Some(goal_in(rooms, room, &stops[it.stop], it.arrival, &gates));
        eprintln!("[autoplay] room={} stop={} goal={:?}", room.id, stops[it.stop].room, it.goal);
    }
    let way = stops[it.stop].way;
    *order = match it.goal.clone() {
        Some(Goal::Leave(hop)) => {
            let Some(zone) = room.loading_zones.iter().find(|zone| zone.id == hop.zone) else {
                it.goal = None;
                *order = AutopilotOrder::Idle;
                return;
            };
            let body = kin.collision_box(sweep);
            if let Some(address) = &hop.dial {
                let linked = gates.state.route_from(&zone.id).is_some_and(|link| link.address == *address);
                if gates.state.terminals.get(&subject).map(String::as_str) == Some(&zone.id) {
                    let action = if linked { GateTerminalAction::Close } else { GateTerminalAction::Dial(address.clone()) };
                    gates.ask(&subject, &zone.id, action, it);
                    *order = AutopilotOrder::Idle;
                    return;
                }
                if !linked {
                    if let Some(open) = gates.state.terminals.get(&subject).cloned() {
                        gates.ask(&subject, &open, GateTerminalAction::Close, it);
                        *order = AutopilotOrder::Idle;
                        return;
                    }
                    let terminal = gates.catalog.ring(&zone.id).and_then(|ring| ring.terminal.as_deref());
                    let at = gates.terminals.iter().find(|(switch, feature, _)| {
                        Some(feature.activation.id.as_str()) == terminal && specs.live().of(*switch) == specs.live().of(entity)
                    }).map(|(_, _, at)| *at);
                    *order = match at {
                        Some(at) if body.strict_intersects(at.aabb()) => press_interact(it),
                        Some(at) => go_to(at.center, way),
                        None => {
                            it.goal = None;
                            AutopilotOrder::Idle
                        }
                    };
                    return;
                }
                if !gates.phases.allows_traversal(&zone.id) {
                    *order = AutopilotOrder::Idle;
                    return;
                }
            }
            enter(zone, body, way, it)
        }
        Some(Goal::Cross(at)) => match way {
            Way::Walk => AutopilotOrder::GoTo(NavGoal::Stand(at)),
            Way::Fly => AutopilotOrder::GoTo(NavGoal::Fly(at)),
        },
        Some(Goal::Done | Goal::Waiting) | None => AutopilotOrder::Idle,
    };
}

impl Itinerary {
    /// On to the next stop, from this room.
    fn advance(&mut self, stops: usize) {
        self.stop = (self.stop + 1) % stops;
        self.goal = None;
        self.frames_in_room = 0;
    }
}

/// What the body goes to in `room` on its way to `stop`, having come in at
/// `arrival`.
fn goal_in(rooms: &RoomSet, room: &RoomSpec, stop: &Stop, arrival: ae::Vec2, gates: &GateTravel) -> Goal {
    if room.id == stop.room {
        return if stop.cross { Goal::Cross(cross_point(room.world.size, arrival, stop.way)) } else { Goal::Done };
    }
    let first_hop = rooms
        .room_index_by_id(&room.id)
        .zip(rooms.room_index_by_id(&stop.room))
        .and_then(|(from, to)| gates.route(rooms, from, to))
        .and_then(|hops| hops.into_iter().next());
    match first_hop {
        Some(hop) => Goal::Leave(hop),
        None => Goal::Waiting,
    }
}

fn go_to(at: ae::Vec2, way: Way) -> AutopilotOrder {
    AutopilotOrder::GoTo(match way { Way::Walk => NavGoal::Stand(at), Way::Fly => NavGoal::Fly(at) })
}

fn press_interact(it: &mut Itinerary) -> AutopilotOrder {
    if it.frames_since_press < PRESS_EVERY { return AutopilotOrder::Idle; }
    it.frames_since_press = 0;
    AutopilotOrder::Press(ControlFrame { interact_pressed: true, ..ControlFrame::default() })
}

fn enter(zone: &LoadingZone, body: ae::Aabb, way: Way, it: &mut Itinerary) -> AutopilotOrder {
    if zone.activation == LoadingZoneActivation::Door && zone.reached_by(body, ae::Vec2::ZERO) {
        return press_interact(it);
    }
    let at = match way {
        Way::Walk => ae::Vec2::new(zone.aabb.center().x, zone.aabb.max.y - 1.0),
        Way::Fly => return AutopilotOrder::GoTo(NavGoal::FlyInto(zone.aabb)),
    };
    go_to(at, way)
}

/// The far side of a room of `size` from `arrival`: on foot at the height it
/// came in at, in the air a little over it.
fn cross_point(size: ae::Vec2, arrival: ae::Vec2, way: Way) -> ae::Vec2 {
    let x = if arrival.x < size.x * 0.5 { size.x - CROSS_INSET } else { CROSS_INSET };
    match way {
        Way::Walk => ae::Vec2::new(x, arrival.y),
        Way::Fly => ae::Vec2::new(x, arrival.y - CROSS_LIFT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_crossing_goes_to_the_far_side_from_where_the_body_came_in() {
        let size = ae::Vec2::new(2000.0, 700.0);
        assert_eq!(cross_point(size, ae::Vec2::new(100.0, 600.0), Way::Walk), ae::Vec2::new(1904.0, 600.0));
        assert_eq!(cross_point(size, ae::Vec2::new(1900.0, 600.0), Way::Fly), ae::Vec2::new(96.0, 536.0));
    }

    #[test]
    fn a_flying_body_opens_an_edge_door_without_putting_its_center_outside_the_room() {
        let zone = LoadingZone {
            id: "west".into(), name: "west".into(), activation: LoadingZoneActivation::Door,
            aabb: ae::Aabb { min: ae::Vec2::new(0.0, 384.0), max: ae::Vec2::new(16.0, 480.0) },
        };
        let body = ae::Aabb::new(ae::Vec2::new(36.0, 426.0), ae::Vec2::new(28.0, 24.0));
        let mut it = Itinerary { frames_since_press: PRESS_EVERY, ..default() };
        assert!(body.center().x > zone.aabb.max.x);
        assert!(matches!(enter(&zone, body, Way::Fly, &mut it), AutopilotOrder::Press(frame) if frame.interact_pressed));
        let far = ae::Aabb::new(ae::Vec2::new(100.0, 426.0), body.half_size());
        assert!(matches!(enter(&zone, far, Way::Fly, &mut it), AutopilotOrder::GoTo(_)));
    }
}
