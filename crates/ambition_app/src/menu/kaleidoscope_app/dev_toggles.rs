//! Developer-page toggle plumbing for the cube menu's System tab: the read/
//! write context structs and the snapshot/apply functions shared with the
//! pause-menu Developer rows.
//!
//! Split out of the (post-test-extraction) `kaleidoscope_app.rs` (2026-06-15).

use super::*;

/// The set of resources the Developer screen reads/writes. The dev-toggle path
/// spans THREE resources: most toggles live on `DeveloperTools`, but the F1/F2
/// global flags live on [`SandboxDevState`] and the F12 LDtk hot-reload toggle
/// lives on [`LdtkHotReloadState`] — mirroring the pause-menu Developer page,
/// which aggregates the same three. Bundled so [`dev_snapshot`] /
/// [`apply_dev_toggle`] stay single-source for every Developer row.
pub(crate) struct DevToggleRead<'a> {
    pub(crate) dev: &'a ambition_gameplay_core::dev::dev_tools::DeveloperTools,
    pub(crate) dev_state: &'a ambition_gameplay_core::SandboxDevState,
    pub(crate) ldtk_reload: &'a ambition_gameplay_core::ldtk_world::LdtkHotReloadState,
    // The Menu Backend row mirrors the `\` hotkey; its value label is the active
    // frontend (Grid / Cube), read from `InventoryUiBackend`.
    pub(crate) backend: InventoryUiBackend,
    // The Portal FX row's live effect (view cones / masks / off). Option so
    // fixtures without the resource still render the row (as "n/a").
    #[cfg(feature = "portal_render")]
    pub(crate) portal_effect: Option<&'a ambition_gameplay_core::portal::PortalEffectSelection>,
    // The Gravity row's ambient direction (down/left/up/right). Option so
    // fixtures without the resource still render the row (as "n/a").
    pub(crate) base_gravity: Option<&'a ambition_gameplay_core::physics::BaseGravity>,
}

pub(crate) struct DevToggleWrite<'a> {
    pub(crate) dev: &'a mut ambition_gameplay_core::dev::dev_tools::DeveloperTools,
    pub(crate) dev_state: &'a mut ambition_gameplay_core::SandboxDevState,
    pub(crate) ldtk_reload: &'a mut ambition_gameplay_core::ldtk_world::LdtkHotReloadState,
    pub(crate) backend: &'a mut InventoryUiBackend,
    #[cfg(feature = "portal_render")]
    pub(crate) portal_effect: Option<&'a mut ambition_gameplay_core::portal::PortalEffectSelection>,
    pub(crate) base_gravity: Option<&'a mut ambition_gameplay_core::physics::BaseGravity>,
}

/// Read every developer toggle/cycle into a [`DevSnapshot`] for the SYSTEM IR. The
/// single place mapping the three dev resources onto [`DevToggleId`]s for display.
pub(crate) fn dev_snapshot(ctx: DevToggleRead<'_>) -> DevSnapshot {
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
    // Gravity (BaseGravity) — the `\`-hotkey row, a cycle whose value label is the
    // active ambient direction (Down / Left / Up / Right).
    values.push(DevSnapshot::cycle(
        D::Gravity,
        ctx.base_gravity.map_or("n/a", |g| g.direction_label()),
    ));
    DevSnapshot { values }
}

/// Apply a single developer toggle/cycle to `DeveloperTools`. `dir` selects the
/// direction for cycles (`<0` prev, otherwise next); toggles flip regardless. This
/// is the single place that mutates `DeveloperTools` from the cube, so the dev
/// menu and the inspector stay in lock-step on field semantics.
pub(crate) fn apply_dev_toggle(ctx: DevToggleWrite<'_>, id: DevToggleId, dir: i32) {
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
        // Cycle ambient gravity (down → left → up → right) via the shared
        // `BaseGravity::cycle` — the in-menu equivalent of the `\` hotkey, so
        // sideways/inverted gravity is reachable on mobile. Direction is moot
        // (always steps forward); absent under fixtures: no-op.
        D::Gravity => {
            let _ = dir;
            if let Some(base) = ctx.base_gravity {
                base.cycle();
            }
        }
    }
}
