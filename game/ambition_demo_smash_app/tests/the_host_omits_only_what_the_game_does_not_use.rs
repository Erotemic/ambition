//! Smash's host composes the engine for a profile, and the profile may
//! leave out only what Smash's rooms declare they do not use.

use ambition_platformer2d::combat::scoped_rules::{ActiveRoom, DeclaredRules};
use ambition_platformer2d::platformer::capability::UsedCapabilities;
use ambition_platformer2d::runtime::profile::CapabilityComposition as _;

/// Each capability the profile omits is one the game's rooms (its mode's) do
/// not use, and the host installs none of them.
#[test]
fn the_host_omits_only_what_the_game_does_not_use() {
    let app = ambition_demo_smash_app::build_demo_app();
    let profile = ambition_demo_smash_app::ENGINE_PROFILE;
    assert!(!profile.omits.is_empty(), "the premise: the `{}` profile leaves something out", profile.name);
    let used = app
        .world()
        .get_resource::<DeclaredRules<UsedCapabilities>>()
        .and_then(|rules| rules.governing(ActiveRoom::live(Some(ambition_demo_smash::SMASH_MODE))))
        .expect("Smash declares the capabilities its rooms use");
    for capability in profile.omits {
        assert!(
            !used.uses(*capability),
            "the `{}` profile leaves out `{}`, which Smash's rooms use",
            profile.name,
            capability.name()
        );
        assert!(!capability.is_installed(&app), "the host installed `{}`, which its profile leaves out", capability.name());
    }
}
