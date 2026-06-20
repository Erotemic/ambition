//! Game-side hookup for the 3D-cube OoT pause menu (#31): adds the lib's reusable
//! cube renderer ([`ambition_menu::kaleidoscope::KaleidoscopeMenuPlugin`]) and feeds it our
//! live 24-item inventory (via [`crate::menu::model`]). Runtime-toggleable vs the
//! existing Bevy-UI grid through [`InventoryUiBackend`].
//!
//! The cube is pause-gated ([`gate_kaleidoscope_menu`]): its order-8 `Camera3d` + ring are
//! only active while the inventory is open, so it never clears the screen to black
//! during play. Routing nav/selection input to it is the next step — see
//! `dev/journals/oot-cube-integration-plan.md`.

use ambition_gameplay_core::menu::backend::{
    InventoryUiBackend, KALEIDOSCOPE_MENU_BACKEND_ENABLED,
};
use ambition_menu::kaleidoscope::{
    rebuild_cube_faces, KaleidoscopeActiveFaceControl, KaleidoscopeFocusVisuals,
    KaleidoscopeMenuConfig, KaleidoscopeMenuPlugin, KaleidoscopeRender, KaleidoscopeRenderPre,
};
use ambition_menu::{
    ActiveMenuPages, AmbitionInventoryUiPlugin, AmbitionMenuControl, MenuDynamicText,
    MenuDynamicTextContent, MenuVisualState,
};
use bevy::prelude::*;

use crate::menu::effects::{MenuEffectManaQuery, MenuEffectPlayers};
use crate::menu::model::{
    build_inventory_pages, items_detail_slot_text, scroll_fraction_to_window_start,
    system_detail_slot_text, system_effective_window_start, system_max_window_start, system_rows,
    MenuFocus, MenuPage, MenuPageAction, SystemRow, SYSTEM_VISIBLE_ROWS,
};
use ambition_gameplay_core::audio::SfxMessage;
use ambition_gameplay_core::engine_core::Vec2;
use ambition_gameplay_core::input::MenuControlFrame;
use ambition_gameplay_core::items::{Item, OwnedItems, ITEM_GRID_COLS, ITEM_GRID_ROWS};
use ambition_gameplay_core::persistence::settings::{
    apply_settings_option, settings_menu_model, DevSnapshot, DevToggleId, RadioSnapshot,
    SettingsOptionId, SettingsOptionKind, SystemMenuAction, SystemMenuEntryId, SystemMenuModel,
    SystemOptionId, UserSettings,
};
use ambition_gameplay_core::player::PlayerHealRequested;

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
        .init_resource::<ambition_gameplay_core::input::ActiveInputKind>()
        .init_resource::<KaleidoscopeSystemNav>()
        .init_resource::<CachedSystemMenu>()
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
    ui_state: Option<Res<ambition_gameplay_core::inventory_ui::InventoryUiState>>,
) -> bool {
    KALEIDOSCOPE_MENU_BACKEND_ENABLED
        && backend.effective() == InventoryUiBackend::LunexKaleidoscope
        && ui_state.map(|s| s.visible).unwrap_or(false)
}

/// The cube renderer needs to tick while open and briefly while folding closed.
///
/// This avoids the original closed-menu churn (camera/ring/picking/fade systems
/// running every frame just because Cube is the selected backend) without cutting
/// off the close animation — including the close triggered by SWITCHING backends.
fn kaleidoscope_render_needed(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<ambition_gameplay_core::inventory_ui::InventoryUiState>>,
    open_state: Option<Res<ambition_menu::kaleidoscope::KaleidoscopeOpenState>>,
) -> bool {
    let (target, amount) = open_state
        .map(|s| (s.target, s.amount))
        .unwrap_or((0.0, 0.0));
    cube_render_needed(
        KALEIDOSCOPE_MENU_BACKEND_ENABLED,
        backend.effective() == InventoryUiBackend::LunexKaleidoscope,
        ui_state.map(|s| s.visible).unwrap_or(false),
        target,
        amount,
    )
}

/// Pure decision for [`kaleidoscope_render_needed`]: should the cube's render set tick
/// this frame? Either it's actively open (Cube backend selected AND the menu is up),
/// OR it's still folding shut (`target`/`amount` not yet decayed) — the latter holds
/// REGARDLESS of backend, so switching Cube→Grid keeps `animate_cube_ring` running
/// long enough to ease `amount` to 0 and let the camera turn off.
fn cube_render_needed(
    enabled: bool,
    backend_is_cube: bool,
    menu_open: bool,
    target: f32,
    amount: f32,
) -> bool {
    if !enabled {
        return false;
    }
    if backend_is_cube && menu_open {
        return true;
    }
    target > 0.0 || amount > 0.08
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
pub(crate) struct KaleidoscopeScrim;

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
        // PHASE 1 — decide this frame's CONTENT, BEFORE the lib rebuild. Nav mutates
        // the cursor; scroll/cache/republish derive the page model + System scroll
        // window from it. Running republish BEFORE `rebuild_cube_faces` means a move
        // that SHIFTS the System scroll window rebuilds the new rows the SAME frame as
        // the highlight moves — eliminating the one-frame "highlight jumps, then the
        // list scrolls" flash (republish used to run AFTER the rebuild, so the shifted
        // window was not drawn until the next frame).
        .add_systems(
            Update,
            (
                // Route pause/Esc, inventory, and map into the cube backend on the
                // matching page before navigation consumes the frame. In
                // MenuNavConsume so `fold_to_menu_control_frame`'s
                // `.before(MenuNavConsume)` guarantees this sees the touch Menu-button
                // press AFTER the fold writes the pressed_this_frame bit.
                kaleidoscope_menu_open_routing
                    .run_if(kaleidoscope_backend_active)
                    .in_set(ambition_gameplay_core::schedule::MenuNavConsume),
                // Nav first (mutates the cursor), then republish (reads the cursor +
                // inventory) so the highlight + detail panel reflect this frame's move.
                // Also in `MenuNavConsume` for the same fold-ordering reason above.
                kaleidoscope_focus_nav
                    .run_if(kaleidoscope_menu_visible)
                    .in_set(ambition_gameplay_core::schedule::MenuNavConsume),
                // Scroll the System window independently of selection before republish.
                kaleidoscope_scroll_wheel.run_if(kaleidoscope_menu_visible),
                kaleidoscope_apply_scroll_drag.run_if(kaleidoscope_menu_visible),
                // Build the System model + radio/dev snapshots ONCE per frame; the
                // republish + both in-place sync systems read this cache instead of
                // each rebuilding it (3 heavy builds/frame → 1 on the System face).
                cache_system_menu.run_if(kaleidoscope_menu_visible),
                // Republish the cube's model only for the cube backend; Grid builds and
                // dirties its own page model.
                republish_kaleidoscope_pages.run_if(kaleidoscope_menu_visible),
            )
                .chain()
                .before(rebuild_cube_faces::<MenuPage, MenuPageAction>),
        )
        // PHASE 2 — reflect that content, AFTER the lib rebuild. The focus HIGHLIGHT
        // and the detail-panel TEXT update IN PLACE from the live cursor over the
        // freshly (re)built controls.
        .add_systems(
            Update,
            (
                kaleidoscope_sync_focus_visuals.run_if(kaleidoscope_menu_visible),
                kaleidoscope_sync_detail_text.run_if(kaleidoscope_menu_visible),
                gate_kaleidoscope_menu,
                cycle_dev_gravity,
                // The readability dim-scrim is cube-only; the Startup node stays
                // transparent when the cube backend is inactive.
                retarget_kaleidoscope_scrim.run_if(kaleidoscope_render_needed),
                fade_kaleidoscope_scrim.run_if(kaleidoscope_render_needed),
            )
                .chain()
                // CURSOR-HIGHLIGHT fix: the lib renders the focus highlight from
                // `MenuVisualState` via the `Changed`-gated `KaleidoscopeFocusVisuals`
                // readers. Run the `kaleidoscope_sync_focus_visuals` WRITER AFTER
                // `rebuild_cube_faces` (so a republish that respawns the controls can't
                // wipe the flags it set) and BEFORE the lib readers (so they see the
                // flipped flags the same frame).
                .after(rebuild_cube_faces::<MenuPage, MenuPageAction>)
                .before(KaleidoscopeFocusVisuals),
        )
        .add_observer(kaleidoscope_pointer_press)
        .add_observer(kaleidoscope_pointer_move)
        .add_observer(kaleidoscope_pointer_release);
}

/// Which input source currently owns the cube cursor. Mirrors the grid's
/// [`ambition_gameplay_core::ui_nav::MenuFocusOwner`]: keyboard/gamepad nav claims focus and keeps
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
pub(crate) struct KaleidoscopeScroll {
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
    dev_tools: ResMut<'w, ambition_gameplay_core::dev::dev_tools::DeveloperTools>,
    // The Developer screen also reaches the F1/F2 global flags + F12 LDtk
    // auto-reload, which live on these two resources (not `DeveloperTools`).
    dev_state: ResMut<'w, ambition_gameplay_core::SandboxDevState>,
    ldtk_reload: ResMut<'w, ambition_gameplay_core::ldtk_world::LdtkHotReloadState>,
    // The active menu frontend, mutated by the Developer "Menu Backend" row (the
    // in-menu `\` toggle). Always present (inserted at startup).
    backend: ResMut<'w, InventoryUiBackend>,
    // The Portal FX cycle's target (portal presentation crate, host adapter).
    // Option: absent in non-portal personas / minimal fixtures — the row then
    // no-ops and reads "n/a".
    #[cfg(feature = "portal_render")]
    portal_effect: Option<ResMut<'w, ambition_gameplay_core::portal::PortalEffectSelection>>,
    // The Gravity cycle's target (ambient gravity). Option so the System nav stays
    // B0002-safe and fixtures without the resource render the row as "n/a".
    base_gravity: Option<ResMut<'w, ambition_gameplay_core::physics::BaseGravity>>,
    reset: ResMut<'w, ambition_gameplay_core::session::reset::SandboxResetRequested>,
    // Movement tuning is derived from the active movement profile, so a
    // Reset All Settings must restore it to match the reset DeveloperTools
    // defaults (mirrors the pause menu's `ResetAllSettings`).
    editable_tuning: ResMut<'w, ambition_gameplay_core::dev::dev_tools::EditableMovementTuning>,
    // The radio resources are `Option`-wrapped so the System nav stays B0002-safe
    // and never panics when audio is off / a fixture omits them: a missing radio
    // resource simply disables station audition (the rows still render). Gated on
    // `audio` so non-audio builds carry none of the types.
    #[cfg(feature = "audio")]
    library: Option<ResMut<'w, ambition_gameplay_core::audio::AudioLibrary>>,
    #[cfg(feature = "audio")]
    asset_server: Option<Res<'w, AssetServer>>,
    #[cfg(feature = "audio")]
    music_state: Option<ResMut<'w, ambition_gameplay_core::audio::MusicPlaybackState>>,
    #[cfg(feature = "audio")]
    radio: Option<ResMut<'w, ambition_gameplay_core::audio::RadioStationState>>,
    #[cfg(feature = "audio")]
    music_channel: Option<
        Res<
            'w,
            bevy_kira_audio::prelude::AudioChannel<ambition_gameplay_core::audio::MusicChannel>,
        >,
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
                        ambition_gameplay_core::audio::set_radio_track(
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
                        base_gravity: self.base_gravity.as_deref_mut(),
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
                        base_gravity: self.base_gravity.as_deref_mut(),
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
        *self.dev_tools = ambition_gameplay_core::dev::dev_tools::DeveloperTools::default();
        ambition_gameplay_core::dev::dev_tools::apply_movement_profile(
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
            base_gravity: self.base_gravity.as_deref(),
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
    state: Res<'w, State<ambition_gameplay_core::session::game_mode::GameMode>>,
    next: ResMut<'w, NextState<ambition_gameplay_core::session::game_mode::GameMode>>,
}

/// Resources `republish_kaleidoscope_pages` reads (immutably) to snapshot the radio + dev
/// state into the SYSTEM IR. Separate `Res` bundle so it never conflicts with the
/// mutable `SystemMenuParams` (different systems).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct SystemMenuSnapshotParams<'w> {
    dev_tools: Res<'w, ambition_gameplay_core::dev::dev_tools::DeveloperTools>,
    dev_state: Res<'w, ambition_gameplay_core::SandboxDevState>,
    ldtk_reload: Res<'w, ambition_gameplay_core::ldtk_world::LdtkHotReloadState>,
    backend: Res<'w, InventoryUiBackend>,
    #[cfg(feature = "portal_render")]
    portal_effect: Option<Res<'w, ambition_gameplay_core::portal::PortalEffectSelection>>,
    base_gravity: Option<Res<'w, ambition_gameplay_core::physics::BaseGravity>>,
    #[cfg(feature = "audio")]
    library: Option<Res<'w, ambition_gameplay_core::audio::AudioLibrary>>,
    #[cfg(feature = "audio")]
    music_state: Option<Res<'w, ambition_gameplay_core::audio::MusicPlaybackState>>,
    #[cfg(feature = "audio")]
    radio: Option<Res<'w, ambition_gameplay_core::audio::RadioStationState>>,
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
            base_gravity: self.base_gravity.as_deref(),
        })
    }
}

/// Build a [`RadioSnapshot`] from the live audio library + playback state. The
/// single place that maps the audio runtime onto the SYSTEM IR's station list.
#[cfg(feature = "audio")]
fn radio_snapshot_from(
    library: &ambition_gameplay_core::audio::AudioLibrary,
    music_state: &ambition_gameplay_core::audio::MusicPlaybackState,
    radio: Option<&ambition_gameplay_core::audio::RadioStationState>,
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

mod dev_toggles;
use dev_toggles::*;

mod scrim;
pub(crate) use scrim::*;

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
    mut menu_frame: ResMut<MenuControlFrame>,
    mut cursor: ResMut<KaleidoscopeCursor>,
    mut system_nav: ResMut<KaleidoscopeSystemNav>,
    // Features C/D: keyboard navigation CLEARS the explicit scroll override so the
    // System window resumes following the selection cursor (the wheel/drag set it).
    mut scroll: ResMut<KaleidoscopeScroll>,
    mut pages: ResMut<ActiveMenuPages<MenuPage, MenuPageAction>>,
    // Single mutable access to the overlay state — also read `.visible` from it (a
    // separate `Res<InventoryUiState>` would be a B0002 conflict with this `ResMut`).
    mut overlay: ResMut<ambition_gameplay_core::inventory_ui::InventoryUiState>,
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
        // Not the active backend — leave the frame for whichever nav owns it.
        return;
    }
    // This frame's menu navigation belongs to the cube now: snapshot it, then CONSUME
    // the one-shot nav edges so the Grid backend's nav (which shares this `Res` in the
    // same frame) can't re-fire the same press if the "Menu Backend" row flips
    // `InventoryUiBackend` mid-frame (the cause of the unreliable in-menu toggle).
    let menu = *menu_frame;
    menu_frame.consume_nav_edges();
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
            // The cube's System face is a closed list — UP/DOWN wrap around.
            true,
        );
        return;
    }

    // Other non-items faces (Map / Quest placeholders) respond to horizontal page
    // turns; arrows rotate, landing the cursor on the new page's back-edge button
    // (Fix 1). The L/R bumpers (Fix 2) are already handled above for every face.
    if active_page != MenuPage::Items {
        // Placeholder faces (Map / Quest) have only the two edge buttons and no centre
        // content, so edge nav is the whole story: `edge_button_nav` rotates on an
        // OUTWARD step / select and crosses to the opposite edge on an INWARD step. The
        // only thing left to do locally is seed onto an edge when the cursor hasn't
        // landed on one yet (rare — these faces spawn the cursor on an edge).
        if edge_button_nav(
            &mut cursor,
            &mut pages,
            active_page,
            dx,
            menu.select,
            true,
            EdgeInward::OppositeEdge,
            &mut sfx,
        ) == EdgeNav::NotOnEdge
            && dx != 0
        {
            cursor.mark_keyboard(if dx < 0 {
                MenuFocus::EdgeLeft
            } else {
                MenuFocus::EdgeRight
            });
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
/// Next row index after a vertical nav step over a `count`-row System list.
/// `wrap_rows` selects the cube's closed-list wrap (UP off the top → bottom row,
/// DOWN off the bottom → top) versus the Grid's clamp (its rows sit below the tab
/// bar, a real target UP must reach). `count` is always ≥ 1, so `rem_euclid` is safe.
fn step_system_row(row: i32, dy: i32, count: i32, wrap_rows: bool) -> i32 {
    if wrap_rows {
        (row + dy).rem_euclid(count)
    } else {
        (row + dy).clamp(0, count - 1)
    }
}

pub(crate) fn system_focus_nav(
    menu: &MenuControlFrame,
    dx: i32,
    dy: i32,
    cursor: &mut KaleidoscopeCursor,
    system_nav: &mut KaleidoscopeSystemNav,
    pages: &mut ActiveMenuPages<MenuPage, MenuPageAction>,
    overlay: &mut ambition_gameplay_core::inventory_ui::InventoryUiState,
    mode: &ambition_gameplay_core::session::game_mode::GameMode,
    next_mode: &mut NextState<ambition_gameplay_core::session::game_mode::GameMode>,
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
    // Vertical wrap-around for the row list: UP off the top lands on the bottom row
    // and DOWN off the bottom lands on the top. The cube enables this (its System
    // face is a closed list with nothing above/below the rows); the Grid passes
    // `false` because its rows sit BELOW a real target (the tab bar), so UP off the
    // top must reach that, not wrap. Kept as a distinct flag from `allow_page_turn`
    // so either behaviour can be flipped independently (e.g. trivially reverted).
    wrap_rows: bool,
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
        row = step_system_row(row, dy, count, wrap_rows);
        cursor.mark_keyboard(MenuFocus::System(row as usize));
    }

    let current = rows[row.max(0).min(count - 1) as usize];

    // Back drills OUT / closes — independent of edge vs row, so handle it first.
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

    // `>`/`<` page-turn buttons: SHARED with the placeholder faces (single source, so
    // edge nav can't drift). On the System face an INWARD step enters the row list, and
    // SELECT / an OUTWARD step rotates. Consumes dx + select while the cursor is on an
    // edge; a row cursor falls through to the value/row handling below.
    if edge_button_nav(
        cursor,
        pages,
        active_page,
        dx,
        menu.select,
        allow_page_turn,
        EdgeInward::Into(MenuFocus::System(0)),
        sfx,
    ) == EdgeNav::Handled
    {
        emit_move_sfx(sfx, focus_before, cursor.focus, page_before, pages.active);
        return;
    }

    // The cursor is on a ROW: LEFT/RIGHT step value rows in place (settings cycles/
    // sliders, dev cycles); otherwise use the horizontal affordance to move onto the
    // edge buttons (cube only — the Grid switches pages via its tab bar, so a non-value
    // step there is simply inert).
    if dx != 0 {
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
            cursor.mark_keyboard(if dx < 0 {
                MenuFocus::EdgeLeft
            } else {
                MenuFocus::EdgeRight
            });
        }
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
            Some(ambition_gameplay_core::persistence::settings::SystemMenuTarget::Action(
                action,
            )) => Some(MenuPageAction::SystemAction(*action)),
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

/// Where an INWARD horizontal step FROM a `>`/`<` edge button lands — the only thing
/// that differs between faces, so [`edge_button_nav`] takes it as a parameter.
#[derive(Clone, Copy)]
enum EdgeInward {
    /// The opposite edge button. Placeholder faces (Map/Quest) have no centre content,
    /// so stepping in from one edge crosses straight to the other.
    OppositeEdge,
    /// A fixed focus — the head of the System face's row list.
    Into(MenuFocus),
}

/// Did [`edge_button_nav`] consume this frame's horizontal/select input?
#[derive(PartialEq, Eq, Clone, Copy)]
enum EdgeNav {
    /// The cursor was on an edge button and an edge action ran (rotate or step in).
    Handled,
    /// The cursor was NOT on an edge button (or there was no edge input) — the caller
    /// owns this frame's input for its own centre content (rows / item grid).
    NotOnEdge,
}

/// Shared `>`/`<` page-turn-button navigation for EVERY cube face — the single source
/// the placeholder Map/Quest handler and [`system_focus_nav`] both call, so edge nav
/// can never drift between them (it did: SELECT-on-edge was silently missing on the
/// System face, where it fell through and activated the first row).
///
/// When the cursor sits on an edge button: SELECT or an OUTWARD `dx` rotates to that
/// neighbour, landing on the new face's back-edge (via [`turn_page_seeded`]); an
/// INWARD `dx` (or any `dx` when `allow_page_turn` is false) moves to `inward`.
/// Returns [`EdgeNav::NotOnEdge`] when the cursor isn't on an edge or there's no
/// horizontal/select input this frame, so the caller handles its own content (and
/// vertical moves off an edge still flow through to the caller).
fn edge_button_nav(
    cursor: &mut KaleidoscopeCursor,
    pages: &mut ActiveMenuPages<MenuPage, MenuPageAction>,
    active_page: MenuPage,
    dx: i32,
    select: bool,
    allow_page_turn: bool,
    inward: EdgeInward,
    sfx: &mut MessageWriter<SfxMessage>,
) -> EdgeNav {
    let edge = cursor.focus;
    let on_left = edge == MenuFocus::EdgeLeft;
    let on_right = edge == MenuFocus::EdgeRight;
    if (!on_left && !on_right) || (dx == 0 && !select) {
        return EdgeNav::NotOnEdge;
    }
    // The neighbour this edge turns toward (outward), or None when turns are disabled.
    let outward_page = match (allow_page_turn, on_left) {
        (false, _) => None,
        (true, true) => Some(active_page.on_viewer_left()),
        (true, false) => Some(active_page.on_viewer_right()),
    };
    let inward_focus = match inward {
        EdgeInward::OppositeEdge if on_left => MenuFocus::EdgeRight,
        EdgeInward::OppositeEdge => MenuFocus::EdgeLeft,
        EdgeInward::Into(f) => f,
    };
    // SELECT activates the edge button = rotate (no-op if turns are disabled).
    if select {
        if let Some(target) = outward_page {
            turn_page_seeded(pages, cursor, target, sfx);
        }
        return EdgeNav::Handled;
    }
    // Horizontal: stepping OUTWARD past the edge rotates; INWARD moves to `inward`.
    let going_outward = if on_left { dx < 0 } else { dx > 0 };
    match outward_page {
        Some(target) if going_outward => turn_page_seeded(pages, cursor, target, sfx),
        _ => cursor.mark_keyboard(inward_focus),
    }
    EdgeNav::Handled
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

mod pointer;
pub(crate) use pointer::*;

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
    mut overlay: ResMut<ambition_gameplay_core::inventory_ui::InventoryUiState>,
    mode: Res<State<ambition_gameplay_core::session::game_mode::GameMode>>,
    mut next_mode: ResMut<NextState<ambition_gameplay_core::session::game_mode::GameMode>>,
    mut pages: ResMut<ActiveMenuPages<MenuPage, MenuPageAction>>,
    mut cursor: ResMut<KaleidoscopeCursor>,
    mut system_nav: ResMut<KaleidoscopeSystemNav>,
    mut map: ResMut<ambition_gameplay_core::menu::map::MapMenuState>,
    mut sfx: MessageWriter<SfxMessage>,
    // Tracks last frame's `menu.start` so we only act on its RISING edge (below).
    mut last_start: Local<bool>,
) {
    use ambition_gameplay_core::session::game_mode::GameMode;

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
    overlay: &mut ambition_gameplay_core::inventory_ui::InventoryUiState,
    mode: &ambition_gameplay_core::session::game_mode::GameMode,
    next_mode: &mut NextState<ambition_gameplay_core::session::game_mode::GameMode>,
    pages: &mut ActiveMenuPages<MenuPage, MenuPageAction>,
    cursor: &mut KaleidoscopeCursor,
    system_nav: &mut KaleidoscopeSystemNav,
    map: &mut ambition_gameplay_core::menu::map::MapMenuState,
) {
    use ambition_gameplay_core::session::game_mode::GameMode;
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
    overlay: &mut ambition_gameplay_core::inventory_ui::InventoryUiState,
    mode: &ambition_gameplay_core::session::game_mode::GameMode,
    next_mode: &mut NextState<ambition_gameplay_core::session::game_mode::GameMode>,
) {
    use ambition_gameplay_core::session::game_mode::GameMode;
    let opened_from_pause = overlay.opened_from_pause;
    overlay.visible = false;
    if !opened_from_pause && matches!(mode, GameMode::Paused) {
        next_mode.set(GameMode::Playing);
    }
}

/// Dev hotkey: `\` cycles the room's ambient gravity through the four cardinal
/// directions — down → left → up → right — so flipped / sideways-gravity behavior
/// (pogo, cling, orientation, slug-crawl) is testable without an authored switch.
/// Mutates [`BaseGravity`]; a gravity zone the player overlaps still wins locally,
/// and a room reset restores the authored default. The inventory-frontend toggle
/// this key used to own now lives only on the dev menu (`D::MenuBackend`).
fn cycle_dev_gravity(
    keys: Res<ButtonInput<KeyCode>>,
    mut base: ResMut<ambition_gameplay_core::physics::BaseGravity>,
) {
    if !keys.just_pressed(KeyCode::Backslash) {
        return;
    }
    // Same step the developer menu's Gravity row uses (shared `BaseGravity::cycle`).
    base.cycle();
    info!(target: "ambition::gravity", "dev gravity cycle: dir = {:?}", base.dir);
}

/// Pause-gate the cube: its order-8 `Camera3d` clears the whole screen every frame,
/// so it must be active only while the inventory is open (and the Cube backend is
/// selected). Off otherwise → the lower-order game cameras render normally.
fn gate_kaleidoscope_menu(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<ambition_gameplay_core::inventory_ui::InventoryUiState>>,
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
    cache: Res<CachedSystemMenu>,
    // Every control across every face, plus whether it sits on the ACTIVE face. The
    // cube spawns all faces at once and a focus key (an edge `>`/`<` button, a row
    // index) collides across faces, so only the active face may carry the cursor
    // highlight. Crucially we still iterate the OTHER faces to RESET them: a control
    // built `selected` (an equipped item / active station) spawns with `focused`
    // pre-set, so skipping inactive faces left those stuck-lit when rotated away.
    mut controls: Query<(
        &AmbitionMenuControl<MenuPageAction>,
        Has<KaleidoscopeActiveFaceControl>,
        &mut MenuVisualState,
    )>,
) {
    let Some(active_page) = pages.active else {
        return;
    };
    // The System row model is built once per frame by `cache_system_menu`; reuse it
    // (empty default off the System face, where no System action is ever matched).
    let fallback = SystemMenuModel::default();
    let model = cache.model.as_ref().unwrap_or(&fallback);
    for (control, on_active_face, mut vis) in &mut controls {
        let Some(action) = control.action else {
            continue;
        };
        // Only the active face highlights; inactive faces always resolve to `false`
        // (and so get reset), never matched against the cursor.
        let focused = on_active_face
            && focus_for_action(action, active_page, model, system_nav.open_entry) == cursor.focus;
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
    cache: Res<CachedSystemMenu>,
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
        MenuPage::System => match cache.model.as_ref() {
            Some(model) => {
                let focused = match cursor.focus {
                    MenuFocus::System(idx) => idx.min(cache.rows.len().saturating_sub(1)),
                    _ => 0,
                };
                system_detail_slot_text(model, &cache.rows, focused)
            }
            None => Vec::new(),
        },
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

mod cache;
mod scroll;
pub(crate) use cache::*;
pub(crate) use scroll::*;

#[cfg(test)]
mod lunex_kaleidoscope_app_tests;
