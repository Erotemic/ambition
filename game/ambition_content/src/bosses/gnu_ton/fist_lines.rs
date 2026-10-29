//! The portal's line of GNU-ton the fighter's smash attacks.
//!
//! On a platform-fighter stage his smash attacks call the fist of his giant
//! out of a portal (the effects `gnu_fist_*` of the sheet `gnu_ton_vfx`).
//! The sheet draws the fore arm and the fist, cut where the line is. It does
//! NOT draw the line: the engine draws each portal
//! (`ambition_portal2d_presentation`), and no sprite has a portal in it.
//!
//! This puts a `PortalLightLine` where the arm is cut, for as long as the
//! effect's clip plays. The line is a look: nothing goes through it and the
//! simulation does not know it. This reads the move; it decides nothing.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_combat::moveset::MovePlayback;
use ambition_entity_catalog::MoveEventKind;
use ambition_platformer2d::platformer::lifecycle::{
    InRoomInstance, LiveRoomInstance, LiveRooms, SessionCommands, SpawnSessionScopedExt,
};
use ambition_portal2d::{PortalChannel, PortalChannelColor};
use ambition_portal2d_presentation::PortalLightLine;

use super::duel::CALLED_FIST;

/// Where the line is in a frame of `gnu_ton_vfx`, from the frame's middle
/// toward the side the fist comes from, as a part of the frame's size (the
/// renderer's `BEHIND` and `VIEW` in `targets/props/gnu_ton_vfx.py`).
const LINE_FROM_MIDDLE: f32 = 0.5 - 46.0 / 328.0;
/// The line's length, as a part of the frame's size (`LINE_HALF`).
const LINE_LENGTH: f32 = 256.0 / 328.0;

/// The line of one fighter's called fist.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FistLine {
    fighter: Entity,
}

/// The way the fist comes out of the line for the row `effect`, for a
/// fighter that looks to `facing`. The stage's down is the room's down.
fn punch_dir(effect: &str, facing: f32) -> ae::Vec2 {
    match &effect[CALLED_FIST.len()..] {
        "up" => ae::Vec2::new(0.0, -1.0),
        "down" => ae::Vec2::new(0.0, 1.0),
        _ => ae::Vec2::new(facing, 0.0),
    }
}

/// Keep a line of light where the arm of each called fist is cut.
pub fn light_fist_lines(sprite_content: bevy::prelude::Res<ambition_platformer2d::sprite_sheet::SpriteContent>,
    mut commands: SessionCommands,
    live: LiveRooms,
    fighters: Query<(Entity, &ae::BodyKinematics, &MovePlayback)>,
    mut lines: Query<(Entity, &FistLine, &mut PortalLightLine)>,
) {
    let Some(scope) = commands.spawn_scope() else {
        return;
    };
    // Each line: its fighter, its room, its middle, the room's side of it,
    // its length.
    let mut wanted: Vec<(Entity, LiveRoomInstance, ae::Vec2, ae::Vec2, f32)> = Vec::new();
    for (fighter, kin, playback) in &fighters {
        let Some(room) = live.of(fighter) else {
            continue;
        };
        for event in &playback.spec.events {
            let MoveEventKind::Vfx { effect, at, scale, .. } = &event.kind else {
                continue;
            };
            if !effect.starts_with(CALLED_FIST) {
                continue;
            }
            let Some(clip) = ambition_sprite_sheet::fx::authored_effects(&sprite_content).get(effect.as_str()) else {
                continue;
            };
            if playback.t < event.at_s || playback.t >= event.at_s + clip.clip_secs() {
                continue;
            }
            let size = ambition_render::fx::FX_DEFAULT_WORLD_SIZE * scale;
            let dir = punch_dir(effect, playback.facing);
            let middle = kin.pos + ae::Vec2::new(at.0 * playback.facing, at.1);
            wanted.push((fighter, room, middle - dir * (LINE_FROM_MIDDLE * size), dir, LINE_LENGTH * size));
        }
    }
    for (entity, tag, mut light) in &mut lines {
        match wanted.iter().position(|(fighter, ..)| *fighter == tag.fighter) {
            Some(index) => {
                let (_, _, at, normal, length) = wanted.swap_remove(index);
                if light.pos != at || light.normal != normal || light.length != length {
                    light.pos = at;
                    light.normal = normal;
                    light.length = length;
                }
            }
            None => commands.entity(entity).despawn(),
        }
    }
    for (fighter, room, at, normal, length) in wanted {
        commands
            .spawn_room_in_session(
                scope,
                (
                    PortalLightLine {
                        // The colour of the line by the giant's shoulder in his boss fight.
                        channel: PortalChannel::Authored(PortalChannelColor::Indexed(24)),
                        pos: at,
                        normal,
                        length,
                        depth: None,
                    },
                    FistLine { fighter },
                    Name::new("GNU-ton fist line"),
                ),
            )
            .insert(InRoomInstance(room));
    }
}
