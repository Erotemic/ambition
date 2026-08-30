//! App-level developer presentation: debug overlays, inspectors, the F6 FPS
//! counter, and the F9 one-shot GGRS rollback proof. These are host/presentation systems with
//! no simulation-state ownership. The observatory's control resource is
//! platform-neutral so desktop keys and future Android developer UI can share
//! one proof-request seam.
pub mod debug_overlay;
pub mod fps_overlay;
pub mod gamepad_probe;
pub mod portal_inspector;
#[cfg(feature = "dev_tools")]
pub mod rollback_observatory;

use bevy::prelude::*;

/// The game's developer tooling, as one plugin (components-as-plugins):
/// the debug overlay + F6 FPS counter, plus (behind the `dev_tools`
/// feature) the egui resource/world inspectors. The dev STATE it drives
/// (`DeveloperTools`, the editable profiles) lives in the machinery lib
/// (`ambition_platformer2d::dev_tools::dev_tools`); this plugin only wires the
/// app-side presentation/inspection of it.
pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        // FPS overlay (ON by default on wasm, OFF on desktop; F6 toggles).
        app.add_plugins(fps_overlay::FpsOverlayPlugin);
        // What the stick actually reports, per pad, with a peak-hold. Shift+F6.
        app.add_plugins(gamepad_probe::GamepadProbePlugin);
        #[cfg(feature = "dev_tools")]
        app.add_plugins(rollback_observatory::RollbackObservatoryPlugin);
        install_egui_inspectors(app);
        install_debug_input_context(app);
    }
}

/// A developer typing into an inspector is not steering the character.
///
/// the condition is "egui WANTS the keyboard", not "the inspector is
/// visible". Watching values while playing is the normal way to use an
/// inspector; capturing input for the whole time a panel is up would break the
/// thing the panel is there to observe. egui already answers the narrower
/// question, and only while a text field actually holds focus.
///
/// The answer is one frame old (egui resolves focus in its own pass, after
/// this set). That is the correct frame anyway: focus is taken by a CLICK, so
/// the first keystroke a developer types is already inside a focused field.
#[cfg(feature = "dev_tools")]
fn install_debug_input_context(app: &mut App) {
    use ambition_platformer2d::input::participant::{
        context_priority, ContextClaim, InputParticipant, ParticipantContexts, DEBUG_CONTEXT,
    };
    use bevy_inspector_egui::bevy_egui::{EguiContext, PrimaryEguiContext};

    fn declare_debug_context(
        mut egui: Query<&mut EguiContext, With<PrimaryEguiContext>>,
        mut participants: Query<&mut ParticipantContexts, With<InputParticipant>>,
    ) {
        let typing = egui
            .iter_mut()
            .any(|mut context| context.get_mut().wants_keyboard_input());
        for mut contexts in &mut participants {
            // Touch the component only when the claim actually moves, so a
            // quiet frame is not a change-detection event downstream.
            if contexts.is_declared(DEBUG_CONTEXT) != typing {
                contexts.sync(
                    ContextClaim::capturing(DEBUG_CONTEXT, context_priority::DEBUG),
                    typing,
                );
            }
        }
    }

    app.add_systems(
        Update,
        declare_debug_context.in_set(ambition_platformer2d::input::InputSet::ResolveContext),
    );
}

#[cfg(not(feature = "dev_tools"))]
fn install_debug_input_context(_app: &mut App) {}

/// Install the egui inspector plugins. Gated by `dev_tools` so
/// shipping/headless builds don't pull `bevy-inspector-egui` /
/// `bevy_egui` into the dep graph; the quick plugins require
/// `EguiPlugin` first, hence the shared gate.
#[cfg(feature = "dev_tools")]
fn install_egui_inspectors(app: &mut App) {
    use ambition_platformer2d::combat::feel::Platformer2dFeelTuningMonolith;
    use ambition_platformer2d::dev_tools::dev_tools::{
        inspector_visible, world_inspector_visible, DeveloperTools, EditableAbilitySet,
        EditableMovementTuning, EditablePlayerStats,
    };
    use bevy_inspector_egui::bevy_egui::EguiPlugin;
    use bevy_inspector_egui::quick::{ResourceInspectorPlugin, WorldInspectorPlugin};

    // ⭐⭐ THE EGUI PASS ITSELF IS GATED, NOT JUST THE INSPECTORS INSIDE IT.
    // Tracy attribution on a headless Smash match, 2026-08-29: egui was the
    // ONLY named per-system cost in the whole frame —
    // `egui::Context::run` 87.6us/frame, plus begin_pass 40.8, end_pass 31.1
    // and the plugin hooks, all Tracy-inflated ~2.4x so roughly 36us real, ~1.2%
    // of a `profiling` frame. It ran EVERY FRAME with no window open and no
    // inspector on screen, because the `run_if(inspector_visible)` below gates
    // the inspector WIDGETS while `EguiPlugin` runs its context pass
    // unconditionally.
    //
    // ⛔ BOTH ENDS OR NEITHER. `EguiPreUpdateSet::BeginPass` opens a pass and
    // `EguiPostUpdateSet::EndPass` closes it; gating only one leaves egui with a
    // pass that is never closed. Gated together, no pass is begun at all while
    // every inspector is hidden, and the frame it is un-hidden begins and ends
    // normally.
    //
    // ⭐ Jon, 2026-08-29: *"developer tooling should be optimized too"* — the
    // inspector still works the instant it is shown; it simply stops running a
    // full egui pass to draw nothing.
    {
        use bevy_inspector_egui::bevy_egui::{EguiPostUpdateSet, EguiPreUpdateSet};
        let wanted = |tools: Option<Res<DeveloperTools>>| {
            tools.is_some_and(|tools| tools.inspector_visible || tools.world_inspector_visible)
        };
        app.configure_sets(
            bevy::app::PreUpdate,
            EguiPreUpdateSet::BeginPass.run_if(wanted),
        );
        // ⛔⛔ THREE SETS, NOT TWO — AND THE THIRD COST 28,353 ERRORS A RUN.
        // The first version of this gated `BeginPass` and `EndPass` only, having
        // reasoned that a pass must be begun and ended together. It must, but
        // `process_output_system` lives in a THIRD set, `ProcessOutput`, and it
        // CONSUMES what the pass produced: with the pass never begun it took the
        // `None` branch and logged `"bevy_egui pass output has not been prepared"`
        // once per frame. Headless there is no render app and nothing complained,
        // so the gate looked clean on the only path it was measured on; the first
        // windowed run produced one error per rendered frame.
        app.configure_sets(
            bevy::app::PostUpdate,
            (
                EguiPostUpdateSet::EndPass,
                EguiPostUpdateSet::ProcessOutput,
                EguiPostUpdateSet::PostProcessOutput,
            )
                .run_if(wanted),
        );
    }

    app.add_plugins(EguiPlugin::default())
        .add_plugins(ResourceInspectorPlugin::<DeveloperTools>::default().run_if(inspector_visible))
        .add_plugins(
            ResourceInspectorPlugin::<EditableAbilitySet>::default().run_if(inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditableMovementTuning>::default().run_if(inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditablePlayerStats>::default().run_if(inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<Platformer2dFeelTuningMonolith>::default()
                .run_if(inspector_visible),
        )
        .add_plugins(portal_inspector::PortalInspectorPlugin);

    app.add_plugins(WorldInspectorPlugin::new().run_if(world_inspector_visible));
}

#[cfg(not(feature = "dev_tools"))]
fn install_egui_inspectors(_app: &mut App) {}
