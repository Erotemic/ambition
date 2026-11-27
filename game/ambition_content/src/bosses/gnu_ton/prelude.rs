//! The prelude to GNU-ton's fight: in the room before his hall he duels the
//! player on his own feet. He is the fighter `npc_gnu_ton` (named
//! [`NAME`]), placed as an `EnemySpawn` in a room of [`EXITS`], and he
//! fights with his own moves and his laws.
//!
//! At [`FLEE_AT`] of his health, he stops fighting and runs to the exit
//! ([`EXITS`]). At a door, he plays the door passage. At a side passage,
//! he leaves when his body reaches the edge. In the arena, he enters from
//! above and jumps onto his giant's shoulders.
//!
//! The retreat uses the saved health, position, and control hold.

use bevy::prelude::*;

use ambition_characters::actor::{BodyHealth, Corpse, Invulnerability};
use ambition_characters::control::{CommandedMove, HoldClaims, claim_hold_as, release_hold_as};
use ambition_combat::components::DeathBeat;
use ambition_combat::components::ActorIdentity;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_session::rooms::passage::{PassageKind, PassageStage, ZonePassage};
use ambition_platformer2d_world::rooms::{LiveRoomSpecs, LoadingZoneActivation};
use ambition_vfx::vfx::{VfxMessage, VfxWriter};

/// The name of GNU-ton in the prelude (`display_name`).
pub const NAME: &str = "GNU-ton";
/// The part of his health at which he runs.
pub const FLEE_AT: f32 = 0.3;
/// How fast he runs to the door (a fencer runs at 250).
pub const FLEE_SPEED: f32 = 330.0;
/// He is at the door when he is this near its middle.
pub const AT_THE_DOOR: f32 = 14.0;
/// The exit ID for each prelude room.
const EXITS: [(&str, &str); 2] =
    [("gnu_ton_approach", "gnu_ton_approach_to_arena"), ("gnu_ton_works_approach", "approach_arena_door")];
const RETREAT: ae::applications::ApplicationKey = ae::applications::ApplicationKey::native("gnu_ton_prelude_retreat");
/// What he says as he runs, and as he goes through the door.
pub const RUNS: &str = "Enough! A body in motion... stays in MOTION!";
pub const GONE: &str = "To the gnu!";

/// Has he turned to run? Health under [`FLEE_AT`] of its most, alive.
pub fn runs(health: &BodyHealth) -> bool {
    let health = health.health;
    health.current > 0 && (health.current as f32) <= (health.max as f32) * FLEE_AT
}

/// Make GNU-ton invulnerable during his run. Remove him when he leaves the room.
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
        Option<&HoldClaims>,
        Option<&mut DeathBeat>,
        Option<&ZonePassage>,
    )>,
) {
    for (entity, id, kin, mut health, walking, holds, beat, passage) in &mut duelists {
        if id.name != NAME {
            continue;
        }
        let Some(room) = live.of(entity) else {
            continue;
        };
        let Some(exit) = rooms.live_rooms().find(|(live_room, _)| *live_room == room).and_then(|(_, definition)| {
            let spec = rooms.rooms().spec(definition);
            let zone = EXITS.iter().find(|(room_id, _)| *room_id == spec.id)?.1;
            spec.loading_zones.iter().find(|z| z.id == zone)
        }) else {
            continue;
        };
        use ae::AabbExt as _;
        let center = exit.aabb.center();
        let retreating = holds.is_some_and(|holds| holds.0.contains(&RETREAT));
        if !health.alive() {
            if let Some(mut beat) = beat {
                if beat.since_s.is_some() && (beat.row_s != 0.0 || beat.corpse != Corpse::Vanish) {
                    beat.row_s = 0.0;
                    beat.corpse = Corpse::Vanish;
                }
            }
            continue;
        }
        if let Some(passage) = passage {
            if passage.stage != PassageStage::Entering || passage.elapsed < ZonePassage::entering_s(passage.kind) {
                continue;
            }
        } else if !retreating && !runs(&health) {
            continue;
        }

        let edge_exit = exit.activation == LoadingZoneActivation::EdgeExit;
        let distance = (kin.pos.x - center.x).abs();
        let body = kin.aabb();
        // The side passage is a barrier for NPCs. Contact ends the run.
        let reached_edge = edge_exit && body.top() < exit.aabb.bottom() && body.bottom() > exit.aabb.top()
            && distance <= body.half_size().x + exit.aabb.half_size().x + 1.0;
        if passage.is_some() || reached_edge {
            if let Some(mut beat) = beat {
                beat.row_s = 0.0;
                beat.corpse = Corpse::Vanish;
            }
            health.health.invulnerable.set(Invulnerability::SCRIPTED, false);
            let all = health.current();
            if health.damage(all) {
                release_hold_as(&mut commands, entity, RETREAT);
                commands.entity(entity).remove::<CommandedMove>();
            } else {
                health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
            }
            continue;
        }
        if !retreating {
            claim_hold_as(&mut commands, entity, RETREAT);
            health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
            vfx.for_room(Some(room)).write(VfxMessage::SpeechBubble {
                pos: kin.pos - ae::Vec2::new(0.0, kin.size().y * 0.5 + 20.0),
                text: RUNS.to_string(),
            });
        }
        if !edge_exit && distance <= AT_THE_DOOR {
            commands.entity(entity).remove::<CommandedMove>().insert(ZonePassage {
                kind: PassageKind::Door,
                stage: PassageStage::Entering,
                elapsed: 0.0,
                from: kin.pos,
                zone: exit.id.clone(),
                foot: center + ae::Vec2::new(0.0, exit.aabb.half_size().y),
                heart: center,
                half_h: kin.size().y * 0.5,
                facing: if kin.facing < 0.0 { -1.0 } else { 1.0 },
                crossing: None,
                recorded: false,
                recorded_at: 0.0,
                arrival: None,
                room: None,
            });
            vfx.for_room(Some(room)).write(VfxMessage::SpeechBubble {
                pos: kin.pos - ae::Vec2::new(0.0, kin.size().y * 0.5 + 20.0),
                text: GONE.to_string(),
            });
            continue;
        }
        let target = ae::Vec2::new(center.x, kin.pos.y);
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
