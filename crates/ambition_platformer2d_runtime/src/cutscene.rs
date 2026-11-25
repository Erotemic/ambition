//! Cutscene playback runtime (the systems that drive the scripts).
//!
//! The cutscene SCRIPT format + stepper + live playback-state resources live in
//! the foundation crate [`ambition_cutscene`] (pure data + ECS resources, no
//! renderer). This module is the gameplay-side *player*: it reads triggers from
//! [`ambition_cutscene::CutsceneTriggerQueue`], starts/advances the active
//! cutscene, and applies its side effects (save-flag writes via
//! [`ambition_persistence::save::AmbitionGameSave`]). The HUD/overlay presentation
//! reads `ActiveCutscene` from the render crate.
//!
//! These systems are gameplay-coupled (rooms, save, schedule) so they live in
//! the runtime that installs them rather than in `ambition_cutscene`, which
//! sits below it and must stay content- and gameplay-free.

use bevy::prelude::*;

use ambition_cutscene::{ActiveCutscene, CutsceneEvent, CutsceneLibrary, CutsceneRuntime, PlayingCutscene};
use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;

use ambition_cutscene::CutsceneTriggerQueue;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

/// Bevy system: when a room id becomes live, queue up the cutscene the room
/// says it starts (`RoomMetadata::entry_cutscene`, the LDtk level field
/// `entry_cutscene`). `drain_cutscene_triggers` skips a cutscene that was seen.
///
/// Every live room (OW1 Cut C): the sole live room was read, so while two
/// rooms were live no room-entry cutscene was queued. A second live room of
/// an id already live queues nothing. With no room live the memory is kept.
pub fn auto_trigger_room_cutscenes(
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    mut queue: ResMut<CutsceneTriggerQueue>,
    mut last_rooms: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<ambition_cutscene::LastCutsceneRoom>,
) {
    // Each live room's id, its entry cutscene, and the room it plays for.
    let mut live: Vec<(String, Option<String>, u32)> = rooms
        .live_rooms()
        .map(|(instance, definition)| {
            let spec = rooms.rooms().spec(definition);
            (spec.id.clone(), spec.metadata.entry_cutscene.clone(), instance.ordinal())
        })
        .collect();
    if live.is_empty() {
        return;
    }
    live.sort();
    live.dedup_by(|a, b| a.0 == b.0);
    let ids: Vec<String> = live.iter().map(|(id, ..)| id.clone()).collect();
    // Read through the immutable deref: on every frame but a change there is
    // nothing to write, and a `DerefMut` would mark the resource changed.
    if last_rooms.0 == ids {
        return;
    }
    // In sorted room order, so a resimulation queues the same cutscenes in
    // the same order.
    for (_, cutscene, room) in live.iter().filter(|(id, ..)| !last_rooms.0.contains(id)) {
        if let Some(cutscene) = cutscene {
            queue.request_in(cutscene, *room);
        }
    }
    last_rooms.0 = ids;
}

/// Queue the cutscene of each authored trigger volume a player body stands
/// in (`RoomSpec::cutscene_triggers`), for the volume's room.
///
/// It asks every tick a body is inside, and that is safe only for a script
/// that remembers it was seen: the queue keeps one copy, the drain skips a
/// seen script, and the seen flag is applied (`GameplayEffects`) on the tick
/// the cutscene ends, before this phase runs again. So a trigger whose script
/// has no seen flag, or one already seen, asks nothing.
pub fn trigger_cutscenes_in_volumes(
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    players: Query<(Entity, &ambition_platformer2d_core::BodyKinematics), With<ambition_characters::control::DrivingParticipant>>,
    library: Res<CutsceneLibrary>,
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    active: Res<ActiveCutscene>,
    mut queue: ResMut<CutsceneTriggerQueue>,
) {
    for (instance, definition) in rooms.live_rooms() {
        for trigger in &rooms.rooms().spec(definition).cutscene_triggers {
            let unseen = library
                .get(&trigger.cutscene)
                .and_then(|script| script.seen_flag.as_deref())
                .is_some_and(|seen| !save.data().flag(seen));
            if !unseen || active.plays_in(&trigger.cutscene, Some(instance.ordinal())) {
                continue;
            }
            // A scene that belongs after a fight waits for its clear.
            let fought = trigger.after_encounter.as_deref().is_none_or(|encounter| {
                save.data().encounter(encounter) == ambition_persistence::save_data::PersistedEncounterState::Cleared
            });
            if !fought {
                continue;
            }
            let (min, max) = (trigger.min, trigger.min + trigger.size);
            let inside = players.iter().filter(|(body, _)| live.of(*body) == Some(instance)).any(|(_, kin)| {
                let half = kin.size() * 0.5;
                let (lo, hi) = (kin.pos - half, kin.pos + half);
                lo.x < max.x && hi.x > min.x && lo.y < max.y && hi.y > min.y
            });
            if inside {
                queue.request_in(trigger.cutscene.clone(), instance.ordinal());
            }
        }
    }
}

/// Drain the trigger queue: start each cutscene whose room has none playing
/// (SIMULATION-ISLANDS N4). Skips any that have already had their seen flag
/// set.
pub fn drain_cutscene_triggers(
    mut queue: ResMut<CutsceneTriggerQueue>,
    library: Res<CutsceneLibrary>,
    mut active: ResMut<ActiveCutscene>,
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    replay: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::RoomReplay>>,
    settings: ambition_platformer2d_world::governing_rules::RulesOf<ambition_cutscene::CutsceneSettings>,
    (rooms, bodies): (
        ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
        Query<(
            Entity,
            &ambition_characters::actor::CharacterForm,
            &ambition_platformer2d_shared_tangle::sim_id::SimId,
            Option<&ambition_characters::actor::BodyHealth>,
        )>,
    ),
) {
    // One trigger starts per room; the rest wait their turn. Two rooms entered
    // on one tick each start their entry cutscene, and each room is announced
    // once (`LastCutsceneRoom`), so a trigger kept waiting is never dropped. A
    // cutscene of the whole world waits for every room's to end, and the
    // triggers behind it wait with it.
    let mut pending = std::mem::take(&mut queue.0).into_iter();
    while let Some(trigger) = pending.next() {
        if replay.as_ref().is_some_and(|replay| !replay.allows(trigger.room.map(LiveRoomInstance::from_ordinal))) {
            queue.0.push(trigger);
            continue;
        }
        let Some(script) = library.get(&trigger.id) else {
            continue;
        };
        if let Some(seen) = script.seen_flag.as_ref() {
            if save.data().flag(seen) {
                continue;
            }
        }
        if !active.can_start(trigger.room) || active.plays_in(&trigger.id, trigger.room) {
            let of_the_world = trigger.room.is_none();
            queue.0.push(trigger);
            if of_the_world {
                break;
            }
            continue;
        }
        let mut runtime = CutsceneRuntime::new(script.clone().admit(|flag| save.data().flag(flag)));
        runtime.settings = match trigger.room {
            Some(room) => settings.in_live_room(Some(LiveRoomInstance::from_ordinal(room))),
            None => settings.of_subject(None),
        }.copied().unwrap_or_default();
        let roles = bind_roles(script, trigger.room, &rooms, &bodies);
        active.start(PlayingCutscene::new(runtime, trigger.room).with_roles(roles));
    }
    queue.0.extend(pending);
}

/// Bind each of `script`'s roles to one live body as it starts (SS7): the
/// one body of a character in the scene's room (in the world, for a scene of
/// the whole world), or the body of a placement. A role with no such body,
/// or with more than one, stays unbound: each call naming it is refused,
/// never given another body of the character.
fn bind_roles(
    script: &ambition_cutscene::CutsceneScript,
    room: Option<u32>,
    rooms: &ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    bodies: &Query<(
        Entity,
        &ambition_characters::actor::CharacterForm,
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        Option<&ambition_characters::actor::BodyHealth>,
    )>,
) -> Vec<(String, String)> {
    use ambition_cutscene::RoleCast;
    let mut bound = Vec::new();
    for (role, cast) in &script.roles {
        let here = |body: Entity| room.is_none() || rooms.of(body).map(|at| at.ordinal()) == room;
        let live = |health: Option<&ambition_characters::actor::BodyHealth>| health.is_none_or(|health| health.health.current > 0);
        let found: Vec<&ambition_platformer2d_shared_tangle::sim_id::SimId> = bodies
            .iter()
            .filter(|(body, worn, id, health)| {
                here(*body)
                    && live(*health)
                    && match cast {
                        RoleCast::Character(character) => worn.0.as_str() == character,
                        RoleCast::Placement(placement) => **id == ambition_platformer2d_shared_tangle::sim_id::SimId::placement(placement),
                    }
            })
            .map(|(_, _, id, _)| id)
            .collect();
        match found.as_slice() {
            [one] => bound.push((role.clone(), one.as_str().to_owned())),
            none_or_many => warn!(
                "cutscene `{}`: role `@{role}` ({cast:?}) found {} live bodies, so it is not bound",
                script.id,
                none_or_many.len()
            ),
        }
    }
    bound
}

/// A playing cutscene's bound roles, as the authored-call boundary takes them.
/// A run's roles, each naming its body in the run's own live room: the same
/// script playing in two live instances of one room binds the same ids.
fn role_bindings(roles: &[(String, String)], room: Option<u32>) -> ambition_platformer2d_shared_tangle::authored_logic::RoleBindings {
    roles.iter().fold(Default::default(), |bindings: ambition_platformer2d_shared_tangle::authored_logic::RoleBindings, (role, id)| {
        bindings.bind(role.clone(), ambition_platformer2d_shared_tangle::sim_id::SimId::from_snapshot(id.clone()))
    }).in_room(room.map(LiveRoomInstance::from_ordinal))
}

/// END a cutscene: mark it seen and tear it down. THE ONE ENDING.
///
/// ⛔⛔ THIS RULE WAS WRITTEN TWICE, once per way a cutscene can stop. Skipping
/// and completing each read `script.seen_flag`, set it, nulled the runtime and
/// reset the presentation -- the same four steps, side by side in one function.
/// ⇒ "What it MEANS for a cutscene to end" had two authorities, and a third
/// ending (a cancel, a room change, an abandoned session) would have had to
/// remember all four steps from scratch. Forgetting the flag is the expensive
/// half: the cutscene has no record of having played, so it plays again every
/// time its trigger is met.
///
/// ⚠ The flag is `Option` because not every script asks to be remembered; a
/// script with no `seen_flag` ends without recording anything, which is a
/// different thing from ending without being marked.
///
/// Ending takes the cutscene out of [`ActiveCutscene`]: its playback, picture,
/// room and skip hold go with it. Returns the seen flag to set, which the
/// caller writes with the cutscene's other flags ([`write_cutscene_flags`]).
#[must_use]
fn end_cutscene(cutscene: PlayingCutscene) -> Option<String> {
    cutscene.runtime.script.seen_flag.clone()
}

/// Write the flags a cutscene set this tick, dated in the room it plays for
/// (ISL-2d): through `SetFlagsRequested`, so a rewind of that room takes them
/// back and the quests they moved, as it does a switch's. A composition with
/// no flag channel writes the save directly.
fn write_cutscene_flags(
    flags: Vec<(String, bool)>,
    written: Option<ambition_platformer2d_shared_tangle::lifecycle::RoomWrite>,
    requests: Option<&mut Messages<ambition_combat::events::SetFlagsRequested>>,
    save: &mut ambition_persistence::save::AmbitionGameSave,
) {
    match requests {
        Some(requests) => {
            for (id, on) in flags {
                requests.write(ambition_combat::events::SetFlagsRequested {
                    changes: vec![(id, on)],
                    reply: None,
                    written
                });
            }
        }
        None => {
            for (id, on) in flags {
                save.data_mut().set_flag(id, on);
            }
        }
    }
}

/// Run condition of [`tick_active_cutscene`]: a cutscene plays. Otherwise the
/// tick would only write the resource back unchanged.
pub fn a_cutscene_plays(active: bevy::prelude::Res<ActiveCutscene>) -> bool {
    active.is_playing()
}

/// Run condition of [`project_cutscene_hold`]: a cutscene plays, or a body
/// it held is still owed its release.
pub fn a_cutscene_hold_is_owed(
    active: Res<ActiveCutscene>,
    held: Query<&ambition_characters::control::ControlHolds>,
    floors: Query<(), With<ambition_combat::components::SceneDamageFloor>>,
) -> bool {
    active.is_playing() || !floors.is_empty()
        || held.iter().any(|holds| holds.holds(ambition_characters::control::ControlHold::Cutscene))
}

/// Hold every body in a playing cutscene's room (every body, for a cutscene
/// of the whole world), and release it when the cutscene ends. In a room's
/// replay, only that room's bodies.
///
/// The cutscene's input context only quiets the seats; the room's hostiles
/// kept their brains and struck a player the dialogue was holding still. A
/// held body's frame is blanked (`blank_scripted_control_frames`), so nobody
/// in the room starts a move, and it cannot be hurt, so a strike already in
/// flight lands on nobody. Bodies in other rooms play on (ISL-1b).
///
/// A body on its way somewhere it was sent (a pending visit: an
/// `actor.walk_to` or `actor.leave_through` of this very scene, or a life
/// going about its day) is not held while it goes: the scene directed it,
/// and it is held again when it gets there.
///
/// A projection of [`ActiveCutscene`], rewritten each tick, so a rewind that
/// restores the cutscene restores the hold with it.
pub fn project_cutscene_hold(
    mut commands: Commands,
    active: Res<ActiveCutscene>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    replay: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::RoomReplay>>,
    mut bodies: Query<
        (
            Entity,
            Option<&mut ambition_characters::control::ControlHolds>,
            Option<&mut ambition_characters::actor::BodyHealth>,
            Option<&ambition_platformer2d_actor::body::visit::Visit>,
            Option<&ambition_platformer2d_actor::body::errand::Errand>,
            Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
            Option<&ambition_combat::components::SceneDamageFloor>,
        ),
        With<ambition_characters::control::ActorControl>,
    >,
) {
    use ambition_characters::actor::Invulnerability;
    use ambition_characters::control::ControlHold;
    use ambition_combat::components::SceneDamageFloor;
    for (body, holds, health, visit, errand, identity, floor) in &mut bodies {
        if replay.as_ref().is_some_and(|replay| !replay.allows(rooms.of(body))) {
            continue;
        }
        let going = visit.is_some_and(|visit| visit.outcome == ambition_platformer2d_actor::body::visit::VisitOutcome::Pending);
        let fetching = errand.is_some_and(|errand| errand.outcome
            == ambition_platformer2d_actor::body::errand::ErrandOutcome::Pending);
        let room = rooms.of(body).map(|room| room.ordinal());
        let held = !going && !fetching && active.holds(room);
        let exposure = identity.and_then(|id| {
            active.iter().filter(|scene| scene.room.is_none() || scene.room == room).find_map(|scene| {
                scene.runtime.script.beats[..scene.runtime.beat_index].iter().rev().find_map(|beat| {
                    let ambition_cutscene::CutsceneBeat::AllowDamage { role, minimum_health } = beat else { return None };
                    scene.roles.iter().any(|(bound, body)| bound == role && body == id.as_str())
                        .then_some(*minimum_health)
                })
            })
        });
        match (floor, exposure.flatten().map(SceneDamageFloor)) {
            (old, Some(new)) if old != Some(&new) => { commands.entity(body).insert(new); }
            (Some(_), None) => { commands.entity(body).remove::<SceneDamageFloor>(); }
            _ => {}
        }
        match holds {
            Some(mut holds) if holds.holds(ControlHold::Cutscene) != held => {
                if held {
                    holds.claim(ControlHold::Cutscene);
                } else {
                    ambition_characters::control::release_control_hold(
                        &mut commands,
                        body,
                        Some(&mut *holds),
                        ControlHold::Cutscene,
                    );
                }
            }
            Some(_) => {}
            None if held => {
                commands.entity(body).try_insert(ambition_characters::control::ControlHolds::only(ControlHold::Cutscene));
            }
            None => {}
        }
        if let Some(mut health) = health {
            let protected = held && exposure.is_none();
            if health.health.invulnerable.holds(Invulnerability::CUTSCENE) != protected {
                health.health.invulnerable.set(Invulnerability::CUTSCENE, protected);
            }
        }
    }
}

/// The seat that answers a playing cutscene ([`PlayingCutscene::answering_seat`]).
fn cutscene_seat(
    active: &PlayingCutscene,
    drivers: &Query<(&ambition_characters::control::DrivingParticipant, Entity)>,
    rooms: &ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) -> ambition_characters::control::PlayerSlot {
    active.answering_seat(
        drivers.iter().map(|(driver, body)| (driver.0, rooms.of(body).map(|at| at.ordinal()))),
        ambition_characters::control::PlayerSlot::PRIMARY,
    )
}

/// Advance the playing cutscene by one SIMULATION step.
///
/// ⛔⛤ **THE CLOCK IS `WorldTime`, NOT `Res<Time>`, FOR TWO SEPARATE REASONS.**
/// This system is registered into
/// `Platformer2dSimulationPhase::Cutscene` (see `:181`), and
/// `sim_schedule()` IS `Update` under the `RenderFrame` host — so on the frame
/// clock a cutscene's beat timings would depend on how fast the machine draws,
/// and two replays of the same input stream could enter different beats.
///
/// ⭐ And `WorldTime::sim_dt` is SCALED (`ambition_time/src/lib.rs:209` returns
/// `sim_dt`), which a frame clock is not: a cutscene playing under slow
/// motion slows with the scene it accompanies instead of running at wall speed
/// over a world in treacle.
///
/// ⇒ Playback state advances on the sim clock and only PRESENTATION reads the
/// wall clock (`PresentationTime::wall_dt`) — the deterministic-elapsed half of
/// `queue.md`'s `CUTSCENE-ROLLBACK-DECISION`. ⚠ That route said "the
/// cutscene-authority row in `tracks.md`" until 2026-09-18; `tracks.md` does
/// not mention cutscenes at all, and this doc block was itself attached to
/// `end_cutscene` rather than to the function it describes.
///
/// ⭐ THE PLAYER'S DISMISS AND SKIP ARE READ FROM THE PRIMARY SEAT'S
/// `ControlFrame` (`confirm_pressed`, `cancel_held`), the input this tick was
/// given. A replay and a peer receive the same frame, so a rewind across the
/// press re-delivers it. The primary seat owns the shell controls, as it owns
/// `MenuControlFrame`.
pub fn tick_active_cutscene(
    #[cfg(feature = "dialogue")]
    dialogue_settings: ambition_platformer2d_world::governing_rules::RulesOf<ambition_dialog::DialogSettings>,
    #[cfg(feature = "dialogue")]
    (voice_catalog, mut dialogue_sfx): (
        Option<Res<ambition_sfx::voice::DialogueVoiceCatalog>>,
        Option<ambition_sfx::SfxWriter>,
    ),
    positions: Query<(Entity, &ambition_platformer2d_shared_tangle::sim_id::SimId, &ambition_platformer2d_core::BodyKinematics)>,
    time: Res<ambition_time::WorldTime>,
    controls: Res<ambition_characters::control::SlotControls>,
    mut active: ResMut<ActiveCutscene>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    // The seats and the rooms of the bodies they drive: a room's cutscene
    // is skipped and dismissed by a seat in its room. And the flag channel
    // and the tick its flags are dated on. And its room's pace.
    (drivers, rooms, mut requests, tick, freeze, mut cast): (
        Query<(&ambition_characters::control::DrivingParticipant, Entity)>,
        ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
        Option<ResMut<Messages<ambition_combat::events::SetFlagsRequested>>>,
        Option<Res<ambition_time::SimTick>>,
        ambition_platformer2d_shared_tangle::lifecycle::RoomFreeze,
        // The bodies a `Fall` beat can name: by the character they wear; and
        // what a death of each keeps (its id and its respawn policy).
        Query<(
            Entity,
            &ambition_characters::actor::CharacterForm,
            &mut ambition_characters::actor::BodyHealth,
            Option<(&ambition_combat::components::ActorIdentity, &ambition_combat::actor_tuning::ActorConfig)>,
        )>,
    ),
    // A `Command` beat's line, prepared and asked through the authored
    // command channel; a `WaitFor` beat's answer for this tick. And the room
    // replaying alone, whose cutscene alone plays.
    (catalog, mut asked, wait, replay): (
        Option<Res<ambition_platformer2d_shared_tangle::authored_logic::CommandCatalog>>,
        Option<ResMut<Messages<ambition_platformer2d_shared_tangle::authored_logic::RunAuthoredCommand>>>,
        Res<CutsceneWaitAnswer>,
        Option<Res<ambition_platformer2d_shared_tangle::lifecycle::RoomReplay>>,
    ),
) {
    // Each room's cutscene in turn (SIMULATION-ISLANDS N4), each on its own
    // room's seat and pace.
    let mut going = Vec::new();
    for mut cutscene in active.take_all() {
        if replay.as_ref().is_some_and(|replay| !replay.allows(cutscene.room.map(LiveRoomInstance::from_ordinal))) {
            going.push(cutscene);
            continue;
        }
        let room = cutscene.room;
        let written = room.zip(tick.as_deref()).map(|(room, tick)| {
            ambition_platformer2d_shared_tangle::lifecycle::RoomWrite { room: LiveRoomInstance::from_ordinal(room), tick: tick.0 }
        });
        let slot = cutscene_seat(&cutscene, &drivers, &rooms);
        let seat = controls.get(slot);
        // A room's cutscene plays at its room's pace: a kill's hold or a slow-down
        // in another room does not stretch it.
        let dt = time.sim_dt() * freeze.scale_in(room.map(LiveRoomInstance::from_ordinal));
        // A `Fall` beat: the character's bodies in the cutscene's room (in every
        // room, for a cutscene of the whole world) fall, as if their health ran
        // out; the death beat and every consequence of a death follow from it.
        // The death is recorded as a hit's would be, under the body's own respawn
        // policy, so a person who falls here lies here when the room is built
        // again (an NPC placement's death stays).
        let mut fall = |character: &str, flags: &mut Vec<(String, bool)>| {
            for (body, worn, mut health, kept) in &mut cast {
                let here = room.is_none() || rooms.of(body).map(|at| at.ordinal()) == room;
                if here && worn.0.as_str() == character && health.health.current > 0 {
                    health.health.current = 0;
                    if let Some((identity, config)) = kept.filter(|(identity, _)| !identity.id.starts_with("encounter:")) {
                        flags.extend(
                            ambition_platformer2d_actor::body::enemy_death_flag(config.tuning.respawn, &identity.id)
                                .map(|id| (id, true)),
                        );
                    }
                }
            }
        };

        // A `Command` beat: its prepared line, asked in the cutscene's room. A
        // line the installed commands refuse was refused when the library
        // loaded (`prepare_cutscene_lines`); here it is logged and skipped.
        let script_id = cutscene.script_id().to_owned();
        let roles = role_bindings(&cutscene.roles, cutscene.room);
        let mut command = |line: &str| {
            use ambition_platformer2d_shared_tangle::authored_logic::{AuthoredAsk, RunAuthoredCommand};
            let Some(catalog) = catalog.as_deref() else {
                return warn!("cutscene `{script_id}` asks `{line}`, and this composition publishes no commands");
            };
            match catalog.prepare_line(line).map_err(|e| e.to_string()).and_then(|call| call.bound(&roles)) {
                Ok(call) => {
                    if let Some(asked) = asked.as_deref_mut() {
                        asked.write(
                            RunAuthoredCommand::prepared(&call, AuthoredAsk::new("cutscene", script_id.clone()))
                                .in_room(room.map(ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::from_ordinal)),
                        );
                    }
                }
                Err(e) => warn!("cutscene `{script_id}`: {e}"),
            }
        };

        let runtime = &mut cutscene.runtime;
        let skip = if runtime.skipping {
            false
        } else if seat.cancel_held {
            cutscene.skip_hold += dt;
            cutscene.skip_hold >= runtime.settings.skip_hold_seconds
        } else {
            cutscene.skip_hold = 0.0;
            false
        };
        // An EDGE: a held confirm does not burn through several beats.
        #[cfg(feature = "dialogue")]
        let mut dismiss = seat.confirm_pressed;
        #[cfg(not(feature = "dialogue"))]
        let dismiss = seat.confirm_pressed;
        #[cfg(feature = "dialogue")]
        let mut beat_dt = dt;
        #[cfg(feature = "dialogue")]
        let mut reveal_to = None;
        #[cfg(not(feature = "dialogue"))]
        let beat_dt = dt;
        #[cfg(feature = "dialogue")]
        if let Some(ambition_cutscene::CutsceneBeat::Dialogue {
            speaker,
            text,
            speaker_character_id,
        }) = runtime.current_beat().filter(|_| !runtime.skipping)
        {
            let policy = dialogue_settings.in_live_room(room.map(
                ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::from_ordinal,
            )).copied().unwrap_or_default();
            let duration = policy.text_reveal.duration(text);
            let clock_end = match policy.line_advance {
                ambition_dialog::LineAdvance::Manual => duration,
                ambition_dialog::LineAdvance::Automatic { delay_seconds } => duration + delay_seconds,
            }.max(f32::EPSILON);
            beat_dt = dt.min((clock_end - runtime.elapsed).max(0.0));
            if dismiss {
                match policy.reveal_confirm_action(runtime.elapsed >= duration) {
                    Some(ambition_dialog::DialogConfirmAction::Reveal) => {
                        reveal_to = Some(duration.max(f32::EPSILON));
                        dismiss = false;
                    },
                    Some(ambition_dialog::DialogConfirmAction::Wait) => dismiss = false,
                    _ => {},
                }
            } else if let ambition_dialog::LineAdvance::Automatic { delay_seconds } = policy.line_advance {
                dismiss = runtime.elapsed >= duration + delay_seconds;
            }
            if let Some(sfx) = dialogue_sfx.as_mut().filter(|_| {
                policy.speech_blips && !skip && !dismiss && reveal_to.is_none() && beat_dt > 0.0
            }) {
                let count = text.chars().count();
                let identity = if speaker_character_id.is_empty() {
                    speaker
                } else {
                    speaker_character_id
                };
                let previous = policy.text_reveal.visible_chars(count, runtime.elapsed);
                let visible = policy.text_reveal.visible_chars(count, runtime.elapsed + beat_dt);
                if let Some(id) = ambition_dialog::typewriter_blip(
                    voice_catalog.as_deref(),
                    identity,
                    &script_id,
                    ambition_dialog::DialogSpeechStyle::Normal,
                    text,
                    previous,
                    visible,
                ) {
                    sfx.in_space(room.map(ambition_sfx::SfxSpace))
                        .write(ambition_sfx::SfxMessage::Play {
                            id,
                            emitter: ambition_sfx::SfxEmitter::Global,
                        });
                }
            }
        }

        // SS8: the show is skipped, not the story. The rest of the script
        // runs on from here with no presentation and no hold on the players:
        // its commands in order, each `WaitFor` still waiting on the arrival,
        // spawn or departure it names (or its longest hold), and the scene is
        // marked seen when its last beat passes, not when the skip is asked.
        if skip {
            runtime.begin_skip();
            cutscene.skip_hold = 0.0;
        }
        let runtime = &mut cutscene.runtime;

        let waiting = CutsceneWaitKey { room, script: script_id.clone(), beat: runtime.beat_index };
        let events = runtime.tick_asking(beat_dt, dismiss, |_| wait.holds(&waiting));
        #[cfg(feature = "dialogue")]
        if let Some(elapsed) = reveal_to { runtime.elapsed = elapsed; }
        let mut completed = false;
        let mut flags = Vec::new();
        for event in events {
            match event {
                // It fires only on the tick a beat begins (`elapsed == 0.0`), so a rollback landing
                // MID-BEAT never saw it again and the banner, camera target and fade were gone for
                // the rest of that beat — while the snapshot's doc claimed the next tick would
                // republish them. It also mutated one field at a time, so a camera beat following a
                // dialogue left the dialogue on screen.
                CutsceneEvent::BeatEntered { .. } => {}
                CutsceneEvent::FlagWritten { id, on } => flags.push((id, on)),
                CutsceneEvent::CharacterFell { character } => fall(&character, &mut flags),
                CutsceneEvent::CommandAsked { line } => command(&line),
                CutsceneEvent::Completed => {
                    completed = true;
                }
            }
        }

        if completed {
            flags.extend(end_cutscene(cutscene).map(|seen| (seen, true)));
            write_cutscene_flags(flags, written, requests.as_deref_mut(), &mut save);
            continue;
        }
        write_cutscene_flags(flags, written, requests.as_deref_mut(), &mut save);

        // THE WHOLE PICTURE, REPLACED, from the state the snapshot carries.
        // A pure function of `(script, beat_index, elapsed)` — so restoring mid-beat
        // restores the picture with it, and no beat can leave another's fields
        // standing because there are no individual fields to leave.
        let runtime = &cutscene.runtime;
        cutscene.presentation = runtime.presentation();
        if !runtime.skipping {
            cutscene.presentation.camera = runtime.camera_with(|roles| {
                let mut bounds = None;
                for role in roles {
                    let Some((_, identity)) = cutscene.roles.iter().find(|(name, _)| name == role) else { continue };
                    let Some((_, _, body)) = positions.iter().find(|(entity, id, _)| {
                        id.as_str() == identity && (room.is_none() || rooms.of(*entity).map(|room| room.ordinal()) == room)
                    }) else { continue };
                    use ambition_platformer2d_core::Vec2;
                    let at = body.pos;
                    let (min, max): (Vec2, Vec2) = bounds.unwrap_or((at, at));
                    bounds = Some((min.min(at), max.max(at)));
                }
                bounds.map(|(min, max)| {
                    let at = (min + max) * 0.5;
                    [at.x, at.y - 35.0]
                })
            });
        }
        going.push(cutscene);
    }
    active.put(going);
}

/// One playing cutscene's `WaitFor` beat: the room it plays for, its script
/// and the beat. Two rooms playing one script, or one line waited on at two
/// beats, are answered apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CutsceneWaitKey {
    pub room: Option<u32>,
    pub script: String,
    pub beat: usize,
}

/// This tick's answers to the playing cutscenes' `WaitFor` beats, each asked
/// with its scene's bound roles. Derived each tick from the world, just
/// before the cutscenes tick, so it is not rollback state.
#[derive(bevy::prelude::Resource, Clone, Debug, Default, PartialEq)]
pub struct CutsceneWaitAnswer {
    pub waits: Vec<(CutsceneWaitKey, bool)>,
}

impl CutsceneWaitAnswer {
    /// Whether the beat `key` waits on holds this tick.
    pub fn holds(&self, key: &CutsceneWaitKey) -> bool {
        self.waits.iter().any(|(asked, holds)| *holds && asked == key)
    }
}

/// Queue each unseen script whose `starts_when` condition holds this tick,
/// for the whole world (SS4's first form: a scene that follows an event, not
/// a volume). A script with no seen flag never starts this way: it would
/// play again on every tick its condition held (`check_cutscene_lines`
/// reports it).
pub fn trigger_cutscenes_on_conditions(world: &mut World) {
    use ambition_platformer2d_shared_tangle::authored_logic::{AuthoredAsk, ConditionCatalog};
    let playing: Vec<String> = world
        .get_resource::<ActiveCutscene>()
        .map(|active| active.iter().map(|cutscene| cutscene.script_id().to_owned()).collect())
        .unwrap_or_default();
    let Some(library) = world.get_resource::<CutsceneLibrary>() else {
        return;
    };
    let Some(catalog) = world.get_resource::<ConditionCatalog>() else {
        return;
    };
    let save = world.get_resource::<ambition_persistence::save::AmbitionGameSave>();
    let mut start = Vec::new();
    for script in library.scripts.values() {
        let (Some(line), Some(seen)) = (script.starts_when.as_deref(), script.seen_flag.as_deref()) else {
            continue;
        };
        if save.is_some_and(|save| save.data().flag(seen)) || playing.contains(&script.id) {
            continue;
        }
        let holds = catalog
            .prepare_line(line)
            .map(|prepared| catalog.ask(world, &prepared, &AuthoredAsk::new("cutscene", script.id.clone())).is_satisfied())
            .unwrap_or(false);
        if holds {
            start.push(script.id.clone());
        }
    }
    if start.is_empty() {
        return;
    }
    let mut queue = world.get_resource_or_insert_with(ambition_cutscene::CutsceneTriggerQueue::default);
    for id in start {
        queue.request(id);
    }
}

/// Run condition of [`trigger_cutscenes_on_conditions`]: some script starts on
/// a condition.
pub fn a_cutscene_starts_on_a_condition(library: Res<CutsceneLibrary>) -> bool {
    library.scripts.values().any(|script| script.starts_when.is_some())
}

/// Answer each playing cutscene's `WaitFor` condition for this tick, through
/// the condition catalog (the vocabulary a lock wall and a Yarn branch use).
/// A line the catalog cannot prepare never holds: the beat waits out its
/// longest hold.
pub fn answer_cutscene_wait(world: &mut World) {
    use ambition_platformer2d_shared_tangle::authored_logic::{AuthoredAsk, ConditionCatalog};
    let waiting: Vec<(CutsceneWaitKey, String, Vec<(String, String)>)> = world
        .get_resource::<ActiveCutscene>()
        .map(|active| {
            active
                .iter()
                .filter_map(|cutscene| {
                    let (condition, _) = cutscene.runtime.current_beat()?.condition()?;
                    Some((
                        CutsceneWaitKey {
                            room: cutscene.room,
                            script: cutscene.script_id().to_owned(),
                            beat: cutscene.runtime.beat_index,
                        },
                        condition.to_owned(),
                        cutscene.roles.clone(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let waits = waiting
        .into_iter()
        .map(|(key, line, roles)| {
            let holds = world.get_resource::<ConditionCatalog>().is_some_and(|catalog| {
                let asked = catalog.prepare_line(&line).map_err(|e| e.to_string()).and_then(|call| call.bound(&role_bindings(&roles, key.room)));
                match asked {
                    Ok(prepared) => catalog.ask(world, &prepared, &AuthoredAsk::new("cutscene", key.script.clone())).is_satisfied(),
                    Err(why) => {
                        warn!("cutscene `{}` waits on `{line}`: {why}", key.script);
                        false
                    }
                }
            });
            (key, holds)
        })
        .collect();
    world.insert_resource(CutsceneWaitAnswer { waits });
}

/// The cutscene lines the installed vocabulary refuses: a `Command` beat
/// whose line names no published command (or the wrong arguments), a
/// `WaitFor` beat whose condition names no published condition. Each names
/// its cutscene and beat. Stated again whenever the library changes (at
/// start and at each reload), so a content check can read it.
#[derive(bevy::prelude::Resource, Clone, Debug, Default, PartialEq)]
pub struct CutsceneLineRefusals(pub Vec<String>);

/// Prepare every `Command` and `WaitFor` line of the library against the
/// installed command and condition catalogs, and report each refusal.
pub fn check_cutscene_lines(
    library: Res<CutsceneLibrary>,
    commands: Option<Res<ambition_platformer2d_shared_tangle::authored_logic::CommandCatalog>>,
    conditions: Option<Res<ambition_platformer2d_shared_tangle::authored_logic::ConditionCatalog>>,
    mut refusals: ResMut<CutsceneLineRefusals>,
) {
    let mut found = Vec::new();
    for script in library.scripts.values() {
        if let Some(line) = &script.starts_when {
            let problem = if script.seen_flag.is_none() {
                Some("it starts on a condition and has no seen flag, so it would play on every tick the condition holds".to_owned())
            } else {
                match conditions.as_deref() {
                    Some(catalog) => catalog.prepare_line(line).err().map(|e| e.to_string()),
                    None => Some(format!("`{line}`: this composition publishes no conditions")),
                }
            };
            if let Some(problem) = problem {
                let line = format!("cutscene `{}` starts_when: {problem}", script.id);
                error!("{line}");
                found.push(line);
            }
        }
        let undeclared = |line: &str| {
            line.split_whitespace()
                .filter_map(|word| word.strip_prefix('@'))
                .find(|role| !script.roles.iter().any(|(declared, _)| declared == role))
                .map(|role| format!("`{line}` names the role `@{role}`, which the script does not declare"))
        };
        for (index, beat) in ambition_cutscene::authored_beats(&script.beats).into_iter().enumerate() {
            let named = match beat {
                ambition_cutscene::CutsceneBeat::Command { line } => undeclared(line),
                ambition_cutscene::CutsceneBeat::WaitFor { condition, .. }
                | ambition_cutscene::CutsceneBeat::WaitUntil { condition } => undeclared(condition),
                _ => None,
            };
            if let Some(problem) = named {
                let line = format!("cutscene `{}` beat {index}: {problem}", script.id);
                error!("{line}");
                found.push(line);
                continue;
            }
            let problem = match beat {
                ambition_cutscene::CutsceneBeat::Command { line } => match commands.as_deref() {
                    Some(catalog) => catalog.prepare_line(line).err().map(|e| e.to_string()),
                    None => Some(format!("`{line}`: this composition publishes no commands")),
                },
                ambition_cutscene::CutsceneBeat::WaitFor { condition, .. }
                | ambition_cutscene::CutsceneBeat::WaitUntil { condition } => match conditions.as_deref() {
                    Some(catalog) => catalog.prepare_line(condition).err().map(|e| e.to_string()),
                    None => Some(format!("`{condition}`: this composition publishes no conditions")),
                },
                _ => None,
            };
            if let Some(problem) = problem {
                let line = format!("cutscene `{}` beat {index}: {problem}", script.id);
                error!("{line}");
                found.push(line);
            }
        }
    }
    refusals.0 = found;
}

/// Log this tick's change to each room's cutscene in its room.
pub fn log_cutscene_changes_in_rooms(
    active: Res<ActiveCutscene>,
    mut log: ResMut<ambition_cutscene::CutscenesInRooms>,
    tick: Option<Res<ambition_time::SimTick>>,
) {
    log.observe(
        &active,
        tick.map_or(0, |tick| tick.0),
        ambition_platformer2d_shared_tangle::lifecycle::ROOM_REWIND_HORIZON_TICKS,
    );
}

/// The room retractor: put back the cutscene `room` had at `saved_at`.
fn retract_room_cutscenes(
    world: &mut World,
    room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    saved_at: u64,
) {
    let Some(put_back) = world
        .get_resource_mut::<ambition_cutscene::CutscenesInRooms>()
        .and_then(|mut log| log.retract_room(room.ordinal(), saved_at))
    else {
        return;
    };
    if let Some(mut active) = world.get_resource_mut::<ActiveCutscene>() {
        active.restore_room(room.ordinal(), put_back);
    }
}

/// Module-local Bevy plugin: schedules the cutscene chain
/// (`auto_trigger_room_cutscenes` → `drain_cutscene_triggers` →
/// `tick_active_cutscene`) into [`ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::Cutscene`].
///
/// The presentation overlay (`ambition_render::cutscene::sync_cutscene_ui`) is
/// scheduled separately by the render/app side — this plugin owns only the
/// gameplay-side playback.
pub struct CutsceneSchedulePlugin;

impl Plugin for CutsceneSchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // The cutscene state channels + an empty library (anti-god rule 5:
        // the domain plugin owns its init). Content POPULATES the library; a
        // game that pre-inserts it wins (init never clobbers). Which room
        // starts which cutscene is the room's own metadata.
        app.init_resource::<CutsceneLibrary>();
        app.init_resource::<CutsceneWaitAnswer>();
        app.init_resource::<CutsceneLineRefusals>();
        // Outside the tick: the library changes at start and at a reload.
        app.add_systems(
            bevy::prelude::Update,
            check_cutscene_lines.run_if(bevy::prelude::resource_changed::<CutsceneLibrary>),
        );
        app.init_resource::<CutsceneTriggerQueue>();
        // The room this session last announced: a component of the session root.
        ambition_platformer2d_shared_tangle::lifecycle::require_on_session_root::<ambition_cutscene::LastCutsceneRoom>(app);
        app.init_resource::<ActiveCutscene>();
        // The cutscene's changes, dated in their room, so a room that rewinds
        // puts back the cutscene it had (ISL-1b). Logged at the tail of the
        // tick, after the playback and anything else that writes it.
        ambition_platformer2d_shared_tangle::lifecycle::register_room_write_log::<ambition_cutscene::CutscenesInRooms>(app);
        ambition_platformer2d_shared_tangle::lifecycle::register_room_write_retractor(
            app,
            "cutscene.changes_in_rooms",
            retract_room_cutscenes,
        );
        app.add_systems(
            sim,
            log_cutscene_changes_in_rooms
                .run_if(cutscene_room_is_running)
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::FeatureViewSync),
        );
        app.add_systems(
            sim,
            (
                auto_trigger_room_cutscenes.run_if(bevy::ecs::schedule::common_conditions::not(
                    bevy::ecs::schedule::common_conditions::resource_exists::<ambition_platformer2d_shared_tangle::lifecycle::RoomReplay>,
                )),
                // Off replays as its siblings are: it asks on every tick a body
                // stands inside, so the first live tick after a replay asks
                // again, and a replay does not queue a second request.
                trigger_cutscenes_in_volumes.run_if(bevy::ecs::schedule::common_conditions::not(
                    bevy::ecs::schedule::common_conditions::resource_exists::<ambition_platformer2d_shared_tangle::lifecycle::RoomReplay>,
                )),
                trigger_cutscenes_on_conditions.run_if(a_cutscene_starts_on_a_condition).run_if(
                    bevy::ecs::schedule::common_conditions::not(
                        bevy::ecs::schedule::common_conditions::resource_exists::<ambition_platformer2d_shared_tangle::lifecycle::RoomReplay>,
                    ),
                ),
                // Only with a trigger queued: it took the queue each tick.
                drain_cutscene_triggers.run_if(
                    |queue: bevy::prelude::Res<ambition_cutscene::CutsceneTriggerQueue>| !queue.0.is_empty(),
                ),
                answer_cutscene_wait.run_if(a_cutscene_plays),
                tick_active_cutscene.run_if(a_cutscene_plays),
                // After the tick, so the tick a cutscene ends releases its room.
                project_cutscene_hold.run_if(a_cutscene_hold_is_owed),
            )
                .chain()
                .run_if(cutscene_room_is_running)
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::Cutscene),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "dialogue")]
    #[test]
    fn a_cutscene_blips_during_reveal_and_keeps_confirm_quiet() {
        use ambition_cutscene::{CutsceneBeat, CutsceneScript};
        use ambition_sfx::{OwnedSfxMessage, SfxId, SfxMessage, voice::DialogueVoiceCatalog};

        let mut app = App::new();
        let room = ambition_platformer2d_world::governing_rules::test_fixtures::two_game_session(&mut app, true);
        let voice = SfxId::from_static("test.dialogue.foreman");
        let mut voices = DialogueVoiceCatalog::default();
        voices.register_voiceprint(voice, &["npc_test_foreman"], &[]).unwrap();
        app.insert_resource(voices);
        app.insert_resource(ambition_time::WorldTime::new(0.1, 0.1));
        app.init_resource::<ambition_characters::control::SlotControls>();
        app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
        app.init_resource::<CutsceneWaitAnswer>();
        app.init_resource::<ActiveCutscene>();
        app.add_message::<OwnedSfxMessage>();
        app.add_systems(Update, tick_active_cutscene);
        let script = CutsceneScript::new("freight", vec![CutsceneBeat::Dialogue {
            speaker: "Foreman".into(), speaker_character_id: "npc_test_foreman".into(),
            text: "The freight window opens tomorrow, at noon.".into(),
        }]);
        app.world_mut().resource_mut::<ActiveCutscene>()
            .start(PlayingCutscene::new(CutsceneRuntime::new(script), Some(room.ordinal())));

        app.update();
        let mut audio = app.world_mut().resource_mut::<Messages<OwnedSfxMessage>>();
        assert!(matches!(audio.drain().next().map(|message| message.request),
            Some(SfxMessage::Play { id, .. }) if id == voice));

        app.world_mut().resource_mut::<ambition_characters::control::SlotControls>().set(
            ambition_characters::control::PlayerSlot::PRIMARY,
            ambition_platformer2d_core::ControlFrame { confirm_pressed: true, ..Default::default() },
        );
        app.update();
        assert!(app.world().resource::<Messages<OwnedSfxMessage>>().is_empty());
    }

    #[test]
    fn each_games_cutscene_uses_its_own_skip_timing_and_meter() {
        use ambition_cutscene::{CutsceneSettings, CutsceneBeat, CutsceneScript};
        use ambition_platformer2d_world::scoped_rules::{DeclareRulesExt, DeclaredRules, RulesScope};
        let mut app = App::new();
        let hall = ambition_platformer2d_world::governing_rules::test_fixtures::two_game_session(&mut app, true);
        app.declare_rules(RulesScope::UntaggedRooms, CutsceneSettings { skip_hold_seconds: 0.2 });
        app.declare_rules(RulesScope::Mode("smash"), CutsceneSettings { skip_hold_seconds: 0.6 });
        app.insert_resource(ambition_time::WorldTime::new(0.1, 0.1));
        app.init_resource::<ambition_characters::control::SlotControls>();
        app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
        app.init_resource::<CutsceneWaitAnswer>();
        app.init_resource::<CutsceneTriggerQueue>();
        app.init_resource::<ActiveCutscene>();
        let mut library = CutsceneLibrary::default();
        library.insert(CutsceneScript::new("long", vec![CutsceneBeat::Wait { seconds: 100.0 }]));
        app.insert_resource(library);
        app.add_systems(Update, (drain_cutscene_triggers, tick_active_cutscene).chain());
        app.world_mut().resource_mut::<ambition_characters::control::SlotControls>().set(
            ambition_characters::control::PlayerSlot::PRIMARY,
            ambition_platformer2d_core::ControlFrame { cancel_held: true, ..Default::default() },
        );
        for (room, duration) in [(hall, 0.2), (hall.next(), 0.6)] {
            app.world_mut().resource_mut::<CutsceneTriggerQueue>().request_in("long", room.ordinal());
            app.update();
            let playing = app.world().resource::<ActiveCutscene>().iter().next().unwrap().clone();
            assert_eq!(playing.runtime.settings.skip_hold_seconds, duration);
            let progress = playing.skip_progress();
            assert!((progress - 0.1 / duration).abs() < 0.0001);
            app.world_mut().resource_mut::<DeclaredRules<CutsceneSettings>>().amend(
                RulesScope::Mode("smash"), |settings| settings.skip_hold_seconds = 0.01,
            );
            if duration > 0.2 {
                app.update();
                assert!(app.world().resource::<ActiveCutscene>().is_playing());
            }
            for _ in 0..8 { app.update(); }
            assert!(!app.world().resource::<ActiveCutscene>().is_playing());
            app.world_mut().resource_mut::<DeclaredRules<CutsceneSettings>>().amend(
                RulesScope::Mode("smash"), |settings| settings.skip_hold_seconds = 0.6,
            );
        }
    }

    /// ⛔⛔ ENDING A CUTSCENE MUST RECORD THAT IT WAS SEEN, and until now nothing
    /// checked it: deleting the `set_flag` left every cutscene test green.
    ///
    /// The flag is what `should_play` reads to refuse a replay, so losing it does
    /// not break a frame — it makes the cutscene play AGAIN every time its
    /// trigger is met, which is a bug the player meets on the second visit and no
    /// test meets at all.
    ///
    /// ⭐ One test now covers BOTH endings because there is only one ending:
    /// skipping and completing were four identical steps written side by side and
    /// are now `end_cutscene`. That is the payoff of the merge — the guard did not
    /// have to be written twice either.
    #[test]
    fn ending_a_cutscene_records_that_it_was_seen() {
        let cutscene = PlayingCutscene::new(
            ambition_cutscene::CutsceneRuntime::new(ambition_cutscene::CutsceneScript {
                id: "probe".to_string(),
                beats: Vec::new(),
                seen_flag: Some("probe_seen".to_string()),
                starts_when: None,
                roles: Vec::new(),
            }),
            None,
        );
        let seen = end_cutscene(cutscene);

        assert_eq!(
            seen.as_deref(),
            Some("probe_seen"),
            "the cutscene ended without recording its seen flag, so its trigger \
             will play it again on the next visit"
        );
    }

    /// The flags a cutscene sets, and its seen flag, under the room it plays
    /// for and the tick (ISL-2d), so a rewind of that room takes them back.
    fn flags_written_by_a_cutscene_in(room: Option<u32>) -> Vec<ambition_combat::events::SetFlagsRequested> {
        let mut app = bevy::prelude::App::new();
        app.insert_resource(ambition_time::WorldTime::new(1.0, 1.0));
        app.insert_resource(ambition_time::SimTick(5));
        app.init_resource::<ambition_characters::control::SlotControls>();
        app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
        app.add_message::<ambition_combat::events::SetFlagsRequested>();
        app.insert_resource(ActiveCutscene::playing(
            ambition_cutscene::CutsceneRuntime::new(
                ambition_cutscene::CutsceneScript::new(
                    "probe",
                    vec![ambition_cutscene::CutsceneBeat::SetFlag { id: "probe_flag".into(), on: true }],
                )
                .with_seen_flag("probe_seen"),
            ),
            room,
        ));
        app.init_resource::<CutsceneWaitAnswer>();
        app.add_systems(bevy::prelude::Update, tick_active_cutscene);
        app.update();
        let messages = app.world().resource::<Messages<ambition_combat::events::SetFlagsRequested>>();
        messages.get_cursor().read(messages).cloned().collect()
    }

    #[test]
    fn a_room_cutscene_dates_its_flags_in_its_room() {
        let room = ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::from_ordinal(3);
        let dated = Some(ambition_platformer2d_shared_tangle::lifecycle::RoomWrite { room, tick: 5 });
        let written: Vec<_> = flags_written_by_a_cutscene_in(Some(3)).into_iter().flat_map(|flag| flag.changes.into_iter().map(move |(id, on)| (id, on, flag.written))).collect();
        assert_eq!(
            written,
            [("probe_flag".to_string(), true, dated), ("probe_seen".to_string(), true, dated)],
            "a room's cutscene wrote its flags undated, or not at all"
        );
        assert!(
            flags_written_by_a_cutscene_in(None).iter().all(|flag| flag.written.is_none()),
            "a cutscene of the whole world dated its flags in a room"
        );
    }

    /// ⭐ SS7: A SCENE BINDS ITS ROLES ONCE, IN ITS OWN ROOM. One script
    /// plays in two rooms at once, each with a body of the same character:
    /// each run binds the body in its own room. A scene of the whole world sees both, and
    /// leaves the role unbound rather than choosing one. The wait answers of
    /// the two rooms' scenes are told apart by room.
    #[test]
    fn each_scene_binds_its_roles_in_its_own_room() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
        use ambition_platformer2d_shared_tangle::sim_id::SimId;
        let mut app = bevy::prelude::App::new();
        app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
        let mut library = CutsceneLibrary::default();
        for id in ["brute_scene", "world_scene"] {
            let mut script = ambition_cutscene::CutsceneScript::new(id, Vec::new());
            script.roles = vec![("brute".into(), ambition_cutscene::RoleCast::Character("npc_brute".into()))];
            library.insert(script);
        }
        app.insert_resource(library);
        app.init_resource::<ActiveCutscene>();
        for (room, name) in [(1u32, "brute_one"), (2, "brute_two")] {
            let at = LiveRoomInstance::from_ordinal(room);
            app.world_mut().spawn((RoomInstanceRoot, at));
            app.world_mut().spawn((
                ambition_characters::actor::CharacterForm("npc_brute".into()),
                SimId::placement(name),
                InRoomInstance(at),
            ));
        }
        let mut queue = CutsceneTriggerQueue::default();
        // The same script in both rooms: each room's run starts, binds its own
        // body and waits on its own answer (SS7).
        queue.request_in("brute_scene", 1);
        queue.request_in("brute_scene", 2);
        app.insert_resource(queue);
        app.add_systems(bevy::prelude::Update, drain_cutscene_triggers);
        app.update();
        let bound: Vec<(Option<u32>, Vec<(String, String)>)> =
            app.world().resource::<ActiveCutscene>().iter().map(|cutscene| (cutscene.room, cutscene.roles.clone())).collect();
        let role = |id: &str| vec![("brute".to_string(), SimId::placement(id).as_str().to_owned())];
        assert_eq!(bound, [(Some(1), role("brute_one")), (Some(2), role("brute_two"))], "each scene binds the body in its own room");

        app.world_mut().resource_mut::<ActiveCutscene>().retain(|_| false);
        app.world_mut().resource_mut::<CutsceneTriggerQueue>().request("world_scene");
        app.update();
        let world: Vec<Vec<(String, String)>> = app.world().resource::<ActiveCutscene>().iter().map(|cutscene| cutscene.roles.clone()).collect();
        assert_eq!(world, [Vec::<(String, String)>::new()], "two bodies of the character: the world's scene binds neither");

        let key = |room| CutsceneWaitKey { room: Some(room), script: "brute_scene".into(), beat: 3 };
        let answer = CutsceneWaitAnswer { waits: vec![(key(1), true), (key(2), false)] };
        assert!(answer.holds(&key(1)) && !answer.holds(&key(2)), "one room's answer is not the other's");
    }

    /// Two rooms entered on one tick queue two entry cutscenes, and both play
    /// at once, one per room (SIMULATION-ISLANDS N4). A second for a room that
    /// has one waits its turn, and so does a cutscene of the whole world, with
    /// the triggers behind it (review 2026-11-11, P1: none is dropped).
    #[test]
    fn each_rooms_cutscene_plays_at_once_and_a_second_waits_its_turn() {
        let mut app = bevy::prelude::App::new();
        app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
        let mut library = CutsceneLibrary::default();
        for id in ["alices_room", "bobs_room", "alices_second", "the_world", "bobs_second"] {
            library.insert(ambition_cutscene::CutsceneScript { id: id.into(), beats: Vec::new(), seen_flag: None, starts_when: None, roles: Vec::new() });
        }
        app.insert_resource(library);
        app.init_resource::<ActiveCutscene>();
        let mut queue = CutsceneTriggerQueue::default();
        queue.request_in("alices_room", 1);
        queue.request_in("bobs_room", 2);
        queue.request_in("alices_second", 1);
        queue.request("the_world");
        queue.request_in("bobs_second", 2);
        app.insert_resource(queue);
        app.add_systems(bevy::prelude::Update, drain_cutscene_triggers);
        let playing = |app: &bevy::prelude::App| -> Vec<(String, Option<u32>)> {
            app.world().resource::<ActiveCutscene>().iter().map(|cutscene| (cutscene.script_id().to_owned(), cutscene.room)).collect()
        };
        let end = |app: &mut bevy::prelude::App, id: &str| app.world_mut().resource_mut::<ActiveCutscene>().retain(|cutscene| cutscene.script_id() != id);
        app.update();
        assert_eq!(playing(&app), [("alices_room".into(), Some(1)), ("bobs_room".into(), Some(2))], "the rooms' cutscenes did not play at once");
        end(&mut app, "alices_room");
        app.update();
        assert_eq!(playing(&app), [("alices_second".into(), Some(1)), ("bobs_room".into(), Some(2))], "Alice's second scene was dropped");
        end(&mut app, "alices_second");
        app.update();
        assert_eq!(playing(&app), [("bobs_room".into(), Some(2))], "the world's scene started over a room's, or the one behind it jumped the line");
        end(&mut app, "bobs_room");
        app.update();
        assert_eq!(playing(&app), [("the_world".into(), None)]);
        end(&mut app, "the_world");
        app.update();
        assert_eq!(playing(&app), [("bobs_second".into(), Some(2))], "the trigger behind the world's was dropped");
    }

    /// N4: two rooms' cutscenes tick at once, each skipped only by the seat in
    /// its own room: Bob holds back over his room's scene and Alice's plays on.
    #[test]
    fn each_rooms_cutscene_answers_only_its_own_rooms_seat() {
        use ambition_characters::control::{DrivingParticipant, PlayerSlot, SlotControls};
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, RoomInstanceRoot};
        let mut app = bevy::prelude::App::new();
        app.insert_resource(ambition_time::WorldTime::new(0.1, 0.1));
        app.init_resource::<SlotControls>();
        app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
        app.init_resource::<CutsceneWaitAnswer>();
        let scene = |id: &str| ambition_cutscene::CutsceneRuntime::new(ambition_cutscene::CutsceneScript::new(
            id, vec![ambition_cutscene::CutsceneBeat::Wait { seconds: 30.0 }],
        ));
        let mut active = ActiveCutscene::playing(scene("alices"), Some(1));
        active.start(PlayingCutscene::new(scene("bobs"), Some(2)));
        app.insert_resource(active);
        for (slot, room) in [(0u8, 1u32), (1, 2)] {
            let at = LiveRoomInstance::from_ordinal(room);
            app.world_mut().spawn((RoomInstanceRoot, at));
            app.world_mut().spawn((DrivingParticipant(PlayerSlot(slot)), InRoomInstance(at)));
        }
        app.world_mut().resource_mut::<SlotControls>().set(
            PlayerSlot(1),
            ambition_platformer2d_core::ControlFrame { cancel_held: true, ..Default::default() },
        );
        app.add_systems(bevy::prelude::Update, tick_active_cutscene.run_if(a_cutscene_plays));
        app.update();
        let holds: Vec<(String, f32)> = app.world().resource::<ActiveCutscene>().iter().map(|cutscene| (cutscene.script_id().to_owned(), cutscene.skip_hold)).collect();
        assert_eq!(holds, [("alices".into(), 0.0), ("bobs".into(), 0.1)], "a seat's hold reached the other room's scene");
        for _ in 0..12 { app.update(); }
        let left: Vec<String> = app.world().resource::<ActiveCutscene>().iter().map(|cutscene| cutscene.script_id().to_owned()).collect();
        assert_eq!(left, ["alices"], "Bob's hold did not skip his scene, or skipped Alice's");
    }

    /// ⚠ AND A SCRIPT THAT ASKS FOR NO FLAG ENDS WITHOUT WRITING ONE. Ending
    /// unrecorded is a different thing from ending unmarked, and a helper that
    /// invented an id would put a row in the save for every unnamed cutscene.
    #[test]
    fn a_cutscene_with_no_seen_flag_records_nothing() {
        let cutscene = PlayingCutscene::new(
            ambition_cutscene::CutsceneRuntime::new(ambition_cutscene::CutsceneScript {
                id: "unnamed".to_string(),
                beats: Vec::new(),
                seen_flag: None,
                starts_when: None,
                roles: Vec::new(),
            }),
            None,
        );
        let seen = end_cutscene(cutscene);

        assert_eq!(seen, None, "a script with no `seen_flag` wrote a durable row anyway");
    }

    /// The tick runs while a cutscene plays ([`a_cutscene_plays`]); once it
    /// ends, the cutscene is not written again.
    #[test]
    fn an_ended_cutscene_leaves_its_resources_unwritten() {
        use bevy::prelude::IntoScheduleConfigs as _;
        let mut app = bevy::prelude::App::new();
        app.insert_resource(ambition_time::WorldTime::new(1.0, 1.0));
        app.init_resource::<ambition_characters::control::SlotControls>();
        app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
        app.insert_resource(ActiveCutscene::playing(
            ambition_cutscene::CutsceneRuntime::new(ambition_cutscene::CutsceneScript {
                id: "probe".to_string(),
                beats: Vec::new(),
                seen_flag: None,
                starts_when: None,
                roles: Vec::new(),
            }),
            None,
        ));
        app.init_resource::<CutsceneWaitAnswer>();
        app.add_systems(bevy::prelude::Update, tick_active_cutscene.run_if(a_cutscene_plays));
        for _ in 0..8 {
            app.update();
            if !app.world().resource::<ActiveCutscene>().is_playing() {
                break;
            }
        }
        assert!(!app.world().resource::<ActiveCutscene>().is_playing(), "a script with no beats never ended");
        app.update();
        let ended = app.world().resource_ref::<ActiveCutscene>().last_changed();
        app.update();
        assert_eq!(app.world().resource_ref::<ActiveCutscene>().last_changed(), ended);
    }

    #[test]
    fn cutscene_trigger_queue_request_appends() {
        let mut queue = CutsceneTriggerQueue::default();
        queue.request("a");
        queue.request("b");
        queue.request_in("c", 7);
        let queued: Vec<_> = queue.0.iter().map(|trigger| (trigger.id.as_str(), trigger.room)).collect();
        assert_eq!(queued, [("a", None), ("b", None), ("c", Some(7))]);
    }

    /// ⭐ **THE CUTSCENE ROOM MEMORY IS THE SESSION'S, AND SO IS THE QUEST ONE
    /// (C03, 2026-10-07).** A new game that starts in the room the last session
    /// ended in must queue that room's entry cutscene again; the memory was a
    /// process-global resource that a reset cleared, and is a component of the
    /// session root now. A candidate beside a live session has none, and the live
    /// session's is not changed by it. Poisoned by giving the new root the old
    /// one's memory at the swap: the second request is skipped.
    #[test]
    fn a_new_session_in_the_same_room_queues_its_entry_cutscene_again() {
        use ambition_cutscene::{CutsceneTriggerQueue, LastCutsceneRoom};
        use ambition_platformer2d_shared_tangle::lifecycle::{
            require_on_session_root, session_world_entity, CandidateSessionRoot, SessionRoot,
            SessionScopeId,
        };
        use ambition_platformer2d_world::rooms::{RoomSet, RoomSpec};

        let rooms = || {
            let mut spec = RoomSpec::new(
                "hall",
                ambition_platformer2d_core::World::new(
                    "hall",
                    ambition_platformer2d_core::Vec2::new(640.0, 480.0),
                    ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                    Vec::new(),
                ),
            );
            spec.metadata.entry_cutscene = Some("hall_intro".to_owned());
            RoomSet::from_parts_or_panic("hall", vec![spec], Vec::new())
        };
        let mut app = App::new();
        app.init_resource::<CutsceneTriggerQueue>()
            .add_systems(Update, auto_trigger_room_cutscenes);
        require_on_session_root::<LastCutsceneRoom>(&mut app);
        app.world_mut().spawn((SessionRoot(SessionScopeId(1)), rooms()));
        ambition_platformer2d_world::rooms::seat_sole_live_room_by_id(app.world_mut(), "hall")
            .expect("the fixture set holds its room");
        let queued = |app: &App| app.world().resource::<CutsceneTriggerQueue>().0.len();
        app.update();
        assert_eq!(queued(&app), 1, "the first frame queues the entry cutscene");
        // Played: the drain takes it (a trigger still waiting is not queued twice).
        app.world_mut().resource_mut::<CutsceneTriggerQueue>().0.clear();
        let a = session_world_entity(app.world()).expect("session A is live");

        let b = app
            .world_mut()
            .spawn((CandidateSessionRoot(SessionScopeId(2)), rooms()))
            .id();
        assert!(app.world().get::<LastCutsceneRoom>(b).is_none());
        app.update();
        assert_eq!(queued(&app), 0, "a candidate queued a cutscene");
        assert_eq!(
            app.world().get::<LastCutsceneRoom>(a).unwrap().0,
            vec!["hall".to_owned()],
            "preparing a candidate changed the live session's memory"
        );

        app.world_mut().despawn(a);
        app.world_mut().entity_mut(b).remove::<CandidateSessionRoot>();
        app.world_mut().entity_mut(b).insert(SessionRoot(SessionScopeId(2)));
        ambition_platformer2d_world::rooms::seat_sole_live_room_by_id(app.world_mut(), "hall")
            .expect("B holds its room");
        assert_eq!(
            app.world().get::<LastCutsceneRoom>(b).unwrap().0,
            Vec::<String>::new(),
            "B was born with A's memory"
        );
        app.update();
        assert_eq!(
            queued(&app),
            1,
            "a new game that starts in the room the last session ended in skipped its \
             entry cutscene"
        );
    }
}

fn cutscene_room_is_running(
    active: Res<ActiveCutscene>,
    replay: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::RoomReplay>>,
) -> bool {
    replay.is_none_or(|replay| {
        !active.is_playing() || active.iter().any(|cutscene| replay.allows(cutscene.room.map(LiveRoomInstance::from_ordinal)))
    })
}
