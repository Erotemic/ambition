//! NPC SPAWN POLICY — the decisions a body's construction needs and nothing else.
//!
//! ⭐⭐ IT IS HERE AND NOT IN `features::npcs` BECAUSE OF ONE SIGNATURE. A review
//! asked, for each thing the spawn primitives still reached upward for, whether it
//! was generic vocabulary that should move down, actor-domain policy that should be
//! SUPPLIED INTO spawn, or authored-content conversion belonging above both.
//!
//! ⛔ These are policy, and "supplied into spawn" is the obvious answer and the
//! unavailable one: `resolve_npc_brain` takes the `ActorConfig` of the body being
//! built — *"so a character whose default policy is a `BrainProfile` can have it
//! lowered against its OWN top speed rather than against a preset's absolute
//! numbers"* — and that config does not exist until the primitive has built it.
//! **The policy is a function of what spawn produces**, so hoisting it to the
//! caller needs a closure or a trait, which trades one coupling for a worse one.
//!
//! ⇒ So it moved DOWN instead, and `features::npcs` keeps NPC BEHAVIOUR: the
//! systems that run while an NPC is alive. The split is spawn-time resolution
//! versus runtime conduct, and the measurement supported it — `npcs.rs`'s only
//! production reach into the feature layer was `enemy_default_brain`, which
//! already lives beside this file.

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_interaction::{Interactable, InteractionKind};
use ambition_characters::actor::character_catalog::binding::{AuthoredBrainContext, BrainBinding};

pub const NPC_HOSTILE_STRIKE_THRESHOLD: i32 = 3;

pub const NPC_TALK_RADIUS: f32 = 80.0;

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
            super::brain_builders::enemy_default_brain(&config, abilities),
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
                super::brain_builders::enemy_default_brain(&config, abilities),
                Some((BrainBinding::from_character_profile(), authored)),
            );
        }
    }
    match ambition_characters::actor::character_catalog::binding::resolve_initial_brain(
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
