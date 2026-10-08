//! SUPPORTED PROFILES: each named composition constructs, steps a REAL BODY, and
//! does not install what it promises it does not.
//!
//! ⭐ Three claims per profile, and none is worth anything without the other two:
//!
//! 1. **it steps a subject** -- the sim tick advanced AND the primary body moved.
//!    `composes_through_the_sdk` already learned that a probe can certify only
//!    that the engine BUILDS (13 MB, 0.47 s, zero fixed steps); the body moving is
//!    the proof the sim ran over something real.
//! 2. **the promised absences are absent** -- `Capability::is_installed` is false
//!    for each omitted capability.
//! 3. **the probe can say yes** -- the CONTROL arm builds the full group and
//!    requires every capability to read installed. Without it, a probe that is
//!    always false makes every profile pass.
//!
//! Each witness prints `PROFILE-WITNESS <name> ...`; `scripts/check_engine_profiles.py`
//! runs this file under both feature sets and requires a line per profile.

use bevy::prelude::*;
use bevy::time::{Fixed, Time, TimeUpdateStrategy};

use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_runtime::profile::{
    Capability, EngineProfile, HostFace, SUPPORTED_PROFILES,
};
use ambition_platformer2d_runtime::{
    add_headless_foundation, PlatformerEnginePlugins, SimTick, SimulationHost,
};
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayer;

mod support;
use support::FixtureContentPlugin;

#[cfg(feature = "render")]
fn drawing_host_installed(app: &App) -> bool {
    app.is_plugin_added::<ambition_platformer2d_host::HostCameraPlugin>()
}
#[cfg(not(feature = "render"))]
fn drawing_host_installed(_app: &App) -> bool {
    false
}

fn build(profile: &EngineProfile) -> App {
    let mut app = App::new();
    add_headless_foundation(&mut app);
    app.add_plugins(PlatformerEnginePlugins::for_profile(SimulationHost::Fixed60Hz, profile));
    if profile.face == HostFace::Windowed {
        app.add_plugins(ambition_platformer2d_host::PlatformerHostPlugins);
    }
    app.add_plugins(FixtureContentPlugin);
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(timestep));
    app
}

fn body_position(app: &mut App) -> Option<Vec2> {
    app.world_mut()
        .query_filtered::<&BodyKinematics, With<PrimaryPlayer>>()
        .iter(app.world())
        .next()
        .map(|body| body.pos)
}

/// Step until the body exists, then step a second and require the body to move.
fn step_a_real_subject(app: &mut App, profile: &EngineProfile) -> u64 {
    app.update(); // Startup builds the world and the body.
    let mut settled = None;
    for _ in 0..30 {
        app.update();
        if let Some(pos) = body_position(app) {
            settled = Some(pos);
            break;
        }
    }
    let before = settled.unwrap_or_else(|| {
        panic!("{}: no primary body appeared, so nothing real was stepped", profile.name)
    });
    let tick_before = app.world().resource::<SimTick>().get();
    for _ in 0..60 {
        app.update();
    }
    let ticks = app.world().resource::<SimTick>().get() - tick_before;
    assert_eq!(ticks, 60, "{}: sixty frames at the tick dt must expend sixty ticks", profile.name);
    let after = body_position(app).expect("the body is gone after stepping");
    assert!(
        after != before,
        "{}: the primary body did not move in a second ({before:?} -> {after:?}); \
         the schedule ran over a body that nothing stepped",
        profile.name
    );
    ticks
}

/// THE CONTROL: the full group installs every capability, so `is_installed` can say yes.
#[test]
fn the_control_installs_every_capability_and_the_probe_sees_it() {
    let full = EngineProfile {
        name: "control-full-engine",
        summary: "nothing omitted",
        face: HostFace::Headless,
        omits: &[],
    };
    let mut app = build(&full);
    step_a_real_subject(&mut app, &full);
    ambition_platformer2d_runtime::profile::session_edge_params_validate(app.world_mut())
        .expect("the full engine's session-edge parameters must validate");
    for capability in Capability::ALL {
        assert!(
            capability.is_installed(&app),
            "the full engine reads `{}` as not installed, so the probe cannot say yes and \
             every profile's absence proof is vacuous",
            capability.name()
        );
    }
    println!("PROFILE-WITNESS control-full-engine ok");
}

#[test]
fn every_supported_profile_steps_a_body_and_installs_none_of_what_it_omits() {
    for profile in SUPPORTED_PROFILES.iter() {
        if profile.face == HostFace::Windowed && !cfg!(feature = "render") {
            // Not compiled in this feature set; the other run carries it. The
            // guard requires its witness line from the union of both runs.
            continue;
        }
        let mut app = build(profile);
        let ticks = step_a_real_subject(&mut app, profile);
        for capability in profile.omits {
            assert!(
                !capability.is_installed(&app),
                "{}: promises `{}` is not installed and it is",
                profile.name,
                capability.name()
            );
        }
        // The session edges: teardown and room transition take capability-owned
        // state as parameters and do not run while a fixture ticks.
        if let Err(refusal) = ambition_platformer2d_runtime::profile::session_edge_params_validate(app.world_mut()) {
            panic!("{}: {refusal}", profile.name);
        }
        // The kept capabilities are still there: an omission list that took out
        // everything would pass the line above.
        for capability in Capability::ALL {
            if !profile.omits.contains(&capability) {
                assert!(
                    capability.is_installed(&app),
                    "{}: `{}` is not omitted by this profile and is not installed",
                    profile.name,
                    capability.name()
                );
            }
        }
        match profile.face {
            HostFace::Windowed => assert!(drawing_host_installed(&app), "{}: no drawing host", profile.name),
            HostFace::Headless => assert!(!drawing_host_installed(&app), "{}: a headless profile installed the drawing host", profile.name),
        }
        println!("PROFILE-WITNESS {} ok ticks={ticks} omits={}", profile.name, profile.omits.len());
    }
}

/// ⭐ A PROFILE REFUSES CONTENT THAT NEEDS WHAT IT OMITS (A9, Q100). For each
/// supported profile and each content capability it omits, a pack that
/// requires that capability is refused at admission with `MissingCapability`
/// against the profile's schemas. The control: the full engine schemas admit
/// the same pack. And the ids the runtime spells are the owners' constants.
#[test]
fn a_profile_refuses_content_that_needs_a_capability_it_omits() {
    use ambition_content_pack::{
        CapabilityId, ContentPackDraft, ContentPackManifest, DiagnosticCode, ModuleNamespace, NoAssets,
        PackId, PackVersion,
    };
    assert_eq!(
        (Capability::Cutscenes.content_capability(), Capability::BossEncounters.content_capability()),
        (
            Some(ambition_cutscene::content_schema::CUTSCENE_CAPABILITY),
            Some(ambition_boss_encounter::pattern::content_schema::BOSS_PATTERN_CAPABILITY),
        ),
        "the runtime's content capability ids are the owners' constants"
    );
    let needing = |capability: &str| {
        ContentPackDraft::from_sources(
            ContentPackManifest {
                id: PackId("needs_one".into()),
                version: PackVersion("1.0.0".into()),
                namespace: ModuleNamespace("profile_test".into()),
                requires: vec![CapabilityId::new(capability)],
                sources: Vec::new(),
            },
            Vec::new(),
        )
        .expect("a manifest with no sources drafts")
    };
    let mut refused = Vec::new();
    for profile in SUPPORTED_PROFILES {
        let schemas = ambition_engine_schemas::engine_schemas_without(&profile.omitted_content_capabilities());
        for capability in profile.omitted_content_capabilities() {
            let draft = needing(capability);
            let under_profile = ambition_content_pack::compile(&draft, &schemas, &NoAssets);
            let under_full = ambition_content_pack::compile(&draft, &ambition_engine_schemas::engine_schemas(), &NoAssets);
            assert!(under_full.is_ok(), "control: the full engine admits a pack that needs `{capability}`");
            if under_profile.is_err_and(|failure| failure.has(DiagnosticCode::MissingCapability)) {
                refused.push(format!("{}: {capability}", profile.name));
            }
        }
    }
    println!("PROFILE-ADMISSION refused {refused:?}");
    let omitting: Vec<String> = SUPPORTED_PROFILES
        .iter()
        .flat_map(|profile| {
            profile
                .omitted_content_capabilities()
                .into_iter()
                .map(move |capability| format!("{}: {capability}", profile.name))
        })
        .collect();
    assert!(!omitting.is_empty(), "precondition: some profile omits a content capability");
    assert_eq!(refused, omitting, "the (profile, omitted capability) pairs whose pack was refused");
}

/// Where the primary body is after `ticks` fixed ticks of a session that
/// `profile` builds, counted from the tick the body first exists.
fn body_after(profile: &EngineProfile, ticks: usize) -> Vec2 {
    let mut app = build(profile);
    app.update();
    for _ in 0..30 {
        app.update();
        if body_position(&mut app).is_some() {
            break;
        }
    }
    for _ in 0..ticks {
        app.update();
    }
    body_position(&mut app).unwrap_or_else(|| panic!("{}: no primary body", profile.name))
}

/// ⭐ RE-ENTRY (A9): A SECOND SESSION IN ONE PROCESS STEPS AS THE FIRST. Each
/// headless profile builds a session, steps it and drops it, then builds a
/// second one and steps it the same way: the body must be at the same
/// position, bit for bit. A process-global value that the first session
/// leaves behind (a `OnceLock`, a static cache) shows here as a difference.
///
/// The control: one more tick moves the body, so the comparison can say no.
#[test]
fn a_second_session_in_one_process_steps_as_the_first() {
    let mut compared = Vec::new();
    for profile in SUPPORTED_PROFILES.iter().filter(|profile| profile.face == HostFace::Headless) {
        // Eight ticks: the body is still falling, so a tick moves it.
        let first = body_after(profile, 8);
        let second = body_after(profile, 8);
        assert_eq!(
            (first.x.to_bits(), first.y.to_bits()),
            (second.x.to_bits(), second.y.to_bits()),
            "{}: the second session's body is at {second:?}, the first's at {first:?}",
            profile.name
        );
        let later = body_after(profile, 9);
        assert_ne!(
            first, later,
            "{}: control: one more tick did not move the body, so equal positions prove nothing",
            profile.name
        );
        compared.push(profile.name);
    }
    println!("PROFILE-REENTRY ok {compared:?}");
    assert!(!compared.is_empty(), "precondition: some supported profile is headless");
}
