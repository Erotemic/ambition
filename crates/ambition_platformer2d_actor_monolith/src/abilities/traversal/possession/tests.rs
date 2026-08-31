use super::*;
use ambition_characters::brain::StateMachineCfg;
use ambition_characters::control::PlayerSlot;
use ambition_combat::components::ActorFaction;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayer;

fn vec2(x: f32, y: f32) -> ambition_platformer2d_core::Vec2 {
    ambition_platformer2d_core::Vec2::new(x, y)
}

/// App with the trigger + 1s/frame real time, so 2 held frames clear the 2s hold.
fn trigger_app() -> App {
    let mut app = App::new();
    app.init_resource::<ambition_characters::control::SlotControls>();
    app.insert_resource(ambition_time::WorldTime {
        raw_dt: 1.0,
        scaled_dt: 1.0,
    });
    app.init_resource::<PossessionState>();
    //  the PROJECTION is part of the mechanic, not decoration. The custody
    // marker a driven body wears is derived from `PossessionState` every tick —
    // see `project_possession_onto_custody` for the rollback reason it is a
    // derive rather than a write at the possess site — so a harness without it
    // composes a possession that only half happens, and every custody assertion
    // below would be measuring the harness.
    app.add_systems(
        Update,
        (
            possession_trigger_system,
            release_possession_if_target_lost,
            crate::body_custody::project_body_custody,
            //  the SEAT is part of the mechanic too, for the same reason the
            // custody projection is: `possession_trigger_system` states the
            // decision and this is the one system that moves
            // `DrivingParticipant` onto the driven body and back.
            crate::control::project_driving_participant,
        )
            .chain(),
    );
    app
}

fn spawn_home(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            // The home avatar is SEATED, and its own policy is to stand still —
            // which is what it does while somebody drives something else.
            DrivingParticipant(PlayerSlot::PRIMARY),
            Brain::stand_still(),
            ActorControl::default(),
            BodyKinematics {
                pos: vec2(0.0, 0.0),
                vel: vec2(0.0, 0.0),
                size: vec2(24.0, 40.0),
                facing: 1.0,
            },
            // The vacate-exit is a discrete transit through the home body's
            // full clusters + policy (ADR 0024 authority) — spawn the real set.
            ambition_platformer2d_core::movement::MotionModel::default(),
            ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    vec2(0.0, 0.0),
                    ambition_platformer2d_core::AbilitySet::default(),
                ),
            ),
        ))
        .id()
}

fn spawn_candidate(app: &mut App, pos: ambition_platformer2d_core::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            CenteredAabb::new(pos, vec2(12.0, 16.0)),
            Brain::StateMachine(StateMachineCfg::StandStill),
            ActorControl::default(),
            ActorFaction::Enemy,
        ))
        .id()
}

/// The participant seat this body holds, if any.
fn brain_slot(app: &App, e: Entity) -> Option<PlayerSlot> {
    app.world().get::<DrivingParticipant>(e).map(|d| d.0)
}

fn faction_of(app: &App, e: Entity) -> ActorFaction {
    *app.world().get::<ActorFaction>(e).unwrap()
}

fn hold_down_interact(app: &mut App, held: bool) {
    let slot = ambition_characters::control::PlayerSlot::PRIMARY;
    let mut slots = app
        .world_mut()
        .resource_mut::<ambition_characters::control::SlotControls>();
    let mut frame = slots.get(slot);
    frame.axis_y = if held { 1.0 } else { 0.0 };
    frame.interact_held = held;
    slots.set(slot, frame);
}

/// ⛔⛔ TWO CANDIDATES AT EQUAL REACH IS A COIN FLIP UNTIL SOMETHING BREAKS THE
/// TIE. The pick was `min_by` on distance alone, which keeps whichever body the
/// query yields first — archetype order, not a gameplay rule, and not what a
/// resimulation reproduces. Which body the player is DRIVING is about as
/// authoritative as state gets.
///
/// ⭐ THE ARM IS SPAWN ORDER, and the identities are fixed to POSITION rather
/// than to the spawn slot, so "the same winner" means the same body both times
/// and not merely the same index.
#[test]
fn possessing_between_two_equidistant_candidates_picks_the_same_one_either_spawn_order() {
    fn possessed_id(left_first: bool) -> String {
        let mut app = trigger_app();
        spawn_home(&mut app);
        let left = vec2(-80.0, 0.0);
        let right = vec2(80.0, 0.0);
        let (first, second) = if left_first {
            (left, right)
        } else {
            (right, left)
        };
        for at in [first, second] {
            let entity = spawn_candidate(&mut app, at);
            let id = if at.x < 0.0 { "left" } else { "right" };
            app.world_mut()
                .entity_mut(entity)
                .insert(ambition_platformer2d_shared_tangle::sim_id::SimId::placement(id));
        }

        hold_down_interact(&mut app, true);
        app.update();
        app.update();

        let target = app
            .world()
            .resource::<PossessionState>()
            .possessed
            .expect("the hold crossed the threshold and possessed something");
        app.world()
            .get::<ambition_platformer2d_shared_tangle::sim_id::SimId>(target)
            .expect("the fixture gave every candidate an identity")
            .as_str()
            .to_string()
    }

    assert_eq!(
        possessed_id(true),
        possessed_id(false),
        "the player possessed one body when the left candidate was spawned \
         first and the other when the right one was — the choice is archetype \
         order, which a rollback resimulation does not reproduce"
    );
}

#[test]
fn possession_transfers_the_seat_and_release_hands_it_back() {
    let mut app = trigger_app();
    let home = spawn_home(&mut app);
    let actor = spawn_candidate(&mut app, vec2(80.0, 0.0)); // in range

    // Before possession: home holds the seat; the actor holds none.
    assert_eq!(brain_slot(&app, home), Some(PlayerSlot::PRIMARY));
    assert_eq!(brain_slot(&app, actor), None);

    // Hold Down+Interact: 1s, then 2s → crosses the threshold → possess.
    hold_down_interact(&mut app, true);
    app.update(); // hold_timer = 1.0
    assert_eq!(brain_slot(&app, actor), None, "not possessed mid-hold");
    app.update(); // hold_timer = 2.0 ≥ threshold → transfer

    // After possession: the ACTOR holds the seat; the home avatar no longer
    // does; the actor is player-aligned; its own brain is UNTOUCHED.
    assert_eq!(brain_slot(&app, actor), Some(PlayerSlot::PRIMARY));
    assert_eq!(
        brain_slot(&app, home),
        None,
        "home avatar's seat is vacated"
    );
    assert!(
        app.world().get::<DrivingParticipant>(home).is_none(),
        "the home avatar still holds the seat it handed over"
    );
    assert!(
        matches!(
            app.world().get::<Brain>(actor),
            Some(Brain::StateMachine(StateMachineCfg::StandStill))
        ),
        "the driven body's OWN policy was displaced — possession moves a seat, \
         never a brain"
    );
    // Effective allegiance: the target's AUTHORED faction is NOT mutated by
    // possession (it stays Enemy). Combat treats it as Player because it
    // holds a `DrivingParticipant` — verified by the targeting/damage tests — so
    // there is no flip to bookkeep and no restore on release.
    assert_eq!(
        faction_of(&app, actor),
        ActorFaction::Enemy,
        "possession must NOT overwrite the authored faction"
    );
    assert_eq!(
        app.world().resource::<PossessionState>().possessed,
        Some(actor)
    );
    // The REPORTED BUG's root cause is gone: the vacated home avatar has a
    // neutral `ActorControl` and no brain to repopulate it, so it emits no
    // melee/attack this frame or any frame while possessed — attack authority
    // can only originate from the body holding the seat.
    assert_eq!(
        app.world().get::<ActorControl>(home).map(|c| c.0),
        Some(ambition_characters::actor::control::ActorControlFrame::neutral()),
        "vacated home avatar's control frame is cleared — no attack authority"
    );

    // Release: a fresh Down+Interact press hands control back.
    hold_down_interact(&mut app, false);
    app.update();
    hold_down_interact(&mut app, true);
    app.update();

    assert_eq!(
        brain_slot(&app, home),
        Some(PlayerSlot::PRIMARY),
        "release hands the seat back to the home avatar"
    );
    assert_eq!(
        brain_slot(&app, actor),
        None,
        "release takes the seat off the actor, which resumes its own policy"
    );
    assert_eq!(
        faction_of(&app, actor),
        ActorFaction::Enemy,
        "authored faction unchanged across the whole possess/release cycle"
    );
    assert!(app
        .world()
        .resource::<PossessionState>()
        .possessed
        .is_none());
    // Vacate exit: the home avatar stepped out where the actor stood.
    let home_pos = app
        .world_mut()
        .query_filtered::<&BodyKinematics, With<PlayerEntity>>()
        .single(app.world())
        .unwrap()
        .pos;
    assert_eq!(home_pos, vec2(80.0, 0.0));
}

/// POSSESSION SUSPENDS RESIDENCY AND LEAVES THE LIFETIME ALONE.
///
/// `InCustodyOf`'s doc states the rule the promotion broke: *"the LIFETIME is unchanged, and that
/// is deliberate … no query that requires the scope silently loses sight of it"*. The query that
/// lost sight of it was `project_custody_onto_authored_occurrences`, which reads
/// `(With<InCustodyOf>, With<RoomScopedEntity>)` — so a possessed body was invisible to the
/// occurrence ledger and its home room AUTHORED A SECOND COPY behind the same
/// `SimId::placement(..)`.
///
///  the retirement the old assertion worried about ("so a room load can't
/// despawn it") needs no promotion: `RoomResident` is
/// `(With<RoomScopedEntity>, Without<InCustodyOf>)`, so the custody marker
/// already excludes a driven body from the sweep a room change runs — which
/// `a_possessed_body_is_carried_through_a_room_transition` proves against the
/// real transition.
///
///  and a body that IS destroyed while driven is separately handled:
/// [`losing_the_target_hands_control_back_to_home`] returns control to the home
/// avatar, which is what makes a new-game reset (a sweep of `RoomScopedEntity`
/// that deliberately does NOT exempt custody) safe.
#[test]
fn possession_suspends_residency_without_touching_the_lifetime() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        ActiveSessionScope, InCustodyOf, RoomScopedEntity, SessionScopedEntity,
    };

    let mut app = trigger_app();
    let mut scope = ActiveSessionScope::default();
    scope.begin();
    app.insert_resource(scope);

    let home = spawn_home(&mut app);
    let actor = spawn_candidate(&mut app, vec2(80.0, 0.0));
    // The candidate starts room-scoped, like every authored room actor.
    app.world_mut().entity_mut(actor).insert(RoomScopedEntity);
    let _ = home;

    let is_room_scoped = |app: &App, e: Entity| app.world().get::<RoomScopedEntity>(e).is_some();
    let in_custody = |app: &App, e: Entity| app.world().get::<InCustodyOf>(e).is_some();
    let session_scope =
        |app: &App, e: Entity| app.world().get::<SessionScopedEntity>(e).map(|s| s.0);
    assert!(is_room_scoped(&app, actor), "candidate begins room-scoped");
    assert_eq!(session_scope(&app, actor), None);
    assert!(!in_custody(&app, actor), "and nobody has custody of it");

    hold_down_interact(&mut app, true);
    app.update();
    app.update();

    assert!(
        is_room_scoped(&app, actor),
        "the possessed body KEEPS its room scope — the lifetime is not what changes"
    );
    assert_eq!(
        session_scope(&app, actor),
        None,
        "and it does not join the session scope: nothing about its lifetime moved"
    );
    assert!(
        in_custody(&app, actor),
        "what changed is RESIDENCY: a participant has custody of this body, which is \
         what excludes it from `RoomResident` and carries it through a door"
    );

    // Release.
    hold_down_interact(&mut app, false);
    app.update();
    hold_down_interact(&mut app, true);
    app.update();

    assert!(
        is_room_scoped(&app, actor),
        "release leaves the room scope exactly as it found it"
    );
    assert_eq!(session_scope(&app, actor), None);
    assert!(
        !in_custody(&app, actor),
        "and the custody is dropped, so the body is a resident of whatever room is \
         active now"
    );
}

#[test]
fn exactly_one_body_carries_the_player_brain_before_and_after() {
    let mut app = trigger_app();
    app.init_resource::<ControlledSubject>();
    app.add_systems(Update, resolve_controlled_subject);
    let home = spawn_home(&mut app);
    let actor = spawn_candidate(&mut app, vec2(80.0, 0.0));
    app.update();
    assert_eq!(app.world().resource::<ControlledSubject>().0, Some(home));

    hold_down_interact(&mut app, true);
    app.update(); // hold_timer = 1.0
    app.update(); // hold_timer = 2.0 → brain transfer commands queued
    app.update(); // transfer applied; resolver re-derives the subject
    assert_eq!(
        app.world().resource::<ControlledSubject>().0,
        Some(actor),
        "controlled subject follows the player brain onto the possessed actor"
    );
}

#[test]
fn a_brief_tap_does_not_possess() {
    let mut app = trigger_app();
    let _home = spawn_home(&mut app);
    let actor = spawn_candidate(&mut app, vec2(80.0, 0.0));
    hold_down_interact(&mut app, true);
    app.update();
    hold_down_interact(&mut app, false);
    app.update();
    assert_eq!(brain_slot(&app, actor), None, "a brief tap doesn't possess");
}

#[test]
fn out_of_range_actors_are_not_possessed() {
    let mut app = trigger_app();
    let _home = spawn_home(&mut app);
    let actor = spawn_candidate(&mut app, vec2(900.0, 0.0)); // far out of range
    hold_down_interact(&mut app, true);
    app.update();
    app.update();
    app.update();
    assert_eq!(
        brain_slot(&app, actor),
        None,
        "nothing in range → no transfer"
    );
}

/// The mandate's headline invariant: while controlling a possessed target, pressing attack emits
/// `ActorActionMessage` for the TARGET, and the vacated home avatar emits nothing.
#[test]
fn attack_while_controlling_target_emits_only_for_the_target() {
    use ambition_characters::actor::ActorPose;
    use ambition_characters::brain::{
        emit_brain_action_messages, ActionSet, ActorActionMessage, MeleeActionSpec, SwipeSpec,
    };

    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    let kit = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    // Vacated home avatar: neutral control (its brain was transferred away),
    // but it still owns a melee ActionSet + a pose.
    let home = app
        .world_mut()
        .spawn((ActorControl::default(), kit.clone(), ActorPose::default()))
        .id();
    // Possessed target: its seat produced a melee-pressed frame.
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.facing = 1.0;
    let target = app
        .world_mut()
        .spawn((ActorControl(frame), kit, ActorPose::default()))
        .id();

    app.add_systems(Update, emit_brain_action_messages);
    app.update();

    let msgs: Vec<_> = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
        .drain()
        .collect();
    let melee: Vec<_> = msgs.iter().filter(|m| m.is_melee()).collect();
    assert_eq!(melee.len(), 1, "exactly one melee action this frame");
    assert_eq!(
        melee[0].actor, target,
        "the attack originates from the possessed target"
    );
    assert!(
        melee.iter().all(|m| m.actor != home),
        "the vacated home avatar emits no attack"
    );
}

#[test]
fn losing_the_target_hands_control_back_to_home() {
    let mut app = trigger_app();
    let home = spawn_home(&mut app);
    let actor = spawn_candidate(&mut app, vec2(80.0, 0.0));
    hold_down_interact(&mut app, true);
    app.update();
    app.update();
    assert_eq!(brain_slot(&app, actor), Some(PlayerSlot::PRIMARY));
    // The possessed actor despawns (died / left the room).
    app.world_mut().entity_mut(actor).despawn();
    app.update();
    assert_eq!(
        brain_slot(&app, home),
        Some(PlayerSlot::PRIMARY),
        "the home avatar reclaims control when the possessed body is lost"
    );
    assert!(app
        .world()
        .resource::<PossessionState>()
        .possessed
        .is_none());
}

fn spawn_dead_candidate(app: &mut App, pos: ambition_platformer2d_core::Vec2) -> Entity {
    let e = spawn_candidate(app, pos);
    app.world_mut()
        .entity_mut(e)
        .insert(ambition_characters::actor::BodyHealth::new(
            ambition_characters::actor::Health {
                current: 0,
                max: 3,
                invulnerable: Default::default(),
            },
        ));
    e
}

#[test]
fn possession_skips_a_nearer_corpse_for_a_farther_living_body() {
    // A dead enemy is an intangible corpse — you cannot possess it, even when it
    // is the NEAREST brain-bearing body. (Enemies die and linger with ActorControl
    // + Brain + no PlayerEntity, so this is reachable.) Poison: drop the
    // body_is_corpse filter in possession_trigger_system and the nearer corpse is
    // possessed instead of the living body.
    let mut app = trigger_app();
    let home = spawn_home(&mut app);
    let corpse = spawn_dead_candidate(&mut app, vec2(40.0, 0.0)); // nearer, DEAD
    let living = spawn_candidate(&mut app, vec2(80.0, 0.0)); // farther, alive
    hold_down_interact(&mut app, true);
    app.update();
    app.update(); // crosses the hold threshold → transfer
    assert_eq!(
        brain_slot(&app, living),
        Some(PlayerSlot::PRIMARY),
        "the farther LIVING body is possessed"
    );
    assert_eq!(
        brain_slot(&app, corpse),
        None,
        "the nearer corpse is NOT possessed"
    );
    assert!(
        app.world().get::<DrivingParticipant>(home).is_none(),
        "the home avatar vacated into the living body"
    );
}

#[test]
fn possession_finds_no_target_in_a_world_of_only_corpses() {
    let mut app = trigger_app();
    let home = spawn_home(&mut app);
    let corpse = spawn_dead_candidate(&mut app, vec2(40.0, 0.0));
    hold_down_interact(&mut app, true);
    app.update();
    app.update();
    assert_eq!(
        brain_slot(&app, home),
        Some(PlayerSlot::PRIMARY),
        "no living candidate → the home avatar keeps the seat (no transfer)"
    );
    assert_eq!(
        brain_slot(&app, corpse),
        None,
        "a corpse is never possessed"
    );
}

/// A MOUNT TRAVELS WITH A PILOTED RIDER AND STAYS PUT UNDER AN AI ONE.
///
///  the transitive link, and both terms. A mount is in its rider's custody
/// exactly while that rider is itself travelling — so possessing the rider
/// carries the mount through a door, and an AI-piloted sky rider patrolling its
/// own room keeps its mount as room furniture.
///
///  the negative half is the one that matters. A rule that gave every
/// mount to its rider would pass the positive assertion and would quietly stop
/// every authored mount in the game from ever being retired with its room.
#[test]
fn a_mount_travels_with_a_piloted_rider_and_not_with_an_ai_one() {
    use ambition_platformer2d_shared_tangle::lifecycle::{InCustodyOf, RoomScopedEntity};

    let mut app = trigger_app();
    let home = spawn_home(&mut app);

    // The pair we will possess: a room-scoped rider on a room-scoped mount.
    let piloted_mount = app.world_mut().spawn(RoomScopedEntity).id();
    let rider = spawn_candidate(&mut app, vec2(80.0, 0.0));
    app.world_mut().entity_mut(rider).insert((
        RoomScopedEntity,
        ambition_mount::RidingOn {
            mount: piloted_mount,
        },
    ));

    // An AI-piloted pair, far away so it is never the possession candidate.
    let ai_mount = app.world_mut().spawn(RoomScopedEntity).id();
    let ai_rider = spawn_candidate(&mut app, vec2(4000.0, 0.0));
    app.world_mut().entity_mut(ai_rider).insert((
        RoomScopedEntity,
        ambition_mount::RidingOn { mount: ai_mount },
    ));

    hold_down_interact(&mut app, true);
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<PossessionState>().possessed,
        Some(rider),
        "setup: the near rider must be the one possessed, or neither assertion below \
         is about what it says it is"
    );
    let _ = home;

    assert_eq!(
        app.world().get::<InCustodyOf>(piloted_mount).map(|c| c.0),
        Some(rider),
        "the mount a PILOTED rider is on is in that rider's custody, so a room change \
         cannot retire it out from under the pilot"
    );
    assert!(
        app.world().get::<InCustodyOf>(ai_mount).is_none(),
        "an AI-piloted rider is room furniture and so is its mount — giving every \
         mount to its rider would stop authored mounts being retired with their room"
    );

    // Release: the mount goes back to being the room's.
    hold_down_interact(&mut app, false);
    app.update();
    hold_down_interact(&mut app, true);
    app.update();
    assert!(
        app.world().get::<InCustodyOf>(piloted_mount).is_none(),
        "letting go of the rider lets go of its mount"
    );
}

/// THE CUSTODY MARKER COMES BACK — it is DERIVED, and a rewind is what proves
/// it.
///
///  `InCustodyOf` is registered to rollback as a DERIVED component, excused
/// from the snapshot by one sentence: *"room residency reprojected from
/// `ItemCustody` every tick"*. A possessed body has no `ItemCustody`, so writing
/// the marker at the possess site would have created the one population nothing
/// reprojects — and a rewind past the possession would drop it with nothing to
/// put it back, leaving the driven body a `RoomResident` again and retiring it at
/// the next door.
///
///  so this deletes the marker the way a rollback restore would and asserts the
/// next tick rebuilds it from `PossessionState`, which IS snapshot state. A
/// version that only checked "the marker exists after possessing" would pass
/// against a plain insert and prove nothing about the rewind.
#[test]
fn the_driven_bodys_custody_marker_is_rederived_after_a_rewind_drops_it() {
    use ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf;

    let mut app = trigger_app();
    let home = spawn_home(&mut app);
    let actor = spawn_candidate(&mut app, vec2(80.0, 0.0));
    hold_down_interact(&mut app, true);
    app.update();
    app.update();
    assert_eq!(
        app.world().get::<InCustodyOf>(actor).map(|c| c.0),
        Some(home),
        "setup: the driven body wears the participant's custody"
    );

    // A rollback restore does not put derived components back.
    app.world_mut().entity_mut(actor).remove::<InCustodyOf>();
    assert!(
        app.world().get::<InCustodyOf>(actor).is_none(),
        "setup: the marker really is gone, so the assertion below is a REBUILD"
    );

    app.update();
    assert_eq!(
        app.world().get::<InCustodyOf>(actor).map(|c| c.0),
        Some(home),
        "the projection rebuilt the marker from `PossessionState`. Without it, a \
         rewind past a possession leaves the body you are driving a `RoomResident` \
         again, and the next door retires it"
    );
}

/// The projection does not touch an ITEM's custody, which the item domain
/// owns and reprojects from its own authority. A blanket retraction would fight
/// `project_custody_onto_residency` every tick.
#[test]
fn the_possession_projection_leaves_item_custody_alone() {
    use ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf;

    let mut app = trigger_app();
    let _home = spawn_home(&mut app);
    let carrier = app.world_mut().spawn_empty().id();
    // A ground item in somebody's custody, exactly as the item domain leaves it.
    let item = app
        .world_mut()
        .spawn((
            crate::items::pickup::GroundItem {
                spec: ambition_characters::brain::HeldItemSpec {
                    id: "axe".into(),
                    melee: None,
                    ranged: None,
                    use_behavior: ambition_characters::brain::HeldUseBehavior::ThrowOnUse,
                },
                pos: vec2(0.0, 0.0),
                vel: vec2(0.0, 0.0),
                half_extent: vec2(8.0, 8.0),
            },
            InCustodyOf(carrier),
        ))
        .id();

    // Nobody is possessing anything: the projection's retraction arm runs.
    app.update();
    app.update();
    assert_eq!(
        app.world().get::<InCustodyOf>(item).map(|c| c.0),
        Some(carrier),
        "the item's custody survived a tick with no possession — the projection \
         retracts only the marker IT owns, and the item domain owns this one"
    );
}

/// A candidate that was ALREADY session-scoped keeps that scope exactly.
///
/// Nothing promotes now, so the interesting claim is the stronger one: the scope is never written
/// at all, in either direction. That is the poison for reintroducing a promotion, because a
/// promote/restore pair would look identical at the end and differ HERE, in the middle.
#[test]
fn a_session_scoped_candidate_keeps_its_own_scope_through_possession() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        ActiveSessionScope, InCustodyOf, RoomScopedEntity, SessionScopeId, SessionScopedEntity,
    };

    let mut app = trigger_app();
    let mut active = ActiveSessionScope::default();
    let active_id = active.begin();
    app.insert_resource(active);
    let _home = spawn_home(&mut app);
    let actor = spawn_candidate(&mut app, vec2(80.0, 0.0));
    // Deliberately NOT the active session's id: a promotion would overwrite it
    // with `active_id`, and that difference is the whole measurement.
    let original_id = SessionScopeId(active_id.0 + 41);
    app.world_mut()
        .entity_mut(actor)
        .insert(SessionScopedEntity(original_id));

    hold_down_interact(&mut app, true);
    app.update();
    app.update();
    assert_eq!(
        app.world()
            .get::<SessionScopedEntity>(actor)
            .map(|scope| scope.0),
        Some(original_id),
        "the driven body keeps ITS OWN session scope — possession does not move a \
         body onto the active session's scope, it suspends the body's residency"
    );
    assert!(app.world().get::<RoomScopedEntity>(actor).is_none());
    assert!(
        app.world().get::<InCustodyOf>(actor).is_some(),
        "and custody is what marks it as driven"
    );

    hold_down_interact(&mut app, false);
    app.update();
    hold_down_interact(&mut app, true);
    app.update();
    assert_eq!(
        app.world()
            .get::<SessionScopedEntity>(actor)
            .map(|scope| scope.0),
        Some(original_id),
        "and release leaves it exactly as it was"
    );
    assert!(app.world().get::<RoomScopedEntity>(actor).is_none());
    assert!(app.world().get::<InCustodyOf>(actor).is_none());
}

/// An UNSCOPED candidate stays unscoped. Possession invents no lifetime for
/// a body that had none, and release invents no room ownership either.
#[test]
fn an_unscoped_candidate_is_never_given_a_lifetime_by_possession() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        ActiveSessionScope, InCustodyOf, RoomScopedEntity, SessionScopedEntity,
    };

    let mut app = trigger_app();
    let mut active = ActiveSessionScope::default();
    active.begin();
    app.insert_resource(active);
    let _home = spawn_home(&mut app);
    let actor = spawn_candidate(&mut app, vec2(80.0, 0.0));
    assert!(app.world().get::<RoomScopedEntity>(actor).is_none());
    assert!(app.world().get::<SessionScopedEntity>(actor).is_none());

    hold_down_interact(&mut app, true);
    app.update();
    app.update();
    assert!(
        app.world().get::<SessionScopedEntity>(actor).is_none(),
        "an unscoped body is NOT promoted while controlled — it used to be, and that \
         promotion is what hid a possessed body from the occurrence ledger"
    );
    assert!(
        app.world().get::<InCustodyOf>(actor).is_some(),
        "it is marked as being in a participant's custody instead"
    );

    hold_down_interact(&mut app, false);
    app.update();
    hold_down_interact(&mut app, true);
    app.update();
    assert!(
        app.world().get::<RoomScopedEntity>(actor).is_none(),
        "release must not invent room ownership"
    );
    assert!(
        app.world().get::<SessionScopedEntity>(actor).is_none(),
        "release returns the candidate to its original unscoped state"
    );
    assert!(app.world().get::<InCustodyOf>(actor).is_none());
}
