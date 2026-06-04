//! The data seam between Ambition's live 24-item inventory and the reusable
//! `ambition_inventory_ui` 3D-cube OoT pause menu (#31).
//!
//! The game owns the item state (`crate::items`); this module builds the cube's
//! page MODELS from it via the lib's host-data seam (`ItemsOnlyPageSpec`, which is
//! deliberately renderer-agnostic — it can feed the Lunex cube, a Bevy-UI grid
//! fallback, or a test renderer). The cube RENDERER itself (promoting
//! `ambition_mock_demo`'s Lunex faces + ring/camera/rebuild systems into the
//! reusable lib as a generic plugin) is the next step — see
//! `dev/journals/oot-cube-integration-plan.md`.
//!
//! This gives the "wire us up to use it" part: our `Item::ALL` (already 24 in OoT
//! grid order) → the cube's items page, with owned/equipped/selected reflected and
//! a host-defined [`CubeAction`] emitted back to the game.

use ambition_inventory_ui::{
    InventoryItemNode, InventorySlotId, ItemsOnlyPageSpec, MenuColor, MenuControlKind,
    MenuPageModel, MenuRect, MenuTextAlign,
};

use crate::items::{Item, OwnedItems};

/// Edge page-turn buttons flank the page. Rects match the lib's decorative nav
/// arrows (`spawn_nav_arrows` in `cube.rs`) so they sit where the L/R affordance
/// has always been. The game draws these as REAL controls and turns the lib's
/// decorative arrows off (`draw_nav_arrows = false`) so they aren't double-drawn.
const EDGE_LEFT_RECT: MenuRect = MenuRect { x: 1.8, y: 43.5, w: 7.5, h: 13.0 };
const EDGE_RIGHT_RECT: MenuRect = MenuRect { x: 90.7, y: 43.5, w: 7.5, h: 13.0 };

impl CubePage {
    /// The neighbouring page when turning the ring left/right (wraps), matching
    /// [`CubePage::ALL`] order.
    pub fn neighbor(self, dir: isize) -> CubePage {
        let all = CubePage::ALL;
        let cur = all.iter().position(|p| *p == self).unwrap_or(0);
        let next = (cur as isize + dir).rem_euclid(all.len() as isize) as usize;
        all[next]
    }
}

/// Append the Left/Right edge page-turn buttons to a page model. Emitted as real
/// `Action` controls (`CubeAction::ChangePage`) so Bevy picking + directional focus
/// can dispatch them; mirrors the demo's `add_edge_buttons` intent.
fn add_edge_buttons(model: &mut MenuPageModel<CubePage, CubeAction>, page: CubePage) {
    model.control(
        EDGE_LEFT_RECT,
        MenuControlKind::Action,
        "<",
        None,
        false,
        false,
        Some(CubeAction::ChangePage(page.neighbor(-1))),
    );
    model.control(
        EDGE_RIGHT_RECT,
        MenuControlKind::Action,
        ">",
        None,
        false,
        false,
        Some(CubeAction::ChangePage(page.neighbor(1))),
    );
}

/// The cube faces (pages). `Items` is wired live from our inventory; the rest
/// mirror OoT's subscreen tabs as host-data-driven placeholders for now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CubePage {
    Items,
    Map,
    Quest,
    System,
}

impl CubePage {
    pub const ALL: [CubePage; 4] = [
        CubePage::Items,
        CubePage::Map,
        CubePage::Quest,
        CubePage::System,
    ];
}

/// Actions the cube emits back to the game (the host consumes these — the cube
/// never mutates item state itself, matching the existing `oot_menu` seam).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CubeAction {
    Equip(Item),
    Use(Item),
    ChangePage(CubePage),
}

/// The items-face spec built from our live inventory. Kept separate from
/// [`build_items_page`] so it's unit-testable without the renderer.
pub fn items_spec(
    owned: &OwnedItems,
    equipped: Option<Item>,
    selected: Option<Item>,
) -> ItemsOnlyPageSpec<CubePage, CubeAction> {
    let mut spec = ItemsOnlyPageSpec::new(CubePage::Items, "ITEMS").with_grid(
        crate::items::ITEM_GRID_ROWS,
        crate::items::ITEM_GRID_COLS,
    );
    spec.selected_slot = selected.map(|i| InventorySlotId(i.index()));
    spec.cells = Item::ALL
        .into_iter()
        .map(|item| {
            let owns = owned.has(item);
            let mut node = if owns {
                InventoryItemNode::new(item.index(), item.display_name())
            } else {
                InventoryItemNode::unowned(item.index(), item.display_name())
            };
            node = node
                .detail(item.description())
                .equipped(equipped == Some(item));
            if owns {
                // Held-item weapons/abilities equip; everything else "uses".
                let (action, label) = if item.held_item_id().is_some() {
                    (CubeAction::Equip(item), "Equip")
                } else {
                    (CubeAction::Use(item), "Use")
                };
                node = node.action(action).action_label(label);
            }
            node
        })
        .collect();
    spec
}

/// The Items cube face, built from our live inventory.
pub fn build_items_page(
    owned: &OwnedItems,
    equipped: Option<Item>,
    selected: Option<Item>,
) -> MenuPageModel<CubePage, CubeAction> {
    let mut model = items_spec(owned, equipped, selected).into_page_model();
    add_edge_buttons(&mut model, CubePage::Items);
    model
}

/// Build every cube face from our inventory (Items live, the rest placeholders
/// until their host data is wired). The renderer consumes these via
/// `ActiveMenuPages<CubePage, CubeAction>`.
pub fn build_inventory_pages(
    owned: &OwnedItems,
    equipped: Option<Item>,
    selected: Option<Item>,
) -> Vec<MenuPageModel<CubePage, CubeAction>> {
    vec![
        build_items_page(owned, equipped, selected),
        placeholder_page(
            CubePage::Map,
            "MAP",
            "Area map: discovered rooms, anchors, portal markers (host data TODO).",
        ),
        placeholder_page(
            CubePage::Quest,
            "QUEST",
            "Quest status + key items from save data (host data TODO).",
        ),
        placeholder_page(CubePage::System, "SYSTEM", "Save / options (TODO)."),
    ]
}

fn placeholder_page(page: CubePage, title: &str, body: &str) -> MenuPageModel<CubePage, CubeAction> {
    let mut model = MenuPageModel::new(page, title, MenuColor::rgba(0.03, 0.04, 0.10, 0.96));
    model.text(
        50.0,
        38.0,
        4.0,
        title,
        MenuTextAlign::Center,
        MenuColor::rgba(1.0, 0.84, 0.38, 1.0),
    );
    model.text(
        50.0,
        54.0,
        1.7,
        body,
        MenuTextAlign::Center,
        MenuColor::rgba(0.85, 0.92, 1.0, 0.9),
    );
    add_edge_buttons(&mut model, page);
    model
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_face_wires_all_24_slots_from_our_catalog() {
        let owned = OwnedItems::default();
        let spec = items_spec(&owned, None, None);
        assert_eq!(
            spec.cells.len(),
            crate::items::ITEM_COUNT,
            "the cube's items face has one cell per inventory slot (24)"
        );
        // Slots are in grid order and labelled from our catalog.
        for (idx, cell) in spec.cells.iter().enumerate() {
            assert_eq!(cell.slot.0, idx);
            assert_eq!(cell.label, Item::ALL[idx].display_name());
        }
    }

    #[test]
    fn owned_and_equipped_flags_reflect_inventory_state() {
        let mut owned = OwnedItems::default();
        owned.grant(Item::Blink, 1);
        let spec = items_spec(&owned, Some(Item::Blink), None);
        let blink = &spec.cells[Item::Blink.index()];
        assert!(blink.owned, "granted item reads owned");
        assert!(blink.equipped, "equipped item reads equipped");
        assert!(blink.action.is_some(), "owned item has an action");
        // An un-granted item is unowned + actionless.
        let unowned = spec.cells.iter().find(|c| !c.owned).expect("some unowned");
        assert!(unowned.action.is_none());
    }
}
