//! Flat controls use picking events on both window and image render targets.

use super::*;
use bevy::picking::events::{Press, Release};
use bevy::picking::pointer::{PointerAction, PointerButton, PointerId, PointerInput, PointerPress};

fn tap_bounds(
    location: &bevy::picking::pointer::Location,
    node: &ComputedNode,
    transform: &UiGlobalTransform,
    camera: &Camera,
) -> Option<crate::PointerTapBounds> {
    if node.is_empty() {
        return None;
    }
    let scale = camera.target_scaling_factor().unwrap_or(1.0);
    let origin = camera
        .physical_viewport_rect()
        .map_or(Vec2::ZERO, |rect| rect.min.as_vec2());
    Some(crate::PointerTapBounds {
        rect: Rect::from_center_size(
            (transform.translation + origin) / scale,
            node.size() / scale,
        ),
        target: location.target.clone(),
    })
}

fn activate<Action>(
    menu: Entity,
    state: &mut crate::MenuInteraction<Action>,
    pointer: PointerId,
    risk: Option<&crate::MenuDestructiveActions<Action>>,
    settings: Option<&ambition_persistence::settings::UserSettings>,
    activated: &mut MessageWriter<crate::MenuActionActivated<Action>>,
) where
    Action: Clone + Send + Sync + 'static,
{
    let Some(focus) = state.arm.armed().copied() else {
        return;
    };
    let Some(action) = state.release_pointer(pointer) else {
        return;
    };
    let destructive = risk.is_some_and(|risk| (risk.is_destructive)(&action));
    let tap_mode = settings
        .map(|settings| settings.controls.menu_tap_mode)
        .unwrap_or_default();
    if tap_mode.resolve_press(focus, &focus, destructive, &mut state.confirm_armed)
        == ambition_input::settings::MenuPointerPress::Confirm
    {
        activated.write(crate::MenuActionActivated { menu, action });
    }
}

fn same_target<Action: PartialEq>(
    state: &crate::MenuInteraction<Action>,
    control: &AmbitionMenuControl<Action>,
) -> bool {
    state.arm.armed() == Some(&control.focus) && state.armed.as_ref() == control.action.as_ref()
}

pub(super) fn install_messages(app: &mut App) {
    app.add_message::<PointerInput>();
}

pub(super) fn press_actions<Action: Clone + Send + Sync + 'static>(
    press: On<Pointer<Press>>,
    controls: Query<(&AmbitionMenuControl<Action>, &crate::MenuSurface), With<Button>>,
    mut menus: Query<&mut crate::MenuInteraction<Action>>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform, &ComputedUiTargetCamera)>,
    cameras: Query<&Camera>,
) {
    if press.button != PointerButton::Primary {
        return;
    }
    let Ok((control, surface)) = controls.get(press.entity) else {
        return;
    };
    let Some(action) = control.action.clone() else {
        return;
    };
    if let Ok(mut state) = menus.get_mut(surface.0) {
        state.press_pointer(
            press.pointer_id,
            control.focus,
            action,
            press.pointer_location.position,
        );
        state.bounds = nodes
            .get(press.entity)
            .ok()
            .and_then(|(node, transform, target)| {
                tap_bounds(
                    &press.pointer_location,
                    node,
                    transform,
                    cameras.get(target.get()?).ok()?,
                )
            });
    }
}

pub(super) fn release_actions<Action: Clone + PartialEq + Send + Sync + 'static>(
    release: On<Pointer<Release>>,
    controls: Query<(&AmbitionMenuControl<Action>, &crate::MenuSurface), With<Button>>,
    mut menus: Query<&mut crate::MenuInteraction<Action>>,
    mut activated: MessageWriter<crate::MenuActionActivated<Action>>,
    risk: Option<Res<crate::MenuDestructiveActions<Action>>>,
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
) {
    if release.button != PointerButton::Primary {
        return;
    }
    let Ok((control, surface)) = controls.get(release.entity) else {
        return;
    };
    let Ok(mut state) = menus.get_mut(surface.0) else {
        return;
    };
    if !same_target(&state, control)
        || state
            .bounds
            .as_ref()
            .is_some_and(|bounds| !bounds.contains(&release.pointer_location))
    {
        state.cancel_pointer(release.pointer_id);
        return;
    }
    state.move_pointer(release.pointer_id, release.pointer_location.position);
    activate(
        surface.0,
        &mut state,
        release.pointer_id,
        risk.as_deref(),
        settings.as_deref(),
        &mut activated,
    );
}

/// Track motion outside widgets before picking sends a release to a control.
pub(super) fn track_action_motion<Action: Send + Sync + 'static>(
    mut inputs: MessageReader<PointerInput>,
    mut menus: Query<&mut crate::MenuInteraction<Action>>,
) {
    for input in inputs.read() {
        for mut state in &mut menus {
            if state.pointer != Some(input.pointer_id) {
                continue;
            }
            match input.action {
                PointerAction::Move { .. } => {
                    state.move_pointer(input.pointer_id, input.location.position)
                }
                PointerAction::Cancel => state.cancel_pointer(input.pointer_id),
                _ => {}
            }
        }
    }
}

/// Complete a release even when a rebuild has removed its event target.
pub(super) fn finish_actions<Action: Clone + PartialEq + Send + Sync + 'static>(
    mut inputs: MessageReader<PointerInput>,
    mut menus: Query<(Entity, &mut crate::MenuInteraction<Action>)>,
    pointers: Query<(&PointerId, &PointerPress)>,
    controls: Query<(&AmbitionMenuControl<Action>, &crate::MenuSurface), With<Button>>,
    risk: Option<Res<crate::MenuDestructiveActions<Action>>>,
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    mut activated: MessageWriter<crate::MenuActionActivated<Action>>,
) {
    for input in inputs.read() {
        if pointers
            .iter()
            .any(|(id, press)| *id == input.pointer_id && press.is_primary_pressed())
        {
            continue;
        }
        if matches!(
            input.action,
            PointerAction::Release(PointerButton::Primary) | PointerAction::Cancel
        ) {
            for (menu, mut state) in &mut menus {
                if state.pointer == Some(input.pointer_id) {
                    let valid = matches!(input.action, PointerAction::Release(_))
                        && state
                            .bounds
                            .as_ref()
                            .is_some_and(|bounds| bounds.contains(&input.location))
                        && controls.iter().any(|(control, surface)| {
                            surface.0 == menu && same_target(&state, control)
                        });
                    if valid {
                        state.move_pointer(input.pointer_id, input.location.position);
                        activate(
                            menu,
                            &mut state,
                            input.pointer_id,
                            risk.as_deref(),
                            settings.as_deref(),
                            &mut activated,
                        );
                    }
                    state.cancel_pointer(input.pointer_id);
                }
            }
        }
    }
}

pub(super) fn press_tabs(
    press: On<Pointer<Press>>,
    tabs: Query<(&BevyUiMenuTab, &crate::MenuSurface)>,
    mut menus: Query<&mut crate::MenuTabInteraction>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform, &ComputedUiTargetCamera)>,
    cameras: Query<&Camera>,
) {
    if press.button != PointerButton::Primary {
        return;
    }
    let Ok((tab, surface)) = tabs.get(press.entity) else {
        return;
    };
    if let Ok(mut state) = menus.get_mut(surface.0) {
        state.pointer = Some(press.pointer_id);
        state
            .arm
            .press(tab.index, Some(press.pointer_location.position));
        state.bounds = nodes
            .get(press.entity)
            .ok()
            .and_then(|(node, transform, target)| {
                tap_bounds(
                    &press.pointer_location,
                    node,
                    transform,
                    cameras.get(target.get()?).ok()?,
                )
            });
    }
}

pub(super) fn release_tabs(
    release: On<Pointer<Release>>,
    tabs: Query<(&BevyUiMenuTab, &crate::MenuSurface)>,
    mut menus: Query<&mut crate::MenuTabInteraction>,
    mut activated: MessageWriter<crate::MenuTabActivated>,
) {
    if release.button != PointerButton::Primary {
        return;
    }
    let Ok((tab, surface)) = tabs.get(release.entity) else {
        return;
    };
    let Ok(mut state) = menus.get_mut(surface.0) else {
        return;
    };
    if state.pointer != Some(release.pointer_id) {
        return;
    }
    state.pointer = None;
    if state
        .bounds
        .as_ref()
        .is_some_and(|bounds| !bounds.contains(&release.pointer_location))
    {
        state.arm.clear();
        return;
    }
    state.arm.moved(Some(release.pointer_location.position));
    if let Some(index) = state
        .arm
        .release(tab.index, Some(release.pointer_location.position))
    {
        activated.write(crate::MenuTabActivated {
            menu: surface.0,
            index,
        });
    }
}

pub(super) fn track_tab_motion(
    mut inputs: MessageReader<PointerInput>,
    mut menus: Query<&mut crate::MenuTabInteraction>,
) {
    for input in inputs.read() {
        for mut state in &mut menus {
            if state.pointer == Some(input.pointer_id) {
                match input.action {
                    PointerAction::Move { .. } => state.arm.moved(Some(input.location.position)),
                    PointerAction::Cancel => {
                        state.pointer = None;
                        state.arm.clear();
                    }
                    _ => {}
                }
            }
        }
    }
}

pub(super) fn finish_tabs(
    mut inputs: MessageReader<PointerInput>,
    mut menus: Query<(Entity, &mut crate::MenuTabInteraction)>,
    pointers: Query<(&PointerId, &PointerPress)>,
    tabs: Query<(&BevyUiMenuTab, &crate::MenuSurface)>,
    mut activated: MessageWriter<crate::MenuTabActivated>,
) {
    for input in inputs.read() {
        if pointers
            .iter()
            .any(|(id, press)| *id == input.pointer_id && press.is_primary_pressed())
        {
            continue;
        }
        if matches!(
            input.action,
            PointerAction::Release(PointerButton::Primary) | PointerAction::Cancel
        ) {
            for (menu, mut state) in &mut menus {
                if state.pointer == Some(input.pointer_id) {
                    let valid = matches!(input.action, PointerAction::Release(_))
                        && state
                            .bounds
                            .as_ref()
                            .is_some_and(|bounds| bounds.contains(&input.location))
                        && tabs.iter().any(|(tab, surface)| {
                            surface.0 == menu && Some(&tab.index) == state.arm.armed()
                        });
                    if valid {
                        state.arm.moved(Some(input.location.position));
                        if let Some(index) = state.arm.release_anywhere() {
                            activated.write(crate::MenuTabActivated { menu, index });
                        }
                    }
                    state.pointer = None;
                    state.arm.clear();
                }
            }
        }
    }
}
