//! The touch HUD's resolved on-screen placement.
//!
//! Before this existed the touch overlay anchored itself to the physical
//! window corners while the presentation layout separately "reserved" surround
//! regions for controls. The two never met, so a fixed-aspect profile drew
//! sidebars that were decoration: at 1920x1200 each side surround is 160px
//! while the movement control reaches 185px and the action bezel is 233px
//! wide, and both clusters sat on top of the gameplay viewport anyway.
//!
//! Now there is ONE resolved answer. This crate publishes what its clusters
//! need ([`touch_control_footprints`]), the presentation resolver decides where
//! they fit, and everything here — the rendered `Node`s, the raw multitouch hit
//! test, and the menu-drag exclusions — reads the SAME rectangles out of
//! [`TouchControlPlacement`]. Nothing in this crate infers a margin from the
//! window any more.

use bevy::prelude::*;

use ambition_platformer_primitives::gameplay_presentation::{
    ControlFootprint, ControlFootprints, ResolvedGameplayPresentation, ScreenRect,
};

use super::layout::{
    ACTION_BEZEL_H, ACTION_BEZEL_PAD, ACTION_BEZEL_W, ACTION_CLUSTER_H, ACTION_CLUSTER_W,
    JOYSTICK_EXCLUSION_SIZE, MENU_ROW_MARGIN, MENU_ROW_W,
};

/// Menu row height, bezel-to-bezel (the row `Node`'s own height).
pub(super) const MENU_ROW_H: f32 = 54.0;

/// The smallest scale at which the action cluster's tightest touch target is
/// still reliably hittable.
///
/// The smallest authored circle is 64 logical px before `TOUCH_SCALE`; the
/// minimum keeps it at roughly 40px, which is the usual floor for a thumb
/// target. Below this the resolver prefers overlaying gameplay to shrinking
/// further — a control you cannot hit is worse than one that covers scenery.
const ACTION_MIN_SCALE: f32 = 0.893;

/// What the touch clusters need, in logical pixels.
///
/// Sizes are the RESERVED footprints (the joystick's generous exclusion box and
/// the action bezel), not the visible art, so a cluster placed at one of these
/// rectangles has its breathing room inside the rectangle rather than spilling
/// out of it.
pub fn touch_control_footprints() -> ControlFootprints {
    ControlFootprints {
        // The movement stick is deliberately NOT compactible. Its knob and
        // base art are sized by the `virtual_joystick` crate, so scaling this
        // crate's node without scaling that art would put the touch region and
        // the drawn stick out of agreement — the exact class of drift this
        // module exists to remove. It either fits a reserved column or it
        // overlays at full size.
        movement: Some(ControlFootprint::fixed(Vec2::splat(JOYSTICK_EXCLUSION_SIZE))),
        primary_actions: Some(ControlFootprint::new(
            Vec2::new(ACTION_BEZEL_W, ACTION_BEZEL_H),
            Vec2::new(ACTION_BEZEL_W, ACTION_BEZEL_H) * ACTION_MIN_SCALE,
        )),
        // The menu row is small, cornered chrome; shrinking it buys nothing.
        system_controls: Some(ControlFootprint::fixed(Vec2::new(
            MENU_ROW_W + MENU_ROW_MARGIN * 2.0,
            MENU_ROW_H + MENU_ROW_MARGIN * 2.0,
        ))),
    }
}

/// THE resolved rectangles for this frame.
///
/// One resource, read by the node placement, the raw hit test, and the
/// menu-drag exclusions — so a control cannot be drawn in one place and
/// tappable in another.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct TouchControlPlacement {
    /// The movement stick's reserved box.
    pub movement: Option<ScreenRect>,
    /// The action cluster's bezel.
    pub action_bezel: Option<ScreenRect>,
    /// The action diamond inside the bezel — the space button positions are
    /// authored in.
    pub action_cluster: Option<ScreenRect>,
    /// Uniform scale applied to the authored action layout.
    pub action_scale: f32,
    /// The Menu/Back row's own rectangle, inside its margin.
    pub menu_row: Option<ScreenRect>,
    /// Uniform scale applied to the movement stick's authored size.
    pub movement_scale: f32,
}

impl Default for TouchControlPlacement {
    fn default() -> Self {
        Self {
            movement: None,
            action_bezel: None,
            action_cluster: None,
            action_scale: 1.0,
            menu_row: None,
            movement_scale: 1.0,
        }
    }
}

/// Project the resolved control regions into the rectangles this crate draws
/// and hit-tests against.
pub fn sync_touch_control_placement(
    presentation: Res<ResolvedGameplayPresentation>,
    mut placement: ResMut<TouchControlPlacement>,
) {
    let regions = &presentation.controls;

    let movement = regions.movement.map(|placed| placed.rect);
    let movement_scale = regions.movement.map_or(1.0, |placed| placed.scale);

    let action_bezel = regions.primary_actions.map(|placed| placed.rect);
    let action_scale = regions.primary_actions.map_or(1.0, |placed| placed.scale);
    // The diamond is inset inside the bezel by the same padding it was
    // authored with, scaled along with everything else.
    let action_cluster = action_bezel.map(|bezel| {
        ScreenRect::from_min_size(
            bezel.min + Vec2::splat(ACTION_BEZEL_PAD * action_scale),
            Vec2::new(ACTION_CLUSTER_W, ACTION_CLUSTER_H) * action_scale,
        )
    });

    // The published system-controls footprint includes the row's margin; the
    // row itself sits inside it.
    let menu_row = regions.system_controls.map(|placed| {
        ScreenRect::from_min_size(
            placed.rect.min + Vec2::splat(MENU_ROW_MARGIN * placed.scale),
            Vec2::new(MENU_ROW_W, MENU_ROW_H) * placed.scale,
        )
    });

    let next = TouchControlPlacement {
        movement,
        action_bezel,
        action_cluster,
        action_scale,
        menu_row,
        movement_scale,
    };
    if *placement != next {
        *placement = next;
    }
}

/// Publish this crate's footprints so the resolver can place them.
///
/// Written every frame rather than once at startup: `TouchControlsVisible` is a
/// live setting, and a hidden HUD must stop reserving space for itself.
pub fn publish_touch_control_footprints(
    visible: Res<super::bevy_plugin::TouchControlsVisible>,
    mut footprints: ResMut<ControlFootprints>,
) {
    let next = if visible.0 {
        touch_control_footprints()
    } else {
        ControlFootprints::default()
    };
    if *footprints != next {
        *footprints = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bevy_plugin::{apply_touch_control_placement, TouchSurface, OCCUPANCY_PAD};
    use crate::layout::{touch_action_at_position, touch_action_circle, touch_action_layout};
    use ambition_platformer_primitives::gameplay_presentation::{
        ControlAnchor, ControlPlacement, PlacedControl, ResolvedControlRegions, ScreenOccluder,
    };

    fn px(value: Val) -> f32 {
        match value {
            Val::Px(px) => px,
            other => panic!("expected Px, got {other:?}"),
        }
    }

    fn node_rect(node: &Node) -> ScreenRect {
        ScreenRect::from_min_size(
            Vec2::new(px(node.left), px(node.top)),
            Vec2::new(px(node.width), px(node.height)),
        )
    }

    /// A layout with the action cluster reserved into a left-hand column and
    /// compacted, which is the case the old window-anchored code could not
    /// express at all.
    fn reserved_and_compacted() -> ResolvedGameplayPresentation {
        let bezel = ScreenRect::from_min_size(
            Vec2::new(6.0, 700.0),
            Vec2::new(ACTION_BEZEL_W, ACTION_BEZEL_H) * 0.9,
        );
        let mut presentation = ResolvedGameplayPresentation::default();
        presentation.controls = ResolvedControlRegions {
            placement: ControlPlacement::CompactSurround,
            movement: Some(PlacedControl {
                rect: ScreenRect::from_min_size(Vec2::new(4.0, 400.0), Vec2::splat(210.0)),
                anchor: ControlAnchor::Surround,
                reserved: true,
                scale: 1.0,
            }),
            primary_actions: Some(PlacedControl {
                rect: bezel,
                anchor: ControlAnchor::Surround,
                reserved: true,
                scale: 0.9,
            }),
            system_controls: None,
            hud: Vec::new(),
        };
        presentation
    }

    fn app_with(presentation: ResolvedGameplayPresentation) -> App {
        let mut app = App::new();
        app.insert_resource(presentation);
        app.init_resource::<TouchControlPlacement>();
        app.add_systems(
            Update,
            (sync_touch_control_placement, apply_touch_control_placement).chain(),
        );
        app
    }

    /// The rendered `Node`s and the raw hit test read the SAME rectangles.
    ///
    /// This is the property the review found missing: the layout reserved
    /// surround regions while the controls anchored to window corners, so the
    /// two descriptions were free to disagree. Now the drawn centre of every
    /// button hit-tests back to itself no matter where — or at what scale —
    /// the cluster was placed.
    #[test]
    fn drawn_buttons_and_the_hit_test_use_one_rectangle() {
        let mut app = app_with(reserved_and_compacted());
        let cluster_root = app
            .world_mut()
            .spawn((TouchSurface::ActionCluster, Node::default()))
            .id();
        let buttons: Vec<_> = touch_action_layout()
            .into_iter()
            .map(|spec| {
                (
                    spec,
                    app.world_mut().spawn((spec.action, Node::default())).id(),
                )
            })
            .collect();
        app.update();

        let cluster = node_rect(app.world().entity(cluster_root).get::<Node>().unwrap());
        let placement = *app.world().resource::<TouchControlPlacement>();
        assert_eq!(
            Some(cluster),
            placement.action_cluster,
            "the cluster root must be drawn at the resolved rectangle",
        );

        for (spec, entity) in buttons {
            let drawn = node_rect(app.world().entity(entity).get::<Node>().unwrap());
            // The button's own drawn box, in absolute screen space.
            let center = cluster.min + drawn.min + drawn.size() * 0.5;
            let (expected_center, _) = touch_action_circle(spec, cluster);
            assert!(
                (center - expected_center).length() < 0.01,
                "{:?}: drawn centre {center:?} != hit-test centre {expected_center:?}",
                spec.action,
            );
            assert_eq!(
                touch_action_at_position(center, Some(cluster), placement.menu_row),
                Some(spec.action),
                "{:?}: its own drawn centre must hit it",
                spec.action,
            );
        }
    }

    /// A compacted cluster really is smaller — otherwise the test above would
    /// pass on an uncompacted fixture and prove nothing about scaling.
    #[test]
    fn a_compacted_cluster_shrinks_its_buttons() {
        let mut app = app_with(reserved_and_compacted());
        let jump = touch_action_layout()
            .into_iter()
            .find(|spec| matches!(spec.action, crate::layout::TouchActionButton::Jump))
            .expect("Jump is in the layout");
        let entity = app.world_mut().spawn((jump.action, Node::default())).id();
        app.update();

        let drawn = node_rect(app.world().entity(entity).get::<Node>().unwrap());
        assert!(
            (drawn.width() - jump.size * 0.9).abs() < 0.01,
            "a 0.9-scaled cluster must draw 0.9-sized buttons, got {}",
            drawn.width(),
        );
    }

    /// Controls that were not placed collapse instead of lingering at their
    /// last rectangle, so a hidden HUD stops being tappable.
    #[test]
    fn an_unplaced_surface_collapses() {
        let mut app = app_with(ResolvedGameplayPresentation::default());
        let root = app
            .world_mut()
            .spawn((TouchSurface::MenuRow, Node::default()))
            .id();
        app.update();

        let node = app.world().entity(root).get::<Node>().unwrap();
        assert_eq!(px(node.width), 0.0);
        assert_eq!(px(node.height), 0.0);
        assert_eq!(app.world().resource::<TouchControlPlacement>().menu_row, None);
    }

    /// Hiding the touch HUD withdraws its footprints, so the layout stops
    /// reserving surround for controls that are not on screen.
    #[test]
    fn hidden_controls_publish_no_footprints() {
        let mut app = App::new();
        app.insert_resource(crate::bevy_plugin::TouchControlsVisible(false));
        app.init_resource::<ControlFootprints>();
        app.add_systems(Update, publish_touch_control_footprints);
        app.update();
        assert!(app.world().resource::<ControlFootprints>().is_empty());

        app.insert_resource(crate::bevy_plugin::TouchControlsVisible(true));
        app.update();
        assert!(!app.world().resource::<ControlFootprints>().is_empty());
    }

    /// A COMPACTED, reserved cluster publishes its ACTUAL final occupancy.
    ///
    /// This is the composition the review asked for: the occluder carries only
    /// a purpose, the node is placed by the resolver, and the occupancy is
    /// derived from that node through the same projection the host uses. A
    /// fallback layout therefore reserves what it really covers — there is no
    /// second descriptor left holding the full-size, corner-anchored rectangle.
    #[test]
    fn a_compacted_cluster_publishes_its_actual_occupancy() {
        let mut app = app_with(reserved_and_compacted());
        let bezel = app
            .world_mut()
            .spawn((TouchSurface::ActionBezel, Node::default()))
            .id();
        app.update();

        let node = node_rect(app.world().entity(bezel).get::<Node>().unwrap());
        let placement = *app.world().resource::<TouchControlPlacement>();
        assert_eq!(Some(node), placement.action_bezel);
        assert!(node.width() < ACTION_BEZEL_W, "the fixture must be compacted");

        // What `bevy_ui` would compute for that node on a 2x display, and what
        // the host would then derive from it.
        let scale = 2.0;
        let occlusion = ScreenOccluder::action_controls()
            .with_padding(Vec2::splat(OCCUPANCY_PAD))
            .from_computed_ui(node.size() * scale, node.center() * scale, 1.0 / scale)
            .expect("a sized node yields occupancy");

        assert_eq!(
            occlusion.rect,
            ScreenRect {
                min: node.min - Vec2::splat(OCCUPANCY_PAD),
                max: node.max + Vec2::splat(OCCUPANCY_PAD),
            },
            "occupancy must be the compacted node's own rectangle, plus padding",
        );
    }

}
