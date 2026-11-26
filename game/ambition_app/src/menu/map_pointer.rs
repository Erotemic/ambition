//! Pointer input for the shared map viewport.

use bevy::picking::{
    events::{self, Pointer, Press},
    pointer::{PointerAction, PointerButton, PointerId, PointerInput},
    PickingSystems,
};
use bevy::prelude::*;

use ambition_platformer2d::inventory_ui::{InventoryMenu, InventoryUiState};
use ambition_platformer2d::menu::{
    backend::InventoryUiBackend,
    map::browser::{MapAction, MapBrowser},
    ActiveMenuPages, AmbitionMenuControl, MenuInteraction, MenuSurface,
};

use super::{
    effects::MenuBodySource,
    map_page::{MapPageSource, MAP_FRAME},
    model::{MenuPage, MenuPageAction},
    regions::{MenuRegion, MenuWheelInput},
};

#[derive(Component, Default)]
pub(crate) struct MapDrag(Option<Drag>);

struct Drag {
    pointer: PointerId,
    origin: Vec2,
    last: Vec2,
    moved: bool,
}

pub(crate) fn install(app: &mut App) {
    app.add_message::<PointerInput>()
        .add_observer(press)
        .add_systems(
            PreUpdate,
            cancel_taps_on_motion
                .after(PickingSystems::ProcessInput)
                .before(events::pointer_events),
        )
        .add_systems(
            Update,
            update
                .after(ambition_platformer2d::input::MenuNavConsume)
                .before(super::MenuActionsApplied),
        );
}

fn press(
    event: On<Pointer<Press>>,
    controls: Query<(&AmbitionMenuControl<MenuPageAction>, &MenuSurface)>,
    mut menus: Query<(
        &InventoryUiState,
        &ActiveMenuPages<MenuPage, MenuPageAction>,
        &mut MapDrag,
    )>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Ok((control, surface)) = controls.get(event.entity) else {
        return;
    };
    if !matches!(
        control.action,
        Some(MenuPageAction::Map(MapAction::Open(_) | MapAction::Canvas))
    ) {
        return;
    }
    let Ok((state, pages, mut gesture)) = menus.get_mut(surface.0) else {
        return;
    };
    if state.visible && pages.active == Some(MenuPage::Map) && gesture.0.is_none() {
        let at = event.pointer_location.position;
        gesture.0 = Some(Drag {
            pointer: event.pointer_id,
            origin: at,
            last: at,
            moved: false,
        });
    }
}

fn cancel_taps_on_motion(
    mut inputs: MessageReader<PointerInput>,
    mut menus: Query<(&MapDrag, &mut MenuInteraction<MenuPageAction>)>,
) {
    for event in inputs.read() {
        if matches!(
            event.action,
            PointerAction::Move { .. } | PointerAction::Release(PointerButton::Primary)
        ) {
            for (gesture, mut interaction) in &mut menus {
                if gesture
                    .0
                    .as_ref()
                    .is_some_and(|drag| drag.pointer == event.pointer_id)
                {
                    interaction.move_pointer(event.pointer_id, event.location.position);
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn update(
    mut pointer: MessageReader<PointerInput>,
    mut wheel: MenuWheelInput,
    source: MapPageSource,
    backend: Res<InventoryUiBackend>,
    bodies: MenuBodySource,
    canvas_nodes: Query<(
        &MenuSurface,
        &AmbitionMenuControl<MenuPageAction>,
        &ComputedNode,
        &ComputedUiTargetCamera,
    )>,
    cameras: Query<&Camera>,
    mut menus: Query<(
        Entity,
        &InventoryMenu,
        &InventoryUiState,
        &ActiveMenuPages<MenuPage, MenuPageAction>,
        &MenuRegion,
        &mut MapBrowser,
        &mut MapDrag,
    )>,
) {
    wheel.read();
    let inputs: Vec<_> = pointer.read().collect();
    for (menu, owner, state, pages, region, mut browser, mut gesture) in &mut menus {
        if !state.visible || pages.active != Some(MenuPage::Map) {
            gesture.0 = None;
            continue;
        }
        let scroll = wheel.lines_for(menu);
        let mut delta = Vec2::ZERO;
        for event in &inputs {
            let Some(drag) = gesture
                .0
                .as_mut()
                .filter(|drag| drag.pointer == event.pointer_id)
            else {
                continue;
            };
            match event.action {
                PointerAction::Move { .. } => {
                    let at = event.location.position;
                    if drag.moved
                        || at.distance(drag.origin) > ambition_platformer2d::ui_nav::ROW_TAP_SLOP_PX
                    {
                        drag.moved = true;
                        delta += drag.last - at;
                        drag.last = at;
                    }
                }
                PointerAction::Release(PointerButton::Primary) | PointerAction::Cancel => {
                    gesture.0 = None;
                }
                _ => {}
            }
        }
        let aspect = super::map_page::frame_aspect(backend.effective(), Some(region));
        if let Some(rect) = region
            .0
            .filter(|_| delta != Vec2::ZERO || scroll != Vec2::ZERO)
        {
            let frame_size = canvas_nodes
                .iter()
                .find_map(|(surface, control, node, target)| {
                    if surface.0 != menu
                        || control.action != Some(MenuPageAction::Map(MapAction::Canvas))
                        || node.is_empty()
                    {
                        return None;
                    }
                    let camera = cameras.get(target.get()?).ok()?;
                    Some(node.size() / camera.target_scaling_factor().unwrap_or(1.0))
                })
                .unwrap_or_else(|| rect.size() * Vec2::new(MAP_FRAME.w, MAP_FRAME.h) * 0.01);
            let pan = (delta - Vec2::new(scroll.x * 40.0, 0.0)) / frame_size.max(Vec2::ONE);
            let body = bodies.read(owner.owner).map(|body| body.entity);
            source.adjust_viewport(&mut browser, body, pan, scroll.y.clamp(-8.0, 8.0), aspect);
        }
    }
}
