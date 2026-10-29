//! Build the bodies requested by encounter waves.

use ambition_platformer2d_actor_spawn::spawn_encounter_mob;

/// Serve the encounter domain's spawn REQUESTS.
///
/// ⭐ THE REQUEST ALREADY EXISTED. `ambition_encounter`'s wave director emits
/// [`ambition_encounter::EncounterEvent::SpawnCommand`] — id, character, kind,
/// pos, size, all primitives — onto the ordinary event bus. What the actor
/// kernel had was not a missing protocol but a SERVER living inside the
/// encounter adapter: the wave driver collected its own `SpawnCommand`s out of a
/// local vector and spawned them itself, so serving and driving were one system.
///
/// This is the server on its own, in the layer that owns body construction.
/// The domain says WHAT it wants spawned; this crate decides HOW a body is
/// assembled — the orchestration/construction line the decomposition doctrine
/// draws.
///
/// ⚠ Ordered `.after(WaveEncounterDriven)`: the request and its service are the
/// same tick, and a reader that ran first would serve last tick's requests.
pub fn serve_encounter_spawn_commands(
    mut commands: ambition_platformer2d_shared_tangle::lifecycle::SessionCommands<'_, '_>,
    mut events: bevy::prelude::MessageReader<ambition_encounter::EncounterEventMsg>,
    catalog: bevy::prelude::Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    // The session's cast, not the App's (`SessionCast`). A shell session that
    // lost its generation is answered with no spawn, as a live room rebuild is.
    cast: ambition_characters::prepared::SessionCast,
    // The sheets of the running session, as every construction road reads
    // them (`GenerationMechanics::sheets`). `None`: no session runs, see
    // `SessionMechanics`.
    generation: Option<bevy::prelude::Res<ambition_platformer2d_construction::mechanics::SessionMechanics>,
    >,
    // The room each mob is built in: it starts on the ground there, as a
    // placed enemy does. `None`: a composition with no rooms.
    rooms: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
) {
    let (Some(generation), Some(session_scope)) = (generation, commands.spawn_scope()) else {
        return;
    };
    let authored_sheets = &generation.sheets;
    let Some(prepared) = cast.get() else {
        if !events.is_empty() {
            bevy::log::warn_once!(
                "an encounter asked for a spawn in a session with no prepared cast; \
                 nothing was spawned (the session lost its generation)"
            );
        }
        return;
    };
    for msg in events.read() {
        let ambition_encounter::EncounterEvent::SpawnCommand {
            id,
            character,
            kind,
            pos,
            size,
        } = &msg.event
        else {
            continue;
        };
        let spec = rooms
            .as_ref()
            .and_then(|rooms| {
            msg.room.and_then(|room| rooms.definition_in(room)).map(|d| rooms.rooms().spec(d))
        });
        let pos = ambition_platformer2d_core::Vec2::new(pos[0], pos[1]);
        let size = ambition_platformer2d_core::Vec2::new(size[0], size[1]);
        let flies = character
            .as_deref()
            .and_then(|character| prepared.get(character))
            .and_then(|character| character.locomotion.as_ref())
            .and_then(|locomotion| locomotion.baseline_free_flight)
            .unwrap_or(false);
        let pos = spec.map_or(pos, |spec| {
            wave_mob_start(spec, &msg.encounter, id, pos, size, flies)
        });
        spawn_encounter_mob(&generation.sprite_content,
            &mut commands,
            &catalog,
            authored_sheets,
            prepared,
            // ⛔ INTO THE OCCURRENCE'S LIVE ROOM, which the request carries. An
            // unstamped body is in NO room while two are live: a wave there
            // spawned mobs nothing in either room could hit, see or count.
            session_scope.in_room(msg.room),
            msg.encounter.clone(),
            ambition_encounter::mob_seed::EncounterMobSeed {
                id: id.clone(),
                character: character.as_deref(),
                brain: ambition_entity_catalog::placements::CharacterBrain::Custom(kind.clone()),
                pos,
                size,
            },
        );
    }
}


/// Where a wave mob starts in `room`: on the ground under its authored box,
/// as a placed enemy does (`RoomSpec::settled_on_ground`), unless it flies.
///
/// ⛔ A box the room cannot settle and that is inside solid ground is an
/// authoring error: the body would start in the floor. It is built where it
/// was authored and reported loudly, so it is found and fixed (Jon,
/// 2026-10-18: "if an enemy spawns in the floor we should flag that"). The
/// content check `wave_spawns_start_clear_of_solid_ground` catches it before
/// it ships.
fn wave_mob_start(
    room: &ambition_platformer2d_world::rooms::RoomSpec,
    encounter: &str,
    id: &str,
    pos: ambition_platformer2d_core::Vec2,
    size: ambition_platformer2d_core::Vec2,
    flies: bool,
) -> ambition_platformer2d_core::Vec2 {
    use ambition_platformer2d_core::AabbExt;
    let body = ambition_platformer2d_core::Aabb::new(pos, size * 0.5);
    if !flies {
        if let Some(settled) = room.settled_on_ground(body) {
            return settled.center();
        }
    }
    let inside = room.embeds(body);
    if !inside.is_empty() {
        bevy::log::error!(
            "SPAWNED IN THE FLOOR: wave mob {id} of encounter `{encounter}` in room `{}` starts at {pos:?} \
             (box {size:?}) inside solid ground {inside:?}; move its spawn in the encounter's waves",
            room.id
        );
    }
    pos
}

#[cfg(test)]
mod tests;

mod plugin;
pub use plugin::EncounterSpawnServicePlugin;
