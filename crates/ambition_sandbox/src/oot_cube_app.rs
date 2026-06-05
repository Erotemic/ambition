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
use ambition_inventory_ui::{
    ActiveMenuPages, AmbitionInventoryUiPlugin, AmbitionMenuControl,
};
use bevy::prelude::*;

use crate::input::MenuControlFrame;
use crate::items::{Item, OwnedItems, ITEM_GRID_COLS, ITEM_GRID_ROWS};
use crate::oot_cube::{build_inventory_pages, CubeAction, CubeFocus, CubePage, SystemOption};
use crate::oot_menu::input::{dispatch_item_confirm, MenuEffectManaQuery, MenuEffectPlayers};
use crate::persistence::settings::{AudioSettings, UserSettings};
use crate::player::PlayerHealRequested;

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
        .add_plugins(AmbitionInventoryUiPlugin)
        .add_plugins(CubeMenuPlugin::<CubePage, CubeAction>::default())
        .add_systems(Startup, spawn_cube_scrim)
        .add_systems(
            Update,
            (
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
        .add_observer(cube_pointer_over)
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
    /// The last focus the POINTER hovered. A parked mouse re-fires `Pointer<Over>`
    /// every model rebuild (new entities spawn under the cursor); gating on this
    /// means a stationary hover over the same logical focus is a no-op, so it can't
    /// (a) loop the rebuild or (b) override a later keyboard move.
    last_pointer_focus: Option<CubeFocus>,
}

impl CubeCursor {
    /// Keyboard/gamepad nav took the cursor to `focus` (claims ownership).
    fn mark_keyboard(&mut self, focus: CubeFocus) {
        self.focus = focus;
        self.owner = FocusSource::Keyboard;
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
        commands.entity(entity).insert(UiTargetCamera(main_camera.0));
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
) {
    if *backend != InventoryUiBackend::Cube || !overlay.visible {
        return;
    }
    let Some(active_page) = pages.active else {
        return;
    };

    // Directional intent (one step; the menu frame already debounces repeat).
    let dx = (menu.right as i32) - (menu.left as i32);
    let dy = (menu.down as i32) - (menu.up as i32);

    // The System face is an interactive option list: UP/DOWN move the cursor
    // between rows, LEFT/RIGHT at the column edges turn the page (or step a
    // value), and SELECT applies the focused option.
    if active_page == CubePage::System {
        system_focus_nav(
            &menu, dx, dy, &mut cursor, &mut pages, &mut overlay, &mut settings, active_page,
            &mut owned, &mut commands, &mut players, &mut mana_q, &mut heals,
        );
        return;
    }

    // Other non-items faces only respond to horizontal page turns (matches the
    // demo's early branch in move_spatial).
    if active_page != CubePage::Items {
        if dx < 0 {
            turn_page(&mut pages, active_page.on_viewer_left());
        } else if dx > 0 {
            turn_page(&mut pages, active_page.on_viewer_right());
        }
        if menu.back {
            overlay.visible = false;
        }
        return;
    }

    if dx != 0 || dy != 0 {
        match move_spatial(cursor.focus, dx, dy, active_page) {
            SpatialMove::Focus(next) => cursor.mark_keyboard(next),
            SpatialMove::TurnLeft => {
                turn_page(&mut pages, active_page.on_viewer_left());
                // Land the cursor on the new face's right arrow (so pressing back
                // toward centre re-enters the grid) — demo's turn_page_from_edge.
                cursor.mark_keyboard(CubeFocus::EdgeRight);
            }
            SpatialMove::TurnRight => {
                turn_page(&mut pages, active_page.on_viewer_right());
                cursor.mark_keyboard(CubeFocus::EdgeLeft);
            }
        }
    }

    if menu.back {
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
                &mut owned,
                &mut settings,
                &mut close_menu,
                &mut commands,
                &mut players,
                &mut mana_q,
                &mut heals,
            );
            if close_menu {
                overlay.visible = false;
            }
        }
    }
}

/// Directional navigation + select for the System face. UP/DOWN move the cursor
/// over [`SystemOption::ALL`]; from the leftmost/rightmost edge LEFT/RIGHT turns
/// the page; SELECT applies the focused option (volume/zoom rows also respond to
/// LEFT/RIGHT to step). `back` closes the menu. Mutations go through
/// [`apply_system_option`] so persistence stays in one place.
#[allow(clippy::too_many_arguments)]
fn system_focus_nav(
    menu: &MenuControlFrame,
    dx: i32,
    dy: i32,
    cursor: &mut CubeCursor,
    pages: &mut ActiveMenuPages<CubePage, CubeAction>,
    overlay: &mut crate::inventory::InventoryUiState,
    settings: &mut UserSettings,
    active_page: CubePage,
    owned: &mut OwnedItems,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers,
    mana_q: &mut MenuEffectManaQuery,
    heals: &mut MessageWriter<PlayerHealRequested>,
) {
    let count = SystemOption::ALL.len() as i32;
    // Normalise the cursor onto a System row (it may arrive as an items/edge focus
    // after a page turn).
    let mut row = match cursor.focus {
        CubeFocus::System(idx) => idx as i32,
        _ => 0,
    };

    if dy != 0 {
        row = (row + dy).clamp(0, count - 1);
        cursor.mark_keyboard(CubeFocus::System(row as usize));
    }

    let option = SystemOption::ALL[row.max(0).min(count - 1) as usize];

    if dx != 0 {
        // LEFT/RIGHT step value rows in place; for non-value rows they turn the page.
        let is_value_row = matches!(
            option,
            SystemOption::CycleMasterVolume
                | SystemOption::CycleMusicVolume
                | SystemOption::CycleSfxVolume
                | SystemOption::CycleCameraZoom
        );
        if is_value_row {
            apply_system_option_step(option, dx, settings);
        } else if dx < 0 {
            turn_page(pages, active_page.on_viewer_left());
            cursor.mark_keyboard(CubeFocus::System(0));
        } else {
            turn_page(pages, active_page.on_viewer_right());
            cursor.mark_keyboard(CubeFocus::System(0));
        }
    }

    if menu.back {
        overlay.visible = false;
        return;
    }

    if menu.select {
        let mut close_menu = false;
        dispatch_cube_action(
            CubeAction::System(option),
            pages,
            owned,
            settings,
            &mut close_menu,
            commands,
            players,
            mana_q,
            heals,
        );
        if close_menu {
            overlay.visible = false;
        }
    }
}

/// Apply a signed LEFT/RIGHT step to a value-style System option (volume up/down,
/// camera-zoom prev/next). Toggle/close rows ignore stepping (they only respond
/// to SELECT). Persistence is automatic via `UserSettings` change detection.
fn apply_system_option_step(option: SystemOption, dx: i32, settings: &mut UserSettings) {
    match option {
        SystemOption::CycleMasterVolume => {
            settings.audio.nudge_master(step_sign(dx) * AudioSettings::VOLUME_STEP);
        }
        SystemOption::CycleMusicVolume => {
            settings.audio.nudge_music(step_sign(dx) * AudioSettings::VOLUME_STEP);
        }
        SystemOption::CycleSfxVolume => {
            settings.audio.nudge_sfx(step_sign(dx) * AudioSettings::VOLUME_STEP);
        }
        SystemOption::CycleCameraZoom => {
            settings.video.camera_zoom = if dx < 0 {
                settings.video.camera_zoom.prev()
            } else {
                settings.video.camera_zoom.next()
            };
        }
        _ => {}
    }
}

fn step_sign(dx: i32) -> f32 {
    if dx < 0 {
        -1.0
    } else {
        1.0
    }
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

/// Set the active page (the lib rotates that face to the camera).
fn turn_page(pages: &mut ActiveMenuPages<CubePage, CubeAction>, page: CubePage) {
    if pages.active != Some(page) {
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
    owned: &mut OwnedItems,
    settings: &mut UserSettings,
    close_menu: &mut bool,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers,
    mana_q: &mut MenuEffectManaQuery,
    heals: &mut MessageWriter<PlayerHealRequested>,
) {
    match action {
        CubeAction::Equip(item) | CubeAction::Use(item) => {
            let decided = dispatch_item_confirm(item, owned, commands, players, mana_q, heals);
            info!("cube action: {:?} \u{2192} {:?}", item, decided);
        }
        CubeAction::ChangePage(page) => {
            pages.active = Some(page);
            info!("cube page \u{2192} {:?}", page);
        }
        CubeAction::System(option) => {
            apply_system_option(option, settings, close_menu);
        }
    }
}

/// Apply a System-face option by mutating `UserSettings` (toggles flip the bool;
/// volume rows step via the audio settings' own `nudge_*` clamping helpers; the
/// camera-zoom row cycles the preset enum). Persistence is NOT re-implemented
/// here: the existing `save_settings_on_change` system writes `settings.ron`
/// whenever `UserSettings` changes, so mutating the resource is the whole job.
/// `CloseMenu` raises `close_menu` for the caller to fold back into the overlay.
fn apply_system_option(option: SystemOption, settings: &mut UserSettings, close_menu: &mut bool) {
    match option {
        SystemOption::ToggleFps => settings.video.show_fps = !settings.video.show_fps,
        SystemOption::ToggleDebugHud => {
            settings.gameplay.debug_hud_visible = !settings.gameplay.debug_hud_visible;
        }
        SystemOption::ToggleQuestHud => {
            settings.gameplay.quest_hud_visible = !settings.gameplay.quest_hud_visible;
        }
        SystemOption::ToggleTouchControls => {
            settings.controls.touch_controls_visible = !settings.controls.touch_controls_visible;
        }
        SystemOption::ToggleMute => settings.audio.toggle_mute(),
        // Volume rows confirm-cycle UP by one step (wrapping at the ceiling), so a
        // single select/tap keeps stepping the value the way a slider would. The
        // audio settings' own `nudge_*` helpers do the clamping + mute coupling.
        SystemOption::CycleMasterVolume => {
            settings.audio.master_volume = step_volume(settings.audio.master_volume);
            // Raising master while muted unmutes, matching `nudge_master`.
            if settings.audio.master_volume > 0.0 && settings.audio.muted {
                settings.audio.muted = false;
            }
        }
        SystemOption::CycleMusicVolume => {
            settings.audio.music_volume = step_volume(settings.audio.music_volume);
        }
        SystemOption::CycleSfxVolume => {
            settings.audio.sfx_volume = step_volume(settings.audio.sfx_volume);
        }
        SystemOption::CycleCameraZoom => {
            settings.video.camera_zoom = settings.video.camera_zoom.next();
        }
        SystemOption::CloseMenu => *close_menu = true,
    }
    info!("cube system option: {:?}", option);
}

/// Step a 0..=1 volume up by one `AudioSettings::VOLUME_STEP`, wrapping back to
/// 0 once it passes the ceiling. Single-select "cycle" behaviour for the System
/// face's volume rows (no L/R needed).
fn step_volume(current: f32) -> f32 {
    let step = AudioSettings::VOLUME_STEP;
    let next = current + step;
    if next > 1.0 + step * 0.5 {
        0.0
    } else {
        next.clamp(0.0, 1.0)
    }
}

/// Map a control's `CubeAction` back to the cursor focus it represents, so pointer
/// hover/click and the keyboard cursor share one model. `Equip`/`Use` carry the
/// item (→ its slot); `ChangePage` is an edge arrow — left vs right is decided by
/// whether its target is the active page's viewer-left neighbour.
fn focus_for_action(action: CubeAction, active_page: CubePage) -> CubeFocus {
    match action {
        CubeAction::Equip(item) | CubeAction::Use(item) => CubeFocus::Item(item.index()),
        CubeAction::ChangePage(target) => {
            if target == active_page.on_viewer_left() {
                CubeFocus::EdgeLeft
            } else {
                CubeFocus::EdgeRight
            }
        }
        CubeAction::System(option) => {
            let idx = SystemOption::ALL
                .iter()
                .position(|o| *o == option)
                .unwrap_or(0);
            CubeFocus::System(idx)
        }
    }
}

/// Pointer hover (mouse/touch) over a cube control: move the focus cursor to it —
/// but ONLY on a genuine pointer move to a DIFFERENT control. Bevy picking fires
/// this for mouse AND touch uniformly.
///
/// Two guards (both essential), mirroring the grid's `MenuFocusState`:
///
/// 1. **Semantic dedup.** Every model rebuild despawns/respawns the controls, so a
///    parked mouse re-fires `Pointer<Over>` on a NEW entity that maps to the SAME
///    logical [`CubeFocus`]. We compare the hovered focus against `last_pointer_focus`
///    and bail when unchanged → no `CubeCursor` write → no rebuild → the
///    "rebuilding 4 faces" loop is broken.
/// 2. **Pointer-vs-keyboard ownership.** When the hovered focus equals the last one
///    the pointer reported, the mouse hasn't moved; we leave the cursor alone even
///    if keyboard nav has since taken it elsewhere. The pointer only re-claims the
///    cursor when it moves onto a genuinely different control. This fixes "can't
///    move away from the hovered option."
fn cube_pointer_over(
    over: On<Pointer<Over>>,
    controls: Query<&AmbitionMenuControl<CubeAction>>,
    pages: Res<ActiveMenuPages<CubePage, CubeAction>>,
    mut cursor: ResMut<CubeCursor>,
) {
    let Some(active_page) = pages.active else {
        return;
    };
    if let Ok(control) = controls.get(over.entity) {
        if let Some(action) = control.action {
            let next = focus_for_action(action, active_page);
            // The pointer hasn't moved to a new control (same logical focus, just a
            // freshly-rebuilt entity under a parked mouse): do nothing. This is the
            // single guard that breaks the rebuild loop AND prevents the parked
            // mouse from locking the cursor against keyboard nav.
            if cursor.last_pointer_focus == Some(next) {
                return;
            }
            cursor.last_pointer_focus = Some(next);
            if cursor.focus != next {
                cursor.focus = next;
                cursor.owner = FocusSource::Pointer;
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
    mut owned: ResMut<OwnedItems>,
    mut settings: ResMut<UserSettings>,
    mut commands: Commands,
    mut players: MenuEffectPlayers,
    mut mana_q: MenuEffectManaQuery,
    mut heals: MessageWriter<PlayerHealRequested>,
) {
    let open = ui_state.as_deref().map(|s| s.visible).unwrap_or(false);
    if *backend != InventoryUiBackend::Cube || !open {
        return;
    }
    if let Ok(control) = controls.get(click.entity) {
        if let Some(action) = control.action {
            if let Some(active_page) = pages.active {
                let next = focus_for_action(action, active_page);
                cursor.focus = next;
                cursor.owner = FocusSource::Pointer;
                cursor.last_pointer_focus = Some(next);
            }
            let mut close_menu = false;
            dispatch_cube_action(
                action,
                &mut pages,
                &mut owned,
                &mut settings,
                &mut close_menu,
                &mut commands,
                &mut players,
                &mut mana_q,
                &mut heals,
            );
            if close_menu {
                if let Some(ui_state) = ui_state.as_deref_mut() {
                    ui_state.visible = false;
                }
            }
        }
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
    mut cameras: Query<(&mut Camera, Has<ambition_inventory_ui::cube::CubePauseCamera>)>,
    mut rings: Query<&mut Visibility, With<ambition_inventory_ui::cube::MenuRing>>,
    mut last_show: Local<Option<bool>>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    let show = *backend == InventoryUiBackend::Cube && open;
    if *last_show != Some(show) {
        info!("cube gate: show={show} backend={:?} menu_open={open}", *backend);
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
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
    mut was_open: Local<bool>,
    mut last: Local<Option<(CubeFocus, Option<CubePage>)>>,
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

    let key = (cursor.focus, pages.active);
    // Republish on: catalog change, settings change (so a toggled System option's
    // label updates immediately), first publish, menu-open (textures that loaded
    // after the initial build get picked up), cursor move, or page change. The
    // open case fixes icons rendering blank until the first rotate.
    let dirty = owned.is_changed()
        || settings.is_changed()
        || pages.pages.is_empty()
        || just_opened
        || *last != Some(key);
    if !dirty {
        return;
    }
    *last = Some(key);

    let active = pages.active.unwrap_or(CubePage::Items);
    pages.pages = build_inventory_pages(&owned, owned.equipped(), cursor.focus, &settings);
    pages.active = Some(active);
}
