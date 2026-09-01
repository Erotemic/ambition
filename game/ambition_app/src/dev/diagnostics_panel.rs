//! The F1 numeric panel, on Bevy's diagnostics overlay.
//!
//! ⭐ ONE MEASUREMENT, MANY CONSUMERS. Nothing here counts anything. Every value
//! shown is a `DiagnosticPath` published by the subsystem that already knew the
//! fact — `ambition_dev_tools::runtime_census` for the ECS populations,
//! `ambition_render::runtime_census` for the camera populations, Bevy itself for
//! frame timing and render-pass spans — so the panel, the periodic `[census]`
//! log and any future report all read the SAME number. A panel that computed
//! its own would be a second answer with the same name.
//!
//! ⛔ AN ABSENT NUMBER IS NEVER A ZERO HERE, and there are TWO different ways a
//! number can be absent. They look different on purpose:
//!
//! - **A row that is SELECTED but not yet sampled reads `Missing` / `No sample`.**
//!   Bevy's overlay says so rather than showing a zero, which is the distinction
//!   this repository insists on elsewhere: a zero from an instrument that never
//!   reported is not a measurement.
//! - **A capability this composition does not install has NO ROW AT ALL.** The
//!   host CPU/memory pair is `#[cfg(feature = "desktop_platform")]`, so off
//!   desktop it is not compiled in; the render-pass timings are discovered from
//!   the store, so outside profiling mode there is nothing to discover. A
//!   permanent `Missing` row for something this build can never report is a
//!   standing accusation against a working game.
//!
//! See [`AmbitionDiagnosticsPanelPlugin`] and [`render_pass_rows`].

use bevy::dev_tools::diagnostics_overlay::{
    DiagnosticsOverlay, DiagnosticsOverlayItem, DiagnosticsOverlayPlugin,
    DiagnosticsOverlayStatistic,
};
use bevy::diagnostic::{DiagnosticPath, DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use std::time::Duration;

// ⭐ THROUGH THE RE-EXPORTS, like every other app-side reader. `ambition_app`
// does not name `ambition_dev_tools` or `ambition_render` as its own
// dependencies; it reaches them as `ambition_platformer2d::{dev_tools, render}`,
// which is the seam that decides what the app is allowed to see.
use ambition_platformer2d::dev_tools::runtime_census::{BODIES, RESOURCE_ENTITIES, SCENE_ENTITIES};
use ambition_platformer2d::dev_tools::DeveloperRuntimeState;
use ambition_platformer2d::render::runtime_census::{CAMERAS, OFFSCREEN_TARGETS, WORLD_DRAWS};

/// Marks the window this module owns, so F1 can retire exactly it.
#[derive(Component)]
struct AmbitionDiagnosticsWindow;

/// F1's numeric surface.
///
/// ⭐ SYSTEM INFORMATION IS DESKTOP-ONLY, AND DELIBERATELY SO. Bevy's
/// `SystemInformationDiagnosticsPlugin` rides `bevy/sysinfo_plugin`, which
/// `default_platform` carries and which Ambition's `android_platform` and
/// `web_platform` feature sets EXCLUDE on purpose. So CPU and memory appear on
/// desktop and are ABSENT elsewhere — the rows are `#[cfg]`-ed out of
/// `panel_items`, not shown as `Missing` — because a platform that can never
/// report them has no unanswered question to display. No substitute is
/// synthesized either way.
pub struct AmbitionDiagnosticsPanelPlugin;

impl Plugin for AmbitionDiagnosticsPanelPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        #[cfg(feature = "desktop_platform")]
        if !app.is_plugin_added::<bevy::diagnostic::SystemInformationDiagnosticsPlugin>() {
            app.add_plugins(bevy::diagnostic::SystemInformationDiagnosticsPlugin);
        }
        app.add_plugins(DiagnosticsOverlayPlugin)
            .add_plugins(ambition_platformer2d::dev_tools::runtime_census::EcsDiagnosticsPlugin)
            .add_plugins(
                ambition_platformer2d::render::runtime_census::RenderDiagnosticsPublishPlugin,
            )
            .add_systems(
                Update,
                (
                    follow_the_debug_toggle,
                    // ⛔ THE ROW SET IS NOT KNOWN WHEN THE PANEL OPENS. See
                    // `keep_the_render_rows_current`.
                    keep_the_render_rows_current
                        .run_if(on_timer(RENDER_ROW_RECONCILE_PERIOD)),
                ),
            );
    }
}

/// Show the panel while F1's debug mode is on, and take it away when it is off.
///
/// ⛔ KEYED TO `DeveloperRuntimeState.debug`, WHICH IS NOW A HOST FACT. Until
/// this campaign's A2 that flag could not be toggled without a live session, so
/// a panel bound to it would have been unreachable from the launcher — the exact
/// place a "why is the title screen slow" question gets asked.
fn follow_the_debug_toggle(
    mut commands: Commands,
    dev_state: Res<DeveloperRuntimeState>,
    store: Res<DiagnosticsStore>,
    windows: Query<Entity, With<AmbitionDiagnosticsWindow>>,
) {
    if !dev_state.is_changed() {
        return;
    }
    let showing = !windows.is_empty();
    if dev_state.debug == showing {
        return;
    }
    if dev_state.debug {
        commands.spawn((
            AmbitionDiagnosticsWindow,
            DiagnosticsOverlay::new("Ambition", panel_items(&store)),
        ));
    } else {
        for window in &windows {
            commands.entity(window).despawn();
        }
    }
}

/// How often an open panel re-asks the store which render passes exist.
///
/// ⛔⛔ BEVY REGISTERS RENDER DIAGNOSTIC PATHS LAZILY, AS PASSES RUN. Its
/// `sync_diagnostics` is literally `if store.get(&path).is_none() {
/// store.add(..) }` — so the set of `render/**/elapsed_*` paths GROWS during a
/// session as rendering roads are first taken. A 2026-08-31 bundle records the
/// growth directly: `render_diagnostics_status.csv` goes `cpu_spans` 0 → 2 → 4
/// over the first three seconds.
///
/// ⇒ a panel that snapshots the row set when F1 opens is permanently missing
/// every pass that had not run yet, and the only way to see them is to toggle
/// F1 off and on. That is a stale instrument, and a stale instrument is the
/// thing this whole campaign exists to stop shipping.
///
/// ⭐ ONE SECOND, matching Bevy's own overlay refresh: reconciling faster than
/// the panel redraws buys nothing a reader can see.
const RENDER_ROW_RECONCILE_PERIOD: Duration = Duration::from_secs(1);

/// Add render-pass rows to an OPEN panel as their passes first run.
///
/// ⭐ READ, COMPARE, THEN WRITE. `Mut<T>` marks the component changed on
/// `deref_mut`, and Bevy's overlay rebuilds from `items`, so taking `&mut` every
/// second whether or not anything moved would rebuild every row of the panel on
/// a timer forever. The comparison below is on an immutable deref; the write
/// happens only when the set actually differs.
fn keep_the_render_rows_current(
    store: Res<DiagnosticsStore>,
    mut panels: Query<&mut DiagnosticsOverlay, With<AmbitionDiagnosticsWindow>>,
) {
    let wanted = render_pass_rows(&store);
    for mut panel in &mut panels {
        let showing: Vec<&DiagnosticPath> = panel
            .items
            .iter()
            .map(|item| &item.path)
            .filter(|path| is_render_timing(path))
            .collect();
        if showing.len() == wanted.len()
            && showing
                .iter()
                .zip(wanted.iter())
                .all(|(shown, want)| **shown == want.path)
        {
            continue;
        }
        // Replace the whole render block rather than appending, so a row keeps
        // its place in the sorted order however late its pass first ran.
        panel.items.retain(|item| !is_render_timing(&item.path));
        panel.items.extend(wanted.iter().cloned());
        break;
    }
}

/// Everything F1 shows, in the order it shows it.
///
/// ⛔ ONE WINDOW, NOT THREE. Until 2026-08-31 this spawned three
/// `DiagnosticsOverlay` entities — Frame, Ambition, Host. Bevy's `build_overlay`
/// observer gives EVERY overlay the same initial `top`/`left`, and nothing
/// staggers them, so all three landed exactly on top of each other: F1 showed
/// one panel with two hidden underneath, findable only by dragging the top one
/// off them. Staggering would also have worked; one window for ten numbers is
/// the better shape, and each row is labelled with its full diagnostic path, so
/// the prefix already says which subsystem answered.
///
/// ⭐ SEPARATE FROM THE SPAWN so the guard test below can assert on the SAME
/// list the panel shows. A test that rebuilt the list itself would agree with
/// whatever it was written against, not with what F1 renders.
fn panel_items(store: &DiagnosticsStore) -> Vec<DiagnosticsOverlayItem> {
    let mut items = vec![
        FrameTimeDiagnosticsPlugin::FPS.into(),
        FrameTimeDiagnosticsPlugin::FRAME_TIME.into(),
        // ⭐ TWO ENTITY NUMBERS, NAMED. One row called "entities" would carry
        // Bevy 0.19's resources-are-entities ambiguity into every note anyone
        // takes from this panel.
        count_of(SCENE_ENTITIES),
        count_of(RESOURCE_ENTITIES),
        count_of(BODIES),
        count_of(CAMERAS),
        count_of(WORLD_DRAWS),
        count_of(OFFSCREEN_TARGETS),
    ];
    #[cfg(feature = "desktop_platform")]
    items.extend([
        bevy::diagnostic::SystemInformationDiagnosticsPlugin::PROCESS_CPU_USAGE.into(),
        bevy::diagnostic::SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE.into(),
    ]);
    items.extend(render_pass_rows(store));
    items
}

/// The render-pass timings, when this run has any.
///
/// ⭐ DISCOVERED, NOT HAND-LISTED. The pass names are Bevy render-graph node
/// names — a 2026-08-31 profile bundle recorded `main_transparent_pass_2d`,
/// `ui`, `msaa_writeback` and `upscaling` on this composition — and a hand-kept
/// list of those goes stale the moment a pass is renamed or added, silently, as
/// a row that reads `Missing` forever. Reading the store instead shows exactly
/// the passes THIS build measures.
///
/// ⛔ AN ORDINARY RUN SHOWS NONE OF THESE, AND THAT IS THE HONEST ANSWER. Bevy's
/// `RenderDiagnosticsPlugin` is what registers them, and Ambition installs it
/// only under `AMBITION_PROFILE_CENSUS`: it adds GPU timestamp and
/// pipeline-statistics queries to every pass, which a dev overlay has no
/// business imposing on a normal session. The campaign's A5 asked "which render
/// pass is expensive" — this is where F1 answers it when the measurement exists,
/// and `render_diagnostics.csv` in a profile bundle is the fuller answer.
/// Whether a path is one of the render-pass TIMINGS.
///
/// ⭐ ONE PREDICATE, TWO CALLERS: the initial discovery and the reconciliation
/// above. Two copies of this rule is how a row gets added by one and stripped by
/// the other on the next tick, forever.
///
/// ⛔ TIMINGS ONLY. `RenderDiagnosticsPlugin` also publishes pipeline statistics
/// (`vertex_shader_invocations` and friends) under the same `render/` prefix,
/// and "which pass is expensive" is not what those answer.
fn is_render_timing(path: &DiagnosticPath) -> bool {
    let path = path.as_str();
    path.starts_with("render/")
        && (path.ends_with("/elapsed_gpu") || path.ends_with("/elapsed_cpu"))
}

fn render_pass_rows(store: &DiagnosticsStore) -> Vec<DiagnosticsOverlayItem> {
    let mut paths: Vec<DiagnosticPath> = store
        .iter()
        .map(|diagnostic| diagnostic.path())
        .filter(|path| is_render_timing(path))
        .cloned()
        .collect();
    // ⛔ THE STORE'S ITERATION ORDER IS A HASH MAP'S. Sorting keeps a row in the
    // same place between two runs of the same build, which is what makes the
    // panel readable at a glance rather than a shuffle to re-scan every launch.
    paths.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    paths.into_iter().map(Into::into).collect()
}

/// A POPULATION row: the latest count, with no decimal point.
///
/// ⛔⛔ `DiagnosticPath::into()` IS WRONG FOR A COUNT, AND SILENTLY SO. Bevy's
/// `From<DiagnosticPath>` picks `Smoothed` — an exponential moving average — at
/// four decimal places. That is right for FPS and frame time and nonsense for a
/// population: after two bodies despawn the panel reads `7.3842 bodies` and
/// keeps lagging the truth for as long as the EMA takes to settle.
///
/// ⛔ THE PUBLISHER'S OWN TEST CANNOT CATCH THIS. `runtime_census`'s test reads
/// `Diagnostic::value()` and proves the published number is right; the panel
/// then renders a DIFFERENT statistic of the same diagnostic. The value being
/// correct and the display being wrong are compatible, which is why the guard
/// for this lives here, next to the choice.
fn count_of(path: DiagnosticPath) -> DiagnosticsOverlayItem {
    DiagnosticsOverlayItem {
        path,
        statistic: DiagnosticsOverlayStatistic::Value,
        precision: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every POPULATION row reads the latest count, not a smoothed average.
    ///
    /// ⛔ THIS IS THE ARM THE PUBLISHER'S TEST CANNOT PROVIDE. `runtime_census`
    /// proves `Diagnostic::value()` is the true population; this proves the
    /// panel asks for `value()` rather than `smoothed()`. Reverting any
    /// `count_of` here to a bare `.into()` makes this red — which is the whole
    /// point, since `.into()` compiles and renders a plausible-looking number.
    #[test]
    fn the_population_rows_show_the_count_and_not_a_smoothed_average() {
        let counts = [
            SCENE_ENTITIES,
            RESOURCE_ENTITIES,
            BODIES,
            CAMERAS,
            WORLD_DRAWS,
            OFFSCREEN_TARGETS,
        ];
        let items = panel_items(&DiagnosticsStore::default());
        for path in counts {
            let item = items
                .iter()
                .find(|item| item.path == path)
                .unwrap_or_else(|| panic!("{path} must be on the panel at all"));
            assert_eq!(
                item.statistic,
                DiagnosticsOverlayStatistic::Value,
                "{path} is a population: an EMA of it lags the truth and prints \
                 fractional entities"
            );
            assert_eq!(item.precision, 0, "{path} counts whole things");
        }
    }

    /// Timing rows keep the smoothing; they are the reason it exists.
    ///
    /// Premise guard: without this, the test above would still pass if someone
    /// "fixed" the panel by making EVERY row a raw value, which would make the
    /// FPS readout flicker with every frame's noise.
    #[test]
    fn the_frame_timing_rows_stay_smoothed() {
        let items = panel_items(&DiagnosticsStore::default());
        for path in [
            FrameTimeDiagnosticsPlugin::FPS,
            FrameTimeDiagnosticsPlugin::FRAME_TIME,
        ] {
            let item = items
                .iter()
                .find(|item| item.path == path)
                .unwrap_or_else(|| panic!("{path} must be on the panel at all"));
            assert_eq!(
                item.statistic,
                DiagnosticsOverlayStatistic::Smoothed,
                "{path} is a timing signal and wants the EMA"
            );
        }
    }

    /// F1 opens exactly ONE window.
    ///
    /// ⛔ THE DEFECT THIS PINS SHIPPED AND WAS INVISIBLE. Three overlays were
    /// spawned, and Bevy's `build_overlay` observer gives each the same initial
    /// `top`/`left` — so they stacked perfectly and F1 looked like it worked.
    /// Nothing in the panel's own tests could tell one window from three,
    /// because they were all about CONTENT.
    #[test]
    fn opening_the_panel_spawns_one_window() {
        let mut app = App::new();
        app.init_resource::<DiagnosticsStore>();
        app.insert_resource(DeveloperRuntimeState {
            debug: true,
            ..default()
        });
        app.add_systems(Update, follow_the_debug_toggle);
        app.update();

        let mut windows = app
            .world_mut()
            .query_filtered::<Entity, With<AmbitionDiagnosticsWindow>>();
        assert_eq!(
            windows.iter(app.world()).count(),
            1,
            "two windows would land on top of each other and hide one another"
        );
    }

    /// A pass that first runs AFTER F1 opened still reaches the open panel.
    ///
    /// ⛔⛔ THE DEFECT THIS PINS IS A STALE INSTRUMENT. Bevy's
    /// `sync_diagnostics` registers a render path the first time its pass runs,
    /// so the row set grows during a session. The panel used to snapshot it at
    /// spawn, which meant every pass that had not yet run was invisible until
    /// somebody toggled F1 off and on — and nobody knows to do that, because the
    /// panel looks complete.
    ///
    /// The arms straddle the registration deliberately: absent before, present
    /// after, with NO despawn in between.
    #[test]
    fn a_render_pass_that_appears_later_reaches_the_open_panel() {
        fn rows(app: &mut App) -> Vec<String> {
            let mut panels = app
                .world_mut()
                .query_filtered::<&DiagnosticsOverlay, With<AmbitionDiagnosticsWindow>>();
            let panel = panels
                .iter(app.world())
                .next()
                .expect("the panel is open for the whole test");
            panel
                .items
                .iter()
                .map(|item| item.path.as_str().to_string())
                .collect()
        }

        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.init_resource::<DiagnosticsStore>();
        app.insert_resource(DeveloperRuntimeState {
            debug: true,
            ..default()
        });
        app.add_systems(
            Update,
            (follow_the_debug_toggle, keep_the_render_rows_current).chain(),
        );
        // One reconcile period per update, so every update reconciles. The first
        // update advances the clock by zero, which is why the panel is opened by
        // it and the assertions below start from the second.
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            RENDER_ROW_RECONCILE_PERIOD,
        ));

        app.update();
        let opened = rows(&mut app);
        assert!(
            !opened.iter().any(|row| row.starts_with("render/")),
            "premise: the panel must open with no render row, or this test \
             cannot tell a late row from an early one: {opened:?}"
        );

        // The pass runs for the first time; Bevy registers its path.
        app.world_mut()
            .resource_mut::<DiagnosticsStore>()
            .add(bevy::diagnostic::Diagnostic::new(DiagnosticPath::new(
                "render/main_transparent_pass_2d/elapsed_cpu",
            )));

        app.update();
        assert!(
            rows(&mut app)
                .iter()
                .any(|row| row == "render/main_transparent_pass_2d/elapsed_cpu"),
            "the row must arrive without an F1 toggle; got {:?}",
            rows(&mut app)
        );

        // ⭐ AND THE PANEL IS STILL THE SAME ONE. Reconciling by despawning and
        // respawning would pass the assertion above while throwing away wherever
        // the developer had dragged the window to.
        let mut panels = app
            .world_mut()
            .query_filtered::<Entity, With<AmbitionDiagnosticsWindow>>();
        assert_eq!(
            panels.iter(app.world()).count(),
            1,
            "one window throughout, not a replacement"
        );
    }

    /// Render-pass rows appear only when this build MEASURES render passes.
    ///
    /// ⛔ THE CAMPAIGN'S A5 WAS MARKED DONE ON THE STRENGTH OF THE PATHS
    /// EXISTING UPSTREAM. They do — but Ambition registers them only under
    /// `AMBITION_PROFILE_CENSUS`, so ordinary F1 selected none and answered
    /// none. The two arms here are the two states that fact has.
    #[test]
    fn render_pass_rows_follow_whether_the_run_measures_render_passes() {
        assert!(
            render_pass_rows(&DiagnosticsStore::default()).is_empty(),
            "premise: with nothing registered there is nothing to show, and a \
             row that always reads Missing is worse than no row"
        );

        // The four passes a 2026-08-31 desktop bundle actually recorded, given
        // to the store out of order.
        let mut store = DiagnosticsStore::default();
        for path in [
            "render/ui/elapsed_gpu",
            "render/main_transparent_pass_2d/elapsed_gpu",
            "render/ui/vertex_shader_invocations",
            "render/upscaling/elapsed_gpu",
        ] {
            store.add(bevy::diagnostic::Diagnostic::new(DiagnosticPath::new(path)));
        }
        let shown: Vec<String> = render_pass_rows(&store)
            .iter()
            .map(|item| item.path.as_str().to_string())
            .collect();
        assert_eq!(
            shown,
            vec![
                "render/main_transparent_pass_2d/elapsed_gpu".to_string(),
                "render/ui/elapsed_gpu".to_string(),
                "render/upscaling/elapsed_gpu".to_string(),
            ],
            "the timing paths, sorted; `vertex_shader_invocations` is a pipeline \
             statistic and not what 'which pass is expensive' asks"
        );
    }
}
