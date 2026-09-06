//! Visible-host integration for gameplay presentation profiles.
//!
//! Design of record: `docs/planning/triage/gameplay-presentation-profiles.md`.
//!
//! This module owns everything the pure resolver deliberately cannot know: the
//! primary window, the platform safe area, which stable presentation
//! environment the session is running in, and the physical Bevy camera
//! viewport. It resolves ONE layout per frame and publishes it; the camera
//! observation seam and every presentation consumer read that one product.
//!
//! The host must not know the names Ambition, Sanic, or Mary O — it cannot
//! even see a route. The provider layer selects
//! [`ActiveGameplayPresentationProfiles`]; this module only asks that resource
//! what policy is in force.

use bevy::camera::{RenderTarget, Viewport};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, Display, Node, UiGlobalTransform};
use bevy::window::PrimaryWindow;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_platformer2d_shared_tangle::gameplay_presentation::{
    resolve_gameplay_presentation, ActiveGameplayPresentationProfiles, ControlFootprints,
    DisplaySafeAreaInsets, GameplayPresentationInput, GameplayPresentationSet,
    GameplayViewportPolicy, NormalizedScreenRegion, PlacedControl, PresentationEnvironment,
    ResolvedGameplayPresentation, ScreenRect,
};
use ambition_sim_view::camera_snapshot::{
    CameraObservationSet, CameraScreenFraming, CameraViewport,
};

/// The occupancy collected from [`ScreenOccluder`] entities this frame,
/// resolved to logical display pixels.
///
/// Kept as its own resource so a debug overlay can show exactly what the
/// framing was composed against, and so collection stays independent of
/// resolution.
///
/// [`ScreenOccluder`]: ambition_platformer2d_shared_tangle::gameplay_presentation::ScreenOccluder
#[derive(Resource, Clone, Debug, Default)]
pub struct ScreenOccupancy(
    pub Vec<ambition_platformer2d_shared_tangle::gameplay_presentation::ScreenOcclusion>,
);

/// Ordering handle for generic UI occupancy collection.
///
/// Collection sits in `PostUpdate` behind two Bevy edges that are easy to get
/// wrong (see [`collect_screen_occupancy`]), so it gets a named set: anything
/// that needs to run around it can say so, and a test can ask the schedule
/// whether the edges are really there rather than observing that today's
/// executor happens to pick the right order.
#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct ScreenOccupancySet;

/// Resolve, publish, and apply the gameplay presentation layout.
pub struct HostGameplayPresentationPlugin;

impl Plugin for HostGameplayPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveGameplayPresentationProfiles>()
            .init_resource::<DisplaySafeAreaInsets>()
            .init_resource::<ResolvedGameplayPresentation>()
            .init_resource::<ScreenOccupancy>()
            // What the on-screen controls need. The touch presenter (or any
            // other control surface) writes it; the host only forwards it into
            // the pure resolver, so no host->touch dependency appears.
            .init_resource::<ControlFootprints>()
            .insert_resource(resolve_presentation_environment());
        // no `init_resource` here any more, and none is possible. These
        // are components on a local view now, spawned by `CameraObservationPlugin`
        // at plugin build time — so "somewhere to write" is a row in a query, and
        // a host with no view writes to nobody instead of to a global nobody
        // reads.

        app.add_systems(
            Update,
            (
                resolve_host_gameplay_presentation,
                (publish_camera_viewport, publish_camera_screen_framing),
            )
                .chain()
                .in_set(GameplayPresentationSet),
        );

        // Generic UI occupancy is collected AFTER `bevy_ui` has laid out AND
        // after hierarchy visibility has propagated, and is therefore consumed
        // by the NEXT frame's resolve. See `collect_screen_occupancy` for why
        // that is the honest schedule rather than a lag to be hidden, and why
        // BOTH edges are needed.
        app.add_systems(
            PostUpdate,
            collect_screen_occupancy
                .in_set(ScreenOccupancySet)
                .after(bevy::ui::UiSystems::Layout)
                .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
        );

        // The observer facts must be THIS frame's, so the whole cluster runs
        // before the camera observation consumes them. Ordering against the
        // observation SET rather than the system is what makes this edge real
        // in every host: the set is declared in `Update` regardless of which
        // schedule the simulation advances in.
        app.configure_sets(Update, GameplayPresentationSet.before(CameraObservationSet));

        // Applying the physical viewport is presentation-only and needs no
        // ordering against the sim, just this frame's resolved layout.
        app.add_systems(
            Update,
            apply_gameplay_camera_viewport.after(GameplayPresentationSet),
        );
    }
}

/// Decide the stable presentation environment ONCE, at app construction.
///
/// Deliberately not a system: the environment must not follow the most recent
/// input device. Glyphs may change the instant a gamepad is touched; the
/// gameplay viewport and camera framing must not, or the composition flickers
/// every time a thumb leaves the glass.
///
/// `AMBITION_PRESENTATION_ENV=desktop|touch|handheld` overrides the platform
/// guess, which is the only way to SEE the touch-primary framing on a desktop
/// dev machine.
fn resolve_presentation_environment() -> PresentationEnvironment {
    match std::env::var("AMBITION_PRESENTATION_ENV")
        .ok()
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("desktop") => return PresentationEnvironment::Desktop,
        Some("touch" | "touch_primary" | "mobile") => return PresentationEnvironment::TouchPrimary,
        Some("handheld") => return PresentationEnvironment::Handheld,
        Some(other) if !other.is_empty() => {
            warn!("AMBITION_PRESENTATION_ENV='{other}' is not a known environment; using the platform default");
        }
        _ => {}
    }

    if cfg!(any(target_os = "android", target_os = "ios")) {
        PresentationEnvironment::TouchPrimary
    } else {
        PresentationEnvironment::Desktop
    }
}

/// Gather visible [`ScreenOccluder`] UI nodes into logical-screen rectangles.
///
/// Geometry is collected after Bevy UI layout and inherited-visibility propagation,
/// then consumed by framing on the next frame. Hidden hierarchy, `Display::None`, and
/// zero-size nodes contribute nothing. `ViewVisibility` is not used for Bevy UI nodes.
/// On-screen controls publish same-frame occupancy separately because their resolver
/// already owns those rectangles.
pub fn collect_screen_occupancy(
    windows: Query<&Window, With<PrimaryWindow>>,
    occluders: Query<(
        &ambition_platformer2d_shared_tangle::gameplay_presentation::ScreenOccluder,
        Option<&InheritedVisibility>,
        Option<&ComputedNode>,
        Option<&UiGlobalTransform>,
        Option<&Node>,
    )>,
    mut occupancy: ResMut<ScreenOccupancy>,
) {
    occupancy.0.clear();
    let Ok(window) = windows.single() else {
        return;
    };
    let display = ScreenRect::from_min_size(
        ae::Vec2::ZERO,
        ae::Vec2::new(window.width().max(1.0), window.height().max(1.0)),
    );

    for (occluder, inherited, computed, transform, node) in &occluders {
        if !inherited.map(|visible| visible.get()).unwrap_or(true) {
            continue;
        }
        if node.is_some_and(|node| node.display == Display::None) {
            continue;
        }

        // Geometry the occluder owns itself (non-UI producers) resolves
        // directly; everything else comes from the computed layout.
        let occlusion = match occluder.self_resolved(display) {
            Some(occlusion) => occlusion,
            None => {
                let (Some(computed), Some(transform)) = (computed, transform) else {
                    continue;
                };
                let derived = occluder.from_computed_ui(
                    computed.size(),
                    // The FULL affine, not its translation: a scaled or rotated
                    // node — or one under a transformed parent — occupies its
                    // transformed bounds, not its layout box.
                    transform.affine(),
                    computed.inverse_scale_factor(),
                );
                let Some(derived) = derived else {
                    continue;
                };
                derived
            }
        };
        if occlusion.rect.is_empty() {
            continue;
        }
        occupancy.0.push(occlusion);
    }
}

/// The surface a windowless composition draws to.
///
/// A capture tool, an offscreen render, a headless acceptance run: each has a
/// real pixel rectangle and no `Window`. Without this the layout resolver below
/// found no primary window and returned, leaving `ResolvedGameplayPresentation`
/// at its default — so the viewport policy, the reserved surround, the HUD
/// regions and the on-screen control placement were ALL silently inert.
///
/// The capture had simply never told anything how big it was.
///
/// a window, when there is one, still wins. This is the fallback for
/// compositions that have no window at all, not an override.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct HeadlessDisplaySurface(pub ae::Vec2);

/// Resolve this frame's layout from the window (or the declared headless
/// surface), the safe area, the active profile, and the collected occupancy.
pub fn resolve_host_gameplay_presentation(
    windows: Query<&Window, With<PrimaryWindow>>,
    headless: Option<Res<HeadlessDisplaySurface>>,
    profiles: Res<ActiveGameplayPresentationProfiles>,
    environment: Res<PresentationEnvironment>,
    insets: Res<DisplaySafeAreaInsets>,
    occupancy: Res<ScreenOccupancy>,
    footprints: Res<ControlFootprints>,
    mut resolved: ResMut<ResolvedGameplayPresentation>,
) {
    // A window if there is one; otherwise the surface this composition declared.
    // Neither means nothing is being drawn to, and resolving a layout for a
    // surface that does not exist would be inventing one.
    let display_px = match windows.single() {
        Ok(window) => ae::Vec2::new(window.width().max(1.0), window.height().max(1.0)),
        Err(_) => match headless {
            Some(surface) => ae::Vec2::new(surface.0.x.max(1.0), surface.0.y.max(1.0)),
            None => return,
        },
    };
    let next = resolve_gameplay_presentation(GameplayPresentationInput {
        display_px,
        safe_area_insets: insets.0,
        profile: profiles.0.for_environment(*environment),
        occlusions: &occupancy.0,
        control_footprints: *footprints,
    });
    if *resolved != next {
        log_resolved_layout(*environment, &next);
        *resolved = next;
    }
}

/// Log the whole resolved layout whenever it materially changes.
///
/// There is no display or GPU in the environment this code is written in, so
/// nothing about the composition has ever been verified against pixels. This
/// line is what makes a device check cheap when someone has a phone in hand:
/// run the game, resize or rotate, and read off which profile is active, which
/// rung of the control ladder the display took, and every rectangle the layout
/// resolved — instead of inferring them from what the picture looks like.
///
/// Fires on CHANGE, not per frame: a steady session logs once, and a drag-resize
/// logs the sequence it actually walked through.
fn log_resolved_layout(
    environment: PresentationEnvironment,
    layout: &ResolvedGameplayPresentation,
) {
    info!("{}", describe_resolved_layout(environment, layout));
}

/// The diagnostic line itself, as a pure function.
///
/// Separated from the `info!` so a test can read it.
pub fn describe_resolved_layout(
    environment: PresentationEnvironment,
    layout: &ResolvedGameplayPresentation,
) -> String {
    let rect = |r: ScreenRect| {
        format!(
            "{}x{}@({},{})",
            r.width().round(),
            r.height().round(),
            r.min.x.round(),
            r.min.y.round()
        )
    };
    let region = |r: NormalizedScreenRegion| {
        format!(
            "({:.2},{:.2})..({:.2},{:.2})",
            r.min.x, r.min.y, r.max.x, r.max.y
        )
    };
    let viewport = |policy: GameplayViewportPolicy| match policy {
        GameplayViewportPolicy::FullBleed => "full-bleed".to_string(),
        GameplayViewportPolicy::FixedAspect { aspect, fit } => format!(
            "fixed-{}:{}-{fit:?}",
            aspect.width.round(),
            aspect.height.round()
        ),
    };
    let control = |name: &str, placed: Option<PlacedControl>| match placed {
        Some(placed) => format!(
            " {name}={} {}x{:.2}",
            rect(placed.rect),
            if placed.reserved {
                "reserved"
            } else {
                "over-gameplay"
            },
            placed.scale
        ),
        None => String::new(),
    };

    // Keep diagnostic labels aligned with the fields they print.
    format!(
        "presentation: env={:?} viewport={} surround={:?} hud={:?} \
         display={} safe={} gameplay={} subject-safe={} safe-region={} \
         framing={} controls={:?}{}{}{} hud-regions={} generic-occlusions={}",
        environment,
        viewport(layout.viewport),
        layout.surround,
        layout.hud,
        rect(layout.display_rect),
        rect(layout.display_safe_rect),
        rect(layout.gameplay_rect),
        rect(layout.subject_safe_rect),
        region(layout.subject_safe_region),
        match layout.soft_framing {
            Some(_) => "soft",
            None => "normal",
        },
        layout.controls.placement,
        control("movement", layout.controls.movement),
        control("actions", layout.controls.primary_actions),
        control("system", layout.controls.system_controls),
        layout.controls.hud.len(),
        layout.occlusions.len() - layout.controls.occlusions.len(),
    )
}

/// Publish the GAMEPLAY viewport — not the window — into the sim's camera
/// observation input.
///
/// one display resolve, N views. `ResolvedGameplayPresentation` describes
/// the physical screen; each local view is told the rectangle it presents into.
///
/// and each view may take a FRACTION of it — `ambition_sim_view::ViewPlacement`,
/// absent on every single-view composition and therefore the whole rectangle.
/// This is where a split layout becomes real: the placement is the composition's
/// data, this system is the one place the display rect and that fraction meet,
/// and `apply_gameplay_camera_viewport` already hands each camera its own view's
/// rectangle. The POLICY that CHOOSES a placement (adaptive share/split with
/// hysteresis) is a writer of that component and still does not exist; what a
/// permanently-split composition needs is only to state it.
pub fn publish_camera_viewport(
    presentation: Res<ResolvedGameplayPresentation>,
    mut views: Query<
        (
            &mut CameraViewport,
            Option<&ambition_sim_view::ViewPlacement>,
        ),
        With<ambition_sim_view::LocalView>,
    >,
) {
    let origin = presentation.gameplay_rect.min;
    let size = presentation.gameplay_rect.size().max(ae::Vec2::ONE);
    for (mut viewport, placement) in &mut views {
        let (origin_px, px) = placement.copied().unwrap_or_default().carve(origin, size);
        let rect = CameraViewport { px, origin_px };
        // Compare before writing: a change tick on every frame is a needless
        // re-run for anything gated on `is_changed()`.
        if *viewport != rect {
            *viewport = rect;
        }
    }
}

/// Publish the subject-safe region for the camera resolver, easing the region
/// itself so occupancy appearing or disappearing cannot step the camera.
pub fn publish_camera_screen_framing(
    time: Res<Time>,
    presentation: Res<ResolvedGameplayPresentation>,
    mut views: Query<&mut CameraScreenFraming, With<ambition_sim_view::LocalView>>,
) {
    for mut framing in &mut views {
        publish_one_views_screen_framing(&time, &presentation, &mut framing);
    }
}

fn publish_one_views_screen_framing(
    time: &Time,
    presentation: &ResolvedGameplayPresentation,
    framing: &mut CameraScreenFraming,
) {
    let Some(profile) = presentation.soft_framing else {
        *framing = CameraScreenFraming::default();
        return;
    };

    let target = presentation.subject_safe_region;
    // Hysteresis: a control fading in shrinks the region over ~a quarter
    // second instead of on one frame. A first activation snaps, since there is
    // no previous region to interpolate from.
    let region = if framing.active {
        let alpha = 1.0 - (-profile.region_ease_hz.max(0.0) * time.delta_secs()).exp();
        framing.subject_safe_region.lerp(target, alpha)
    } else {
        target
    };

    *framing = CameraScreenFraming {
        active: true,
        subject_safe_region: region,
        subject_padding_px: profile.subject_padding_px,
        look_ahead_seconds: profile.look_ahead_seconds,
    };
}

/// Apply each view's rectangle to the physical viewport of the camera that
/// presents it, leaving the front HUD camera full-screen.
///
/// Each camera now resolves its OWN view through `PresentsView`, by the same binding rule
/// `camera_follow` uses, and takes that view's rectangle.
///
/// the single-view case is unchanged, deliberately.
/// `publish_camera_viewport` writes the whole gameplay rectangle onto any view
/// that declares no `ambition_sim_view::ViewPlacement`, so the "full-bleed needs
/// no viewport at all" test below still compares exactly the same two rectangles
/// it always did and still leaves `Camera::viewport` cleared. Nothing about the
/// shipped picture moves.
///
/// `Camera::viewport` is in PHYSICAL pixels while the whole layout is resolved
/// in logical pixels (the space window cursors, touches, and `bevy_ui` share),
/// so the scale factor is applied here and nowhere else.
///
/// Only cameras rendering to the WINDOW are touched. The resolved layout is a
/// fact about the physical display, so applying it to a camera retargeted at
/// an offscreen image — which `capture_scene` does to the main camera, sizing
/// the image itself and resolving its own snapshot against that size — would
/// clip the capture to a rectangle computed for a window it is not drawing to.
pub fn apply_gameplay_camera_viewport(
    presentation: Res<ResolvedGameplayPresentation>,
    windows: Query<&Window, With<PrimaryWindow>>,
    views: Query<(Entity, &CameraViewport), With<ambition_sim_view::LocalView>>,
    // `RenderTarget` is a required COMPONENT of `Camera` rather than a field,
    // so every camera carries one; it defaults to the primary window.
    mut cameras: Query<
        (
            &mut Camera,
            &RenderTarget,
            Option<&ambition_sim_view::PresentsView>,
        ),
        With<MainCamera>,
    >,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let scale = window.scale_factor().max(f32::EPSILON);
    let display = presentation.display_rect;
    let on_hand = ambition_sim_view::ViewsOnHand::survey(views.iter().map(|(view, _)| view));

    for (mut camera, target, link) in &mut cameras {
        if !matches!(target, RenderTarget::Window(_)) {
            continue;
        }
        let Some(view) = on_hand.presented_by(link.copied()) else {
            continue;
        };
        let Ok((_, viewport)) = views.get(view) else {
            bevy::log::error_once!("a camera presents view {view:?}, which is not a local view");
            continue;
        };

        // A view filling the whole display needs no viewport at all. Leaving it
        // cleared keeps the ordinary path byte-identical to the pre-viewport
        // engine instead of round-tripping through physical pixels every frame.
        let fills_the_display = viewport.origin_px == display.min && viewport.px == display.size();
        let desired = (!fills_the_display).then(|| Viewport {
            physical_position: (viewport.origin_px * scale)
                .round()
                .max(ae::Vec2::ZERO)
                .as_uvec2(),
            physical_size: (viewport.px * scale).round().max(ae::Vec2::ONE).as_uvec2(),
            ..default()
        });

        // Compare before writing: touching `Camera` marks it changed, and a
        // camera that "changes" every frame is a needless render-world sync.
        if !viewport_matches(camera.viewport.as_ref(), desired.as_ref()) {
            camera.viewport = desired;
        }
    }
}

/// `bevy::camera::Viewport` is not `PartialEq`, and the fields that matter to
/// us are the physical rect and depth range.
fn viewport_matches(current: Option<&Viewport>, desired: Option<&Viewport>) -> bool {
    match (current, desired) {
        (None, None) => true,
        (Some(current), Some(desired)) => {
            current.physical_position == desired.physical_position
                && current.physical_size == desired.physical_size
                && current.depth == desired.depth
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;
