//! Peaceful-actor (NPC) glue for the unified actor simulation: the catalog
//! brain builder ([`npc_brain_from_catalog`]) and the hit/hostile/dialogue/
//! idle-bark line resolvers. Peaceful actors are the SAME ECS cluster as hostile
//! enemies now (see [`crate::features::ecs::actor_clusters`]); this module no
//! longer owns a separate NPC runtime view — only the dialogue/bark selection and
//! the peaceful brain selection. Talk/hostility tuning consts
//! ([`NPC_TALK_RADIUS`], [`NPC_HOSTILE_STRIKE_THRESHOLD`]) live here.
//!
//! A character's VOICE (its per-situation bark pools) is content: it lives in
//! the catalog `barks` field, keyed by the character id (the single source of
//! truth — see `bark_line_for_character_id`). This module keeps only the
//! engine-generic default an anonymous actor (no catalog id, or an empty pool)
//! falls back to; every NAMED voice was evicted to the catalog (R3.4).

use super::*;

/// Number of player attacks before a peaceful NPC turns hostile.
/// Three lets the player commit to the choice intentionally without
/// flipping by accident on a stray slash.
pub const NPC_HOSTILE_STRIKE_THRESHOLD: i32 = 3;

/// Fixed talk radius for patrolling NPCs. When the player gets
/// within this many world pixels, a patrolling NPC stops and faces
/// the player so the dialog interact is reachable. ~80 px ≈ 2.5
/// player widths — close enough to commit to dialog, far enough
/// that an NPC doesn't freeze the moment you walk past their
/// patrol range.
pub const NPC_TALK_RADIUS: f32 = 80.0;

/// Patrol speed for NPCs. Moved to the brain (its consumer,
/// `ambition_characters::brain::PatrolCfg::NPC_DEFAULT`); re-exported here for
/// authoring-side reference.
pub use ambition_characters::brain::NPC_PATROL_SPEED;

/// Engine-generic on-hit barks for an interactable actor whose catalog row
/// authors no `barks.on_hit` pool (an unnamed mob, or a placed NPC carrying no
/// `character_id`). Named per-character voices live in the catalog — this is
/// only the anonymous default. Rotation cycles the pool.
const GENERIC_HIT_BARKS: &[&str] = &["Hey.", "Cut it out.", "Okay, now I'm mad."];

/// Engine-generic shout an anonymous actor makes when it turns hostile (no
/// catalog `barks.provoked` pool). Named archetypes author their own.
const GENERIC_HOSTILE_BARK: &str = "That's it!";

/// Build the peaceful `Brain` component for a catalog/authored NPC.
///
/// Data-driven: if the NPC was authored from a catalog row that asks for a RICH,
/// PEACEFUL brain (past Patrol/StandStill but not hostile — e.g. the lively
/// Aerial flyer), honor the catalog `default_brain`. A placed `NpcSpawn` is
/// peaceful/talkable BY CONSTRUCTION, so a catalog row whose `default_brain` is
/// HOSTILE (some catalog rows carry a combat brain for when they spawn as
/// ENEMIES) must NOT turn the friendly NPC into a player-chaser — those fall
/// through to the peaceful patrol/standstill below. (An NPC only turns hostile
/// by being struck past its retaliation threshold.)
///
/// Drives the actor's movement at the unified tick; the cluster `config.brain`
/// (an `CharacterBrain`) only feeds the integrator's patrol-stall intent.
pub(crate) fn npc_brain_from_catalog(
    interactable: &ambition_interaction::Interactable,
    spawn_x: f32,
    patrol_radius: f32,
    talk_radius: f32,
    has_motion: bool,
) -> ambition_characters::brain::Brain {
    if let ambition_interaction::InteractionKind::Npc {
        character_id: Some(cid),
        ..
    } = &interactable.kind
    {
        if let Some(brain) = crate::character_roster::default_brain_for_character_id(cid, spawn_x) {
            let is_basic = matches!(
                brain,
                ambition_characters::brain::Brain::StateMachine(
                    ambition_characters::brain::StateMachineCfg::Patrol { .. }
                        | ambition_characters::brain::StateMachineCfg::StandStill
                )
            );
            if !is_basic && !brain.is_hostile() {
                return brain;
            }
        }
    }
    if patrol_radius > 0.0 || has_motion {
        let mut cfg = ambition_characters::brain::PatrolCfg::NPC_DEFAULT;
        cfg.lane = ambition_characters::brain::AuthoredWorldPatrolLane::new(spawn_x, patrol_radius);
        cfg.aggro_radius = talk_radius;
        ambition_characters::brain::Brain::StateMachine(
            ambition_characters::brain::StateMachineCfg::Patrol {
                cfg,
                state: ambition_characters::brain::PatrolState::default(),
            },
        )
    } else {
        ambition_characters::brain::Brain::stand_still()
    }
}

// --- Interaction-based free helpers -----------------------------------
//
// These derive flags + bark/dialogue lines from the actor's *interaction*
// payload (`Interactable`) plus its identity (`name`/`id`) and a couple of
// status scalars (`strikes`/`hostile`), explicitly threaded — never from a
// per-family cluster. That keeps dialogue an actor capability (the
// `ActorInteraction` seam): any talkable actor can drive them.

use ambition_characters::actor::character_catalog::BarkSituation;
use ambition_interaction::{Interactable, InteractionKind};

pub(crate) fn npc_flag_id(id: &str) -> String {
    format!("npc_{id}_hostile")
}

/// The catalog `character_id` carried by an NPC interaction payload, if any.
/// This is the identity key the catalog `barks` pools are authored against.
fn npc_character_id(interactable: &Interactable) -> Option<&str> {
    match &interactable.kind {
        InteractionKind::Npc {
            character_id: Some(cid),
            ..
        } => Some(cid.as_str()),
        _ => None,
    }
}

/// On-hit bark for a struck peaceful actor: the character's catalog `barks.on_hit`
/// pool (its authored voice), rotated by `strikes`; an actor with no catalog id
/// or no on-hit pool gets the engine-generic default.
pub(crate) fn npc_hit_bark_line(interactable: &Interactable, strikes: i32) -> &'static str {
    let rotation = strikes.saturating_sub(1).max(0) as u32;
    if let Some(cid) = npc_character_id(interactable) {
        if let Some(line) =
            crate::character_roster::bark_line_for_character_id(cid, BarkSituation::OnHit, rotation)
        {
            return line;
        }
    }
    GENERIC_HIT_BARKS[(rotation as usize).min(GENERIC_HIT_BARKS.len().saturating_sub(1))]
}

/// The shout a peaceful actor makes at the moment it turns hostile: the catalog
/// `barks.provoked` pool (rotation 0), else the engine-generic default.
pub(crate) fn npc_hostile_bark_line(interactable: &Interactable) -> &'static str {
    if let Some(cid) = npc_character_id(interactable) {
        if let Some(line) =
            crate::character_roster::bark_line_for_character_id(cid, BarkSituation::Provoked, 0)
        {
            return line;
        }
    }
    GENERIC_HOSTILE_BARK
}

/// Ambient one-liner for the idle-bark ticker: the catalog pool for
/// `situation` (`Idle` while roaming a normal room, `Hall` while on a Hall
/// pedestal), keyed by the actor's catalog id. `None` = nothing to say, so the
/// ticker skips this actor (an anonymous actor has no ambient voice). Rotation
/// cycles the pool.
pub(crate) fn npc_ambient_bark_line(
    interactable: &Interactable,
    situation: BarkSituation,
    rotation: u32,
) -> Option<&'static str> {
    let cid = npc_character_id(interactable)?;
    crate::character_roster::bark_line_for_character_id(cid, situation, rotation)
}

pub(crate) fn npc_message(interactable: &Interactable, name: &str, hostile: bool) -> String {
    if hostile {
        return format!("{name} attacks!");
    }
    match &interactable.kind {
        InteractionKind::Npc {
            dialogue_id: Some(dialogue_id),
            ..
        } => format!("{name} opens dialogue {dialogue_id}"),
        _ => format!("{name} opens fallback dialogue"),
    }
}

pub(crate) fn npc_dialogue_request(
    interactable: &Interactable,
    name: &str,
    id: &str,
) -> NpcDialogueRequest {
    let dialogue_id = match &interactable.kind {
        InteractionKind::Npc {
            dialogue_id: Some(dialogue_id),
            ..
        } => dialogue_id.clone(),
        _ => "generic_npc".to_string(),
    };
    NpcDialogueRequest {
        npc_id: id.to_string(),
        npc_name: name.to_string(),
        dialogue_id,
    }
}
