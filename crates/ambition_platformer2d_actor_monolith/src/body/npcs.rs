//! Character voice, dialogue requests and ambient speech for talkable actors.

use super::*;

/// Engine-generic on-hit barks for an interactable actor whose catalog row
/// authors no `barks.on_hit` pool (an unnamed mob, or a placed NPC carrying no
/// `character_id`). Named per-character voices live in the catalog — this is
/// only the anonymous default. Rotation cycles the pool.
const GENERIC_HIT_BARKS: &[&str] = &["Hey.", "Cut it out.", "Okay, now I'm mad."];

/// Engine-generic shout an anonymous actor makes when it turns hostile (no
/// catalog `barks.provoked` pool). Named archetypes author their own.
const GENERIC_HOSTILE_BARK: &str = "That's it!";

use ambition_characters::actor::character_catalog::{BarkSituation, CharacterCatalog};
use ambition_interaction::{Interactable, InteractionKind};

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

#[cfg(test)]
mod tests {
    use super::*;

    use ambition_characters::actor::character_catalog::{parse_catalog, CharacterCatalog};

    const FIRST: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "voice": (
                display_name: "Voice",
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
                display_name: "Voice",
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

    #[test]
    fn explicit_catalog_argument_is_the_bark_authority() {
        let first = CharacterCatalog::from_data(parse_catalog(FIRST));
        let second = CharacterCatalog::from_data(parse_catalog(SECOND));
        let npc = interactable();

        assert_eq!(npc_hit_bark_line(&first, &npc, 1), "first hit");
        assert_eq!(npc_hit_bark_line(&second, &npc, 1), "second hit");
        assert_eq!(npc_hostile_bark_line(&first, &npc), "first provoked");
        assert_eq!(
            npc_hostile_bark_line(&second, &npc),
            "second provoked"
        );
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

/// Resolve the text for a bark requested by the conversation domain.
///
/// The character catalog is required because this system resolves the
/// speaker's identity and authored dialogue. A missing authored bark is
/// valid and produces no speech. Conversation continuity decides when
/// to request a bark; character runtime decides what it says.
pub fn speak_conversation_cut_barks(
    mut requests: bevy::prelude::MessageReader<ambition_conversation::ConversationCutBark>,
    speakers: bevy::prelude::Query<(
        &ambition_platformer2d_core::BodyKinematics,
        &ambition_combat::ActorInteraction,
    )>,
    character_catalog: bevy::prelude::Res<CharacterCatalog>,
    mut vfx: ambition_vfx::vfx::VfxWriter,
    // The bark is drawn in the speaker's own live room.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    for request in requests.read() {
        let Ok((kin, interaction)) = speakers.get(request.speaker) else {
            continue;
        };
        let Some(line) = npc_ambient_bark_line(
            &character_catalog,
            &interaction.interactable,
            BarkSituation::ConversationCut,
            0,
        ) else {
            continue;
        };
        vfx.for_room(rooms.of(request.speaker)).write(ambition_vfx::vfx::VfxMessage::SpeechBubble {
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
                display_name: "Puppy Slug", manifest: "x_spritesheet.ron",
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
        definition.autonomous_policy = profile
            .map(ambition_characters::brain::BrainProfile::from_template)
            .map(ambition_characters::actor::AutonomousPolicy::Inline);
        let finalized = ambition_characters::prepared::prepare_and_finalize_for_test(
            definition,
            &ambition_characters::prepared::CharacterBindings::default(),
        );
        let mut registry = ambition_characters::prepared::PreparedCharacterRegistry::default();
        registry.insert_prepared(finalized.prepared);
        registry
    }

    fn test_identity() -> ambition_combat::components::ActorIdentity {
        ambition_combat::components::ActorIdentity::new("probe", "Probe")
    }

    /// A minimal body for the two parameters `resolve_npc_brain` needs only when
    /// a character's default is its own `BrainProfile`.
    fn test_body(sprite_content: &ambition_sprite_sheet::SpriteContent) -> ambition_combat::actor_tuning::ActorConfig {
        ambition_body_seed::ActorClusterSeed::new_peaceful_npc(&sprite_content,
            "probe",
            "Probe",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(8.0, 12.0)),
            &npc(None),
            &[],
        )
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
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
        let catalog = assembled_catalog();
        let (brain, binding) = ambition_platformer2d_actor_spawn::npc_policy::resolve_npc_brain(
            &catalog,
            &registry_naming(Some(
                ambition_characters::brain::CharacterBrainTemplate::StandStill,
            )),
            &npc(None),
            0.0,
            &test_body(&sprite_content),
            &test_identity(),
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
            ambition_characters::control::brain_binding::AutonomousDefault::CharacterProfile,
            "and the binding says WHAT IT WILL GO BACK TO — not a preset id, and \
             above all not an empty one, which is what crashed two rooms"
        );
        assert_eq!(
            binding.source,
            ambition_characters::control::brain_binding::AutonomousSource::CharacterProfile,
        );
    }

    /// the parity case, and it is the one that must not break: a character that authors nothing
    /// leaves the catalog row in charge.
    #[test]
    fn a_character_authoring_nothing_leaves_the_catalog_row_in_charge() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
        let catalog = assembled_catalog();
        for registry in [
            registry_naming(None),
            ambition_characters::prepared::PreparedCharacterRegistry::default(),
        ] {
            let (brain, _) = ambition_platformer2d_actor_spawn::npc_policy::resolve_npc_brain(
                &catalog,
                &registry,
                &npc(None),
                0.0,
                &test_body(&sprite_content),
                &test_identity(),
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
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
        let catalog = assembled_catalog();
        let (brain, _) = ambition_platformer2d_actor_spawn::npc_policy::resolve_npc_brain(
            &catalog,
            &registry_naming(Some(
                ambition_characters::brain::CharacterBrainTemplate::Wanderer,
            )),
            &npc(Some("stand_still")),
            0.0,
            &test_body(&sprite_content),
            &test_identity(),
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
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
        let catalog = assembled_catalog();
        let registry = registry_naming(Some(
            ambition_characters::brain::CharacterBrainTemplate::Wanderer,
        ));
        let resolve = |forced: &ambition_characters::brain::AuthoredBrainOverride| {
            ambition_platformer2d_actor_spawn::npc_policy::resolve_npc_brain(
                &catalog,
                &registry,
                &npc(None),
                0.0,
                &test_body(&sprite_content),
                &test_identity(),
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
