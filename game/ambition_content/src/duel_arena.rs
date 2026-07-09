//! Spectator-duel CONTENT — the PCA-vs-robot exhibition fight (R3.3: room
//! mechanics split by kind; this one is a `RoomLoaded` consumer).
//!
//! The two fighters are **normal `Npc`s** — not a hostile faction — holding
//! a mutual **grudge**: relational targeting hunts a grudge entity, and the
//! per-entity `damage_lands` override lets a same-faction hit land, so the
//! pair duels WITHOUT either being tagged Enemy/Boss. They never aim at the
//! observing player — yet damage stays physical, so a stray still catches a
//! player who wades into the crossfire. Once a fighter's grudge foe dies it
//! goes target-less and stands down like any other NPC.
//!
//! Two entry points, both writing plain [`SpawnActorRequest`] messages the
//! engine's request applier resolves (grudges cross-wired once the batch's
//! entities exist):
//! - [`stage_duel_on_room_loaded`] — walking into the authored duel-arena
//!   room auto-stages the fight (the engine emits `RoomLoaded` when a
//!   room's contents finish staging; no engine code names this room).
//! - [`install_duel_yarn_binding`] — the `<<duel>>` dialogue command stages
//!   the same fight beside the player, anywhere.

use ambition_engine_core as ae;
use bevy::prelude::*;

use ambition_actors::combat::components::ActorFaction;
use ambition_actors::features::{SpawnActorKind, SpawnActorRequest};
use ambition_actors::rooms::{RoomLoaded, RoomSet};

/// Feature id of the duel's PCA fighter.
pub const DUEL_PCA_ID: &str = "duel_pca";
/// Feature id of the duel's robot fighter.
pub const DUEL_ROBOT_ID: &str = "duel_robot";

/// Level id of the spectator duel arena room (authored in `sandbox.ldtk`).
pub const DUEL_ARENA_ROOM_ID: &str = "duel_arena";

/// How far ahead of the room's player-arrival point the duel is centered, so
/// the fight sits on-screen the moment the player enters.
const DUEL_ARENA_OFFSET_X: f32 = 360.0;

/// The two fighter spawn requests for a PCA-vs-robot duel centered at `center`.
/// Both spawn as plain `Npc`s holding a mutual grudge (each names the other's
/// id in `grudge_against`); the engine's request applier cross-wires the
/// grudges once both entities exist. The grudge — not a hostile faction — is
/// what makes them fight: it drives relational targeting AND authorizes
/// same-faction damage (`damage_lands`).
pub fn duel_spawn_requests(center: ae::Vec2) -> [SpawnActorRequest; 2] {
    [
        SpawnActorRequest {
            id: DUEL_PCA_ID.to_string(),
            // Display name MUST match the character-catalog `display_name` — the
            // sprite sheet AND the authored hitbox metadata both resolve from it
            // (`character_id_for_display_name`). "Perfect Cellular Automaton" →
            // the PCA sheet; a mismatch falls back to a generic placeholder.
            name: "Perfect Cellular Automaton".to_string(),
            pos: center + ae::Vec2::new(-75.0, 0.0),
            half_size: ae::Vec2::new(14.0, 23.0),
            faction: ActorFaction::Npc,
            grudge_against: Some(DUEL_ROBOT_ID.to_string()),
            kind: SpawnActorKind::Enemy {
                brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                    "cellular_automaton_fighter".to_string(),
                ),
            },
        },
        SpawnActorRequest {
            id: DUEL_ROBOT_ID.to_string(),
            // The robot copy of the player uses the player-robot body sheet, which
            // lives under the catalog's "Player" display_name (player_robot
            // spritesheet, proportionate scale). Actor display names aren't shown
            // in-game, so this drives only sprite + hitbox-metadata resolution.
            name: "Player".to_string(),
            pos: center + ae::Vec2::new(75.0, 0.0),
            half_size: ae::Vec2::new(14.0, 23.0),
            faction: ActorFaction::Npc,
            grudge_against: Some(DUEL_PCA_ID.to_string()),
            kind: SpawnActorKind::Enemy {
                brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                    "player_robot".to_string(),
                ),
            },
        },
    ]
}

/// Content system: when the duel-arena room's contents finish staging (the
/// engine's `RoomLoaded` fact — fired on initial load, transitions, resets,
/// and hot-reload restages alike), stage the exhibition fight so the pair is
/// already battling the instant the player walks in. The fighters are
/// runtime-staged actors, room-scoped like the rest of the room's spawns, so
/// re-staging after a reset re-emits them exactly as before.
pub fn stage_duel_on_room_loaded(
    mut rooms: MessageReader<RoomLoaded>,
    room_set: Res<RoomSet>,
    mut spawns: MessageWriter<SpawnActorRequest>,
) {
    for message in rooms.read() {
        if message.room_id != DUEL_ARENA_ROOM_ID {
            continue;
        }
        let Some(spec) = room_set
            .rooms
            .iter()
            .find(|room| room.id == message.room_id)
        else {
            continue;
        };
        let center = spec.world.spawn + ae::Vec2::new(DUEL_ARENA_OFFSET_X, 0.0);
        for request in duel_spawn_requests(center) {
            spawns.write(request);
        }
    }
}

/// `<<duel>>` — stage the spectator duel beside the player, anywhere. The
/// reusable way to show off / iterate the advanced fighter brain in-game; it
/// does NOT touch the dialog-challenged PCA (separate ids).
#[cfg(feature = "ui")]
fn cmd_duel(
    player: Query<
        &ambition_actors::actor::BodyKinematics,
        ambition_actors::actor::PrimaryPlayerOnly,
    >,
    mut spawns: MessageWriter<SpawnActorRequest>,
) {
    let Some(kin) = player.iter().next() else {
        warn!("<<duel>>: no player to center the duel on; ignoring");
        return;
    };
    // Stage the duel off to the side the player faces; the mutual grudge
    // (carried on the requests, cross-wired at spawn) makes the pair target
    // each other regardless of where the observer stands.
    let center = kin.pos + ae::Vec2::new(kin.facing.signum() * 220.0, 0.0);
    for request in duel_spawn_requests(center) {
        spawns.write(request);
    }
}

/// Install the `<<duel>>` command on the dialogue runner (the
/// `YarnContentBindings` seam).
#[cfg(feature = "ui")]
pub fn install_duel_yarn_binding(
    commands: &mut Commands,
    runner: &mut bevy_yarnspinner::prelude::DialogueRunner,
    _mirror: &ambition_dialog::YarnStateMirror,
) {
    let duel_id = commands.register_system(cmd_duel);
    runner.commands_mut().add_command("duel", duel_id);
}
