use super::systems::detect_room_transition_system;
use crate::lifecycle_commit::{LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent};
use crate::participants::SessionParticipants;
use ambition_platformer2d_core::party_controls::PartyTravelMode;
use crate::party_travel::{PartyMembers, PartyTravel, PartyTravelRules};
use ambition_characters::control::{DrivingParticipant, PlayerSlot, SlotInteractionState};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::*;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use ambition_platformer2d_world::rooms::*;
use bevy::prelude::*;

fn fixture(mode: PartyTravelMode, activation: LoadingZoneActivation) -> (App, [Entity; 2]) {
    let zone = |id: &str, x| LoadingZone { presentation: Default::default(),
        id: id.into(), name: id.into(), activation,
        aabb: ae::Aabb::new(ae::Vec2::new(x, 100.0), ae::Vec2::splat(24.0)),
    };
    let room = |id: &str| RoomSpec::new(id, ae::World::new(
        id, ae::Vec2::new(640.0, 480.0), ae::Vec2::new(96.0, 96.0), Vec::new()));
    let mut source = room("source");
    source.loading_zones = vec![zone("first", 100.0), zone("second", 300.0)];
    let mut destination = room("destination");
    destination.loading_zones = vec![zone("entry", 200.0)];
    let set = RoomSet::from_parts_or_panic("source", vec![source, destination],
        ["first", "second"].into_iter().map(|id| RoomLink {
            from_room: "source".into(), from_zone: id.into(), to_room: "destination".into(),
            to_zone: "entry".into(), bidirectional: false,
        }).collect());
    let mut app = App::new();
    insert_room_set(app.world_mut(), set);
    insert_live_room_component(app.world_mut(), LiveRoomInstance::ACTIVATION);
    insert_session_world_component(app.world_mut(),
        ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown::default());
    let mut party = PartyTravel::default();
    party.configure(PartyTravelRules { mode, leader: PlayerSlot(1), members: PartyMembers::Session, ..Default::default() }).unwrap();
    insert_session_world_component(app.world_mut(), party);
    let mut admitted = SessionParticipants::default();
    admitted.admit(PlayerSlot(0)); admitted.admit(PlayerSlot(1));
    insert_session_world_component(app.world_mut(), admitted);
    app.init_resource::<ambition_time::WorldTime>()
        .init_resource::<GatePortalRegistry>()
        .init_resource::<GatePortalPhases>()
        .init_resource::<GateNetworkState>()
        .init_resource::<SlotInteractionState>()
        .init_resource::<PendingLifecycleCommit>()
        .add_message::<ambition_sfx::OwnedSfxMessage>();
    let bodies = [0, 1].map(|slot| app.world_mut().spawn((
        SimId::player_slot(slot), InRoomInstance(LiveRoomInstance::ACTIVATION), DrivingParticipant(PlayerSlot(slot)),
        ae::BodyKinematics { material: Default::default(), pos: ae::Vec2::new(500.0, 100.0), vel: ae::Vec2::ZERO,
            canonical_size: ae::Vec2::new(28.0, 46.0), facing: 1.0 },
    )).id());
    app.add_systems(Update, detect_room_transition_system);
    (app, bodies)
}

fn at(app: &mut App, body: Entity, x: f32) {
    app.world_mut().get_mut::<ae::BodyKinematics>(body).unwrap().pos.x = x;
}
fn press(app: &mut App, slot: u8) {
    app.world_mut().resource_mut::<SlotInteractionState>().get_mut(PlayerSlot(slot)).unwrap()
        .buffered_interact(true, 0.0, 1.0);
}
fn crossing(app: &App) -> Option<&RoomTransitionIntent> {
    match &app.world().resource::<PendingLifecycleCommit>().peek()?.kind {
        LifecycleIntent::Transition(crossing) => Some(crossing), _ => None,
    }
}
fn party(app: &App) -> &PartyTravel {
    session_world_component(app.world()).unwrap()
}

#[test]
fn any_member_selects_the_current_group_without_a_camera() {
    let (mut app, bodies) = fixture(PartyTravelMode::AnyMember, LoadingZoneActivation::Door);
    at(&mut app, bodies[1], 100.0); press(&mut app, 1); app.update();
    let crossing = crossing(&app).unwrap();
    assert_eq!(crossing.initiator.subject.sim_id, SimId::player_slot(1));
    assert_eq!(crossing.initiator.participant, Some(PlayerSlot(1)));
    assert_eq!(crossing.companions.len(), 1);
    assert_eq!(crossing.companions[0].subject.sim_id, SimId::player_slot(0));
    assert_eq!(crossing.companions[0].arrival,
        crate::lifecycle_commit::RoomArrival::NearBody(crossing.initiator.subject.clone()));
}

#[test]
fn the_designated_leader_is_not_inferred_from_slot_zero() {
    let (mut app, bodies) = fixture(PartyTravelMode::Leader, LoadingZoneActivation::Door);
    at(&mut app, bodies[0], 100.0); press(&mut app, 0); app.update();
    assert!(crossing(&app).is_none());
    at(&mut app, bodies[1], 100.0); press(&mut app, 1); app.update();
    assert_eq!(crossing(&app).unwrap().initiator.participant, Some(PlayerSlot(1)));
    assert_eq!(crossing(&app).unwrap().companions.len(), 1);
}

#[test]
fn an_agreement_survives_a_tap_until_the_other_member_agrees() {
    let (mut app, bodies) = fixture(PartyTravelMode::AllMembers, LoadingZoneActivation::Door);
    at(&mut app, bodies[0], 100.0); press(&mut app, 0); app.update();
    assert!(crossing(&app).is_none());
    assert!(party(&app).vote(PlayerSlot(0)).is_some());
    assert!(!app.world().resource::<SlotInteractionState>().get(PlayerSlot(0)).buffered());
    at(&mut app, bodies[1], 100.0); app.update();
    assert!(crossing(&app).is_none());
    press(&mut app, 1); app.update();
    assert_eq!(crossing(&app).unwrap().companions.len(), 1);
    assert!(!party(&app).has_votes());
}

#[test]
fn leaving_the_exit_cancels_an_agreement() {
    let (mut app, bodies) = fixture(PartyTravelMode::AllMembers, LoadingZoneActivation::Door);
    at(&mut app, bodies[0], 100.0); press(&mut app, 0); app.update();
    at(&mut app, bodies[0], 500.0); at(&mut app, bodies[1], 100.0);
    press(&mut app, 1); app.update();
    assert!(crossing(&app).is_none());
    assert!(party(&app).vote(PlayerSlot(0)).is_none());
    assert!(party(&app).vote(PlayerSlot(1)).is_some());
}

#[test]
fn different_exits_to_the_same_destination_do_not_form_an_agreement() {
    let (mut app, bodies) = fixture(PartyTravelMode::AllMembers, LoadingZoneActivation::Door);
    at(&mut app, bodies[0], 100.0); at(&mut app, bodies[1], 300.0);
    press(&mut app, 0); press(&mut app, 1); app.update();
    assert!(crossing(&app).is_none());
    assert!(party(&app).vote(PlayerSlot(0)).is_some());
    assert!(party(&app).vote(PlayerSlot(1)).is_some());
}

#[test]
fn changing_the_driven_body_cancels_its_previous_agreement() {
    let (mut app, bodies) = fixture(PartyTravelMode::AllMembers, LoadingZoneActivation::Door);
    at(&mut app, bodies[0], 100.0); press(&mut app, 0); app.update();
    app.world_mut().entity_mut(bodies[0]).remove::<DrivingParticipant>();
    let kin = *app.world().get::<ae::BodyKinematics>(bodies[0]).unwrap();
    app.world_mut().spawn((kin, SimId::placement("possessed"), DrivingParticipant(PlayerSlot(0)),
        InRoomInstance(LiveRoomInstance::ACTIVATION)));
    at(&mut app, bodies[1], 100.0); press(&mut app, 1); app.update();
    assert!(crossing(&app).is_none());
    assert!(party(&app).vote(PlayerSlot(0)).is_none());
}

#[test]
fn an_unavailable_member_blocks_a_group_until_that_participant_leaves() {
    let (mut app, bodies) = fixture(PartyTravelMode::AnyMember, LoadingZoneActivation::Door);
    app.world_mut().entity_mut(bodies[0]).insert(ambition_combat::death_rules::OutOfPlay);
    at(&mut app, bodies[1], 100.0); press(&mut app, 1); app.update();
    assert!(crossing(&app).is_none());
    session_world_component_mut::<SessionParticipants>(app.world_mut()).unwrap().remove(PlayerSlot(0));
    app.update();
    assert!(crossing(&app).unwrap().companions.is_empty());
}

#[test]
fn the_same_exit_in_two_room_instances_cannot_form_an_agreement() {
    let (mut app, bodies) = fixture(PartyTravelMode::AllMembers, LoadingZoneActivation::Door);
    let definition = session_world_component::<RoomSet>(app.world()).unwrap().definition_by_id("source").unwrap();
    let second = LiveRoomInstance::ACTIVATION.next();
    app.world_mut().spawn((RoomInstanceRoot, second, definition));
    app.world_mut().entity_mut(bodies[1]).insert(InRoomInstance(second));
    for body in bodies { at(&mut app, body, 100.0); }
    press(&mut app, 0); press(&mut app, 1); app.update();
    assert!(crossing(&app).is_none());
}

#[test]
fn entry_animations_start_only_after_the_group_agrees() {
    use ambition_platformer2d_world::scoped_rules::{DeclareRulesExt, RulesScope};
    use super::passage::{ZonePassage, ZonePassageRules};
    let (mut app, bodies) = fixture(PartyTravelMode::AllMembers, LoadingZoneActivation::Door);
    app.declare_rules(RulesScope::EveryRoom, ZonePassageRules);
    at(&mut app, bodies[0], 100.0); press(&mut app, 0); app.update();
    assert!(app.world().get::<ZonePassage>(bodies[0]).is_none());
    at(&mut app, bodies[1], 100.0); press(&mut app, 1); app.update();
    assert!(app.world().get::<ZonePassage>(bodies[1]).is_some());
    assert!(app.world().get::<ZonePassage>(bodies[0]).is_none());
    assert!(crossing(&app).is_none());
}

#[test]
fn a_joined_participant_must_agree_before_the_group_can_cross() {
    let (mut app, bodies) = fixture(PartyTravelMode::AllMembers, LoadingZoneActivation::Door);
    at(&mut app, bodies[0], 100.0); press(&mut app, 0); app.update();
    session_world_component_mut::<SessionParticipants>(app.world_mut()).unwrap().admit(PlayerSlot(2));
    at(&mut app, bodies[1], 100.0); press(&mut app, 1); app.update();
    assert!(crossing(&app).is_none());
    session_world_component_mut::<SessionParticipants>(app.world_mut()).unwrap().remove(PlayerSlot(2));
    app.update();
    assert_eq!(crossing(&app).unwrap().companions.len(), 1);
}

#[test]
fn leaving_a_walk_exit_does_not_renew_the_vote_from_its_departure_sweep() {
    let (mut app, bodies) = fixture(PartyTravelMode::AllMembers, LoadingZoneActivation::Walk);
    at(&mut app, bodies[0], 100.0); app.update();
    at(&mut app, bodies[0], 200.0);
    app.world_mut().entity_mut(bodies[0]).insert(ae::SweepSample { material: Default::default(), travel: Default::default(), time: [0.0, 1.0],
        prev: ae::Vec2::new(100.0, 100.0), curr: ae::Vec2::new(200.0, 100.0),
        vel: ae::Vec2::X * 100.0, half: ae::Vec2::new(14.0, 23.0), down: ae::Vec2::Y,
    });
    at(&mut app, bodies[1], 100.0); app.update();
    assert!(crossing(&app).is_none());
    assert!(party(&app).vote(PlayerSlot(0)).is_none());
}

#[test]
fn a_changed_route_cancels_an_agreement_to_the_old_destination() {
    let (mut app, bodies) = fixture(PartyTravelMode::AllMembers, LoadingZoneActivation::Door);
    at(&mut app, bodies[0], 100.0); press(&mut app, 0); app.update();
    let mut rooms = session_world_component::<RoomSet>(app.world()).unwrap().rooms.clone();
    let mut other = rooms[1].clone(); other.id = "other".into(); rooms.push(other);
    let set = RoomSet::from_parts_or_panic("source", rooms, vec![RoomLink {
        from_room: "source".into(), from_zone: "first".into(), to_room: "other".into(),
        to_zone: "entry".into(), bidirectional: false,
    }]);
    insert_session_world_component(app.world_mut(), set);
    at(&mut app, bodies[1], 100.0); press(&mut app, 1); app.update();
    assert!(crossing(&app).is_none());
    assert!(party(&app).vote(PlayerSlot(0)).is_none());
}

#[test]
fn only_the_admitted_leaders_replayed_preference_changes_the_rules() {
    use ambition_characters::control::SlotControls;
    let (mut app, _) = fixture(PartyTravelMode::Independent, LoadingZoneActivation::Door);
    app.init_resource::<SlotControls>();
    app.add_systems(Update, crate::party_travel::apply_party_preferences);
    let mut rules = party(&app).rules().clone();
    rules.accept_preferences = true;
    session_world_component_mut::<PartyTravel>(app.world_mut()).unwrap().configure(rules).unwrap();
    let choice = ae::party_controls::PartyControlPreference::new(PartyTravelMode::Leader, true);
    app.world_mut().resource_mut::<SlotControls>().set(PlayerSlot(0), ae::ControlFrame {
        party_preference: choice, ..Default::default()
    });
    app.update();
    assert_eq!(party(&app).rules().mode, PartyTravelMode::Independent);
    app.world_mut().resource_mut::<SlotControls>().set(PlayerSlot(1), ae::ControlFrame {
        party_preference: choice, ..Default::default()
    });
    app.update();
    assert_eq!(party(&app).rules().mode, PartyTravelMode::Leader);
    assert!(party(&app).rules().return_to_leader);
}

#[test]
fn a_follower_returns_its_driven_body_without_a_camera_or_reset_ability() {
    use ambition_characters::control::SlotControls;
    let (mut app, bodies) = fixture(PartyTravelMode::Independent, LoadingZoneActivation::Door);
    app.init_resource::<SlotControls>();
    app.add_systems(Update, crate::party_travel::request_party_return);
    let mut rules = party(&app).rules().clone(); rules.return_to_leader = true;
    session_world_component_mut::<PartyTravel>(app.world_mut()).unwrap().configure(rules).unwrap();
    app.world_mut().resource_mut::<SlotControls>().set(PlayerSlot(0), ae::ControlFrame {
        return_to_leader_pressed: true, reset_pressed: true, ..Default::default()
    });
    assert!(!app.world().resource::<SlotControls>().get(PlayerSlot(0)).reset_pressed);
    app.update();
    let intent = crossing(&app).expect("the follower returns");
    assert_eq!(intent.initiator.subject, LiveBodyId::of_entity(app.world(), bodies[0]).unwrap());
    assert_eq!(intent.initiator.arrival, crate::lifecycle_commit::RoomArrival::NearBody(
        LiveBodyId::of_entity(app.world(), bodies[1]).unwrap()));
    assert!(intent.companions.is_empty());
}
