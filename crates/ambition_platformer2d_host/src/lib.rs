//! Windowed host composition for the platformer runtime.
//!
//! [`PlatformerHostPlugins`] adds device input, camera/presentation wiring, and
//! optional portal presentation on top of the content-free simulation runtime.
//! Headless/RL entry points omit this group. This crate may depend on rendering,
//! input, and runtime infrastructure but must not depend on game content or the
//! actor monolith.

// ⚠ `App`, `Plugin` and the prelude are used only by the plugins behind
// `render` and `input`. A host with neither is a legal composition — it is the
// A9 minimum profile — and an unconditional import there is a warning in exactly
// the configuration this crate's feature split exists to make buildable.
use bevy::app::{PluginGroup, PluginGroupBuilder};
#[cfg(any(feature = "render", feature = "input"))]
use bevy::app::{App, Plugin};
#[cfg(any(feature = "render", feature = "input"))]
use bevy::prelude::*;

pub mod gameplay_presentation;
#[cfg(feature = "portal_render")]
pub mod portal;

// Only the input bridge + portal continuity order against the sandbox phases.
#[cfg(any(feature = "input", feature = "portal_render"))]
use ambition_platformer2d_shared_tangle::schedule::{
    Platformer2dSimulationPhaseMonolith, SimScheduleExt as _,
};

/// The windowed-host plugin group (see the crate docs).
pub struct PlatformerHostPlugins;

impl PluginGroup for PlatformerHostPlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>()
            .add(ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperHotkeyPlugin);
        // ⛔ THE DRAWING HALF IS BEHIND `render`, and it is the whole reason
        // this crate's `ambition_render` edge could become optional: all three
        // of these plugins exist to wire presentation, so a host that installs
        // no renderer has nothing for them to wire. See the manifest's `render`
        // feature for the dependency path this closed.
        #[cfg(feature = "render")]
        let builder = builder
            .add(HostCameraPlugin)
            .add(HostProjectileVisualsPlugin)
            .add(HostVfxPresentationPlugin);
        #[cfg(feature = "input")]
        let builder = builder.add(HostInputBindingsPlugin);
        builder
    }
}

/// The leafwing-input-manager plugin, the player-input attach startup system, and the bridge
/// that keeps `Res<ControlFrame>` in sync with leafwing's `ActionState`.
#[cfg(feature = "input")]
pub struct HostInputBindingsPlugin;

#[cfg(feature = "input")]
impl Plugin for HostInputBindingsPlugin {
    fn build(&self, app: &mut App) {
        use ambition_input::{MenuControlFrame, MenuInputState, Platformer2dInputActionMonolith};
        use ambition_platformer2d_runtime::host_input::{
            apply_menu_frame_to_cutscene_request, commit_seat_raw_frames,
            declare_gameplay_input_context, declare_in_session_input_contexts,
            dialog_pointer_input, populate_menu_control_frame_from_actions,
            populate_seat_control_frames, populate_seat_menu_frames,
            spawn_primary_input_participant, sync_primary_recipe_from_settings,
            toggle_player_trail_emission_from_actions, MenuFrameConsume, MenuFrameCutsceneSkip,
            MenuFramePopulate, MenuNavConsume,
        };
        use leafwing_input_manager::prelude::InputManagerPlugin;

        // ⭐ THE INPUT CAPABILITY DECLARES ITS OWN PIPELINE. This host restated
        // `InputSet`'s six-phase chain and registered the two device-ownership systems
        // itself; both moved to `ambition_input::install_input_pipeline`, which is
        // where the sets and the systems live. The `configure_sets` went WITH them —
        // leaving it behind is the half that gets forgotten, and it fails silently by
        // turning "consumed this frame" into "may arrive one frame later".
        ambition_input::install_input_pipeline(app);
        ambition_input::install_provider_action_road(app);
        ambition_input::install_seat_device_tracking(app);
        ambition_input::install_rebind_edge_swallow(app);

        // See `ambition_input::seating`.
        app.init_resource::<ambition_input::SessionSeatingSource>();
        app.init_resource::<ambition_input::SeatInputContexts>();
        app.init_resource::<ambition_input::SeatBindings>();
        app.init_resource::<ambition_input::ActiveUiCues>();

        // ── The frame→tick input latch (netcode N0.1) ─────────────────────
        //
        // A device samples on the FEEL clock, once per rendered frame. When the
        // sim runs on the TICK clock the two diverge, and every device sample
        // between two ticks has to reach the sim as ONE control frame: axes
        // take the latest, press/release edges OR together so a sub-tick tap is
        // never swallowed and a single tap never fires twice.
        //
        // This lives in the DEVICE plugin, not the engine group: headless, RL, and replay
        // drivers have no device and author the per-tick `ControlFrame` themselves.
        // Frame-stepped hosts skip it too — one frame IS one tick, so there is nothing to
        // bridge. Both advance the simulation on a cadence of their own, so several rendered
        // frames can pass between ticks and a short press sampled in between must not be lost —
        // which is the whole job of these latches.
        //
        // Asked through `SimulationHost` rather than by naming `GgrsSchedule`,
        // because this crate must not depend on `bevy_ggrs` — the schedule
        // owner is optional and the host's vocabulary stays independent of it.
        let rollback_host = app
            .world()
            .get_resource::<ambition_platformer2d_runtime::SimulationHost>()
            .is_some_and(|host| host.is_rollback());
        // `SlotControlLatches` needs no system of its own:
        // `populate_seat_control_frames` folds into it whenever the resource
        // exists and writes `SlotControls` straight through when it does not, so
        // installing the resource IS the switch. Under rollback,
        // `capture_latched_local_input` drains it on the `ReadInputs` edge —
        // which is where a rollback host asks for input — and publishes each
        // seat into the session.
        //
        // Every word of that is true about DESKTOP-DEV and none of it is an ownership boundary:
        // `dev::rollback_observatory` is behind `dev_tools`, which the web persona does not enable.
        // So the browser composed a live GGRS session, live leafwing actions, and seat latches —
        // with no primary latch. `capture_latched_local_input` takes it as `Option` and leaves
        // `PendingLocalInput` alone when it is missing, so seat zero published a NEUTRAL frame
        // every tick, forever, in silence. Arrow keys navigated menus (those never enter the
        // session) and moved nothing.
        //
        //  A DEVELOPER INSTRUMENT MAY NEVER BE LOAD-BEARING FOR GAMEPLAY.
        // The device host owns the frame→tick bridge because the device host is
        // what HAS a device; removing an observatory from a visible composition
        // must not be able to remove input. The observatory's copy is deleted,
        // so this is the only registration and there is nothing to double.
        if app.sim_is_fixed_tick() || rollback_host {
            //  ONE table, seat zero included. There were two `init_resource`
            // calls here and two systems below, because seat zero had its own
            // spelling of the latch — see `SlotControlLatches`.
            app.init_resource::<ambition_platformer2d_runtime::host_input::SlotControlLatches>();
        }
        // Fold after proposal-side input shaping and before confirmed-input derivations.
        // Device- or wall-clock-derived shaping belongs in `InputSet::Route`; confirmed
        // input derivations belong in the simulation schedule.
        app.add_systems(
            Update,
            commit_seat_raw_frames
                .after(ambition_input::InputSet::Route)
                .in_set(ambition_platformer2d_runtime::host_input::PrimarySlotInputCommit),
        );
        //  ONE drain for every seat. This was two systems because their
        // destinations differed — seat zero's latched frame went to the global
        // `ControlFrame`, which the shapers only it had still read.
        // `SlotControls` is every seat's destination now, and the `ControlFrame`
        // mirror is registered once in `player_schedule`, where a composition
        // without this host still gets it.
        //
        // ⭐ THE DRAIN INSTALLS ITSELF, like the seating pair below and for the
        // same reason: the sim phase it sits in, the `InputSet::Route` edge it
        // must precede, and the fixed-tick condition (a variable-timestep host
        // has no tick edge to publish ON; `capture_latched_local_input` drains
        // the same latches at `ReadInputs`, where a rollback host asks) are all
        // facts about that module, not about this composition. This host spelled
        // all four and reached for `app.sim_schedule()` to do it.
        ambition_platformer2d_runtime::host_input::install_latched_slot_publication(app);

        // leafwing's `InputManagerPlugin` runs systems (e.g. `filter_captured_input`)
        // over Bevy's `ButtonInput<..>` resources, which `bevy::input::InputPlugin`
        // provides. A windowed host gets it from `DefaultPlugins`; a headless boot
        // (exit_3, RL, tests) uses `add_headless_foundation`, which has no
        // `InputPlugin` — and Bevy 0.18's strict system-param validation PANICS on the
        // missing `ButtonInput<MouseButton>` rather than skipping the system. Add it
        // here so the host input group is SELF-SUFFICIENT headless — the "boots from
        // the host groups alone" claim `exit_3` makes. Guarded, so it is a no-op when
        // `DefaultPlugins` already added it.
        if !app.is_plugin_added::<bevy::input::InputPlugin>() {
            app.add_plugins(bevy::input::InputPlugin);
        }
        // `update_active_input_kind` (added below) reads `MessageReader<CursorMoved>`,
        // a WINDOW message that `InputPlugin` does NOT register — a windowed host
        // gets it from `WindowPlugin`/`DefaultPlugins`, but a headless boot has no
        // window and Bevy 0.18 PANICS on the unregistered channel. Register it here
        // (idempotent) so the standard host-input path runs headlessly — the shape
        // RL and `tests/standard_input_path.rs` need.
        app.add_message::<bevy::window::CursorMoved>();

        app.init_resource::<MenuInputState>()
            .init_resource::<MenuControlFrame>()
            .init_resource::<ambition_input::SeatMenuFrames>()
            .init_resource::<ambition_input::SeatActiveDevices>()
            .add_plugins(InputManagerPlugin::<Platformer2dInputActionMonolith>::default())
            // ⭐ The SECOND action map's per-action registrations moved into
            // `ambition_input::install_provider_action_road` with the defect they
            // avoid: `InputManagerPlugin::<A>::build` adds two systems
            // UNCONDITIONALLY, so a second action type registers them twice and one of
            // them DRAINS the central input store. That is a fact about leafwing and
            // this crate's own action type, not about the host.
            // ⭐ The provider action road installs itself — including the
            // two-schedule split and the measured reason for it, which is a fact
            // about `ambition_input`'s own set layout rather than about this host.
            .add_systems(
                bevy::app::PreUpdate,
                tune_clash_strategy_to_bindings
                    .before(leafwing_input_manager::plugin::InputManagerSystem::Update),
            )
            // Leafwing orders both its Tick set (which CLEARS the central
            // input store) and its Unify set (which recomputes it from
            // devices) before Update, but leaves Tick vs Unify UNORDERED — a
            // topology seed decides whether a device kind's freshly computed
            // values survive to the action update or are wiped first. Pin
            // the only correct order explicitly: clear first, then every
            // device kind publishes, then actions resolve.
            .configure_sets(
                bevy::app::PreUpdate,
                leafwing_input_manager::plugin::InputManagerSystem::Tick
                    .before(leafwing_input_manager::plugin::InputManagerSystem::Unify),
            )
            // Track which input device each SEAT most recently produced GENUINE input with
            // (and, via `machine()`, the newest overall — which gates the menu mouse-hover
            // handlers so a rebuild-induced `Pointer<Over>` under a stationary mouse can't snap
            // the cursor back while a player navigates with keyboard / gamepad / touch). The
            // detector covers keyboard / mouse / gamepad / raw touch; the touch virtual-device
            // gesture adapter additionally marks `Touch` for overlay input a mouse can drive.
            // The persistent participant spawns ONCE at boot — before any
            // route, session, or gameplay actor exists — and is never
            // session-scoped. Startup cards and the launcher read the same
            // participant a later gameplay session does; possession, session
            // relaunch, and actor death never touch its device state.
            .add_systems(Startup, spawn_primary_input_participant)
            // A participant's map is BUILT from its declared recipe: the
            // persisted preset reaches the primary's recipe, and any recipe
            // change rebuilds that seat's map — every seat, in every
            // composition, not one seat in one app. `Collect` so the rebuilt
            // map is what `ResolveActions` projects into `SeatBindings` the
            // same frame (a preset change may not leave prompts showing the
            // old keys for a frame).
            .add_systems(
                Update,
                (
                    sync_primary_recipe_from_settings,
                    // …and the GAME's layout, between the person's settings and
                    // the rebuild: `device -> game profile -> semantic action`.
                    // Every seat, because a layout is a fact about the mode
                    // being played and not about who is holding which pad.
                    ambition_input::layout::apply_active_binding_layout_to_recipes,
                    ambition_input::rebuild_maps_from_recipes,
                )
                    .chain()
                    .in_set(ambition_input::InputSet::Collect),
            )
            // Context ownership: surfaces declare claims during
            // `ResolveContext` (the session lifecycle here; the shell's
            // startup/launcher surfaces in `ambition_game_shell`), then the
            // one resolver reduces them before anything routes on the answer.
            .add_systems(
                Update,
                (
                    // ONE authority for "which physical control is this
                    // action on", projected from the live `InputMap` every
                    // frame so a rebind cannot leave a prompt behind.
                    ambition_input::publish_seat_bindings
                        .in_set(ambition_input::InputSet::ResolveActions),
                    declare_gameplay_input_context.in_set(ambition_input::InputSet::ResolveContext),
                    // The in-session surfaces (dialogue, cutscene) declare in the
                    // same set as the session itself, so one resolver sees every
                    // claim before any router reads the answer.
                    declare_in_session_input_contexts
                        .in_set(ambition_input::InputSet::ResolveContext),
                    ambition_input::resolve_active_input_context
                        .after(ambition_input::InputSet::ResolveContext)
                        .before(ambition_input::InputSet::Route),
                ),
            )
            // Nesting rather than folding: `MenuNavConsume` keeps its own identity because the
            // menu-backend switch pins `.after` it and must NOT start waiting on cutscene skip too.
            .configure_sets(
                Update,
                (MenuFrameCutsceneSkip, MenuNavConsume).in_set(MenuFrameConsume),
            )
            // Collect semantic menu intent before gameplay input is
            // suppressed. `populate_seat_control_frames` may zero the
            // sim-side `ControlFrame` in UI modes, but it must not mutate
            // leafwing's `ActionState`; held keyboard/menu buttons should not
            // become `just_pressed` again on every dialog frame.
            //
            // Therefore the order is:
            // 1. read the participant's unified keyboard/gamepad/touch actions
            //    into `MenuControlFrame`,
            // 2. read/suppress gameplay into `ControlFrame`,
            // 3. let pointer gestures add scroll before consumers.
            .add_systems(
                Update,
                (
                    populate_menu_control_frame_from_actions
                        .in_set(ambition_input::InputSet::Route)
                        .in_set(MenuFramePopulate),
                    // The per-seat companion. Same phase, same inputs, different
                    // question: the global frame answers "did anybody press
                    // this", this one answers "which seat did".
                    populate_seat_menu_frames
                        .in_set(ambition_input::InputSet::Route)
                        .in_set(MenuFramePopulate),
                    //  ONE registration. There were two adjacent entries
                    // here, and the comment on the second said *"same phase as
                    // the primary bridge: both are device→control translation,
                    // and neither reads the other's output"* — which is the
                    // argument for them being one system, made while they were
                    // two. See `populate_seat_control_frames` for the six ways
                    // they had drifted.
                    populate_seat_control_frames.in_set(ambition_input::InputSet::Route),
                    toggle_player_trail_emission_from_actions,
                    apply_menu_frame_to_cutscene_request.in_set(MenuFrameCutsceneSkip),
                    dialog_pointer_input,
                )
                    .chain()
                    //  LOAD-BEARING ONLY under the `RenderFrame` host, where the
                    // sim schedule IS `Update`. `CoreSimulation` is a sim-schedule
                    // set, and a Bevy set node belongs to one schedule — under
                    // `Fixed60Hz`/`Ggrs` this creates an empty node here and
                    // constrains nothing.  and the frame order does not rescue a
                    // `.before` the way it rescues an `.after`: the sim has
                    // already run by the time `Update` starts.
                    //
                    // Keep the pin for the `RenderFrame` host; do not read it as proof the
                    // other two are ordered.
                    .before(Platformer2dSimulationPhaseMonolith::CoreSimulation),
            );

        // The roster-driven seating pair installs ITSELF: both systems live in the
        // monolith and its ordering against `InputSet::Collect` is that crate's
        // fact, not this composition's.
        ambition_platformer2d_runtime::host_input::install_roster_seating(app);
        // The menu crate cannot know an asset path and the render crate must
        // not own the menu IR, so the host is where the font handle crosses —
        // and only a host that HAS a renderer has a font to carry.
        #[cfg(feature = "render")]
        app.add_systems(Update, publish_menu_font);
    }
}

/// Derive leafwing's [`ClashStrategy`] from the live bindings: chord-free maps
/// run `PressAll` (skipping the per-frame `possible_clash` pair scans — 1-2.6%
/// of CPU in desktop-lifecycle-4, where no chord existed to resolve), and the
/// moment any composed game authors a chorded binding the strategy flips back
/// to `PrioritizeLongest` the same frame. Per-game by construction: each
/// composition's actual `InputMap`s decide, no configuration to forget.
#[cfg(feature = "input")]
fn tune_clash_strategy_to_bindings(
    maps: Query<
        bevy::ecs::change_detection::Ref<
            leafwing_input_manager::prelude::InputMap<
                ambition_input::Platformer2dInputActionMonolith,
            >,
        >,
    >,
    mut strategy: ResMut<leafwing_input_manager::prelude::ClashStrategy>,
) {
    use leafwing_input_manager::clashing_inputs::BasicInputs;
    use leafwing_input_manager::prelude::ClashStrategy;
    if !maps.iter().any(|map| map.is_changed()) {
        return;
    }
    let any_chord = maps.iter().any(|map| {
        map.iter_buttonlike().any(|(_, inputs)| {
            inputs
                .iter()
                .any(|input| matches!(input.decompose(), BasicInputs::Chord(_)))
        })
    });
    let desired = if any_chord {
        ClashStrategy::PrioritizeLongest
    } else {
        ClashStrategy::PressAll
    };
    if *strategy != desired {
        *strategy = desired;
    }
}

/// Projectiles that were fired are projectiles that are drawn.
///
/// Found by `scripts/check_engine_systems_are_engine_installed.py`, built after the same shape cost
/// three defects in four days (the world-label pass, the parallax theme load, the parallax layer
/// sync).
///
/// It lives in the HOST rather than in `PlatformerPresentationPlugin` for a
/// layering reason, not a taste one: the ordering edges name
/// `ambition_platformer2d_runtime::projectile_schedule::step_projectiles` and
/// `ambition_sim_view::PresentedPoseSet`, and `ambition_render` depends on
/// neither runtime nor the schedule. This crate's own description says it MAY
/// name render, runtime and sim_view — that is what the host layer is for, and
/// `camera_follow` is already here for exactly the same reason.
#[cfg(feature = "render")]
pub struct HostProjectileVisualsPlugin;

#[cfg(feature = "render")]
impl Plugin for HostProjectileVisualsPlugin {
    fn build(&self, app: &mut App) {
        // The systems below draw from sheet metadata, and the registry that
        // holds it was ALSO app-local — moving the systems out without this
        // made them fail `Res<SheetRegistry>` validation in eight tests, which
        // is the class one layer down: not a system nobody installed, a
        // RESOURCE nobody installed. `SheetRegistryPlugin` is idempotent for
        // exactly this reason.
        app.add_plugins(ambition_sprite_sheet::SheetRegistryPlugin);
        app.add_systems(
            Update,
            (
                // One unified, kind-driven visual pass for ALL projectiles
                // (player + enemy); the charge indicator is its own player-only
                // pass. Both after the step so a projectile fired this frame is
                // visible this frame rather than one frame late.
                ambition_render::rendering::projectile_visuals::sync_projectile_visuals
                    .in_set(ambition_render::rendering::SpriteVisualSync)
                    .after(ambition_platformer2d_runtime::projectile_schedule::ProjectileStepSet),
                ambition_render::rendering::projectile_visuals::sync_projectile_charge_visuals
                    .after(ambition_platformer2d_runtime::projectile_schedule::ProjectileStepSet),
            )
                // Both passes hang art off the PRESENTED body pose (the charge
                // orb tracks the hand; a projectile's origin tracks the firer),
                // so the frame-clock resample must already have run. Without
                // this edge they read last frame's presented pose while the
                // camera is on this one, and the art shears away from the body
                // it belongs to by a frame of motion.
                .after(ambition_sim_view::PresentedPoseSet)
                .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
        );
    }
}

/// A `VfxMessage` that is written is a `VfxMessage` that is drawn.
///
/// `VfxMessage` has TWO presentation consumers and only ONE of them was engine-installed:
/// `ambition_render::rendering::slash_visuals::spawn_slash_effects` is registered by
/// `ambition_render`'s own plugin and reaches every composition, while `fx::vfx_spawn_messages` —
/// the subscriber that draws `Burst` / `Dust` / `Impact` / `CoinPop` / `Explosion` / `BlinkEffects`
/// / `ResetEffects` / `SpeechBubble` — was registered nowhere but the shipped app. So every demo
/// binary wrote those messages into a queue nobody read.
///
/// It lives in the HOST rather than in `PlatformerPresentationPlugin` for the
/// same layering reason `HostProjectileVisualsPlugin` gives: the ordering edge
/// names `Platformer2dSimulationPhaseMonolith::CoreSimulation`, and
/// `ambition_render` depends on neither the schedule nor the runtime.
///
///  `update_blink_preview` is deliberately NOT here. It reads leafwing
/// action state to know the blink button is held, so it stays behind the app's
/// `input` persona with the rest of the input-driven presentation.
#[cfg(feature = "render")]
pub struct HostVfxPresentationPlugin;

#[cfg(feature = "render")]
impl Plugin for HostVfxPresentationPlugin {
    fn build(&self, app: &mut App) {
        // spelled out on purpose — a short path is INVISIBLE to
        // `scripts/check_engine_systems_are_engine_installed.py`. That checker only recognises a
        // registration whose FIRST path segment is an engine crate name, and the app registered
        // these as `fx::update_particles` / bare `vfx_spawn_messages`. The blind spot is already
        // written down for `setup_capture_target` in that file's own waiver list; do not re-shorten
        // these paths.
        // ⭐ THE FX CAPABILITY OWNS ITS OWN THREE-STAGE ORDER. This host registered
        // eight `ambition_render::fx` systems and carried every ordering argument for
        // them; both moved to `ambition_render::fx::install_fx_pipeline`, beside the
        // systems and the sets they name. Nothing inverts: the phase enum and
        // `session_world_exists` are shared_tangle's, which render already depends on.
        ambition_render::fx::install_fx_pipeline(app);
    }
}

/// The camera follow/shake cluster: publish the observer viewport into the
/// sim's camera-observation resolve, then apply the resolved snapshot
/// (`camera_follow`) after shake ticks. With `portal_render`, also the portal
/// camera-continuity wiring and the portal observation glue.
///
/// A host that needs to draw AFTER the camera lands (debug overlays, HUD
/// anchors) orders `.after(ambition_render::rendering::camera_follow)`.
#[cfg(feature = "render")]
pub struct HostCameraPlugin;

#[cfg(feature = "render")]
impl Plugin for HostCameraPlugin {
    fn build(&self, app: &mut App) {
        use ambition_render::rendering::camera_follow;

        // Render-owned camera view state, initialized with the presentation half that reads it
        // (nameplates, HUD, overlays) — the sim never touches it. The observer facts (gameplay
        // viewport + subject-safe region) are published by HostGameplayPresentationPlugin,
        // which orders its whole cluster before the sim's observation resolve. `camera_follow`
        // only APPLIES the resulting snapshot (E4-17).
        app.add_plugins(crate::gameplay_presentation::HostGameplayPresentationPlugin);
        // Render owns the painting; the host owns the fact that it is owed.
        app.add_plugins(ambition_render::gameplay_surround::GameplaySurroundPlugin);
        // The declared-HUD surface. A game gets a HUD by declaring slots on
        // its provider and publishing readouts; a route that declared none
        // spawns nothing, so this is inert for every game that has no HUD.
        app.add_plugins(ambition_render::hud::declared::DeclaredHudPlugin);
        app.add_systems(
            Update,
            (
                ambition_platformer2d_shared_tangle::camera_ease::tick_camera_shake,
                // ⭐ CHAINED AHEAD OF `camera_follow` FOR THE SAME REASON THE
                // SHAKE IS: the follow reads both live values, so ticking after
                // it would present a value one frame stale on every frame.
                ambition_platformer2d_shared_tangle::camera_ease::tick_finish_zoom,
                camera_follow,
            )
                .chain()
                .after(ambition_render::rendering::BossAnimation)
                // Read THIS frame's resolved snapshot, not last frame's.
                .after(ambition_sim_view::camera_snapshot::CameraObservationSet)
                .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
        );

        #[cfg(feature = "portal_render")]
        {
            app.add_plugins(crate::portal::PortalObservationPlugin);
            app.add_systems(
                Update,
                (
                    crate::portal::apply_portal_camera_continuity
                        .after(Platformer2dSimulationPhaseMonolith::CoreSimulation)
                        .after(crate::portal::sync_portal_camera_continuity_focus)
                        .before(camera_follow),
                    // Same-frame pad into the sim resolve (E4-17): after the
                    // continuity update, before the observation resolves.
                    ambition_render::rendering::publish_portal_camera_clamp
                        .after(crate::portal::apply_portal_camera_continuity)
                        .before(ambition_sim_view::camera_snapshot::CameraObservationSet),
                )
                    .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
            );
            // Hosts drawing camera-anchored debug visuals order themselves
            // `.after(this system)` (the Ambition debug overlay does).
            app.add_systems(
                Update,
                crate::portal::tag_portal_camera_continuity_camera
                    .in_set(crate::portal::PortalContinuityCameraTagged)
                    .after(camera_follow),
            );
        }
    }
}

#[cfg(all(test, feature = "input"))]
mod clash_strategy_tests {
    use super::tune_clash_strategy_to_bindings;
    use ambition_input::Platformer2dInputActionMonolith;
    use bevy::prelude::*;
    use leafwing_input_manager::prelude::{ButtonlikeChord, ClashStrategy, InputMap};

    fn app_with_map(map: InputMap<Platformer2dInputActionMonolith>) -> App {
        let mut app = App::new();
        app.init_resource::<ClashStrategy>()
            .add_systems(Update, tune_clash_strategy_to_bindings);
        app.world_mut().spawn(map);
        app
    }

    #[test]
    fn chord_free_bindings_relax_to_press_all() {
        let map = InputMap::new([(Platformer2dInputActionMonolith::Jump, KeyCode::Space)]);
        let mut app = app_with_map(map);
        app.update();
        assert_eq!(
            *app.world().resource::<ClashStrategy>(),
            ClashStrategy::PressAll,
        );
    }

    #[test]
    fn authoring_a_chord_reenables_clash_resolution_same_frame() {
        let map = InputMap::new([(Platformer2dInputActionMonolith::Jump, KeyCode::Space)]);
        let mut app = app_with_map(map);
        app.update();
        assert_eq!(
            *app.world().resource::<ClashStrategy>(),
            ClashStrategy::PressAll,
        );
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<InputMap<Platformer2dInputActionMonolith>>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .get_mut::<InputMap<Platformer2dInputActionMonolith>>(entity)
            .unwrap()
            .insert(
                Platformer2dInputActionMonolith::Interact,
                ButtonlikeChord::new([KeyCode::ControlLeft, KeyCode::KeyE]),
            );
        app.update();
        assert_eq!(
            *app.world().resource::<ClashStrategy>(),
            ClashStrategy::PrioritizeLongest,
        );
    }
}

/// Hand the menu crate the font the render side resolved.
///
///  `ambition_menu` set a font SIZE and no handle, so Bevy resolved
/// `Handle::<Font>::default()` — its built-in `FiraMono-subset.ttf`. Forcing the default handle
/// back reproduces the box; see `MenuFont`, which also records what about this is still
/// UNEXPLAINED (the same handle renders those glyphs elsewhere).
///
///  this is the composition root because it is the only place the two crates
/// can meet. `ambition_render` must not depend on `ambition_menu`
/// (presentation does not own the menu IR) and `ambition_menu` must not know an
/// asset path (it is renderer-agnostic). A host is exactly the thing that knows
/// both.
///
/// Idempotent and cheap: it writes only when the resolved handle changes, which
/// is once.
///
/// Gated with its only caller (`HostInputBindingsPlugin`, which is
/// `feature = "input"`); without this a feature-stripped build warns
/// `never used`, and CI compiles with `-D warnings` across configs.
///
/// ⛔ AND WITH `render` TOO, because the font it hands across is
/// `ambition_render`'s: an input-only host has no `UiFonts` to read and no menu
/// crate to write to. The two halves it exists to introduce are both behind that
/// feature.
#[cfg(all(feature = "input", feature = "render"))]
fn publish_menu_font(
    mut commands: bevy::prelude::Commands,
    fonts: Option<bevy::prelude::Res<ambition_render::ui_fonts::UiFonts>>,
    current: Option<bevy::prelude::Res<ambition_menu::render::bevy_ui::MenuFont>>,
) {
    let Some(fonts) = fonts else { return };
    // The SEMANTIC request, not a handle: `UiFonts` owns which family that is.
    let wanted = fonts.font_source(ambition_render::ui_fonts::UiFontWeight::Regular);
    if current.map(|current| current.0.clone()) == Some(wanted.clone()) {
        return;
    }
    commands.insert_resource(ambition_menu::render::bevy_ui::MenuFont(wanted));
}
