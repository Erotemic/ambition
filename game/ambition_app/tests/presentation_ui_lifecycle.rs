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
    profiles, ActiveGameplayPresentationProfiles, PresentationEnvironment,
    ResolvedGameplayPresentation, ScreenOcclusionPurpose, ScreenRect,
};
use ambition::touch_input::bevy_plugin::{
    apply_touch_control_placement, TouchControlsVisible, TouchSurface,
};
use ambition::touch_input::placement::{
    publish_touch_control_footprints, sync_touch_control_placement, TouchControlPlacement,
    TouchPresentationSet,
};

/// An app running the REAL `bevy_ui` layout pass, the real host presentation
/// cluster, and the real touch placement systems against a synthetic window.
///
/// Deliberately not `TouchControlsPlugin`: that would drag in leafwing, the
/// joystick crate, fonts and settings, none of which this lifecycle question
/// depends on. The three placement systems are the ones the plugin registers,
/// with the ordering contract the plugin declares.
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
    app.init_resource::<TouchControlPlacement>();
    // The SAME declaration `TouchControlsPlugin` uses — not a restatement of
    // it, so this test cannot pass against an ordering the real app lacks.
    TouchPresentationSet::configure(&mut app);
    app.add_systems(
        Update,
        publish_touch_control_footprints.in_set(TouchPresentationSet::PublishRequirements),
    );
    app.add_systems(
        Update,
        (sync_touch_control_placement, apply_touch_control_placement)
            .chain()
            .in_set(TouchPresentationSet::ApplyPlacement),
    );

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

    app.insert_resource(TouchControlsVisible(false));
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
