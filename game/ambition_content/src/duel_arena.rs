//! Spectator-duel CONTENT — the PCA-vs-robot exhibition fight (R3.3: room
//! mechanics split by kind; this one is a `RoomLoaded` consumer).
//!
//! The two fighters are normal `Npc`s — not a hostile faction — holding
//! a mutual grudge: relational targeting hunts a grudge entity, and the
//! per-entity `damage_lands` override lets a same-faction hit land, so the
//! pair duels WITHOUT either being tagged Enemy/Boss. They never aim at the
//! observing player — yet damage stays physical, so a stray still catches a
//! player who wades into the crossfire. Once a fighter's grudge foe dies it
//! goes target-less and stands down like any other NPC.
//!
//! Two entry points, both producing plain [`SpawnActorRequest`]s the engine's
//! request applier resolves (grudges cross-wired once the batch's entities
//! exist):
//! - [`register_duel_content_staging`] — the pack's arena room stages the fight
//!   as part of room CONSTRUCTION, through the engine's
//!   [`RoomContentStagingRegistry`] seam. Registered staging (not a
//!   `RoomLoaded` consumer) is what makes the fighters part of the room's
//!   authoritative roster: activation, transition, reset, hot-reload, and a
//!   snapshot restore that stages this room all rebuild them identically
//!   (netcode.md N3.2b).
//! - [`install_duel_yarn_binding`] — the `<<duel>>` dialogue command stages
//!   the same fight beside the player, anywhere.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_characters::actor::ActorFaction;
use ambition_content_schemas::rules::Duel;
use ambition_platformer2d::actor::{
    RoomContentStagingRegistry, SpawnActorKind, SpawnActorRequest,
};

/// The registration's owner and source in the room staging registry. A
/// content reload restates the stager under the same pair.
const STAGING_OWNER: &str = "ambition_content";
const STAGING_SOURCE: &str = "duel_arena";

/// The fighter spawn requests for the pack's duel centered at `center`. Each
/// fighter spawns as a plain `Npc` that names its foe in `grudge_against`;
/// the engine's request applier cross-wires the grudges once all entities
/// exist. The grudge, not a hostile faction, is what makes them fight: it
/// drives relational targeting and lets a same-faction hit land
/// (`damage_lands`).
pub fn duel_spawn_requests(duel: &Duel, center: ae::Vec2) -> Vec<SpawnActorRequest> {
    duel.fighters
        .iter()
        .map(|fighter| SpawnActorRequest {
            id: fighter.id.clone(),
            // A label that nothing downstream reads: a request that names a
            // character takes its shown name, its sheet and its hitbox from
            // that character (pinned by
            // `a_staged_actor_naming_a_character_takes_the_characters_label_not_its_requests`).
            name: fighter.id.clone(),
            pos: center + ae::Vec2::new(fighter.offset.0, fighter.offset.1),
            half_size: ae::Vec2::new(fighter.half_size.0, fighter.half_size.1),
            faction: ActorFaction::Npc,
            grudge_against: Some(fighter.grudge_against.clone()),
            kind: SpawnActorKind::Enemy {
                brain: ambition_entity_catalog::placements::CharacterBrain::Custom(fighter.brain.clone()),
                character: ambition_entity_catalog::CharacterId::from(fighter.character.as_str()),
            },
        })
        .collect()
}

/// Register the pack's duel as the arena room's content staging: whenever
/// the room's contents are staged (activation, transition, reset, hot
/// reload, and a snapshot restore that stages the room), the fight stages
/// with them, so the pair already fights when the player walks in. The
/// stager is a pure function of the `RoomSpec` and the duel it holds, so a
/// restore preflight can predict the fighters' identities without the world.
/// A content reload calls this again with the new duel.
pub fn register_duel_content_staging(registry: &mut RoomContentStagingRegistry, duel: &Duel) {
    let staged = duel.clone();
    registry
        .restate(duel.room.clone(), STAGING_OWNER, STAGING_SOURCE, "duel-staging.v1", move |spec| {
            let center = spec.world.spawn + ae::Vec2::new(staged.arena_offset_x, 0.0);
            duel_spawn_requests(&staged, center)
        })
        .expect("the duel's staging identity is not empty");
}

/// `<<duel>>` — stage the spectator duel beside the player, anywhere. The
/// reusable way to show off / iterate the advanced fighter brain in-game; it
/// does NOT touch the dialog-challenged PCA (separate ids).
#[cfg(feature = "ui")]
fn cmd_duel(
    player: Query<
        &ambition_platformer2d_core::BodyKinematics,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    // not `MessageWriter`. Spawning two fighters is a simulation act, and
    // `SpawnActorRequest` is cleared on rollback by a host that will not re-run
    // this presentation-side command — so a rewind past the duel deleted the
    // request and the duellists never appeared.
    mut spawns: ambition_conversation::NarrativeInputWriter<SpawnActorRequest>,
    tuning: Res<crate::tuning::AmbitionTuning>,
) {
    let Some(kin) = player.iter().next() else {
        warn!("<<duel>>: no player to center the duel on; ignoring");
        return;
    };
    // Stage the duel off to the side the player faces; the mutual grudge
    // (carried on the requests, cross-wired at spawn) makes the pair target
    // each other regardless of where the observer stands.
    let duel = &tuning.0.duel;
    let center = kin.pos + ae::Vec2::new(kin.facing.signum() * duel.beside_player_x, 0.0);
    for request in duel_spawn_requests(duel, center) {
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
