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

    // ⭐ EVERY TECHNIQUE THE SHIPPED COMPOSITION INSTALLS, engine and game
    // alike. Named, because a COUNT cannot say which one left.
    //
    // ⛔⛔ THE GAME'S HANDLERS USED TO DECLARE NOTHING. The first four are the
    // engine composition's; the rest live in `ambition_demo_smash` and were
    // bare `add_systems` calls, so "which techniques does this build install"
    // had no answer for the sixteen that matter most to a player. Until they
    // declared their keys, no strict unknown-key pass could be turned on: it
    // would have refused every authored use of them.
    for expected in [
        ambition_platformer2d::characters::smash_teleport::TELEPORT,
        ambition_platformer2d::characters::smash_vitality::VITALITY,
        ambition_platformer2d::characters::smash_trapdoor::TRAPDOOR,
        ambition_platformer2d::characters::smash_flyline::FLYLINE,
        ambition_platformer2d::characters::smash_sleep::SLEEP,
        ambition_platformer2d::characters::smash_portal::PORTAL_PAIR,
        ambition_platformer2d::characters::smash_bolt::STEERED_BOLT,
        ambition_platformer2d::characters::smash_homing::HOMING_DASH,
        ambition_platformer2d::characters::smash_riposte::RIPOSTE_STRIKE,
        ambition_platformer2d::characters::smash_tether::TETHER_PULL,
        ambition_platformer2d::characters::smash_spring::PLACE_SPRING,
        ambition_platformer2d::characters::smash_mine::PLACE_MINE,
        ambition_platformer2d::characters::smash_counter::COUNTER,
        ambition_platformer2d::characters::smash_ride::SUMMON_RIDE,
        ambition_platformer2d::characters::smash_bomb::DROP_BOMB,
        // One handler, four keys — the shape that needed `install_techniques`.
        ambition_platformer2d::characters::smash_capture::CAPTURE_ATTEMPT,
        ambition_platformer2d::characters::smash_capture::CAPTURE_CARRY,
        ambition_platformer2d::characters::smash_capture::CAPTURE_PUMMEL,
        ambition_platformer2d::characters::smash_capture::CAPTURE_THROW,
        // ⛔ THESE THREE WERE MISSED BY THE FIRST SWEEP, and the miss is worth
        // recording: their handlers test `if key != KEY` while the others write
        // `if key.as_str() != KEY`, so a grep keyed on one spelling found
        // fifteen of eighteen. `smash.mark_body` is an ON-HIT effect
        // (`hit.effect.key`), a different shape again — and it shares its
        // registration block with the mine, which is why that block declares
        // TWO keys through `install_techniques`.
        ambition_platformer2d::characters::smash_limit::FILL_METER,
        ambition_platformer2d::characters::smash_mark::MARK_BODY,
        ambition_platformer2d::characters::smash_time_dilation::TIME_DILATION,
        // ⛔⛔ THE ENGINE'S OWN TECHNIQUE, AND THE ONE THE CORPUS CAUGHT. 36
        // characters author `pogo_bounce` on `attack_air_down` and nothing
        // declared it — invisible to a `smash.`-scoped search, because it is not
        // a game key. Its two handlers live inside a `.chain()` in
        // `combat_schedule`, so the whole chained tuple goes through the install
        // seam rather than being lifted apart.
        ambition_platformer2d::characters::technique::POGO_BOUNCE_KEY,
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

/// ⛔⛔ **A KEY CAN BE DECLARED WHILE THE THING IT NAMES GOES UNCHECKED.** The
/// guard above proves the composition installs a handler for each key; it says
/// nothing about whether that declaration carries the NESTED REFERENCE its
/// params hold. Three technique params name another authored definition —
/// `SummonRideParams::character_id` and the two `item_id`s — and preparation
/// checked none of them until 2026-09-10. Forgetting to wire one is invisible to
/// every other guard in the tree: the key is declared, the params hydrate, and
/// the move still summons a character or drops an item nothing answers to.
///
/// ⭐ ASKED OF THE BUILT APP, like its neighbour, because a declaration is a line
/// in a composition and a line can be deleted.
#[test]
fn the_techniques_that_name_other_definitions_declare_that_they_do() {
    use ambition_platformer2d::combat::technique::{InstalledTechniques, NestedReferences};

    let sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let installed = &sim
        .world()
        .get_resource::<InstalledTechniques>()
        .expect("the composition declares its techniques")
        .0;

    for (key, expected) in [
        (
            ambition_platformer2d::characters::smash_ride::SUMMON_RIDE,
            "a character",
        ),
        (
            ambition_platformer2d::characters::smash_bomb::DROP_BOMB,
            "a held item",
        ),
        (
            ambition_platformer2d::characters::smash_mine::PLACE_MINE,
            "a held item",
        ),
    ] {
        let offer = installed
            .offer(key)
            .unwrap_or_else(|| panic!("'{key}' is not declared at all"));
        assert!(
            !matches!(offer.references, NestedReferences::None),
            "'{key}' names {expected} in its params and its declaration says it \
             references nothing, so preparation never resolves it — the move \
             plays and the thing it names does not exist"
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
