//! App-level developer presentation: debug overlays, inspectors, the F6 FPS
//! counter, and the F9 one-shot GGRS rollback proof. These are host/presentation systems with
//! no simulation-state ownership. The observatory's control resource is
//! platform-neutral so desktop keys and future Android developer UI can share
//! one proof-request seam.
pub mod debug_overlay;
pub mod fps_overlay;
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
        #[cfg(feature = "dev_tools")]
        app.add_plugins(rollback_observatory::RollbackObservatoryPlugin);
        install_egui_inspectors(app);
        install_debug_input_context(app);
    }
}

/// **A developer typing into an inspector is not steering the character.**
///
/// egui receives the same key presses leafwing does, so editing a tuning field
/// with the inspector open also drove the actor: typing a jump height walked
/// the player off the ledge being measured. `DEBUG_CONTEXT` has existed for
/// this since the claim system landed — priority 195, above every in-session
/// surface, *"because a developer reaching for the inspector means it"* — and
/// nothing declared it.
///
/// ⚠ **the condition is "egui WANTS the keyboard", not "the inspector is
/// visible".** Watching values while playing is the normal way to use an
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
            ResourceInspectorPlugin::<Platformer2dFeelTuningMonolith>::default().run_if(inspector_visible),
        )
        .add_plugins(portal_inspector::PortalInspectorPlugin);

    app.add_plugins(WorldInspectorPlugin::new().run_if(world_inspector_visible));
}

#[cfg(not(feature = "dev_tools"))]
fn install_egui_inspectors(_app: &mut App) {}
