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

/// Resolve the explicit initial brain (plus its runtime [`BrainBinding`] and the
/// [`AuthoredBrainContext`] it will rebuild from) for a placed NPC.
///
/// Precedence is entirely explicit: the placement's `brain_override` preset, else
/// the character's catalog `default_brain`. The placement's `patrol_radius` is
/// threaded only as a PARAMETER a *selected* patrol preset consumes (its lane
/// radius); it never SELECTS the brain. A `patrol_path_id` is a separate movement
/// attachment, not a brain-build parameter. This function never inspects the
/// resulting brain — no "basic brain" classification, no `is_hostile` gate, and no
/// `patrol_radius == 0` sentinel.
///
/// An NPC placed without a `character_id` (legacy / synthetic) has no catalog
/// identity to resolve a default from: it gets a plain stand-still brain and no
/// binding (nothing to switch or snapshot). A catalog-backed NPC returns its
/// binding + authored context so runtime gameplay can switch its brain, rebuild
/// its default around the authored home, and snapshot the selection.
///
/// Fails loud (panics) on an unknown preset name — matching the catalog's
/// pre-release fail-loud stance; unknown presets never fall back silently. An
/// unknown `character_id` is tolerated (stand-still, no binding): a partial-provider
/// composition (a Hall character whose provider fragment isn't registered in this
/// host) is an intentional content contract, not a spawn-time crash — the
/// prepared-content validation test is where unknown ids are caught for a host
/// that DOES claim to own them.
pub(crate) fn resolve_npc_brain(
    catalog: &CharacterCatalog,
    // **The prepared cast**, consulted only for the character's own default
    // autonomous profile (D73 phase 1). An EMPTY registry is a legal, meaningful
    // value: no character states a default, which is what this path assumed
    // before definitions could state one.
    prepared: &crate::character_runtime::PreparedCharacterRegistry,
    interactable: &Interactable,
    spawn_world_x: f32,
) -> (
    ambition_characters::brain::Brain,
    Option<(BrainBinding, AuthoredBrainContext)>,
) {
    let InteractionKind::Npc {
        character_id,
        patrol_radius,
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
    let authored = AuthoredBrainContext::from_placement(spawn_world_x, *patrol_radius);
    match resolve_initial_brain(
        catalog,
        cid,
        brain_override.as_deref(),
        // **The character's OWN default autonomous profile**, when its
        // definition names one (D73 phase 1). It outranks the catalog row's
        // `default_brain` and is outranked by the placement's `brain_override`
        // above — the whole precedence rule, resolved in one call.
        prepared
            .get(cid)
            .and_then(|prepared| prepared.default_brain_profile.as_ref()),
        &authored.build_context(),
    ) {
        Ok((binding, brain)) => (brain, Some((binding, authored))),
        // No catalog row for this id in this host (a partial-provider composition,
        // e.g. a Hall provider character not registered here). The body is an inert
        // stand-still with no binding (nothing to switch or snapshot).
        Err(ambition_characters::actor::character_catalog::BrainBuildError::UnknownCharacter(
            _,
        )) => {
            bevy::log::warn!(
                target: "ambition_platformer2d_actor_monolith::npcs",
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
    resolve_initial_brain, AuthoredBrainContext, BarkSituation, BrainBinding, CharacterCatalog,
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
    // AD8: the prepared cast, so a REGISTERED-only character is hurt in its own
    // voice. Without it the floor for this situation was engine-generic English
    // — see the fall-through below.
    registry: Option<&'a crate::character_runtime::PreparedCharacterRegistry>,
    interactable: &Interactable,
    strikes: i32,
) -> &'a str {
    let rotation = strikes.saturating_sub(1).max(0) as u32;
    if let Some(cid) = npc_character_id(interactable) {
        if let Some(line) = catalog.bark_line(cid, BarkSituation::OnHit, rotation) {
            return line;
        }
        // **THE FLOOR** — the same one the ambient ticker uses, which this path
        // did not consult. `CharacterDefinition::voice`'s doc calls itself the
        // floor so that "the floor is 'says something in character' rather than
        // silence", and for a hit that was not true: a registered-only character
        // said "Hey." in the engine's voice (AD8).
        if let Some(line) = registry.and_then(|registry| registry.get(cid)?.voice_line(rotation)) {
            return line;
        }
    }
    GENERIC_HIT_BARKS[(rotation as usize).min(GENERIC_HIT_BARKS.len().saturating_sub(1))]
}

/// The shout a peaceful actor makes at the moment it turns hostile: the catalog
/// `barks.provoked` pool (rotation 0), else the engine-generic default.
pub(crate) fn npc_hostile_bark_line<'a>(
    catalog: &'a CharacterCatalog,
    // AD8: as above — the moment a character turns on you is the worst one to
    // say it in somebody else's words.
    registry: Option<&'a crate::character_runtime::PreparedCharacterRegistry>,
    interactable: &Interactable,
) -> &'a str {
    if let Some(cid) = npc_character_id(interactable) {
        if let Some(line) = catalog.bark_line(cid, BarkSituation::Provoked, 0) {
            return line;
        }
        if let Some(line) = registry.and_then(|registry| registry.get(cid)?.voice_line(0)) {
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
    // The prepared cast, when this composition has one. A REGISTERED-only
    // character has no catalog row to hold pools, so without this it is mute —
    // which is what four Hall pedestals were (Jon, 2026-07-29).
    registry: Option<&'a crate::character_runtime::PreparedCharacterRegistry>,
    interactable: &Interactable,
    situation: BarkSituation,
    rotation: u32,
) -> Option<&'a str> {
    let cid = npc_character_id(interactable)?;
    if let Some(line) = catalog.bark_line(cid, situation, rotation) {
        return line.into();
    }
    // **THE FLOOR.** The catalog had nothing — either no pool for this
    // situation and no `fallback_dialogue`, or no row for this character at all.
    // A definition's own voice answers last, so a character another game
    // registered still speaks in its own words rather than standing silent.
    registry?.get(cid)?.voice_line(rotation)
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
    // An authored id must be non-EMPTY, not merely present. LDtk stores an
    // unset string field as `""`, so a spawn with no conversation arrives here
    // as `Some("")` and used to be forwarded verbatim -- the dialogue bridge
    // then logged `start(""): Yarn node not found` and the NPC opened nothing.
    // Blank is the same statement as absent: this character has no bespoke
    // scene, so it gets the generic one.
    let dialogue_id = match &interactable.kind {
        InteractionKind::Npc {
            dialogue_id: Some(dialogue_id),
            ..
        } if !dialogue_id.trim().is_empty() => dialogue_id.clone(),
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

    /// LDtk writes an unset string field as `""`, so a pedestal with no bespoke
    /// conversation reaches the dialogue bridge as `Some("")`. Forwarding that
    /// produced `start(""): Yarn node not found` and an NPC that opened nothing
    /// when you pressed interact -- the exact shape of "this character has no
    /// dialogue", reported as a missing Yarn node.
    #[test]
    fn a_blank_dialogue_id_means_absent_not_a_yarn_node_named_nothing() {
        let blank = Interactable::new(
            "voice",
            "Talk",
            ambition_platformer2d_core::Aabb::new(
                ambition_platformer2d_core::Vec2::ZERO,
                ambition_platformer2d_core::Vec2::new(1.0, 1.0),
            ),
            InteractionKind::Npc {
                character_id: Some("npc_marie_curry".to_string()),
                dialogue_id: Some(String::new()),
                patrol_radius: 0.0,
                patrol_path_id: None,
                brain_override: None,
            },
        );
        assert_eq!(
            npc_dialogue_request(&blank, "Marie Curry", "pedestal").dialogue_id,
            "generic_npc",
        );
        // Whitespace is blank too.
        let spaces = Interactable::new(
            "voice",
            "Talk",
            ambition_platformer2d_core::Aabb::new(
                ambition_platformer2d_core::Vec2::ZERO,
                ambition_platformer2d_core::Vec2::new(1.0, 1.0),
            ),
            InteractionKind::Npc {
                character_id: None,
                dialogue_id: Some("   ".to_string()),
                patrol_radius: 0.0,
                patrol_path_id: None,
                brain_override: None,
            },
        );
        assert_eq!(
            npc_dialogue_request(&spaces, "X", "y").dialogue_id,
            "generic_npc",
        );
    }
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
            ambition_platformer2d_core::Aabb::new(
                ambition_platformer2d_core::Vec2::ZERO,
                ambition_platformer2d_core::Vec2::new(1.0, 1.0),
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

    /// One registered character, prepared and published, with no `App` around it.
    fn registry_with(
        definition: crate::character_runtime::CharacterDefinition,
    ) -> crate::character_runtime::PreparedCharacterRegistry {
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &crate::character_runtime::CharacterBindings::default(),
        );
        let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
        registry.insert_prepared(finalized.prepared);
        registry
    }

    /// **A character with NO catalog row still speaks, if it brought a voice.**
    ///
    /// The Hall's ambient ticker skips whoever `npc_ambient_bark_line` answers
    /// `None` for, so "registered but not in the catalog" and "mute on a
    /// pedestal" were the same state — which is what four characters in the
    /// gallery were (Jon, 2026-07-29). Every character another game brings is
    /// registered-only, so this is the floor for consumers of the engine, not a
    /// detail of Ambition's own cast.
    #[test]
    fn a_registered_only_character_speaks_its_own_voice() {
        // A catalog that has never heard of this character.
        let catalog = CharacterCatalog::from_data(parse_catalog(FIRST));
        let npc = interactable();
        let registry = registry_with(
            crate::character_runtime::CharacterDefinition::new("voice", "Voice", "another_game")
                .with_voice(["only line", "second line"]),
        );

        assert_eq!(
            npc_ambient_bark_line(&catalog, None, &npc, BarkSituation::Hall, 0),
            None,
            "vacuity check: without the registry this character is mute, which is \
             the state this seam exists to fix"
        );
        assert_eq!(
            npc_ambient_bark_line(&catalog, Some(&registry), &npc, BarkSituation::Hall, 0),
            Some("only line"),
        );
        assert_eq!(
            npc_ambient_bark_line(&catalog, Some(&registry), &npc, BarkSituation::Hall, 1),
            Some("second line"),
            "rotation cycles the pool, so a repeated bark varies"
        );
    }

    /// **The voice is a floor for EVERY situation, not only the ambient one.**
    /// (AD8)
    ///
    /// `CharacterDefinition::voice` calls itself the floor so that "the floor is
    /// 'says something in character' rather than silence". It was reached by the
    /// ambient ticker and by nothing else: a registered-only character that got
    /// hit said "Hey." and one that turned hostile said "That's it!" — the
    /// engine's words in a character's mouth, which is the failure the whole
    /// registered-character voice seam exists to end.
    #[test]
    fn a_registered_characters_voice_is_the_floor_when_it_is_hit_and_provoked() {
        // A row that authors an IDLE pool and nothing for being hit or provoked
        // — the ordinary state of a character somebody has written ambience for
        // and not combat lines. `FIRST` authors both, so the catalog would
        // correctly win there and the floor would never be reached.
        const AMBIENT_ONLY: &str = r#"(
            brain_presets: { "idle": StandStill },
            action_set_presets: { "peaceful": (move_style: Walk) },
            characters: {
                "voice": (
                    display_name: "Voice", spritesheet: "voice.png",
                    manifest: "voice_spritesheet.ron", tier: MainHall,
                    body_kind: Standard, composition: None,
                    default_brain: "idle", default_action_set: "peaceful", tags: [],
                    barks: ( idle: ["first idle"] ),
                ),
            },
        )"#;
        let catalog = CharacterCatalog::from_data(parse_catalog(AMBIENT_ONLY));
        let npc = interactable();
        let registry = registry_with(
            crate::character_runtime::CharacterDefinition::new("voice", "Voice", "another_game")
                .with_voice(["ow, my paint", "that is enough"]),
        );

        // VACUITY FIRST: without the registry these are the engine's lines, which
        // is the state this closes.
        assert_eq!(
            npc_hit_bark_line(&catalog, None, &npc, 1),
            GENERIC_HIT_BARKS[0],
            "vacuity check: with no prepared cast the engine speaks, which is what \
             made this a defect rather than a preference"
        );
        assert_eq!(
            npc_hostile_bark_line(&catalog, None, &npc),
            GENERIC_HOSTILE_BARK
        );

        assert_eq!(
            npc_hit_bark_line(&catalog, Some(&registry), &npc, 1),
            "ow, my paint",
            "a struck character still spoke in the engine's voice"
        );
        assert_eq!(
            npc_hit_bark_line(&catalog, Some(&registry), &npc, 2),
            "that is enough",
            "the hit rotation must cycle the character's own pool, like the \
             catalog's does"
        );
        assert_eq!(
            npc_hostile_bark_line(&catalog, Some(&registry), &npc),
            "ow, my paint",
            "a character turning hostile spoke in the engine's voice — the worst \
             moment to borrow somebody else's words"
        );
    }

    /// The CATALOG still outranks a definition's voice: the voice is a floor,
    /// not an override.
    #[test]
    fn an_authored_catalog_pool_outranks_the_definitions_voice() {
        let catalog = CharacterCatalog::from_data(parse_catalog(FIRST));
        let npc = interactable();
        let registry = registry_with(
            crate::character_runtime::CharacterDefinition::new("voice", "Voice", "another_game")
                .with_voice(["floor line"]),
        );

        assert_eq!(
            npc_ambient_bark_line(&catalog, Some(&registry), &npc, BarkSituation::Idle, 0),
            Some("first idle"),
            "the catalog authored an Idle pool, so the definition's floor must not \
             displace it"
        );
    }

    #[test]
    fn explicit_catalog_argument_is_the_bark_authority() {
        let first = CharacterCatalog::from_data(parse_catalog(FIRST));
        let second = CharacterCatalog::from_data(parse_catalog(SECOND));
        let npc = interactable();

        assert_eq!(npc_hit_bark_line(&first, None, &npc, 1), "first hit");
        assert_eq!(npc_hit_bark_line(&second, None, &npc, 1), "second hit");
        assert_eq!(npc_hostile_bark_line(&first, None, &npc), "first provoked");
        assert_eq!(
            npc_hostile_bark_line(&second, None, &npc),
            "second provoked"
        );
        assert_eq!(
            npc_ambient_bark_line(&first, None, &npc, BarkSituation::Idle, 0),
            Some("first idle")
        );
        assert_eq!(
            npc_ambient_bark_line(&second, None, &npc, BarkSituation::Idle, 0),
            Some("second idle")
        );
    }
}

/// **Answer a cut conversation's bark request.** (sim)
///
/// ⭐ **the CAST half of the continuity port.** `conversation::rules` decides a
/// conversation broke and says WHO should speak; this decides WHAT they say,
/// because that needs the character catalog, the prepared registry and the
/// `Interactable` → character-id resolution — none of which is about continuity.
/// Splitting it this way is what removes `conversation`'s only two edges back
/// into this crate (`docs/planning/engine/actor-monolith-decomposition.md`).
///
/// ⚠ **an empty pool is SILENCE, and that is the finished behaviour.** No
/// character has a `conversation_cut` line yet; those are Jon's voice to write,
/// not the engine's to invent. The mechanism is complete and the content is a
/// seam.
///
/// ⚠ the catalog is OPTIONAL for the reason the break rule's was: a composition
/// with no catalog (a demo, a headless fixture) must still break conversations,
/// and losing an unwritten line is not worth failing over.
pub fn speak_conversation_cut_barks(
    mut requests: bevy::prelude::MessageReader<crate::conversation::ConversationCutBark>,
    speakers: bevy::prelude::Query<(
        &ambition_platformer2d_shared_tangle::body::BodyKinematics,
        &ambition_combat::ActorInteraction,
    )>,
    character_catalog: Option<bevy::prelude::Res<CharacterCatalog>>,
    prepared_cast: Option<bevy::prelude::Res<crate::character_runtime::PreparedCharacterRegistry>>,
    mut vfx: bevy::prelude::MessageWriter<ambition_vfx::vfx::VfxMessage>,
) {
    for request in requests.read() {
        let Ok((kin, interaction)) = speakers.get(request.speaker) else {
            continue;
        };
        let Some(line) = character_catalog.as_deref().and_then(|catalog| {
            npc_ambient_bark_line(
                catalog,
                prepared_cast.as_deref(),
                &interaction.interactable,
                BarkSituation::ConversationCut,
                0,
            )
        }) else {
            continue;
        };
        vfx.write(ambition_vfx::vfx::VfxMessage::SpeechBubble {
            pos: kin.pos + ambition_platformer2d_core::Vec2::new(0.0, -kin.size.y * 0.72 - 16.0),
            text: line.to_string(),
        });
    }
}

/// D73 phase 1: the NPC spawn path asks the CHARACTER what it normally does.
#[cfg(test)]
mod default_profile_tests {
    use super::*;
    use ambition_characters::actor::character_catalog::parse_catalog;

    const CATALOG: &str = r#"(
        brain_presets: {
            "stand_still": StandStill,
            "wanderer_puppy_slug": Wanderer(speed: 36.0, aggressiveness: 0.0),
            "patrol_peaceful": Patrol(
                spawn_local_x: 0.0, radius: 64.0, speed: 28.0,
                aggressiveness: 0.0, aggro_radius: 80.0, attack_range: 0.0,
            ),
        },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "npc_puppy_slug": (
                display_name: "Puppy Slug", spritesheet: "x.png", manifest: "x_spritesheet.ron",
                tier: MainHall, body_kind: Crawler, composition: None,
                default_brain: "wanderer_puppy_slug", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

    fn npc(brain_override: Option<&str>) -> Interactable {
        Interactable::new(
            "slug",
            "Talk",
            ambition_platformer2d_core::Aabb::new(
                ambition_platformer2d_core::Vec2::ZERO,
                ambition_platformer2d_core::Vec2::new(1.0, 1.0),
            ),
            InteractionKind::Npc {
                character_id: Some("npc_puppy_slug".to_string()),
                dialogue_id: None,
                patrol_radius: 0.0,
                patrol_path_id: None,
                brain_override: brain_override.map(str::to_string),
            },
        )
    }

    /// A registry holding one prepared character that names `profile` as its
    /// own default, built through the real preparation path rather than by
    /// hand — a hand-built `PreparedCharacterDefinition` would prove that this
    /// test can construct a struct.
    fn registry_naming(
        profile: Option<&str>,
    ) -> crate::character_runtime::PreparedCharacterRegistry {
        let mut definition = crate::character_runtime::CharacterDefinition::new(
            "npc_puppy_slug",
            "Puppy Slug",
            "test",
        );
        definition.default_brain_profile =
            profile.map(ambition_characters::actor::character_catalog::BrainProfileRef::from);
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &crate::character_runtime::CharacterBindings::default(),
        );
        let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
        registry.insert_prepared(finalized.prepared);
        registry
    }

    /// ⭐ **the character's own default reaches a spawned NPC.** Before this the
    /// only thing that could state an NPC's normal behaviour was its catalog
    /// row, so a registered character with its own view of itself was ignored on
    /// the one path that spawns most of the cast.
    #[test]
    fn an_npc_takes_its_definitions_default_profile_over_the_catalog_rows() {
        let catalog = ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
            parse_catalog(CATALOG),
        );
        let (brain, binding) = resolve_npc_brain(
            &catalog,
            &registry_naming(Some("patrol_peaceful")),
            &npc(None),
            0.0,
        );
        assert_eq!(
            brain.label(),
            "patrol",
            "the definition said `patrol_peaceful`; the row says `wanderer_puppy_slug`"
        );
        assert_eq!(
            binding
                .as_ref()
                .and_then(|(b, _)| b.default_preset.as_ref())
                .map(|p| p.as_str()),
            Some("patrol_peaceful"),
            "and it is the binding's DEFAULT, so a later restore returns here"
        );
    }

    /// ⛔ **the parity case, and it is the one that must not break**: a
    /// definition that names nothing leaves the catalog row in charge. Every
    /// character in the repo is this one today, so a regression here is a
    /// silent behaviour change across the whole cast.
    #[test]
    fn a_definition_naming_nothing_leaves_the_catalog_row_in_charge() {
        let catalog = ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
            parse_catalog(CATALOG),
        );
        for registry in [
            registry_naming(None),
            crate::character_runtime::PreparedCharacterRegistry::default(),
        ] {
            let (brain, _) = resolve_npc_brain(&catalog, &registry, &npc(None), 0.0);
            assert_eq!(brain.label(), "wanderer");
        }
    }

    /// And an authored placement override still outranks both.
    #[test]
    fn a_placement_override_outranks_the_definitions_default() {
        let catalog = ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
            parse_catalog(CATALOG),
        );
        let (brain, _) = resolve_npc_brain(
            &catalog,
            &registry_naming(Some("patrol_peaceful")),
            &npc(Some("stand_still")),
            0.0,
        );
        assert_eq!(brain.label(), "stand_still");
    }
}
