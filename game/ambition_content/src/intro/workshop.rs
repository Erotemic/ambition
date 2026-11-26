//! Physical results that advance the workshop story.

use ambition_combat::events::SetFlagsRequested;
use ambition_platformer2d::characters::{actor::CharacterForm, control::DrivingParticipant};
use ambition_platformer2d::damage::BodyHitResolved;
use ambition_platformer2d::engine_core::{BodyKinematics, SweepSample, Vec2};
use ambition_platformer2d_core::replay_messages::ReplayMessageReader;
use ambition_platformer2d_shared_tangle::{
    authored_logic::{AuthoredArg, CommandContext, CommandDescriptor, CommandId, CommandOutcome, ConditionDescriptor, ConditionId, ConditionOutcome, PublishCommand, PublishCondition, WhyNot},
    lifecycle::{AuthoredOccurrences, InRoomInstance, OccurrenceWhereabouts, RoomReplay, RoomWrite},
    schedule::Platformer2dSimulationPhase,
    sim_id::SimId,
};
use ambition_platformer2d_world::rooms::LiveRoomSpecs;
use ambition_held_items::{GroundItem, ItemCustody};
use bevy::ecs::system::SystemState;
use bevy::prelude::{App, Commands, Entity, IntoScheduleConfigs, MessageWriter, Query, Res, ResMut, World};

const CASE: &str = "intro_bench_case";
const BOMB: &str = "intro_brute_bomb";
const BREACH_BOMB: &str = "intro_breach_bomb";
const SIDING: &str = "intro_freight_yard";
const RECOVERY_MARK: &str = "intro_case_recovery";
const WORKSHOP: &str = "intro_wake_room";

pub(super) fn install(app: &mut App, schedule: bevy::ecs::schedule::InternedScheduleLabel) {
    app.publish_condition(
        ConditionDescriptor {
            id: ConditionId::new("workshop", "raid_in_progress"),
            summary: "the raiders have been called and the case is still in the workshop",
            params: &[],
        },
        |world, _| {
            let save = world.resource::<ambition_persistence::save::AmbitionGameSave>();
            let data = save.data();
            ConditionOutcome::from_bool(
                data.flag("intro_raiders_coming") && !data.flag("intro_case_in_transit")
                    && !data.flag("intro_creator_fragment_seen"),
                || WhyNot::new("workshop.raid_in_progress", WORKSHOP, "the raid is outside its fight phase"),
            )
        },
    );
    app.publish_command(
        CommandDescriptor {
            id: CommandId::new("workshop", "ship_case"),
            summary: "send the case in the brute's custody to the freight siding",
            params: &[],
        },
        ship_case,
    );
    app.add_systems(
        schedule,
        observe_workshop
            .before(ambition_platformer2d_runtime::cutscene::drain_cutscene_triggers)
            .in_set(Platformer2dSimulationPhase::Cutscene),
    );
}

fn observe_workshop(
    mut commands: Commands,
    mut hits: ReplayMessageReader<BodyHitResolved>,
    tick: Res<ambition_time::SimTick>,
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    specs: Option<LiveRoomSpecs>,
    replay: Option<Res<RoomReplay>>,
    cast: Query<&CharacterForm>,
    drivers: Query<&DrivingParticipant>,
    doors: Query<(Entity, &SimId, &ambition_platformer2d::breakables::BreakableFeature)>,
    objects: Query<(Entity, &SimId, &ItemCustody, Option<&ambition_held_items::ErrandPickupOnly>), bevy::prelude::With<GroundItem>>,
    mut flags: MessageWriter<SetFlagsRequested>,
    mut scenes: ResMut<ambition_platformer2d::cutscene::CutsceneTriggerQueue>,
    active: Res<ambition_platformer2d::cutscene::ActiveCutscene>,
    beaten: Query<(Entity, &CharacterForm, &ambition_platformer2d::characters::actor::BodyHealth, &ambition_combat::components::CombatCapabilities)>,
) {
    let Some(specs) = specs else { return };
    let rooms = specs.live();
    let allowed = |room| replay.as_ref().is_none_or(|replay| replay.allows(room));
    let name_of = |room| {
        specs.definition_in(room).map(|definition| specs.rooms().spec(definition).id.as_str())
    };
    let mut record = |flag: &str, room| {
        if !save.data().flag(flag) && allowed(Some(room)) {
            flags.write(SetFlagsRequested {
                reply: None,
                changes: vec![(flag.to_string(), true)],
                written: Some(RoomWrite { room, tick: tick.0 }),
            });
        }
    };
    let bomb_id = SimId::placement(BOMB);
    for hit in hits.read() {
        if hit.attacker_identity.as_ref().and_then(|id| id.id.as_ref()) != Some(&bomb_id)
            || hit.resolution.damage_applied() <= 0
            || !cast.get(hit.body).is_ok_and(|form| form.id() == "npc_creator")
        {
            continue;
        }
        let Some(room) = hit.identity.room else { continue };
        if name_of(room) == Some(WORKSHOP) {
            record("intro_dennis_wounded", room);
        }
    }
    let door = SimId::placement("intro_barn_door");
    for (entity, id, state) in &doors {
        if *id == door && state.broken() {
            if let Some(room) = rooms.of(entity) {
                record("intro_barn_door_broken", room);
            }
        }
    }
    let case = SimId::placement(CASE);
    let breach_bomb = SimId::placement(BREACH_BOMB);
    for (entity, id, custody, restriction) in &objects {
        let story_item = [&case, &bomb_id, &breach_bomb].contains(&id);
        if !story_item && restriction.is_none() {
            continue;
        }
        let anchor = match custody {
            ItemCustody::Held { holder } => *holder,
            _ => entity,
        };
        let Some(room) = rooms.of(anchor).filter(|room| allowed(Some(*room))) else { continue };
        let name = name_of(room);
        let reserve = story_item && name == Some(WORKSHOP);
        if reserve && restriction.is_none() {
            commands.entity(entity).insert(ambition_held_items::ErrandPickupOnly);
        } else if !reserve && restriction.is_some() {
            commands.entity(entity).remove::<ambition_held_items::ErrandPickupOnly>();
        }
        if *id == case && name == Some(SIDING) {
            if let ItemCustody::Held { holder } = custody {
                if drivers.contains(*holder) {
                    if !save.data().flag("intro_creator_notes_recovered") {
                        scenes.request_in("intro_case_recovered", room.ordinal());
                    }
                }
            }
        }
    }
    if save.data().flag("intro_case_in_transit") && !save.data().flag("intro_creator_fragment_seen") {
        for (room, definition) in specs.live_rooms() {
            if specs.rooms().spec(definition).id != WORKSHOP || !allowed(Some(room))
                || active.iter().any(|scene| scene.room == Some(room.ordinal()))
                || beaten.iter().any(|(body, form, _, _)| form.id() == "npc_raid_brute" && rooms.of(body) == Some(room))
            {
                continue;
            }
            record("intro_brute_scene_seen", room);
            scenes.request_in("creator_final_fragment", room.ordinal());
        }
        return;
    }
    if save.data().flag("intro_brute_scene_seen") {
        return;
    }
    for (entity, form, health, caps) in &beaten {
        let remaining = health.current();
        if form.id() != "npc_raid_brute"
            || remaining <= 0
            || !caps.beaten_at.is_some_and(|floor| remaining <= floor)
        {
            continue;
        }
        let Some(room) = rooms.of(entity).filter(|room| allowed(Some(*room))) else { continue };
        if name_of(room) == Some(WORKSHOP)
            && !active.holds(Some(room.ordinal()))
            && !active.plays_in("intro_brute_takes_the_case", Some(room.ordinal()))
        {
            scenes.request_in("intro_brute_takes_the_case", room.ordinal());
        }
    }
}

fn ship_case(world: &mut World, _: &[AuthoredArg], context: &CommandContext) -> CommandOutcome {
    let Some(room) = context.room else { return CommandOutcome::refused("the case is in the workshop"); };
    let mut state: SystemState<(
        Commands,
        LiveRoomSpecs,
        ResMut<AuthoredOccurrences>,
        Option<Res<ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry>>,
        Query<(&CharacterForm, ambition_combat::hand::RepertoireQuery), bevy::prelude::Without<GroundItem>>,
        Query<(Entity, &SimId, &mut ItemCustody, &mut BodyKinematics), bevy::prelude::With<GroundItem>>,
    )> = SystemState::new(world);
    let Ok((mut commands, specs, mut ledger, geometry, mut cast, mut items)) = state.get_mut(world) else {
        return CommandOutcome::refused("the case needs the live room and item ledger");
    };
    let Some(destination) = specs.rooms().definition_by_id(SIDING) else {
        return CommandOutcome::refused("the freight siding is absent");
    };
    let Some(mark) = specs.rooms().spec(destination).debug_labels.iter().find(|mark| mark.name == RECOVERY_MARK) else {
        return CommandOutcome::refused("the freight siding has no case mark");
    };
    let at: Vec2 = mark.payload.position;
    let live_targets: Vec<_> = specs.live_rooms().filter(|(_, definition)| *definition == destination).map(|(id, _)| id).collect();
    let case = SimId::placement(CASE);
    let Some((entity, _, mut custody, mut body)) = items.iter_mut().find(|(_, id, custody, _)| {
        *id == &case && matches!(**custody, ItemCustody::Held { holder } if specs.live().of(holder) == Some(room))
    }) else {
        return CommandOutcome::refused("the case is absent");
    };
    let ItemCustody::Held { holder } = *custody else {
        return CommandOutcome::refused("the case has no carrier");
    };
    let Ok((form, mut hand)) = cast.get_mut(holder) else {
        return CommandOutcome::refused("the case carrier is absent");
    };
    if form.id() != "npc_raid_brute" || specs.live().of(holder) != Some(room) || live_targets.len() > 1 {
        return CommandOutcome::refused("the brute must carry the case to one siding");
    }
    match ledger.whereabouts(&case) {
        None | Some(OccurrenceWhereabouts::InCustody) => {}
        Some(OccurrenceWhereabouts::Placed { room, .. }) if room == WORKSHOP => {}
        _ => return CommandOutcome::refused("the case already left the workshop"),
    }
    let placement = if let Some(&target) = live_targets.first() {
        let desired = BodyKinematics { pos: at, vel: Vec2::ZERO, ..*body };
        let admission = geometry.as_deref().and_then(|geometry| geometry.motion_for(entity, Some(target)));
        let pose = ambition_platformer2d::engine_core::movement::prepare_body_materialization(desired, at, admission, []);
        let Ok(pose) = pose else { return CommandOutcome::refused("the siding cannot admit the case"); };
        Some((target, pose))
    } else {
        None
    };
    // The carried case becomes a shipment when it leaves the workshop.
    let mut shipped = ledger.with_custody_held(&[case.clone()].into_iter().collect());
    if !shipped.republish_placements(SIDING, [(case, at)].into_iter().collect()).is_empty() {
        return CommandOutcome::refused("the ledger could not place the case at the siding");
    }
    *ledger = shipped;
    ambition_held_items::empty_hand(&mut commands, holder, &mut hand);
    if let Some((target, pose)) = placement {
        pose.commit(&mut body, None);
        *custody = ItemCustody::InWorld;
        commands.entity(entity)
            .insert((InRoomInstance(target), SweepSample::at_tick_end(*body, body.vel)))
            .remove::<(ambition_held_items::ErrandPickupOnly, ambition_held_items::SettledItem)>();
    } else {
        commands.entity(entity).despawn();
    }
    state.apply(world);
    world.write_message(SetFlagsRequested {
        reply: None,
        changes: vec![("intro_case_in_transit".into(), true)],
        written: world.get_resource::<ambition_time::SimTick>().map(|tick| RoomWrite { room, tick: tick.0 }),
    });
    CommandOutcome::Done
}
