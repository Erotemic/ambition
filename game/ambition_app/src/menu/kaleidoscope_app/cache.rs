//! Cube page publication from each menu's cached System model.

use super::*;
/// Republish the cube's faces from our live inventory + the focus cursor (the
/// host-owned data seam — the cube renderer treats `ActiveMenuPages` as read-only).
///
/// Runs after [`kaleidoscope_focus_nav`] in the chain so this frame's cursor move is
/// reflected in the rebuilt page (highlight + detail panel). To avoid an infinite
/// rebuild loop (writing `pages.pages` marks the component changed), it republishes
/// only when something it depends on actually changed: the inventory, the focus
/// cursor, the active page, the just-opened edge, or the very first publish.
pub(crate) fn republish_kaleidoscope_pages(
    settings: Res<UserSettings>,
    owned: Option<Res<OwnedItems>>,
    // The menu owner supplies the hand and worn abilities shown as equipped.
    body_source: MenuBodySource,
    mut menus: Query<(
        &ambition_platformer2d::inventory_ui::InventoryMenu,
        &mut ActiveMenuPages<MenuPage, MenuPageAction>,
        &KaleidoscopeCursor,
        &KaleidoscopeSystemNav,
        &CachedSystemMenu,
        &KaleidoscopeScroll,
        &mut MenuPublication,
        &ambition_platformer2d::inventory_ui::InventoryUiState,
    )>,
    map_page: crate::menu::map_page::MapPageSource,
) {
    let Some(owned) = owned else {
        return;
    };
    for (menu_owner, mut pages, cursor, system_nav, cache, scroll, mut publication, ui_state) in
        &mut menus
    {
        let MenuPublication { was_open, last } = &mut *publication;
        let body = body_source.read(menu_owner.owner);
        let inventory = ambition_platformer2d::items::Inventory::new(
            &owned,
            body.and_then(|body| body.in_hand),
        );
        let open = ui_state.visible;
        let just_opened = open && !*was_open;
        *was_open = open;

        // A cursor-only move (mouse OR keyboard) no longer rebuilds the face — the highlight
        // (`kaleidoscope_sync_focus_visuals`) and the detail text (`kaleidoscope_sync_detail_text`)
        // update IN PLACE. Without this, a `Pointer<Move>` between a press and release despawned
        // the hovered control and Bevy dropped the `Pointer<Click>`. Only a focus change that
        // SHIFTS the System scroll window changes the rendered rows, so only that needs a rebuild;
        // the drill-down state is also keyed so drilling in/out republishes the new rows.
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
        let Some(model) = cache.model.as_ref() else {
            continue;
        };
        let key = RebuildKey {
            system_revision: cache.model_builds,
            window_start,
            active: pages.active,
            open_entry: system_nav.open_entry,
            owned_counts: Item::ALL.map(|item| inventory.count(item)),
            equipped: body.map(|body| body.equipped).unwrap_or_default(),
            settings: settings.clone(),
            radio: cache.radio.clone(),
            dev: cache.dev.clone(),
            quality: cache.quality,
            // Only while the menu is open: the map is not built behind gameplay.
            map: if open {
                map_page.view(body.map(|body| body.entity))
            } else {
                None
            },
        };
        // Republish on first publish, menu-open (textures that loaded after the initial
        // build get picked up), or an actual VALUE change in anything rendered by the
        // cube: inventory counts/equipment, user settings, page/drill/scroll state, radio
        // and developer snapshots, or pending quality confirmation.
        //
        // PERF: never use Bevy change ticks as the rebuild oracle here. Systems that take
        // `ResMut<OwnedItems>` / `ResMut<UserSettings>` can dirty those ticks while
        // handling navigation even when their values remain identical. OR-ing
        // `is_changed()` into this gate recreated the historical rebuild-every-frame
        // cliff: every cube face was despawned and respawned on every visible frame. The
        // complete value key below makes those false-positive ticks harmless while still
        // rebuilding immediately for real inventory or settings changes.
        let pages_empty = pages.pages.is_empty();
        let key_changed = last.as_ref() != Some(&key);
        // A content reload that publishes a new item catalog changes the names
        // and verbs on the faces; the value key cannot see that, nor a game's new
        // Map page presentation (set at startup, so this does not tick per frame).
        let dirty = pages_empty
            || just_opened
            || key_changed
            || body_source.items_changed()
            || map_page.presentation_changed();
        if !dirty {
            continue;
        }
        let changed_fields = last
            .as_ref()
            .map(|previous| previous.changed_fields(&key))
            .unwrap_or_else(|| vec!["initial"]);
        debug!(
            target: "ambition_platformer2d::kaleidoscope_rebuild",
            "republishing cube pages pages_empty={} just_opened={} changed_fields={:?}",
            pages_empty,
            just_opened,
            changed_fields,
        );

        let active = pages.active.unwrap_or(MenuPage::Items);
        // `replace_pages` bumps `version` — the renderer's ONLY content-invalidation
        // signal. `rebuild_cube_faces` deliberately ignores Bevy change ticks (see its
        // gate) and rebuilds solely on a `version` or `active` change, so a republish
        // that assigns `pages.pages` directly without the bump renders NOTHING: the
        // drill-down/scroll/inventory change this republish just baked never reaches
        // the cube faces.
        let built = build_inventory_pages_with_quality_prompt(
            &inventory,
            body_source.items(),
            key.equipped,
            cursor.focus,
            model,
            &key.radio,
            window_start,
            system_nav.open_entry,
            key.quality,
            map_page.page(key.map.as_ref()),
        );
        pages.replace_pages(built, active);
        *last = Some(key);
    }
}

/// The value-equality key that gates a cube republish. Two frames with an equal
/// key render identical pages, so the rebuild is skipped. Compared by VALUE (not
/// Bevy change-ticks) so a per-frame resource mutation that does not alter the
/// rendered content (e.g. the music director's per-frame audio-gain updates) never
/// forces a rebuild — see `republish_kaleidoscope_pages`.
#[derive(PartialEq)]
pub(crate) struct RebuildKey {
    system_revision: u64,
    window_start: usize,
    active: Option<MenuPage>,
    open_entry: Option<SystemMenuEntryId>,
    /// Complete inventory value snapshot. `OwnedItems` intentionally does not expose
    /// its storage, so derive the stable catalog-order counts through its public API.
    owned_counts: [u32; ambition_platformer2d::items::ITEM_COUNT],
    equipped: ambition_platformer2d::items::EquippedItems,
    /// Complete persisted settings value.
    settings: UserSettings,
    radio: RadioSnapshot,
    dev: DevSnapshot,
    quality: Option<VisualQualityProfile>,
    /// The Map page's region, by value (`map_page`).
    map: Option<ambition_platformer2d::menu::map::view::MapView>,
}

impl RebuildKey {
    fn changed_fields(&self, next: &Self) -> Vec<&'static str> {
        let mut changed = Vec::new();
        if self.system_revision != next.system_revision {
            changed.push("system_revision");
        }
        if self.window_start != next.window_start {
            changed.push("window_start");
        }
        if self.active != next.active {
            changed.push("active_page");
        }
        if self.open_entry != next.open_entry {
            changed.push("system_open_entry");
        }
        if self.owned_counts != next.owned_counts {
            changed.push("owned_counts");
        }
        if self.equipped != next.equipped {
            changed.push("equipped");
        }
        if self.settings.video != next.settings.video {
            changed.push("settings.video");
        }
        if self.settings.audio != next.settings.audio {
            changed.push("settings.audio");
        }
        if self.settings.controls != next.settings.controls {
            changed.push("settings.controls");
        }
        if self.settings.gameplay != next.settings.gameplay {
            changed.push("settings.gameplay");
        }
        if self.radio != next.radio {
            changed.push("radio");
        }
        if self.dev != next.dev {
            changed.push("developer_snapshot");
        }
        if self.quality != next.quality {
            changed.push("quality_confirmation");
        }
        if self.map != next.map {
            changed.push("map");
        }
        changed
    }
}

/// The last content published for one menu and its open edge.
#[derive(Component, Default)]
pub(crate) struct MenuPublication {
    was_open: bool,
    last: Option<RebuildKey>,
}
