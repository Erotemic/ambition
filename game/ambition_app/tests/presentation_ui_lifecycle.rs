//! Does the presentation layout see THIS frame's control geometry?
//!
//! Design of record: `docs/planning/triage/gameplay-presentation-profiles.md`.
//!
//! Every other presentation test either checks the pure resolver or hands the
//! host a hand-written `ComputedNode`. Both prove arithmetic. Neither proves
//! the thing that actually decides whether a participant sees a correct frame:
//! **when** the numbers are available relative to Bevy's own UI layout pass.
//!
//! `bevy_ui` computes `ComputedNode` and `UiGlobalTransform` in `PostUpdate`
//! (`UiSystems::Layout`). Anything reading them from `Update` is reading the
//! PREVIOUS frame. So this test runs the real `UiPlugin` and the real touch
//! placement systems, changes the layout, and advances exactly one frame.
//!
//! The control clusters are placed BY the resolver and their occupancy feeds
//! BACK into it, so a computed-layout round trip would make that loop lag a
//! frame by construction. The invariant pinned here is that control occupancy
//! is same-frame: after one update, the resolved control regions, the published
//! occlusions, the subject-safe region, and the actually-laid-out `Node`s all
//! describe ONE layout.

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResolution};

use ambition::engine_core as ae;
use ambition::host::gameplay_presentation::{HostGameplayPresentationPlugin, ScreenOccupancy};
use ambition::platformer::camera_layers::MainCamera;
use ambition::presentation::gameplay_presentation::{
    profiles, ActiveGameplayPresentationProfiles, GameplayPresentationProfiles,
    PresentationEnvironment, ResolvedGameplayPresentation, ScreenOcclusionPurpose, ScreenRect,
};
use ambition::touch_input::bevy_plugin::{MobileStick, VirtualJoystickNode, VirtualJoystickPlugin};
use ambition::touch_input::bevy_plugin::{MobileTouchUiRoot, TouchControlsVisible, TouchSurface};
use ambition::touch_input::placement::{TouchControlPlacement, TouchPresentationPlugin};

/// An app running the REAL `bevy_ui` layout pass, the real host presentation
/// cluster, and the real touch placement systems against a synthetic window.
///
/// Deliberately not the whole `TouchControlsPlugin`: that would drag in
/// leafwing and the input stack, which no lifecycle question here depends on.
/// `TouchPresentationPlugin` is the exact unit the real plugin installs, so
/// this cannot pass against an ordering the shipping app lacks.
fn app(display: ae::Vec2) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(bevy::image::ImagePlugin::default());
    app.init_asset::<bevy::image::TextureAtlasLayout>();
    app.add_plugins(bevy::text::TextPlugin);
    // `bevy_ui`'s viewport picking runs unconditionally and hard-requires
    // `HoverMap`, so the picking plugins come along whether or not this test
    // cares about pointers. `PointerInputPlugin` is left out: it reads window
    // events this app never produces.
    app.add_plugins((
        bevy::input::InputPlugin,
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
    ));
    app.add_plugins(bevy::ui::UiPlugin);
    app.add_plugins(HostGameplayPresentationPlugin);

    app.insert_resource(ActiveGameplayPresentationProfiles(
        profiles::adaptive_platformer(),
    ));
    app.insert_resource(PresentationEnvironment::TouchPrimary);
    app.insert_resource(TouchControlsVisible(true));
    // The SAME unit `TouchControlsPlugin` installs.
    app.insert_resource(ambition::persistence::settings::UserSettings::default());
    app.add_plugins(TouchPresentationPlugin);

    let mut resolution = WindowResolution::new(display.x as u32, display.y as u32);
    resolution.set_scale_factor(1.0);
    resolution.set(display.x, display.y);
    app.world_mut().spawn((
        Window {
            resolution,
            ..default()
        },
        PrimaryWindow,
    ));
    app.world_mut().spawn((Camera2d, MainCamera));

    // The three surfaces the resolver places. Real `Node`s, so taffy lays them
    // out and `ComputedNode`/`UiGlobalTransform` are computed for real.
    //
    // Note what is NOT on these entities: a `ScreenOccluder`. Control occupancy
    // is published by the resolver that places them, so the drawn node is a
    // CONSUMER of the layout and never an input to it.
    for surface in [
        TouchSurface::Movement,
        TouchSurface::ActionBezel,
        TouchSurface::MenuRow,
    ] {
        app.world_mut()
            .spawn((Node::default(), surface, Name::new(format!("{surface:?}"))))
            // `InheritedVisibility` is propagated by the render world's
            // visibility pass, which this app does not run; it defaults to
            // FALSE. Set it directly — visibility gating has its own host-side
            // tests, and this one is about WHEN geometry is available.
            .insert(InheritedVisibility::VISIBLE);
    }
    app
}

/// The participant-facing touch-overlay setting, which
/// `sync_touch_visibility_from_settings` mirrors into `TouchControlsVisible`.
fn set_touch_controls_visible(app: &mut App, visible: bool) {
    app.world_mut()
        .resource_mut::<ambition::persistence::settings::UserSettings>()
        .controls
        .touch_controls_visible = visible;
}

fn resize(app: &mut App, display: ae::Vec2) {
    let mut windows = app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>();
    let mut window = windows
        .single_mut(app.world_mut())
        .expect("a primary window");
    window
        .resolution
        .set_physical_resolution(display.x as u32, display.y as u32);
    window.resolution.set(display.x, display.y);
}

/// The rectangle `bevy_ui` actually laid this surface out at, in logical px.
fn laid_out(app: &mut App, want: TouchSurface) -> ScreenRect {
    let mut query = app.world_mut().query::<(
        &TouchSurface,
        &bevy::ui::ComputedNode,
        &bevy::ui::UiGlobalTransform,
    )>();
    let (_, computed, transform) = query
        .iter(app.world())
        .find(|(surface, _, _)| **surface == want)
        .unwrap_or_else(|| panic!("{want:?} exists"));
    let scale = computed.inverse_scale_factor();
    let size = computed.size() * scale;
    let center = transform.translation * scale;
    ScreenRect::from_min_size(center - size * 0.5, size)
}

/// The occupancy THIS frame's layout was composed against.
///
/// Read off the resolved layout rather than [`ScreenOccupancy`]: that resource
/// holds only the generic `bevy_ui` occluders, which are collected after
/// layout and consumed next frame. Control occupancy is produced by the
/// resolve itself and never passes through it.
fn occlusion(app: &App, purpose: ScreenOcclusionPurpose) -> Option<ScreenRect> {
    resolved(app)
        .occlusions
        .iter()
        .find(|occlusion| occlusion.purpose == purpose)
        .map(|occlusion| occlusion.rect)
}

fn resolved(app: &App) -> &ResolvedGameplayPresentation {
    app.world().resource::<ResolvedGameplayPresentation>()
}

fn approx(a: ScreenRect, b: ScreenRect) -> bool {
    (a.min - b.min).length() < 0.5 && (a.max - b.max).length() < 0.5
}

/// ONE frame after the display changes, everything describes the NEW layout.
///
/// This is the regression the review asked for. Before it existed, control
/// occupancy was read off the previous frame's `ComputedNode`, so a resize left
/// the camera protecting the rectangles the controls used to occupy for one
/// visible frame — and every existing test passed, because they all wrote
/// `ComputedNode` by hand and never exercised the real `PostUpdate` layout.
#[test]
fn one_frame_after_a_resize_the_layout_and_its_occupancy_agree() {
    let mut app = app(ae::Vec2::new(1600.0, 900.0));
    // Settle: spawn, first layout, steady state.
    for _ in 0..4 {
        app.update();
    }

    let before = resolved(&app)
        .controls
        .primary_actions
        .expect("actions placed");
    resize(&mut app, ae::Vec2::new(1100.0, 720.0));

    // EXACTLY one frame.
    app.update();

    let now = resolved(&app);
    let actions = now.controls.primary_actions.expect("actions placed");
    assert!(
        !approx(actions.rect, before.rect),
        "the fixture must actually move the controls; got {:?} both times",
        actions.rect,
    );

    // 1. The published occupancy is THIS frame's control geometry.
    let published = occlusion(&app, ScreenOcclusionPurpose::VirtualActionCluster)
        .expect("the action cluster publishes occupancy");
    assert!(
        published.contains(actions.rect.min) && published.contains(actions.rect.max),
        "occupancy {published:?} must cover this frame's action region {:?}, \
         not the pre-resize one ({:?})",
        actions.rect,
        before.rect,
    );

    // 2. The subject-safe region was carved against THIS frame's occupancy.
    assert!(
        !now.subject_safe_rect.overlaps(actions.rect),
        "the subject-safe region {:?} still overlaps the action cluster {:?} — \
         it was composed against stale occupancy",
        now.subject_safe_rect,
        actions.rect,
    );

    // 3. The rectangle `bevy_ui` actually laid out is the same rectangle.
    let drawn = laid_out(&mut app, TouchSurface::ActionBezel);
    assert!(
        approx(drawn, actions.rect),
        "the laid-out bezel {drawn:?} must be the resolved region {:?}",
        actions.rect,
    );
}

/// The movement stick tells the same story on the other side of the display,
/// so the property is about the lifecycle rather than one lucky cluster.
#[test]
fn the_movement_stick_occupancy_is_also_same_frame() {
    let mut app = app(ae::Vec2::new(1600.0, 900.0));
    for _ in 0..4 {
        app.update();
    }

    let before = resolved(&app).controls.movement.expect("movement placed");
    // Small enough that the stick really does intrude on the authored safe
    // region, so the carve assertion below is load-bearing rather than
    // trivially true.
    resize(&mut app, ae::Vec2::new(900.0, 600.0));
    app.update();

    let movement = resolved(&app).controls.movement.expect("movement placed");
    assert!(
        !approx(movement.rect, before.rect),
        "the fixture must move the stick",
    );
    let published = occlusion(&app, ScreenOcclusionPurpose::VirtualMovementStick)
        .expect("the movement stick publishes occupancy");
    assert!(
        published.contains(movement.rect.min) && published.contains(movement.rect.max),
        "occupancy {published:?} must cover this frame's movement region {:?}",
        movement.rect,
    );
    assert!(
        !resolved(&app).subject_safe_rect.overlaps(movement.rect),
        "the subject-safe region must clear this frame's movement stick",
    );

    let drawn = laid_out(&mut app, TouchSurface::Movement);
    assert!(
        approx(drawn, movement.rect),
        "the laid-out stick {drawn:?} must be the resolved region {:?}",
        movement.rect,
    );
}

/// Withdrawing the footprints takes effect in the SAME update, not the next.
///
/// This is the footprint-publication ordering contract: requirements are
/// published before the resolve consumes them. Without the declared edge the
/// two systems are only ordered by an undeclared `ControlFootprints` resource
/// conflict, which the executor may serialize either way.
#[test]
fn hiding_the_controls_changes_the_layout_in_the_same_update() {
    // A display small enough that the overlaid controls actually carve the
    // safe region; on a roomy one the authored inset already clears the
    // corners and withdrawing them would change nothing.
    let mut app = app(ae::Vec2::new(1100.0, 720.0));
    for _ in 0..4 {
        app.update();
    }
    let framed = resolved(&app).subject_safe_rect;
    assert!(resolved(&app).controls.primary_actions.is_some());

    // Flip the SETTING, which is the source of truth; `TouchControlsVisible`
    // is a mirror of it and gets overwritten every frame.
    set_touch_controls_visible(&mut app, false);
    app.update();

    let now = resolved(&app);
    assert!(
        now.controls.primary_actions.is_none() && now.controls.movement.is_none(),
        "hidden controls must withdraw their footprints in the same update",
    );
    assert!(
        now.occlusions.is_empty(),
        "hidden controls must publish no occupancy, got {:?}",
        now.occlusions,
    );
    assert!(
        now.subject_safe_rect.area() > framed.area(),
        "withdrawing the controls must give the subject its space back \
         ({:?} vs {:?})",
        now.subject_safe_rect,
        framed,
    );
}

/// A transformed PARENT reaches its child's occupancy, through real layout.
///
/// `ui_layout_system` multiplies each node's local affine into its parent's, so
/// a child under a scaled parent has a scaled `UiGlobalTransform` while its own
/// `ComputedNode::size` is untouched. The host must therefore project the FULL
/// affine; a translation-only read would report the child's unscaled layout box
/// and reserve roughly four times the screen it actually covers.
///
/// Asserted as a ratio against the identical hierarchy at scale 1, so the test
/// pins the propagation rather than restating Bevy's transform arithmetic.
#[test]
fn a_transformed_parent_reaches_its_childs_occupancy() {
    fn occupancy_under_parent_scale(scale: f32) -> ScreenRect {
        let mut app = app(ae::Vec2::new(1600.0, 900.0));
        let child = app
            .world_mut()
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(40.0),
                    top: Val::Px(30.0),
                    width: Val::Px(200.0),
                    height: Val::Px(100.0),
                    ..default()
                },
                ambition::presentation::gameplay_presentation::ScreenOccluder::hud(),
            ))
            .insert(InheritedVisibility::VISIBLE)
            .id();
        app.world_mut()
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(400.0),
                    top: Val::Px(200.0),
                    width: Val::Px(400.0),
                    height: Val::Px(300.0),
                    ..default()
                },
                UiTransform::from_scale(Vec2::splat(scale)),
            ))
            .insert(InheritedVisibility::VISIBLE)
            .add_child(child);

        // Occupancy is collected in PostUpdate, so one update suffices to
        // observe it in `ScreenOccupancy` (the resolve consumes it next frame).
        app.update();
        let occupancy = &app.world().resource::<ScreenOccupancy>().0;
        assert_eq!(occupancy.len(), 1, "exactly the HUD child occludes");
        occupancy[0].rect
    }

    let plain = occupancy_under_parent_scale(1.0);
    let halved = occupancy_under_parent_scale(0.5);

    assert!(
        (plain.size() - ae::Vec2::new(200.0, 100.0)).length() < 0.5,
        "the untransformed child occupies its own layout box, got {plain:?}",
    );
    assert!(
        (halved.size() - plain.size() * 0.5).length() < 0.5,
        "a 0.5-scaled parent must halve its child's occupancy: {halved:?} vs {plain:?}",
    );
}

/// A parent hidden with ordinary `Visibility` stops its child occluding, in
/// the frame Bevy propagates that decision.
///
/// The host-side hidden-parent test inserts `InheritedVisibility::HIDDEN`
/// directly, which proves the collector reads the component but assumes the
/// component is current. It is not current for free: `InheritedVisibility` is
/// written by `VisibilitySystems::VisibilityPropagate`, which Bevy schedules
/// after `TransformSystems::Propagate`, while `ui_layout_system` runs before
/// it. Ordering occupancy collection against layout alone therefore allowed
/// current geometry with stale visibility.
///
/// This test never touches `InheritedVisibility`. It flips ordinary
/// `Visibility` on a parent and lets the real propagation system decide, in
/// both directions.
#[test]
fn hiding_a_parent_withdraws_its_childs_occupancy() {
    let mut app = app(ae::Vec2::new(1600.0, 900.0));
    // The real propagation pass, which `MinimalPlugins` does not include.
    // `VisibilityPlugin` also runs mesh-bounds systems that hard-require
    // `Assets<Mesh>`; this app draws no meshes but must still satisfy them.
    app.init_asset::<bevy::mesh::Mesh>();
    app.add_plugins(bevy::camera::visibility::VisibilityPlugin);

    let child = app
        .world_mut()
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(40.0),
                top: Val::Px(30.0),
                width: Val::Px(200.0),
                height: Val::Px(100.0),
                ..default()
            },
            ambition::presentation::gameplay_presentation::ScreenOccluder::hud(),
        ))
        .id();
    let parent = app
        .world_mut()
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(400.0),
                top: Val::Px(200.0),
                width: Val::Px(400.0),
                height: Val::Px(300.0),
                ..default()
            },
            Visibility::Visible,
        ))
        .add_child(child)
        .id();

    let occluding = |app: &App| !app.world().resource::<ScreenOccupancy>().0.is_empty();

    for _ in 0..3 {
        app.update();
    }
    assert!(
        occluding(&app),
        "a visible child under a visible parent must occlude",
    );

    // Hide the PARENT with ordinary visibility, and let Bevy propagate.
    *app.world_mut()
        .entity_mut(parent)
        .get_mut::<Visibility>()
        .unwrap() = Visibility::Hidden;
    app.update();
    assert!(
        !occluding(&app),
        "a hidden parent must withdraw its child's occupancy in the frame the \
         propagation runs, not the frame after",
    );

    // And back again, so the test pins a transition rather than a one-way door.
    *app.world_mut()
        .entity_mut(parent)
        .get_mut::<Visibility>()
        .unwrap() = Visibility::Visible;
    app.update();
    assert!(
        occluding(&app),
        "showing the parent again must restore its child's occupancy",
    );
}

// ---------------------------------------------------------------------------
// The REAL virtual joystick
// ---------------------------------------------------------------------------
//
// Every test above spawns its own already-tagged `TouchSurface` nodes, which
// assumes away the one root this crate does not spawn: the movement stick's,
// which belongs to `virtual_joystick` and can only be DISCOVERED. Discovery
// tags it through `Commands`, so until it was part of the declared lifecycle
// the frame a joystick appeared it carried neither marker — unplaced by
// `apply_touch_control_placement`, unhidden by `sync_touch_ui_visibility` — and
// showed for one frame at the joystick crate's own bottom-left corner
// position, over gameplay, whatever the touch-controls setting said.

/// An app that spawns the REAL joystick through the real plugin.
fn joystick_app(display: ae::Vec2, profiles: GameplayPresentationProfiles) -> App {
    let mut app = app(display);
    app.world_mut()
        .resource_mut::<ActiveGameplayPresentationProfiles>()
        .0 = profiles;
    app.add_plugins(VirtualJoystickPlugin::<MobileStick>::default());
    // The real spawner, in the schedule the real plugin uses.
    app.add_systems(
        Startup,
        ambition::touch_input::bevy_plugin::spawn_touch_joysticks,
    );
    app
}

/// The joystick root, once it exists.
fn joystick_root(app: &mut App) -> Entity {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, With<VirtualJoystickNode<MobileStick>>>();
    query
        .iter(app.world())
        .next()
        .expect("the real joystick root exists")
}

/// On the FIRST frame the real joystick exists it is already part of the
/// pipeline: tagged, placed, and obeying the touch-controls setting.
#[test]
fn the_real_joystick_is_placed_on_its_first_frame() {
    let mut app = joystick_app(
        ae::Vec2::new(1600.0, 900.0),
        profiles::adaptive_platformer(),
    );
    // Exactly one frame. Startup spawns the joystick; Update must discover,
    // publish, resolve and place it before this returns.
    app.update();

    let root = joystick_root(&mut app);
    let entity = app.world().entity(root);
    assert_eq!(
        entity.get::<TouchSurface>().copied(),
        Some(TouchSurface::Movement),
        "the joystick root must be tagged in the same frame it appears",
    );
    assert!(
        entity.contains::<MobileTouchUiRoot>(),
        "and carry the touch UI root marker, or nothing can hide it",
    );

    let placement = *app.world().resource::<TouchControlPlacement>();
    let movement = placement.movement.expect("the stick is placed");
    let node = entity.get::<Node>().expect("the root has a Node");
    assert_eq!(
        (node.left, node.top),
        (Val::Px(movement.min.x), Val::Px(movement.min.y)),
        "the drawn root must sit at the resolved rectangle, not at the \
         joystick crate's authored corner",
    );

    // Rendered placement and the drag-exclusion geometry are the same
    // rectangle, read from the same resource — not two formulas that agree.
    assert!(
        movement.contains(movement.center()),
        "the resolved movement region must be a real rectangle",
    );
    assert_eq!(
        resolved(&app).controls.movement.map(|placed| placed.rect),
        Some(movement),
        "hit testing and rendering must read ONE resolved region",
    );
}

/// Mary O reserves a 4:3 viewport. The joystick must never be seen over it,
/// including on the frame it is created.
#[test]
fn the_real_joystick_never_flashes_over_mary_o_gameplay() {
    // 2400x1080 leaves 720px side surrounds — room for the stick's reserved
    // column, so the correct answer here is genuinely "outside gameplay".
    let mut app = joystick_app(
        ae::Vec2::new(2400.0, 1080.0),
        profiles::fixed_four_by_three(),
    );
    app.update();

    let layout = resolved(&app);
    let movement = layout
        .controls
        .movement
        .expect("the stick is placed in the reserved surround");
    assert!(
        movement.reserved,
        "the fixture must actually reserve, got {:?}",
        layout.controls.placement,
    );

    let root = joystick_root(&mut app);
    let node = app.world().entity(root).get::<Node>().unwrap();
    let drawn = ScreenRect::from_min_size(
        ae::Vec2::new(px(node.left), px(node.top)),
        ae::Vec2::new(px(node.width), px(node.height)),
    );
    assert!(
        !drawn.overlaps(resolved(&app).gameplay_rect),
        "the joystick must not be drawn over the 4:3 gameplay rect even on \
         its first frame: {drawn:?} vs {:?}",
        resolved(&app).gameplay_rect,
    );
}

/// With the touch overlay switched off, the joystick must not be visible for
/// even one frame.
#[test]
fn the_real_joystick_does_not_flash_when_touch_controls_are_off() {
    let mut app = joystick_app(
        ae::Vec2::new(1600.0, 900.0),
        profiles::adaptive_platformer(),
    );
    set_touch_controls_visible(&mut app, false);
    app.update();

    let root = joystick_root(&mut app);
    assert_eq!(
        app.world().entity(root).get::<Visibility>().copied(),
        Some(Visibility::Hidden),
        "a joystick created while the touch overlay is off must be hidden on \
         the frame it appears, not the frame after",
    );
    assert!(
        app.world()
            .resource::<TouchControlPlacement>()
            .movement
            .is_none(),
        "and reserve no space, since hidden controls withdraw their footprints",
    );

    // Turning it back on restores both, without waiting a frame for a marker.
    set_touch_controls_visible(&mut app, true);
    app.update();
    assert_eq!(
        app.world().entity(root).get::<Visibility>().copied(),
        Some(Visibility::Inherited),
    );
    assert!(app
        .world()
        .resource::<TouchControlPlacement>()
        .movement
        .is_some());
}

fn px(value: Val) -> f32 {
    match value {
        Val::Px(px) => px,
        other => panic!("expected Px, got {other:?}"),
    }
}
