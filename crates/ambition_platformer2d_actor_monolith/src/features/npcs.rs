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

pub const NPC_HOSTILE_STRIKE_THRESHOLD: i32 = 3;

/// When the player gets within this many world pixels, a patrolling NPC stops and faces the
/// player so the dialog interact is reachable. ~80 px ≈ 2.5 player widths — close enough to
/// commit to dialog, far enough that an NPC doesn't freeze the moment you walk past their
/// patrol range.
pub const NPC_TALK_RADIUS: f32 = 80.0;

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
    // An EMPTY registry is a legal, meaningful value: no character states a default, which is
    // what this path assumed before definitions could state one.
    prepared: &ambition_characters::prepared::PreparedCharacterRegistry,
    interactable: &Interactable,
    spawn_world_x: f32,
    // The body being built, so a character whose default policy is a
    // `BrainProfile` can have it lowered against its OWN top speed rather than
    // against a preset's absolute numbers (§4.7).
    body: &ambition_combat::actor_tuning::ActorConfig,
    abilities: ambition_platformer2d_core::AbilitySet,
    // ⭐⭐ THE MEASUREMENT KNOBS, AS A VALUE THE CALLER HANDS DOWN. This function
    // used to call `ambition_dev_tools::brain_override::forced_profile()` and
    // `forced_preset()` — the simulation reaching up into a developer crate,
    // mid-brain-construction, to decide what the world contains. It reads a
    // session-owned [`AuthoredBrainOverride`] now, which the dev tool writes;
    // `Default` is "the author decides" and is what a composition with no
    // developer tools supplies.
    forced: &ambition_characters::brain::AuthoredBrainOverride,
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
    // ```text
    //   the placement's brain_override   a scene saying "this one is a guard"
    //   the CHARACTER's own profile      what a Goblin normally does
    //   the catalog row's default_brain  what a body gets when nobody migrated it
    // ```
    //
    // a content guard (`a_character_states_its_policy_in_one_place`) already
    // forbids a character authoring a profile while its row names a preset, so
    // this branch and that guard agree today. The guard is the belt; this is the
    // structure — a rule that only holds because content happens not to violate
    // it is not a rule.
    let character_profile = prepared
        .get(cid)
        .and_then(|prepared| prepared.autonomous_profile);
    // ⚠ A MEASUREMENT KNOB, UNSET IN EVERY ORDINARY RUN. When set it stands in
    // for the placement's own override, so it beats the character's profile
    // exactly as an authored override does — the hall's cast is authored
    // `stand_still`, and a forced preset that lost to a profile would silently
    // measure the wrong cast. See `ambition_dev_tools::brain_override`.
    // ⚠ THE OTHER MEASUREMENT KNOB, and it exists because the preset road cannot
    // reach a perception-reading brain: every catalog preset lowers to an arm
    // `tick_simple_state_machine` answers, and that takes no `WorldView`.
    // `Fighter` is reachable only here. Unknown names panic rather than falling
    // back, for the same reason a bad preset does.
    if let Some(name) = forced.profile() {
        let profile = catalog.autonomous_profile(name).unwrap_or_else(|| {
            panic!(
                "AMBITION_ACTOR_BRAIN_PROFILE names unknown autonomous profile `{name}`"
            )
        });
        let mut config = body.clone();
        config.brain_profile = *profile;
        return (
            crate::features::ecs::enemy_default_brain(&config, abilities),
            Some((BrainBinding::from_character_profile(), authored)),
        );
    }
    let forced_preset = forced.preset();
    if forced_preset.is_none()
        && brain_override
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
    {
        if let Some(profile) = character_profile {
            let mut config = body.clone();
            config.brain_profile = profile;
            return (
                crate::features::ecs::enemy_default_brain(&config, abilities),
                Some((BrainBinding::from_character_profile(), authored)),
            );
        }
    }
    match resolve_initial_brain(
        catalog,
        cid,
        forced_preset.or(brain_override.as_deref()),
        &authored.build_context(),
    ) {
        Ok((binding, brain)) => (brain, Some((binding, authored))),
        // THE CHARACTER'S OWN POLICY IS THE DEFAULT.
        //
        // this is the seam the migration left half-crossed. A migrated character states its normal
        // behaviour as a `BrainProfile` and its catalog `default_brain` was emptied so one
        // authority decides — but this road only spoke the PRESET vocabulary.
        //
        // the lowering happens HERE and not in `resolve_initial_brain` because
        // it needs the BODY: §4.7's seam is a policy's normalized effort against
        // the body's own top speed, and `ambition_characters` has no body. That
        // is why the resolver redirects rather than answering.
        Err(
            ambition_characters::actor::character_catalog::BrainBuildError::NoAutonomousDefault {
                ..
            },
        ) => {
            // reaching here means NOTHING is authored anywhere. The profile
            // rank above already answered for every character that states one, and
            // the row is empty or this error would not exist — so this is a
            // genuinely unauthored character, not a vocabulary mismatch. A body
            // that stands still is the honest answer, and the same one the
            // anonymous-NPC arm gives.
            //
            // it is also reachable for a placement whose `brain_override` is
            // present but blank, on a character that authors a profile — a case
            // the branch above deliberately routes through the profile rank by
            // treating a blank override as absent, exactly as
            // `resolve_initial_brain` does.
            bevy::log::warn!(
                target: "crate::npcs",
                "NPC `{cid}` names no brain preset and its character authors no \
                 autonomous profile; stand-still fallback",
            );
            (ambition_characters::brain::Brain::stand_still(), None)
        }
        // No catalog row for this id in this host (a partial-provider composition,
        // e.g. a Hall provider character not registered here). The body is an inert
        // stand-still with no binding (nothing to switch or snapshot).
        Err(ambition_characters::actor::character_catalog::BrainBuildError::UnknownCharacter(
            _,
        )) => {
            bevy::log::warn!(
                target: "crate::npcs",
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
use ambition_combat::events::NpcDialogueRequest;
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
    registry: Option<&'a ambition_characters::prepared::PreparedCharacterRegistry>,
    interactable: &Interactable,
    strikes: i32,
) -> &'a str {
    let rotation = strikes.saturating_sub(1).max(0) as u32;
    if let Some(cid) = npc_character_id(interactable) {
        if let Some(line) = catalog.bark_line(cid, BarkSituation::OnHit, rotation) {
            return line;
        }
        // THE FLOOR — the same one the ambient ticker uses, which this path
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
    registry: Option<&'a ambition_characters::prepared::PreparedCharacterRegistry>,
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
    // which is what four Hall pedestals were.
    registry: Option<&'a ambition_characters::prepared::PreparedCharacterRegistry>,
    interactable: &Interactable,
    situation: BarkSituation,
    rotation: u32,
) -> Option<&'a str> {
    let cid = npc_character_id(interactable)?;
    if let Some(line) = catalog.bark_line(cid, situation, rotation) {
        return line.into();
    }
    // THE FLOOR. The catalog had nothing — either no pool for this
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
    // An authored id must be non-EMPTY, not merely present. Blank is the same statement as absent:
    // this character has no bespoke scene, so it gets the generic one.
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

    /// LDtk writes an unset string field as `""`, so a pedestal with no bespoke conversation
    /// reaches the dialogue bridge as `Some("")`.
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
        definition: ambition_characters::actor::definition::CharacterDefinition,
    ) -> ambition_characters::prepared::PreparedCharacterRegistry {
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &ambition_characters::prepared::CharacterBindings::default(),
        );
        let mut registry = ambition_characters::prepared::PreparedCharacterRegistry::default();
        registry.insert_prepared(finalized.prepared);
        registry
    }

    /// A character with NO catalog row still speaks, if it brought a voice.
    ///
    /// The Hall's ambient ticker skips whoever `npc_ambient_bark_line` answers
    /// `None` for, so "registered but not in the catalog" and "mute on a
    /// pedestal" were the same state — which is what four characters in the
    /// gallery were. Every character another game brings is
    /// registered-only, so this is the floor for consumers of the engine, not a
    /// detail of Ambition's own cast.
    #[test]
    fn a_registered_only_character_speaks_its_own_voice() {
        // A catalog that has never heard of this character.
        let catalog = CharacterCatalog::from_data(parse_catalog(FIRST));
        let npc = interactable();
        let registry = registry_with(
            ambition_characters::actor::definition::CharacterDefinition::new(
                "voice",
                "Voice",
                "another_game",
            )
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

    /// The voice is a floor for EVERY situation, not only the ambient one.
    /// (AD8)
    ///
    /// `CharacterDefinition::voice` calls itself the floor so that "the floor is 'says something in
    /// character' rather than silence".
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
            ambition_characters::actor::definition::CharacterDefinition::new(
                "voice",
                "Voice",
                "another_game",
            )
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
            ambition_characters::actor::definition::CharacterDefinition::new(
                "voice",
                "Voice",
                "another_game",
            )
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

/// Answer a cut conversation's bark request. (sim)
///
/// the CAST half of the continuity port. `ambition_conversation::rules` decides a
/// conversation broke and says WHO should speak; this decides WHAT they say,
/// because that needs the character catalog, the prepared registry and the
/// `Interactable` → character-id resolution — none of which is about continuity.
/// Splitting it this way is what removes `conversation`'s only two edges back
/// into this crate (`docs/planning/engine/actor-monolith-decomposition.md`).
///
/// the catalog is OPTIONAL for the reason the break rule's was: a composition
/// with no catalog (a demo, a headless fixture) must still break conversations,
/// and losing an unwritten line is not worth failing over.
pub fn speak_conversation_cut_barks(
    mut requests: bevy::prelude::MessageReader<ambition_conversation::ConversationCutBark>,
    speakers: bevy::prelude::Query<(
        &ambition_platformer2d_shared_tangle::body::BodyKinematics,
        &ambition_combat::ActorInteraction,
    )>,
    character_catalog: Option<bevy::prelude::Res<CharacterCatalog>>,
    prepared_cast: Option<
        bevy::prelude::Res<ambition_characters::prepared::PreparedCharacterRegistry>,
    >,
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

/// : the NPC spawn path asks the CHARACTER what it normally does.
#[cfg(test)]
mod default_profile_tests {
    use super::*;

    /// The provider both authorities are registered under. ONE constant on
    /// purpose: the whole point of the fixture is that a definition's provider
    /// and its catalog fragment's provider are the same identifier.
    const PROVIDER: &str = "test";

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

    /// The fixture catalog, ASSEMBLED — not parsed.
    ///
    /// the difference is the whole test. A parsed fragment keeps its authored
    /// keys raw (`patrol_peaceful`); assembly namespaces every preset as
    /// `provider::name`, which is what the game actually runs against. While
    /// this fixture skipped assembly, a definition whose provider qualified its
    /// own profile produced a key no catalog had — and that was read as
    /// evidence against qualifying by provider, when it was evidence the
    /// fixture was not modelling production.
    fn assembled_catalog() -> ambition_characters::actor::character_catalog::CharacterCatalog {
        let mut registry =
            ambition_characters::actor::character_catalog::CharacterCatalogRegistry::default();
        registry
            .register(
                ambition_characters::actor::character_catalog::CharacterCatalogFragment::from_ron(
                    PROVIDER,
                    None::<String>,
                    CATALOG,
                )
                .expect("the fixture catalog is valid"),
            )
            .expect("one fragment always registers");
        registry
            .assemble()
            .expect("one fragment always assembles")
            .catalog
    }

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

    /// A registry holding one prepared character that authors `profile` as its
    /// own policy, built through the real preparation path rather than by hand —
    /// a hand-built `PreparedCharacterDefinition` would prove that this test can
    /// construct a struct.
    ///
    /// a `BrainProfile`, which is the only vocabulary a character has. This took a
    /// `BrainPresetRef` and wrote `definition.default_brain_profile`, a field no character in
    /// the repo ever authored and which is now deleted.
    fn registry_naming(
        profile: Option<ambition_characters::brain::CharacterBrainTemplate>,
    ) -> ambition_characters::prepared::PreparedCharacterRegistry {
        let mut definition = ambition_characters::actor::definition::CharacterDefinition::new(
            "npc_puppy_slug",
            "Puppy Slug",
            PROVIDER,
        );
        definition.autonomous_profile =
            profile.map(ambition_characters::brain::BrainProfile::from_template);
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &ambition_characters::prepared::CharacterBindings::default(),
        );
        let mut registry = ambition_characters::prepared::PreparedCharacterRegistry::default();
        registry.insert_prepared(finalized.prepared);
        registry
    }

    /// A minimal body for the two parameters `resolve_npc_brain` needs only when
    /// a character's default is its own `BrainProfile`.
    fn test_body() -> ambition_combat::actor_tuning::ActorConfig {
        crate::features::ecs::actor_clusters::ActorClusterSeed::new_peaceful_npc(
            "probe",
            "Probe",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(8.0, 12.0)),
            &npc(None),
            &[],
        )
        .0
        .config
    }

    /// the character's own policy reaches a spawned NPC, and OUTRANKS its catalog row — the
    /// rule, restated in the vocabulary that survived.
    ///
    /// the previous version of this test authored a `BrainPresetRef` on the
    /// definition, and it passed for months while being about a road no
    /// character in the repo ever took. The row therefore outranked the
    /// character on every body in the game, and the test said otherwise.
    #[test]
    fn an_npc_takes_its_characters_own_policy_over_the_catalog_rows() {
        let catalog = assembled_catalog();
        let (brain, binding) = resolve_npc_brain(
            &catalog,
            &registry_naming(Some(
                ambition_characters::brain::CharacterBrainTemplate::StandStill,
            )),
            &npc(None),
            0.0,
            &test_body(),
            ambition_platformer2d_core::AbilitySet::NONE,
            &ambition_characters::brain::AuthoredBrainOverride::default(),
        );
        assert_eq!(
            brain.label(),
            "stand_still",
            "the character authors a StandStill policy; the row says \
             `wanderer_puppy_slug` and must not win"
        );
        let binding = binding.expect("a catalog-backed NPC is bound").0;
        assert_eq!(
            binding.default_preset,
            ambition_characters::actor::character_catalog::AutonomousDefault::CharacterProfile,
            "and the binding says WHAT IT WILL GO BACK TO — not a preset id, and \
             above all not an empty one, which is what crashed two rooms"
        );
        assert_eq!(
            binding.source,
            ambition_characters::actor::character_catalog::AutonomousSource::CharacterProfile,
        );
    }

    /// the parity case, and it is the one that must not break: a character that authors nothing
    /// leaves the catalog row in charge.
    #[test]
    fn a_character_authoring_nothing_leaves_the_catalog_row_in_charge() {
        let catalog = assembled_catalog();
        for registry in [
            registry_naming(None),
            ambition_characters::prepared::PreparedCharacterRegistry::default(),
        ] {
            let (brain, _) = resolve_npc_brain(
                &catalog,
                &registry,
                &npc(None),
                0.0,
                &test_body(),
                ambition_platformer2d_core::AbilitySet::NONE,
                &ambition_characters::brain::AuthoredBrainOverride::default(),
            );
            assert_eq!(brain.label(), "wanderer");
        }
    }

    /// And an authored placement override still outranks both — a scene that
    /// deliberately says *"this one is a guard"* beats what the character
    /// normally does, which is the entire reason the override exists.
    #[test]
    fn a_placement_override_outranks_the_characters_own_policy() {
        let catalog = assembled_catalog();
        let (brain, _) = resolve_npc_brain(
            &catalog,
            &registry_naming(Some(
                ambition_characters::brain::CharacterBrainTemplate::Wanderer,
            )),
            &npc(Some("stand_still")),
            0.0,
            &test_body(),
            ambition_platformer2d_core::AbilitySet::NONE,
            &ambition_characters::brain::AuthoredBrainOverride::default(),
        );
        assert_eq!(brain.label(), "stand_still");
    }

    /// ⛔⛔ THE SIM READS A VALUE, AND THAT VALUE IS WHAT STEERS THE CAST.
    ///
    /// Until 2026-09-02 this function called
    /// `ambition_dev_tools::brain_override::forced_preset()` and
    /// `forced_profile()` — the actor kernel reaching UP into a developer crate,
    /// mid-brain-construction, to decide what the world contains. The knob is a
    /// session-owned `AuthoredBrainOverride` the dev tool writes and lowering
    /// reads, which is D33's stated shape and the one `ClockScaleRequest`
    /// already uses for slow-motion.
    ///
    /// ⛔ BOTH ARMS, because either alone passes on a broken build: an override
    /// that never applies passes the quiet arm, and one that always applies
    /// passes the forced arm. What is pinned is that the VALUE decides — and
    /// the pair is the same character and the same placement, so nothing but
    /// the override differs.
    #[test]
    fn the_developer_override_steers_the_cast_and_its_absence_does_not() {
        let catalog = assembled_catalog();
        let registry = registry_naming(Some(
            ambition_characters::brain::CharacterBrainTemplate::Wanderer,
        ));
        let resolve = |forced: &ambition_characters::brain::AuthoredBrainOverride| {
            resolve_npc_brain(
                &catalog,
                &registry,
                &npc(None),
                0.0,
                &test_body(),
                ambition_platformer2d_core::AbilitySet::NONE,
                forced,
            )
            .0
            .label()
        };

        assert_eq!(
            resolve(&ambition_characters::brain::AuthoredBrainOverride::default()),
            "wanderer",
            "with nobody steering, the character's own policy decides — which is \
             what an unset environment variable has always meant",
        );
        assert_eq!(
            resolve(&ambition_characters::brain::AuthoredBrainOverride {
                preset: Some("stand_still".to_string()),
                profile: None,
            }),
            "stand_still",
            "the override changed nothing: a measurement taken with \
             AMBITION_ACTOR_BRAIN_OVERRIDE set would report the authored cast \
             while claiming to describe a forced one",
        );
    }
}
