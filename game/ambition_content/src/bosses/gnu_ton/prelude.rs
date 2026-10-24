//! The prelude to GNU-ton's fight: in the room before his hall he duels the
//! player on his own feet. He is the fighter `npc_gnu_ton` (named
//! [`NAME`]), placed as an `EnemySpawn` in a room of [`DOORS`], and he
//! fights with his own moves and his laws.
//!
//! Beaten down to [`FLEE_AT`] of his health, he stops fighting. Blows do
//! not hurt him any more; he says so, and he runs to the door of the room
//! that goes to his hall ([`DOORS`]). At the door he dives through it, and
//! he is gone from this room. The player chases him: in the hall he comes
//! in by its top door and leaps onto his giant's shoulders (the arrival
//! beat of his conductor, `gnu_ton::conductor` in `ambition_content_modules`).
//!
//! All of it is derived each tick from his health and his place, which are
//! rollback state: a rewind gives the same flight.

use bevy::prelude::*;

use ambition_characters::actor::{BodyHealth, Corpse, Invulnerability};
use ambition_characters::control::CommandedMove;
use ambition_combat::components::DeathBeat;
use ambition_combat::components::ActorIdentity;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_world::rooms::LiveRoomSpecs;
use ambition_vfx::vfx::{VfxMessage, VfxWriter};

/// The name of the prelude's GNU-ton (his character's `display_name`).
pub const NAME: &str = "GNU-ton";
/// The part of his health at which he runs.
pub const FLEE_AT: f32 = 0.3;
/// How fast he runs to the door (a fencer runs at 250).
pub const FLEE_SPEED: f32 = 330.0;
/// He is at the door when he is this near its middle.
pub const AT_THE_DOOR: f32 = 14.0;
/// The door of each prelude room that goes to his hall, by room id: the
/// sandbox's approach, and the Science dimension's.
pub const DOORS: [(&str, &str); 2] =
    [("gnu_ton_approach", "gnu_ton_approach_to_arena"), ("gnu_ton_works_approach", "approach_arena_door")];
/// What he says as he runs, and as he goes through the door.
pub const RUNS: &str = "Enough! A body in motion... stays in MOTION!";
pub const GONE: &str = "To the gnu!";

/// Has he turned to run? Health under [`FLEE_AT`] of its most, alive.
pub fn runs(health: &BodyHealth) -> bool {
    let health = health.health;
    health.current > 0 && (health.current as f32) <= (health.max as f32) * FLEE_AT
}

/// While he runs: he cannot be hurt, his steps go to the door, and at the
/// door he dives through it and is gone.
#[allow(clippy::type_complexity)]
pub fn flee_to_the_hall(
    mut commands: Commands,
    rooms: LiveRoomSpecs,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut vfx: VfxWriter,
    mut duelists: Query<(
        Entity,
        &ActorIdentity,
        &ae::BodyKinematics,
        &mut BodyHealth,
        Option<&CommandedMove>,
        Option<&mut DeathBeat>,
    )>,
) {
    for (entity, id, kin, mut health, walking, beat) in &mut duelists {
        if id.name != NAME {
            continue;
        }
        // Only the GNU-ton of a prelude room: the door of it that goes to
        // his hall.
        let Some(room) = live.of(entity) else {
            continue;
        };
        let Some(door) = rooms.live_rooms().find(|(live_room, _)| *live_room == room).and_then(|(_, definition)| {
            let spec = rooms.rooms().spec(definition);
            let zone = DOORS.iter().find(|(room_id, _)| *room_id == spec.id)?.1;
            spec.loading_zones.iter().find(|z| z.id == zone).map(|z| ae::AabbExt::center(z.aabb))
        }) else {
            continue;
        };
        // Gone through the door: no death row, he blinks out at once.
        if !health.alive() {
            if let Some(mut beat) = beat {
                if beat.since_s.is_some() && (beat.row_s != 0.0 || beat.corpse != Corpse::Vanish) {
                    beat.row_s = 0.0;
                    beat.corpse = Corpse::Vanish;
                }
            }
            continue;
        }
        if !runs(&health) {
            continue;
        }
        if !health.health.invulnerable.holds(Invulnerability::SCRIPTED) {
            health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
            vfx.for_room(Some(room)).write(VfxMessage::SpeechBubble {
                pos: kin.pos - ae::Vec2::new(0.0, kin.size.y * 0.5 + 20.0),
                text: RUNS.to_string(),
            });
        }
        let target = ae::Vec2::new(door.x, kin.pos.y);
        if (kin.pos.x - door.x).abs() <= AT_THE_DOOR {
            // Through the door.
            health.health.invulnerable.set(Invulnerability::SCRIPTED, false);
            let all = health.current();
            health.damage(all);
            commands.entity(entity).remove::<CommandedMove>();
            vfx.for_room(Some(room)).write(VfxMessage::SpeechBubble {
                pos: kin.pos - ae::Vec2::new(0.0, kin.size.y * 0.5 + 20.0),
                text: GONE.to_string(),
            });
            continue;
        }
        let wanted = CommandedMove { target, speed: FLEE_SPEED, arrive_tolerance: AT_THE_DOOR * 0.5 };
        if walking != Some(&wanted) {
            commands.entity(entity).insert(wanted);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::actor::Health;

    /// He runs under his part of his health, and not when he is beaten.
    #[test]
    fn he_runs_when_he_is_nearly_beaten() {
        let mut health = BodyHealth::new(Health::new(10));
        assert!(!runs(&health));
        health.damage(7);
        assert!(runs(&health));
        health.damage(3);
        assert!(!runs(&health), "a dead body does not run");
    }
}
