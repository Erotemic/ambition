//! System-tab page caching: the cached snapshot + the cache/republish systems
//! and the change-detection RebuildKey that suppresses no-op republishes.
//!
//! Split out of the kaleidoscope menu host (2026-06-15).

use super::*;

/// Per-frame cache of the System face's built model + windowed rows + the radio/dev
/// snapshots. `SystemMenuModel::build` (the full settings IR plus many per-row string
/// allocations) and the radio/dev snapshots were each rebuilt THREE times per frame
/// on the System face — once each in [`republish_kaleidoscope_pages`],
/// [`kaleidoscope_sync_focus_visuals`], and [`kaleidoscope_sync_detail_text`]. They
/// run back-to-back in one chain with identical inputs, so [`cache_system_menu`]
/// builds them once and the three consumers read this.
#[derive(Resource, Default)]
pub(crate) struct CachedSystemMenu {
    /// The System model for the live face, or `None` when another face is active.
    pub(crate) model: Option<SystemMenuModel>,
    /// `system_rows(model, open_entry)` for the live drill state (empty off-System).
    pub(crate) rows: Vec<SystemRow>,
    /// Snapshots feeding the model AND the republish `RebuildKey`, which carries them
    /// on every face — so these are refreshed each frame regardless of active page.
    pub(crate) radio: RadioSnapshot,
    pub(crate) dev: DevSnapshot,
    /// Pending visual-quality profile, if the Video screen is asking for confirmation.
    pub(crate) quality: Option<VisualQualityProfile>,
}

/// Build the System model + radio/dev snapshots ONCE per frame (front of the visible
/// chain) into [`CachedSystemMenu`]. The model + rows are built only while the System
/// face is active; the snapshots are always refreshed (the republish key carries them
/// on every face). See [`CachedSystemMenu`] for why.
pub(crate) fn cache_system_menu(
    pages: Res<ActiveMenuPages<MenuPage, MenuPageAction>>,
    settings: Res<UserSettings>,
    system_nav: Res<KaleidoscopeSystemNav>,
    snapshot: SystemMenuSnapshotParams,
    quality_confirm: Res<VisualQualityConfirmState>,
    mut cache: ResMut<CachedSystemMenu>,
) {
    let radio = snapshot.radio_snapshot();
    let dev = snapshot.dev_snapshot();
    let quality = quality_confirm.pending();
    if pages.active == Some(MenuPage::System) {
        let model = crate::menu::model::system_menu_model_with_pending_quality(
            &settings, &radio, &dev, quality,
        );
        cache.rows = system_rows_with_quality_prompt(&model, system_nav.open_entry, quality);
        cache.model = Some(model);
    } else {
        cache.model = None;
        cache.rows.clear();
    }
    cache.radio = radio;
    cache.dev = dev;
    cache.quality = quality;
}

/// Republish the cube's faces from our live inventory + the focus cursor (the
/// host-owned data seam — the cube renderer treats `ActiveMenuPages` as read-only).
///
/// Runs after [`kaleidoscope_focus_nav`] in the chain so this frame's cursor move is
/// reflected in the rebuilt page (highlight + detail panel). To avoid an infinite
/// rebuild loop (writing `pages.pages` marks the resource changed), it republishes
/// only when something it depends on actually changed: the inventory, the focus
/// cursor, the active page, the just-opened edge, or the very first publish.
pub(crate) fn republish_kaleidoscope_pages(
    ui_state: Option<Res<ambition_inventory_ui::InventoryUiState>>,
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
    // The System model + radio/dev snapshots, built ONCE this frame by
    // `cache_system_menu` (runs just before us in the chain with identical inputs),
    // so the dirty-key + rebuild reuse them instead of rebuilding a third time.
    cache: Res<CachedSystemMenu>,
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

    // Deferred Bug 2 fix: the page key keys off the System scroll-window START, NOT
    // the raw `cursor.focus`. A cursor-only move (mouse OR keyboard) no longer
    // rebuilds the face — the highlight (`kaleidoscope_sync_focus_visuals`) and the
    // detail text (`kaleidoscope_sync_detail_text`) update IN PLACE. Without this, a
    // `Pointer<Move>` between a press and release despawned the hovered control and
    // Bevy dropped the `Pointer<Click>`. Only a focus change that SHIFTS the System
    // scroll window changes the rendered rows, so only that needs a rebuild; the
    // drill-down state is also keyed so drilling in/out republishes the new rows.
    let window_start = if pages.active == Some(MenuPage::System) {
        // The EFFECTIVE window start: an explicit drag/wheel override wins (Features
        // C/D), otherwise it follows the cursor. Keying the rebuild off this means a
        // wheel/drag scroll rebuilds the windowed rows, while a cursor-only move
        // inside the window still does not (preserving A's click fix). Rows come from
        // the shared per-frame cache.
        system_effective_window_start(&cache.rows, cursor.focus, scroll.system_window_start)
    } else {
        0
    };
    let key = RebuildKey {
        window_start,
        active: pages.active,
        open_entry: system_nav.open_entry,
        radio: cache.radio.clone(),
        dev: cache.dev.clone(),
        quality: cache.quality,
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
    pages.pages = build_inventory_pages_with_quality_prompt(
        &owned,
        owned.equipped(),
        cursor.focus,
        &settings,
        &key.radio,
        &key.dev,
        window_start,
        system_nav.open_entry,
        key.quality,
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
pub(crate) struct RebuildKey {
    window_start: usize,
    active: Option<MenuPage>,
    open_entry: Option<SystemMenuEntryId>,
    radio: RadioSnapshot,
    dev: DevSnapshot,
    quality: Option<VisualQualityProfile>,
}
