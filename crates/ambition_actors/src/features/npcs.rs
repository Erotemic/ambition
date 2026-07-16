//! Peaceful-actor (NPC) glue for the unified actor simulation: the catalog
//! brain resolver ([`resolve_npc_brain`]) and the hit/hostile/dialogue/
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

/// Resolve the explicit initial brain (and its runtime [`BrainBinding`]) for a
/// placed NPC.
///
/// Precedence is entirely explicit: the placement's `brain_override` preset,
/// else the character's catalog `default_brain`. The placement's `patrol_radius`
/// / `patrol_path_id` are threaded only as PARAMETERS a *selected* patrol preset
/// consumes (its lane radius / path); they never SELECT the brain. This function
/// never inspects the resulting brain — there is no "basic brain" classification,
/// no `is_hostile` gate, and no `patrol_radius == 0` sentinel.
///
/// An NPC placed without a `character_id` (legacy / synthetic) has no catalog
/// identity to resolve a default from: it gets a plain stand-still brain and no
/// binding (nothing to switch or snapshot). A catalog-backed NPC returns its
/// binding so runtime gameplay can switch its brain and snapshot the selection.
///
/// Fails loud (panics) on unresolvable content — an unknown `character_id` or an
/// unknown preset name — matching the catalog's pre-release fail-loud stance.
/// Unknown preset names never fall back silently to the default or StandStill.
pub(crate) fn resolve_npc_brain(
    catalog: &CharacterCatalog,
    interactable: &Interactable,
    spawn_world_x: f32,
) -> (ambition_characters::brain::Brain, Option<BrainBinding>) {
    let InteractionKind::Npc {
        character_id,
        patrol_radius,
        patrol_path_id,
        brain_override,
        ..
    } = &interactable.kind
    else {
        return (ambition_characters::brain::Brain::stand_still(), None);
    };
    let Some(cid) = character_id.as_deref() else {
        // Anonymous NPC: no catalog row, so no default to resolve and nothing to
        // bind. A stand-still body is the honest inert default.
        return (ambition_characters::brain::Brain::stand_still(), None);
    };
    let selection = InitialBrainSelection::from_authored(brain_override.as_deref());
    let ctx =
        BrainBuildContext::from_placement(spawn_world_x, *patrol_radius, patrol_path_id.clone());
    match resolve_initial_brain(catalog, cid, &selection, &ctx) {
        Ok((binding, brain)) => (brain, Some(binding)),
        // No catalog row for this id in the current context — e.g. a provider
        // character exhibited (the Hall) where its provider fragment isn't
        // registered, or legacy content. The old heuristic fell back to
        // stand-still here; keep that tolerance so a partial catalog never
        // crashes a spawn. The body is an inert stand-still with no binding
        // (nothing to switch or snapshot). Unknown-character validation belongs
        // in a content test, not a spawn-time crash.
        Err(ambition_characters::actor::character_catalog::BrainBuildError::UnknownCharacter(
            _,
        )) => {
            bevy::log::warn!(
                target: "ambition_actors::npcs",
                "NPC `{cid}` has no character catalog row in this context; stand-still fallback",
            );
            (ambition_characters::brain::Brain::stand_still(), None)
        }
        // An authored `brain_override` naming a preset that does not exist (after
        // namespace qualification) is a genuine content error with no valid
        // interpretation — fail loud (pre-release stance), never silently fall back.
        Err(err) => panic!("NPC spawn `{cid}`: {err}"),
    }
}

// --- Interaction-based free helpers -----------------------------------
//
// These derive flags + bark/dialogue lines from the actor's *interaction*
// payload (`Interactable`) plus its identity (`name`/`id`) and a couple of
// status scalars (`strikes`/`hostile`), explicitly threaded — never from a
// per-family cluster. That keeps dialogue an actor capability (the
// `ActorInteraction` seam): any talkable actor can drive them.

use ambition_characters::actor::character_catalog::{
    resolve_initial_brain, BarkSituation, BrainBinding, BrainBuildContext, CharacterCatalog,
    InitialBrainSelection,
};
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
pub(crate) fn npc_hit_bark_line<'a>(
    catalog: &'a CharacterCatalog,
    interactable: &Interactable,
    strikes: i32,
) -> &'a str {
    let rotation = strikes.saturating_sub(1).max(0) as u32;
    if let Some(cid) = npc_character_id(interactable) {
        if let Some(line) = catalog.bark_line(cid, BarkSituation::OnHit, rotation) {
            return line;
        }
    }
    GENERIC_HIT_BARKS[(rotation as usize).min(GENERIC_HIT_BARKS.len().saturating_sub(1))]
}

/// The shout a peaceful actor makes at the moment it turns hostile: the catalog
/// `barks.provoked` pool (rotation 0), else the engine-generic default.
pub(crate) fn npc_hostile_bark_line<'a>(
    catalog: &'a CharacterCatalog,
    interactable: &Interactable,
) -> &'a str {
    if let Some(cid) = npc_character_id(interactable) {
        if let Some(line) = catalog.bark_line(cid, BarkSituation::Provoked, 0) {
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
pub(crate) fn npc_ambient_bark_line<'a>(
    catalog: &'a CharacterCatalog,
    interactable: &Interactable,
    situation: BarkSituation,
    rotation: u32,
) -> Option<&'a str> {
    let cid = npc_character_id(interactable)?;
    catalog.bark_line(cid, situation, rotation)
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

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::actor::character_catalog::{parse_catalog, CharacterCatalog};

    const FIRST: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "voice": (
                display_name: "Voice", spritesheet: "voice.png",
                manifest: "voice_spritesheet.ron", tier: MainHall,
                body_kind: Standard, composition: None,
                default_brain: "idle", default_action_set: "peaceful", tags: [],
                barks: (
                    on_hit: ["first hit"], provoked: ["first provoked"],
                    idle: ["first idle"],
                ),
            ),
        },
    )"#;

    const SECOND: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "voice": (
                display_name: "Voice", spritesheet: "voice.png",
                manifest: "voice_spritesheet.ron", tier: MainHall,
                body_kind: Standard, composition: None,
                default_brain: "idle", default_action_set: "peaceful", tags: [],
                barks: (
                    on_hit: ["second hit"], provoked: ["second provoked"],
                    idle: ["second idle"],
                ),
            ),
        },
    )"#;

    fn interactable() -> Interactable {
        Interactable::new(
            "voice",
            "Talk",
            ambition_engine_core::Aabb::new(
                ambition_engine_core::Vec2::ZERO,
                ambition_engine_core::Vec2::new(1.0, 1.0),
            ),
            InteractionKind::Npc {
                character_id: Some("voice".to_string()),
                dialogue_id: None,
                patrol_radius: 0.0,
                patrol_path_id: None,
                brain_override: None,
            },
        )
    }

    #[test]
    fn explicit_catalog_argument_is_the_bark_authority() {
        let first = CharacterCatalog::from_data(parse_catalog(FIRST));
        let second = CharacterCatalog::from_data(parse_catalog(SECOND));
        let npc = interactable();

        assert_eq!(npc_hit_bark_line(&first, &npc, 1), "first hit");
        assert_eq!(npc_hit_bark_line(&second, &npc, 1), "second hit");
        assert_eq!(npc_hostile_bark_line(&first, &npc), "first provoked");
        assert_eq!(npc_hostile_bark_line(&second, &npc), "second provoked");
        assert_eq!(
            npc_ambient_bark_line(&first, &npc, BarkSituation::Idle, 0),
            Some("first idle")
        );
        assert_eq!(
            npc_ambient_bark_line(&second, &npc, BarkSituation::Idle, 0),
            Some("second idle")
        );
    }
}
