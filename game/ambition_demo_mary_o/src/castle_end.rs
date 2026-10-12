//! The end of a castle: a switch, the fall of the bridge, and who waits past it.
//!
//! A castle has no pole. Its goal is a switch (the block [`GOAL_SWITCH`]) past
//! the bridge of its boss, and the flag sequence takes a switch as it takes a
//! pole ([`crate::room_pole`]): the bridge falls when the goal is taken
//! (`bricks::drop_planks_when_the_goal_is_taken`), she walks off, and the level
//! goes where its room says. This module adds what only a castle has. She
//! waits at the switch while the bridge breaks and the lava takes the boss
//! (Jon, 2026-10-11: "you need to see the void boss swallowed by the lava,
//! and see the bridge break"), the switch shows that it is down, and the one
//! she saved thanks her before the level goes. The level gives the bridge one
//! view that does not move (`Level.arena`), so each of these is in it.

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
/// How fast the lava takes a boss down, in world px per second.
const SINK_SPEED: f32 = 46.0;
/// Ticks from one splash of the lava about a boss to the next.
const SPLASH_TICKS: u64 = 7;
/// A boss is dead when its top is this far under the top of the lava.
const SUNK_UNDER: f32 = 6.0;
/// With no lava under it, a boss is dead this far under the floor.
const FALLEN_UNDER: f32 = 96.0;

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

/// Put the cutscene of the rescue in the library of the app, one time, when
/// the app is built: a provider gives the library the rows of its own ids.
pub fn install(app: &mut App) {
    app.init_resource::<CutsceneLibrary>();
    app.world_mut().resource_mut::<CutsceneLibrary>().insert(rescue_script());
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

/// Whether a living Void Lizard of `room` is under the line of the floor
/// `floor`: it falls, or the lava has it.
fn a_boss_falls(
    bosses: impl Iterator<Item = (Entity, bool, ae::Vec2, ae::Vec2)>,
    live: &ambition_platformer2d::platformer::lifecycle::LiveRooms,
    room: Option<ambition_platformer2d::platformer::lifecycle::LiveRoomInstance>,
    floor: f32,
) -> bool {
    bosses
        .filter(|(entity, alive, _, _)| *alive && live.of(*entity) == room)
        .any(|(_, _, pos, size)| pos.y + size.y * 0.5 > floor + 1.0)
}

/// She waits at the switch while the bridge breaks and the boss falls.
///
/// The walk-off of the sequence waits (`FlagSequence::hold`) while a plank of
/// her room is whole, or a living Void Lizard is under the line of the floor.
/// Each has an end: a plank breaks each few ticks, and the lava takes a boss
/// down at a fixed speed. A boss that did not fall does not hold her: it dies
/// when she has walked off. Written each tick from state that rewinds.
#[allow(clippy::type_complexity)]
pub fn she_waits_while_the_bridge_falls(
    subject: Option<Res<ControlledSubject>>,
    rooms: Option<LiveRoomSpecs>,
    live: ambition_platformer2d::platformer::lifecycle::LiveRooms,
    broken: Option<Res<crate::bricks::BrokenBricks>>,
    geometry: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<ae::RoomGeometry>,
    mut sequences: Query<&mut FlagSequence>,
    bosses: Query<(
        Entity,
        &ambition_platformer2d::combat::actor_tuning::ActorConfig,
        &ambition_platformer2d::characters::actor::BodyHealth,
        &ae::BodyKinematics,
    )>,
) {
    let Ok(mut sequence) = sequences.single_mut() else {
        return;
    };
    let hold = sequence.active()
        && switch_of_her_room(rooms.as_ref(), subject.as_deref()).is_some_and(|(body, switch)| {
            let planks = match (broken.as_deref(), geometry.room_of(body), geometry.of(body)) {
                (Some(broken), Some(room), Some(room_geometry)) => {
                    crate::bricks::planks_left(broken, room, room_geometry).next().is_some()
                }
                _ => false,
            };
            let lizards = bosses
                .iter()
                .filter(|(_, config, _, _)| crate::void_lizard::is_void_lizard_brain(&config.brain))
                .map(|(entity, _, health, kin)| (entity, health.alive(), kin.pos, kin.size));
            planks || a_boss_falls(lizards, &live, live.of(body), switch.max.y)
        });
    if sequence.hold != hold {
        sequence.hold = hold;
    }
}

/// The lava takes the boss.
///
/// After the switch is taken, a Void Lizard of her room that is in the lava
/// goes down at a fixed speed, with a splash each few ticks, and dies when it
/// is under. The lava is drawn over a body, so it is seen to go in. With no
/// lava under it, it dies a few tiles under the floor. Each one that is left
/// dies when she has walked off. So the switch is a sure end of the boss, also
/// when a post held it up.
#[allow(clippy::type_complexity)]
pub fn the_lava_takes_the_boss(
    sequences: Query<&FlagSequence>,
    subject: Option<Res<ControlledSubject>>,
    rooms: Option<LiveRoomSpecs>,
    live: ambition_platformer2d::platformer::lifecycle::LiveRooms,
    tick: Option<Res<ambition_platformer2d::time::SimTick>>,
    mut vfx: ambition_platformer2d::vfx::VfxWriter,
    mut bosses: Query<(
        Entity,
        &ambition_platformer2d::combat::actor_tuning::ActorConfig,
        &mut ambition_platformer2d::characters::actor::BodyHealth,
        &mut ae::BodyKinematics,
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
    let pools = rooms
        .as_ref()
        .and_then(|rooms| rooms.spec_of(body))
        .map(crate::scenery::lava_pools_of)
        .unwrap_or_default();
    let walked_off = matches!(sequence.phase, FlagPhase::Tallied { .. });
    let room = live.of(body);
    // The base of the switch is the line of the floor.
    let floor = switch.max.y;
    for (entity, config, mut health, mut kin) in &mut bosses {
        if !crate::void_lizard::is_void_lizard_brain(&config.brain) || !health.alive() {
            continue;
        }
        if live.of(entity) != room {
            continue;
        }
        if walked_off {
            health.health.current = 0;
            continue;
        }
        let (top, feet) = (kin.pos.y - kin.size.y * 0.5, kin.pos.y + kin.size.y * 0.5);
        let lava = pools
            .iter()
            .map(|(_, pool)| *pool)
            .find(|pool| kin.pos.x >= pool.min.x && kin.pos.x <= pool.max.x);
        match lava {
            Some(pool) if feet >= pool.min.y => {
                // In the lava: down at a fixed speed, and no way to the side.
                kin.vel = ae::Vec2::new(0.0, SINK_SPEED);
                if tick.as_ref().is_some_and(|tick| tick.get() % SPLASH_TICKS == 0) {
                    vfx.for_room(room).write(ambition_platformer2d::vfx::VfxMessage::Burst {
                        pos: ae::Vec2::new(kin.pos.x, pool.min.y),
                        count: 10,
                        speed: 190.0,
                        color: [1.0, 0.52, 0.14, 1.0],
                        kind: ambition_platformer2d::vfx::ParticleKind::Shard,
                    });
                }
                if top > pool.min.y + SUNK_UNDER {
                    health.health.current = 0;
                }
            }
            None if top > floor + FALLEN_UNDER => health.health.current = 0,
            _ => {}
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
    queue: Option<ResMut<CutsceneTriggerQueue>>,
    active: Option<Res<ActiveCutscene>>,
) {
    // A world with no cutscenes (a test of one rule) ends its castle with no line.
    let (Some(mut queue), Some(active)) = (queue, active) else {
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
