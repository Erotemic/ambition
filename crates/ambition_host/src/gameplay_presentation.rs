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
//! **The host must not know the names Ambition, Sanic, or Mary O** — it cannot
//! even see a route. The provider layer selects
//! [`ActiveGameplayPresentationProfiles`]; this module only asks that resource
//! what policy is in force.

use bevy::camera::{RenderTarget, Viewport};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, Display, Node, UiGlobalTransform};
use bevy::window::PrimaryWindow;

use ambition_engine_core as ae;
use ambition_platformer_primitives::camera_layers::MainCamera;
use ambition_platformer_primitives::gameplay_presentation::{
    resolve_gameplay_presentation, ActiveGameplayPresentationProfiles, ControlFootprints,
    DisplaySafeAreaInsets, GameplayPresentationInput, GameplayPresentationSet,
    PresentationEnvironment, ResolvedGameplayPresentation, ScreenRect,
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
/// [`ScreenOccluder`]: ambition_platformer_primitives::gameplay_presentation::ScreenOccluder
#[derive(Resource, Clone, Debug, Default)]
pub struct ScreenOccupancy(
    pub Vec<ambition_platformer_primitives::gameplay_presentation::ScreenOcclusion>,
);

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
        // Owned by `CameraObservationPlugin`, but this cluster WRITES them, so
        // it must not depend on plugin ordering to have somewhere to write.
        app.init_resource::<CameraViewport>()
            .init_resource::<CameraScreenFraming>();

        app.add_systems(
            Update,
            (
                resolve_host_gameplay_presentation,
                (publish_camera_viewport, publish_camera_screen_framing),
            )
                .chain()
                .in_set(GameplayPresentationSet),
        );

        // Generic UI occupancy is collected AFTER `bevy_ui` has laid out, and
        // is therefore consumed by the NEXT frame's resolve. See
        // `collect_screen_occupancy` for why that is the honest schedule rather
        // than a lag to be hidden.
        app.add_systems(
            PostUpdate,
            collect_screen_occupancy.after(bevy::ui::UiSystems::Layout),
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

/// Gather every published [`ScreenOccluder`] into one ordered list.
///
/// Occupancy for an ordinary `bevy_ui` producer is READ OFF ITS LAYOUT rather
/// than restated: `ComputedNode` plus `UiGlobalTransform` already account for
/// percentage sizing, parent constraints, flex reflow, safe-area shifts and
/// compact fallback layouts, so a HUD panel that moves or resizes changes its
/// occupancy with no second descriptor to keep in sync. `ComputedNode` is in
/// PHYSICAL pixels; `inverse_scale_factor` converts to the logical space the
/// rest of the layout uses, so this is DPI-correct without a window read.
///
/// # Lifecycle: collected after layout, consumed next frame
///
/// `bevy_ui` computes `ComputedNode` and `UiGlobalTransform` in `PostUpdate`
/// (`UiSystems::Layout`), so this runs there too — reading them from `Update`
/// would silently return the PREVIOUS frame's geometry while looking like a
/// same-frame read. The resolve in `Update` therefore composes against
/// occupancy collected at the end of the previous frame, and that is stated
/// rather than papered over.
///
/// One frame is the right trade for a GENERIC occluder: a HUD panel or dialogue
/// box is authored `bevy_ui`, its geometry is only knowable after taffy has
/// run, and the framing region eases toward its target at
/// `SoftFramingProfile::region_ease_hz` (~4 Hz) anyway, so a single frame is
/// well inside the hysteresis that already exists.
///
/// It is NOT the right trade for on-screen controls, which is why they are not
/// here: the resolver places them, so it can publish their occupancy in the
/// same pass with no round trip at all. See
/// [`ResolvedControlRegions::occlusions`].
///
/// [`ResolvedControlRegions::occlusions`]:
///     ambition_platformer_primitives::gameplay_presentation::ResolvedControlRegions::occlusions
///
/// An entity that is not actually displayed contributes nothing:
///
/// - `InheritedVisibility` false (this is the propagated hierarchy answer, so
///   an invisible PARENT suppresses its children too);
/// - `Display::None`, which taffy also collapses to a zero-sized node;
/// - a zero-sized layout, which cannot occlude anything by definition.
///
/// `ViewVisibility` is deliberately NOT consulted: it is computed per render
/// view for entities the visibility system culls, and `bevy_ui` nodes are not
/// among them, so it stays false on every control forever. Reading it would
/// publish no occupancy at all while every test still passed.
pub fn collect_screen_occupancy(
    windows: Query<&Window, With<PrimaryWindow>>,
    occluders: Query<(
        &ambition_platformer_primitives::gameplay_presentation::ScreenOccluder,
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

/// Resolve this frame's layout from the window, the safe area, the active
/// profile, and the collected occupancy.
pub fn resolve_host_gameplay_presentation(
    windows: Query<&Window, With<PrimaryWindow>>,
    profiles: Res<ActiveGameplayPresentationProfiles>,
    environment: Res<PresentationEnvironment>,
    insets: Res<DisplaySafeAreaInsets>,
    occupancy: Res<ScreenOccupancy>,
    footprints: Res<ControlFootprints>,
    mut resolved: ResMut<ResolvedGameplayPresentation>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let next = resolve_gameplay_presentation(GameplayPresentationInput {
        display_px: ae::Vec2::new(window.width().max(1.0), window.height().max(1.0)),
        safe_area_insets: insets.0,
        profile: profiles.0.for_environment(*environment),
        occlusions: &occupancy.0,
        control_footprints: *footprints,
    });
    if *resolved != next {
        *resolved = next;
    }
}

/// Publish the GAMEPLAY viewport — not the window — into the sim's camera
/// observation input.
///
/// This is the single line the whole fixed-aspect slice turns on: every
/// consumer of [`CameraViewport`] (orthographic scale, visible-world extent,
/// clamp half-extents) inherits the gameplay rectangle from here, so nothing
/// downstream needs to learn that a viewport exists.
pub fn publish_camera_viewport(
    presentation: Res<ResolvedGameplayPresentation>,
    mut viewport: ResMut<CameraViewport>,
) {
    let size = presentation.gameplay_rect.size().max(ae::Vec2::ONE);
    if viewport.px != size {
        viewport.px = size;
    }
}

/// Publish the subject-safe region for the camera resolver, easing the region
/// itself so occupancy appearing or disappearing cannot step the camera.
pub fn publish_camera_screen_framing(
    time: Res<Time>,
    presentation: Res<ResolvedGameplayPresentation>,
    mut framing: ResMut<CameraScreenFraming>,
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

/// Apply the resolved gameplay rectangle to the main camera's physical
/// viewport, leaving the front HUD camera full-screen.
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
    // `RenderTarget` is a required COMPONENT of `Camera` rather than a field,
    // so every camera carries one; it defaults to the primary window.
    mut cameras: Query<(&mut Camera, &RenderTarget), With<MainCamera>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let scale = window.scale_factor().max(f32::EPSILON);

    // Full-bleed with no safe-area inset needs no viewport at all. Leaving it
    // cleared keeps the ordinary path byte-identical to the pre-viewport
    // engine instead of round-tripping through physical pixels every frame.
    let desired = (presentation.gameplay_rect != presentation.display_rect).then(|| {
        let rect = presentation.gameplay_rect;
        Viewport {
            physical_position: (rect.min * scale).round().max(ae::Vec2::ZERO).as_uvec2(),
            physical_size: (rect.size() * scale).round().max(ae::Vec2::ONE).as_uvec2(),
            ..default()
        }
    });

    for (mut camera, target) in &mut cameras {
        if !matches!(target, RenderTarget::Window(_)) {
            continue;
        }
        // Compare before writing: touching `Camera` marks it changed, and a
        // camera that "changes" every frame is a needless render-world sync.
        if !viewport_matches(camera.viewport.as_ref(), desired.as_ref()) {
            camera.viewport = desired.clone();
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
