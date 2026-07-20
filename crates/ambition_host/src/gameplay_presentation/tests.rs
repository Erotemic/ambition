//! Runtime oracles for the host presentation cluster.
//!
//! These pin acceptance oracles 3, 5 and 9 from
//! `docs/planning/triage/gameplay-presentation-profiles.md` against the real
//! systems, with a synthetic primary window instead of a winit surface.

use bevy::camera::RenderTarget;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, Display, Node, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowResolution};

use ambition_engine_core as ae;
use ambition_platformer_primitives::camera_layers::MainCamera;
use ambition_platformer_primitives::gameplay_presentation::{
    profiles, ActiveGameplayPresentationProfiles, ControlFootprint, GameplayPresentationProfile,
    GameplayPresentationProfiles, PresentationEnvironment, ResolvedGameplayPresentation,
    ScreenOccluder, ScreenOcclusionPurpose, ScreenRect, SoftFramingProfile,
};
use ambition_sim_view::camera_snapshot::{CameraScreenFraming, CameraViewport};

use super::*;

/// Build an app running just this cluster against a synthetic window.
fn host_app(
    display: ae::Vec2,
    scale_factor: f32,
    profiles: GameplayPresentationProfiles,
    environment: PresentationEnvironment,
) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(HostGameplayPresentationPlugin);
    app.insert_resource(ActiveGameplayPresentationProfiles(profiles));
    app.insert_resource(environment);

    let mut resolution = WindowResolution::new(display.x as u32, display.y as u32);
    resolution.set_scale_factor(scale_factor);
    resolution.set(display.x, display.y);
    app.world_mut().spawn((
        Window {
            resolution,
            ..default()
        },
        PrimaryWindow,
    ));
    app.world_mut().spawn((Camera::default(), MainCamera));
    app
}

fn resolved(app: &App) -> &ResolvedGameplayPresentation {
    app.world().resource::<ResolvedGameplayPresentation>()
}

/// The viewport of the ONE main camera that renders to the window. Cameras are
/// always born window-targeted, so a test that adds an offscreen camera is
/// filtered out here rather than making `.single()` ambiguous.
fn main_camera_viewport(app: &mut App) -> Option<Viewport> {
    let mut query = app
        .world_mut()
        .query_filtered::<(&Camera, &RenderTarget), With<MainCamera>>();
    let mut window_targeted = query
        .iter(app.world())
        .filter(|(_, target)| matches!(target, RenderTarget::Window(_)))
        .map(|(camera, _)| camera.viewport.clone());
    let viewport = window_targeted
        .next()
        .expect("a window-targeted main camera");
    assert!(window_targeted.next().is_none(), "expected exactly one");
    viewport
}

/// Oracle 3 — the camera observer reports the GAMEPLAY viewport dimensions,
/// not blindly the window dimensions. This is the single fact the whole
/// fixed-aspect slice rests on: every downstream visible-world and clamp
/// calculation reads `CameraViewport`.
#[test]
fn fixed_aspect_publishes_the_gameplay_viewport_not_the_window() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        profiles::fixed_four_by_three(),
        PresentationEnvironment::Desktop,
    );
    app.update();

    let gameplay = resolved(&app).gameplay_rect;
    assert_eq!(gameplay.size(), ae::Vec2::new(1440.0, 1080.0));
    assert_eq!(app.world().resource::<CameraViewport>().px, gameplay.size());
    assert_ne!(
        app.world().resource::<CameraViewport>().px,
        ae::Vec2::new(2400.0, 1080.0),
        "publishing the window here would silently stretch the 4:3 view",
    );
}

/// Full bleed publishes the whole window, which is exactly the pre-existing
/// behavior — a game that declares nothing must not move.
#[test]
fn full_bleed_publishes_the_whole_window() {
    let mut app = host_app(
        ae::Vec2::new(1920.0, 1080.0),
        1.0,
        GameplayPresentationProfiles::default(),
        PresentationEnvironment::Desktop,
    );
    app.update();

    assert_eq!(
        app.world().resource::<CameraViewport>().px,
        ae::Vec2::new(1920.0, 1080.0),
    );
    assert!(
        main_camera_viewport(&mut app).is_none(),
        "full bleed must leave the camera viewport cleared, not set it to the window",
    );
}

/// Oracle 5's mechanism — the gameplay rectangle becomes a real physical
/// `Camera::viewport`, converted through the window scale factor exactly once.
#[test]
fn fixed_aspect_applies_a_physical_camera_viewport() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        2.0,
        profiles::fixed_four_by_three(),
        PresentationEnvironment::Desktop,
    );
    app.update();

    let gameplay = resolved(&app).gameplay_rect;
    let viewport = main_camera_viewport(&mut app).expect("fixed aspect sets a viewport");
    assert_eq!(
        viewport.physical_position,
        (gameplay.min * 2.0).round().as_uvec2(),
    );
    assert_eq!(
        viewport.physical_size,
        (gameplay.size() * 2.0).round().as_uvec2(),
    );
    // Sanity: the physical rect is inside the physical window.
    assert!(viewport.physical_position.x + viewport.physical_size.x <= 4800);
}

/// The front HUD camera is untouched, so full-screen menus, load screens and
/// dialogue keep the whole display (oracle 11).
#[test]
fn only_the_main_camera_receives_a_viewport() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        profiles::fixed_four_by_three(),
        PresentationEnvironment::Desktop,
    );
    let hud = app.world_mut().spawn(Camera::default()).id();
    app.update();

    assert!(app
        .world()
        .entity(hud)
        .get::<Camera>()
        .expect("hud camera")
        .viewport
        .is_none());
}

/// Selection is by declared ENVIRONMENT, never by game name, and a touch
/// profile really does differ from its desktop sibling.
#[test]
fn the_environment_selects_the_declared_profile() {
    let display = ae::Vec2::new(900.0, 900.0);
    let mut desktop = host_app(
        display,
        1.0,
        profiles::fixed_four_by_three(),
        PresentationEnvironment::Desktop,
    );
    let mut touch = host_app(
        display,
        1.0,
        profiles::fixed_four_by_three(),
        PresentationEnvironment::TouchPrimary,
    );
    desktop.update();
    touch.update();

    assert_eq!(
        resolved(&desktop).gameplay_rect.size(),
        resolved(&touch).gameplay_rect.size(),
        "the same aspect is requested in both environments",
    );
    assert!(
        resolved(&touch).gameplay_rect.min.y < resolved(&desktop).gameplay_rect.min.y,
        "touch-primary pins the rectangle toward the top",
    );
}

/// A UI-shaped occluder bundle: the component carries only PURPOSE and
/// padding, and the rectangle comes from the computed layout — which is the
/// whole point of item 3. `ComputedNode` is in physical pixels and
/// `UiGlobalTransform` places the node's centre, exactly as `bevy_ui` writes
/// them.
fn ui_occluder(
    purpose: ScreenOcclusionPurpose,
    center_logical: ae::Vec2,
    size_logical: ae::Vec2,
    scale_factor: f32,
) -> (
    ScreenOccluder,
    ComputedNode,
    UiGlobalTransform,
    Node,
    InheritedVisibility,
) {
    let mut computed = ComputedNode::default();
    computed.size = size_logical * scale_factor;
    computed.inverse_scale_factor = 1.0 / scale_factor;
    (
        ScreenOccluder::new(purpose),
        computed,
        UiGlobalTransform::from_translation(center_logical * scale_factor),
        Node::default(),
        // `Node` requires `Visibility`, whose `InheritedVisibility` DEFAULTS TO
        // FALSE and is only turned true by the visibility propagation system.
        // A real app runs that; `MinimalPlugins` does not, so a fixture that
        // omitted this would report an unoccluded screen and quietly pass.
        InheritedVisibility::VISIBLE,
    )
}

/// The bottom-left stick, as a laid-out UI node.
fn stick_bundle() -> (
    ScreenOccluder,
    ComputedNode,
    UiGlobalTransform,
    Node,
    InheritedVisibility,
) {
    ui_occluder(
        ScreenOcclusionPurpose::VirtualMovementStick,
        ae::Vec2::new(324.0, 756.0),
        ae::Vec2::splat(600.0),
        1.0,
    )
}

/// Run enough frames for a GENERIC occluder to reach the resolve.
///
/// Generic occupancy is collected in `PostUpdate` (after `bevy_ui` has laid
/// out) and consumed by the NEXT frame's resolve — the declared lifecycle, not
/// an accident. One update is therefore not enough to see a newly spawned
/// occluder's effect, and a test that used one would be asserting a same-frame
/// contract this path deliberately does not offer.
///
/// On-screen CONTROLS are different: the resolver places them, so it publishes
/// their occupancy in the same pass. That is pinned end-to-end against the real
/// `bevy_ui` layout in `presentation_ui_lifecycle`.
fn settle(app: &mut App) {
    app.update();
    app.update();
}

fn occlusion_aware() -> GameplayPresentationProfiles {
    GameplayPresentationProfiles::uniform(
        GameplayPresentationProfile::full_bleed()
            .with_occlusion_aware_framing(SoftFramingProfile::platformer()),
    )
}

/// A control that is not on screen must not reserve space for itself.
#[test]
fn hidden_occluders_do_not_reserve_space() {
    let display = ae::Vec2::new(2400.0, 1080.0);
    let mut visible = host_app(
        display,
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    visible.world_mut().spawn(stick_bundle());
    settle(&mut visible);

    let mut hidden = host_app(
        display,
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    hidden
        .world_mut()
        .spawn(stick_bundle())
        .insert(InheritedVisibility::HIDDEN);
    settle(&mut hidden);

    let mut none = host_app(
        display,
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    settle(&mut none);

    assert_eq!(
        resolved(&hidden).subject_safe_rect,
        resolved(&none).subject_safe_rect,
        "a hidden control must not shrink framing",
    );
    assert_ne!(
        resolved(&visible).subject_safe_rect,
        resolved(&none).subject_safe_rect,
        "a visible control must shrink framing",
    );
}

/// A real touch control is a `bevy_ui` node: `Visibility` propagates into
/// `InheritedVisibility`, but `ViewVisibility` is never set true for UI, since
/// UI is not what the visibility system culls. Judging occupancy on
/// `ViewVisibility` would publish NOTHING in the real app while every test that
/// spawns a bare occluder still passed — occlusion-aware framing would
/// silently become plain soft framing.
#[test]
fn a_ui_shaped_occluder_is_collected_despite_false_view_visibility() {
    let display = ae::Vec2::new(2400.0, 1080.0);
    let mut app = host_app(
        display,
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    app.world_mut()
        .spawn(stick_bundle())
        // Exactly what a UI node carries: `ViewVisibility` default, i.e. NOT
        // visible to any view.
        .insert((Visibility::Visible, ViewVisibility::default()));
    settle(&mut app);

    let mut baseline = host_app(
        display,
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    settle(&mut baseline);

    assert_eq!(
        app.world().resource::<ScreenOccupancy>().0.len(),
        1,
        "a visible UI-shaped occluder must be collected",
    );
    assert_ne!(
        resolved(&app).subject_safe_rect,
        resolved(&baseline).subject_safe_rect,
        "and must actually shrink the safe region",
    );
}

/// Normal framing publishes an INACTIVE screen-framing fact, so the camera
/// resolver takes its ordinary centering path untouched (oracle 9's mechanism
/// on the desktop side).
#[test]
fn normal_framing_publishes_an_inactive_fact() {
    let mut app = host_app(
        ae::Vec2::new(1920.0, 1080.0),
        1.0,
        profiles::adaptive_platformer(),
        PresentationEnvironment::Desktop,
    );
    app.update();

    let framing = app.world().resource::<CameraScreenFraming>();
    assert!(!framing.active);
    assert_eq!(*framing, CameraScreenFraming::default());
}

/// Soft framing publishes the resolved region and its tuning.
#[test]
fn soft_framing_publishes_the_resolved_region() {
    let mut app = host_app(
        ae::Vec2::new(1920.0, 1080.0),
        1.0,
        profiles::high_speed_full_bleed(),
        PresentationEnvironment::Desktop,
    );
    app.update();

    let expected = SoftFramingProfile::high_speed();
    let framing = app.world().resource::<CameraScreenFraming>();
    assert!(framing.active);
    assert_eq!(framing.subject_safe_region, expected.safe_region);
    assert_eq!(framing.look_ahead_seconds, expected.look_ahead_seconds);
    assert_eq!(framing.subject_padding_px, expected.subject_padding_px);
}

/// Hysteresis: occupancy appearing mid-session must ease the region rather
/// than step it, or the camera twitches every time a contextual button shows.
#[test]
fn a_control_appearing_eases_the_region_instead_of_stepping_it() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    settle(&mut app);
    let settled = app
        .world()
        .resource::<CameraScreenFraming>()
        .subject_safe_region;

    app.world_mut().spawn(stick_bundle());
    // The occluder is collected at the end of this update and reaches the
    // resolve on the next one, which is the first frame the region has a new
    // target to ease toward.
    app.update();
    app.update();
    let after_one_frame = app
        .world()
        .resource::<CameraScreenFraming>()
        .subject_safe_region;
    let target = resolved(&app).subject_safe_region;

    assert_ne!(
        target, settled,
        "the fixture must actually change the region"
    );
    assert_ne!(
        after_one_frame, target,
        "the published region must not jump to the new target in one frame",
    );
    assert!(
        after_one_frame.min.x > settled.min.x && after_one_frame.min.x < target.min.x,
        "the published region should be easing between {settled:?} and {target:?}, got {after_one_frame:?}",
    );
}

/// A camera retargeted at an offscreen image sizes and frames itself against
/// that image (`capture_scene` does exactly this to the main camera). Applying
/// a window-derived viewport to it would clip the capture to a rectangle
/// computed for a display it is not drawing to.
#[test]
fn an_image_targeted_main_camera_keeps_its_own_framing() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        profiles::fixed_four_by_three(),
        PresentationEnvironment::Desktop,
    );

    let offscreen = app
        .world_mut()
        .spawn((
            Camera::default(),
            RenderTarget::Image(Handle::<Image>::default().into()),
            MainCamera,
        ))
        .id();
    app.update();

    assert!(
        app.world()
            .entity(offscreen)
            .get::<Camera>()
            .expect("offscreen camera")
            .viewport
            .is_none(),
        "an image-targeted camera must not inherit the display's gameplay rect",
    );
    // ...while the window-targeted main camera still gets one.
    assert!(main_camera_viewport(&mut app).is_some());
}

// ---------------------------------------------------------------------------
// Occupancy derived from real UI layout
// ---------------------------------------------------------------------------

fn occupied_rects(app: &App) -> Vec<ScreenRect> {
    app.world()
        .resource::<ScreenOccupancy>()
        .0
        .iter()
        .map(|occlusion| occlusion.rect)
        .collect()
}

/// Moving or resizing the node changes the collected occupancy, with NO second
/// geometry descriptor to update. This is the property the anchored form could
/// not have: it stored its own offset and size, so a control that moved kept
/// reserving the place it used to be.
#[test]
fn occupancy_follows_the_node_with_no_second_descriptor() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    let entity = app.world_mut().spawn(stick_bundle()).id();
    app.update();
    let before = occupied_rects(&app);
    assert_eq!(before.len(), 1);
    assert_eq!(
        before[0],
        ScreenRect::from_min_size(ae::Vec2::new(24.0, 456.0), ae::Vec2::splat(600.0))
    );

    // Move it, touching ONLY the layout — the `ScreenOccluder` component is
    // never rewritten.
    let occluder_before = *app.world().entity(entity).get::<ScreenOccluder>().unwrap();
    {
        let mut entity_mut = app.world_mut().entity_mut(entity);
        *entity_mut.get_mut::<UiGlobalTransform>().unwrap() =
            UiGlobalTransform::from_translation(ae::Vec2::new(2000.0, 300.0));
        entity_mut.get_mut::<ComputedNode>().unwrap().size = ae::Vec2::splat(200.0);
    }
    app.update();

    let after = occupied_rects(&app);
    assert_eq!(
        after[0],
        ScreenRect::from_min_size(ae::Vec2::new(1900.0, 200.0), ae::Vec2::splat(200.0)),
        "occupancy must follow the laid-out node",
    );
    assert_eq!(
        *app.world().entity(entity).get::<ScreenOccluder>().unwrap(),
        occluder_before,
        "and the occluder component itself must not have needed an edit",
    );
}

/// Occupancy collection is ORDERED against visibility propagation, not merely
/// observed to run after it.
///
/// The behavioural half of this lives in the app suite, where a real parent is
/// hidden with ordinary `Visibility` and the real propagation system decides.
/// That test passes with the ordering edge removed, because an unordered pair
/// still gets *some* order from the executor and today it happens to be the
/// right one — a green guardrail proving nothing. The invariant is the EDGE, so
/// this checks the edge.
///
/// `collect_screen_occupancy` reads `&InheritedVisibility` and
/// `visibility_propagate_system` writes it, so with nothing ordering them Bevy
/// records a genuine data conflict between the two. Declaring the dependency is
/// the only thing that removes it.
#[test]
fn occupancy_collection_is_ordered_against_visibility_propagation() {
    use bevy::camera::visibility::VisibilitySystems;
    use bevy::ecs::schedule::Schedules;
    use bevy::ecs::schedule::SystemSet as _;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // `VisibilityPlugin` runs mesh-bounds systems that hard-require these.
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<bevy::mesh::Mesh>();
    app.add_plugins(bevy::camera::visibility::VisibilityPlugin);
    app.add_plugins(HostGameplayPresentationPlugin);
    app.update();

    let schedules = app.world().resource::<Schedules>();
    let graph = schedules
        .get(PostUpdate)
        .expect("PostUpdate exists")
        .graph();

    let collectors = graph
        .systems_in_set(ScreenOccupancySet.intern())
        .expect("the occupancy collector is registered");
    let propagators = graph
        .systems_in_set(VisibilitySystems::VisibilityPropagate.intern())
        .expect("Bevy propagates visibility in PostUpdate");
    assert!(!collectors.is_empty() && !propagators.is_empty());

    let ambiguous = graph
        .conflicting_systems()
        .iter()
        .filter(|(a, b, _)| {
            (collectors.contains(a) && propagators.contains(b))
                || (collectors.contains(b) && propagators.contains(a))
        })
        .count();

    assert_eq!(
        ambiguous, 0,
        "occupancy collection must be ordered against visibility propagation, \
         but the schedule reports {ambiguous} unordered conflict(s) on \
         InheritedVisibility between them",
    );
}

/// A SCALED node occupies its scaled bounds.
///
/// `UiTransform::scale` does not change `ComputedNode::size` — it lands in
/// `UiGlobalTransform`'s matrix. Reading only the translation reported the
/// unscaled box, so a HUD panel animating in with a scale tween reserved its
/// final footprint on frame one and its actual footprint never.
#[test]
fn a_scaled_node_occupies_its_scaled_bounds() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    app.world_mut()
        .spawn(stick_bundle())
        .insert(UiGlobalTransform::from(
            bevy::math::Affine2::from_scale_angle_translation(
                ae::Vec2::new(0.5, 2.0),
                0.0,
                ae::Vec2::new(1000.0, 500.0),
            ),
        ));
    settle(&mut app);

    // A 600x600 node at half width and double height around (1000, 500).
    assert_eq!(
        occupied_rects(&app),
        vec![ScreenRect::from_min_size(
            ae::Vec2::new(850.0, -100.0),
            ae::Vec2::new(300.0, 1200.0),
        )],
    );
}

/// A ROTATED node occupies the bounding box of its rotated corners.
///
/// Occupancy is axis-aligned by contract, so a 45-degree square must reserve
/// the circumscribing box — `size * sqrt(2)` — not the box it would have had
/// unrotated.
#[test]
fn a_rotated_node_occupies_its_bounding_box() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    app.world_mut()
        .spawn(stick_bundle())
        .insert(UiGlobalTransform::from(
            bevy::math::Affine2::from_scale_angle_translation(
                ae::Vec2::ONE,
                std::f32::consts::FRAC_PI_4,
                ae::Vec2::new(1200.0, 540.0),
            ),
        ));
    settle(&mut app);

    let rects = occupied_rects(&app);
    assert_eq!(rects.len(), 1);
    let expected = 600.0 * std::f32::consts::SQRT_2;
    assert!(
        (rects[0].width() - expected).abs() < 0.01 && (rects[0].height() - expected).abs() < 0.01,
        "a 45-degree 600x600 node must reserve its {expected}px bounding box, got {:?}",
        rects[0],
    );
    assert!(
        (rects[0].center() - ae::Vec2::new(1200.0, 540.0)).length() < 0.01,
        "and stay centred on the node, got {:?}",
        rects[0].center(),
    );
}

/// A transform that collapses the node to zero area occludes nothing, the same
/// as a zero-sized layout.
#[test]
fn a_degenerate_transform_contributes_no_occlusion() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    app.world_mut()
        .spawn(stick_bundle())
        .insert(UiGlobalTransform::from(
            bevy::math::Affine2::from_scale_angle_translation(
                ae::Vec2::new(0.0, 1.0),
                0.0,
                ae::Vec2::new(1000.0, 500.0),
            ),
        ));
    settle(&mut app);
    assert!(occupied_rects(&app).is_empty());
}

/// `Display::None` removes a node from layout, so it occludes nothing.
#[test]
fn a_display_none_node_contributes_no_occlusion() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    let (occluder, computed, transform, mut node, visibility) = stick_bundle();
    node.display = Display::None;
    app.world_mut()
        .spawn((occluder, computed, transform, node, visibility));
    app.update();

    assert!(occupied_rects(&app).is_empty());
}

/// A zero-sized layout cannot occlude anything.
#[test]
fn a_zero_sized_node_contributes_no_occlusion() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    let (occluder, mut computed, transform, node, visibility) = stick_bundle();
    computed.size = ae::Vec2::ZERO;
    app.world_mut()
        .spawn((occluder, computed, transform, node, visibility));
    app.update();

    assert!(occupied_rects(&app).is_empty());
}

/// An invisible PARENT suppresses its children's occupancy, because
/// `InheritedVisibility` is the propagated hierarchy answer rather than the
/// entity's own `Visibility`.
#[test]
fn an_invisible_parent_suppresses_child_occlusion() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    // The propagation system is what writes HIDDEN onto the child; this fixture
    // asserts the collector honours that result.
    app.world_mut()
        .spawn(stick_bundle())
        .insert(InheritedVisibility::HIDDEN);
    app.update();

    assert!(occupied_rects(&app).is_empty());
}

/// `ComputedNode` is physical; the layout is logical. A 2x display must not
/// report occupancy at twice the size.
#[test]
fn physical_layout_converts_to_logical_occupancy() {
    let mut app = host_app(
        ae::Vec2::new(1200.0, 540.0),
        2.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    app.world_mut().spawn(ui_occluder(
        ScreenOcclusionPurpose::VirtualActionCluster,
        ae::Vec2::new(1000.0, 400.0),
        ae::Vec2::new(200.0, 100.0),
        2.0,
    ));
    app.update();

    assert_eq!(
        occupied_rects(&app),
        vec![ScreenRect::from_min_size(
            ae::Vec2::new(900.0, 350.0),
            ae::Vec2::new(200.0, 100.0),
        )],
        "occupancy is reported in the same logical space as the layout",
    );
}

/// A producer with no `bevy_ui` node keeps an explicit rectangle.
#[test]
fn a_non_ui_producer_may_supply_its_own_rectangle() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    let rect = ScreenRect::from_min_size(ae::Vec2::new(10.0, 20.0), ae::Vec2::new(300.0, 400.0));
    app.world_mut().spawn(ScreenOccluder::explicit(
        ScreenOcclusionPurpose::PersistentHud,
        rect,
    ));
    app.update();

    assert_eq!(occupied_rects(&app), vec![rect]);
}

/// A `ComputedUi` occluder with no layout yet contributes nothing rather than
/// falling back to some invented rectangle.
#[test]
fn a_ui_occluder_without_layout_contributes_nothing() {
    let mut app = host_app(
        ae::Vec2::new(2400.0, 1080.0),
        1.0,
        occlusion_aware(),
        PresentationEnvironment::TouchPrimary,
    );
    app.world_mut().spawn(ScreenOccluder::action_controls());
    app.update();

    assert!(occupied_rects(&app).is_empty());
}

/// Every field in the device diagnostic says what it actually holds.
///
/// The line this replaces printed `layout.surround` under the label
/// `viewport`. Nothing caught it, because the layout was resolving perfectly —
/// only the report was lying, and no test read the report. This one does.
#[test]
fn the_device_diagnostic_labels_are_truthful() {
    let display = ae::Vec2::new(2400.0, 1080.0);
    let mut app = host_app(
        display,
        1.0,
        profiles::fixed_four_by_three(),
        PresentationEnvironment::TouchPrimary,
    );
    app.world_mut()
        .resource_mut::<ControlFootprints>()
        .primary_actions = Some(ControlFootprint::new(
        ae::Vec2::new(233.0, 234.0),
        ae::Vec2::new(208.0, 209.0),
    ));
    app.update();

    let layout = resolved(&app);
    let line = describe_resolved_layout(PresentationEnvironment::TouchPrimary, layout);

    // A 4:3 profile pinned to the top for touch: the viewport field must name
    // the VIEWPORT policy, not the surround policy that used to sit there.
    assert!(
        line.contains("viewport=fixed-4:3-Top"),
        "the viewport field must describe the viewport policy: {line}",
    );
    assert!(
        line.contains(&format!("surround={:?}", layout.surround)),
        "and the surround policy must have its own field: {line}",
    );
    assert!(
        !line.contains(&format!("viewport={:?}", layout.surround)),
        "the surround policy must never be printed as the viewport: {line}",
    );

    // Every rectangle field carries the rectangle it names.
    for (label, rect) in [
        ("display", layout.display_rect),
        ("safe", layout.display_safe_rect),
        ("gameplay", layout.gameplay_rect),
        ("subject-safe", layout.subject_safe_rect),
    ] {
        let expected = format!(
            "{label}={}x{}@({},{})",
            rect.width().round(),
            rect.height().round(),
            rect.min.x.round(),
            rect.min.y.round(),
        );
        assert!(line.contains(&expected), "expected `{expected}` in: {line}",);
    }

    // And the facts a device check needs to see are all present.
    assert!(line.contains("env=TouchPrimary"), "{line}");
    assert!(
        line.contains(&format!("controls={:?}", layout.controls.placement)),
        "the control-placement fallback rung must be reported: {line}",
    );
    assert!(line.contains("actions="), "the action region: {line}");
    assert!(
        line.contains("safe-region="),
        "the subject-safe region: {line}"
    );
    assert!(
        line.contains("generic-occlusions="),
        "the generic occlusion count: {line}",
    );
    // Compact enough to read in a device log.
    assert!(
        line.len() < 400,
        "diagnostic is {} chars: {line}",
        line.len()
    );
}
