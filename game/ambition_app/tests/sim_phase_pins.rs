//! **Where `CoreSimulation` actually is, measured rather than assumed.**
//!
//! `Platformer2dSimulationPhaseMonolith::CoreSimulation` is configured in
//! `app.sim_schedule()`, which is a HOST CHOICE: `Update` for
//! `SimulationHost::RenderFrame` (the default), `FixedUpdate` for `Fixed60Hz`,
//! `GgrsSchedule` for `Ggrs`. So `.before(CoreSimulation)` means different things
//! depending on which schedule the pinning system was added to:
//!
//! * added to `app.sim_schedule()` — real in every host, by construction;
//! * added to a LITERAL schedule — real only in the host whose sim schedule
//!   happens to be that one. Elsewhere Bevy silently creates an empty node and
//!   the pin constrains nothing.
//!
//! ⛔ **and the frame order does not rescue a `.before` the way it rescues an
//! `.after`.** A `.after(CoreSimulation)` pinned in `Update` is usually true
//! anyway under a fixed-tick or rollback host, because `PreUpdate` → fixed →
//! `Update` already ran the sim. A `.before` in the same position is the
//! opposite: the sim has ALREADY run by the time `Update` starts, so the pin is
//! not merely unenforced, it is the wrong way round.
//!
//! ## Why this is a test and not a lint
//!
//! The GPT review of `5cc4337..47d7de3` (finding #11) proposed sweeping the
//! `.before(CoreSimulation)` pins out. A blind sweep deletes the ones that are
//! load-bearing in a `RenderFrame` host — which is the default host, and the one
//! every engine-side fixture and most of the demo shells run under. The pins are
//! not wrong; they are conditional, and nothing said out loud which condition.
//!
//! ⭐ **and the conditional is sharper than "some host somewhere" — measured
//! 2026-08-03.** The shipped desktop app is a GGRS host only because
//! `build_visible_app` sets it inside `#[cfg(feature = "dev_tools")]` and
//! `dev_tools` is in the default feature set. **`run_web` never sets a host at
//! all**, so the browser build resolves to the render-frame default. The
//! `.before(CoreSimulation)` pins in literal `Update` are therefore not a
//! hypothetical host's concern — they are **the web build's**, and sweeping them
//! would break the one composition that still needs them.
//!
//! This says it out loud, for the composition Jon actually runs, in a form that
//! FAILS when the answer changes. If the shipped app ever moves its sim into
//! `Update`, the literal-`Update` pins become load-bearing that same day and this
//! test is what reports it.
//!
//! ## The census that motivated it (2026-08-03, B7)
//!
//! Nine non-test `.before(…CoreSimulation)` pins existed at the census; the
//! `sync_preset_input_map` row was deleted with its system on 2026-08-06 (the
//! preset resync is engine-owned now, in the host's `InputSet::Collect`
//! pipeline, which never names `CoreSimulation`). By schedule:
//!
//! | site | schedule | verdict |
//! |---|---|---|
//! | `platformer2d_runtime/portal_schedule.rs:39` (`PortalSet::Carves`) | `sim` | real everywhere |
//! | `platformer2d_runtime/lib.rs:269` (`clear_class_b_remap_log`) | `sim` | real everywhere |
//! | `platformer2d_runtime/lib.rs:283` (sim-id minting) | `sim` | real everywhere |
//! | `actor_monolith/gravity/plugin.rs:63` (`GravitySet::ZoneSnapshot`) | `sim` | real everywhere |
//! | `actor_monolith/gravity/plugin.rs:76` (`FrameResolveSet`) | `sim` | real everywhere |
//! | `platformer2d_runtime/rollback/session.rs:621` (`publish_ggrs_input`) | `GgrsSchedule` | real — that schedule only runs under the host where it IS the sim schedule |
//! | `platformer2d_host/lib.rs:371` (device control write) | literal `Update` | conditional, and ALREADY says so in a comment |
//! | `ambition_app/menu/grid_backend.rs:1141` (grid menu nav) | literal `Update` | conditional — annotated by B7 |
//!
//! So six of eight are unconditionally real, one is real by host exclusivity,
//! and the literal-`Update` pins are the whole of what #11 was pointing at —
//! none of which should be deleted, because `RenderFrame` is the default host.

use bevy::ecs::schedule::{ScheduleLabel, Schedules, SystemSet};
use bevy::prelude::*;

use ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith as Phase;
use ambition_platformer2d::runtime::rollback::GgrsSchedule;

/// How many systems the set owns in that schedule, or `None` when the set has no
/// node there at all.
///
/// ⚠ **a schedule that has never RUN reports no sets**, however many systems it
/// holds — Bevy builds the graph lazily on first run. The first draft of this
/// probe measured that laziness and reported `GgrsSchedule` as having 415 systems
/// and no `CoreSimulation`, which would have been a spectacular false finding.
/// Hence the explicit `initialize` below.
fn systems_in(app: &mut App, schedule: impl ScheduleLabel, set: impl SystemSet) -> Option<usize> {
    let label = schedule.intern();
    app.world_mut()
        .resource_scope(|world, mut schedules: Mut<Schedules>| {
            let built = schedules.get_mut(label)?;
            let _ = built.initialize(world);
            built
                .graph()
                .systems_in_set(set.intern())
                .ok()
                .map(|systems| systems.len())
        })
}

fn shipped_app() -> App {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // Enough frames that every deferred registration has landed.
    for _ in 0..4 {
        app.update();
    }
    app
}

/// ⭐ **The measurement.** In the shipped composition the sim lives in
/// `GgrsSchedule`, and the `CoreSimulation` node in `Update` is the empty husk
/// that the literal-`Update` pins created by naming it.
///
/// Both halves matter. "`Update` has zero" alone would also be true of an app
/// where the sim had failed to compose at all, so the sim schedule's count is
/// what makes the zero mean *the pins are decorative here* rather than *nothing
/// is running*.
#[test]
fn the_shipped_apps_core_simulation_is_in_the_ggrs_schedule_and_update_holds_an_empty_husk() {
    let mut app = shipped_app();

    let in_ggrs = systems_in(&mut app, GgrsSchedule, Phase::CoreSimulation)
        .expect("the shipped app hosts its sim in GgrsSchedule, so the set has a node there");
    assert!(
        in_ggrs > 100,
        "the whole core simulation should be in GgrsSchedule; found only {in_ggrs} systems, \
         which means the composition changed shape rather than that this pin moved"
    );

    let in_update = systems_in(&mut app, Update, Phase::CoreSimulation);
    assert_eq!(
        in_update,
        Some(0),
        "`Update` should hold a CoreSimulation node with NO members — the husk that the \
         literal-`Update` `.before(CoreSimulation)` pins create by naming the set. Getting \
         `None` means even those pins are gone; getting a positive count means the sim moved \
         into `Update` and those pins JUST BECAME LOAD-BEARING — go read them, they were \
         written when they constrained nothing."
    );
}

/// The same fact from the other side, so neither statement can drift alone: the
/// sub-phases inside `CoreSimulation` are in the sim schedule too, and are not in
/// `Update` at all. A pin naming one of THOSE from `Update` would not even create
/// a husk to notice later.
#[test]
fn the_core_simulation_sub_phases_live_where_core_simulation_does() {
    let mut app = shipped_app();

    for phase in [Phase::PlayerInput, Phase::WorldPrep, Phase::Combat] {
        let ggrs = systems_in(&mut app, GgrsSchedule, phase);
        assert!(
            ggrs.is_some_and(|count| count > 0),
            "{phase:?} should own systems in the sim schedule; found {ggrs:?}"
        );
        assert_eq!(
            systems_in(&mut app, Update, phase),
            None,
            "{phase:?} should have no node in `Update` at all — nothing pins against it from \
             there, so unlike `CoreSimulation` there is not even an empty husk"
        );
    }
}
