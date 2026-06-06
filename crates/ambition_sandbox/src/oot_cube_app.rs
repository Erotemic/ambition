//! Game-side hookup for the 3D-cube OoT pause menu (#31): adds the lib's reusable
//! cube renderer ([`ambition_inventory_ui::cube::CubeMenuPlugin`]) and feeds it our
//! live 24-item inventory (via [`crate::oot_cube`]). Runtime-toggleable vs the
//! existing Bevy-UI grid through [`InventoryUiBackend`].
//!
//! The cube is pause-gated ([`gate_cube_menu`]): its order-8 `Camera3d` + ring are
//! only active while the inventory is open, so it never clears the screen to black
//! during play. Routing nav/selection input to it is the next step — see
//! `dev/journals/oot-cube-integration-plan.md`.

use ambition_inventory_ui::cube::{CubeMenuConfig, CubeMenuPlugin};
use ambition_inventory_ui::{ActiveMenuPages, AmbitionInventoryUiPlugin, AmbitionMenuControl};
use bevy::prelude::*;

use crate::audio::SfxMessage;
use crate::engine_core::Vec2;
use crate::input::MenuControlFrame;
use crate::items::{Item, OwnedItems, ITEM_GRID_COLS, ITEM_GRID_ROWS};
use crate::oot_cube::{
    build_inventory_pages, system_rows, CubeAction, CubeFocus, CubePage, SystemRow,
};
use crate::oot_menu::effects::MenuAction;
use crate::oot_menu::input::{dispatch_item_confirm, MenuEffectManaQuery, MenuEffectPlayers};
use crate::persistence::settings::{
    apply_settings_option, settings_menu_model, DevSnapshot, DevToggleId, RadioSnapshot,
    SettingsOptionId, SettingsOptionKind, SystemMenuAction, SystemMenuEntryId, SystemMenuModel,
    SystemOptionId, UserSettings,
};
use crate::player::PlayerHealRequested;

/// Play a one-shot UI sound for the cube menu. Mirrors the proven pause-menu emit
/// (`pause_menu::input::pause_menu_toggle`): `Play { id, pos }` with `pos = ZERO`.
/// `Play` is non-spatialized (see `audio::runtime::audio_play_sfx_messages` — it
/// looks the id up in the bank and plays it full-volume; the `pos` is unused for
/// `Play`), so `Vec2::ZERO` keeps menu sounds audible at full volume. If the id
/// isn't packed into the runtime bank yet the play just no-ops (safe).
#[inline]
fn play_ui(sfx: &mut MessageWriter<SfxMessage>, id: ambition_sfx::SfxId) {
    sfx.write(SfxMessage::Play {
        id,
        pos: Vec2::ZERO,
    });
}

/// Which inventory frontend renders. Runtime toggle (both compiled in); defaults to
/// the 3D `Cube` (#31), with `\` flipping to the proven Bevy-UI `Grid` (see
/// [`toggle_inventory_backend`]).
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum InventoryUiBackend {
    Grid,
    #[default]
    Cube,
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
struct CubeScrim;

/// Wire the 3D-cube menu into the app: the lib plugins + our page-feed system.
pub fn install_cube_menu(app: &mut App) {
    // The game uses Bevy picking on the cube controls AND draws its own real L/R
    // edge buttons (see `oot_cube::add_edge_buttons`), so it inserts its own
    // `CubeMenuConfig` (lib overlay defaults, but `draw_nav_arrows = false` so the
    // decorative arrows don't double-draw and `pickable_controls = true` so
    // `Pointer<*>` events fire) BEFORE the plugin (which only inserts a default
    // if the host hasn't).
    if !app.world().contains_resource::<CubeMenuConfig>() {
        app.insert_resource(CubeMenuConfig {
            draw_nav_arrows: false,
            pickable_controls: true,
            ..Default::default()
        });
    }
    app.init_resource::<InventoryUiBackend>()
        .init_resource::<ActiveMenuPages<CubePage, CubeAction>>()
        .init_resource::<CubeCursor>()
        .init_resource::<CubeSystemNav>()
        .add_plugins(AmbitionInventoryUiPlugin)
        .add_plugins(CubeMenuPlugin::<CubePage, CubeAction>::default())
        .add_systems(Startup, spawn_cube_scrim)
        .add_systems(
            Update,
            (
                // Fix 3: when Cube is the backend, the game's menu-open inputs
                // (pause/Esc, inventory, map) open the cube on the matching page
                // instead of the old Bevy-UI menus. Runs before nav so the page is
                // set the same frame the cube opens.
                cube_menu_open_routing,
                // Nav first (mutates the cursor), then republish (reads the cursor +
                // inventory) so the highlight + detail panel reflect this frame's move.
                cube_focus_nav,
                republish_cube_pages,
                gate_cube_menu,
                toggle_inventory_backend,
                retarget_cube_scrim,
                fade_cube_scrim,
            )
                .chain(),
        )
        .add_observer(cube_pointer_move)
        .add_observer(cube_pointer_click);
}

/// Which input source currently owns the cube cursor. Mirrors the grid's
/// [`crate::ui_nav::MenuFocusOwner`]: keyboard/gamepad nav claims focus and keeps
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
/// (`MockDemo::selected`). [`cube_focus_nav`] moves it with `move_spatial`-style
/// rules; [`republish_cube_pages`] republishes the page model whenever its
/// SEMANTIC focus changes so the highlight + detail panel follow it.
#[derive(Resource, Default)]
struct CubeCursor {
    focus: CubeFocus,
    /// Which input source last moved the cursor (keyboard nav vs pointer hover).
    owner: FocusSource,
    /// The last focus the POINTER moved over. A parked mouse should not count as a
    /// selection; only actual pointer motion can change the cursor here.
    last_pointer_focus: Option<CubeFocus>,
}

impl CubeCursor {
    /// Keyboard/gamepad nav took the cursor to `focus` (claims ownership).
    fn mark_keyboard(&mut self, focus: CubeFocus) {
        self.focus = focus;
        self.owner = FocusSource::Keyboard;
    }
}

/// Drill-down state for the System face. `None` = the top-level category list is
/// shown (Video / Audio / Controls / Gameplay + Close Menu); `Some(category)` = the
/// open category's option rows + a Back row are shown. Mirrors the Bevy-UI pause
/// menu's settings page stack. `republish_cube_pages` feeds this into
/// `build_system_page`, and changing it republishes (the System cursor resets to
/// row 0). B0002-safe: only `cube_focus_nav` / `cube_pointer_click` mutate it (both
/// `ResMut`); `republish_cube_pages` reads it as `Res`.
#[derive(Resource, Default)]
struct CubeSystemNav {
    open_entry: Option<SystemMenuEntryId>,
}

/// All the live resources the broadened SYSTEM screens need to READ a snapshot
/// and APPLY a selection, bundled into one [`SystemParam`] so the cube nav system
/// / pointer observer stay within Bevy's 16-param ceiling. The radio resources are
/// `audio`-gated; `DeveloperTools` + `SandboxResetRequested` are always present
/// (inserted at startup), so accessing them never panics. Held mutably here; the
/// two consumers (`cube_focus_nav`, `cube_pointer_click`) are separate systems so
/// there is no B0002 conflict, and `republish_cube_pages` reads its own `Res`
/// copies (`SystemMenuSnapshotParams`) in a third system.
#[derive(bevy::ecs::system::SystemParam)]
struct SystemMenuParams<'w> {
    dev_tools: ResMut<'w, crate::dev::dev_tools::DeveloperTools>,
    reset: ResMut<'w, crate::runtime::reset::SandboxResetRequested>,
    // The radio resources are `Option`-wrapped so the System nav stays B0002-safe
    // and never panics when audio is off / a fixture omits them: a missing radio
    // resource simply disables station audition (the rows still render). Gated on
    // `audio` so non-audio builds carry none of the types.
    #[cfg(feature = "audio")]
    library: Option<ResMut<'w, crate::audio::AudioLibrary>>,
    #[cfg(feature = "audio")]
    asset_server: Option<Res<'w, AssetServer>>,
    #[cfg(feature = "audio")]
    music_state: Option<ResMut<'w, crate::audio::MusicPlaybackState>>,
    #[cfg(feature = "audio")]
    radio: Option<ResMut<'w, crate::audio::RadioStationState>>,
    #[cfg(feature = "audio")]
    music_channel:
        Option<Res<'w, bevy_kira_audio::prelude::AudioChannel<crate::audio::MusicChannel>>>,
}

impl SystemMenuParams<'_> {
    /// Apply a non-settings System screen option against its live resource.
    /// Radio auditions a station (keeps the menu open); Locale is a no-op stub
    /// (only English exists); Dev toggles/cycles mutate `DeveloperTools`.
    /// Returns the SFX id to play for feedback.
    fn apply_option(&mut self, opt: SystemOptionId) -> ambition_sfx::SfxId {
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
                        crate::audio::set_radio_track(
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
                apply_dev_toggle(&mut self.dev_tools, id, 0);
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
                apply_dev_toggle(&mut self.dev_tools, id, dir);
                Some(ambition_sfx::ids::UI_SLIDER_TICK)
            }
            _ => None,
        }
    }

    fn request_reset(&mut self) {
        self.reset.request();
    }

    /// Build the live radio snapshot for the SYSTEM IR (empty under no `audio` /
    /// when the radio resources are absent).
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
        dev_snapshot(&self.dev_tools)
    }

    /// Build the live SYSTEM model from current settings + held resources.
    fn model(&self, settings: &UserSettings) -> SystemMenuModel {
        SystemMenuModel::build(settings, &self.radio_snapshot(), &self.dev_snapshot())
    }
}

/// Resources `republish_cube_pages` reads (immutably) to snapshot the radio + dev
/// state into the SYSTEM IR. Separate `Res` bundle so it never conflicts with the
/// mutable `SystemMenuParams` (different systems).
#[derive(bevy::ecs::system::SystemParam)]
struct SystemMenuSnapshotParams<'w> {
    dev_tools: Res<'w, crate::dev::dev_tools::DeveloperTools>,
    #[cfg(feature = "audio")]
    library: Option<Res<'w, crate::audio::AudioLibrary>>,
    #[cfg(feature = "audio")]
    music_state: Option<Res<'w, crate::audio::MusicPlaybackState>>,
    #[cfg(feature = "audio")]
    radio: Option<Res<'w, crate::audio::RadioStationState>>,
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
        dev_snapshot(&self.dev_tools)
    }

    /// True when any radio/dev resource changed this frame (so the cube republishes
    /// the System face to reflect an auditioned station / toggled dev flag).
    fn is_changed(&self) -> bool {
        let mut changed = self.dev_tools.is_changed();
        #[cfg(feature = "audio")]
        {
            changed = changed
                || self
                    .library
                    .as_ref()
                    .map(|r| r.is_changed())
                    .unwrap_or(false)
                || self
                    .music_state
                    .as_ref()
                    .map(|r| r.is_changed())
                    .unwrap_or(false)
                || self.radio.as_ref().map(|r| r.is_changed()).unwrap_or(false);
        }
        changed
    }
}

/// Build a [`RadioSnapshot`] from the live audio library + playback state. The
/// single place that maps the audio runtime onto the SYSTEM IR's station list.
#[cfg(feature = "audio")]
fn radio_snapshot_from(
    library: &crate::audio::AudioLibrary,
    music_state: &crate::audio::MusicPlaybackState,
    radio: Option<&crate::audio::RadioStationState>,
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

/// Read every developer toggle/cycle into a [`DevSnapshot`] for the SYSTEM IR. The
/// single place mapping `DeveloperTools` fields onto [`DevToggleId`]s for display.
fn dev_snapshot(dev: &crate::dev::dev_tools::DeveloperTools) -> DevSnapshot {
    use DevToggleId as D;
    let mut values = Vec::with_capacity(DevToggleId::ALL.len());
    values.push(DevSnapshot::toggle(D::Inspector, dev.inspector_visible));
    values.push(DevSnapshot::toggle(
        D::WorldInspector,
        dev.world_inspector_visible,
    ));
    values.push(DevSnapshot::toggle(D::Gizmos, dev.gizmos_enabled));
    values.push(DevSnapshot::toggle(D::ShowHud, dev.show_hud));
    values.push(DevSnapshot::toggle(D::ShowHitboxes, dev.show_player_hitbox));
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
    DevSnapshot { values }
}

/// Apply a single developer toggle/cycle to `DeveloperTools`. `dir` selects the
/// direction for cycles (`<0` prev, otherwise next); toggles flip regardless. This
/// is the single place that mutates `DeveloperTools` from the cube, so the dev
/// menu and the inspector stay in lock-step on field semantics.
fn apply_dev_toggle(dev: &mut crate::dev::dev_tools::DeveloperTools, id: DevToggleId, dir: i32) {
    use DevToggleId as D;
    match id {
        D::Inspector => dev.inspector_visible = !dev.inspector_visible,
        D::WorldInspector => dev.world_inspector_visible = !dev.world_inspector_visible,
        D::Gizmos => dev.gizmos_enabled = !dev.gizmos_enabled,
        D::ShowHud => dev.show_hud = !dev.show_hud,
        D::ShowHitboxes => dev.show_player_hitbox = !dev.show_player_hitbox,
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
    }
}

/// Spawn the readability dim-scrim node (full-screen, starts fully transparent).
///
/// The scrim DIMS THE WORLD, so it must render BEHIND the order-8 cube. Since the
/// default UI camera is now the order-9 [`FrontHudCamera`] (which draws in front of
/// the cube), the scrim is explicitly retargeted onto the order-0 main camera via
/// [`retarget_cube_scrim`] (the `MainCameraEntity` resource isn't guaranteed to
/// exist yet at this Startup point, so the target is attached from an Update guard).
/// [`fade_cube_scrim`] drives its alpha.
fn spawn_cube_scrim(mut commands: Commands) {
    commands.spawn((
        CubeScrim,
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
fn retarget_cube_scrim(
    mut commands: Commands,
    main_camera: Option<Res<crate::runtime::camera_layers::MainCameraEntity>>,
    scrim: Query<Entity, (With<CubeScrim>, Without<UiTargetCamera>)>,
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
fn fade_cube_scrim(
    open_state: Res<ambition_inventory_ui::cube::CubeOpenState>,
    mut scrim: Query<&mut BackgroundColor, With<CubeScrim>>,
) {
    let alpha = open_state.amount.clamp(0.0, 1.0) * SCRIM_PEAK_ALPHA;
    for mut bg in &mut scrim {
        bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
    }
}

/// Directional focus navigation for the cube (keyboard / gamepad), porting the
/// demo's `MockDemo::move_spatial` (`crates/ambition_mock_demo/src/app/state.rs`).
/// The cursor lives on the [`CubeCursor`] resource as a [`CubeFocus`], and the
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
/// `select` on an item dispatches its `CubeAction`; `select` on an arrow turns the
/// page; `back` closes the menu. The republish runs after this in the chain.
#[allow(clippy::too_many_arguments)]
fn cube_focus_nav(
    backend: Res<InventoryUiBackend>,
    menu: Res<MenuControlFrame>,
    mut cursor: ResMut<CubeCursor>,
    mut system_nav: ResMut<CubeSystemNav>,
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
    // Single mutable access to the overlay state — also read `.visible` from it (a
    // separate `Res<InventoryUiState>` would be a B0002 conflict with this `ResMut`).
    mut overlay: ResMut<crate::inventory::InventoryUiState>,
    mut owned: ResMut<OwnedItems>,
    mut settings: ResMut<UserSettings>,
    mut commands: Commands,
    mut players: MenuEffectPlayers,
    mut mana_q: MenuEffectManaQuery,
    mut heals: MessageWriter<PlayerHealRequested>,
    mut sfx: MessageWriter<SfxMessage>,
    mut system: SystemMenuParams,
) {
    if *backend != InventoryUiBackend::Cube || !overlay.visible {
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
    if active_page == CubePage::System {
        system_focus_nav(
            &menu,
            dx,
            dy,
            &mut cursor,
            &mut system_nav,
            &mut pages,
            &mut overlay,
            &mut settings,
            active_page,
            &mut owned,
            &mut commands,
            &mut players,
            &mut mana_q,
            &mut heals,
            &mut sfx,
            &mut system,
        );
        return;
    }

    // Other non-items faces (Map / Quest placeholders) respond to horizontal page
    // turns; arrows rotate, landing the cursor on the new page's back-edge button
    // (Fix 1). The L/R bumpers (Fix 2) are already handled above for every face.
    if active_page != CubePage::Items {
        // Placeholder faces (Map / Quest) have only the two edge buttons and no centre
        // content. LEFT/RIGHT move BETWEEN the edges when stepping INWARD; only stepping
        // OUTWARD past an edge rotates the page — the same arrow/edge rule as the items
        // face, just with nothing in the middle. (Was: any L/R rotated immediately, so
        // right-from-the-left-edge jumped to the next page instead of the right edge.)
        if dx != 0 {
            match cursor.focus {
                CubeFocus::EdgeLeft if dx > 0 => cursor.mark_keyboard(CubeFocus::EdgeRight),
                CubeFocus::EdgeLeft => turn_page_seeded(
                    &mut pages,
                    &mut cursor,
                    active_page.on_viewer_left(),
                    &mut sfx,
                ),
                CubeFocus::EdgeRight if dx < 0 => cursor.mark_keyboard(CubeFocus::EdgeLeft),
                CubeFocus::EdgeRight => turn_page_seeded(
                    &mut pages,
                    &mut cursor,
                    active_page.on_viewer_right(),
                    &mut sfx,
                ),
                // Cursor not yet on an edge — seed onto the edge for the pressed direction.
                _ => cursor.mark_keyboard(if dx < 0 {
                    CubeFocus::EdgeLeft
                } else {
                    CubeFocus::EdgeRight
                }),
            }
        }
        if menu.select {
            // The only selectable controls on a placeholder are the edge buttons.
            match cursor.focus {
                CubeFocus::EdgeLeft => turn_page_seeded(
                    &mut pages,
                    &mut cursor,
                    active_page.on_viewer_left(),
                    &mut sfx,
                ),
                CubeFocus::EdgeRight => turn_page_seeded(
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
                cursor.mark_keyboard(CubeFocus::EdgeRight);
            }
            SpatialMove::TurnRight => {
                turn_page(&mut pages, active_page.on_viewer_right(), &mut sfx);
                cursor.mark_keyboard(CubeFocus::EdgeLeft);
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
            CubeFocus::EdgeLeft => Some(CubeAction::ChangePage(active_page.on_viewer_left())),
            CubeFocus::EdgeRight => Some(CubeAction::ChangePage(active_page.on_viewer_right())),
            CubeFocus::Item(idx) => owned_item_action(&owned, idx),
            // System focus is handled by the System branch above; never reached here.
            CubeFocus::System(_) => None,
        };
        if let Some(action) = action {
            let mut close_menu = false;
            dispatch_cube_action(
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
                overlay.visible = false;
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
    focus_before: CubeFocus,
    focus_after: CubeFocus,
    page_before: Option<CubePage>,
    page_after: Option<CubePage>,
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
fn system_focus_nav(
    menu: &MenuControlFrame,
    dx: i32,
    dy: i32,
    cursor: &mut CubeCursor,
    system_nav: &mut CubeSystemNav,
    pages: &mut ActiveMenuPages<CubePage, CubeAction>,
    overlay: &mut crate::inventory::InventoryUiState,
    settings: &mut UserSettings,
    active_page: CubePage,
    owned: &mut OwnedItems,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers,
    mana_q: &mut MenuEffectManaQuery,
    heals: &mut MessageWriter<PlayerHealRequested>,
    sfx: &mut MessageWriter<SfxMessage>,
    system: &mut SystemMenuParams,
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
        CubeFocus::System(idx) => (idx as i32).min(count - 1),
        _ => 0,
    };

    if dy != 0 {
        row = (row + dy).clamp(0, count - 1);
        cursor.mark_keyboard(CubeFocus::System(row as usize));
    }

    let current = rows[row.max(0).min(count - 1) as usize];

    if dx != 0 {
        match cursor.focus {
            CubeFocus::EdgeLeft => {
                if dx > 0 {
                    // Move inward from the page-turn button into the row list.
                    cursor.mark_keyboard(CubeFocus::System(0));
                } else {
                    // Moving further outward from the edge still rotates the cube.
                    turn_page(pages, active_page.on_viewer_left(), sfx);
                    cursor.mark_keyboard(CubeFocus::System(0));
                }
            }
            CubeFocus::EdgeRight => {
                if dx < 0 {
                    cursor.mark_keyboard(CubeFocus::System(0));
                } else {
                    turn_page(pages, active_page.on_viewer_right(), sfx);
                    cursor.mark_keyboard(CubeFocus::System(0));
                }
            }
            CubeFocus::System(_) | CubeFocus::Item(_) => {
                // LEFT/RIGHT step value rows in place (settings cycles/sliders, dev
                // cycles); otherwise use the horizontal affordance to move onto the
                // edge buttons.
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
                if !stepped {
                    if dx < 0 {
                        cursor.mark_keyboard(CubeFocus::EdgeLeft);
                    } else {
                        cursor.mark_keyboard(CubeFocus::EdgeRight);
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
            dispatch_cube_action(
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
                overlay.visible = false;
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

/// The `CubeAction` a System row dispatches on select.
fn system_row_action_for(model: &SystemMenuModel, row: SystemRow) -> Option<CubeAction> {
    match row {
        SystemRow::Entry(id) => match model.entry(id).map(|e| &e.target) {
            Some(crate::persistence::settings::SystemMenuTarget::Action(action)) => {
                Some(CubeAction::SystemAction(*action))
            }
            _ => Some(CubeAction::OpenSystemEntry(id)),
        },
        SystemRow::Setting(o) => Some(CubeAction::System(o)),
        SystemRow::Option(o) => Some(CubeAction::SystemOption(o)),
        SystemRow::Back => Some(CubeAction::CloseSystemEntry),
    }
}

/// Drill OUT of an open System entry back to the entry list, resetting the cursor
/// to the first row so the highlight lands sensibly.
fn close_system_entry(system_nav: &mut CubeSystemNav, cursor: &mut CubeCursor) {
    system_nav.open_entry = None;
    cursor.mark_keyboard(CubeFocus::System(0));
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
    Focus(CubeFocus),
    /// The cursor was on the left arrow and pressed further left → rotate left.
    TurnLeft,
    /// The cursor was on the right arrow and pressed further right → rotate right.
    TurnRight,
}

/// Port of the demo's `MockDemo::move_spatial` for the items grid + flanking
/// arrows. Pure (no ECS) so it's unit-testable and easy to reason about. See
/// [`cube_focus_nav`] for the rule list.
fn move_spatial(focus: CubeFocus, dx: i32, dy: i32, _page: CubePage) -> SpatialMove {
    let cols = ITEM_GRID_COLS as i32;
    let rows = ITEM_GRID_ROWS as i32;

    // Rule 3: on an arrow, moving further OUTWARD rotates the page; UP/DOWN never
    // reach/leave an arrow (rule 2); moving INWARD enters the adjacent column.
    match focus {
        CubeFocus::EdgeLeft => {
            if dx < 0 {
                return SpatialMove::TurnLeft;
            }
            if dx > 0 {
                // Rule 1: enter the LEFTMOST item column (col 0), keep the row band.
                return SpatialMove::Focus(CubeFocus::Item(0));
            }
            // Up/Down on an arrow: stay put (rule 2).
            return SpatialMove::Focus(focus);
        }
        CubeFocus::EdgeRight => {
            if dx > 0 {
                return SpatialMove::TurnRight;
            }
            if dx < 0 {
                // Rule 1: enter the RIGHTMOST item column.
                return SpatialMove::Focus(CubeFocus::Item((cols - 1) as usize));
            }
            return SpatialMove::Focus(focus);
        }
        CubeFocus::Item(idx) => {
            let idx = idx as i32;
            let row = idx / cols;
            let col = idx % cols;
            // Rule 4: stepping off the left/right column lands on the arrow.
            if dx < 0 && col == 0 {
                return SpatialMove::Focus(CubeFocus::EdgeLeft);
            }
            if dx > 0 && col == cols - 1 {
                return SpatialMove::Focus(CubeFocus::EdgeRight);
            }
            // Rule 2: UP/DOWN stays within the columns (never reaches an arrow).
            let next_col = (col + dx).clamp(0, cols - 1);
            let next_row = (row + dy).clamp(0, rows - 1);
            SpatialMove::Focus(CubeFocus::Item((next_row * cols + next_col) as usize))
        }
        // `move_spatial` is only invoked on the Items face; a System focus here
        // would be a logic error — re-enter the grid at slot 0 to stay safe.
        CubeFocus::System(_) => SpatialMove::Focus(CubeFocus::Item(0)),
    }
}

/// The `CubeAction` for an owned item slot, or `None` if the slot is empty/unowned
/// (so confirming an empty cell is a no-op, matching the grid backend).
fn owned_item_action(owned: &OwnedItems, idx: usize) -> Option<CubeAction> {
    let item = Item::from_index(idx)?;
    if !owned.has(item) {
        return None;
    }
    Some(if item.held_item_id().is_some() {
        CubeAction::Equip(item)
    } else {
        CubeAction::Use(item)
    })
}

/// The edge-button focus on `to` that turns BACK toward `from` (Fix 1). After a page
/// turn the cursor lands here, so the arriving control is highlighted and an immediate
/// rotate/select returns to the page we came from. On `to`, the LEFT edge button
/// targets `to.on_viewer_left()` and the RIGHT targets `to.on_viewer_right()`; we pick
/// whichever points back at `from`. When `from` is unknown (first open) we default to
/// the left edge button so there is always a highlighted control.
fn back_edge_focus(from: Option<CubePage>, to: CubePage) -> CubeFocus {
    match from {
        Some(from) if to.on_viewer_right() == from => CubeFocus::EdgeRight,
        Some(from) if to.on_viewer_left() == from => CubeFocus::EdgeLeft,
        _ => CubeFocus::EdgeLeft,
    }
}

/// Set the active page (the lib rotates that face to the camera), landing the cursor
/// on the new page's back-edge button (Fix 1) via [`back_edge_focus`].
fn turn_page_seeded(
    pages: &mut ActiveMenuPages<CubePage, CubeAction>,
    cursor: &mut CubeCursor,
    page: CubePage,
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
    pages: &mut ActiveMenuPages<CubePage, CubeAction>,
    page: CubePage,
    sfx: &mut MessageWriter<SfxMessage>,
) {
    if pages.active != Some(page) {
        play_ui(sfx, rotate_sfx(pages.active, page));
        pages.active = Some(page);
        info!("cube page \u{2192} {:?}", page);
    }
}

/// Dispatch a [`CubeAction`]. Item Equip/Use reuse the grid's shared
/// [`dispatch_item_confirm`] (no portal/equip/heal duplication); page-change sets
/// the active page so the lib rotates that face to the camera.
#[allow(clippy::too_many_arguments)]
fn dispatch_cube_action(
    action: CubeAction,
    pages: &mut ActiveMenuPages<CubePage, CubeAction>,
    system_nav: &mut CubeSystemNav,
    cursor: &mut CubeCursor,
    owned: &mut OwnedItems,
    settings: &mut UserSettings,
    close_menu: &mut bool,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers,
    mana_q: &mut MenuEffectManaQuery,
    heals: &mut MessageWriter<PlayerHealRequested>,
    sfx: &mut MessageWriter<SfxMessage>,
    system: &mut SystemMenuParams,
) {
    match action {
        CubeAction::Equip(item) | CubeAction::Use(item) => {
            let decided = dispatch_item_confirm(item, owned, commands, players, mana_q, heals);
            // Pick the confirm sound from the RESOLVED action so equip/unequip/use
            // are distinct, and a no-op (not owned / nothing to do) gives error feedback.
            let id = match decided {
                MenuAction::Equip(_) => ambition_sfx::ids::UI_MENU_EQUIP,
                MenuAction::Unequip(_) => ambition_sfx::ids::UI_MENU_UNEQUIP,
                MenuAction::UseConsumable(_) => ambition_sfx::ids::UI_MENU_ACCEPT,
                MenuAction::Inspect(_) | MenuAction::NotOwned(_) => {
                    ambition_sfx::ids::UI_MENU_ERROR
                }
            };
            play_ui(sfx, id);
            info!("cube action: {:?} \u{2192} {:?}", item, decided);
        }
        CubeAction::ChangePage(page) => {
            let from = pages.active;
            play_ui(sfx, rotate_sfx(from, page));
            pages.active = Some(page);
            // Fix 1: land the cursor on the new page's "back" edge button — the one
            // that turns BACK toward the page we came from — so an immediate select /
            // rotate goes home and the arriving control is highlighted.
            cursor.mark_keyboard(back_edge_focus(from, page));
            info!("cube page \u{2192} {:?}", page);
        }
        CubeAction::System(option) => {
            apply_system_option(option, settings, close_menu, sfx);
        }
        CubeAction::SystemOption(opt) => {
            // Radio / Language / Developer screen options apply against their live
            // resource (radio auditions + keeps the menu open; dev toggles mutate
            // DeveloperTools). The menu never closes from these.
            let id = system.apply_option(opt);
            play_ui(sfx, id);
            info!("cube system option: {:?}", opt);
        }
        CubeAction::SystemAction(SystemMenuAction::ResetSandbox) => {
            // Immediate, no-confirm: queue the reset and fold the menu shut.
            system.request_reset();
            *close_menu = true;
            play_ui(sfx, ambition_sfx::ids::UI_MENU_ACCEPT);
            info!("cube system action: reset sandbox");
        }
        CubeAction::OpenSystemEntry(entry) => {
            // Drill INTO an entry: show its screen rows, land the cursor on the
            // first row. The republish picks up the new drill state + cursor.
            play_ui(sfx, ambition_sfx::ids::UI_TAB_CHANGE);
            system_nav.open_entry = Some(entry);
            cursor.mark_keyboard(CubeFocus::System(0));
            info!("cube system entry \u{2192} {:?}", entry);
        }
        CubeAction::CloseSystemEntry => {
            play_ui(sfx, ambition_sfx::ids::UI_MENU_BACK);
            close_system_entry(system_nav, cursor);
            info!("cube system entry \u{2192} (list)");
        }
    }
}

/// The directional page-turn sound for a rotation `from` → `to`: rotating to the
/// page that sits on the viewer-LEFT of `from` plays the left rotate, otherwise the
/// right rotate. When `from` is unknown (first publish) defaults to the right rotate.
fn rotate_sfx(from: Option<CubePage>, to: CubePage) -> ambition_sfx::SfxId {
    match from {
        Some(from) if from.on_viewer_left() == to => ambition_sfx::ids::UI_MENU_ROTATE_LEFT,
        _ => ambition_sfx::ids::UI_MENU_ROTATE_RIGHT,
    }
}

/// Apply a System-face option (SELECT/confirm) by mutating `UserSettings` through
/// the shared settings IR ([`apply_settings_option`]): toggles flip, cycles +
/// sliders advance one step (confirm = next), and `Close` folds the menu. The SFX
/// is chosen from the option's IR `kind` (toggle on/off, slider tick, close).
/// Persistence is NOT re-implemented here: the existing `save_settings_on_change`
/// system writes `settings.ron` whenever `UserSettings` changes, so mutating the
/// resource is the whole job.
fn apply_system_option(
    option: SettingsOptionId,
    settings: &mut UserSettings,
    close_menu: &mut bool,
    sfx: &mut MessageWriter<SfxMessage>,
) {
    // Resolve the option's kind BEFORE mutating, so a toggle reports its NEW state
    // and a slider/cycle gets a tick. `Close` is the only kind that folds the menu.
    let kind = settings_menu_model(settings)
        .categories
        .iter()
        .flat_map(|c| c.options.iter())
        .find(|o| o.id == option)
        .map(|o| o.kind)
        .unwrap_or(SettingsOptionKind::Action);

    // Confirm advances like Next (dir 0 == next/toggle/up in the IR).
    let closed = apply_settings_option(option, 0, settings);
    if closed {
        *close_menu = true;
        play_ui(sfx, ambition_sfx::ids::UI_MENU_CLOSE);
        info!("cube system option: {:?}", option);
        return;
    }

    match kind {
        SettingsOptionKind::Toggle(_) => {
            // Read the now-current state from the rebuilt model for the on/off SFX.
            let on = settings_menu_model(settings)
                .categories
                .iter()
                .flat_map(|c| c.options.iter())
                .find(|o| o.id == option)
                .map(|o| matches!(o.kind, SettingsOptionKind::Toggle(true)))
                .unwrap_or(false);
            play_ui(
                sfx,
                if on {
                    ambition_sfx::ids::UI_TOGGLE_ON
                } else {
                    ambition_sfx::ids::UI_TOGGLE_OFF
                },
            );
        }
        SettingsOptionKind::Cycle { .. } | SettingsOptionKind::Slider { .. } => {
            play_ui(sfx, ambition_sfx::ids::UI_SLIDER_TICK);
        }
        SettingsOptionKind::Action => {}
    }
    info!("cube system option: {:?}", option);
}

/// Map a control's `CubeAction` back to the cursor focus it represents, so pointer
/// hover/click and the keyboard cursor share one model. `Equip`/`Use` carry the
/// item (→ its slot); `ChangePage` is an edge arrow — left vs right is decided by
/// whether its target is the active page's viewer-left neighbour.
fn focus_for_action(
    action: CubeAction,
    active_page: CubePage,
    model: &SystemMenuModel,
    open_entry: Option<SystemMenuEntryId>,
) -> CubeFocus {
    // System rows are positional: the focus index is the action's row in the
    // currently-displayed System row list (the entry list, or an open entry's
    // screen rows + Back), so hover/click and the keyboard cursor agree on the row.
    let system_row = |want: SystemRow| {
        let idx = system_rows(model, open_entry)
            .iter()
            .position(|r| *r == want)
            .unwrap_or(0);
        CubeFocus::System(idx)
    };
    match action {
        CubeAction::Equip(item) | CubeAction::Use(item) => CubeFocus::Item(item.index()),
        CubeAction::ChangePage(target) => {
            if target == active_page.on_viewer_left() {
                CubeFocus::EdgeLeft
            } else {
                CubeFocus::EdgeRight
            }
        }
        CubeAction::System(option) => system_row(SystemRow::Setting(option)),
        CubeAction::SystemOption(opt) => system_row(SystemRow::Option(opt)),
        CubeAction::SystemAction(_) => {
            // An Action entry sits at top level; find its entry row.
            let entry = match action {
                CubeAction::SystemAction(SystemMenuAction::ResetSandbox) => {
                    SystemMenuEntryId::ResetSandbox
                }
                _ => return CubeFocus::System(0),
            };
            system_row(SystemRow::Entry(entry))
        }
        CubeAction::OpenSystemEntry(entry) => system_row(SystemRow::Entry(entry)),
        CubeAction::CloseSystemEntry => system_row(SystemRow::Back),
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
fn cube_pointer_move(
    move_: On<Pointer<Move>>,
    controls: Query<&AmbitionMenuControl<CubeAction>>,
    pages: Res<ActiveMenuPages<CubePage, CubeAction>>,
    system_nav: Res<CubeSystemNav>,
    settings: Res<UserSettings>,
    snapshot: SystemMenuSnapshotParams,
    mut cursor: ResMut<CubeCursor>,
    mut sfx: MessageWriter<SfxMessage>,
) {
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

/// Pointer click (mouse/touch) on a cube control: dispatch its `CubeAction`.
#[allow(clippy::too_many_arguments)]
fn cube_pointer_click(
    click: On<Pointer<Click>>,
    backend: Res<InventoryUiBackend>,
    mut ui_state: Option<ResMut<crate::inventory::InventoryUiState>>,
    controls: Query<&AmbitionMenuControl<CubeAction>>,
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
    mut cursor: ResMut<CubeCursor>,
    mut system_nav: ResMut<CubeSystemNav>,
    mut owned: ResMut<OwnedItems>,
    mut settings: ResMut<UserSettings>,
    mut commands: Commands,
    mut players: MenuEffectPlayers,
    mut mana_q: MenuEffectManaQuery,
    mut heals: MessageWriter<PlayerHealRequested>,
    mut sfx: MessageWriter<SfxMessage>,
    mut system: SystemMenuParams,
) {
    let open = ui_state.as_deref().map(|s| s.visible).unwrap_or(false);
    if *backend != InventoryUiBackend::Cube || !open {
        return;
    }
    if let Ok(control) = controls.get(click.entity) {
        if let Some(action) = control.action {
            if let Some(active_page) = pages.active {
                let model = system.model(&settings);
                let next = focus_for_action(action, active_page, &model, system_nav.open_entry);
                cursor.focus = next;
                cursor.owner = FocusSource::Pointer;
                cursor.last_pointer_focus = Some(next);
            }
            let mut close_menu = false;
            // Clicks route through the SAME `dispatch_cube_action` as the keyboard
            // select path, so the action sounds (equip/use/rotate/toggle/...) live in
            // one place and are identical for pointer + keyboard.
            dispatch_cube_action(
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
                    ui_state.visible = false;
                }
            }
        }
    }
}

/// Fix 3: route the game's menu-open inputs to the CUBE when it is the active
/// backend, opening it on the page that matches the requested menu:
///
/// * pause / `Esc` (`menu.start`) → open on [`CubePage::System`] (replacing the old
///   pause/system menu); pressing it again while the cube is open CLOSES the cube.
/// * inventory key (`menu.inventory`) → open on [`CubePage::Items`].
/// * map key (`menu.map`) → open on [`CubePage::Map`].
///
/// Opening pauses the sim (`GameMode::Paused`) and raises `InventoryUiState.visible`,
/// exactly like the inventory open path — which makes the existing pause-menu UI
/// auto-suppress (`Paused && !inventory.visible`). The old `pause_menu_toggle` and
/// `handle_map_menu_hotkeys` are gated to no-op under the Cube backend (see their
/// `cube_backend_active` guards), so nothing double-fires the `GameMode` toggle and
/// the map panel never opens behind the cube.
///
/// `Esc`-to-close is owned HERE (not by `cube_focus_nav`'s `menu.back`) so the close
/// also restores `GameMode::Playing`; the routing runs before `cube_focus_nav`, and
/// consuming the open/close intent keeps the two from fighting over the same frame.
#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
fn cube_menu_open_routing(
    backend: Res<InventoryUiBackend>,
    mut menu: ResMut<MenuControlFrame>,
    mut overlay: ResMut<crate::inventory::InventoryUiState>,
    mode: Res<State<crate::runtime::game_mode::GameMode>>,
    mut next_mode: ResMut<NextState<crate::runtime::game_mode::GameMode>>,
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
    mut cursor: ResMut<CubeCursor>,
    mut system_nav: ResMut<CubeSystemNav>,
    mut map: ResMut<crate::map_menu::MapMenuState>,
    mut sfx: MessageWriter<SfxMessage>,
) {
    use crate::runtime::game_mode::GameMode;
    if *backend != InventoryUiBackend::Cube {
        return;
    }

    // pause / Esc: toggle the cube on the System page.
    if menu.start {
        // Esc binds to BOTH `pause` (→ `menu.start`) AND `MenuBack` (→ `menu.back`),
        // so a single Esc sets both bits. This system OWNS the Esc open/close toggle;
        // consume the duplicate `back` so the later `cube_focus_nav` / `system_focus_nav`
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
                close_cube_menu(&mut overlay, mode.get(), &mut next_mode);
            }
        } else if matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
            play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_OPEN);
            open_cube_menu(
                CubePage::System,
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

    // inventory key: the shared open/close TOGGLE stays in `oot_menu_input` (it raises
    // `visible` + pauses for both backends); here we only point the cube at the Items
    // page + seed the cursor whenever that key fires. Closing is handled there too —
    // when the key closes the overlay this just sets a page that won't be shown.
    if menu.inventory {
        // The overlay's `visible` is flipped by `oot_menu_input`; play open vs close
        // off the state it WILL be in this frame (we observe the pre-flip value here,
        // so a currently-hidden overlay is opening and a visible one is closing).
        play_ui(
            &mut sfx,
            if overlay.visible {
                ambition_sfx::ids::UI_MENU_CLOSE
            } else {
                ambition_sfx::ids::UI_MENU_OPEN
            },
        );
        pages.active = Some(CubePage::Items);
        system_nav.open_entry = None;
        cursor.last_pointer_focus = None;
        cursor.mark_keyboard(CubeFocus::Item(0));
        map.open = false;
        return;
    }

    // map key: open on the Map page (suppressing the standalone map panel).
    if menu.map && matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
        if overlay.visible {
            pages.active = Some(CubePage::Map);
            cursor.mark_keyboard(CubeFocus::EdgeLeft);
        } else {
            play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_OPEN);
            open_cube_menu(
                CubePage::Map,
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

/// Open the cube overlay on `page`, pausing the sim and seeding the cursor. Mirrors
/// the inventory open path (`oot_menu_input`): raise `visible`, switch to
/// `GameMode::Paused` when coming from gameplay, and make sure the standalone map
/// panel stays shut so it can't render behind the cube.
#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
fn open_cube_menu(
    page: CubePage,
    overlay: &mut crate::inventory::InventoryUiState,
    mode: &crate::runtime::game_mode::GameMode,
    next_mode: &mut NextState<crate::runtime::game_mode::GameMode>,
    pages: &mut ActiveMenuPages<CubePage, CubeAction>,
    cursor: &mut CubeCursor,
    system_nav: &mut CubeSystemNav,
    map: &mut crate::map_menu::MapMenuState,
) {
    use crate::runtime::game_mode::GameMode;
    overlay.visible = true;
    overlay.opened_from_pause = matches!(mode, GameMode::Paused);
    pages.active = Some(page);
    // Seed a sensible cursor for the opening page.
    system_nav.open_entry = None;
    cursor.last_pointer_focus = None;
    cursor.mark_keyboard(match page {
        CubePage::Items => CubeFocus::Item(0),
        CubePage::System => CubeFocus::System(0),
        CubePage::Map | CubePage::Quest => CubeFocus::EdgeLeft,
    });
    // Never leave the standalone map panel open underneath the cube.
    map.open = false;
    if matches!(mode, GameMode::Playing) {
        next_mode.set(GameMode::Paused);
    }
}

/// Close the cube overlay (Esc while open), restoring `GameMode::Playing` when the
/// cube was opened directly from gameplay (matching `close_oot_menu`).
#[cfg(feature = "input")]
fn close_cube_menu(
    overlay: &mut crate::inventory::InventoryUiState,
    mode: &crate::runtime::game_mode::GameMode,
    next_mode: &mut NextState<crate::runtime::game_mode::GameMode>,
) {
    use crate::runtime::game_mode::GameMode;
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
        *backend = match *backend {
            InventoryUiBackend::Grid => InventoryUiBackend::Cube,
            InventoryUiBackend::Cube => InventoryUiBackend::Grid,
        };
        info!("inventory backend → {:?}", *backend);
    }
}

/// Pause-gate the cube: its order-8 `Camera3d` clears the whole screen every frame,
/// so it must be active only while the inventory is open (and the Cube backend is
/// selected). Off otherwise → the lower-order game cameras render normally.
fn gate_cube_menu(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<crate::inventory::InventoryUiState>>,
    mut open_state: ResMut<ambition_inventory_ui::cube::CubeOpenState>,
    mut cameras: Query<(
        &mut Camera,
        Has<ambition_inventory_ui::cube::CubePauseCamera>,
    )>,
    mut rings: Query<&mut Visibility, With<ambition_inventory_ui::cube::MenuRing>>,
    mut last_show: Local<Option<bool>>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    let show = *backend == InventoryUiBackend::Cube && open;
    if *last_show != Some(show) {
        info!(
            "cube gate: show={show} backend={:?} menu_open={open}",
            *backend
        );
        *last_show = Some(show);
    }
    // Drive the lib's open/close fold: it eases `amount` toward this target each
    // frame (see `animate_cube_open`). We gate the camera/visibility off the eased
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
    for (mut cam, is_cube) in &mut cameras {
        if is_cube && cam.is_active != shown {
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

/// Republish the cube's faces from our live inventory + the focus cursor (the
/// host-owned data seam — the cube renderer treats `ActiveMenuPages` as read-only).
///
/// Runs after [`cube_focus_nav`] in the chain so this frame's cursor move is
/// reflected in the rebuilt page (highlight + detail panel). To avoid an infinite
/// rebuild loop (writing `pages.pages` marks the resource changed), it republishes
/// only when something it depends on actually changed: the inventory, the focus
/// cursor, the active page, the just-opened edge, or the very first publish.
fn republish_cube_pages(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<crate::inventory::InventoryUiState>>,
    owned: Option<Res<OwnedItems>>,
    // Read-only here. The mutators (`cube_focus_nav`, `cube_pointer_click`) take
    // `ResMut<UserSettings>` in SEPARATE systems, so this `Res` is not a B0002
    // conflict; `UserSettings` is inserted at startup so the `Res` never panics.
    settings: Res<UserSettings>,
    cursor: Res<CubeCursor>,
    // Read-only here; the mutators (`cube_focus_nav`, `cube_pointer_click`) take
    // `ResMut<CubeSystemNav>` in SEPARATE systems/observers, so this `Res` is not a
    // B0002 conflict. Inserted at startup (`init_resource`) so it never panics.
    system_nav: Res<CubeSystemNav>,
    // The radio + developer snapshots feed the broadened SYSTEM screens. Read-only
    // here; the mutators take the `ResMut` `SystemMenuParams` in separate systems,
    // so no B0002. Audio resources are absent under no `audio` (the bundle cfgs out).
    snapshot: SystemMenuSnapshotParams,
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
    mut was_open: Local<bool>,
    mut last: Local<Option<(CubeFocus, Option<CubePage>, Option<SystemMenuEntryId>)>>,
) {
    if *backend != InventoryUiBackend::Cube {
        return;
    }
    let Some(owned) = owned else {
        return;
    };
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    let just_opened = open && !*was_open;
    *was_open = open;

    // The drill-down state is part of the page key, so drilling into/out of a
    // System entry republishes the (now different) System rows.
    let key = (cursor.focus, pages.active, system_nav.open_entry);
    // Republish on: catalog change, settings change (so a toggled setting's label
    // updates immediately), radio/dev change (so an auditioned station or toggled
    // dev flag updates), first publish, menu-open (textures that loaded after the
    // initial build get picked up), cursor move, page change, or a System drill
    // in/out. The open case fixes icons rendering blank until the first rotate.
    let dirty = owned.is_changed()
        || settings.is_changed()
        || snapshot.is_changed()
        || pages.pages.is_empty()
        || just_opened
        || *last != Some(key);
    if !dirty {
        return;
    }
    *last = Some(key);

    let active = pages.active.unwrap_or(CubePage::Items);
    pages.pages = build_inventory_pages(
        &owned,
        owned.equipped(),
        cursor.focus,
        &settings,
        &snapshot.radio_snapshot(),
        &snapshot.dev_snapshot(),
        system_nav.open_entry,
    );
    pages.active = Some(active);
}

#[cfg(test)]
mod oot_cube_app_tests {
    //! Behaviour tests for the cube's interaction seams, driven through the real
    //! systems / observers exactly as the app wires them.
    //!
    //! * Fix 1 — [`back_edge_focus`] lands the cursor on the "back" edge button.
    //! * Fix 4 — `cube_pointer_click` dispatches System-page clicks (drill in,
    //!   apply an option, Close) at parity with keyboard select.
    use super::*;
    use crate::brain::ActionSet;
    use crate::game_mode::GameMode;
    use crate::player::{PlayerEntity, PlayerMana, PrimaryPlayer};
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::backend::HitData;
    use bevy::picking::events::{Click, Move, Pointer};
    use bevy::picking::pointer::{Location, PointerId};
    use core::time::Duration;

    // ---- Fix 1: back-edge seeding --------------------------------------------

    #[test]
    fn back_edge_lands_opposite_the_direction_travelled() {
        // Turning RIGHT brings the viewer-right page to front; to go BACK you turn
        // left, so the cursor lands on the LEFT edge button — and vice-versa.
        let from = CubePage::Items;
        let right = from.on_viewer_right();
        assert_eq!(back_edge_focus(Some(from), right), CubeFocus::EdgeLeft);
        let left = from.on_viewer_left();
        assert_eq!(back_edge_focus(Some(from), left), CubeFocus::EdgeRight);
        // First open (no prior page) defaults to a highlighted left edge button.
        assert_eq!(back_edge_focus(None, CubePage::Map), CubeFocus::EdgeLeft);
    }

    // ---- Fix 4: System-page pointer clicks -----------------------------------

    fn click_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameMode>();
        app.init_resource::<InventoryUiBackend>();
        app.init_resource::<ActiveMenuPages<CubePage, CubeAction>>();
        app.init_resource::<CubeCursor>();
        app.init_resource::<CubeSystemNav>();
        app.init_resource::<OwnedItems>();
        app.init_resource::<crate::dev::dev_tools::DeveloperTools>();
        app.init_resource::<crate::runtime::reset::SandboxResetRequested>();
        app.init_resource::<UserSettings>();
        app.init_resource::<crate::inventory::InventoryUiState>();
        app.add_message::<PlayerHealRequested>();
        app.add_message::<SfxMessage>();
        app.add_observer(cube_pointer_click);
        *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::Cube;
        app.world_mut()
            .resource_mut::<crate::inventory::InventoryUiState>()
            .visible = true;
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                ActionSet::default(),
                PlayerMana::default(),
            ))
            .id();
        app.update();
        (app, player)
    }

    fn open_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameMode>();
        app.init_resource::<InventoryUiBackend>();
        app.init_resource::<ActiveMenuPages<CubePage, CubeAction>>();
        app.init_resource::<CubeCursor>();
        app.init_resource::<CubeSystemNav>();
        app.init_resource::<OwnedItems>();
        app.init_resource::<crate::dev::dev_tools::DeveloperTools>();
        app.init_resource::<crate::runtime::reset::SandboxResetRequested>();
        app.init_resource::<UserSettings>();
        app.init_resource::<crate::inventory::InventoryUiState>();
        app.init_resource::<crate::map_menu::MapMenuState>();
        app.init_resource::<MenuControlFrame>();
        app.add_message::<PlayerHealRequested>();
        app.add_message::<SfxMessage>();
        app.add_systems(Update, cube_menu_open_routing);
        app.add_observer(cube_pointer_move);
        *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::Cube;
        app.world_mut()
            .resource_mut::<crate::inventory::InventoryUiState>()
            .visible = false;
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            ActionSet::default(),
            PlayerMana::default(),
        ));
        app.update();
        app
    }

    /// Spawn a cube control carrying `action` and fire a real `Pointer<Click>` at it,
    /// exactly as Bevy picking would.
    fn click_control(app: &mut App, action: CubeAction) {
        let entity = app
            .world_mut()
            .spawn(AmbitionMenuControl::<CubeAction> {
                kind: ambition_inventory_ui::MenuControlKind::OptionToggle,
                action: Some(action),
                focus: ambition_inventory_ui::MenuFocusKey::default(),
            })
            .id();
        // The observer only reads `click.entity`; any render target works for the
        // location, so the simplest no-render target keeps the fixture minimal.
        let location = Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        };
        let click = Pointer::new(
            PointerId::Mouse,
            location,
            Click {
                button: bevy::picking::pointer::PointerButton::Primary,
                hit: HitData::new(entity, 0.0, None, None),
                duration: Duration::ZERO,
            },
            entity,
        );
        app.world_mut().trigger(click);
        app.update();
    }

    fn move_control(app: &mut App, action: CubeAction) {
        let entity = app
            .world_mut()
            .spawn(AmbitionMenuControl::<CubeAction> {
                kind: ambition_inventory_ui::MenuControlKind::OptionToggle,
                action: Some(action),
                focus: ambition_inventory_ui::MenuFocusKey::default(),
            })
            .id();
        let location = Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        };
        let pointer_move = Pointer::new(
            PointerId::Mouse,
            location,
            Move {
                hit: HitData::new(entity, 0.0, None, None),
                delta: Vec2::new(1.0, 0.0),
            },
            entity,
        );
        app.world_mut().trigger(pointer_move);
        app.update();
    }

    // ---- Fix 2: shoulder-bumper page turns -----------------------------------

    fn nav_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameMode>();
        app.init_resource::<InventoryUiBackend>();
        app.init_resource::<ActiveMenuPages<CubePage, CubeAction>>();
        app.init_resource::<CubeCursor>();
        app.init_resource::<CubeSystemNav>();
        app.init_resource::<OwnedItems>();
        app.init_resource::<crate::dev::dev_tools::DeveloperTools>();
        app.init_resource::<crate::runtime::reset::SandboxResetRequested>();
        app.init_resource::<UserSettings>();
        app.init_resource::<crate::inventory::InventoryUiState>();
        app.init_resource::<MenuControlFrame>();
        app.add_message::<PlayerHealRequested>();
        app.add_message::<SfxMessage>();
        app.add_systems(Update, cube_focus_nav);
        *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::Cube;
        app.world_mut()
            .resource_mut::<crate::inventory::InventoryUiState>()
            .visible = true;
        app.world_mut()
            .resource_mut::<ActiveMenuPages<CubePage, CubeAction>>()
            .active = Some(CubePage::Items);
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            ActionSet::default(),
            PlayerMana::default(),
        ));
        app.update();
        app
    }

    fn system_nav_app(focus: CubeFocus) -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameMode>();
        app.init_resource::<InventoryUiBackend>();
        app.init_resource::<ActiveMenuPages<CubePage, CubeAction>>();
        app.init_resource::<CubeCursor>();
        app.init_resource::<CubeSystemNav>();
        app.init_resource::<OwnedItems>();
        app.init_resource::<crate::dev::dev_tools::DeveloperTools>();
        app.init_resource::<crate::runtime::reset::SandboxResetRequested>();
        app.init_resource::<UserSettings>();
        app.init_resource::<crate::inventory::InventoryUiState>();
        app.init_resource::<crate::map_menu::MapMenuState>();
        app.init_resource::<MenuControlFrame>();
        app.add_message::<PlayerHealRequested>();
        app.add_message::<SfxMessage>();
        app.add_systems(Update, cube_focus_nav);
        *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::Cube;
        app.world_mut()
            .resource_mut::<crate::inventory::InventoryUiState>()
            .visible = true;
        app.world_mut()
            .resource_mut::<ActiveMenuPages<CubePage, CubeAction>>()
            .active = Some(CubePage::System);
        app.world_mut().resource_mut::<CubeCursor>().focus = focus;
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            ActionSet::default(),
            PlayerMana::default(),
        ));
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
                .resource::<ActiveMenuPages<CubePage, CubeAction>>()
                .active,
            Some(CubePage::Items.on_viewer_right()),
            "right bumper rotates to the viewer-right page (Fix 2)"
        );
        // The cursor lands on the new page's back-edge button (Fix 1): arriving from
        // the right edge means the LEFT edge button turns back home.
        assert_eq!(
            app.world().resource::<CubeCursor>().focus,
            CubeFocus::EdgeLeft,
            "cursor seeds onto the back (left) edge button"
        );
    }

    #[test]
    fn left_bumper_turns_to_the_viewer_left_page() {
        let mut app = nav_app();
        press_bumper(&mut app, false);
        assert_eq!(
            app.world()
                .resource::<ActiveMenuPages<CubePage, CubeAction>>()
                .active,
            Some(CubePage::Items.on_viewer_left()),
            "left bumper rotates to the viewer-left page (Fix 2)"
        );
        assert_eq!(
            app.world().resource::<CubeCursor>().focus,
            CubeFocus::EdgeRight,
            "cursor seeds onto the back (right) edge button"
        );
    }

    #[test]
    fn clicking_a_system_entry_drills_in() {
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<CubePage, CubeAction>>()
            .active = Some(CubePage::System);
        assert!(app.world().resource::<CubeSystemNav>().open_entry.is_none());
        click_control(
            &mut app,
            CubeAction::OpenSystemEntry(SystemMenuEntryId::Audio),
        );
        assert_eq!(
            app.world().resource::<CubeSystemNav>().open_entry,
            Some(SystemMenuEntryId::Audio),
            "clicking a System entry drills into it (Fix 4)"
        );
    }

    #[test]
    fn clicking_a_system_setting_applies_it() {
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<CubePage, CubeAction>>()
            .active = Some(CubePage::System);
        app.world_mut().resource_mut::<CubeSystemNav>().open_entry = Some(SystemMenuEntryId::Video);
        let before = app.world().resource::<UserSettings>().video.show_fps;
        click_control(&mut app, CubeAction::System(SettingsOptionId::ShowFps));
        let after = app.world().resource::<UserSettings>().video.show_fps;
        assert_ne!(before, after, "clicking a setting toggles it (Fix 4)");
    }

    #[test]
    fn clicking_back_drills_out_to_the_entry_list() {
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<CubePage, CubeAction>>()
            .active = Some(CubePage::System);
        app.world_mut().resource_mut::<CubeSystemNav>().open_entry = Some(SystemMenuEntryId::Audio);
        click_control(&mut app, CubeAction::CloseSystemEntry);
        assert!(
            app.world().resource::<CubeSystemNav>().open_entry.is_none(),
            "clicking Back drills out to the entry list (Fix 4)"
        );
    }

    #[test]
    fn clicking_a_radio_station_keeps_the_menu_open() {
        // Selecting a radio station auditions it WITHOUT closing the cube, so the
        // user can keep browsing. Audio is absent under `oot_inventory`, so the apply
        // no-ops, but the menu must still stay open.
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<CubePage, CubeAction>>()
            .active = Some(CubePage::System);
        app.world_mut().resource_mut::<CubeSystemNav>().open_entry = Some(SystemMenuEntryId::Radio);
        click_control(&mut app, CubeAction::SystemOption(SystemOptionId::Radio(0)));
        assert!(
            app.world()
                .resource::<crate::inventory::InventoryUiState>()
                .visible,
            "auditioning a station keeps the cube open"
        );
    }

    #[test]
    fn system_edge_left_moves_inward_to_the_row_list() {
        let mut app = system_nav_app(CubeFocus::EdgeLeft);
        let mut frame = MenuControlFrame::default();
        frame.right = true;
        app.insert_resource(frame);
        app.update();

        assert_eq!(
            app.world().resource::<CubeCursor>().focus,
            CubeFocus::System(0),
            "moving right from the < Items button enters the System row list instead of rotating"
        );
        assert_eq!(
            app.world()
                .resource::<ActiveMenuPages<CubePage, CubeAction>>()
                .active,
            Some(CubePage::System),
            "the cube stays on the System face while moving into the rows"
        );
    }

    #[test]
    fn system_row_horizontal_moves_to_the_edge_buttons() {
        let mut app = system_nav_app(CubeFocus::System(1));
        let mut frame = MenuControlFrame::default();
        frame.left = true;
        app.insert_resource(frame);
        app.update();

        assert_eq!(
            app.world().resource::<CubeCursor>().focus,
            CubeFocus::EdgeLeft,
            "horizontal motion from a row should land on the left edge button"
        );
    }

    #[test]
    fn pointer_motion_selects_a_cube_control() {
        let mut app = open_app();
        app.world_mut()
            .resource_mut::<crate::inventory::InventoryUiState>()
            .visible = true;
        app.world_mut()
            .resource_mut::<ActiveMenuPages<CubePage, CubeAction>>()
            .active = Some(CubePage::Items);
        app.world_mut().resource_mut::<CubeCursor>().focus = CubeFocus::EdgeRight;

        move_control(
            &mut app,
            CubeAction::ChangePage(CubePage::Items.on_viewer_left()),
        );

        assert_eq!(
            app.world().resource::<CubeCursor>().focus,
            CubeFocus::EdgeLeft,
            "actual pointer motion updates the cube cursor"
        );
        assert_eq!(
            app.world().resource::<CubeCursor>().last_pointer_focus,
            Some(CubeFocus::EdgeLeft),
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
    fn esc_backs_out_then_closes_the_cube_via_real_input() {
        use crate::input::SandboxAction;
        use crate::presentation::rendering::PlayerVisual;
        use leafwing_input_manager::prelude::*;

        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.add_plugins(bevy::time::TimePlugin);
        app.add_plugins(bevy::input::InputPlugin);
        app.add_plugins(InputManagerPlugin::<SandboxAction>::default());
        app.init_state::<GameMode>();
        app.init_resource::<InventoryUiBackend>();
        app.init_resource::<ActiveMenuPages<CubePage, CubeAction>>();
        app.init_resource::<CubeCursor>();
        app.init_resource::<CubeSystemNav>();
        app.init_resource::<OwnedItems>();
        app.init_resource::<crate::dev::dev_tools::DeveloperTools>();
        app.init_resource::<crate::runtime::reset::SandboxResetRequested>();
        app.init_resource::<UserSettings>();
        app.init_resource::<crate::inventory::InventoryUiState>();
        app.init_resource::<crate::map_menu::MapMenuState>();
        app.init_resource::<MenuControlFrame>();
        app.init_resource::<crate::input::MenuInputState>();
        app.init_resource::<crate::oot_menu::OotMenuState>();
        app.add_message::<PlayerHealRequested>();
        app.add_message::<SfxMessage>();
        app.add_systems(
            Update,
            (
                crate::app::populate_menu_control_frame_from_actions,
                crate::oot_menu::oot_menu_input,
                cube_menu_open_routing,
                cube_focus_nav,
            )
                .chain(),
        );
        *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::Cube;

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
                .resource::<crate::inventory::InventoryUiState>()
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
            .resource_mut::<ActiveMenuPages<CubePage, CubeAction>>()
            .active = Some(CubePage::System);
        app.world_mut().resource_mut::<CubeSystemNav>().open_entry = Some(SystemMenuEntryId::Audio);
        app.world_mut().resource_mut::<CubeCursor>().focus = CubeFocus::System(0);

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
            app.world().resource::<CubeSystemNav>().open_entry.is_none(),
            "second Esc drilled out of the open System entry"
        );

        // Third Esc press → now at the top level, CLOSES the cube.
        press_esc(&mut app, true);
        app.update();
        press_esc(&mut app, false);
        app.update();
        assert!(!visible(&app), "third Esc (top level) closes the cube");
    }

    /// Bug 1 regression: Esc closes the cube and it STAYS closed. Reproduces the
    /// real failure by including the bevy-UI pause menu's `pause_menu_navigate` in
    /// the SAME schedule as the cube routing. Before the fix, `pause_menu_navigate`
    /// ran on every `Paused` frame (it is only gated on `Paused`), and its Top-page
    /// nav would re-raise `InventoryUiState.visible` behind the invisible cube — so
    /// the menu the cube just closed popped straight back open. With the Cube-backend
    /// guard added to `pause_menu_navigate`, the bevy-UI pause menu is inert under the
    /// cube and the close sticks.
    ///
    /// Regression guard: removing the `pause_menu_ui_active` gate makes this test
    /// fail (the now-ungated `pause_menu_navigate` runs under the cube and re-raises
    /// `visible` / accesses pause-menu resources this minimal fixture omits) — so the
    /// re-open can never silently come back.
    #[test]
    fn esc_close_stays_closed_with_pause_menu_in_schedule() {
        use crate::input::SandboxAction;
        use crate::presentation::rendering::PlayerVisual;
        use leafwing_input_manager::prelude::*;

        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.add_plugins(bevy::time::TimePlugin);
        app.add_plugins(bevy::input::InputPlugin);
        app.add_plugins(InputManagerPlugin::<SandboxAction>::default());
        app.init_state::<GameMode>();
        app.init_resource::<InventoryUiBackend>();
        app.init_resource::<ActiveMenuPages<CubePage, CubeAction>>();
        app.init_resource::<CubeCursor>();
        app.init_resource::<CubeSystemNav>();
        app.init_resource::<OwnedItems>();
        app.init_resource::<crate::dev::dev_tools::DeveloperTools>();
        app.init_resource::<crate::runtime::reset::SandboxResetRequested>();
        app.init_resource::<UserSettings>();
        app.init_resource::<crate::inventory::InventoryUiState>();
        app.init_resource::<crate::map_menu::MapMenuState>();
        app.init_resource::<MenuControlFrame>();
        app.init_resource::<crate::input::MenuInputState>();
        app.init_resource::<crate::oot_menu::OotMenuState>();
        app.init_resource::<crate::pause_menu::PauseMenuState>();
        app.init_resource::<crate::host::windowing::DisplayModeState>();
        app.init_resource::<crate::dev::dev_tools::EditableMovementTuning>();
        app.init_resource::<crate::SandboxDevState>();
        app.init_resource::<crate::ldtk_world::LdtkHotReloadState>();
        app.add_message::<PlayerHealRequested>();
        app.add_message::<SfxMessage>();
        app.add_message::<bevy::app::AppExit>();
        // The cube routing (owns Esc under the cube) PLUS the bevy-UI pause menu
        // navigate system, in the same Update set, exactly mirroring the real app's
        // wiring where both are registered. The pause-menu system is the one that
        // used to stomp `visible` back on.
        app.add_systems(
            Update,
            (
                crate::app::populate_menu_control_frame_from_actions,
                crate::oot_menu::oot_menu_input,
                cube_menu_open_routing,
                // Exactly as registered in the real app (`app/plugins.rs`): gated by
                // `pause_menu_ui_active` so it is inert under the Cube backend. This
                // run-condition IS the Bug 1 fix; the assertions below would fail if it
                // were removed (the navigate system would re-raise `visible`).
                crate::pause_menu::pause_menu_navigate
                    .run_if(crate::pause_menu::pause_menu_ui_active),
                cube_focus_nav,
            )
                .chain(),
        );
        *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::Cube;

        let mut map = InputMap::<SandboxAction>::default();
        map.insert(SandboxAction::Start, KeyCode::Escape);
        map.insert(SandboxAction::MenuBack, KeyCode::Escape);
        app.world_mut().spawn((
            PlayerVisual,
            PlayerEntity,
            PrimaryPlayer,
            ActionSet::default(),
            PlayerMana::default(),
            crate::player::BodyKinematics::default(),
            crate::player::PlayerAbilities::default(),
            crate::player::PlayerDashState::default(),
            crate::player::PlayerJumpState::default(),
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
                .resource::<crate::inventory::InventoryUiState>()
                .visible
        };

        // Open the cube (lands on the System top level).
        press_esc(&mut app, true);
        app.update();
        press_esc(&mut app, false);
        app.update();
        assert!(visible(&app), "first Esc opens the cube");
        assert!(
            matches!(
                app.world().resource::<State<GameMode>>().get(),
                GameMode::Paused
            ),
            "opening the cube pauses the game"
        );

        // Close it (top level → close). It must STAY closed across the close frame
        // and several idle Paused frames afterwards: the bevy-UI pause menu must not
        // re-raise `visible`.
        press_esc(&mut app, true);
        app.update();
        press_esc(&mut app, false);
        app.update();
        assert!(!visible(&app), "second Esc closes the cube");
        for _ in 0..5 {
            app.update();
            assert!(
                !visible(&app),
                "the cube stays closed — pause_menu_navigate must not re-open it under the Cube backend"
            );
        }
    }

    #[test]
    fn opening_the_cube_clears_stale_pointer_hover_state() {
        let mut app = open_app();
        app.world_mut()
            .resource_mut::<CubeCursor>()
            .last_pointer_focus = Some(CubeFocus::Item(7));
        app.world_mut().resource_mut::<MenuControlFrame>().start = true;
        app.world_mut()
            .resource_mut::<crate::inventory::InventoryUiState>()
            .visible = false;
        app.update();

        assert_eq!(
            app.world().resource::<CubeCursor>().last_pointer_focus,
            None,
            "opening the cube clears stale pointer hover state so parked hover cannot select immediately"
        );
    }
}
