#![cfg(feature = "input")]
//! A resource published by multiple experiences may not be released by type.
//! `SoleRemoval` asserts one publisher; the composed host can falsify that claim when
//! multiple experience scopes declare removal of the same resource.

use std::collections::BTreeMap;

use ambition_platformer2d::game_shell::{ReleaseKind, ShellExperienceScopes};
use bevy::prelude::*;

/// Compose the shipped multi-game host; scope declarations are installed at build time.
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

/// Resource removals in the composed host, grouped with their declaring owners.
/// Resets are excluded because resetting a shared value is not a sole-publisher claim.
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

/// A resource removed by multiple experiences must use owner-aware removal, not `SoleRemoval`.
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

/// Guard the ownership test against a vacuous composition with fewer than two match experiences.
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
