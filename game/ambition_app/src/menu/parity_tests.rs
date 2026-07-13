//! Cross-backend menu parity / no-drift safety net (design doc §8).
//!
//! The unified menu has ONE content model
//! ([`crate::menu::model::build_inventory_pages`] + the settings IR in
//! [`ambition::settings_menu`]) rendered by TWO presentations (the flat Grid
//! [`crate::menu::grid_backend`] and the 3D cube [`crate::menu::kaleidoscope_app`]),
//! dispatched through ONE [`crate::menu::dispatch::dispatch_menu_action`]. The
//! tests here LOCK that the two presentations can never silently drift:
//!
//! 1. **No-drift exhaustiveness** — every [`SettingsOptionId`] is surfaced by the
//!    settings IR (`settings_menu_model`), and every [`SystemMenuEntryId`] is
//!    surfaced by the System model (`SystemMenuModel::build`). The match/iteration
//!    is EXHAUSTIVE over the enums, so adding a new variant fails compilation (the
//!    match arm) or the test (the surfaced-set assertion) until it is wired into a
//!    presentation — the test is a living inventory, not a silent gap.
//! 2. **Dispatch parity** — for a representative set of actions (equip an item,
//!    change page, toggle a setting, drill a System entry), the GRID's real
//!    pointer-release path and the CUBE's real pointer-release path produce the
//!    SAME observable effect, because both route through the one dispatcher.
//! 3. **Content parity** — for each [`MenuPage`] at a fixed state, the actionable
//!    controls the GRID renders equal what the CUBE renders, EXCEPT the cube's
//!    `MenuPageAction::ChangePage` edge page-turn controls which the Grid strips.

use super::model::{build_inventory_pages, MenuFocus, MenuPage, MenuPageAction};
use ambition::items::{Item, OwnedItems};
use ambition::persistence::settings::UserSettings;
use ambition::settings_menu::settings::{settings_menu_model, SettingsOptionId};
use ambition::settings_menu::system::{
    DevSnapshot, RadioSnapshot, SystemMenuEntryId, SystemMenuModel, SystemMenuTarget,
};

// ---------------------------------------------------------------------------
// Part 1a — no-drift exhaustiveness (settings IR + System model)
// ---------------------------------------------------------------------------

/// EVERY [`SettingsOptionId`] variant, listed exhaustively. The trailing
/// `match`-less destructuring of a value below forces this list to stay in sync:
/// a NEW variant fails to compile here until it is added, and adding it here but
/// not to the IR fails [`every_settings_option_is_surfaced_by_the_settings_ir`].
const ALL_SETTINGS_OPTION_IDS: &[SettingsOptionId] = &[
    SettingsOptionId::DisplayMode,
    SettingsOptionId::CameraZoom,
    SettingsOptionId::CameraAspect,
    SettingsOptionId::CameraFraming,
    SettingsOptionId::Flashes,
    SettingsOptionId::Colorblind,
    SettingsOptionId::ShowFps,
    SettingsOptionId::FramePacing,
    SettingsOptionId::VisualQuality,
    SettingsOptionId::ShaderStrength,
    SettingsOptionId::ShaderCrtStrength,
    SettingsOptionId::ShaderCrtScanlines,
    SettingsOptionId::ShaderCrtMask,
    SettingsOptionId::ShaderCrtCurvature,
    SettingsOptionId::ShaderCrtBloom,
    SettingsOptionId::ShaderCrtChroma,
    SettingsOptionId::ShaderFilmGrainStrength,
    SettingsOptionId::ShaderFilmGrainSize,
    SettingsOptionId::ShaderFilmGrainFps,
    SettingsOptionId::ShaderFilmGrainLumaBias,
    SettingsOptionId::ShaderRobotDeathStrength,
    SettingsOptionId::ShaderRobotStatic,
    SettingsOptionId::ShaderRobotTear,
    SettingsOptionId::ShaderRobotDesaturate,
    SettingsOptionId::ShaderRobotScanlines,
    SettingsOptionId::ShaderUnderwaterStrength,
    SettingsOptionId::ShaderUnderwaterDistortion,
    SettingsOptionId::ShaderDeepDreamStrength,
    SettingsOptionId::ShaderVignetteStrength,
    SettingsOptionId::MasterVolume,
    SettingsOptionId::MusicVolume,
    SettingsOptionId::SfxVolume,
    SettingsOptionId::Mute,
    SettingsOptionId::KeyboardPreset,
    SettingsOptionId::ControllerProfile,
    SettingsOptionId::LeftStickDeadzone,
    SettingsOptionId::RightStickDeadzone,
    SettingsOptionId::TriggerPress,
    SettingsOptionId::TriggerRelease,
    SettingsOptionId::DpadMenuNav,
    SettingsOptionId::InvertAimY,
    SettingsOptionId::DashInputMode,
    SettingsOptionId::TouchControls,
    SettingsOptionId::MenuTapMode,
    SettingsOptionId::ResetControlFiltering,
    SettingsOptionId::Difficulty,
    SettingsOptionId::Assist,
    SettingsOptionId::PlayerDamage,
    SettingsOptionId::DebugHud,
    SettingsOptionId::QuestHud,
    SettingsOptionId::TraceAutoDump,
    SettingsOptionId::PauseInputUnfocused,
    SettingsOptionId::PortalReverseFacing,
    SettingsOptionId::MovementFrameMode,
    SettingsOptionId::AimFrameMode,
    SettingsOptionId::Close,
];

/// Compile-time completeness guard for [`ALL_SETTINGS_OPTION_IDS`]: an EXHAUSTIVE
/// match over the enum that does nothing but force a build error the moment a new
/// `SettingsOptionId` variant is added without being listed above (and therefore
/// without being checked for surfacing). Never called — its only job is to fail
/// `cargo build` on an unlisted variant.
#[allow(dead_code)]
fn assert_all_settings_option_ids_listed(id: SettingsOptionId) {
    match id {
        SettingsOptionId::DisplayMode
        | SettingsOptionId::CameraZoom
        | SettingsOptionId::CameraAspect
        | SettingsOptionId::CameraFraming
        | SettingsOptionId::Flashes
        | SettingsOptionId::Colorblind
        | SettingsOptionId::ShowFps
        | SettingsOptionId::FramePacing
        | SettingsOptionId::VisualQuality
        | SettingsOptionId::ShaderStrength
        | SettingsOptionId::ShaderCrtStrength
        | SettingsOptionId::ShaderCrtScanlines
        | SettingsOptionId::ShaderCrtMask
        | SettingsOptionId::ShaderCrtCurvature
        | SettingsOptionId::ShaderCrtBloom
        | SettingsOptionId::ShaderCrtChroma
        | SettingsOptionId::ShaderFilmGrainStrength
        | SettingsOptionId::ShaderFilmGrainSize
        | SettingsOptionId::ShaderFilmGrainFps
        | SettingsOptionId::ShaderFilmGrainLumaBias
        | SettingsOptionId::ShaderRobotDeathStrength
        | SettingsOptionId::ShaderRobotStatic
        | SettingsOptionId::ShaderRobotTear
        | SettingsOptionId::ShaderRobotDesaturate
        | SettingsOptionId::ShaderRobotScanlines
        | SettingsOptionId::ShaderUnderwaterStrength
        | SettingsOptionId::ShaderUnderwaterDistortion
        | SettingsOptionId::ShaderDeepDreamStrength
        | SettingsOptionId::ShaderVignetteStrength
        | SettingsOptionId::MasterVolume
        | SettingsOptionId::MusicVolume
        | SettingsOptionId::SfxVolume
        | SettingsOptionId::Mute
        | SettingsOptionId::KeyboardPreset
        | SettingsOptionId::ControllerProfile
        | SettingsOptionId::LeftStickDeadzone
        | SettingsOptionId::RightStickDeadzone
        | SettingsOptionId::TriggerPress
        | SettingsOptionId::TriggerRelease
        | SettingsOptionId::DpadMenuNav
        | SettingsOptionId::InvertAimY
        | SettingsOptionId::DashInputMode
        | SettingsOptionId::TouchControls
        | SettingsOptionId::MenuTapMode
        | SettingsOptionId::ResetControlFiltering
        | SettingsOptionId::Difficulty
        | SettingsOptionId::Assist
        | SettingsOptionId::PlayerDamage
        | SettingsOptionId::DebugHud
        | SettingsOptionId::QuestHud
        | SettingsOptionId::TraceAutoDump
        | SettingsOptionId::PauseInputUnfocused
        | SettingsOptionId::PortalReverseFacing
        | SettingsOptionId::MovementFrameMode
        | SettingsOptionId::AimFrameMode
        | SettingsOptionId::Close => {}
    }
}

/// EVERY [`SystemMenuEntryId`] variant, listed exhaustively (compile-guarded by
/// [`assert_all_system_menu_entry_ids_listed`]).
const ALL_SYSTEM_MENU_ENTRY_IDS: &[SystemMenuEntryId] = &[
    SystemMenuEntryId::Radio,
    SystemMenuEntryId::Video,
    SystemMenuEntryId::Audio,
    SystemMenuEntryId::Controls,
    SystemMenuEntryId::Gameplay,
    SystemMenuEntryId::Language,
    SystemMenuEntryId::ResetAllSettings,
    SystemMenuEntryId::Quit,
    SystemMenuEntryId::Developer,
    SystemMenuEntryId::ResetSandbox,
];

#[allow(dead_code)]
fn assert_all_system_menu_entry_ids_listed(id: SystemMenuEntryId) {
    match id {
        SystemMenuEntryId::Radio
        | SystemMenuEntryId::Video
        | SystemMenuEntryId::Audio
        | SystemMenuEntryId::Controls
        | SystemMenuEntryId::Gameplay
        | SystemMenuEntryId::Language
        | SystemMenuEntryId::ResetAllSettings
        | SystemMenuEntryId::Quit
        | SystemMenuEntryId::Developer
        | SystemMenuEntryId::ResetSandbox => {}
    }
}

/// No-drift: every `SettingsOptionId` is surfaced by the settings IR
/// (`settings_menu_model`) — i.e. appears in some category's option rows — EXCEPT
/// the documented exclusions below. A newly-added variant fails to compile (the
/// `assert_all_settings_option_ids_listed` match) or fails this test (the
/// surfaced-set check) until it is wired into the IR.
#[test]
fn every_settings_option_is_surfaced_by_the_settings_ir() {
    let model = settings_menu_model(&UserSettings::default());
    let surfaced: std::collections::HashSet<SettingsOptionId> = model
        .categories
        .iter()
        .flat_map(|c| c.options.iter())
        .map(|o| o.id)
        .collect();

    for id in ALL_SETTINGS_OPTION_IDS {
        // DOCUMENTED EXCLUSION: `Close` is a renderer pseudo-option (close the
        // menu) provided by `close_menu_option()`, not a `UserSettings` field, so
        // it is intentionally NOT a category row in `settings_menu_model`. Assert
        // its exclusion explicitly so this stays a living inventory.
        if *id == SettingsOptionId::Close {
            assert!(
                !surfaced.contains(id),
                "Close is the close-menu pseudo-option, not a settings row"
            );
            continue;
        }
        assert!(
            surfaced.contains(id),
            "{id:?} is not surfaced by settings_menu_model — wire it into a category \
             (or document it as an explicit exclusion in this test)"
        );
    }
}

/// No-drift: every `SystemMenuEntryId` appears in the System page model
/// (`SystemMenuModel::build`), except the dev-build-gated entries when the
/// `dev_tools` feature is off. The exhaustive list + compile guard mean a new
/// entry variant must be surfaced (or explicitly excluded) before this passes.
#[test]
fn every_system_menu_entry_is_surfaced_by_the_system_model() {
    let model = SystemMenuModel::build(
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    let surfaced: std::collections::HashSet<SystemMenuEntryId> =
        model.entries.iter().map(|e| e.id).collect();

    let dev_build = cfg!(feature = "dev_tools");
    for id in ALL_SYSTEM_MENU_ENTRY_IDS {
        match id {
            // DOCUMENTED EXCLUSION: Developer + Reset Sandbox are dev-build only
            // (`SystemMenuModel::build` gates them on `DEV_BUILD`). In a dev build
            // they MUST be surfaced; in a non-dev build they MUST be absent.
            SystemMenuEntryId::Developer | SystemMenuEntryId::ResetSandbox => {
                assert_eq!(
                    surfaced.contains(id),
                    dev_build,
                    "{id:?} is dev-build gated: present iff the dev_tools feature is on"
                );
            }
            _ => assert!(
                surfaced.contains(id),
                "{id:?} is not surfaced by SystemMenuModel::build — add it to the model \
                 (or document it as an explicit exclusion in this test)"
            ),
        }
    }
}

/// No-drift: every `SettingsOptionId` is not just in the IR but actually
/// *curated* into the System menu (`SystemMenuModel::build` → `curated_options`),
/// which is what BOTH the grid and the kaleidoscope render. An option can sit in
/// `settings_menu_model` yet be invisible in-game if it is never added to a
/// System entry's curated list — exactly the gap that hid "Input Frame" /
/// "Portal Reverses Facing" after they were wired into the IR. This guards both
/// renderers at once because they share `SystemMenuModel`.
#[test]
fn every_settings_option_is_curated_into_the_system_model() {
    let model = SystemMenuModel::build(
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    let curated: std::collections::HashSet<SettingsOptionId> = model
        .entries
        .iter()
        .filter_map(|e| match &e.target {
            SystemMenuTarget::Settings(options) => Some(options),
            _ => None,
        })
        .flatten()
        .map(|o| o.id)
        .collect();

    for id in ALL_SETTINGS_OPTION_IDS {
        // DOCUMENTED EXCLUSION: `Close` is the close-menu pseudo-option, never a
        // curated settings row.
        if *id == SettingsOptionId::Close {
            assert!(!curated.contains(id), "Close is not a curated settings row");
            continue;
        }
        assert!(
            curated.contains(id),
            "{id:?} is in the settings IR but NOT curated into any System entry — add it to \
             `curated_options(..)` in menu/ir/system so it renders in BOTH the grid and the \
             kaleidoscope (or document it as an explicit exclusion here)"
        );
    }
}

// ---------------------------------------------------------------------------
// Part 3 — content parity (Grid actions == Cube actions, minus ChangePage edges)
// ---------------------------------------------------------------------------

/// The actions the CUBE renders for a page: every node's action (the cube draws
/// the page model verbatim, including the `ChangePage` edge page-turn controls).
fn cube_actions(
    page: &ambition::menu::MenuPageModel<MenuPage, MenuPageAction>,
) -> Vec<MenuPageAction> {
    page.nodes
        .iter()
        .filter_map(|n| n.action().copied())
        .collect()
}

/// The actions the GRID renders for a page: the cube's set with the `ChangePage`
/// edge page-turn controls stripped — EXACTLY the filter
/// `grid_menu_republish_view` applies (the tab bar replaces those edges). This
/// mirrors the backend's real `page.nodes.retain(...)`, so the test tracks the
/// one documented divergence rather than reimplementing layout.
fn grid_actions(
    page: &ambition::menu::MenuPageModel<MenuPage, MenuPageAction>,
) -> Vec<MenuPageAction> {
    cube_actions(page)
        .into_iter()
        .filter(|a| !matches!(a, MenuPageAction::ChangePage(_)))
        .collect()
}

/// Content parity: at a fixed state, for EACH page, the set of actionable controls
/// the Grid renders equals the cube's set MINUS the `ChangePage` edge controls.
/// This is the positive complement to `flat_renderer_skips_page_turn_edge_controls`
/// (which only checks the Grid drops the edges): here we assert the NON-edge
/// controls are otherwise identical, so neither backend can grow/drop a control the
/// other has.
#[test]
fn grid_and_cube_render_the_same_non_edge_controls_per_page() {
    // A representative fixed state: an item granted + equipped so Items has a live
    // Equip/Use action, on a stable System window.
    let mut owned = OwnedItems::default();
    let axe = Item::from_index(1).expect("an item at index 1");
    owned.grant(axe, 1);
    let pages = build_inventory_pages(
        &owned,
        Some(axe),
        MenuFocus::Item(1),
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
        0,
        None,
    );

    // `MenuPageAction` is `Eq` but not `Hash`/`Ord`, so compare action SETS as a
    // sorted, deduped list of their debug strings (order-independent membership).
    fn action_set(actions: &[MenuPageAction]) -> Vec<String> {
        let mut v: Vec<String> = actions.iter().map(|a| format!("{a:?}")).collect();
        v.sort();
        v.dedup();
        v
    }

    for page in &pages {
        let cube: Vec<MenuPageAction> = cube_actions(page);
        let grid: Vec<MenuPageAction> = grid_actions(page);

        // The cube's edge page-turn controls are exactly the difference.
        let edges: Vec<MenuPageAction> = cube
            .iter()
            .copied()
            .filter(|a| matches!(a, MenuPageAction::ChangePage(_)))
            .collect();
        assert!(
            !edges.is_empty(),
            "{:?}: the cube always bakes L/R ChangePage edge controls",
            page.id
        );

        // The NON-edge action sets must be identical between the two backends.
        let cube_non_edge: Vec<MenuPageAction> = cube
            .iter()
            .copied()
            .filter(|a| !matches!(a, MenuPageAction::ChangePage(_)))
            .collect();
        assert_eq!(
            action_set(&grid),
            action_set(&cube_non_edge),
            "{:?}: Grid + Cube must render the same non-edge controls (only the cube's \
             ChangePage edges differ)",
            page.id
        );

        // And the Grid renders NO ChangePage edges (the documented divergence).
        assert!(
            !grid
                .iter()
                .any(|a| matches!(a, MenuPageAction::ChangePage(_))),
            "{:?}: Grid strips the cube's page-turn edge controls",
            page.id
        );
    }
}

// ---------------------------------------------------------------------------
// Part 2 — dispatch parity (Grid release path == Cube release path)
// ---------------------------------------------------------------------------

#[cfg(feature = "input")]
mod dispatch_parity {
    use super::*;
    use bevy::prelude::*;

    use ambition::menu::ActiveMenuPages;

    use crate::menu::grid_backend::{
        grid_menu_pointer_press, grid_menu_pointer_release, GridMenuTabState, GridPointerPress,
    };
    use crate::menu::kaleidoscope_app::{
        kaleidoscope_pointer_press, kaleidoscope_pointer_release, KaleidoscopeCursor,
        KaleidoscopePointerPress, KaleidoscopeSystemNav,
    };
    use crate::menu::model::{MenuPage, MenuPageAction};
    use crate::menu::test_support::click_control as click;
    use ambition::actors::actor::BodyMana;
    use ambition::actors::actor::{PlayerEntity, PrimaryPlayer};
    use ambition::actors::avatar::PlayerHealRequested;
    use ambition::characters::brain::ActionSet;
    use ambition::input::MenuControlFrame;
    use ambition::inventory_ui::InventoryUiState;
    use ambition::menu::backend::InventoryUiBackend;
    use ambition::persistence::settings::UserSettings;
    use ambition::platformer::schedule::GameMode;
    use ambition::settings_menu::system::SystemMenuEntryId;

    /// Build a menu app for one backend, with every resource/observer the shared
    /// cursor/dispatch path touches. Mirrors the per-backend harnesses in
    /// `grid_backend.rs` / `lunex_kaleidoscope_app.rs` so BOTH exercise the same
    /// `dispatch_menu_action`.
    fn menu_app(backend: InventoryUiBackend) -> App {
        let mut app = App::new();
        app.init_resource::<crate::menu::quality_confirm::VisualQualityConfirmState>();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameMode>();
        app.init_resource::<InventoryUiBackend>();
        app.init_resource::<ActiveMenuPages<MenuPage, MenuPageAction>>();
        app.init_resource::<KaleidoscopeCursor>();
        app.init_resource::<KaleidoscopeSystemNav>();
        app.init_resource::<KaleidoscopePointerPress>();
        app.init_resource::<GridPointerPress>();
        app.init_resource::<GridMenuTabState>();
        app.init_resource::<OwnedItems>();
        app.init_resource::<ambition::dev_tools::dev_tools::DeveloperTools>();
        app.init_resource::<ambition::dev_tools::SandboxDevState>();
        app.init_resource::<ambition::actors::ldtk_world::LdtkHotReloadState>();
        app.init_resource::<ambition::actors::session::reset::SandboxResetRequested>();
        app.init_resource::<ambition::dev_tools::dev_tools::EditableMovementTuning>();
        app.init_resource::<UserSettings>();
        app.init_resource::<InventoryUiState>();
        app.init_resource::<ambition::menu::map::MapMenuState>();
        app.init_resource::<MenuControlFrame>();
        app.init_resource::<ambition::input::ActiveInputKind>();
        app.add_message::<PlayerHealRequested>();
        app.add_message::<ambition::sfx::OwnedSfxMessage>();
        app.add_message::<bevy::app::AppExit>();
        // BOTH backends' pointer observers are installed; each gates on the active
        // backend, so only the matching one dispatches.
        app.add_observer(kaleidoscope_pointer_press);
        app.add_observer(kaleidoscope_pointer_release);
        app.add_observer(grid_menu_pointer_press);
        app.add_observer(grid_menu_pointer_release);
        *app.world_mut().resource_mut::<InventoryUiBackend>() = backend;
        app.world_mut().resource_mut::<InventoryUiState>().visible = true;
        // The cube release routes a close through GameMode; start Paused like an
        // open menu so a close (not exercised here) would be well-defined.
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            ActionSet::default(),
            BodyMana::default(),
        ));
        app.update();
        app
    }

    /// Dispatch parity: an EQUIP action equips the same item through either
    /// backend's real release path (both call the one dispatcher).
    #[test]
    fn equip_dispatches_identically_on_both_backends() {
        let axe = Item::from_index(1).expect("equippable item at index 1");
        assert!(
            axe.held_item_id().is_some(),
            "index 1 is an equippable item"
        );

        let mut grid = menu_app(InventoryUiBackend::Grid);
        grid.world_mut().resource_mut::<OwnedItems>().grant(axe, 1);
        click(&mut grid, MenuPageAction::Equip(axe));
        let grid_equipped = grid.world().resource::<OwnedItems>().equipped();

        let mut cube = menu_app(InventoryUiBackend::LunexKaleidoscope);
        cube.world_mut().resource_mut::<OwnedItems>().grant(axe, 1);
        click(&mut cube, MenuPageAction::Equip(axe));
        let cube_equipped = cube.world().resource::<OwnedItems>().equipped();

        assert_eq!(grid_equipped, Some(axe), "grid release equipped the item");
        assert_eq!(
            grid_equipped, cube_equipped,
            "both backends equip the same item through dispatch_menu_action"
        );
    }

    /// Dispatch parity: a ChangePage action sets the same active page on both
    /// backends.
    ///
    /// Page-change is the ONE action whose effect is DELIBERATELY asymmetric, and
    /// this test pins that asymmetry so it can't drift silently:
    ///
    /// - the CUBE turns pages via the baked `ChangePage` edge controls, so its
    ///   release honors `ChangePage(page)` and leaves `pages.active = Some(page)`;
    /// - the GRID turns pages via its TAB BAR (it STRIPS the `ChangePage` edge
    ///   controls — see `flat_renderer_skips_page_turn_edge_controls` + the content-
    ///   parity test), and its release re-pins `pages.active` to the GRID's own
    ///   active tab AFTER dispatch (`grid_backend.rs`: `pages.active =
    ///   Some(tab_page(active_tab))`). So a synthetic `ChangePage` does NOT move the
    ///   grid's page — the tab is the grid's source of truth.
    ///
    /// This is intentional (two different page-change UIs over the same model), not
    /// a dispatcher drift: both still route through the one dispatcher; only the
    /// grid's deliberate post-dispatch re-pin differs. The shared part — the cube
    /// honoring `ChangePage` — is asserted; the grid's documented override is too.
    #[test]
    fn change_page_is_a_deliberate_per_backend_asymmetry() {
        // Cube: ChangePage moves the active page (the cube's real page-turn path).
        let mut cube = menu_app(InventoryUiBackend::LunexKaleidoscope);
        click(&mut cube, MenuPageAction::ChangePage(MenuPage::System));
        let cube_page = cube
            .world()
            .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active;
        assert_eq!(
            cube_page,
            Some(MenuPage::System),
            "the cube honors ChangePage (its page-turn affordance)"
        );

        // Grid: a synthetic ChangePage is overridden by the grid's tab source of
        // truth, so the active page stays on the grid's tab (Items by default). The
        // grid never emits ChangePage controls in practice (it strips them).
        let mut grid = menu_app(InventoryUiBackend::Grid);
        click(&mut grid, MenuPageAction::ChangePage(MenuPage::System));
        let grid_page = grid
            .world()
            .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active;
        assert_eq!(
            grid_page,
            Some(MenuPage::Items),
            "the grid re-pins pages.active to its own tab; ChangePage is a no-op for it \
             (page changes come from the tab bar, and ChangePage controls are stripped)"
        );
    }

    /// Dispatch parity: a settings TOGGLE mutates `UserSettings` identically on
    /// both backends (same IR `apply_settings_option` path).
    #[test]
    fn settings_toggle_dispatches_identically_on_both_backends() {
        let action = MenuPageAction::System(SettingsOptionId::QuestHud);

        let mut grid = menu_app(InventoryUiBackend::Grid);
        let grid_before = grid
            .world()
            .resource::<UserSettings>()
            .gameplay
            .quest_hud_visible;
        click(&mut grid, action);
        let grid_after = grid
            .world()
            .resource::<UserSettings>()
            .gameplay
            .quest_hud_visible;

        let mut cube = menu_app(InventoryUiBackend::LunexKaleidoscope);
        let cube_before = cube
            .world()
            .resource::<UserSettings>()
            .gameplay
            .quest_hud_visible;
        click(&mut cube, action);
        let cube_after = cube
            .world()
            .resource::<UserSettings>()
            .gameplay
            .quest_hud_visible;

        assert_ne!(grid_after, grid_before, "grid toggled the setting");
        assert_ne!(cube_after, cube_before, "cube toggled the setting");
        assert_eq!(
            grid_after, cube_after,
            "both backends land the same toggled value through apply_settings_option"
        );
    }

    /// Dispatch parity: drilling INTO a System entry sets the same `open_entry`
    /// drill-down state on both backends.
    #[test]
    fn open_system_entry_dispatches_identically_on_both_backends() {
        let action = MenuPageAction::OpenSystemEntry(SystemMenuEntryId::Audio);

        let mut grid = menu_app(InventoryUiBackend::Grid);
        click(&mut grid, action);
        let grid_open = grid.world().resource::<KaleidoscopeSystemNav>().open_entry;

        let mut cube = menu_app(InventoryUiBackend::LunexKaleidoscope);
        click(&mut cube, action);
        let cube_open = cube.world().resource::<KaleidoscopeSystemNav>().open_entry;

        assert_eq!(grid_open, Some(SystemMenuEntryId::Audio));
        assert_eq!(
            grid_open, cube_open,
            "both backends drill into the same System entry through dispatch_menu_action"
        );
    }
}
