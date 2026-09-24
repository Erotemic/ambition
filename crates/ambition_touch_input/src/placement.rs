//! The touch HUD's resolved on-screen placement.
//!
//! There is one resolved answer. This crate publishes what its clusters need
//! ([`touch_control_footprints`]), the presentation resolver decides where
//! they fit, and the rendered `Node`s, the raw multitouch hit test, and the
//! menu-drag exclusions all read the same rectangles from
//! [`TouchControlPlacement`]. Nothing here infers a margin from the window.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::gameplay_presentation::{
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
/// The smallest authored circle is 64 logical px before `TOUCH_SCALE`; this
/// keeps it at about 40px, the usual thumb-target floor. Below this, the
/// resolver overlays gameplay instead of shrinking further.
const ACTION_MIN_SCALE: f32 = 0.893;

/// Breathing room so the controlled subject is never framed flush against a
/// control.
///
/// Declared on the footprint, not the drawn node: the resolver places the
/// cluster and publishes what it occupies, so the padding goes with the
/// requirement.
pub const OCCUPANCY_PAD: f32 = 12.0;

/// What the touch clusters need, in logical pixels.
///
/// Sizes are the reserved footprints (the joystick's exclusion box and the
/// action bezel), not the visible art, so the breathing room stays inside the
/// rectangle.
pub fn touch_control_footprints() -> ControlFootprints {
    let pad = Vec2::splat(OCCUPANCY_PAD);
    ControlFootprints {
        // The movement stick does not compact. `virtual_joystick` sizes its
        // knob and base art, so scaling only this node would put the touch
        // region and the drawn stick out of step. It fits a reserved column
        // or overlays at full size.
        movement: Some(
            ControlFootprint::fixed(Vec2::splat(JOYSTICK_EXCLUSION_SIZE))
                .with_occlusion_padding(pad),
        ),
        primary_actions: Some(
            ControlFootprint::new(
                Vec2::new(ACTION_BEZEL_W, ACTION_BEZEL_H),
                Vec2::new(ACTION_BEZEL_W, ACTION_BEZEL_H) * ACTION_MIN_SCALE,
            )
            .with_occlusion_padding(pad),
        ),
        // The menu row is small corner chrome; shrinking it gains nothing. No
        // padding: `SystemMenuControl` does not reserve subject space.
        system_controls: Some(ControlFootprint::fixed(Vec2::new(
            MENU_ROW_W + MENU_ROW_MARGIN * 2.0,
            MENU_ROW_H + MENU_ROW_MARGIN * 2.0,
        ))),
    }
}

/// The touch overlay's half of the presentation lifecycle.
///
/// The order is:
///
/// ```text
/// touch requirements and visible-action selection   (PublishRequirements)
///     -> resolve gameplay presentation and fallback (GameplayPresentationSet)
///         -> apply resolved control placement       (ApplyPlacement)
/// ```
///
/// Both edges must be real ordering constraints. Otherwise, hiding the touch
/// HUD would collapse the reserved surround a frame late.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum TouchPresentationSet {
    /// Find the control roots this crate does not spawn itself and tag them.
    ///
    /// The movement stick's root belongs to `virtual_joystick`, so this crate
    /// discovers it after the fact. Discovery is part of the lifecycle
    /// because everything downstream queries for the markers it adds.
    Discover,
    /// What the clusters need this frame, given contextual availability and
    /// the overlay visibility setting.
    PublishRequirements,
    /// Project the resolver's answer onto the nodes this crate draws and
    /// hit-tests against.
    ApplyPlacement,
}

impl TouchPresentationSet {
    /// Declare the lifecycle: discover, then requirements before the resolve,
    /// then placement after it.
    ///
    /// One function, so no composer restates (and forgets) an edge; a missing
    /// edge draws controls at last frame's rectangles without failing.
    ///
    /// `Discover → PublishRequirements` is also the deferred-command boundary:
    /// discovery tags roots through `Commands`, and later systems query those
    /// markers. Bevy's `auto_insert_apply_deferred` (on by default) inserts
    /// the sync point on this ordering edge.
    pub fn configure(app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::gameplay_presentation::GameplayPresentationSet;
        use bevy::prelude::{IntoScheduleConfigs as _, Update};
        app.configure_sets(
            Update,
            (
                Self::Discover,
                Self::PublishRequirements.before(GameplayPresentationSet),
                Self::ApplyPlacement.after(GameplayPresentationSet),
            )
                .chain(),
        );
    }
}

/// The touch overlay's presentation lifecycle, as one installable unit.
///
/// The one declaration of the ordering contract. A hand-wired composer
/// (including a test) could forget an edge and draw controls at last frame's
/// rectangles, or show a new joystick unplaced for a frame. Installing this
/// plugin is the only supported way to get the pipeline.
pub struct TouchPresentationPlugin;

impl bevy::prelude::Plugin for TouchPresentationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        // A surface starts hidden. Otherwise it shows for one frame at the
        // joystick crate's own corner, over gameplay, whatever the setting.
        //
        // An observer, not `Display::None` at spawn: the movement surface is
        // a `TouchSurface` inserted onto an entity the joystick crate owns.
        app.add_observer(
            |surface: bevy::prelude::On<bevy::prelude::Add, crate::bevy_plugin::TouchSurface>,
             mut nodes: bevy::prelude::Query<&mut bevy::prelude::Node>| {
                if let Ok(mut node) = nodes.get_mut(surface.entity) {
                    node.display = bevy::prelude::Display::None;
                }
            },
        );
        use bevy::prelude::{IntoScheduleConfigs as _, Update};

        TouchPresentationSet::configure(app);
        app.init_resource::<TouchControlPlacement>();

        app.add_systems(
            Update,
            crate::bevy_plugin::tag_virtual_joystick_root.in_set(TouchPresentationSet::Discover),
        );
        app.add_systems(
            Update,
            (
                crate::bevy_plugin::sync_touch_visibility_from_settings,
                crate::bevy_plugin::sync_touch_ui_visibility,
                publish_touch_control_footprints,
            )
                .chain()
                .in_set(TouchPresentationSet::PublishRequirements),
        );
        app.add_systems(
            Update,
            (
                sync_touch_control_placement,
                crate::bevy_plugin::apply_touch_control_placement,
            )
                .chain()
                .in_set(TouchPresentationSet::ApplyPlacement),
        );
    }
}

/// The resolved rectangles for this frame.
///
/// Read by node placement, the raw hit test, and the menu-drag exclusions, so
/// a control cannot be drawn in one place and tappable in another.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct TouchControlPlacement {
    /// The movement stick's reserved box.
    pub movement: Option<ScreenRect>,
    /// The action cluster's bezel.
    pub action_bezel: Option<ScreenRect>,
    /// The action diamond inside the bezel: the space button positions are
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
    // The diamond is inset in the bezel by its authored padding, scaled.
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
/// Written every frame: `TouchControlsVisible` is a live setting, and a hidden
/// HUD must stop reserving space.
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
    use crate::bevy_plugin::{apply_touch_control_placement, TouchSurface};
    use crate::layout::{touch_action_at_position, touch_action_circle, touch_action_layout};
    use ambition_platformer2d_shared_tangle::gameplay_presentation::{
        ControlAnchor, ControlPlacement, PlacedControl, ResolvedControlRegions,
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

    /// A layout with the action cluster reserved into a left column and
    /// compacted.
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
            occlusions: Vec::new(),
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

    /// The rendered `Node`s and the raw hit test read the same rectangles: the
    /// drawn center of every button hit-tests to itself at any placement or
    /// scale.
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

    /// A compacted cluster is smaller; otherwise the test above would not
    /// cover scaling.
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

    /// Controls that were not placed leave the layout, so a hidden HUD is
    /// neither drawn nor tappable. Checks `display`, which decides whether the
    /// subtree renders.
    #[test]
    fn an_unplaced_surface_leaves_the_layout() {
        let mut app = app_with(ResolvedGameplayPresentation::default());
        let root = app
            .world_mut()
            .spawn((TouchSurface::MenuRow, Node::default()))
            .id();
        app.update();

        let node = app.world().entity(root).get::<Node>().unwrap();
        assert_eq!(
            node.display,
            Display::None,
            "a collapsed-but-displayed surface still draws its absolutely \
             positioned children at the screen origin"
        );
        assert_eq!(
            app.world().resource::<TouchControlPlacement>().menu_row,
            None
        );
    }

    /// A visible touch stick is drawn at the bottom-left.
    ///
    /// The companion to `an_unplaced_surface_leaves_the_layout`: a surface
    /// that should be on screen still gets a rectangle. A misplaced stick is
    /// playable; an absent one is not. Runs the real resolver with the real
    /// footprints and the real placement system.
    #[test]
    fn a_visible_touch_stick_is_placed_at_the_bottom_left_corner() {
        use ambition_platformer2d_shared_tangle::gameplay_presentation::{
            resolve_gameplay_presentation, GameplayPresentationInput, GameplayPresentationProfile,
            ScreenInsets,
        };

        let display = Vec2::new(1280.0, 720.0);
        let profile = GameplayPresentationProfile::full_bleed();
        let resolved = resolve_gameplay_presentation(GameplayPresentationInput {
            display_px: display,
            safe_area_insets: ScreenInsets::ZERO,
            profile: &profile,
            occlusions: &[],
            control_footprints: touch_control_footprints(),
        });

        let movement = resolved
            .controls
            .movement
            .expect(
                "a visible touch session must place its movement stick; if this is None the \
                 overlay is hidden entirely and the game is unplayable by touch",
            )
            .rect;
        assert!(
            movement.min.x < display.x * 0.5,
            "the stick belongs on the LEFT, got min.x={}",
            movement.min.x
        );
        assert!(
            movement.max.y > display.y - 1.0,
            "the stick belongs at the BOTTOM — flush with the safe area's lower \
             edge — got max.y={} on a {}px-tall display. Drawn at the TOP is the \
             exact symptom that put a d-pad over the versus scoreboard",
            movement.max.y,
            display.y
        );

        // The placement system makes it a drawn node, not a hidden one.
        let mut app = app_with(resolved);
        let root = app
            .world_mut()
            .spawn((TouchSurface::Movement, Node::default()))
            .id();
        app.update();
        let node = app.world().entity(root).get::<Node>().unwrap().clone();
        assert_eq!(
            node.display,
            Display::Flex,
            "a placed surface must stay in the layout"
        );
        assert_eq!(node.left, Val::Px(movement.min.x));
        assert_eq!(node.top, Val::Px(movement.min.y));
    }

    /// A surface is hidden the moment it is created, before placement runs
    /// (Z′6). Otherwise it shows for one frame at the joystick crate's
    /// corner. Checked without the placement system, because the point is the
    /// state before the first placement pass.
    #[test]
    fn a_freshly_created_surface_is_hidden_before_placement_ever_runs() {
        let mut app = App::new();
        app.add_plugins(TouchPresentationPlugin);
        let surface = app
            .world_mut()
            .spawn((TouchSurface::Movement, Node::default()))
            .id();
        assert_eq!(
            app.world().entity(surface).get::<Node>().unwrap().display,
            Display::None,
            "a surface drew for one frame at whatever rectangle it was spawned \
             with, before anything decided where it belongs"
        );
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

    /// A compacted cluster reserves what it actually covers.
    ///
    /// The occupancy comes from the same resolve that placed the cluster, so
    /// no second descriptor keeps the full-size corner rectangle. Uses the
    /// real resolver and footprints, because the padding is declared on the
    /// footprint and applied during placement.
    #[test]
    fn a_compacted_cluster_reserves_what_it_covers() {
        use ambition_platformer2d_shared_tangle::gameplay_presentation::{
            resolve_gameplay_presentation, ControlPlacementPolicy, GameplayPresentationInput,
            GameplayPresentationProfile, ScreenInsets, ScreenOcclusionPurpose, SoftFramingProfile,
        };

        // A 4:3 viewport on a 16:10 display leaves side columns too narrow for
        // the action bezel at full size, which is the compacting case.
        let profile = GameplayPresentationProfile::fixed_aspect(4.0, 3.0)
            .with_control_placement(ControlPlacementPolicy::PreferSurround)
            .with_occlusion_aware_framing(SoftFramingProfile::platformer());
        let resolved = resolve_gameplay_presentation(GameplayPresentationInput {
            display_px: Vec2::new(2400.0, 1080.0),
            safe_area_insets: ScreenInsets::ZERO,
            profile: &profile,
            occlusions: &[],
            control_footprints: touch_control_footprints(),
        });

        let actions = resolved
            .controls
            .primary_actions
            .expect("the action cluster is placed");
        let occlusion = resolved
            .controls
            .occlusions
            .iter()
            .find(|o| o.purpose == ScreenOcclusionPurpose::VirtualActionCluster)
            .expect("the action cluster publishes occupancy");

        let pad = Vec2::splat(OCCUPANCY_PAD) * actions.scale;
        assert_eq!(
            occlusion.rect,
            ScreenRect {
                min: actions.rect.min - pad,
                max: actions.rect.max + pad,
            },
            "occupancy must be the placed rectangle plus its declared padding",
        );
        assert!(
            !resolved.subject_safe_rect.overlaps(occlusion.rect),
            "and the framed region must clear it",
        );
    }
}
