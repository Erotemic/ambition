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
//! Two entry points, both producing plain [`SpawnActorRequest`]s the engine's
//! request applier resolves (grudges cross-wired once the batch's entities
//! exist):
//! - [`register_duel_content_staging`] — the duel-arena room stages the fight
//!   as part of room CONSTRUCTION, through the engine's
//!   [`RoomContentStagingRegistry`] seam. Registered staging (not a
//!   `RoomLoaded` consumer) is what makes the fighters part of the room's
//!   authoritative roster: activation, transition, reset, hot-reload, and a
//!   snapshot restore that stages this room all rebuild them identically
//!   (netcode.md N3.2b).
//! - [`install_duel_yarn_binding`] — the `<<duel>>` dialogue command stages
//!   the same fight beside the player, anywhere.

use ambition_engine_core as ae;
use bevy::prelude::*;

use ambition_actors::combat::components::ActorFaction;
use ambition_actors::features::{RoomContentStagingRegistry, SpawnActorKind, SpawnActorRequest};

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

/// Register the duel as the arena room's content staging: whenever the room's
/// contents are staged — activation, transition, reset, hot-reload, and a
/// snapshot restore staging the room — the exhibition fight stages with them,
/// so the pair is already battling the instant the player walks in. A pure
/// function of the `RoomSpec`, which is what lets a restore preflight predict
/// the fighters' identities without touching the world.
pub fn register_duel_content_staging(registry: &mut RoomContentStagingRegistry) {
    registry
        .register(
            DUEL_ARENA_ROOM_ID,
            "ambition_content",
            "duel_arena",
            "duel-staging.v1",
            |spec| {
                let center = spec.world.spawn + ae::Vec2::new(DUEL_ARENA_OFFSET_X, 0.0);
                duel_spawn_requests(center).to_vec()
            },
        )
        .expect("duel staging registration is unique");
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
