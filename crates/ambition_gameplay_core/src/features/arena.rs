//! Spectator-duel staging — two brain fighters dueling while the player observes.
//!
//! The reusable mechanism behind the `<<duel>>` dialog command and the headless
//! duel test. The two fighters are **normal `Npc`s** — not a hostile faction —
//! that hold a mutual **grudge** against each other: relational targeting hunts a
//! grudge entity, and the per-entity `damage_lands` override lets a same-faction hit
//! land, so the two duel WITHOUT either being tagged Enemy/Boss. Because they aren't
//! hostile to the Player faction (and grudge only each other), they never aim at the
//! observing player — yet damage stays physical, so a stray still catches a player
//! who wades into the crossfire (Player is a different faction). Once a fighter's
//! grudge foe dies it goes target-less and stands down like any other NPC.
//!
//! This is the "second instance" duel: it spawns its own fighters with their own
//! ids and does not touch the dialog-challenged PCA. Both share the smash brain,
//! so tuning the brain improves both.

use ambition_engine_core as ae;

use crate::combat::components::ActorFaction;
use crate::features::{SpawnActorKind, SpawnActorRequest};

/// Feature id of the duel's PCA fighter.
pub const DUEL_PCA_ID: &str = "duel_pca";
/// Feature id of the duel's robot fighter.
pub const DUEL_ROBOT_ID: &str = "duel_robot";

/// The two fighter spawn requests for a PCA-vs-robot duel centered at `center`.
/// Both spawn as plain `Npc`s holding a mutual grudge (each names the other's id in
/// `grudge_against`); the spawn path cross-wires the grudges once both entities
/// exist. The grudge — not a hostile faction — is what makes them fight: it drives
/// relational targeting AND authorizes same-faction damage (`damage_lands`).
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
                brain: ambition_characters::actor::CharacterBrain::Custom(
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
                brain: ambition_characters::actor::CharacterBrain::Custom("player_robot".to_string()),
            },
        },
    ]
}

/// Level id of the spectator duel arena room (authored in `sandbox.ldtk`). When a
/// room with this id loads, [`stage_room_duel`] auto-stages the fight so the two
/// fighters are already battling the instant the player walks in — no trigger.
pub const DUEL_ARENA_ROOM_ID: &str = "duel_arena";

/// How far ahead of the room's player-arrival point the duel is centered, so the
/// fight sits on-screen the moment the player enters.
const DUEL_ARENA_OFFSET_X: f32 = 360.0;

/// If `room` is the spectator duel arena, return the pair of fighter spawn requests
/// (staged ahead of the room's arrival point, in view); `None` for any other room.
///
/// This stays a pure data helper — it does NOT touch the ECS or any global resource.
/// The duel needs no faction-relations mutation: the fighters are plain `Npc`s whose
/// mutual grudge (carried on each request, cross-wired at spawn) drives the fight.
/// The per-room-load spawn path ([`crate::features::spawn_room_feature_entities`])
/// calls this, applies the returned requests through the normal feature-spawn path,
/// and cross-wires their grudges once both entities exist.
pub fn stage_room_duel(room: &crate::rooms::RoomSpec) -> Option<[SpawnActorRequest; 2]> {
    if room.id != DUEL_ARENA_ROOM_ID {
        return None;
    }
    let center = room.world.spawn + ae::Vec2::new(DUEL_ARENA_OFFSET_X, 0.0);
    Some(duel_spawn_requests(center))
}
