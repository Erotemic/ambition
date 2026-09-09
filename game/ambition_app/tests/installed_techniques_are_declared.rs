//! WHICH TECHNIQUES DOES THIS BUILD ACTUALLY INSTALL?
//!
//! ⛔⛔ **THE QUESTION HAD NO ANSWER, AND A MISSPELLED MOVE PAID FOR IT.** A
//! technique's key is a `pub const` in `ambition_characters`; its handler is a
//! system added by `combat_schedule`; nothing joined them. So an authored
//! `smash.teleprot` matched no handler's `key.as_str() != KEY` guard, fell out of
//! every consumer, and surfaced as a `warn!` in the middle of a fight — on a move
//! that plays and does nothing. `ParamSchemaRegistry` was supposed to catch that
//! and had ZERO production callers; `ambition_demo_smash`'s own source says so.
//!
//! ⭐ A11a's answer is that the statement installing the handler also declares
//! the key. This asks the BUILT APP whether that happened, because the
//! declaration is a line in a composition and a line can be deleted.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{Platformer2dSimHarness, TimestepMode};

/// ⛔⛔ **IT ASKS THE APP, NOT THE SOURCE.** A census that greps for
/// `install_technique` certifies the spelling of a call; only the built world
/// says the call ran and its declaration survived composition. This repository
/// has already been caught once by the difference — a guard that read an `impl`
/// passed with every production registration renamed away.
#[test]
fn the_shipped_composition_declares_the_techniques_it_installs() {
    use ambition_platformer2d::combat::technique::InstalledTechniques;

    let sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let installed = sim
        .world()
        .get_resource::<InstalledTechniques>()
        .expect(
            "no technique was declared at all, so the composition installs handlers \
             through some other road and nothing knows which keys exist",
        );
    let declared: Vec<&str> = installed.0.keys().collect();

    // The four the engine composition installs today. Named, because a COUNT
    // cannot say which one left — and each of these is a capability whose
    // handler exists in a different crate.
    for expected in [
        ambition_platformer2d::characters::smash_teleport::TELEPORT,
        ambition_platformer2d::characters::smash_vitality::VITALITY,
        ambition_platformer2d::characters::smash_trapdoor::TRAPDOOR,
        ambition_platformer2d::characters::smash_flyline::FLYLINE,
    ] {
        assert!(
            declared.contains(&expected),
            "'{expected}' has a handler installed by `combat_schedule` and no \
             declaration, so an authored move naming it — or misspelling it — is \
             admitted by every check in the tree and fails silently at runtime. \
             Declared: {declared:?}"
        );
    }
}

/// ⛔ A DECLARED TECHNIQUE ADMITS ITS OWN AUTHORED USE, and refuses a typo of it.
///
/// ⭐ THE SECOND HALF IS THE POINT. A support table that admits everything would
/// pass the first assertion alone, and admitting everything is exactly what the
/// registry this replaces did — "the engine matches no key, so an unregistered
/// key always passes", in its own doc.
#[test]
fn a_misspelled_technique_key_is_refused_by_the_shipped_composition() {
    use ambition_platformer2d::combat::technique::InstalledTechniques;
    use ambition_platformer2d::entity_catalog::{EffectRef, ParamValue};

    let sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let installed = &sim
        .world()
        .get_resource::<InstalledTechniques>()
        .expect("the composition declares its techniques")
        .0;

    let real = ambition_platformer2d::characters::smash_teleport::TELEPORT;
    let typo = "smash.teleprot";
    assert!(
        installed.offer(real).is_some(),
        "the fixture's premise is gone: '{real}' is not declared, so the refusal \
         below says nothing about typos"
    );
    let refused = installed.admit(&EffectRef {
        key: typo.to_string(),
        params: ParamValue::default(),
    });
    assert!(
        refused.is_err(),
        "'{typo}' was admitted. A key nothing installed declares reaches the \
         runtime, matches no handler, and the move plays and does nothing"
    );
}
