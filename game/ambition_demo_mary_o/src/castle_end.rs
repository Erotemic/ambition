//! The end of a castle: a switch, the fall of the bridge, and who waits past it.
//!
//! A castle has no pole. Its goal is a switch (the block [`GOAL_SWITCH`]) past
//! the bridge of its boss, and the flag sequence takes a switch as it takes a
//! pole ([`crate::room_pole`]): the bridge falls when the goal is taken
//! (`bricks::drop_planks_when_the_goal_is_taken`), she walks off, and the level
//! goes where its room says. This module adds what only a castle has. The boss
//! dies with its bridge, the switch shows that it is down, and the one she
//! saved thanks her before the level goes.

use ambition_platformer2d::cutscene::{
    ActiveCutscene, CutsceneBeat, CutsceneLibrary, CutsceneScript, CutsceneTriggerQueue,
};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::ControlledSubject;
use ambition_platformer2d::world::rooms::{LiveRoomSpecs, RoomSpec};
use bevy::prelude::*;

use crate::flag::{FlagPhase, FlagSequence};

/// The name of the block that is the switch of a castle.
pub const GOAL_SWITCH: &str = "goal_switch";
/// The name of the prop that draws the switch.
pub const SWITCH_ART: &str = "goal_switch_art";
/// The row of the sheet of the switch when it is down.
const PRESSED_ROW: &str = "pressed";

/// The cutscene in which the one she saved thanks her.
pub const RESCUE_CUTSCENE: &str = "mary_o_castle_rescue";
/// The character of the catalog that waits past the bridge.
pub const RESCUED_CHARACTER_ID: &str = "mary_o_robot";
/// What he says.
pub const RESCUE_LINE: &str = "ROBOT:  \"Thank you, Mary-O. You saved me.\"";
/// How long his line stays, in seconds. It needs no button: a level that
/// waits for a button at its end can hold a player who does not know which.
const RESCUE_LINE_SECONDS: f32 = 4.0;
/// The tally does not go past this while his line is on the screen, so the
/// level waits for the line, and goes a short time after it.
const TALLY_HOLD: f32 = crate::LEVEL_CYCLE_DWELL - 0.75;

/// The box of the switch of a room, if the room ends on one.
pub fn room_switch(room: &RoomSpec) -> Option<ae::Aabb> {
    room.world
        .blocks
        .iter()
        .find(|block| block.name == GOAL_SWITCH)
        .map(|block| block.aabb)
}

fn rescue_script() -> CutsceneScript {
    CutsceneScript::new(
        RESCUE_CUTSCENE,
        vec![
            CutsceneBeat::Wait { seconds: 0.5 },
            CutsceneBeat::Banner {
                text: RESCUE_LINE.to_string(),
                seconds: RESCUE_LINE_SECONDS,
            },
        ],
    )
}

/// The switch of the room the controlled body is in, with that body.
fn switch_of_her_room(
    rooms: Option<&LiveRoomSpecs>,
    subject: Option<&ControlledSubject>,
) -> Option<(Entity, ae::Aabb)> {
    let body = subject?.0?;
    let switch = room_switch(rooms?.spec_of(body)?)?;
    Some((body, switch))
}

/// The boss dies with its bridge.
///
/// After the switch is taken, a Void Lizard of her room dies when it is below
/// the floor of the bridge, and each one that is left dies when she has walked
/// off. So the switch is a sure end of the boss, also when a post or a pit held
/// it up.
pub fn the_fall_kills_the_boss(
    sequences: Query<&FlagSequence>,
    subject: Option<Res<ControlledSubject>>,
    rooms: Option<LiveRoomSpecs>,
    live: ambition_platformer2d::platformer::lifecycle::LiveRooms,
    mut bosses: Query<(
        Entity,
        &ambition_platformer2d::combat::actor_tuning::ActorConfig,
        &mut ambition_platformer2d::characters::actor::BodyHealth,
        &ae::BodyKinematics,
    )>,
) {
    let Ok(sequence) = sequences.single() else {
        return;
    };
    if !sequence.active() {
        return;
    }
    let Some((body, switch)) = switch_of_her_room(rooms.as_ref(), subject.as_deref()) else {
        return;
    };
    let walked_off = matches!(sequence.phase, FlagPhase::Tallied { .. });
    let room = live.of(body);
    for (entity, config, mut health, kin) in &mut bosses {
        if !crate::void_lizard::is_void_lizard_brain(&config.brain) || !health.alive() {
            continue;
        }
        if live.of(entity) != room {
            continue;
        }
        // The base of the switch is the line of the floor.
        if walked_off || kin.pos.y - kin.size.y * 0.5 > switch.max.y {
            health.health.current = 0;
        }
    }
}

/// The one she saved thanks her, and the level waits for his line.
///
/// On the first tick of the tally of a room with a switch, his cutscene is
/// asked for. While it is asked for or on the screen, the time of the tally
/// stays under its end, so `cycle_level_on_flag_tally` (which runs after this)
/// leaves the level after the line and not before.
pub fn the_robot_thanks_her(
    subject: Option<Res<ControlledSubject>>,
    rooms: Option<LiveRoomSpecs>,
    mut owners: Query<(&FlagSequence, &mut crate::MaryOLevelState)>,
    library: Option<ResMut<CutsceneLibrary>>,
    queue: Option<ResMut<CutsceneTriggerQueue>>,
    active: Option<Res<ActiveCutscene>>,
) {
    // A world with no cutscenes (a test of one rule) ends its castle with no line.
    let (Some(mut library), Some(mut queue), Some(active)) = (library, queue, active) else {
        return;
    };
    let Ok((sequence, mut level)) = owners.single_mut() else {
        return;
    };
    if !matches!(sequence.phase, FlagPhase::Tallied { .. }) {
        return;
    }
    if switch_of_her_room(rooms.as_ref(), subject.as_deref()).is_none() {
        return;
    }
    if level.tally_dwell == 0.0 {
        // The library is the session's, and a load of content can put a new
        // one in its place. So the script is put in when it is asked for.
        if library.get(RESCUE_CUTSCENE).is_none() {
            library.insert(rescue_script());
        }
        queue.request(RESCUE_CUTSCENE);
        return;
    }
    let his_line_is_up = queue.0.iter().any(|id| id == RESCUE_CUTSCENE)
        || active
            .runtime
            .as_ref()
            .is_some_and(|runtime| runtime.script.id == RESCUE_CUTSCENE);
    if his_line_is_up && level.tally_dwell > TALLY_HOLD {
        level.tally_dwell = TALLY_HOLD;
    }
}

/// The switch shows that it is down from the tick the goal is taken.
///
/// Derived from the sequence each frame, so a rewind to before the switch puts
/// it up again.
pub fn dress_the_switch(
    mut commands: Commands,
    sequences: Query<&FlagSequence>,
    switches: Query<(
        Entity,
        &ambition_platformer2d::render::rendering::PropVisual,
        Option<&ambition_platformer2d::render::rendering::PropClip>,
    )>,
) {
    use ambition_platformer2d::render::rendering::PropClip;
    let down = sequences.single().is_ok_and(|sequence| sequence.active());
    for (entity, prop, clip) in &switches {
        if prop.name != SWITCH_ART {
            continue;
        }
        match (down, clip.is_some()) {
            (true, false) => {
                let request = ambition_platformer2d::sim_view::ClipRequest::from_chain(&[PRESSED_ROW])
                    .expect("one row");
                commands.entity(entity).insert(PropClip(request));
            }
            (false, true) => {
                commands.entity(entity).remove::<PropClip>();
            }
            _ => {}
        }
    }
}
