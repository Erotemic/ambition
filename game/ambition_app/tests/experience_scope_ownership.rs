#![cfg(feature = "input")]
//! **No two experiences may claim to be the sole publisher of one resource.**
//!
//! `ExperienceScopeBuilder::releasing` removes a resource outright when its
//! experience leaves, and the shape of that giveback IS a claim: *nobody else
//! publishes this, so deleting it can only ever delete mine.* The scope module's
//! headline rule says as much — *"release is OWNER-SCOPED, never 'remove the
//! resource'. Two experiences publish into the same global resource, so a scope
//! that removed it unconditionally would be one game deleting another's
//! match."*
//!
//! **the claim is unfalsifiable from inside one declaration.** Versus says
//! "the match ends WITH its route" and removes `ActiveMatch` by type; Smash says
//! "a match that ended with the route it ran on" and removes `ActiveMatch` by
//! type. Each file reads as correct. Both are registered into the SAME host
//! (`shell_host.rs` lists `SmashExperiencePlugin` beside Ambition's own), and it
//! is only with both declarations in front of you that the contradiction is
//! visible at all. That is why this lives in a test over the composed registry
//! rather than in a reviewer's head.
//!
//! **this is the roster's lesson, one resource later — for the fourth time.**
//! `MatchParticipantRoster` learned `published_by` across three separate
//! incidents, the last one a stage opening with one fighter instead of two. Both
//! files here carry a comment explaining that very lesson, correctly, on the
//! line ABOVE the declaration that repeats it.
//!
//! The invariant is deliberately about the CLAIM and not about any particular
//! resource: a new experience that removes somebody else's global by type fails
//! this the day it is written, whatever the resource is called.

use std::collections::BTreeMap;

use ambition_platformer2d::game_shell::{ReleaseKind, ShellExperienceScopes};
use bevy::prelude::*;

/// Compose the shipped multi-game host and hand back its scope registry.
///
/// Build-time only: scopes are declared in plugin `build`, so no frame has to run and none does.
fn compose_the_shipped_host() -> App {
    use ambition_app::app::shell_host;
    use bevy::asset::AssetPlugin;
    use bevy::image::ImagePlugin;
    use bevy::state::app::StatesPlugin;
    use bevy::transform::TransformPlugin;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition_platformer2d::platformer::schedule::GameMode>();
    app.insert_resource(shell_host::AmbitionShellHosted);
    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);
    app.add_plugins(ambition_platformer2d::host::PlatformerHostPlugins);
    shell_host::compose_ambition_shell_host(&mut app);
    app
}

/// Every REMOVAL in the composed host, keyed by the resource and naming the
/// owner and the ownership claim it made.
///
/// **removals only — a `Reset` is not evidence of publishing.** Putting a
/// resource back to its default is a claim about staleness, not about who owns
/// it, and counting it here would make two experiences sharing a cursor look
/// like two experiences owning a match.
fn removals(app: &App) -> BTreeMap<&'static str, Vec<(String, ReleaseKind)>> {
    let scopes = app
        .world()
        .get_resource::<ShellExperienceScopes>()
        .expect("the shipped host registers experience scopes");

    let mut removals: BTreeMap<&'static str, Vec<(String, ReleaseKind)>> = BTreeMap::new();
    for scope in scopes.iter() {
        for (what, kind) in scope.releases() {
            if matches!(kind, ReleaseKind::SoleRemoval | ReleaseKind::OwnedRemoval) {
                removals
                    .entry(what)
                    .or_default()
                    .push((scope.owner().as_str().to_string(), kind));
            }
        }
    }
    removals
}

/// **A resource more than one experience removes may not be removed BY TYPE.**
///
/// The rule that holds: **a second experience declaring a removal is itself the evidence that a
/// second experience publishes.** Nobody writes a giveback for state they never create.
#[test]
fn a_resource_two_experiences_remove_is_never_removed_by_type() {
    let app = compose_the_shipped_host();

    let offenders: Vec<_> = removals(&app)
        .into_iter()
        .filter(|(_, owners)| {
            owners.len() > 1
                && owners
                    .iter()
                    .any(|(_, kind)| *kind == ReleaseKind::SoleRemoval)
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "more than one experience removes each of these, so more than one \
         publishes it — and at least one removes it BY TYPE, which deletes the \
         other's state on the way out:\n{}\n\
         `releasing` claims the declaring experience is the only publisher. When \
         two of them do, the shape that is true is `releasing_owned` (ask the \
         value who published it) or `releasing_witnessed` (ask the resource that \
         knows, for state that cannot carry its own owner).",
        offenders
            .iter()
            .map(|(what, owners)| {
                let who = owners
                    .iter()
                    .map(|(owner, kind)| format!("{owner} ({kind:?})"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("  {what}\n    removed by: {who}")
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// **the probe for the probe.** An invariant over a registry is worth exactly
/// as much as the registry's contents, and a composition that registered one
/// experience — or none — would satisfy the assertion above by having nothing to
/// contest.
#[test]
fn the_host_composes_both_match_experiences_with_givebacks() {
    let app = compose_the_shipped_host();
    let scopes = app
        .world()
        .get_resource::<ShellExperienceScopes>()
        .expect("the shipped host registers experience scopes");

    let declaring: std::collections::BTreeSet<String> = scopes
        .iter()
        .filter(|scope| scope.releases().next().is_some())
        .map(|scope| scope.owner().as_str().to_string())
        .collect();

    for expected in ["ambition_versus", "smash"] {
        assert!(
            declaring.contains(expected),
            "`{expected}` declares no scoped givebacks in the composed host, so \
             the ownership assertion cannot see it at all. Declaring scopes: \
             {declaring:?}",
        );
    }
}
