//! Game-side hookup for the 3D-cube OoT pause menu (#31): adds the lib's reusable
//! cube renderer ([`ambition_menu::kaleidoscope::KaleidoscopeMenuPlugin`]) and feeds it our
//! live 24-item inventory (via [`crate::menu::model`]). Runtime-toggleable vs the
//! existing Bevy-UI grid through [`InventoryUiBackend`].
//!
//! The cube is pause-gated ([`gate_kaleidoscope_menu`]): its order-8 `Camera3d` + ring are
//! only active while the inventory is open, so it never clears the screen to black
//! during play. Routing nav/selection input to it is the next step — see
//! `dev/journals/oot-cube-integration-plan.md`.

use ambition_menu::kaleidoscope::{
    rebuild_cube_faces, KaleidoscopeFocusVisuals, KaleidoscopeMenuConfig, KaleidoscopeMenuPlugin,
    KaleidoscopeRender, KaleidoscopeRenderPre,
};
use ambition_menu::{
    ActiveMenuPages, AmbitionInventoryUiPlugin, AmbitionMenuControl, MenuDynamicText,
    MenuDynamicTextContent, MenuVisualState,
};
use ambition_sandbox::menu::backend::{InventoryUiBackend, KALEIDOSCOPE_MENU_BACKEND_ENABLED};
use bevy::prelude::*;

use crate::menu::effects::{MenuEffectManaQuery, MenuEffectPlayers};
use crate::menu::model::{
    build_inventory_pages, items_detail_slot_text, scroll_fraction_to_window_start,
    system_detail_slot_text, system_effective_window_start, system_max_window_start, system_rows,
    MenuFocus, MenuPage, MenuPageAction, SystemRow, SYSTEM_VISIBLE_ROWS,
};
use ambition_sandbox::audio::SfxMessage;
use ambition_sandbox::engine_core::Vec2;
use ambition_sandbox::input::MenuControlFrame;
use ambition_sandbox::items::{Item, OwnedItems, ITEM_GRID_COLS, ITEM_GRID_ROWS};
use ambition_sandbox::persistence::settings::{
    apply_settings_option, settings_menu_model, DevSnapshot, DevToggleId, RadioSnapshot,
    SettingsOptionId, SettingsOptionKind, SystemMenuAction, SystemMenuEntryId, SystemMenuModel,
    SystemOptionId, UserSettings,
};
use ambition_sandbox::player::PlayerHealRequested;

/// Play a one-shot UI sound for the cube menu: `Play { id, pos }` with `pos = ZERO`.
/// `Play` is non-spatialized (see `audio::runtime::audio_play_sfx_messages` — it
/// looks the id up in the bank and plays it full-volume; the `pos` is unused for
/// `Play`), so `Vec2::ZERO` keeps menu sounds audible at full volume. If the id
/// isn't packed into the runtime bank yet the play just no-ops (safe).
#[inline]
pub(crate) fn play_ui(sfx: &mut MessageWriter<SfxMessage>, id: ambition_sfx::SfxId) {
    sfx.write(SfxMessage::Play {
        id,
        pos: Vec2::ZERO,
    });
}

/// Install backend-agnostic menu resources/plugins shared by the flat Grid and
/// the optional 3D cube backend. Keep this separate from cube installation so a
/// Grid-only build does not spawn the cube camera/ring or register Lunex systems.
pub fn install_unified_menu_shared(app: &mut App) {
    app.init_resource::<InventoryUiBackend>()
        .init_resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .init_resource::<KaleidoscopeCursor>()
        // The pointer-hover handlers read `ActiveInputKind`. The input plugin
        // also inits it; init here too so the menu remains self-sufficient
        // (`init_resource` is idempotent).
        .init_resource::<ambition_sandbox::input::ActiveInputKind>()
        .init_resource::<KaleidoscopeSystemNav>()
        .add_plugins(AmbitionInventoryUiPlugin);
}

/// The menu BACKEND SEAM as a single run-condition: gate a system on
/// "the 3D kaleidoscope backend is installed and active." Systems whose only
/// backend handling was a bare `if *backend != LunexKaleidoscope { return; }`
/// early-return are now registered `.run_if(kaleidoscope_backend_active)`
/// instead, so "which backend is active" is expressed in ONE place rather than
/// scattered across each system body.
fn kaleidoscope_backend_active(backend: Res<InventoryUiBackend>) -> bool {
    KALEIDOSCOPE_MENU_BACKEND_ENABLED
        && backend.effective() == InventoryUiBackend::LunexKaleidoscope
}

/// The cube backend is selected and the inventory overlay is currently open.
///
/// Use this for host-side model/text/focus work that has no value while the
/// cube is closed. The open-routing and camera gate intentionally stay broader:
/// they must run while closed so they can open the menu and keep the camera off.
fn kaleidoscope_menu_visible(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<ambition_sandbox::inventory::InventoryUiState>>,
) -> bool {
    KALEIDOSCOPE_MENU_BACKEND_ENABLED
        && backend.effective() == InventoryUiBackend::LunexKaleidoscope
        && ui_state.map(|s| s.visible).unwrap_or(false)
}

/// The cube renderer needs to tick while open and briefly while folding closed.
///
/// This avoids the original closed-menu churn (camera/ring/picking/fade systems
/// running every frame just because Cube is the selected backend) without cutting
/// off the close animation.
fn kaleidoscope_render_needed(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<ambition_sandbox::inventory::InventoryUiState>>,
    open_state: Option<Res<ambition_menu::kaleidoscope::KaleidoscopeOpenState>>,
) -> bool {
    if !KALEIDOSCOPE_MENU_BACKEND_ENABLED
        || backend.effective() != InventoryUiBackend::LunexKaleidoscope
    {
        return false;
    }
    if ui_state.map(|s| s.visible).unwrap_or(false) {
        return true;
    }
    open_state
        .map(|s| s.target > 0.0 || s.amount > 0.08)
        .unwrap_or(false)
}

/// Peak opacity of the readability dim-scrim (black) when the cube is fully open.
/// The game runs the cube as an Option-1 overlay (cube camera clears `None`, so the
/// live world shows through); that busy world wrecks the cube text contrast. A
/// full-screen translucent-black `bevy_ui` Node on the order-0 `Camera2d` renders
/// UNDER the order-8 cube but OVER the world, dimming the world so the cube text
/// reads. The demo doesn't need this (it has a dark `ClearColor`).
const SCRIM_PEAK_ALPHA: f32 = 0.7;

/// Marks the full-screen readability dim-scrim node (game overlay only).
#[derive(Component)]
struct KaleidoscopeScrim;

/// Wire the 3D-cube menu into the app: the lib plugins + our page-feed system.
///
/// This compatibility wrapper also installs the shared menu resources, which keeps
/// older tests/fixtures that call `install_kaleidoscope_menu` directly working.
pub fn install_kaleidoscope_menu(app: &mut App) {
    install_unified_menu_shared(app);
    install_kaleidoscope_menu_backend(app);
}

/// Install only the optional 3D cube backend. The caller must install
/// [`install_unified_menu_shared`] first.
pub fn install_kaleidoscope_menu_backend(app: &mut App) {
    // The game uses Bevy picking on the cube controls AND draws its own real L/R
    // edge buttons (see `menu::model::add_edge_buttons`), so it inserts its own
    // `KaleidoscopeMenuConfig` (lib overlay defaults, but `draw_nav_arrows = false` so the
    // decorative arrows don't double-draw and `pickable_controls = true` so
    // `Pointer<*>` events fire) BEFORE the plugin (which only inserts a default
    // if the host hasn't).
    if !app.world().contains_resource::<KaleidoscopeMenuConfig>() {
        app.insert_resource(KaleidoscopeMenuConfig {
            draw_nav_arrows: false,
            pickable_controls: true,
            ..Default::default()
        });
    }
    app.init_resource::<KaleidoscopeScroll>()
        .init_resource::<KaleidoscopePointerPress>()
        .add_plugins(KaleidoscopeMenuPlugin::<MenuPage, MenuPageAction>::default());
    // Gate only the cube's render sets: closed menus can still open and manage
    // camera routing, while face rendering stops when it is not visible/animating.
    app.configure_sets(
        Update,
        KaleidoscopeRender.run_if(kaleidoscope_render_needed),
    )
    .configure_sets(
        PostUpdate,
        KaleidoscopeRender.run_if(kaleidoscope_render_needed),
    )
    .configure_sets(
        PreUpdate,
        KaleidoscopeRenderPre.run_if(kaleidoscope_render_needed),
    );
    app.add_systems(Startup, spawn_kaleidoscope_scrim)
        .add_systems(
            Update,
            (
                // Route pause/Esc, inventory, and map into the cube backend on the
                // matching page before navigation consumes the frame.
                // In MenuNavConsume so `fold_to_menu_control_frame`'s
                // `.before(MenuNavConsume)` guarantees this sees the touch
                // Menu-button press AFTER the fold writes it. Without set
                // membership, the `.after(rebuild_cube_faces)` chain constraint
                // could schedule this before the fold, making it miss the
                // pressed_this_frame bit and never opening the menu on touch.
                kaleidoscope_menu_open_routing
                    .run_if(kaleidoscope_backend_active)
                    .in_set(ambition_sandbox::app::MenuNavConsume),
                // Nav first (mutates the cursor), then republish (reads the cursor +
                // inventory) so the highlight + detail panel reflect this frame's move.
                // Also in `MenuNavConsume` for the same fold-ordering reason above.
                kaleidoscope_focus_nav
                    .run_if(kaleidoscope_menu_visible)
                    .in_set(ambition_sandbox::app::MenuNavConsume),
                // Scroll the System window independently of selection before republish.
                kaleidoscope_scroll_wheel.run_if(kaleidoscope_menu_visible),
                kaleidoscope_apply_scroll_drag.run_if(kaleidoscope_menu_visible),
                // Republish the cube's model only for the cube backend; Grid builds and
                // dirties its own page model.
                republish_kaleidoscope_pages.run_if(kaleidoscope_menu_visible),
                // The focus HIGHLIGHT and the detail-panel TEXT now update IN PLACE
                // from the live cursor (no face rebuild), so a mouse move no longer
                // despawns the hovered control between a pointer press and release —
                // the deferred Bug 2 (clicks were dropped on the entity-id mismatch).
                kaleidoscope_sync_focus_visuals.run_if(kaleidoscope_menu_visible),
                kaleidoscope_sync_detail_text.run_if(kaleidoscope_menu_visible),
                gate_kaleidoscope_menu,
                toggle_inventory_backend,
                // The readability dim-scrim is cube-only; the Startup node stays
                // transparent when the cube backend is inactive.
                retarget_kaleidoscope_scrim.run_if(kaleidoscope_render_needed),
                fade_kaleidoscope_scrim.run_if(kaleidoscope_render_needed),
            )
                .chain()
                // CURSOR-HIGHLIGHT fix: the lib renders the focus highlight (material
                // recolour + white selection corners) from `MenuVisualState` via the
                // `Changed`-gated `KaleidoscopeFocusVisuals` readers. Run this chain —
                // which includes the `kaleidoscope_sync_focus_visuals` WRITER — AFTER
                // `rebuild_cube_faces` (so a republish that respawns the controls can't
                // wipe the flags the writer set) and BEFORE the lib readers (so they see
                // the flipped flags the same frame). Without these edges the writer and
                // the rebuild/readers were unordered: a republish-driven rebuild reset
                // `MenuVisualState` after the write and/or the readers ran first, so the
                // highlight never appeared (keyboard nav + mouse hover both went dark).
                .after(rebuild_cube_faces::<MenuPage, MenuPageAction>)
                .before(KaleidoscopeFocusVisuals),
        )
        .add_observer(kaleidoscope_pointer_press)
        .add_observer(kaleidoscope_pointer_move)
        .add_observer(kaleidoscope_pointer_release);
}

/// Which input source currently owns the cube cursor. Mirrors the grid's
/// [`ambition_sandbox::ui_nav::MenuFocusOwner`]: keyboard/gamepad nav claims focus and keeps
/// it until the pointer moves to a DIFFERENT control. A stationary hover must not
/// keep reasserting itself over newer directional navigation (the "can't move away
/// from the hovered option" bug).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum FocusSource {
    #[default]
    Keyboard,
    Pointer,
}

/// The directional-focus cursor for the items page: which item slot or edge
/// (page-turn) button the cursor sits on. Mirrors the demo's selection state
/// (`MockDemo::selected`). [`kaleidoscope_focus_nav`] moves it with `move_spatial`-style
/// rules; [`republish_kaleidoscope_pages`] republishes the page model whenever its
/// SEMANTIC focus changes so the highlight + detail panel follow it.
#[derive(Resource, Default)]
pub(crate) struct KaleidoscopeCursor {
    focus: MenuFocus,
    /// Which input source last moved the cursor (keyboard nav vs pointer hover).
    owner: FocusSource,
    /// The last focus the POINTER moved over. A parked mouse should not count as a
    /// selection; only actual pointer motion can change the cursor here.
    last_pointer_focus: Option<MenuFocus>,
}

impl KaleidoscopeCursor {
    /// Keyboard/gamepad nav took the cursor to `focus` (claims ownership).
    pub(crate) fn mark_keyboard(&mut self, focus: MenuFocus) {
        self.focus = focus;
        self.owner = FocusSource::Keyboard;
    }

    /// The cursor's current logical focus (shared by both backends).
    pub(crate) fn focus(&self) -> MenuFocus {
        self.focus
    }
}

/// Drill-down state for the System face. `None` = the top-level category list is
/// shown (Video / Audio / Controls / Gameplay + Close Menu); `Some(category)` = the
/// open category's option rows + a Back row are shown. Mirrors the Bevy-UI pause
/// menu's settings page stack. `republish_kaleidoscope_pages` feeds this into
/// `build_system_page`, and changing it republishes (the System cursor resets to
/// row 0). B0002-safe: only `kaleidoscope_focus_nav` / `kaleidoscope_pointer_release` mutate it (both
/// `ResMut`); `republish_kaleidoscope_pages` reads it as `Res`.
#[derive(Resource, Default)]
pub(crate) struct KaleidoscopeSystemNav {
    pub(crate) open_entry: Option<SystemMenuEntryId>,
}

/// Feature E: how far (screen pixels) a pointer may travel between press and release
/// before the press is treated as a DRAG (cancelling the would-be click). A clean tap
/// stays under this; a press-then-drag-away exceeds it and does NOT activate. Touch is
/// a pointer in Bevy, so this same threshold governs touch taps vs touch drags.
const KALEIDOSCOPE_TAP_DRAG_THRESHOLD: f32 = 12.0;

/// Feature E: the in-flight pointer press, so a press-then-drag-away can be CANCELLED
/// (no activation) while a clean tap still activates. Set on `Pointer<Press>`, marked
/// `cancelled` once the pointer travels past [`KALEIDOSCOPE_TAP_DRAG_THRESHOLD`] from
/// the press origin (a drag, not a tap), and consumed by the click observer. Mouse OR
/// touch — both arrive through the same pointer events, so this is mouse-testable.
#[derive(Resource, Default)]
pub(crate) struct KaleidoscopePointerPress {
    /// The entity the active press landed on, if any.
    entity: Option<Entity>,
    /// The ACTION the pressed control carries, captured at press time. Dispatch on
    /// RELEASE uses THIS (not the release entity), so a face rebuild that despawns +
    /// respawns the control between press and release cannot drop the click — the
    /// historical `Pointer<Click>` failure (press/release must resolve to the SAME
    /// entity, which the rebuilding perspective cube routinely broke).
    action: Option<MenuPageAction>,
    /// Screen position the press started at.
    origin: Vec2,
    /// True once the pointer dragged past the tap threshold (cancels the click).
    cancelled: bool,
}

/// Host-owned, SELECTION-INDEPENDENT scroll position for the System face's windowed
/// list (Features C/D). `None` = the window follows the keyboard/pointer cursor
/// (the historical behaviour); `Some(start)` = an explicit scroll override set by a
/// scrollbar DRAG (Feature C, via the lib's neutral [`ambition_menu::kaleidoscope::MenuScrollDragged`]
/// signal) or the MOUSE WHEEL (Feature D). Keyboard navigation clears the override so
/// the window resumes following the cursor. This is the host-side meaning of the
/// lib's backend-agnostic scroll signal — the lib never knows about rows/window_start.
#[derive(Resource, Default)]
struct KaleidoscopeScroll {
    /// Explicit System scroll-window start, or `None` to follow the cursor.
    system_window_start: Option<usize>,
}

/// All the live resources the broadened SYSTEM screens need to READ a snapshot
/// and APPLY a selection, bundled into one [`SystemParam`] so the cube nav system
/// / pointer observer stay within Bevy's 16-param ceiling. The radio resources are
/// `audio`-gated; `DeveloperTools` + `SandboxResetRequested` are always present
/// (inserted at startup), so accessing them never panics. Held mutably here; the
/// two consumers (`kaleidoscope_focus_nav`, `kaleidoscope_pointer_release`) are separate systems so
/// there is no B0002 conflict, and `republish_kaleidoscope_pages` reads its own `Res`
/// copies (`SystemMenuSnapshotParams`) in a third system.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct SystemMenuParams<'w> {
    dev_tools: ResMut<'w, ambition_sandbox::dev::dev_tools::DeveloperTools>,
    // The Developer screen also reaches the F1/F2 global flags + F12 LDtk
    // auto-reload, which live on these two resources (not `DeveloperTools`).
    dev_state: ResMut<'w, ambition_sandbox::SandboxDevState>,
    ldtk_reload: ResMut<'w, ambition_sandbox::ldtk_world::LdtkHotReloadState>,
    // The active menu frontend, mutated by the Developer "Menu Backend" row (the
    // in-menu `\` toggle). Always present (inserted at startup).
    backend: ResMut<'w, InventoryUiBackend>,
    // The Portal FX cycle's target (portal presentation crate, host adapter).
    // Option: absent in non-portal personas / minimal fixtures — the row then
    // no-ops and reads "n/a".
    #[cfg(feature = "portal_render")]
    portal_effect: Option<ResMut<'w, ambition_sandbox::portal::PortalEffectSelection>>,
    reset: ResMut<'w, ambition_sandbox::runtime::reset::SandboxResetRequested>,
    // Movement tuning is derived from the active movement profile, so a
    // Reset All Settings must restore it to match the reset DeveloperTools
    // defaults (mirrors the pause menu's `ResetAllSettings`).
    editable_tuning: ResMut<'w, ambition_sandbox::dev::dev_tools::EditableMovementTuning>,
    // The radio resources are `Option`-wrapped so the System nav stays B0002-safe
    // and never panics when audio is off / a fixture omits them: a missing radio
    // resource simply disables station audition (the rows still render). Gated on
    // `audio` so non-audio builds carry none of the types.
    #[cfg(feature = "audio")]
    library: Option<ResMut<'w, ambition_sandbox::audio::AudioLibrary>>,
    #[cfg(feature = "audio")]
    asset_server: Option<Res<'w, AssetServer>>,
    #[cfg(feature = "audio")]
    music_state: Option<ResMut<'w, ambition_sandbox::audio::MusicPlaybackState>>,
    #[cfg(feature = "audio")]
    radio: Option<ResMut<'w, ambition_sandbox::audio::RadioStationState>>,
    #[cfg(feature = "audio")]
    music_channel: Option<
        Res<'w, bevy_kira_audio::prelude::AudioChannel<ambition_sandbox::audio::MusicChannel>>,
    >,
}

impl SystemMenuParams<'_> {
    /// The active inventory frontend. Read it from HERE (not a separate
    /// `Res<InventoryUiBackend>` param) in any system that also holds
    /// `SystemMenuParams` — this bundle owns the resource (mutably, for the
    /// Developer "Menu Backend" row), so a duplicate `Res` access in the same
    /// system would be a Bevy B0002 conflict.
    pub(crate) fn backend(&self) -> InventoryUiBackend {
        self.backend.effective()
    }

    /// Apply a non-settings System screen option against its live resource.
    /// Radio auditions a station (keeps the menu open); Locale is a no-op stub
    /// (only English exists); Dev toggles/cycles mutate `DeveloperTools`.
    /// Returns the SFX id to play for feedback.
    pub(crate) fn apply_option(&mut self, opt: SystemOptionId) -> ambition_sfx::SfxId {
        match opt {
            SystemOptionId::Radio(index) => {
                #[cfg(feature = "audio")]
                if let (
                    Some(library),
                    Some(asset_server),
                    Some(music_state),
                    Some(radio),
                    Some(music_channel),
                ) = (
                    self.library.as_deref_mut(),
                    self.asset_server.as_deref(),
                    self.music_state.as_deref_mut(),
                    self.radio.as_deref_mut(),
                    self.music_channel.as_deref(),
                ) {
                    if let Some(track_id) = library.track_at(index).map(|t| t.id.clone()) {
                        ambition_sandbox::audio::set_radio_track(
                            library,
                            asset_server,
                            radio,
                            music_state,
                            music_channel,
                            &track_id,
                        );
                        return ambition_sfx::ids::UI_MENU_ACCEPT;
                    }
                }
                let _ = index;
                ambition_sfx::ids::UI_MENU_ERROR
            }
            SystemOptionId::Locale(id) => {
                // Language is a stub: only English is selectable. Selecting it is a
                // confirm; anything else is an error beep.
                if id.is_available() {
                    ambition_sfx::ids::UI_MENU_ACCEPT
                } else {
                    ambition_sfx::ids::UI_MENU_ERROR
                }
            }
            SystemOptionId::Dev(id) => {
                apply_dev_toggle(
                    DevToggleWrite {
                        dev: &mut self.dev_tools,
                        dev_state: &mut self.dev_state,
                        ldtk_reload: &mut self.ldtk_reload,
                        backend: &mut self.backend,
                        #[cfg(feature = "portal_render")]
                        portal_effect: self.portal_effect.as_deref_mut(),
                    },
                    id,
                    0,
                );
                if id.is_cycle() {
                    ambition_sfx::ids::UI_SLIDER_TICK
                } else {
                    ambition_sfx::ids::UI_TOGGLE_ON
                }
            }
        }
    }

    /// Step a value-style screen option in place (radio prev/next station, dev
    /// cycle prev/next). Toggles + locales ignore stepping (handled by select).
    fn step_option(&mut self, opt: SystemOptionId, dir: i32) -> Option<ambition_sfx::SfxId> {
        match opt {
            SystemOptionId::Dev(id) if id.is_cycle() => {
                apply_dev_toggle(
                    DevToggleWrite {
                        dev: &mut self.dev_tools,
                        dev_state: &mut self.dev_state,
                        ldtk_reload: &mut self.ldtk_reload,
                        backend: &mut self.backend,
                        #[cfg(feature = "portal_render")]
                        portal_effect: self.portal_effect.as_deref_mut(),
                    },
                    id,
                    dir,
                );
                Some(ambition_sfx::ids::UI_SLIDER_TICK)
            }
            _ => None,
        }
    }

    pub(crate) fn request_reset(&mut self) {
        self.reset.request();
    }

    /// Reset every persisted settings/dev resource back to defaults — the same
    /// reset the pause menu's `SettingsItem::ResetAllSettings` performs: restore
    /// `UserSettings` + `DeveloperTools` to defaults, then re-derive the editable
    /// movement tuning from the (now-default) movement profile so dependent state
    /// stays coherent. The cube dispatch holds no live player kinematics here, so
    /// the live-movement refs are `None` (the pause menu also passes `None` when
    /// it has no live player to poke); the persisted resources still reset fully.
    pub(crate) fn reset_all_settings(&mut self, settings: &mut UserSettings) {
        *settings = UserSettings::default();
        *self.dev_tools = ambition_sandbox::dev::dev_tools::DeveloperTools::default();
        ambition_sandbox::dev::dev_tools::apply_movement_profile(
            &mut self.editable_tuning,
            self.dev_tools.movement_profile,
            None,
        );
    }

    /// Build the live radio snapshot for the SYSTEM IR (empty under no `audio` /
    /// when the radio resources are absent). `pub(crate)` so the Grid backend's
    /// republish can build the System page directly from the same snapshot the cube
    /// uses (one model, two renderers).
    pub(crate) fn radio_snapshot(&self) -> RadioSnapshot {
        #[cfg(feature = "audio")]
        if let (Some(library), Some(music_state)) =
            (self.library.as_deref(), self.music_state.as_deref())
        {
            return radio_snapshot_from(library, music_state, self.radio.as_deref());
        }
        RadioSnapshot::default()
    }

    /// Build the live developer-toggle snapshot for the SYSTEM IR. `pub(crate)` for
    /// the same reason as [`Self::radio_snapshot`].
    pub(crate) fn dev_snapshot(&self) -> DevSnapshot {
        dev_snapshot(DevToggleRead {
            dev: &self.dev_tools,
            dev_state: &self.dev_state,
            ldtk_reload: &self.ldtk_reload,
            backend: *self.backend,
            #[cfg(feature = "portal_render")]
            portal_effect: self.portal_effect.as_deref(),
        })
    }

    /// Build the live SYSTEM model from current settings + held resources.
    pub(crate) fn model(&self, settings: &UserSettings) -> SystemMenuModel {
        SystemMenuModel::build(settings, &self.radio_snapshot(), &self.dev_snapshot())
    }
}

/// Read the current game mode + queue the next one, bundled into ONE [`SystemParam`]
/// so the nav system / pointer observer that need to UNPAUSE on a close-via-action
/// (e.g. Reset Sandbox) stay within Bevy's 16-param ceiling. Threaded into
/// [`close_kaleidoscope_menu`] via [`Self::mode`] + [`Self::next_mode`].
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct GameModeIo<'w> {
    state: Res<'w, State<ambition_sandbox::runtime::game_mode::GameMode>>,
    next: ResMut<'w, NextState<ambition_sandbox::runtime::game_mode::GameMode>>,
}

/// Resources `republish_kaleidoscope_pages` reads (immutably) to snapshot the radio + dev
/// state into the SYSTEM IR. Separate `Res` bundle so it never conflicts with the
/// mutable `SystemMenuParams` (different systems).
#[derive(bevy::ecs::system::SystemParam)]
struct SystemMenuSnapshotParams<'w> {
    dev_tools: Res<'w, ambition_sandbox::dev::dev_tools::DeveloperTools>,
    dev_state: Res<'w, ambition_sandbox::SandboxDevState>,
    ldtk_reload: Res<'w, ambition_sandbox::ldtk_world::LdtkHotReloadState>,
    backend: Res<'w, InventoryUiBackend>,
    #[cfg(feature = "portal_render")]
    portal_effect: Option<Res<'w, ambition_sandbox::portal::PortalEffectSelection>>,
    #[cfg(feature = "audio")]
    library: Option<Res<'w, ambition_sandbox::audio::AudioLibrary>>,
    #[cfg(feature = "audio")]
    music_state: Option<Res<'w, ambition_sandbox::audio::MusicPlaybackState>>,
    #[cfg(feature = "audio")]
    radio: Option<Res<'w, ambition_sandbox::audio::RadioStationState>>,
}

impl SystemMenuSnapshotParams<'_> {
    /// Build the live radio-station snapshot for the SYSTEM IR (empty under no
    /// `audio` / when the radio resources are absent).
    fn radio_snapshot(&self) -> RadioSnapshot {
        #[cfg(feature = "audio")]
        if let (Some(library), Some(music_state)) =
            (self.library.as_deref(), self.music_state.as_deref())
        {
            return radio_snapshot_from(library, music_state, self.radio.as_deref());
        }
        RadioSnapshot::default()
    }

    /// Build the live developer-toggle snapshot for the SYSTEM IR.
    fn dev_snapshot(&self) -> DevSnapshot {
        dev_snapshot(DevToggleRead {
            dev: &self.dev_tools,
            dev_state: &self.dev_state,
            ldtk_reload: &self.ldtk_reload,
            backend: *self.backend,
            #[cfg(feature = "portal_render")]
            portal_effect: self.portal_effect.as_deref(),
        })
    }
}

/// Build a [`RadioSnapshot`] from the live audio library + playback state. The
/// single place that maps the audio runtime onto the SYSTEM IR's station list.
#[cfg(feature = "audio")]
fn radio_snapshot_from(
    library: &ambition_sandbox::audio::AudioLibrary,
    music_state: &ambition_sandbox::audio::MusicPlaybackState,
    radio: Option<&ambition_sandbox::audio::RadioStationState>,
) -> RadioSnapshot {
    let active_id = radio
        .and_then(|r| r.selected_track())
        .unwrap_or(music_state.active_track.as_str())
        .to_string();
    let active = library.track_index(&active_id);
    let stations = (0..library.track_count())
        .filter_map(|i| library.display_name_at(i).map(|name| (i, name.to_string())))
        .collect();
    RadioSnapshot { stations, active }
}

/// The set of resources the Developer screen reads/writes. The dev-toggle path
/// spans THREE resources: most toggles live on `DeveloperTools`, but the F1/F2
/// global flags live on [`SandboxDevState`] and the F12 LDtk hot-reload toggle
/// lives on [`LdtkHotReloadState`] — mirroring the pause-menu Developer page,
/// which aggregates the same three. Bundled so [`dev_snapshot`] /
/// [`apply_dev_toggle`] stay single-source for every Developer row.
struct DevToggleRead<'a> {
    dev: &'a ambition_sandbox::dev::dev_tools::DeveloperTools,
    dev_state: &'a ambition_sandbox::SandboxDevState,
    ldtk_reload: &'a ambition_sandbox::ldtk_world::LdtkHotReloadState,
    // The Menu Backend row mirrors the `\` hotkey; its value label is the active
    // frontend (Grid / Cube), read from `InventoryUiBackend`.
    backend: InventoryUiBackend,
    // The Portal FX row's live effect (view cones / masks / off). Option so
    // fixtures without the resource still render the row (as "n/a").
    #[cfg(feature = "portal_render")]
    portal_effect: Option<&'a ambition_sandbox::portal::PortalEffectSelection>,
}

struct DevToggleWrite<'a> {
    dev: &'a mut ambition_sandbox::dev::dev_tools::DeveloperTools,
    dev_state: &'a mut ambition_sandbox::SandboxDevState,
    ldtk_reload: &'a mut ambition_sandbox::ldtk_world::LdtkHotReloadState,
    backend: &'a mut InventoryUiBackend,
    #[cfg(feature = "portal_render")]
    portal_effect: Option<&'a mut ambition_sandbox::portal::PortalEffectSelection>,
}

/// Read every developer toggle/cycle into a [`DevSnapshot`] for the SYSTEM IR. The
/// single place mapping the three dev resources onto [`DevToggleId`]s for display.
fn dev_snapshot(ctx: DevToggleRead<'_>) -> DevSnapshot {
    use DevToggleId as D;
    let dev = ctx.dev;
    let mut values = Vec::with_capacity(DevToggleId::ALL.len());
    // Global dev flags (SandboxDevState) — the F1/F2 rows.
    values.push(DevSnapshot::toggle(
        D::DebugOverlay,
        ctx.dev_state.debug_enabled(),
    ));
    values.push(DevSnapshot::toggle(D::SlowMotion, ctx.dev_state.slowmo));
    values.push(DevSnapshot::toggle(D::Inspector, dev.inspector_visible));
    values.push(DevSnapshot::toggle(
        D::WorldInspector,
        dev.world_inspector_visible,
    ));
    values.push(DevSnapshot::toggle(D::Gizmos, dev.gizmos_enabled));
    values.push(DevSnapshot::toggle(D::ShowHud, dev.show_hud));
    values.push(DevSnapshot::toggle(
        D::ShowHitboxes,
        dev.show_feature_hitboxes,
    ));
    values.push(DevSnapshot::toggle(D::HideSprites, dev.hide_sprites));
    values.push(DevSnapshot::toggle(
        D::PlaceholderSprites,
        dev.placeholder_sprites,
    ));
    values.push(DevSnapshot::toggle(D::FillDebugBoxes, dev.fill_debug_boxes));
    values.push(DevSnapshot::toggle(D::MicroGrid, dev.show_micro_grid));
    values.push(DevSnapshot::toggle(D::CameraFrame, dev.show_camera_frame));
    values.push(DevSnapshot::toggle(D::OverviewCamera, dev.overview_camera));
    values.push(DevSnapshot::cycle(
        D::DebugViewMode,
        dev.debug_view_mode.label(),
    ));
    values.push(DevSnapshot::cycle(
        D::DebugArtMode,
        dev.debug_art_mode.label(),
    ));
    values.push(DevSnapshot::cycle(
        D::PlayerBodyProfile,
        dev.player_body_profile.label(),
    ));
    values.push(DevSnapshot::cycle(
        D::MovementProfile,
        dev.movement_profile.label(),
    ));
    // LDtk hot-reload (LdtkHotReloadState) — the F12 row.
    values.push(DevSnapshot::toggle(
        D::LdtkAutoApply,
        ctx.ldtk_reload.auto_apply,
    ));
    // Menu frontend (InventoryUiBackend) — the `\`-hotkey row, a cycle whose value
    // label is the active frontend name.
    values.push(DevSnapshot::cycle(D::MenuBackend, ctx.backend.label()));
    // Portal FX (PortalEffectSelection, host adapter) — the A/B profiling
    // cycle over the compiled-in portal transit visuals.
    #[cfg(feature = "portal_render")]
    values.push(DevSnapshot::cycle(
        D::PortalEffect,
        ctx.portal_effect.map_or("n/a", |s| s.active.label()),
    ));
    #[cfg(not(feature = "portal_render"))]
    values.push(DevSnapshot::cycle(D::PortalEffect, "not compiled"));
    DevSnapshot { values }
}

/// Apply a single developer toggle/cycle to `DeveloperTools`. `dir` selects the
/// direction for cycles (`<0` prev, otherwise next); toggles flip regardless. This
/// is the single place that mutates `DeveloperTools` from the cube, so the dev
/// menu and the inspector stay in lock-step on field semantics.
fn apply_dev_toggle(ctx: DevToggleWrite<'_>, id: DevToggleId, dir: i32) {
    use DevToggleId as D;
    let dev = ctx.dev;
    match id {
        // Global dev flags — F1/F2, on `SandboxDevState` (mirrors the pause menu's
        // `SettingsItem::DebugOverlay` / `SlowMotion` arms).
        D::DebugOverlay => ctx.dev_state.debug = !ctx.dev_state.debug,
        D::SlowMotion => ctx.dev_state.slowmo = !ctx.dev_state.slowmo,
        D::Inspector => dev.inspector_visible = !dev.inspector_visible,
        D::WorldInspector => dev.world_inspector_visible = !dev.world_inspector_visible,
        D::Gizmos => dev.gizmos_enabled = !dev.gizmos_enabled,
        D::ShowHud => dev.show_hud = !dev.show_hud,
        // Mirror the pause menu's `ShowHitboxes` arm exactly: mark the debug view
        // custom and flip BOTH the feature- and player-hitbox flags together.
        D::ShowHitboxes => {
            dev.mark_debug_view_custom();
            let next = !dev.show_feature_hitboxes;
            dev.show_feature_hitboxes = next;
            dev.show_player_hitbox = next;
        }
        D::HideSprites => dev.hide_sprites = !dev.hide_sprites,
        D::PlaceholderSprites => dev.placeholder_sprites = !dev.placeholder_sprites,
        D::FillDebugBoxes => dev.fill_debug_boxes = !dev.fill_debug_boxes,
        D::MicroGrid => dev.show_micro_grid = !dev.show_micro_grid,
        D::CameraFrame => dev.show_camera_frame = !dev.show_camera_frame,
        D::OverviewCamera => dev.overview_camera = !dev.overview_camera,
        D::DebugViewMode => {
            dev.debug_view_mode = if dir < 0 {
                dev.debug_view_mode.prev()
            } else {
                dev.debug_view_mode.next()
            };
        }
        D::DebugArtMode => {
            dev.debug_art_mode = if dir < 0 {
                dev.debug_art_mode.prev()
            } else {
                dev.debug_art_mode.next()
            };
        }
        D::PlayerBodyProfile => {
            dev.player_body_profile = if dir < 0 {
                dev.player_body_profile.prev()
            } else {
                dev.player_body_profile.next()
            };
        }
        D::MovementProfile => {
            dev.movement_profile = if dir < 0 {
                dev.movement_profile.prev()
            } else {
                dev.movement_profile.next()
            };
        }
        // LDtk auto-reload — F12, on `LdtkHotReloadState` (mirrors the pause
        // menu's `SettingsItem::LdtkAutoApply` arm, including the status line).
        D::LdtkAutoApply => {
            let r = &mut *ctx.ldtk_reload;
            r.auto_apply = !r.auto_apply;
            r.last_status = format!(
                "LDtk auto-apply {}",
                if r.auto_apply { "enabled" } else { "disabled" }
            );
        }
        // Cycle the menu frontend — the in-menu equivalent of the `\` hotkey
        // (`toggle_inventory_backend`). Only two states, so direction is moot; flip.
        D::MenuBackend => {
            let next = (*ctx.backend).next();
            *ctx.backend = next;
        }
        // Portal FX: cycle the compiled-in portal transit visuals on the
        // presentation crate's selection resource (absent under fixtures /
        // non-portal builds: no-op).
        D::PortalEffect => {
            #[cfg(feature = "portal_render")]
            if let Some(selection) = ctx.portal_effect {
                selection.cycle(dir);
            }
            #[cfg(not(feature = "portal_render"))]
            let _ = dir;
        }
    }
}

/// Spawn the readability dim-scrim node (full-screen, starts fully transparent).
///
/// The scrim DIMS THE WORLD, so it must render BEHIND the order-8 cube. Since the
/// default UI camera is now the order-9 [`FrontHudCamera`] (which draws in front of
/// the cube), the scrim is explicitly retargeted onto the order-0 main camera via
/// [`retarget_kaleidoscope_scrim`] (the `MainCameraEntity` resource isn't guaranteed to
/// exist yet at this Startup point, so the target is attached from an Update guard).
/// [`fade_kaleidoscope_scrim`] drives its alpha.
fn spawn_kaleidoscope_scrim(mut commands: Commands) {
    commands.spawn((
        KaleidoscopeScrim,
        Name::new("Cube readability scrim"),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        // Never eat clicks meant for the world/cube; purely a visual dimmer.
        GlobalZIndex(-1),
        Pickable::IGNORE,
    ));
}

/// Retarget the dim-scrim onto the order-0 main camera so it renders BEHIND the cube.
///
/// The default UI camera is the order-9 front HUD camera (so the HUD draws in front
/// of the cube); without this retarget the scrim would inherit that default and dim
/// the cube itself. Runs once, as soon as both the scrim and the `MainCameraEntity`
/// resource exist (Startup ordering between them is not guaranteed, so this Update
/// guard does it on the first frame both are present). `Option<Res<_>>` keeps it
/// B0002-safe and never panics on an uninserted resource.
fn retarget_kaleidoscope_scrim(
    mut commands: Commands,
    main_camera: Option<Res<ambition_sandbox::runtime::camera_layers::MainCameraEntity>>,
    scrim: Query<Entity, (With<KaleidoscopeScrim>, Without<UiTargetCamera>)>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let Some(main_camera) = main_camera else {
        return;
    };
    let mut any = false;
    for entity in &scrim {
        commands
            .entity(entity)
            .insert(UiTargetCamera(main_camera.0));
        any = true;
    }
    if any {
        *done = true;
    }
}

/// Fade the dim-scrim's alpha with the cube's eased open `amount`, so the world
/// dims in/out exactly with the fold. Fully transparent when the cube is shut.
fn fade_kaleidoscope_scrim(
    open_state: Res<ambition_menu::kaleidoscope::KaleidoscopeOpenState>,
    mut scrim: Query<&mut BackgroundColor, With<KaleidoscopeScrim>>,
) {
    let alpha = open_state.amount.clamp(0.0, 1.0) * SCRIM_PEAK_ALPHA;
    for mut bg in &mut scrim {
        bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
    }
}

/// Directional focus navigation for the cube (keyboard / gamepad), porting the
/// demo's `MockDemo::move_spatial` (`crates/ambition_mock_demo/src/app/state.rs`).
/// The cursor lives on the [`KaleidoscopeCursor`] resource as a [`MenuFocus`], and the
/// 6×4 item grid is flanked by two edge (page-turn) buttons. The exact rules
/// (user spec) honoured here:
///
/// 1. From a side arrow, moving toward centre lands on the nearest ITEM in the
///    adjacent column — not across the screen to the other arrow.
/// 2. UP/DOWN never reach a side arrow (vertical stays within the item columns).
/// 3. On a side arrow, moving further outward ROTATES to that page (same as a
///    click).
/// 4. From the leftmost / rightmost column, LEFT/RIGHT moves onto the arrow.
///
/// `select` on an item dispatches its `MenuPageAction`; `select` on an arrow turns the
/// page; `back` closes the menu. The republish runs after this in the chain.
#[allow(clippy::too_many_arguments)]
fn kaleidoscope_focus_nav(
    menu: Res<MenuControlFrame>,
    mut cursor: ResMut<KaleidoscopeCursor>,
    mut system_nav: ResMut<KaleidoscopeSystemNav>,
    // Features C/D: keyboard navigation CLEARS the explicit scroll override so the
    // System window resumes following the selection cursor (the wheel/drag set it).
    mut scroll: ResMut<KaleidoscopeScroll>,
    mut pages: ResMut<ActiveMenuPages<MenuPage, MenuPageAction>>,
    // Single mutable access to the overlay state — also read `.visible` from it (a
    // separate `Res<InventoryUiState>` would be a B0002 conflict with this `ResMut`).
    mut overlay: ResMut<ambition_sandbox::inventory::InventoryUiState>,
    // A close-via-action (e.g. Reset Sandbox) must restore `GameMode::Playing` exactly
    // like the canonical Esc-close — so thread the game mode through to
    // `close_kaleidoscope_menu` instead of bare `overlay.visible = false`. Bundled into
    // one `SystemParam` to stay under Bevy's 16-param ceiling.
    mut mode_io: GameModeIo,
    mut owned: ResMut<OwnedItems>,
    mut settings: ResMut<UserSettings>,
    mut commands: Commands,
    mut players: MenuEffectPlayers,
    mut mana_q: MenuEffectManaQuery,
    mut heals: MessageWriter<PlayerHealRequested>,
    mut sfx: MessageWriter<SfxMessage>,
    mut system: SystemMenuParams,
) {
    // Read the backend from `system` (the bundle owns it); a separate `Res` here
    // would be a B0002 conflict with that `ResMut`.
    if system.backend() != InventoryUiBackend::LunexKaleidoscope || !overlay.visible {
        return;
    }
    // The Esc/pause toggle (`menu.start`) is owned ENTIRELY by
    // `kaleidoscope_menu_open_routing` (close the cube / drill out of a System category /
    // restore GameMode). Esc co-fires `menu.back`, and this nav system (and the
    // `system_focus_nav` it calls) closes on `menu.back` below — so without bailing
    // here a single Esc would CLOSE the cube HERE and then `kaleidoscope_menu_open_routing`
    // would see `!visible` and RE-OPEN it (the Esc-Esc reopen bug). The router was
    // meant to consume `menu.back` before this system, but the chain order is no
    // longer guaranteed once these systems join multiple sets, so make the router the
    // SOLE Esc handler order-independently by bailing on `menu.start` here.
    if menu.start {
        return;
    }
    let Some(active_page) = pages.active else {
        return;
    };

    // Remember the focus we start the frame on. A cursor MOVE (focus actually
    // changes) plays `UI_MENU_MOVE` exactly once at the end of this system — NOT on
    // the per-frame rebuild churn (this only fires when keyboard/gamepad nav lands on
    // a different control). Page turns / selects emit their own distinct sounds, so
    // we suppress the move sound when the page changed this frame.
    let focus_before = cursor.focus;
    let page_before = pages.active;

    // Directional intent (one step; the menu frame already debounces repeat).
    let dx = (menu.right as i32) - (menu.left as i32);
    let dy = (menu.down as i32) - (menu.up as i32);

    // Fix 2: the L/R shoulder bumpers turn the page DIRECTLY (same target as the
    // on-screen L/R edge buttons), independent of the arrow/d-pad item cursor. Left
    // bumper rotates to the viewer-left page, right bumper to the viewer-right page.
    // Handled before the per-face nav so a bumper press always rotates regardless of
    // where the item cursor sits. The cursor lands on the new page's back-edge button.
    let bump = (menu.page_right as i32) - (menu.page_left as i32);
    if bump < 0 {
        turn_page_seeded(
            &mut pages,
            &mut cursor,
            active_page.on_viewer_left(),
            &mut sfx,
        );
        return;
    } else if bump > 0 {
        turn_page_seeded(
            &mut pages,
            &mut cursor,
            active_page.on_viewer_right(),
            &mut sfx,
        );
        return;
    }

    // The System face is an interactive option list: UP/DOWN move the cursor
    // between rows, LEFT/RIGHT at the column edges turn the page (or step a
    // value), and SELECT applies the focused option.
    if active_page == MenuPage::System {
        // Features C/D: a keyboard move/select takes the selection cursor back over
        // from the wheel/scrollbar — drop any explicit scroll override so the window
        // snaps to follow the cursor again.
        if dx != 0 || dy != 0 || menu.select {
            scroll.system_window_start = None;
        }
        system_focus_nav(
            &menu,
            dx,
            dy,
            &mut cursor,
            &mut system_nav,
            &mut pages,
            &mut overlay,
            mode_io.state.get(),
            &mut mode_io.next,
            &mut settings,
            active_page,
            &mut owned,
            &mut commands,
            &mut players,
            &mut mana_q,
            &mut heals,
            &mut sfx,
            &mut system,
            // The cube turns its face on LEFT/RIGHT at the row edges.
            true,
        );
        return;
    }

    // Other non-items faces (Map / Quest placeholders) respond to horizontal page
    // turns; arrows rotate, landing the cursor on the new page's back-edge button
    // (Fix 1). The L/R bumpers (Fix 2) are already handled above for every face.
    if active_page != MenuPage::Items {
        // Placeholder faces (Map / Quest) have only the two edge buttons and no centre
        // content. LEFT/RIGHT move BETWEEN the edges when stepping INWARD; only stepping
        // OUTWARD past an edge rotates the page — the same arrow/edge rule as the items
        // face, just with nothing in the middle. (Was: any L/R rotated immediately, so
        // right-from-the-left-edge jumped to the next page instead of the right edge.)
        if dx != 0 {
            match cursor.focus {
                MenuFocus::EdgeLeft if dx > 0 => cursor.mark_keyboard(MenuFocus::EdgeRight),
                MenuFocus::EdgeLeft => turn_page_seeded(
                    &mut pages,
                    &mut cursor,
                    active_page.on_viewer_left(),
                    &mut sfx,
                ),
                MenuFocus::EdgeRight if dx < 0 => cursor.mark_keyboard(MenuFocus::EdgeLeft),
                MenuFocus::EdgeRight => turn_page_seeded(
                    &mut pages,
                    &mut cursor,
                    active_page.on_viewer_right(),
                    &mut sfx,
                ),
                // Cursor not yet on an edge — seed onto the edge for the pressed direction.
                _ => cursor.mark_keyboard(if dx < 0 {
                    MenuFocus::EdgeLeft
                } else {
                    MenuFocus::EdgeRight
                }),
            }
        }
        if menu.select {
            // The only selectable controls on a placeholder are the edge buttons.
            match cursor.focus {
                MenuFocus::EdgeLeft => turn_page_seeded(
                    &mut pages,
                    &mut cursor,
                    active_page.on_viewer_left(),
                    &mut sfx,
                ),
                MenuFocus::EdgeRight => turn_page_seeded(
                    &mut pages,
                    &mut cursor,
                    active_page.on_viewer_right(),
                    &mut sfx,
                ),
                _ => {}
            }
        }
        if menu.back {
            play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_CLOSE);
            overlay.visible = false;
        }
        emit_move_sfx(
            &mut sfx,
            focus_before,
            cursor.focus,
            page_before,
            pages.active,
        );
        return;
    }

    if dx != 0 || dy != 0 {
        match move_spatial(cursor.focus, dx, dy, active_page) {
            SpatialMove::Focus(next) => cursor.mark_keyboard(next),
            SpatialMove::TurnLeft => {
                turn_page(&mut pages, active_page.on_viewer_left(), &mut sfx);
                // Land the cursor on the new face's right arrow (so pressing back
                // toward centre re-enters the grid) — demo's turn_page_from_edge.
                cursor.mark_keyboard(MenuFocus::EdgeRight);
            }
            SpatialMove::TurnRight => {
                turn_page(&mut pages, active_page.on_viewer_right(), &mut sfx);
                cursor.mark_keyboard(MenuFocus::EdgeLeft);
            }
        }
    }

    if menu.back {
        play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_CLOSE);
        overlay.visible = false;
        return;
    }

    if menu.select {
        let action = match cursor.focus {
            MenuFocus::EdgeLeft => Some(MenuPageAction::ChangePage(active_page.on_viewer_left())),
            MenuFocus::EdgeRight => Some(MenuPageAction::ChangePage(active_page.on_viewer_right())),
            MenuFocus::Item(idx) => owned_item_action(&owned, idx),
            // System focus is handled by the System branch above; never reached here.
            MenuFocus::System(_) => None,
        };
        if let Some(action) = action {
            let mut close_menu = false;
            crate::menu::dispatch::dispatch_menu_action(
                action,
                &mut pages,
                &mut system_nav,
                &mut cursor,
                &mut owned,
                &mut settings,
                &mut close_menu,
                &mut commands,
                &mut players,
                &mut mana_q,
                &mut heals,
                &mut sfx,
                &mut system,
            );
            if close_menu {
                // A close-via-action must unpause exactly like the canonical Esc-close.
                close_kaleidoscope_menu(&mut overlay, mode_io.state.get(), &mut mode_io.next);
            }
        } else {
            // Selecting an empty / unowned item slot is a no-op: error feedback.
            play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_ERROR);
        }
    }

    emit_move_sfx(
        &mut sfx,
        focus_before,
        cursor.focus,
        page_before,
        pages.active,
    );
}

/// Emit `UI_MENU_MOVE` ONCE when the cursor's focus actually changed this frame and
/// the page did NOT turn (a page turn plays its own directional rotate sound, and a
/// select that lands on a new control already played accept/equip/etc.). This is the
/// single gate that keeps the per-frame republish churn from spamming the move sound:
/// it compares the pre-frame focus to the post-frame focus, not "did a system run".
fn emit_move_sfx(
    sfx: &mut MessageWriter<SfxMessage>,
    focus_before: MenuFocus,
    focus_after: MenuFocus,
    page_before: Option<MenuPage>,
    page_after: Option<MenuPage>,
) {
    if page_before == page_after && focus_before != focus_after {
        play_ui(sfx, ambition_sfx::ids::UI_MENU_MOVE);
    }
}

/// Directional navigation + select for the System face. UP/DOWN move the cursor
/// over the live row list; LEFT/RIGHT moves between the edge buttons and the row
/// list, while value rows also respond to LEFT/RIGHT to step. `back` closes the
/// menu. Mutations go through [`apply_system_option`] so persistence stays in one
/// place.
#[allow(clippy::too_many_arguments)]
pub(crate) fn system_focus_nav(
    menu: &MenuControlFrame,
    dx: i32,
    dy: i32,
    cursor: &mut KaleidoscopeCursor,
    system_nav: &mut KaleidoscopeSystemNav,
    pages: &mut ActiveMenuPages<MenuPage, MenuPageAction>,
    overlay: &mut ambition_sandbox::inventory::InventoryUiState,
    mode: &ambition_sandbox::runtime::game_mode::GameMode,
    next_mode: &mut NextState<ambition_sandbox::runtime::game_mode::GameMode>,
    settings: &mut UserSettings,
    active_page: MenuPage,
    owned: &mut OwnedItems,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers,
    mana_q: &mut MenuEffectManaQuery,
    heals: &mut MessageWriter<PlayerHealRequested>,
    sfx: &mut MessageWriter<SfxMessage>,
    system: &mut SystemMenuParams,
    // The cube turns its face when LEFT/RIGHT walks off the row list onto the
    // page-turn edge buttons; the flat Grid switches pages with its TAB BAR, never
    // by edge arrows, so it passes `false` — System-row LEFT/RIGHT then only steps
    // value rows and is otherwise inert (it can never reach an edge or `turn_page`,
    // which used to leak the cube's rotate-SFX + a one-frame face flip into Grid mode).
    allow_page_turn: bool,
) {
    let focus_before = cursor.focus;
    let page_before = pages.active;
    // The rows shown for the current drill-down state: the SYSTEM entry list at the
    // top level, or the open entry's screen rows + a Back row. Built from the live
    // model so radio/dev/language rows are enumerated correctly.
    let model = system.model(settings);
    let rows = system_rows(&model, system_nav.open_entry);
    let count = rows.len().max(1) as i32;
    // Normalise the cursor onto a System row (it may arrive as an items/edge focus
    // after a page turn).
    let mut row = match cursor.focus {
        MenuFocus::System(idx) => (idx as i32).min(count - 1),
        _ => 0,
    };

    if dy != 0 {
        row = (row + dy).clamp(0, count - 1);
        cursor.mark_keyboard(MenuFocus::System(row as usize));
    }

    let current = rows[row.max(0).min(count - 1) as usize];

    if dx != 0 {
        match cursor.focus {
            MenuFocus::EdgeLeft => {
                if dx > 0 || !allow_page_turn {
                    // Move inward from the page-turn button into the row list. With
                    // page-turns disabled (Grid) an edge focus is never reachable, but
                    // normalise defensively to a row rather than rotating.
                    cursor.mark_keyboard(MenuFocus::System(0));
                } else {
                    // Moving further outward from the edge still rotates the cube.
                    turn_page(pages, active_page.on_viewer_left(), sfx);
                    cursor.mark_keyboard(MenuFocus::System(0));
                }
            }
            MenuFocus::EdgeRight => {
                if dx < 0 || !allow_page_turn {
                    cursor.mark_keyboard(MenuFocus::System(0));
                } else {
                    turn_page(pages, active_page.on_viewer_right(), sfx);
                    cursor.mark_keyboard(MenuFocus::System(0));
                }
            }
            MenuFocus::System(_) | MenuFocus::Item(_) => {
                // LEFT/RIGHT step value rows in place (settings cycles/sliders, dev
                // cycles); otherwise use the horizontal affordance to move onto the
                // edge buttons (cube only — the Grid switches pages via its tab bar,
                // so a non-value step there is simply inert).
                let stepped = match current {
                    SystemRow::Setting(o) if is_value_setting(o, settings) => {
                        apply_system_option_step(o, dx, settings, sfx);
                        true
                    }
                    SystemRow::Option(o) => {
                        if let Some(id) = system.step_option(o, dx) {
                            play_ui(sfx, id);
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                };
                if !stepped && allow_page_turn {
                    if dx < 0 {
                        cursor.mark_keyboard(MenuFocus::EdgeLeft);
                    } else {
                        cursor.mark_keyboard(MenuFocus::EdgeRight);
                    }
                }
            }
        }
    }

    if menu.back {
        // Inside an entry, Back drills OUT to the entry list; at the top level Back
        // closes the menu (matching the items face).
        if system_nav.open_entry.is_some() {
            play_ui(sfx, ambition_sfx::ids::UI_MENU_BACK);
            close_system_entry(system_nav, cursor);
        } else {
            play_ui(sfx, ambition_sfx::ids::UI_MENU_CLOSE);
            overlay.visible = false;
        }
        return;
    }

    if menu.select {
        if let Some(action) = system_row_action_for(&model, current) {
            let mut close_menu = false;
            crate::menu::dispatch::dispatch_menu_action(
                action,
                pages,
                system_nav,
                cursor,
                owned,
                settings,
                &mut close_menu,
                commands,
                players,
                mana_q,
                heals,
                sfx,
                system,
            );
            if close_menu {
                // A close-via-action must unpause exactly like the canonical Esc-close.
                close_kaleidoscope_menu(overlay, mode, next_mode);
            }
        }
        return;
    }

    emit_move_sfx(sfx, focus_before, cursor.focus, page_before, pages.active);
}

/// True for SETTINGS rows whose value steps with LEFT/RIGHT in place (cycles +
/// sliders). Toggles ignore horizontal stepping and instead use the horizontal
/// affordance to move onto the edge buttons. Read from the shared settings IR so
/// the cube can never disagree with the option's real kind.
fn is_value_setting(option: SettingsOptionId, settings: &UserSettings) -> bool {
    settings_menu_model(settings)
        .categories
        .iter()
        .flat_map(|c| c.options.iter())
        .find(|o| o.id == option)
        .map(|o| {
            matches!(
                o.kind,
                SettingsOptionKind::Cycle { .. } | SettingsOptionKind::Slider { .. }
            )
        })
        .unwrap_or(false)
}

/// The `MenuPageAction` a System row dispatches on select.
pub(crate) fn system_row_action_for(
    model: &SystemMenuModel,
    row: SystemRow,
) -> Option<MenuPageAction> {
    match row {
        SystemRow::Entry(id) => match model.entry(id).map(|e| &e.target) {
            Some(ambition_sandbox::persistence::settings::SystemMenuTarget::Action(action)) => {
                Some(MenuPageAction::SystemAction(*action))
            }
            _ => Some(MenuPageAction::OpenSystemEntry(id)),
        },
        SystemRow::Setting(o) => Some(MenuPageAction::System(o)),
        SystemRow::Option(o) => Some(MenuPageAction::SystemOption(o)),
        SystemRow::Back => Some(MenuPageAction::CloseSystemEntry),
    }
}

/// Drill OUT of an open System entry back to the entry list, resetting the cursor
/// to the first row so the highlight lands sensibly.
pub(crate) fn close_system_entry(
    system_nav: &mut KaleidoscopeSystemNav,
    cursor: &mut KaleidoscopeCursor,
) {
    system_nav.open_entry = None;
    cursor.mark_keyboard(MenuFocus::System(0));
}

/// Apply a signed LEFT/RIGHT step to a value-style System option (slider up/down,
/// cycle prev/next) through the shared settings IR. Toggle/close rows ignore
/// stepping (they only respond to SELECT). Persistence is automatic via
/// `UserSettings` change detection.
fn apply_system_option_step(
    option: SettingsOptionId,
    dx: i32,
    settings: &mut UserSettings,
    sfx: &mut MessageWriter<SfxMessage>,
) {
    apply_settings_option(option, dx, settings);
    play_ui(sfx, ambition_sfx::ids::UI_SLIDER_TICK);
}

/// Outcome of a spatial cursor move on the items page.
enum SpatialMove {
    /// The cursor moves to a new focus (item or arrow) on the same page.
    Focus(MenuFocus),
    /// The cursor was on the left arrow and pressed further left → rotate left.
    TurnLeft,
    /// The cursor was on the right arrow and pressed further right → rotate right.
    TurnRight,
}

/// Port of the demo's `MockDemo::move_spatial` for the items grid + flanking
/// arrows. Pure (no ECS) so it's unit-testable and easy to reason about. See
/// [`kaleidoscope_focus_nav`] for the rule list.
fn move_spatial(focus: MenuFocus, dx: i32, dy: i32, _page: MenuPage) -> SpatialMove {
    let cols = ITEM_GRID_COLS as i32;
    let rows = ITEM_GRID_ROWS as i32;

    // Rule 3: on an arrow, moving further OUTWARD rotates the page; UP/DOWN never
    // reach/leave an arrow (rule 2); moving INWARD enters the adjacent column.
    match focus {
        MenuFocus::EdgeLeft => {
            if dx < 0 {
                return SpatialMove::TurnLeft;
            }
            if dx > 0 {
                // Rule 1: enter the LEFTMOST item column (col 0), keep the row band.
                return SpatialMove::Focus(MenuFocus::Item(0));
            }
            // Up/Down on an arrow: stay put (rule 2).
            return SpatialMove::Focus(focus);
        }
        MenuFocus::EdgeRight => {
            if dx > 0 {
                return SpatialMove::TurnRight;
            }
            if dx < 0 {
                // Rule 1: enter the RIGHTMOST item column.
                return SpatialMove::Focus(MenuFocus::Item((cols - 1) as usize));
            }
            return SpatialMove::Focus(focus);
        }
        MenuFocus::Item(idx) => {
            let idx = idx as i32;
            let row = idx / cols;
            let col = idx % cols;
            // Rule 4: stepping off the left/right column lands on the arrow.
            if dx < 0 && col == 0 {
                return SpatialMove::Focus(MenuFocus::EdgeLeft);
            }
            if dx > 0 && col == cols - 1 {
                return SpatialMove::Focus(MenuFocus::EdgeRight);
            }
            // Rule 2: UP/DOWN stays within the columns (never reaches an arrow).
            let next_col = (col + dx).clamp(0, cols - 1);
            let next_row = (row + dy).clamp(0, rows - 1);
            SpatialMove::Focus(MenuFocus::Item((next_row * cols + next_col) as usize))
        }
        // `move_spatial` is only invoked on the Items face; a System focus here
        // would be a logic error — re-enter the grid at slot 0 to stay safe.
        MenuFocus::System(_) => SpatialMove::Focus(MenuFocus::Item(0)),
    }
}

/// The `MenuPageAction` for an owned item slot, or `None` if the slot is empty/unowned
/// (so confirming an empty cell is a no-op, matching the grid backend).
pub(crate) fn owned_item_action(owned: &OwnedItems, idx: usize) -> Option<MenuPageAction> {
    let item = Item::from_index(idx)?;
    if !owned.has(item) {
        return None;
    }
    Some(if item.held_item_id().is_some() {
        MenuPageAction::Equip(item)
    } else {
        MenuPageAction::Use(item)
    })
}

/// The edge-button focus on `to` that turns BACK toward `from` (Fix 1). After a page
/// turn the cursor lands here, so the arriving control is highlighted and an immediate
/// rotate/select returns to the page we came from. On `to`, the LEFT edge button
/// targets `to.on_viewer_left()` and the RIGHT targets `to.on_viewer_right()`; we pick
/// whichever points back at `from`. When `from` is unknown (first open) we default to
/// the left edge button so there is always a highlighted control.
pub(crate) fn back_edge_focus(from: Option<MenuPage>, to: MenuPage) -> MenuFocus {
    match from {
        Some(from) if to.on_viewer_right() == from => MenuFocus::EdgeRight,
        Some(from) if to.on_viewer_left() == from => MenuFocus::EdgeLeft,
        _ => MenuFocus::EdgeLeft,
    }
}

/// Set the active page (the lib rotates that face to the camera), landing the cursor
/// on the new page's back-edge button (Fix 1) via [`back_edge_focus`].
fn turn_page_seeded(
    pages: &mut ActiveMenuPages<MenuPage, MenuPageAction>,
    cursor: &mut KaleidoscopeCursor,
    page: MenuPage,
    sfx: &mut MessageWriter<SfxMessage>,
) {
    let from = pages.active;
    turn_page(pages, page, sfx);
    cursor.mark_keyboard(back_edge_focus(from, page));
}

/// Set the active page (the lib rotates that face to the camera). Emits the
/// directional rotate SFX only when the page ACTUALLY changes (so re-selecting the
/// current page is silent).
fn turn_page(
    pages: &mut ActiveMenuPages<MenuPage, MenuPageAction>,
    page: MenuPage,
    sfx: &mut MessageWriter<SfxMessage>,
) {
    if pages.active != Some(page) {
        play_ui(sfx, rotate_sfx(pages.active, page));
        pages.active = Some(page);
        info!("cube page \u{2192} {:?}", page);
    }
}

/// The directional page-turn sound for a rotation `from` → `to`: rotating to the
/// page that sits on the viewer-LEFT of `from` plays the left rotate, otherwise the
/// right rotate. When `from` is unknown (first publish) defaults to the right rotate.
pub(crate) fn rotate_sfx(from: Option<MenuPage>, to: MenuPage) -> ambition_sfx::SfxId {
    match from {
        Some(from) if from.on_viewer_left() == to => ambition_sfx::ids::UI_MENU_ROTATE,
        _ => ambition_sfx::ids::UI_MENU_ROTATE,
    }
}

/// Map a control's `MenuPageAction` back to the cursor focus it represents, so pointer
/// hover/click and the keyboard cursor share one model. `Equip`/`Use` carry the
/// item (→ its slot); `ChangePage` is an edge arrow — left vs right is decided by
/// whether its target is the active page's viewer-left neighbour.
pub(crate) fn focus_for_action(
    action: MenuPageAction,
    active_page: MenuPage,
    model: &SystemMenuModel,
    open_entry: Option<SystemMenuEntryId>,
) -> MenuFocus {
    // System rows are positional: the focus index is the action's row in the
    // currently-displayed System row list (the entry list, or an open entry's
    // screen rows + Back), so hover/click and the keyboard cursor agree on the row.
    let system_row = |want: SystemRow| {
        let idx = system_rows(model, open_entry)
            .iter()
            .position(|r| *r == want)
            .unwrap_or(0);
        MenuFocus::System(idx)
    };
    match action {
        MenuPageAction::Equip(item) | MenuPageAction::Use(item) => MenuFocus::Item(item.index()),
        MenuPageAction::ChangePage(target) => {
            if target == active_page.on_viewer_left() {
                MenuFocus::EdgeLeft
            } else {
                MenuFocus::EdgeRight
            }
        }
        MenuPageAction::System(option) => system_row(SystemRow::Setting(option)),
        // Fix 2: a ◀ / ▶ step zone lands the cursor on its parent value row.
        MenuPageAction::SystemStep(option, _) => system_row(SystemRow::Setting(option)),
        MenuPageAction::SystemOption(opt) => system_row(SystemRow::Option(opt)),
        MenuPageAction::SystemAction(_) => {
            // An Action entry sits at top level; find its entry row.
            let entry = match action {
                MenuPageAction::SystemAction(SystemMenuAction::ResetSandbox) => {
                    SystemMenuEntryId::ResetSandbox
                }
                MenuPageAction::SystemAction(SystemMenuAction::ResetAllSettings) => {
                    SystemMenuEntryId::ResetAllSettings
                }
                MenuPageAction::SystemAction(SystemMenuAction::Quit) => SystemMenuEntryId::Quit,
                _ => return MenuFocus::System(0),
            };
            system_row(SystemRow::Entry(entry))
        }
        MenuPageAction::OpenSystemEntry(entry) => system_row(SystemRow::Entry(entry)),
        MenuPageAction::CloseSystemEntry => system_row(SystemRow::Back),
    }
}

/// Feature E: record the start of a pointer press on a cube control so a
/// press-then-drag-away can be CANCELLED (no activation). Stores the pressed entity
/// + the press origin; `kaleidoscope_pointer_move` marks it cancelled once the
/// pointer drags past the tap threshold, and `kaleidoscope_pointer_release` honours
/// that. Mouse OR touch (same `Pointer<Press>` path).
pub(crate) fn kaleidoscope_pointer_press(
    press: On<Pointer<Press>>,
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<ambition_sandbox::inventory::InventoryUiState>>,
    controls: Query<&AmbitionMenuControl<MenuPageAction>>,
    mut state: ResMut<KaleidoscopePointerPress>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    if backend.effective() != InventoryUiBackend::LunexKaleidoscope || !open {
        return;
    }
    // Only arm the tap-guard for real controls (so a press on decoration is a no-op).
    if let Ok(control) = controls.get(press.entity) {
        state.entity = Some(press.entity);
        // Capture the action NOW so RELEASE can dispatch it entity-independently
        // (survives a face rebuild between press and release).
        state.action = control.action;
        state.origin = press.pointer_location.position;
        state.cancelled = false;
    }
}

/// Pointer motion (mouse/touch) over a cube control: move the focus cursor to it.
/// We listen to `Pointer<Move>` instead of `Pointer<Over>` so a menu that opens
/// under a parked mouse does not immediately select whatever is already under the
/// cursor. A real move is required before hover can take ownership.
///
/// Two guards (both essential), mirroring the grid's `MenuFocusState`:
///
/// 1. **Semantic dedup.** A moving pointer can emit several events while it stays
///    over the same control. We compare the hovered focus against `last_pointer_focus`
///    and bail when unchanged, so the cursor only reacts once per logical focus.
/// 2. **Pointer-vs-keyboard ownership.** The pointer only re-claims the cursor when
///    it moves onto a genuinely different control. This fixes "can't move away from
///    the hovered option."
fn kaleidoscope_pointer_move(
    move_: On<Pointer<Move>>,
    controls: Query<&AmbitionMenuControl<MenuPageAction>>,
    pages: Res<ActiveMenuPages<MenuPage, MenuPageAction>>,
    system_nav: Res<KaleidoscopeSystemNav>,
    settings: Res<UserSettings>,
    active_input: Res<ambition_sandbox::input::ActiveInputKind>,
    snapshot: SystemMenuSnapshotParams,
    mut cursor: ResMut<KaleidoscopeCursor>,
    // Feature E: a press in flight is cancelled (no click) once the pointer drags
    // past the tap threshold from its press origin.
    mut press: ResMut<KaleidoscopePointerPress>,
    mut sfx: MessageWriter<SfxMessage>,
) {
    // Feature E: if a press is active and the pointer has now travelled past the tap
    // threshold, this is a DRAG — mark the press cancelled so the eventual click does
    // not activate the control. (This drag-cancel runs regardless of the active-input
    // gate below: a touch/pen drag must still cancel a tap.)
    if press.entity.is_some()
        && !press.cancelled
        && move_.pointer_location.position.distance(press.origin) > KALEIDOSCOPE_TAP_DRAG_THRESHOLD
    {
        press.cancelled = true;
    }
    // Hover-select is gated on a GENUINE mouse being the active source. A cube
    // republish respawns controls under a stationary mouse and fires `Pointer<Move>`
    // for the new control; without this gate the cursor snaps back to the mouse on
    // every keyboard/gamepad/touch directional move. A real mouse move sets
    // active=Mouse (see `update_active_input_kind`) so hovering still works; clicks
    // are unaffected (separate press/release observers).
    if *active_input != ambition_sandbox::input::ActiveInputKind::Mouse {
        return;
    }
    let Some(active_page) = pages.active else {
        return;
    };
    if let Ok(control) = controls.get(move_.entity) {
        if let Some(action) = control.action {
            let model = SystemMenuModel::build(
                &settings,
                &snapshot.radio_snapshot(),
                &snapshot.dev_snapshot(),
            );
            let next = focus_for_action(action, active_page, &model, system_nav.open_entry);
            // The pointer hasn't moved to a new control (same logical focus as the
            // previous move event): do nothing.
            if cursor.last_pointer_focus == Some(next) {
                return;
            }
            cursor.last_pointer_focus = Some(next);
            if cursor.focus != next {
                cursor.focus = next;
                cursor.owner = FocusSource::Pointer;
                // The move landed on a genuinely different control: play the move
                // sound, matching the keyboard nav path.
                play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_MOVE);
            }
        }
    }
}

/// Pointer release (mouse/touch) dispatches the action armed at press time.
///
/// Cube controls can be despawned/rebuilt between press and release, so dispatch is
/// entity-independent: store the action on `Pointer<Press>` and consume it on
/// release. Drag cancellation still wins when movement exceeds the tap threshold.
#[allow(clippy::too_many_arguments)]
pub(crate) fn kaleidoscope_pointer_release(
    _release: On<Pointer<Release>>,
    mut ui_state: Option<ResMut<ambition_sandbox::inventory::InventoryUiState>>,
    // A close-via-action (e.g. Reset Sandbox) must restore `GameMode::Playing` exactly
    // like the canonical Esc-close — so route the close through `close_kaleidoscope_menu`.
    // Bundled into one `SystemParam` to stay under Bevy's 16-param ceiling.
    mut mode_io: GameModeIo,
    mut pages: ResMut<ActiveMenuPages<MenuPage, MenuPageAction>>,
    mut cursor: ResMut<KaleidoscopeCursor>,
    mut system_nav: ResMut<KaleidoscopeSystemNav>,
    mut owned: ResMut<OwnedItems>,
    mut settings: ResMut<UserSettings>,
    mut commands: Commands,
    mut players: MenuEffectPlayers,
    mut mana_q: MenuEffectManaQuery,
    mut heals: MessageWriter<PlayerHealRequested>,
    mut sfx: MessageWriter<SfxMessage>,
    mut system: SystemMenuParams,
    // In-flight press; activation uses the action stored at press time.
    mut press: ResMut<KaleidoscopePointerPress>,
) {
    let open = ui_state.as_deref().map(|s| s.visible).unwrap_or(false);
    // Read the backend from `system` (it owns the resource); a separate `Res` here
    // would be a B0002 conflict with that `ResMut`.
    if system.backend() != InventoryUiBackend::LunexKaleidoscope || !open {
        return;
    }
    // Consume the press guard (whatever happens, the next press starts fresh). A
    // release with no armed press, a drag-away cancel, or a press on a control with
    // no action all fall through to "no activation".
    let armed = press.entity.is_some();
    let cancelled = press.cancelled;
    let action = press.action;
    press.entity = None;
    press.action = None;
    press.cancelled = false;
    if !armed || cancelled {
        return;
    }
    let Some(action) = action else {
        return;
    };
    if let Some(active_page) = pages.active {
        let model = system.model(&settings);
        let next = focus_for_action(action, active_page, &model, system_nav.open_entry);
        cursor.focus = next;
        cursor.owner = FocusSource::Pointer;
        cursor.last_pointer_focus = Some(next);
    }
    let mut close_menu = false;
    // Releases route through the SAME `crate::menu::dispatch::dispatch_menu_action` as the keyboard
    // select path, so the action sounds (equip/use/rotate/toggle/...) live in
    // one place and are identical for pointer + keyboard.
    crate::menu::dispatch::dispatch_menu_action(
        action,
        &mut pages,
        &mut system_nav,
        &mut cursor,
        &mut owned,
        &mut settings,
        &mut close_menu,
        &mut commands,
        &mut players,
        &mut mana_q,
        &mut heals,
        &mut sfx,
        &mut system,
    );
    if close_menu {
        if let Some(ui_state) = ui_state.as_deref_mut() {
            // A close-via-action must unpause exactly like the canonical Esc-close.
            close_kaleidoscope_menu(ui_state, mode_io.state.get(), &mut mode_io.next);
        }
    }
}

/// Fix 3: route the game's menu-open inputs to the CUBE when it is the active
/// backend, opening it on the page that matches the requested menu:
///
/// * pause / `Esc` (`menu.start`) → open on [`MenuPage::System`] (replacing the old
///   pause/system menu); pressing it again while the cube is open CLOSES the cube.
/// * inventory key (`menu.inventory`) → open on [`MenuPage::Items`].
/// * map key (`menu.map`) → open on [`MenuPage::Map`].
///
/// Opening pauses the sim (`GameMode::Paused`) and raises `InventoryUiState.visible`,
/// exactly like the inventory open path — which makes the existing pause-menu UI
/// auto-suppress (`Paused && !inventory.visible`). The old `pause_menu_toggle` and
/// `handle_map_menu_hotkeys` are gated to no-op under the Cube backend (see their
/// `kaleidoscope_backend_active` guards), so nothing double-fires the `GameMode` toggle and
/// the map panel never opens behind the cube.
///
/// `Esc`-to-close is owned HERE (not by `kaleidoscope_focus_nav`'s `menu.back`) so the close
/// also restores `GameMode::Playing`; the routing runs before `kaleidoscope_focus_nav`, and
/// consuming the open/close intent keeps the two from fighting over the same frame.
#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
fn kaleidoscope_menu_open_routing(
    mut menu: ResMut<MenuControlFrame>,
    mut overlay: ResMut<ambition_sandbox::inventory::InventoryUiState>,
    mode: Res<State<ambition_sandbox::runtime::game_mode::GameMode>>,
    mut next_mode: ResMut<NextState<ambition_sandbox::runtime::game_mode::GameMode>>,
    mut pages: ResMut<ActiveMenuPages<MenuPage, MenuPageAction>>,
    mut cursor: ResMut<KaleidoscopeCursor>,
    mut system_nav: ResMut<KaleidoscopeSystemNav>,
    mut map: ResMut<ambition_sandbox::menu::map::MapMenuState>,
    mut sfx: MessageWriter<SfxMessage>,
    // Tracks last frame's `menu.start` so we only act on its RISING edge (below).
    mut last_start: Local<bool>,
) {
    use ambition_sandbox::runtime::game_mode::GameMode;

    // pause / Esc: toggle the cube on the System page.
    //
    // Rising-edge debounce: `menu.start` is `just_pressed(Start)`, but it can be
    // observed on MORE THAN ONE consecutive frame (e.g. when the Update schedule
    // ticks more than once per leafwing input update). Without edge-detection a
    // single Esc press would CLOSE the cube on frame N (overlay.visible true→false)
    // and then immediately RE-OPEN it on frame N+1 (start still set, overlay now
    // hidden → the `else` open branch) — the "Esc-Esc reopen" bug. Acting only on
    // the rising edge guarantees one open/close per physical press.
    let start_edge = menu.start && !*last_start;
    *last_start = menu.start;
    if start_edge {
        // Esc binds to BOTH `pause` (→ `menu.start`) AND `MenuBack` (→ `menu.back`),
        // so a single Esc sets both bits. This system OWNS the Esc open/close toggle;
        // consume the duplicate `back` so the later `kaleidoscope_focus_nav` / `system_focus_nav`
        // in the chain can't act on the same Esc (e.g. immediately re-close what we just
        // opened, or drill out of a System category instead of closing). `back` from a
        // NON-Esc source (Backspace / gamepad East) never co-occurs with `start`, so it
        // still reaches the nav systems for its own close / drill-out handling.
        menu.back = false;
        if overlay.visible {
            // Fix 1: while the menu is open, Esc BACKS OUT one level when the cursor
            // is inside a nested System screen (a drilled-in category / Radio /
            // Developer entry, i.e. `open_entry.is_some()`); only at the top level
            // does Esc CLOSE the whole menu. The restructure renamed `open_category`
            // → `open_entry` but this Esc handler never consulted the nesting, so it
            // always closed — the regression. Owning the drill-out HERE (instead of
            // leaving it to `system_focus_nav`'s `menu.back`) is required because we
            // consume the co-firing `menu.back` just above, so the nav systems can't
            // see this Esc at all.
            if system_nav.open_entry.is_some() {
                play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_BACK);
                close_system_entry(&mut system_nav, &mut cursor);
            } else {
                play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_CLOSE);
                close_kaleidoscope_menu(&mut overlay, mode.get(), &mut next_mode);
            }
        } else if matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
            play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_OPEN);
            // SHARED entry→tab mapping: Esc/Start lands on the System face.
            // Keep this mapping local to the backend-agnostic menu vocabulary so
            // the cube can compile without the Bevy-UI/Grid backend feature.
            open_kaleidoscope_menu(
                MenuPage::System,
                &mut overlay,
                mode.get(),
                &mut next_mode,
                &mut pages,
                &mut cursor,
                &mut system_nav,
                &mut map,
            );
        }
        return;
    }

    // Inventory key: shared cube open/close toggle, mirroring the Esc branch.
    if menu.inventory {
        if overlay.visible {
            // Closing: leave the active page alone so the fold-close animation plays
            // out from whatever face was shown (re-seeding to Items here snapped the
            // cube to the Items face mid-close — the "I" close-animation glitch).
            play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_CLOSE);
            close_kaleidoscope_menu(&mut overlay, mode.get(), &mut next_mode);
        } else if matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
            // Opening on the Items page (shared entry→tab mapping) + seed the cursor.
            play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_OPEN);
            open_kaleidoscope_menu(
                MenuPage::Items,
                &mut overlay,
                mode.get(),
                &mut next_mode,
                &mut pages,
                &mut cursor,
                &mut system_nav,
                &mut map,
            );
        }
        return;
    }

    // map key: open on the Map page (suppressing the standalone map panel).
    if menu.map && matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
        let map_page = MenuPage::Map;
        if overlay.visible {
            pages.active = Some(map_page);
            cursor.mark_keyboard(MenuFocus::EdgeLeft);
        } else {
            play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_OPEN);
            open_kaleidoscope_menu(
                map_page,
                &mut overlay,
                mode.get(),
                &mut next_mode,
                &mut pages,
                &mut cursor,
                &mut system_nav,
                &mut map,
            );
        }
    }
}

/// Open the cube overlay on `page`, pausing the sim and seeding the cursor: raise
/// `visible`, switch to
/// `GameMode::Paused` when coming from gameplay, and make sure the standalone map
/// panel stays shut so it can't render behind the cube.
#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
fn open_kaleidoscope_menu(
    page: MenuPage,
    overlay: &mut ambition_sandbox::inventory::InventoryUiState,
    mode: &ambition_sandbox::runtime::game_mode::GameMode,
    next_mode: &mut NextState<ambition_sandbox::runtime::game_mode::GameMode>,
    pages: &mut ActiveMenuPages<MenuPage, MenuPageAction>,
    cursor: &mut KaleidoscopeCursor,
    system_nav: &mut KaleidoscopeSystemNav,
    map: &mut ambition_sandbox::menu::map::MapMenuState,
) {
    use ambition_sandbox::runtime::game_mode::GameMode;
    overlay.visible = true;
    overlay.opened_from_pause = matches!(mode, GameMode::Paused);
    pages.active = Some(page);
    // Seed a sensible cursor for the opening page.
    system_nav.open_entry = None;
    cursor.last_pointer_focus = None;
    cursor.mark_keyboard(match page {
        MenuPage::Items => MenuFocus::Item(0),
        MenuPage::System => MenuFocus::System(0),
        MenuPage::Map | MenuPage::Quest => MenuFocus::EdgeLeft,
    });
    // Never leave the standalone map panel open underneath the cube.
    map.open = false;
    if matches!(mode, GameMode::Playing) {
        next_mode.set(GameMode::Paused);
    }
}

/// Close the cube overlay (Esc while open), restoring `GameMode::Playing` when the
/// cube was opened directly from gameplay (matching `close_grid_menu`). Also used by the
/// close-via-action paths (`kaleidoscope_focus_nav` / `system_focus_nav` /
/// `kaleidoscope_pointer_release`) so an action-triggered close unpauses identically.
fn close_kaleidoscope_menu(
    overlay: &mut ambition_sandbox::inventory::InventoryUiState,
    mode: &ambition_sandbox::runtime::game_mode::GameMode,
    next_mode: &mut NextState<ambition_sandbox::runtime::game_mode::GameMode>,
) {
    use ambition_sandbox::runtime::game_mode::GameMode;
    let opened_from_pause = overlay.opened_from_pause;
    overlay.visible = false;
    if !opened_from_pause && matches!(mode, GameMode::Paused) {
        next_mode.set(GameMode::Playing);
    }
}

/// Dev runtime toggle (#31): `\` flips the inventory frontend between the Bevy-UI
/// grid and the 3D cube. Logs the new backend so it's visible in the console.
fn toggle_inventory_backend(
    keys: Res<ButtonInput<KeyCode>>,
    mut backend: ResMut<InventoryUiBackend>,
) {
    if keys.just_pressed(KeyCode::Backslash) {
        *backend = backend.next();
        info!(
            "inventory backend → {:?}{}",
            backend.effective(),
            backend.unavailable_note()
        );
    }
}

/// Pause-gate the cube: its order-8 `Camera3d` clears the whole screen every frame,
/// so it must be active only while the inventory is open (and the Cube backend is
/// selected). Off otherwise → the lower-order game cameras render normally.
fn gate_kaleidoscope_menu(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<ambition_sandbox::inventory::InventoryUiState>>,
    mut open_state: ResMut<ambition_menu::kaleidoscope::KaleidoscopeOpenState>,
    mut cameras: Query<(
        &mut Camera,
        Has<ambition_menu::kaleidoscope::KaleidoscopePauseCamera>,
    )>,
    mut rings: Query<&mut Visibility, With<ambition_menu::kaleidoscope::MenuRing>>,
    mut last_show: Local<Option<bool>>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    let show = KALEIDOSCOPE_MENU_BACKEND_ENABLED
        && backend.effective() == InventoryUiBackend::LunexKaleidoscope
        && open;
    if *last_show != Some(show) {
        info!(
            "cube gate: show={show} backend={:?} menu_open={open}",
            *backend
        );
        *last_show = Some(show);
    }
    // Drive the lib's open/close fold: it eases `amount` toward this target each
    // frame (see `animate_kaleidoscope_open`). We gate the camera/visibility off the eased
    // AMOUNT (not the binary `show`) so the close-fold animation stays on-screen
    // until the cube has fully folded shut.
    open_state.target = if show { 1.0 } else { 0.0 };
    // Hide the camera/ring once the close-fold has decayed past a sizable cutoff
    // (not a near-zero `0.002`) so the slow fold/scrim TAIL is cut and the menu
    // clears snappily. Combined with the lib's faster close decay
    // (`close_speed_scale`), the scrim (which follows `amount`) reads as a quick
    // fade-out. The cutoff only matters while CLOSING; opening crosses it instantly.
    let shown = open_state.amount > 0.08;
    // Option 1 overlay experiment: toggle ONLY the cube camera and LEAVE the game's
    // 2D camera active, so the live world renders behind the cube (which now clears
    // None). This is the configuration we previously avoided (sole-camera) to dodge
    // the 2D/3D share bug — but that bug's real cause was the camera-drag (now fixed
    // via With<Camera2d>) plus an MSAA mismatch (now matched), so it's worth a try.
    for (mut cam, is_kaleidoscope) in &mut cameras {
        if is_kaleidoscope && cam.is_active != shown {
            cam.is_active = shown;
        }
    }
    let want = if shown {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut vis in &mut rings {
        if *vis != want {
            *vis = want;
        }
    }
}

/// Apply the focus HIGHLIGHT in place: set each live control's [`MenuVisualState`]
/// from the live cursor (the lib's `sync_control_focus_visuals` then recolors it),
/// WITHOUT touching `ActiveMenuPages`. This is what lets a mouse move re-highlight a
/// control without a face rebuild — the rebuild used to despawn the hovered control
/// between a pointer press and release, dropping `Pointer<Click>` (deferred Bug 2).
///
/// A control's focus identity is the inverse of [`focus_for_action`], so the cursor
/// (keyboard OR pointer) and the highlighted control always agree — keyboard select
/// keeps working identically.
fn kaleidoscope_sync_focus_visuals(
    cursor: Res<KaleidoscopeCursor>,
    pages: Res<ActiveMenuPages<MenuPage, MenuPageAction>>,
    system_nav: Res<KaleidoscopeSystemNav>,
    settings: Res<UserSettings>,
    snapshot: SystemMenuSnapshotParams,
    mut controls: Query<(&AmbitionMenuControl<MenuPageAction>, &mut MenuVisualState)>,
) {
    let Some(active_page) = pages.active else {
        return;
    };
    // PERF (2026-06-10): only the System face's controls consult the System row
    // model; `focus_for_action` never touches it for Items/page-turn actions.
    // Building it (which formats every setting value to a string + clones the radio
    // station list) on the Items/Map/Quest pages was wasted work every frame. Build
    // it only when the System face is active; an empty model is fine elsewhere
    // because no System action is ever matched against it there.
    let model = if active_page == MenuPage::System {
        SystemMenuModel::build(
            &settings,
            &snapshot.radio_snapshot(),
            &snapshot.dev_snapshot(),
        )
    } else {
        SystemMenuModel::default()
    };
    for (control, mut vis) in &mut controls {
        let Some(action) = control.action else {
            continue;
        };
        let focus = focus_for_action(action, active_page, &model, system_nav.open_entry);
        let focused = focus == cursor.focus;
        // Change-detection friendly: only write when the flags actually flip, so the
        // lib's `Changed<MenuVisualState>` recolor stays cheap.
        if vis.focused != focused || vis.selected != focused {
            vis.focused = focused;
            vis.selected = focused;
        }
    }
}

/// Fill the detail-panel's dynamic text IN PLACE from the live cursor (Items face
/// right panel + System face bottom panel), writing [`MenuDynamicTextContent`] by
/// slot. The page data itself is cursor-INDEPENDENT, so the cursor-dependent detail
/// text updates without a face rebuild — the lib's `apply_dynamic_text` copies the
/// content into the `Text3d`.
fn kaleidoscope_sync_detail_text(
    owned: Option<Res<OwnedItems>>,
    cursor: Res<KaleidoscopeCursor>,
    pages: Res<ActiveMenuPages<MenuPage, MenuPageAction>>,
    system_nav: Res<KaleidoscopeSystemNav>,
    settings: Res<UserSettings>,
    snapshot: SystemMenuSnapshotParams,
    mut texts: Query<(&MenuDynamicText, &mut MenuDynamicTextContent)>,
) {
    let Some(owned) = owned else {
        return;
    };
    let Some(active_page) = pages.active else {
        return;
    };
    // Build the slot→string map for whichever face's detail panel is live. Only the
    // active page carries dynamic-text slots, so a single map covers the panel.
    let slot_text: Vec<(u32, String)> = match active_page {
        MenuPage::Items => items_detail_slot_text(&owned, owned.equipped(), cursor.focus),
        MenuPage::System => {
            let model = SystemMenuModel::build(
                &settings,
                &snapshot.radio_snapshot(),
                &snapshot.dev_snapshot(),
            );
            let rows = system_rows(&model, system_nav.open_entry);
            let focused = match cursor.focus {
                MenuFocus::System(idx) => idx.min(rows.len().saturating_sub(1)),
                _ => 0,
            };
            system_detail_slot_text(&model, &rows, focused)
        }
        // Placeholder faces (Map / Quest) have no dynamic detail panel.
        _ => Vec::new(),
    };
    for (dynamic, mut content) in &mut texts {
        if let Some((_, text)) = slot_text.iter().find(|(slot, _)| *slot == dynamic.slot) {
            // Change-detection friendly: only rewrite when the string differs.
            if content.0 != *text {
                content.0 = text.clone();
            }
        }
    }
}

/// Republish the cube's faces from our live inventory + the focus cursor (the
/// host-owned data seam — the cube renderer treats `ActiveMenuPages` as read-only).
///
/// Runs after [`kaleidoscope_focus_nav`] in the chain so this frame's cursor move is
/// reflected in the rebuilt page (highlight + detail panel). To avoid an infinite
/// rebuild loop (writing `pages.pages` marks the resource changed), it republishes
/// only when something it depends on actually changed: the inventory, the focus
/// cursor, the active page, the just-opened edge, or the very first publish.
fn republish_kaleidoscope_pages(
    ui_state: Option<Res<ambition_sandbox::inventory::InventoryUiState>>,
    owned: Option<Res<OwnedItems>>,
    // Read-only here. The mutators (`kaleidoscope_focus_nav`, `kaleidoscope_pointer_release`) take
    // `ResMut<UserSettings>` in SEPARATE systems, so this `Res` is not a B0002
    // conflict; `UserSettings` is inserted at startup so the `Res` never panics.
    settings: Res<UserSettings>,
    cursor: Res<KaleidoscopeCursor>,
    // Read-only here; the mutators (`kaleidoscope_focus_nav`, `kaleidoscope_pointer_release`) take
    // `ResMut<KaleidoscopeSystemNav>` in SEPARATE systems/observers, so this `Res` is not a
    // B0002 conflict. Inserted at startup (`init_resource`) so it never panics.
    system_nav: Res<KaleidoscopeSystemNav>,
    // The radio + developer snapshots feed the broadened SYSTEM screens. Read-only
    // here; the mutators take the `ResMut` `SystemMenuParams` in separate systems,
    // so no B0002. Audio resources are absent under no `audio` (the bundle cfgs out).
    snapshot: SystemMenuSnapshotParams,
    // Read-only here; the mutators (`kaleidoscope_focus_nav`, `kaleidoscope_scroll_wheel`,
    // `kaleidoscope_apply_scroll_drag`) take `ResMut<KaleidoscopeScroll>` in separate
    // systems, so this `Res` is not a B0002 conflict. Inserted at startup so it never panics.
    scroll: Res<KaleidoscopeScroll>,
    mut pages: ResMut<ActiveMenuPages<MenuPage, MenuPageAction>>,
    mut was_open: Local<bool>,
    mut last: Local<Option<RebuildKey>>,
) {
    let Some(owned) = owned else {
        return;
    };
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    let just_opened = open && !*was_open;
    *was_open = open;

    // Snapshot the System face's CONTENT once (radio station list + active station,
    // dev-toggle flags). Used both for the dirty check and for the rebuild, so the
    // per-frame `SystemMenuModel::build` is not duplicated.
    let radio = snapshot.radio_snapshot();
    let dev = snapshot.dev_snapshot();

    // Deferred Bug 2 fix: the page key keys off the System scroll-window START, NOT
    // the raw `cursor.focus`. A cursor-only move (mouse OR keyboard) no longer
    // rebuilds the face — the highlight (`kaleidoscope_sync_focus_visuals`) and the
    // detail text (`kaleidoscope_sync_detail_text`) update IN PLACE. Without this, a
    // `Pointer<Move>` between a press and release despawned the hovered control and
    // Bevy dropped the `Pointer<Click>`. Only a focus change that SHIFTS the System
    // scroll window changes the rendered rows, so only that needs a rebuild; the
    // drill-down state is also keyed so drilling in/out republishes the new rows.
    let window_start = if pages.active == Some(MenuPage::System) {
        let model = SystemMenuModel::build(&settings, &radio, &dev);
        let rows = system_rows(&model, system_nav.open_entry);
        // The EFFECTIVE window start: an explicit drag/wheel override wins (Features
        // C/D), otherwise it follows the cursor. Keying the rebuild off this means a
        // wheel/drag scroll rebuilds the windowed rows, while a cursor-only move
        // inside the window still does not (preserving A's click fix).
        system_effective_window_start(&rows, cursor.focus, scroll.system_window_start)
    } else {
        0
    };
    let key = RebuildKey {
        window_start,
        active: pages.active,
        open_entry: system_nav.open_entry,
        radio,
        dev,
    };
    // Republish on: catalog change, settings change (so a toggled setting's label
    // updates immediately), first publish, menu-open (textures that loaded after
    // the initial build get picked up), page change, a System scroll-window shift,
    // a System drill in/out, or a change to the rendered System CONTENT (auditioned
    // station / toggled dev flag).
    //
    // PERF (2026-06-10): the System content is compared by VALUE (via `key`), not by
    // Bevy change-ticks. The old `snapshot.is_changed()` ORed `AudioLibrary`'s
    // change tick, which the music director bumps EVERY FRAME while music plays (it
    // rewrites per-layer crossfade gains) — so the whole cube despawned + respawned
    // every frame with the menu open, the dominant Android FPS cliff. A gain update
    // does not change the station list / active station, so the value comparison
    // ignores it.
    let dirty = owned.is_changed()
        || settings.is_changed()
        || pages.pages.is_empty()
        || just_opened
        || last.as_ref() != Some(&key);
    if !dirty {
        return;
    }

    let active = pages.active.unwrap_or(MenuPage::Items);
    pages.pages = build_inventory_pages(
        &owned,
        owned.equipped(),
        cursor.focus,
        &settings,
        &key.radio,
        &key.dev,
        window_start,
        system_nav.open_entry,
    );
    pages.active = Some(active);
    *last = Some(key);
}

/// The value-equality key that gates a cube republish. Two frames with an equal
/// key render identical pages, so the rebuild is skipped. Compared by VALUE (not
/// Bevy change-ticks) so a per-frame resource mutation that does not alter the
/// rendered content (e.g. the music director's per-frame audio-gain updates) never
/// forces a rebuild — see `republish_kaleidoscope_pages`.
#[derive(PartialEq)]
struct RebuildKey {
    window_start: usize,
    active: Option<MenuPage>,
    open_entry: Option<SystemMenuEntryId>,
    radio: RadioSnapshot,
    dev: DevSnapshot,
}

/// The live System row count for the current drill-down state (0 outside the System
/// face). Shared by the wheel + drag scroll appliers to clamp the scroll position.
fn system_row_count(
    pages: &ActiveMenuPages<MenuPage, MenuPageAction>,
    system_nav: &KaleidoscopeSystemNav,
    settings: &UserSettings,
    snapshot: &SystemMenuSnapshotParams,
) -> usize {
    if pages.active != Some(MenuPage::System) {
        return 0;
    }
    let model = SystemMenuModel::build(
        settings,
        &snapshot.radio_snapshot(),
        &snapshot.dev_snapshot(),
    );
    system_rows(&model, system_nav.open_entry).len()
}

/// Feature D: the MOUSE WHEEL scrolls the System window (the visible rows), NOT the
/// keyboard selection. Each wheel notch moves the scroll override by one row,
/// clamped to `[0, system_max_window_start]`. The cursor/selection is untouched — a
/// later keyboard move clears the override and the window snaps back to the cursor.
/// Only acts on a scrollable System list (more rows than fit); a short list ignores
/// the wheel. Mouse OR touchpad scroll both arrive as `MouseWheel`.
fn kaleidoscope_scroll_wheel(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<ambition_sandbox::inventory::InventoryUiState>>,
    pages: Res<ActiveMenuPages<MenuPage, MenuPageAction>>,
    system_nav: Res<KaleidoscopeSystemNav>,
    settings: Res<UserSettings>,
    snapshot: SystemMenuSnapshotParams,
    cursor: Res<KaleidoscopeCursor>,
    mut scroll: ResMut<KaleidoscopeScroll>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    if backend.effective() != InventoryUiBackend::LunexKaleidoscope || !open {
        wheel.clear();
        return;
    }
    // Sum this frame's wheel deltas into integer row steps (wheel up = scroll up).
    let mut steps = 0i32;
    for ev in wheel.read() {
        steps += if ev.y > 0.0 {
            -1
        } else if ev.y < 0.0 {
            1
        } else {
            0
        };
    }
    if steps == 0 {
        return;
    }
    let total = system_row_count(&pages, &system_nav, &settings, &snapshot);
    if total <= SYSTEM_VISIBLE_ROWS {
        return; // nothing to scroll
    }
    let max = system_max_window_start(total) as i32;
    // Seed from the effective start so the first wheel notch moves relative to what
    // is currently shown (cursor-derived window) rather than jumping to 0.
    let model = SystemMenuModel::build(
        &settings,
        &snapshot.radio_snapshot(),
        &snapshot.dev_snapshot(),
    );
    let rows = system_rows(&model, system_nav.open_entry);
    let current =
        system_effective_window_start(&rows, cursor.focus, scroll.system_window_start) as i32;
    let next = (current + steps).clamp(0, max) as usize;
    scroll.system_window_start = Some(next);
}

/// Apply the lib's backend-agnostic scrollbar-drag fraction (`0..=1`) to the
/// host System-menu window. Selection is unchanged; only the visible rows move.
fn kaleidoscope_apply_scroll_drag(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<ambition_sandbox::inventory::InventoryUiState>>,
    pages: Res<ActiveMenuPages<MenuPage, MenuPageAction>>,
    system_nav: Res<KaleidoscopeSystemNav>,
    settings: Res<UserSettings>,
    snapshot: SystemMenuSnapshotParams,
    mut scroll: ResMut<KaleidoscopeScroll>,
    mut dragged: MessageReader<ambition_menu::kaleidoscope::MenuScrollDragged>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    if backend.effective() != InventoryUiBackend::LunexKaleidoscope || !open {
        dragged.clear();
        return;
    }
    // Use the LAST drag fraction this frame (the freshest pointer position).
    let Some(fraction) = dragged.read().last().map(|d| d.fraction.clamp(0.0, 1.0)) else {
        return;
    };
    let total = system_row_count(&pages, &system_nav, &settings, &snapshot);
    let result = scroll_fraction_to_window_start(total, fraction);
    if let Some(start) = result {
        scroll.system_window_start = Some(start);
    }
}

#[cfg(test)]
mod lunex_kaleidoscope_app_tests {
    //! Behaviour tests for the cube's interaction seams, driven through the real
    //! systems/observers as the app wires them.
    use super::*;
    use crate::menu::test_support::{
        click_control, pointer_location, spawn_control, trigger_move, trigger_press,
        trigger_release,
    };
    use ambition_sandbox::brain::ActionSet;
    use ambition_sandbox::game_mode::GameMode;
    use ambition_sandbox::player::{PlayerEntity, PlayerMana, PrimaryPlayer};
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::backend::HitData;
    use bevy::picking::events::{Move, Pointer, Press, Release};
    use bevy::picking::pointer::{Location, PointerId};

    fn base_kaleidoscope_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameMode>();
        app.init_resource::<InventoryUiBackend>();
        app.init_resource::<ActiveMenuPages<MenuPage, MenuPageAction>>();
        app.init_resource::<KaleidoscopeCursor>();
        app.init_resource::<ambition_sandbox::input::ActiveInputKind>();
        app.init_resource::<KaleidoscopeSystemNav>();
        app.init_resource::<KaleidoscopeScroll>();
        app.init_resource::<KaleidoscopePointerPress>();
        app.init_resource::<OwnedItems>();
        app.init_resource::<ambition_sandbox::dev::dev_tools::DeveloperTools>();
        app.init_resource::<ambition_sandbox::SandboxDevState>();
        app.init_resource::<ambition_sandbox::ldtk_world::LdtkHotReloadState>();
        app.init_resource::<ambition_sandbox::runtime::reset::SandboxResetRequested>();
        app.init_resource::<ambition_sandbox::dev::dev_tools::EditableMovementTuning>();
        app.init_resource::<UserSettings>();
        app.init_resource::<ambition_sandbox::inventory::InventoryUiState>();
        app.init_resource::<ambition_sandbox::menu::map::MapMenuState>();
        app.init_resource::<MenuControlFrame>();
        app.add_message::<PlayerHealRequested>();
        app.add_message::<SfxMessage>();
        *app.world_mut().resource_mut::<InventoryUiBackend>() =
            InventoryUiBackend::LunexKaleidoscope;
        app
    }

    fn set_kaleidoscope_visible(app: &mut App, visible: bool) {
        app.world_mut()
            .resource_mut::<ambition_sandbox::inventory::InventoryUiState>()
            .visible = visible;
    }

    fn spawn_kaleidoscope_test_player(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                ActionSet::default(),
                PlayerMana::default(),
            ))
            .id()
    }

    // ---- Developer-resource toggles ------------------------------------------

    /// Dispatching the F1/F2/F12 Developer rows flips the right resource:
    /// `DebugOverlay` → `SandboxDevState::debug`, `SlowMotion` →
    /// `SandboxDevState::slowmo`, `LdtkAutoApply` → `LdtkHotReloadState::auto_apply`
    /// — none of which live on `DeveloperTools`. Driven through the real
    /// `apply_dev_toggle` path so the cube and pause menu can't drift.
    #[test]
    fn extra_dev_toggles_flip_their_non_developer_resources() {
        let mut dev = ambition_sandbox::dev::dev_tools::DeveloperTools::default();
        let mut dev_state = ambition_sandbox::SandboxDevState::default();
        let mut ldtk_reload = ambition_sandbox::ldtk_world::LdtkHotReloadState::default();
        let mut backend = InventoryUiBackend::default();

        let debug_before = dev_state.debug;
        let slowmo_before = dev_state.slowmo;
        let auto_before = ldtk_reload.auto_apply;

        for id in [
            DevToggleId::DebugOverlay,
            DevToggleId::SlowMotion,
            DevToggleId::LdtkAutoApply,
        ] {
            apply_dev_toggle(
                DevToggleWrite {
                    dev: &mut dev,
                    dev_state: &mut dev_state,
                    ldtk_reload: &mut ldtk_reload,
                    backend: &mut backend,
                    #[cfg(feature = "portal_render")]
                    portal_effect: None,
                },
                id,
                0,
            );
        }

        assert_eq!(
            dev_state.debug, !debug_before,
            "F1 flips SandboxDevState.debug"
        );
        assert_eq!(
            dev_state.slowmo, !slowmo_before,
            "F2 flips SandboxDevState.slowmo"
        );
        assert_eq!(
            ldtk_reload.auto_apply, !auto_before,
            "F12 flips LdtkHotReloadState.auto_apply"
        );
        // The snapshot mirrors the live state for all three (no field drift).
        let snap = dev_snapshot(DevToggleRead {
            dev: &dev,
            dev_state: &dev_state,
            ldtk_reload: &ldtk_reload,
            backend: InventoryUiBackend::default(),
            #[cfg(feature = "portal_render")]
            portal_effect: None,
        });
        let read = |id: DevToggleId| snap.values.iter().find(|(d, _, _)| *d == id).unwrap().1;
        assert_eq!(read(DevToggleId::DebugOverlay), dev_state.debug);
        assert_eq!(read(DevToggleId::SlowMotion), dev_state.slowmo);
        assert_eq!(read(DevToggleId::LdtkAutoApply), ldtk_reload.auto_apply);
    }

    /// The Developer "Menu Backend" row exists and dispatching it cycles
    /// `InventoryUiBackend` (Grid ↔ Cube) — the in-menu equivalent of the `\`
    /// hotkey, wired through the shared `apply_dev_toggle` path so BOTH backends can
    /// trigger it. Its snapshot value label reflects the active frontend.
    #[test]
    fn menu_backend_dev_row_cycles_inventory_backend() {
        // The row is part of the Developer vocabulary and is a cycle (shows a label).
        assert!(DevToggleId::ALL.contains(&DevToggleId::MenuBackend));
        assert!(DevToggleId::MenuBackend.is_cycle());

        let mut dev = ambition_sandbox::dev::dev_tools::DeveloperTools::default();
        let mut dev_state = ambition_sandbox::SandboxDevState::default();
        let mut ldtk_reload = ambition_sandbox::ldtk_world::LdtkHotReloadState::default();
        let mut backend = InventoryUiBackend::Grid;

        // The snapshot label reflects the live backend.
        let label = |b: InventoryUiBackend| {
            dev_snapshot(DevToggleRead {
                dev: &dev,
                dev_state: &dev_state,
                ldtk_reload: &ldtk_reload,
                backend: b,
                #[cfg(feature = "portal_render")]
                portal_effect: None,
            })
            .values
            .iter()
            .find(|(d, _, _)| *d == DevToggleId::MenuBackend)
            .unwrap()
            .2
            .clone()
        };
        assert_eq!(label(InventoryUiBackend::Grid), "Grid");
        assert_eq!(label(InventoryUiBackend::LunexKaleidoscope), "Cube");

        // Dispatching the row flips the backend Grid → Cube …
        apply_dev_toggle(
            DevToggleWrite {
                dev: &mut dev,
                dev_state: &mut dev_state,
                ldtk_reload: &mut ldtk_reload,
                backend: &mut backend,
                #[cfg(feature = "portal_render")]
                portal_effect: None,
            },
            DevToggleId::MenuBackend,
            0,
        );
        assert_eq!(backend, InventoryUiBackend::LunexKaleidoscope);
        // … and back Cube → Grid.
        apply_dev_toggle(
            DevToggleWrite {
                dev: &mut dev,
                dev_state: &mut dev_state,
                ldtk_reload: &mut ldtk_reload,
                backend: &mut backend,
                #[cfg(feature = "portal_render")]
                portal_effect: None,
            },
            DevToggleId::MenuBackend,
            0,
        );
        assert_eq!(backend, InventoryUiBackend::Grid);
    }

    /// ShowHitboxes from the System menu toggles the SAME field(s) the pause menu
    /// does: BOTH `show_feature_hitboxes` and `show_player_hitbox`, and the
    /// snapshot reads `show_feature_hitboxes` (matching the pause menu's source).
    #[test]
    fn show_hitboxes_toggles_feature_and_player_fields_like_pause() {
        let mut dev = ambition_sandbox::dev::dev_tools::DeveloperTools::default();
        let mut dev_state = ambition_sandbox::SandboxDevState::default();
        let mut ldtk_reload = ambition_sandbox::ldtk_world::LdtkHotReloadState::default();
        dev.show_feature_hitboxes = false;
        dev.show_player_hitbox = false;
        let mut backend = InventoryUiBackend::default();

        apply_dev_toggle(
            DevToggleWrite {
                dev: &mut dev,
                dev_state: &mut dev_state,
                ldtk_reload: &mut ldtk_reload,
                backend: &mut backend,
                #[cfg(feature = "portal_render")]
                portal_effect: None,
            },
            DevToggleId::ShowHitboxes,
            0,
        );
        assert!(dev.show_feature_hitboxes, "feature hitboxes flip on");
        assert!(
            dev.show_player_hitbox,
            "player hitbox flips together (pause-menu parity)"
        );

        let snap = dev_snapshot(DevToggleRead {
            dev: &dev,
            dev_state: &dev_state,
            ldtk_reload: &ldtk_reload,
            backend: InventoryUiBackend::default(),
            #[cfg(feature = "portal_render")]
            portal_effect: None,
        });
        let on = snap
            .values
            .iter()
            .find(|(d, _, _)| *d == DevToggleId::ShowHitboxes)
            .unwrap()
            .1;
        assert!(on, "snapshot reads show_feature_hitboxes, now ON");
    }

    // ---- Fix 1: back-edge seeding --------------------------------------------

    #[test]
    fn back_edge_lands_opposite_the_direction_travelled() {
        // Turning RIGHT brings the viewer-right page to front; to go BACK you turn
        // left, so the cursor lands on the LEFT edge button — and vice-versa.
        let from = MenuPage::Items;
        let right = from.on_viewer_right();
        assert_eq!(back_edge_focus(Some(from), right), MenuFocus::EdgeLeft);
        let left = from.on_viewer_left();
        assert_eq!(back_edge_focus(Some(from), left), MenuFocus::EdgeRight);
        // First open (no prior page) defaults to a highlighted left edge button.
        assert_eq!(back_edge_focus(None, MenuPage::Map), MenuFocus::EdgeLeft);
    }

    // ---- Fix 4: System-page pointer clicks -----------------------------------

    fn click_app() -> (App, Entity) {
        let mut app = base_kaleidoscope_test_app();
        // Feature E: the tap/drag-cancel guard needs the press + move observers in
        // addition to the release-dispatch observer.
        app.add_observer(kaleidoscope_pointer_press);
        app.add_observer(kaleidoscope_pointer_move);
        app.add_observer(kaleidoscope_pointer_release);
        set_kaleidoscope_visible(&mut app, true);
        let player = spawn_kaleidoscope_test_player(&mut app);
        app.update();
        (app, player)
    }

    fn open_app() -> App {
        let mut app = base_kaleidoscope_test_app();
        app.add_systems(Update, kaleidoscope_menu_open_routing);
        app.add_observer(kaleidoscope_pointer_move);
        set_kaleidoscope_visible(&mut app, false);
        spawn_kaleidoscope_test_player(&mut app);
        app.update();
        app
    }

    fn move_control(app: &mut App, action: MenuPageAction) {
        app.world_mut()
            .insert_resource(ambition_sandbox::input::ActiveInputKind::Mouse);
        let entity = spawn_control(app, action);
        trigger_move(app, entity, Vec2::new(1.0, 0.0));
        app.update();
    }

    // ---- Fix 2: shoulder-bumper page turns -----------------------------------

    fn nav_app() -> App {
        let mut app = base_kaleidoscope_test_app();
        app.add_systems(Update, kaleidoscope_focus_nav);
        set_kaleidoscope_visible(&mut app, true);
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::Items);
        spawn_kaleidoscope_test_player(&mut app);
        app.update();
        app
    }

    fn system_nav_app(focus: MenuFocus) -> App {
        let mut app = base_kaleidoscope_test_app();
        app.add_systems(Update, kaleidoscope_focus_nav);
        set_kaleidoscope_visible(&mut app, true);
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::System);
        app.world_mut().resource_mut::<KaleidoscopeCursor>().focus = focus;
        spawn_kaleidoscope_test_player(&mut app);
        app.update();
        app
    }

    fn press_bumper(app: &mut App, right: bool) {
        let mut frame = MenuControlFrame::default();
        if right {
            frame.page_right = true;
        } else {
            frame.page_left = true;
        }
        app.insert_resource(frame);
        app.update();
    }

    #[test]
    fn right_bumper_turns_to_the_viewer_right_page() {
        let mut app = nav_app();
        press_bumper(&mut app, true);
        assert_eq!(
            app.world()
                .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
                .active,
            Some(MenuPage::Items.on_viewer_right()),
            "right bumper rotates to the viewer-right page (Fix 2)"
        );
        // The cursor lands on the new page's back-edge button (Fix 1): arriving from
        // the right edge means the LEFT edge button turns back home.
        assert_eq!(
            app.world().resource::<KaleidoscopeCursor>().focus,
            MenuFocus::EdgeLeft,
            "cursor seeds onto the back (left) edge button"
        );
    }

    #[test]
    fn left_bumper_turns_to_the_viewer_left_page() {
        let mut app = nav_app();
        press_bumper(&mut app, false);
        assert_eq!(
            app.world()
                .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
                .active,
            Some(MenuPage::Items.on_viewer_left()),
            "left bumper rotates to the viewer-left page (Fix 2)"
        );
        assert_eq!(
            app.world().resource::<KaleidoscopeCursor>().focus,
            MenuFocus::EdgeRight,
            "cursor seeds onto the back (right) edge button"
        );
    }

    #[test]
    fn clicking_a_system_entry_drills_in() {
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::System);
        assert!(app
            .world()
            .resource::<KaleidoscopeSystemNav>()
            .open_entry
            .is_none());
        click_control(
            &mut app,
            MenuPageAction::OpenSystemEntry(SystemMenuEntryId::Audio),
        );
        assert_eq!(
            app.world().resource::<KaleidoscopeSystemNav>().open_entry,
            Some(SystemMenuEntryId::Audio),
            "clicking a System entry drills into it (Fix 4)"
        );
    }

    #[test]
    fn clicking_a_system_setting_applies_it() {
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::System);
        app.world_mut()
            .resource_mut::<KaleidoscopeSystemNav>()
            .open_entry = Some(SystemMenuEntryId::Video);
        let before = app.world().resource::<UserSettings>().video.show_fps;
        click_control(&mut app, MenuPageAction::System(SettingsOptionId::ShowFps));
        let after = app.world().resource::<UserSettings>().video.show_fps;
        assert_ne!(before, after, "clicking a setting toggles it (Fix 4)");
    }

    #[test]
    fn clicking_back_drills_out_to_the_entry_list() {
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::System);
        app.world_mut()
            .resource_mut::<KaleidoscopeSystemNav>()
            .open_entry = Some(SystemMenuEntryId::Audio);
        click_control(&mut app, MenuPageAction::CloseSystemEntry);
        assert!(
            app.world()
                .resource::<KaleidoscopeSystemNav>()
                .open_entry
                .is_none(),
            "clicking Back drills out to the entry list (Fix 4)"
        );
    }

    #[test]
    fn clicking_a_radio_station_keeps_the_menu_open() {
        // Selecting a radio station auditions it WITHOUT closing the cube, so the
        // user can keep browsing. Audio is absent in this minimal fixture, so the
        // apply no-ops, but the menu must still stay open.
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::System);
        app.world_mut()
            .resource_mut::<KaleidoscopeSystemNav>()
            .open_entry = Some(SystemMenuEntryId::Radio);
        click_control(
            &mut app,
            MenuPageAction::SystemOption(SystemOptionId::Radio(0)),
        );
        assert!(
            app.world()
                .resource::<ambition_sandbox::inventory::InventoryUiState>()
                .visible,
            "auditioning a station keeps the cube open"
        );
    }

    #[test]
    fn reset_sandbox_action_closes_and_unpauses() {
        // Reset Sandbox closes the cube via a dispatched action (`close_menu = true`).
        // When the menu was opened from gameplay (paused, not opened-from-pause), the
        // action-close must ALSO restore `GameMode::Playing` — exactly like a normal
        // Esc-close — instead of leaving the sim paused with the menu hidden. Before the
        // fix the close path only did `ui_state.visible = false`, so this stayed Paused.
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::System);
        // Open the menu from gameplay: paused, but NOT nested under the pause menu.
        app.world_mut()
            .resource_mut::<ambition_sandbox::inventory::InventoryUiState>()
            .opened_from_pause = false;
        app.world_mut()
            .resource_mut::<NextState<GameMode>>()
            .set(GameMode::Paused);
        app.update();
        assert_eq!(
            *app.world().resource::<State<GameMode>>().get(),
            GameMode::Paused,
            "precondition: menu opened from gameplay leaves the sim paused"
        );

        // Dispatch Reset Sandbox through the real pointer release/dispatch path.
        click_control(
            &mut app,
            MenuPageAction::SystemAction(SystemMenuAction::ResetSandbox),
        );

        assert!(
            !app.world()
                .resource::<ambition_sandbox::inventory::InventoryUiState>()
                .visible,
            "Reset Sandbox hides the cube"
        );
        // The action-close set NextState(Playing); apply the transition and confirm the
        // sim is unpaused (the bug left it stuck on Paused).
        app.update();
        assert_eq!(
            *app.world().resource::<State<GameMode>>().get(),
            GameMode::Playing,
            "Reset Sandbox closes the menu AND unpauses (back to Playing)"
        );
    }

    #[test]
    fn reset_all_settings_action_resets_settings_and_closes() {
        // The cube's System menu surfaces Reset All Settings as a top-level Action
        // entry; dispatching it resets every persisted settings/dev resource to
        // defaults (the same set the pause menu's ResetAllSettings restores) and
        // folds the menu shut (close, which also unpauses).
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::System);

        // The IR surfaces Reset All Settings as an always-present Action entry.
        let model = SystemMenuModel::build(
            &UserSettings::default(),
            &RadioSnapshot::default(),
            &DevSnapshot::default(),
        );
        assert!(
            model.entry(SystemMenuEntryId::ResetAllSettings).is_some(),
            "Reset All Settings is surfaced as a top-level entry"
        );

        // Mutate persisted resources away from their defaults.
        app.world_mut()
            .resource_mut::<UserSettings>()
            .audio
            .master_volume = 0.123;
        app.world_mut()
            .resource_mut::<ambition_sandbox::dev::dev_tools::DeveloperTools>()
            .inspector_visible = true;

        // Dispatch Reset All Settings through the real pointer release/dispatch path.
        click_control(
            &mut app,
            MenuPageAction::SystemAction(SystemMenuAction::ResetAllSettings),
        );

        // UserSettings + DeveloperTools are back to defaults.
        assert_eq!(
            *app.world().resource::<UserSettings>(),
            UserSettings::default(),
            "Reset All Settings restores UserSettings to defaults"
        );
        assert!(
            !app.world()
                .resource::<ambition_sandbox::dev::dev_tools::DeveloperTools>()
                .inspector_visible,
            "Reset All Settings restores DeveloperTools to defaults"
        );
        // The cube folds shut (same close as Reset Sandbox).
        assert!(
            !app.world()
                .resource::<ambition_sandbox::inventory::InventoryUiState>()
                .visible,
            "Reset All Settings closes the cube"
        );
    }

    #[test]
    fn quit_action_writes_app_exit_and_closes() {
        // The cube's System menu surfaces Quit to Desktop as a top-level Action
        // entry; dispatching it writes `AppExit::Success` and folds the menu shut.
        let (mut app, _player) = click_app();
        app.add_message::<bevy::app::AppExit>();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::System);

        // The IR surfaces Quit as an always-present Action entry.
        let model = SystemMenuModel::build(
            &UserSettings::default(),
            &RadioSnapshot::default(),
            &DevSnapshot::default(),
        );
        assert_eq!(
            model.entry(SystemMenuEntryId::Quit).map(|e| &e.target),
            Some(
                &ambition_sandbox::persistence::settings::SystemMenuTarget::Action(
                    SystemMenuAction::Quit
                )
            ),
            "Quit is surfaced as a top-level Action entry"
        );

        // Dispatch Quit through the real pointer release/dispatch path.
        click_control(
            &mut app,
            MenuPageAction::SystemAction(SystemMenuAction::Quit),
        );

        // An AppExit::Success was written.
        let messages = app.world().resource::<Messages<bevy::app::AppExit>>();
        let mut reader = messages.get_cursor();
        let exits: Vec<_> = reader.read(messages).collect();
        assert_eq!(
            exits,
            vec![&bevy::app::AppExit::Success],
            "Quit dispatches a single AppExit::Success"
        );

        // The cube folds shut (same close as the other immediate actions).
        assert!(
            !app.world()
                .resource::<ambition_sandbox::inventory::InventoryUiState>()
                .visible,
            "Quit closes the cube"
        );
    }

    #[test]
    fn system_edge_left_moves_inward_to_the_row_list() {
        let mut app = system_nav_app(MenuFocus::EdgeLeft);
        let mut frame = MenuControlFrame::default();
        frame.right = true;
        app.insert_resource(frame);
        app.update();

        assert_eq!(
            app.world().resource::<KaleidoscopeCursor>().focus,
            MenuFocus::System(0),
            "moving right from the < Items button enters the System row list instead of rotating"
        );
        assert_eq!(
            app.world()
                .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
                .active,
            Some(MenuPage::System),
            "the cube stays on the System face while moving into the rows"
        );
    }

    #[test]
    fn system_row_horizontal_moves_to_the_edge_buttons() {
        let mut app = system_nav_app(MenuFocus::System(1));
        let mut frame = MenuControlFrame::default();
        frame.left = true;
        app.insert_resource(frame);
        app.update();

        assert_eq!(
            app.world().resource::<KaleidoscopeCursor>().focus,
            MenuFocus::EdgeLeft,
            "horizontal motion from a row should land on the left edge button"
        );
    }

    #[test]
    fn pointer_motion_selects_a_kaleidoscope_control() {
        let mut app = open_app();
        app.world_mut()
            .resource_mut::<ambition_sandbox::inventory::InventoryUiState>()
            .visible = true;
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::Items);
        app.world_mut().resource_mut::<KaleidoscopeCursor>().focus = MenuFocus::EdgeRight;

        move_control(
            &mut app,
            MenuPageAction::ChangePage(MenuPage::Items.on_viewer_left()),
        );

        assert_eq!(
            app.world().resource::<KaleidoscopeCursor>().focus,
            MenuFocus::EdgeLeft,
            "actual pointer motion updates the cube cursor"
        );
        assert_eq!(
            app.world()
                .resource::<KaleidoscopeCursor>()
                .last_pointer_focus,
            Some(MenuFocus::EdgeLeft),
            "the hovered focus is remembered for later move dedup"
        );
    }

    /// Faithful reproduction of the real app's input wiring: a leafwing player with
    /// Esc bound to BOTH `Start` (pause) and `MenuBack`, the menu-frame populate
    /// system, AND the cube routing — registered in the SAME default Update set so
    /// the scheduler is free to order them as it does in the real app.
    ///
    /// Fix 1 behaviour: while the menu is open, Esc BACKS OUT one level when inside a
    /// nested System screen (`open_entry.is_some()`) and only CLOSES at the top level.
    /// So from a drilled-in category: first Esc → back to the entry list (still open),
    /// second Esc → close. There must be no double-trigger (Esc fires both
    /// `menu.start` and `menu.back`).
    #[test]
    fn esc_backs_out_then_closes_the_kaleidoscope_via_real_input() {
        use ambition_sandbox::input::SandboxAction;
        use ambition_sandbox::presentation::rendering::PlayerVisual;
        use leafwing_input_manager::prelude::*;

        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.add_plugins(bevy::time::TimePlugin);
        app.add_plugins(bevy::input::InputPlugin);
        app.add_plugins(InputManagerPlugin::<SandboxAction>::default());
        app.init_state::<GameMode>();
        app.init_resource::<InventoryUiBackend>();
        app.init_resource::<ActiveMenuPages<MenuPage, MenuPageAction>>();
        app.init_resource::<KaleidoscopeCursor>();
        app.init_resource::<ambition_sandbox::input::ActiveInputKind>();
        app.init_resource::<KaleidoscopeSystemNav>();
        app.init_resource::<KaleidoscopeScroll>();
        app.init_resource::<KaleidoscopePointerPress>();
        app.init_resource::<OwnedItems>();
        app.init_resource::<ambition_sandbox::dev::dev_tools::DeveloperTools>();
        app.init_resource::<ambition_sandbox::SandboxDevState>();
        app.init_resource::<ambition_sandbox::ldtk_world::LdtkHotReloadState>();
        app.init_resource::<ambition_sandbox::runtime::reset::SandboxResetRequested>();
        app.init_resource::<ambition_sandbox::dev::dev_tools::EditableMovementTuning>();
        app.init_resource::<UserSettings>();
        app.init_resource::<ambition_sandbox::inventory::InventoryUiState>();
        app.init_resource::<ambition_sandbox::menu::map::MapMenuState>();
        app.init_resource::<MenuControlFrame>();
        app.init_resource::<ambition_sandbox::input::MenuInputState>();
        app.add_message::<PlayerHealRequested>();
        app.add_message::<SfxMessage>();
        app.add_systems(
            Update,
            (
                ambition_sandbox::app::populate_menu_control_frame_from_actions,
                kaleidoscope_menu_open_routing,
                kaleidoscope_focus_nav,
            )
                .chain(),
        );
        *app.world_mut().resource_mut::<InventoryUiBackend>() =
            InventoryUiBackend::LunexKaleidoscope;

        // Esc → both Start (pause) and MenuBack, exactly like the keyboard preset.
        let mut map = InputMap::<SandboxAction>::default();
        map.insert(SandboxAction::Start, KeyCode::Escape);
        map.insert(SandboxAction::MenuBack, KeyCode::Escape);
        app.world_mut().spawn((
            PlayerVisual,
            PlayerEntity,
            PrimaryPlayer,
            ActionSet::default(),
            PlayerMana::default(),
            ActionState::<SandboxAction>::default(),
            map,
        ));
        app.update();

        let press_esc = |app: &mut App, down: bool| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            if down {
                keys.press(KeyCode::Escape);
            } else {
                keys.release(KeyCode::Escape);
            }
        };
        let visible = |app: &App| {
            app.world()
                .resource::<ambition_sandbox::inventory::InventoryUiState>()
                .visible
        };

        // First Esc press → opens the cube.
        press_esc(&mut app, true);
        app.update();
        press_esc(&mut app, false);
        app.update();
        assert!(visible(&app), "first Esc opens the cube");

        // Drill INTO a System-page category. The close path is page-dependent: inside
        // a category Esc must BACK OUT one level (not close), and that drill-out is
        // owned by the start branch (we consume the co-firing `menu.back` so the nav
        // systems never see this Esc).
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::System);
        app.world_mut()
            .resource_mut::<KaleidoscopeSystemNav>()
            .open_entry = Some(SystemMenuEntryId::Audio);
        app.world_mut().resource_mut::<KaleidoscopeCursor>().focus = MenuFocus::System(0);

        // Second Esc press → backs OUT to the entry list (menu stays open).
        press_esc(&mut app, true);
        app.update();
        press_esc(&mut app, false);
        app.update();
        assert!(
            visible(&app),
            "second Esc (nested) backs out one level, keeping the menu open"
        );
        assert!(
            app.world()
                .resource::<KaleidoscopeSystemNav>()
                .open_entry
                .is_none(),
            "second Esc drilled out of the open System entry"
        );

        // Third Esc press → now at the top level, CLOSES the cube.
        press_esc(&mut app, true);
        app.update();
        press_esc(&mut app, false);
        app.update();
        assert!(!visible(&app), "third Esc (top level) closes the cube");
    }

    #[test]
    fn opening_the_kaleidoscope_clears_stale_pointer_hover_state() {
        let mut app = open_app();
        app.world_mut()
            .resource_mut::<KaleidoscopeCursor>()
            .last_pointer_focus = Some(MenuFocus::Item(7));
        app.world_mut().resource_mut::<MenuControlFrame>().start = true;
        app.world_mut()
            .resource_mut::<ambition_sandbox::inventory::InventoryUiState>()
            .visible = false;
        app.update();

        assert_eq!(
            app.world().resource::<KaleidoscopeCursor>().last_pointer_focus,
            None,
            "opening the cube clears stale pointer hover state so parked hover cannot select immediately"
        );
    }

    // ---- Bug 2: click/tap activation survives a hover-driven republish ---------
    //
    // Root cause (now fixed): a `Pointer<Move>` changed `cursor.focus`, which the
    // republish baked into its dirty key, so it rewrote `ActiveMenuPages`; the lib's
    // `rebuild_cube_faces` then despawned + respawned every control. When that
    // happened BETWEEN a pointer press and release, Bevy dropped the `Pointer<Click>`
    // (the press entity no longer existed), so clicking a control did NOTHING while
    // mouse-over highlight worked. The fix moves the highlight + detail text in place
    // (no rebuild on a cursor-only move). These tests reproduce the drop and assert
    // the click now dispatches.

    /// A faithful stand-in for the lib's `rebuild_cube_faces`: whenever
    /// `ActiveMenuPages` is `Changed` (which the OLD republish did on every cursor
    /// move), despawn every `AmbitionMenuControl` and respawn the actionable controls
    /// from `pages.pages`. This reproduces the exact entity-id churn that dropped the
    /// click — the real renderer is too heavy to run headless.
    fn fake_rebuild_cube_faces(
        mut commands: Commands,
        pages: Res<ActiveMenuPages<MenuPage, MenuPageAction>>,
        existing: Query<Entity, With<AmbitionMenuControl<MenuPageAction>>>,
        mut built: Local<bool>,
    ) {
        if !pages.is_changed() && *built {
            return;
        }
        *built = true;
        for entity in &existing {
            commands.entity(entity).despawn();
        }
        for page in &pages.pages {
            for node in &page.nodes {
                if let ambition_menu::MenuNode::Control {
                    kind,
                    action: Some(action),
                    ..
                } = node
                {
                    commands.spawn((
                        AmbitionMenuControl::<MenuPageAction> {
                            kind: *kind,
                            action: Some(action.clone()),
                            focus: ambition_menu::MenuFocusKey::default(),
                        },
                        MenuVisualState::default(),
                    ));
                }
            }
        }
    }

    /// A full Bug-2 fixture: the REAL republish + in-place highlight/detail systems +
    /// the `fake_rebuild` (mirroring the lib) + the real pointer observers, on the
    /// given active page. Drives the genuine despawn-on-republish path.
    fn bug2_app(active: MenuPage) -> App {
        let mut app = base_kaleidoscope_test_app();
        app.add_systems(
            Update,
            (
                republish_kaleidoscope_pages,
                kaleidoscope_sync_focus_visuals,
                kaleidoscope_sync_detail_text,
                fake_rebuild_cube_faces,
            )
                .chain(),
        );
        app.add_observer(kaleidoscope_pointer_press);
        app.add_observer(kaleidoscope_pointer_move);
        app.add_observer(kaleidoscope_pointer_release);
        set_kaleidoscope_visible(&mut app, true);
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(active);
        spawn_kaleidoscope_test_player(&mut app);
        // First update: republish builds the page data, fake_rebuild spawns controls.
        app.update();
        app
    }

    /// The live control entity carrying `action` (the one the renderer spawned).
    fn control_entity(app: &mut App, action: MenuPageAction) -> Entity {
        let mut q = app
            .world_mut()
            .query::<(Entity, &AmbitionMenuControl<MenuPageAction>)>();
        q.iter(app.world())
            .find(|(_, c)| c.action.as_ref() == Some(&action))
            .map(|(e, _)| e)
            .unwrap_or_else(|| panic!("no live control for {action:?}"))
    }

    /// Reproduce Bug 2 on the NEW release-dispatch path: PRESS the original
    /// `click_target` (arming its action), then hover-move onto `move_to` (which
    /// rebuilds the face and DESPAWNS the pressed control), then RELEASE. The action
    /// must still dispatch because it was captured at press time — entity-independent.
    ///
    /// Under the OLD `Pointer<Click>` path this dropped the activation: the press
    /// entity was gone by release, so the compound click never resolved.
    fn hover_then_click(app: &mut App, move_to: MenuPageAction, click_target: MenuPageAction) {
        // The entity a real pointer press latches onto, captured BEFORE the rebuild.
        let target = control_entity(app, click_target);
        // 1. PRESS the target: arms the action in `KaleidoscopePointerPress`.
        app.world_mut().trigger(Pointer::new(
            PointerId::Mouse,
            pointer_location(),
            Press {
                button: bevy::picking::pointer::PointerButton::Primary,
                hit: HitData::new(target, 0.0, None, None),
            },
            target,
        ));
        // 2. Hover-move onto a different control: changes `cursor.focus`, which the
        //    republish bakes into pages → fake_rebuild despawns `target`.
        let move_target = control_entity(app, move_to);
        app.world_mut().trigger(Pointer::new(
            PointerId::Mouse,
            pointer_location(),
            Move {
                hit: HitData::new(move_target, 0.0, None, None),
                delta: Vec2::new(2.0, 0.0),
            },
            move_target,
        ));
        app.update();
        // 3. RELEASE. The release entity (`target`) may now be despawned, but the
        //    handler dispatches the action STORED at press time, not the release
        //    entity — so the activation survives the rebuild (the fix).
        app.world_mut().trigger(Pointer::new(
            PointerId::Mouse,
            pointer_location(),
            Release {
                button: bevy::picking::pointer::PointerButton::Primary,
                hit: HitData::new(target, 0.0, None, None),
            },
            target,
        ));
        app.update();
    }

    #[test]
    fn bug2_item_equip_click_survives_a_hover_republish() {
        let mut app = bug2_app(MenuPage::Items);
        // Two owned, equippable (held-item) weapons so both an equip target and a
        // distinct hover target exist as live controls.
        {
            let mut owned = app.world_mut().resource_mut::<OwnedItems>();
            owned.grant(Item::Blink, 1);
            owned.grant(Item::Axe, 1);
        }
        app.update();
        assert!(
            !app.world()
                .resource::<OwnedItems>()
                .is_equipped(Item::Blink),
            "precondition: Blink not equipped yet"
        );
        // Hover Axe (moves focus → old rebuild), then click Blink (was despawned).
        hover_then_click(
            &mut app,
            MenuPageAction::Equip(Item::Axe),
            MenuPageAction::Equip(Item::Blink),
        );
        assert!(
            app.world()
                .resource::<OwnedItems>()
                .is_equipped(Item::Blink),
            "clicking an item after a hover-move must still equip it (Bug 2)"
        );
    }

    #[test]
    fn bug2_page_turn_click_survives_a_hover_republish() {
        let mut app = bug2_app(MenuPage::Items);
        app.update();
        let target_page = MenuPage::Items.on_viewer_right();
        // Hover the LEFT edge (moves focus), then click the RIGHT edge (page turn).
        hover_then_click(
            &mut app,
            MenuPageAction::ChangePage(MenuPage::Items.on_viewer_left()),
            MenuPageAction::ChangePage(target_page),
        );
        assert_eq!(
            app.world()
                .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
                .active,
            Some(target_page),
            "clicking a page-turn edge after a hover-move must still turn the page (Bug 2)"
        );
    }

    #[test]
    fn bug2_system_row_click_survives_a_hover_republish() {
        let mut app = bug2_app(MenuPage::System);
        app.update();
        assert!(
            app.world()
                .resource::<KaleidoscopeSystemNav>()
                .open_entry
                .is_none(),
            "precondition: no System entry open"
        );
        // Hover the Video entry (moves focus), then click the Audio entry (drill in).
        hover_then_click(
            &mut app,
            MenuPageAction::OpenSystemEntry(SystemMenuEntryId::Video),
            MenuPageAction::OpenSystemEntry(SystemMenuEntryId::Audio),
        );
        assert_eq!(
            app.world().resource::<KaleidoscopeSystemNav>().open_entry,
            Some(SystemMenuEntryId::Audio),
            "clicking a System row after a hover-move must still drill in (Bug 2)"
        );
    }

    // ---- Features C/D/E: scroll position + tap/drag cancel --------------------

    /// A System fixture drilled into Developer (16 toggles + Back = a list LONGER
    /// than `SYSTEM_VISIBLE_ROWS`, so it is scrollable) running the real scroll
    /// chain: keyboard nav, the mouse-wheel scroller, the scrollbar-drag applier,
    /// and the page republish. No audio resources needed (dev toggles overflow).
    fn scroll_app() -> App {
        let mut app = base_kaleidoscope_test_app();
        app.add_message::<bevy::input::mouse::MouseWheel>();
        app.add_message::<ambition_menu::kaleidoscope::MenuScrollDragged>();
        app.add_systems(
            Update,
            (
                kaleidoscope_focus_nav,
                kaleidoscope_scroll_wheel.run_if(kaleidoscope_menu_visible),
                kaleidoscope_apply_scroll_drag.run_if(kaleidoscope_menu_visible),
                republish_kaleidoscope_pages,
            )
                .chain(),
        );
        set_kaleidoscope_visible(&mut app, true);
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::System);
        app.world_mut()
            .resource_mut::<KaleidoscopeSystemNav>()
            .open_entry = Some(SystemMenuEntryId::Developer);
        app.world_mut().resource_mut::<KaleidoscopeCursor>().focus = MenuFocus::System(0);
        spawn_kaleidoscope_test_player(&mut app);
        app.update();
        app
    }

    /// The live Developer row count for the scroll fixture (18 toggles + Back).
    fn scroll_total_rows(app: &App) -> usize {
        let settings = app.world().resource::<UserSettings>();
        let dev = app
            .world()
            .resource::<ambition_sandbox::dev::dev_tools::DeveloperTools>();
        let dev_state = app.world().resource::<ambition_sandbox::SandboxDevState>();
        let ldtk_reload = app
            .world()
            .resource::<ambition_sandbox::ldtk_world::LdtkHotReloadState>();
        let backend = *app.world().resource::<InventoryUiBackend>();
        let snap = dev_snapshot(DevToggleRead {
            dev,
            dev_state,
            ldtk_reload,
            backend,
            #[cfg(feature = "portal_render")]
            portal_effect: None,
        });
        let model = SystemMenuModel::build(settings, &RadioSnapshot::default(), &snap);
        system_rows(&model, Some(SystemMenuEntryId::Developer)).len()
    }

    /// Feature D: the mouse wheel scrolls the System window (window_start) WITHOUT
    /// moving the keyboard selection cursor.
    #[test]
    fn mouse_wheel_scrolls_window_not_selection() {
        let mut app = scroll_app();
        let total = scroll_total_rows(&app);
        assert!(
            total > SYSTEM_VISIBLE_ROWS,
            "fixture list must overflow: {total}"
        );

        let cursor_before = app.world().resource::<KaleidoscopeCursor>().focus;
        assert_eq!(
            app.world()
                .resource::<KaleidoscopeScroll>()
                .system_window_start,
            None,
            "starts following the cursor (no override)"
        );

        // Wheel DOWN three notches (negative y = scroll down).
        for _ in 0..3 {
            app.world_mut()
                .resource_mut::<Messages<bevy::input::mouse::MouseWheel>>()
                .write(bevy::input::mouse::MouseWheel {
                    unit: bevy::input::mouse::MouseScrollUnit::Line,
                    x: 0.0,
                    y: -1.0,
                    window: Entity::PLACEHOLDER,
                });
            app.update();
        }

        let scroll = app
            .world()
            .resource::<KaleidoscopeScroll>()
            .system_window_start;
        assert_eq!(
            scroll,
            Some(3),
            "three wheel-down notches scroll the window to row 3"
        );
        assert_eq!(
            app.world().resource::<KaleidoscopeCursor>().focus,
            cursor_before,
            "the wheel must NOT move the selection cursor (Feature D)"
        );
    }

    /// Feature C: applying a scrollbar drag fraction (the lib's neutral signal) moves
    /// the window_start proportionally across the scrollable range.
    #[test]
    fn scrollbar_drag_fraction_sets_window_start_proportionally() {
        let mut app = scroll_app();
        let total = scroll_total_rows(&app);
        let max = system_max_window_start(total);
        assert!(max > 0, "fixture must be scrollable");

        let cursor_before = app.world().resource::<KaleidoscopeCursor>().focus;

        // Drag to the BOTTOM of the track (fraction 1.0) -> window_start == max.
        app.world_mut()
            .resource_mut::<Messages<ambition_menu::kaleidoscope::MenuScrollDragged>>()
            .write(ambition_menu::kaleidoscope::MenuScrollDragged { fraction: 1.0 });
        app.update();
        assert_eq!(
            app.world()
                .resource::<KaleidoscopeScroll>()
                .system_window_start,
            Some(max),
            "fraction 1.0 scrolls to the bottom (Feature C)"
        );

        // Drag to the MIDDLE (fraction 0.5) -> ~half the range.
        app.world_mut()
            .resource_mut::<Messages<ambition_menu::kaleidoscope::MenuScrollDragged>>()
            .write(ambition_menu::kaleidoscope::MenuScrollDragged { fraction: 0.5 });
        app.update();
        let expected_mid = (0.5 * max as f32).round() as usize;
        assert_eq!(
            app.world()
                .resource::<KaleidoscopeScroll>()
                .system_window_start,
            Some(expected_mid),
            "fraction 0.5 maps to the middle of the range"
        );
        assert_eq!(
            app.world().resource::<KaleidoscopeCursor>().focus,
            cursor_before,
            "a scrollbar drag does not move the selection cursor"
        );
    }

    /// Feature C/D: a keyboard move after a wheel/drag scroll CLEARS the override so
    /// the window snaps back to following the selection cursor.
    #[test]
    fn keyboard_nav_clears_the_scroll_override() {
        let mut app = scroll_app();
        // Establish an override via a wheel notch.
        app.world_mut()
            .resource_mut::<Messages<bevy::input::mouse::MouseWheel>>()
            .write(bevy::input::mouse::MouseWheel {
                unit: bevy::input::mouse::MouseScrollUnit::Line,
                x: 0.0,
                y: -1.0,
                window: Entity::PLACEHOLDER,
            });
        app.update();
        assert!(
            app.world()
                .resource::<KaleidoscopeScroll>()
                .system_window_start
                .is_some(),
            "wheel set an override"
        );

        // A DOWN keypress moves the cursor and clears the override.
        let mut frame = MenuControlFrame::default();
        frame.down = true;
        app.insert_resource(frame);
        app.update();
        assert_eq!(
            app.world()
                .resource::<KaleidoscopeScroll>()
                .system_window_start,
            None,
            "keyboard nav resumes cursor-follow scrolling (Features C/D)"
        );
    }

    // ---- Feature E: tap activates, drag-away cancels --------------------------

    /// Build a control + fire a Press at `press_pos`, a Move at `move_pos`, then a
    /// Release — exactly the mouse/touch sequence Bevy picking produces. Returns the
    /// `KaleidoscopeSystemNav.open_entry` after, so the test can see whether the
    /// release's drill-in action fired (activated) or was cancelled by a drag.
    fn press_move_click(app: &mut App, press_pos: Vec2, move_pos: Vec2) -> Entity {
        let entity = app
            .world_mut()
            .spawn(AmbitionMenuControl::<MenuPageAction> {
                kind: ambition_menu::MenuControlKind::OptionToggle,
                action: Some(MenuPageAction::OpenSystemEntry(SystemMenuEntryId::Video)),
                focus: ambition_menu::MenuFocusKey::default(),
            })
            .id();
        let loc = |p: Vec2| Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: p,
        };
        app.world_mut().trigger(Pointer::new(
            PointerId::Mouse,
            loc(press_pos),
            Press {
                button: bevy::picking::pointer::PointerButton::Primary,
                hit: HitData::new(entity, 0.0, None, None),
            },
            entity,
        ));
        app.world_mut().trigger(Pointer::new(
            PointerId::Mouse,
            loc(move_pos),
            Move {
                hit: HitData::new(entity, 0.0, None, None),
                delta: move_pos - press_pos,
            },
            entity,
        ));
        app.world_mut().trigger(Pointer::new(
            PointerId::Mouse,
            loc(move_pos),
            Release {
                button: bevy::picking::pointer::PointerButton::Primary,
                hit: HitData::new(entity, 0.0, None, None),
            },
            entity,
        ));
        app.update();
        entity
    }

    /// Feature E: a clean tap (press + tiny move + release under the drag threshold)
    /// ACTIVATES the control; a press + drag-away beyond the threshold CANCELS it.
    #[test]
    fn tap_activates_drag_away_cancels() {
        // Clean tap: tiny move -> drill into Video.
        let (mut app, _player) = click_app();
        // The control's drill-in action needs an active System page for the click
        // dispatch to resolve OpenSystemEntry against the live model.
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::System);
        press_move_click(&mut app, Vec2::new(10.0, 10.0), Vec2::new(12.0, 11.0));
        assert_eq!(
            app.world().resource::<KaleidoscopeSystemNav>().open_entry,
            Some(SystemMenuEntryId::Video),
            "a clean tap activates the control (Feature E)"
        );

        // Drag away: a large move past the threshold -> NO activation.
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::System);
        press_move_click(&mut app, Vec2::new(10.0, 10.0), Vec2::new(200.0, 200.0));
        assert_eq!(
            app.world().resource::<KaleidoscopeSystemNav>().open_entry,
            None,
            "a press-then-drag-away is cancelled, not activated (Feature E)"
        );
    }

    /// Spawn a real control carrying `action` and fire a `Pointer<Press>` on it
    /// (arming the guard via the real press handler), returning its entity.
    fn arm_press(app: &mut App, action: MenuPageAction) -> Entity {
        let entity = spawn_control(app, action);
        trigger_press(app, entity);
        app.update();
        entity
    }

    /// Fire a `Pointer<Release>` whose hit/target is `entity` (which may be despawned).
    fn fire_release(app: &mut App, entity: Entity) {
        trigger_release(app, entity);
        app.update();
    }

    /// THE KEY TEST. The GUI failure exactly: a press is armed on a control, then the
    /// perspective cube REBUILDS its faces (despawns + respawns every control) BEFORE
    /// the release lands. With the old `Pointer<Click>` observer this dropped the
    /// activation (press/release no longer resolved to the same live entity). The new
    /// release-dispatch path stores the action at PRESS time, so it survives the
    /// rebuild: the release still equips the item.
    #[test]
    fn release_dispatch_survives_a_control_rebuild_between_press_and_release() {
        let (mut app, _player) = click_app();
        {
            let mut owned = app.world_mut().resource_mut::<OwnedItems>();
            owned.grant(Item::Blink, 1);
        }
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::Items);
        app.update();
        assert!(
            !app.world()
                .resource::<OwnedItems>()
                .is_equipped(Item::Blink),
            "precondition: Blink not equipped yet"
        );

        // 1. Arm a press on the Blink control.
        let pressed = arm_press(&mut app, MenuPageAction::Equip(Item::Blink));
        assert_eq!(
            app.world().resource::<KaleidoscopePointerPress>().action,
            Some(MenuPageAction::Equip(Item::Blink)),
            "the press armed the control's action in the guard"
        );

        // 2. Simulate a face rebuild: despawn EVERY control (incl. the pressed one)
        //    and respawn a fresh one with a NEW entity id, exactly like the cube does
        //    on a hover-driven republish.
        {
            let to_despawn: Vec<Entity> = app
                .world_mut()
                .query_filtered::<Entity, With<AmbitionMenuControl<MenuPageAction>>>()
                .iter(app.world())
                .collect();
            for e in to_despawn {
                app.world_mut().entity_mut(e).despawn();
            }
            app.world_mut()
                .spawn(AmbitionMenuControl::<MenuPageAction> {
                    kind: ambition_menu::MenuControlKind::OptionToggle,
                    action: Some(MenuPageAction::Equip(Item::Blink)),
                    focus: ambition_menu::MenuFocusKey::default(),
                });
        }
        assert!(
            app.world().get_entity(pressed).is_err(),
            "the pressed entity is gone after the rebuild (this is what broke Pointer<Click>)"
        );

        // 3. Release on the now-DEAD pressed entity. The handler dispatches the action
        //    stored at press time, not the release entity — so it still equips.
        fire_release(&mut app, pressed);
        assert!(
            app.world()
                .resource::<OwnedItems>()
                .is_equipped(Item::Blink),
            "release dispatches the action armed at press time even after the control \
             was despawned + respawned between press and release (the GUI mouse-click fix)"
        );
    }

    /// A plain press→release on a live control activates (the common case).
    #[test]
    fn press_then_release_equips_an_item() {
        let (mut app, _player) = click_app();
        {
            let mut owned = app.world_mut().resource_mut::<OwnedItems>();
            owned.grant(Item::Blink, 1);
        }
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::Items);
        app.update();
        let entity = arm_press(&mut app, MenuPageAction::Equip(Item::Blink));
        fire_release(&mut app, entity);
        assert!(
            app.world()
                .resource::<OwnedItems>()
                .is_equipped(Item::Blink),
            "a clean press→release on an item control equips it"
        );
    }

    // ---- CURSOR HIGHLIGHT regression -----------------------------------------

    /// Build an app with the real lib cube plugin so `rebuild_cube_faces` spawns
    /// REAL controls (with their `MenuVisualState`, `KaleidoscopeControlStyle`, and
    /// HIDDEN `SelectionCorner` children), wire the sandbox focus writer + the lib
    /// focus readers, publish the Items page with one owned item, and grant that item.
    fn highlight_app(owned_item: Item) -> App {
        highlight_app_ordered(owned_item, true)
    }

    /// `writer_first = true` mirrors a correctly-ordered chain (writer before the
    /// lib `Changed` readers). `writer_first = false` reproduces the REAL app's
    /// hazard: the lib readers (added by the plugin as plain unordered `Update`
    /// systems) can run BEFORE the sandbox writer, so the `Changed<MenuVisualState>`
    /// the writer raises is consumed one frame too late — and the writer is
    /// change-detection-gated, so it never re-raises it. The highlight never shows.
    fn highlight_app_ordered(owned_item: Item, writer_first: bool) -> App {
        use ambition_menu::kaleidoscope::{
            sync_control_focus_visuals, sync_selection_corner_visuals,
        };
        // The icon asset loads (`AssetServer::load`) need the IO task pool.
        bevy::tasks::IoTaskPool::get_or_init(Default::default);
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Mesh>();
        app.init_asset::<Image>();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameMode>();
        // Resources the host systems read.
        app.init_resource::<InventoryUiBackend>();
        app.init_resource::<ActiveMenuPages<MenuPage, MenuPageAction>>();
        app.init_resource::<KaleidoscopeCursor>();
        app.init_resource::<ambition_sandbox::input::ActiveInputKind>();
        app.init_resource::<KaleidoscopeSystemNav>();
        app.init_resource::<KaleidoscopeScroll>();
        app.init_resource::<KaleidoscopePointerPress>();
        let mut owned = OwnedItems::default();
        owned.grant(owned_item, 1);
        app.insert_resource(owned);
        app.init_resource::<ambition_sandbox::dev::dev_tools::DeveloperTools>();
        app.init_resource::<ambition_sandbox::SandboxDevState>();
        app.init_resource::<ambition_sandbox::ldtk_world::LdtkHotReloadState>();
        app.init_resource::<ambition_sandbox::runtime::reset::SandboxResetRequested>();
        app.init_resource::<ambition_sandbox::dev::dev_tools::EditableMovementTuning>();
        app.init_resource::<UserSettings>();
        app.init_resource::<ambition_sandbox::inventory::InventoryUiState>();
        app.add_message::<SfxMessage>();
        *app.world_mut().resource_mut::<InventoryUiBackend>() =
            InventoryUiBackend::LunexKaleidoscope;
        app.world_mut()
            .resource_mut::<ambition_sandbox::inventory::InventoryUiState>()
            .visible = true;

        // The lib's ring root that `rebuild_cube_faces` parents faces under. We spawn
        // it directly (the plugin's `setup_cube` would also add a Camera3d we don't
        // need headlessly).
        app.world_mut().spawn((
            ambition_menu::AmbitionMenuRoot,
            ambition_menu::kaleidoscope::MenuRing,
            Transform::default(),
            Visibility::Visible,
        ));
        app.insert_resource(KaleidoscopeMenuConfig {
            draw_nav_arrows: false,
            pickable_controls: true,
            ..Default::default()
        });

        // Wire it like the REAL app does. The sandbox writer lives in its own chain;
        // the lib `Changed<MenuVisualState>` readers + the rebuild are added as plain,
        // UNORDERED `Update` systems (exactly as `KaleidoscopeMenuPlugin::build` adds
        // them). `writer_first` forces the writer to run before the readers (the fixed
        // ordering); `!writer_first` leaves them unordered so the readers may be
        // scheduled BEFORE the writer (the regression hazard).
        app.add_systems(
            Update,
            ambition_menu::kaleidoscope::rebuild_cube_faces::<MenuPage, MenuPageAction>,
        );
        if writer_first {
            // The FIX: republish + the host focus writer run AFTER the lib rebuild (so
            // the writer always writes to the freshly (re)spawned controls), and the
            // lib `Changed` readers run AFTER the writer (so they see the flipped flags
            // the same frame). This is the ordering `install_kaleidoscope_menu` +
            // `KaleidoscopeMenuPlugin` declare on the real app.
            app.add_systems(
                Update,
                (
                    republish_kaleidoscope_pages,
                    kaleidoscope_sync_focus_visuals,
                )
                    .chain()
                    .after(
                        ambition_menu::kaleidoscope::rebuild_cube_faces::<MenuPage, MenuPageAction>,
                    ),
            );
            app.add_systems(
                Update,
                (sync_control_focus_visuals, sync_selection_corner_visuals)
                    .after(kaleidoscope_sync_focus_visuals),
            );
        } else {
            // The REGRESSION wiring: nothing orders the host writer against the lib
            // rebuild, so `rebuild_cube_faces` can despawn+respawn controls (resetting
            // `MenuVisualState` to focused:false) AFTER the writer flipped them, and the
            // `Changed` readers run before the writer. The highlight is dropped.
            app.add_systems(
                Update,
                (
                    republish_kaleidoscope_pages,
                    kaleidoscope_sync_focus_visuals,
                )
                    .chain(),
            );
            app.add_systems(
                Update,
                (sync_control_focus_visuals, sync_selection_corner_visuals)
                    .before(kaleidoscope_sync_focus_visuals),
            );
        }

        // Publish the Items page (one frame to spawn the controls/corners).
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active = Some(MenuPage::Items);
        let pages = build_inventory_pages(
            &app.world().resource::<OwnedItems>().clone(),
            None,
            MenuFocus::Item(owned_item.index()),
            &app.world().resource::<UserSettings>().clone(),
            &RadioSnapshot::default(),
            &DevSnapshot::default(),
            0,
            None,
        );
        app.world_mut()
            .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .replace_pages(pages, MenuPage::Items);
        app.update();
        app
    }

    /// REGRESSION pin: setting the cursor onto an owned item's focus must (a) flip
    /// that control's `MenuVisualState.focused`, (b) make its `SelectionCorner`
    /// children VISIBLE, and (c) leave a non-focused control's corners HIDDEN.
    #[test]
    fn cursor_focus_highlights_the_control_and_reveals_its_corners() {
        let item = Item::PortalGun;
        let mut app = highlight_app(item);
        set_focus_and_step(&mut app, item, 1);
        assert_highlight_visible(&mut app, item);
    }

    /// REGRESSION reproduction: when the host republishes (a hover, a late texture
    /// load, an inventory change — all common in-game), `rebuild_cube_faces` despawns
    /// and respawns every control with a fresh `MenuVisualState { focused: false }`.
    /// With the UN-ordered wiring (lib rebuild + `Changed` readers added as plain
    /// `Update` systems, nothing ordering them against the host focus writer), that
    /// rebuild can run AFTER the writer flipped the focus flag, wiping it — and the
    /// `Changed` readers run before the writer — so the corners never show. The FIXED
    /// ordering (`cursor_focus_*`) keeps the writer after the rebuild and the readers
    /// after the writer, so the highlight survives a same-frame republish.
    #[test]
    fn republish_during_focus_keeps_the_highlight_under_fixed_ordering() {
        let item = Item::PortalGun;

        // Fixed ordering: a republish on the focus frame must NOT drop the highlight.
        let mut fixed = highlight_app_ordered(item, /* writer_first */ true);
        force_republish_and_focus(&mut fixed, item);
        assert_highlight_visible(&mut fixed, item);

        // Un-ordered (regression) wiring: the same republish drops it.
        let mut broken = highlight_app_ordered(item, /* writer_first */ false);
        force_republish_and_focus(&mut broken, item);
        let focus = MenuFocus::Item(item.index());
        let model = SystemMenuModel::build(
            &broken.world().resource::<UserSettings>().clone(),
            &RadioSnapshot::default(),
            &DevSnapshot::default(),
        );
        let world = broken.world_mut();
        let mut q = world.query::<(&AmbitionMenuControl<MenuPageAction>, &MenuVisualState)>();
        let highlighted = q.iter(world).any(|(c, vis)| {
            c.action
                .map(|a| focus_for_action(a, MenuPage::Items, &model, None) == focus)
                .unwrap_or(false)
                && vis.focused
        });
        assert!(
            !highlighted,
            "documents the regression: un-ordered wiring drops the highlight when a \
             republish rebuilds the controls on the focus frame"
        );
    }

    /// Set the cursor onto `item` AND force a host republish the same frame (bump the
    /// page version so `rebuild_cube_faces` despawns+respawns the controls), then run
    /// one frame — exactly the in-game hover / texture-load / inventory-change churn.
    fn force_republish_and_focus(app: &mut App, item: Item) {
        app.world_mut().resource_mut::<KaleidoscopeCursor>().focus = MenuFocus::Item(item.index());
        // Mark the inventory changed so `republish_kaleidoscope_pages` rebuilds.
        app.world_mut().resource_mut::<OwnedItems>().set_changed();
        app.update();
    }

    /// Set the cursor onto `item`'s focus and run `frames` updates.
    fn set_focus_and_step(app: &mut App, item: Item, frames: usize) {
        let focus = MenuFocus::Item(item.index());
        app.world_mut().resource_mut::<KaleidoscopeCursor>().focus = focus;
        for _ in 0..frames {
            app.update();
        }
    }

    /// Assert the highlight is visible for `item`: (a) its control's
    /// `MenuVisualState.focused`, (b) its corners Visible, (c) others' corners Hidden.
    fn assert_highlight_visible(app: &mut App, item: Item) {
        let focus = MenuFocus::Item(item.index());
        // Find the control whose action maps to the focused item.
        let active_page = MenuPage::Items;
        let model = SystemMenuModel::build(
            &app.world().resource::<UserSettings>().clone(),
            &RadioSnapshot::default(),
            &DevSnapshot::default(),
        );
        let world = app.world_mut();
        let mut focused_control = None;
        let mut other_control = None;
        let mut q = world.query::<(
            Entity,
            &AmbitionMenuControl<MenuPageAction>,
            &MenuVisualState,
        )>();
        let rows: Vec<(Entity, bool, bool)> = q
            .iter(world)
            .filter_map(|(e, c, vis)| {
                let action = c.action?;
                let f = focus_for_action(action, active_page, &model, None);
                Some((e, f == focus, vis.focused))
            })
            .collect();
        for (e, is_focused, vis_focused) in rows {
            if is_focused {
                focused_control = Some((e, vis_focused));
            } else if other_control.is_none() {
                other_control = Some(e);
            }
        }
        let (focused_entity, vis_focused) =
            focused_control.expect("a control maps to the focused item");
        assert!(
            vis_focused,
            "(a) the focused control's MenuVisualState.focused must be true"
        );

        // (b) the focused control's selection corners are VISIBLE.
        let corners_visible = corner_visibilities(world, focused_entity);
        assert!(
            !corners_visible.is_empty(),
            "the focused control must have SelectionCorner children"
        );
        assert!(
            corners_visible.iter().all(|v| *v == Visibility::Visible),
            "(b) focused control's corners must be Visible, got {corners_visible:?}"
        );

        // (c) a non-focused control's corners stay HIDDEN.
        let other = other_control.expect("a non-focused control exists");
        let other_corners = corner_visibilities(world, other);
        assert!(
            other_corners.iter().all(|v| *v == Visibility::Hidden),
            "(c) non-focused control's corners must be Hidden, got {other_corners:?}"
        );
    }

    /// Collect the `Visibility` of the `SelectionCorner`-style children of a control.
    /// Corners are the lib's hidden bracket meshes; identify them as children that are
    /// neither text nor icon (they carry a `UiMeshPlane3d` + `Visibility` and no
    /// `Text3d`). We match on the lib-set Name "selection corner".
    fn corner_visibilities(world: &mut World, control: Entity) -> Vec<Visibility> {
        let children: Vec<Entity> = world
            .get::<Children>(control)
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        children
            .into_iter()
            .filter(|&c| {
                world
                    .get::<Name>(c)
                    .map(|n| n.as_str() == "selection corner")
                    .unwrap_or(false)
            })
            .filter_map(|c| world.get::<Visibility>(c).copied())
            .collect()
    }

    /// Proves the host's `kaleidoscope_render_needed` gate disables cube rendering
    /// for Grid or settled-closed menus, while still allowing visible cube menus.
    #[test]
    fn render_set_is_gated_off_under_the_grid_backend() {
        #[derive(Resource, Default)]
        struct RenderRan(u32);

        fn build(backend: InventoryUiBackend, menu_visible: bool) -> App {
            let mut app = App::new();
            app.init_resource::<InventoryUiBackend>();
            app.init_resource::<RenderRan>();
            *app.world_mut().resource_mut::<InventoryUiBackend>() = backend;
            let mut ui_state = ambition_sandbox::inventory::InventoryUiState::default();
            ui_state.visible = menu_visible;
            app.insert_resource(ui_state);
            // Exactly the host's gating from `install_kaleidoscope_menu`.
            app.configure_sets(
                Update,
                KaleidoscopeRender.run_if(kaleidoscope_render_needed),
            );
            // A stand-in for the lib's cube render systems (rebuild/animate/fade…),
            // which all live in `KaleidoscopeRender`.
            app.add_systems(
                Update,
                (|mut ran: ResMut<RenderRan>| ran.0 += 1).in_set(KaleidoscopeRender),
            );
            app
        }

        let mut grid = build(InventoryUiBackend::Grid, true);
        grid.update();
        grid.update();
        assert_eq!(
            grid.world().resource::<RenderRan>().0,
            0,
            "cube render set must NOT run when the Grid backend is active"
        );

        let mut cube = build(InventoryUiBackend::LunexKaleidoscope, true);
        cube.update();
        cube.update();
        assert_eq!(
            cube.world().resource::<RenderRan>().0,
            2,
            "cube render set runs every frame while the cube menu is visible"
        );

        // P4b settle early-out: closed + no fade in flight = the set is skipped.
        let mut closed = build(InventoryUiBackend::LunexKaleidoscope, false);
        closed.update();
        closed.update();
        assert_eq!(
            closed.world().resource::<RenderRan>().0,
            0,
            "cube render set must NOT run when the menu is closed and settled"
        );
    }
}
