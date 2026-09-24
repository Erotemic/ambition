//! Profiling-only presentation census: the views, targets and draw population
//! Ambition hands the renderer.
//!
//! `perf` shows that the renderer was hot and Tracy names the pass; neither
//! shows that the frame had three world-rendering cameras and two portal
//! captures. These rows do, on the shared clock in
//! [`ambition_dev_tools::runtime_census`], so a slow interval can be read
//! against its scene.
//!
//! Rows written here (one line each, `[census] <kind> t=<seconds> k=v ...`):
//!
//! - `views` — the per-frame rollup: how many cameras, how many active, how
//!   many draw the world, how many draw offscreen.
//! - `camera` — one row per active camera: identity, semantic role, target,
//!   resolution, order, render layers, and the view it presents.
//! - `draws` — sprite / text / mesh population and how much of it is visible.
//! - `portal` — capture rigs, their budget, and the resolution they capture at.
//! - `render_pass` — Bevy's own render diagnostics, when the backend supplies
//!   them.
//!
//! Everything uses one gate: without `AMBITION_PROFILE_CENSUS` each system is
//! one bool test per frame, with no per-entity iteration on non-sample frames.

use bevy::camera::visibility::{RenderLayers, ViewVisibility};
use bevy::camera::RenderTarget;
use bevy::diagnostic::{
    Diagnostic, DiagnosticPath, Diagnostics, DiagnosticsStore, RegisterDiagnostic,
};
use bevy::prelude::*;

use ambition_dev_tools::runtime_census::RuntimeCensus;
use ambition_platformer2d_shared_tangle::camera_layers::{
    FrontHudCamera, MainCamera, FRONT_HUD_LAYER, LOCAL_VIEW_RENDER_LAYER_BASE,
    PARALLAX_BACKGROUND_LAYER,
};
use ambition_sim_view::{LocalView, LocalViewId, PresentedForView, PresentsView};

/// What a camera is for, as far as the composition says.
///
/// Roles come from the markers the spawner sets, not from geometry. A camera
/// with no marker is [`Self::Other`] and still reports its `Name`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CameraRole {
    /// A gameplay camera in a single-view composition.
    MainGameplay,
    /// A gameplay camera bound to one local (split-screen) view.
    LocalView,
    /// The front HUD/UI camera.
    Hud,
    /// A portal capture rig drawing into an offscreen image.
    PortalCapture,
    /// Not a gameplay camera and not the HUD, but it renders to an image —
    /// a menu backdrop, a kaleidoscope face, a capture harness.
    Offscreen,
    /// Marked by nobody. Read the `name=` field on the row.
    Other,
}

impl CameraRole {
    /// Stable token for the CSV column; do not rename without updating
    /// `scripts/profile_desktop.sh`'s summary.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MainGameplay => "main_gameplay",
            Self::LocalView => "local_view",
            Self::Hud => "hud",
            Self::PortalCapture => "portal_capture",
            Self::Offscreen => "offscreen",
            Self::Other => "other",
        }
    }

    /// Whether this role draws the simulated world, as opposed to overlaying it.
    /// The count of these answers "is the same world rendered more than once".
    pub fn renders_world(self) -> bool {
        matches!(
            self,
            Self::MainGameplay | Self::LocalView | Self::PortalCapture
        )
    }
}

/// Classify one camera from the markers its spawner set.
///
/// Order matters: a portal capture rig also carries an image target, and the
/// HUD camera is a `Camera2d` like the gameplay cameras. The most specific
/// claim wins.
pub fn classify_camera(
    is_portal_rig: bool,
    is_hud: bool,
    is_main: bool,
    presents_view: bool,
    target_is_image: bool,
) -> CameraRole {
    if is_portal_rig {
        CameraRole::PortalCapture
    } else if is_hud {
        CameraRole::Hud
    } else if is_main {
        if presents_view {
            CameraRole::LocalView
        } else {
            CameraRole::MainGameplay
        }
    } else if target_is_image {
        CameraRole::Offscreen
    } else {
        CameraRole::Other
    }
}

/// `RenderTarget` is a component, not a camera field. A camera without one
/// draws to the primary window, the common case, which must not read as
/// "unknown".
fn target_kind(target: Option<&RenderTarget>) -> &'static str {
    match target {
        None => "primary_window",
        Some(RenderTarget::Window(_)) => "window",
        Some(RenderTarget::Image(_)) => "image",
        Some(RenderTarget::TextureView(_)) => "texture_view",
        Some(RenderTarget::None { .. }) => "none",
    }
}

fn target_is_image(target: Option<&RenderTarget>) -> bool {
    matches!(target, Some(RenderTarget::Image(_)))
}

fn layers_token(layers: Option<&RenderLayers>) -> String {
    match layers {
        None => "default".to_string(),
        Some(layers) => {
            let mut parts: Vec<String> = layers.iter().map(|layer| layer.to_string()).collect();
            if parts.is_empty() {
                parts.push("none".to_string());
            }
            parts.join("+")
        }
    }
}

/// The visual-quality tier in force, and the render budgets that follow.
///
/// The tier is the largest single factor in render cost: one room draws 23x
/// more sprite area at `high` than at `potato`. Without this row, two
/// captures at different tiers look like the same experiment.
///
/// One row at 1 Hz, because the tier is configuration, not a population.
pub fn report_visual_quality_census(
    census: Res<RuntimeCensus>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    // The frame cap sets the frame time. `FramePaceCap::Auto` is the default
    // (caps to display refresh), so captures are paced unless it is turned
    // off. Recorded so paced and unpaced captures are not compared.
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    // The present mode limits it first. Bevy's default `Fifo` (v-sync) caps
    // the frame rate at the refresh rate, and a missed refresh costs a whole
    // interval, so "72 fps" at 144 Hz can be an 8 ms frame. Read from the
    // window, which is the state the frames were taken in.
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let Some(at) = census.due() else {
        return;
    };
    let present_mode = window
        .single()
        .map(|window| format!("{:?}", window.present_mode))
        .unwrap_or_else(|_| "none".to_string());
    // The two asset experiment knobs, as the process saw them, so captures
    // with and without them are not grouped.
    let images_render_world_only = ambition_sprite_sheet::game_assets::images_render_world_only();
    let upload_mb_per_frame = std::env::var("AMBITION_RENDER_ASSET_MB_PER_FRAME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "unlimited".to_string());
    let frame_cap = settings
        .as_deref()
        .map(|settings| format!("{:?}", settings.video.frame_cap))
        .unwrap_or_else(|| "unrecorded".to_string());
    let Some(quality) = quality else {
        // Report, do not skip. No quality resource is a real state (headless, no
        // renderer), and a missing row would look like "the tier did not matter".
        eprintln!(
            "[census] visual_quality t={at:.3} profile=none frame_cap={frame_cap} \
             present_mode={present_mode} images_render_world_only={images_render_world_only} \
             upload_mb_per_frame={upload_mb_per_frame}"
        );
        return;
    };
    let budget = &quality.budget;
    eprintln!(
        "[census] visual_quality t={at:.3} profile={:?} parallax_enabled={} \
         parallax_max_layers={} parallax_resolution={:?} msaa_samples={} \
         max_scale_factor={} frame_cap={} present_mode={} images_render_world_only={} \
         upload_mb_per_frame={}",
        quality.profile,
        budget.parallax.enabled,
        budget
            .parallax
            .max_layers
            .map(|n| n.to_string())
            .unwrap_or_else(|| "unbounded".to_string()),
        budget.parallax.resolution_scale,
        // The sanitized count, not the requested one. `sanitized_msaa_samples`
        // rounds down to a tier Bevy supports (3 -> 2, 5 -> 4, 16 -> 8) and is
        // what `ambition_render::quality` configures. Logging the raw field would
        // name an arm that never ran. D-RASTER-3 uses this line to tell MSAA arms
        // apart.
        budget.raster.sanitized_msaa_samples(),
        budget
            .raster
            .max_scale_factor
            .map(|s| format!("{s}"))
            .unwrap_or_else(|| "compositor".to_string()),
        frame_cap,
        present_mode,
        images_render_world_only,
        upload_mb_per_frame,
    );
}

#[allow(clippy::type_complexity)]
/// How many cameras exist at all.
pub const CAMERAS: DiagnosticPath = DiagnosticPath::const_new("ambition/render/cameras");

/// How many active cameras render the world.
///
/// This answers "why is this frame expensive". Four world-rendering cameras
/// draw the world four times, which a frame time alone does not show.
pub const WORLD_DRAWS: DiagnosticPath = DiagnosticPath::const_new("ambition/render/world_draws");

/// How many active cameras draw into an offscreen image rather than the display.
pub const OFFSCREEN_TARGETS: DiagnosticPath =
    DiagnosticPath::const_new("ambition/render/offscreen_targets");

/// Register the render-population diagnostics and keep them fed.
///
/// Not gated on `AMBITION_PROFILE_CENSUS`, like the ECS publisher: that
/// variable gates a stderr printer, and a developer turns on F1 without a
/// restart. The cost is one walk of a single-digit camera list per frame.
pub struct RenderDiagnosticsPublishPlugin;

impl Plugin for RenderDiagnosticsPublishPlugin {
    fn build(&self, app: &mut App) {
        app.register_diagnostic(Diagnostic::new(CAMERAS).with_suffix(" cameras"))
            .register_diagnostic(Diagnostic::new(WORLD_DRAWS).with_suffix(" world draws"))
            .register_diagnostic(Diagnostic::new(OFFSCREEN_TARGETS).with_suffix(" offscreen"))
            .add_systems(Update, publish_view_diagnostics);
    }
}

/// Walks the cameras itself instead of reusing `report_view_census`'s loop,
/// which is tied to its per-camera row. Both use `classify_camera`, so the
/// rule for "renders the world" has one source.
fn publish_view_diagnostics(
    mut diagnostics: Diagnostics,
    cameras: Query<(
        Entity,
        &Camera,
        Option<&RenderTarget>,
        Option<&PresentsView>,
        Has<MainCamera>,
        Has<FrontHudCamera>,
    )>,
    #[cfg(feature = "portal_render")] portal_rigs: Query<
        (),
        With<ambition_portal2d_presentation::PortalViewRig>,
    >,
) {
    let mut total = 0usize;
    let mut world_rendering = 0usize;
    let mut offscreen = 0usize;
    for (entity, camera, target, presents, is_main, is_hud) in &cameras {
        total += 1;
        if !camera.is_active {
            // An inactive camera still costs its extract but not a pass, and these
            // paths count passes.
            continue;
        }
        let draws_offscreen = target_is_image(target);
        #[cfg(feature = "portal_render")]
        let is_portal_rig = portal_rigs.get(entity).is_ok();
        #[cfg(not(feature = "portal_render"))]
        let is_portal_rig = {
            let _ = entity;
            false
        };
        let role = classify_camera(
            is_portal_rig,
            is_hud,
            is_main,
            presents.is_some(),
            draws_offscreen,
        );
        if role.renders_world() {
            world_rendering += 1;
        }
        if draws_offscreen {
            offscreen += 1;
        }
    }
    diagnostics.add_measurement(&CAMERAS, || total as f64);
    diagnostics.add_measurement(&WORLD_DRAWS, || world_rendering as f64);
    diagnostics.add_measurement(&OFFSCREEN_TARGETS, || offscreen as f64);
}

/// Per-camera rows plus the rollup that summarizes them.
///
/// The population is cameras, not entities: under a dozen even in a large
/// scene, so a per-row line at 1 Hz is cheap.
pub fn report_view_census(
    census: Res<RuntimeCensus>,
    cameras: Query<(
        Entity,
        &Camera,
        Option<&RenderTarget>,
        Option<&Name>,
        Option<&RenderLayers>,
        Option<&PresentsView>,
        Has<MainCamera>,
        Has<FrontHudCamera>,
    )>,
    #[cfg(feature = "portal_render")] portal_rigs: Query<
        (),
        With<ambition_portal2d_presentation::PortalViewRig>,
    >,
    views: Query<&LocalViewId, With<LocalView>>,
    device: Option<Res<bevy::render::renderer::RenderDevice>>,
) {
    let Some(at) = census.due() else {
        return;
    };

    let mut total = 0usize;
    let mut active = 0usize;
    let mut world_rendering = 0usize;
    let mut offscreen = 0usize;

    for (entity, camera, target, name, layers, presents, is_main, is_hud) in &cameras {
        total += 1;
        // An inactive camera still costs its extract; it does not cost a pass.
        // Both facts are on the row so a reader can tell which population a
        // count belongs to.
        if camera.is_active {
            active += 1;
        }
        let draws_offscreen = target_is_image(target);
        #[cfg(feature = "portal_render")]
        let is_portal_rig = portal_rigs.get(entity).is_ok();
        #[cfg(not(feature = "portal_render"))]
        let is_portal_rig = false;
        let role = classify_camera(
            is_portal_rig,
            is_hud,
            is_main,
            presents.is_some(),
            draws_offscreen,
        );
        if camera.is_active && role.renders_world() {
            world_rendering += 1;
        }
        if camera.is_active && draws_offscreen {
            offscreen += 1;
        }
        let size = camera
            .physical_target_size()
            .map(|size| format!("{}x{}", size.x, size.y))
            .unwrap_or_else(|| "unknown".to_string());
        let viewport = camera
            .viewport
            .as_ref()
            .map(|viewport| {
                format!(
                    "{}x{}+{}+{}",
                    viewport.physical_size.x,
                    viewport.physical_size.y,
                    viewport.physical_position.x,
                    viewport.physical_position.y
                )
            })
            .unwrap_or_else(|| "full".to_string());
        eprintln!(
            "[census] camera t={at:.3} entity={entity} role={} active={} target={} size={size} \
             viewport={viewport} order={} layers={} presents_view={} name={:?}",
            role.as_str(),
            camera.is_active,
            target_kind(target),
            camera.order,
            layers_token(layers),
            presents.map(|p| format!("{}", p.0)).unwrap_or_default(),
            name.map(Name::as_str).unwrap_or("<unnamed>"),
        );
    }

    eprintln!(
        "[census] views t={at:.3} cameras={total} active={active} world_rendering={world_rendering} \
         offscreen={offscreen} local_views={}",
        views.iter().count(),
    );

    // Warn here, beside the evidence: the phase split is not trustworthy
    // while anything renders.
    //
    // `[census] phases` attributes wall time between schedule markers. When
    // the render path blocks the main thread (submission, readback, a
    // software rasterizer), whichever phase brackets that moment absorbs the
    // time. For example, a larger render target made `StateTransition` grow
    // with pixel count. `fragment_shader_invocations = 0` does not make it
    // safe: submission and upscaling still cost time.
    //
    // The camera count alone is not the test. `NoWindow` mode sets
    // `backends: None`, which omits the RenderApp and draws nothing, but still
    // reports `world_rendering=1`. `RenderDevice` reaches the main world only
    // when the renderer initialized, so it is the test for a GPU.
    let gpu = device.is_some();
    if gpu && (world_rendering > 0 || offscreen > 0) {
        eprintln!(
            "[census] phases_warning t={at:.3} untrustworthy=render_blocking \
             world_rendering={world_rendering} offscreen={offscreen} — `[census] phases` \
             attributes wall time between markers, so GPU blocking lands in whichever \
             phase brackets it. Trust phase splits only from a run with no rendering."
        );
    } else if !gpu {
        // Report the positive case too. "No warning" looks the same as "the
        // check never ran".
        eprintln!(
            "[census] phases_trust t={at:.3} trustworthy=no_render_backend \
             world_rendering={world_rendering} offscreen={offscreen} — no `RenderDevice` in \
             the main world, so nothing is drawn and `[census] phases` is not absorbing \
             GPU time. Phase splits from this run are usable."
        );
    }
}

/// The draw population: how much exists, how much is visible, and how much
/// area it covers. A big gap between the first two is work the renderer
/// discarded; a large area against a small visible count is overdraw.
///
/// The area columns identify which entities produce overdraw; counts alone
/// cannot (76 visible sprites make 41M fragments only if some cover the
/// viewport).
///
/// Areas are in world units, not pixels. Pixels need each sprite's view and
/// projection, a per-view question this per-world pass does not answer. The
/// ratio of `sprite_area` to `sprite_area_max` finds a few full-screen panels
/// among many small sprites, and a ratio does not depend on the unit.
///
/// `sprite_area_unsized` is part of the reading. A `Sprite` with no
/// `custom_size` takes its extent from its image, which this pass cannot
/// resolve without the asset store. Those sprites are counted and excluded. A
/// large `unsized` means the area columns are a floor.
#[allow(clippy::type_complexity)]
pub fn report_draw_census(
    census: Res<RuntimeCensus>,
    sprites: Query<(
        Option<&ViewVisibility>,
        &Sprite,
        &GlobalTransform,
        Option<&RenderLayers>,
    )>,
    texts: Query<(), With<Text2d>>,
    projections: Query<(), With<PresentedForView>>,
) {
    let Some(at) = census.due() else {
        return;
    };
    let mut sprite_total = 0usize;
    let mut sprite_visible = 0usize;
    let mut area_total = 0.0f32;
    let mut area_max = 0.0f32;
    let mut unsized_visible = 0usize;
    // Area split by semantic layer, so a reader can tell which coverage to
    // cut. It is a count, not a timing, so it is the same on llvmpipe and a
    // real GPU and can be gathered anywhere (unlike D-RASTER-3 timings).
    // Values are world units, like `sprite_area`. Compare a layer's share of
    // the total; do not divide by a pixel count.
    let mut area_world = 0.0f32;
    let mut area_hud = 0.0f32;
    let mut area_parallax = 0.0f32;
    let mut area_local = 0.0f32;
    let mut area_other = 0.0f32;
    for (visibility, sprite, transform, layers) in &sprites {
        sprite_total += 1;
        if !visibility.is_some_and(|visible| visible.get()) {
            continue;
        }
        sprite_visible += 1;
        // Do not size an unsized sprite from its image. The parallax backdrop
        // is spawned with `custom_size: None` on purpose (see
        // `rendering/parallax.rs`), and `sync_parallax_layers` later sizes it to
        // the owning view. Its final size is not its image's: a 512x512 image
        // filling a 1280x720 viewport covers 921,600 pixels, not 262,144. So it
        // stays skipped, and `sprite_area_unsized` flags the area as a floor.
        let Some(size) = sprite.custom_size else {
            unsized_visible += 1;
            continue;
        };
        // The drawn quad is the size times the transform scale; `abs` because a
        // mirrored sprite covers the same area.
        let scale = transform.scale();
        let area = (size.x * scale.x).abs() * (size.y * scale.y).abs();
        area_total += area;
        if area > area_max {
            area_max = area;
        }
        // The lowest layer a sprite draws on names it. A sprite on several
        // layers is drawn by several cameras (overdraw), so this is a floor for
        // that case; `sprite_area` stays the ungrouped total to check against.
        let lowest = layers
            .map(|l| l.iter().min().unwrap_or(usize::MAX))
            .unwrap_or(0);
        match lowest {
            0 => area_world += area,
            FRONT_HUD_LAYER => area_hud += area,
            PARALLAX_BACKGROUND_LAYER => area_parallax += area,
            n if n >= LOCAL_VIEW_RENDER_LAYER_BASE => area_local += area,
            _ => area_other += area,
        }
    }
    eprintln!(
        "[census] draws t={at:.3} sprites={sprite_total} sprites_visible={sprite_visible} \
         text2d={} per_view_projections={} sprite_area={area_total:.0} \
         sprite_area_max={area_max:.0} sprite_area_unsized={unsized_visible} \
         area_world={area_world:.0} area_parallax={area_parallax:.0} \
         area_hud={area_hud:.0} area_local={area_local:.0} area_other={area_other:.0}",
        texts.iter().count(),
        projections.iter().count(),
    );
}

/// How much presentation state is rewritten each frame versus how much exists.
///
/// This measures whether a sim-to-presentation projection rewrites state that
/// did not change; Bevy's extraction then pays for that churn again.
///
/// It counts what Bevy sees as changed, not what differs. `Changed<T>` is set
/// by any `DerefMut`, so identical writes count as changes. A low number
/// clears the projection; a high number is only a suspicion. `changed ==
/// total` on a still scene is the sign. Read it on a scene with hundreds of
/// sprites, not two bodies.
pub fn report_presentation_churn_census(
    census: Res<RuntimeCensus>,
    transforms: Query<(), With<Transform>>,
    transforms_changed: Query<(), Changed<Transform>>,
    sprites: Query<(), With<Sprite>>,
    sprites_changed: Query<(), Changed<Sprite>>,
    visibility: Query<(), With<Visibility>>,
    visibility_changed: Query<(), Changed<Visibility>>,
) {
    let Some(at) = census.due() else {
        return;
    };
    eprintln!(
        "[census] churn t={at:.3} transforms={} transforms_changed={} sprites={} \
         sprites_changed={} visibility={} visibility_changed={}",
        transforms.iter().count(),
        transforms_changed.iter().count(),
        sprites.iter().count(),
        sprites_changed.iter().count(),
        visibility.iter().count(),
        visibility_changed.iter().count(),
    );
}

/// Offscreen render targets and the memory they hold.
///
/// Growth here across room transitions is a leak that a frame-time graph
/// cannot show: capture textures that were replaced but never dropped keep
/// their bytes until VRAM runs out.
pub fn report_render_target_census(
    census: Res<RuntimeCensus>,
    cameras: Query<&RenderTarget, With<Camera>>,
    images: Res<Assets<Image>>,
) {
    let Some(at) = census.due() else {
        return;
    };
    let mut targets = 0usize;
    let mut bytes = 0u64;
    let mut widest = 0u32;
    for render_target in &cameras {
        let RenderTarget::Image(target) = render_target else {
            continue;
        };
        targets += 1;
        if let Some(image) = images.get(&target.handle) {
            widest = widest.max(image.width().max(image.height()));
            bytes += image.data.as_ref().map_or(0, |data| data.len() as u64);
        }
    }
    eprintln!(
        "[census] render_targets t={at:.3} image_targets={targets} cpu_bytes={bytes} \
         largest_dim={widest} images_resident={}",
        images.len(),
    );
}

/// Portal capture workload: how many rigs exist, how many are live, and the
/// budget that is supposed to bound them.
///
/// The budget is on the row because a rig count alone cannot say whether the
/// cost is expected: two rigs under a two-capture budget is the design; two
/// rigs refreshing every frame under a one-per-frame budget is a bug.
#[cfg(feature = "portal_render")]
pub fn report_portal_census(
    census: Res<RuntimeCensus>,
    rigs: Query<&ambition_portal2d_presentation::PortalViewRig>,
    active: Query<&Camera, With<ambition_portal2d_presentation::PortalViewRig>>,
    config: Option<Res<ambition_portal2d_presentation::PortalViewConeConfig>>,
    quality: Option<Res<ambition_portal2d_presentation::PortalCaptureQualityBudget>>,
) {
    let Some(at) = census.due() else {
        return;
    };
    let total = rigs.iter().count();
    let live = active.iter().filter(|camera| camera.is_active).count();
    let budget = match (config.as_deref(), quality.as_deref()) {
        (Some(config), Some(quality)) => {
            Some(ambition_portal2d_presentation::effective_portal_capture_budget(config, quality))
        }
        _ => None,
    };
    match budget {
        Some(budget) => eprintln!(
            "[census] portal t={at:.3} rigs={total} active={live} max_resolution={} \
             recursion_depth={} max_active_captures={} max_updates_per_frame={} \
             min_refresh_interval_s={:.3} include_parallax={}",
            budget.max_resolution,
            budget.recursion_depth,
            budget.max_active_captures,
            budget.max_updates_per_frame,
            budget.min_refresh_interval_s,
            budget.include_parallax,
        ),
        None => {
            eprintln!("[census] portal t={at:.3} rigs={total} active={live} budget=unavailable")
        }
    }
}

/// Bevy's own render diagnostics, one row per measured span.
///
/// `RenderDiagnosticsPlugin` always records `render/<pass>/elapsed_cpu`, and
/// records `elapsed_gpu` and pipeline statistics only where the adapter
/// supports timestamp queries. No GPU rows means the backend could not measure
/// the GPU, so the header row states how many of each kind were found.
pub fn report_render_pass_census(census: Res<RuntimeCensus>, store: Option<Res<DiagnosticsStore>>) {
    let Some(at) = census.due() else {
        return;
    };
    let Some(store) = store else {
        eprintln!("[census] render_pass_summary t={at:.3} status=no_diagnostics_store");
        return;
    };
    let mut cpu_rows = 0usize;
    let mut gpu_rows = 0usize;
    let mut stat_rows = 0usize;
    for diagnostic in store.iter() {
        let path = diagnostic.path().as_str();
        if !path.starts_with("render/") {
            continue;
        }
        let Some(value) = diagnostic.value() else {
            continue;
        };
        if path.ends_with("/elapsed_cpu") {
            cpu_rows += 1;
        } else if path.ends_with("/elapsed_gpu") {
            gpu_rows += 1;
        } else {
            stat_rows += 1;
        }
        eprintln!(
            "[census] render_pass t={at:.3} path={path} value={value:.6} avg={:.6} suffix={}",
            diagnostic.average().unwrap_or(value),
            diagnostic.suffix,
        );
    }
    eprintln!(
        "[census] render_pass_summary t={at:.3} cpu_spans={cpu_rows} gpu_spans={gpu_rows} \
         pipeline_stat_spans={stat_rows}"
    );
}

/// Cumulative asset decode work, sampled on the census clock.
///
/// The `[image-census]` line reports a five-second delta; this row reports
/// the running total on the shared clock, so a room's decode cost is a
/// subtraction between two rows.
pub fn report_asset_census(
    census: Res<RuntimeCensus>,
    images: Option<Res<crate::asset_census::ImageCensus>>,
    image_assets: Res<Assets<Image>>,
    // The HUD's retained images. `loads` rising while `hits` stays flat means
    // a reopened screen is re-decoding what it had.
    hud_images: Option<Res<crate::hud::declared::RetainedHudImages>>,
) {
    let Some(at) = census.due() else {
        return;
    };
    match images {
        Some(images) => eprintln!(
            "[census] assets t={at:.3} decoded_images={} decoded_megapixels={:.1} \
             decoded_bytes={} derived_byte_images={} images_resident={} \
             hud_image_hits={} hud_image_loads={}",
            images.total_images(),
            images.total_megapixels(),
            images.total_bytes(),
            // How much of `decoded_bytes` was derived from the texture descriptor
            // because the CPU copy was dropped. The total is then not purely
            // measured.
            images.derived_byte_images(),
            image_assets.len(),
            // `unavailable`, not `0`, when the cache is not installed. Otherwise a
            // composition without the declared HUD prints `hits=0 loads=0`, which
            // looks like a cache that is present and unused.
            hud_images.as_ref().map_or_else(
                || "unavailable".to_string(),
                |c| c.hits_and_loads().0.to_string()
            ),
            hud_images.as_ref().map_or_else(
                || "unavailable".to_string(),
                |c| c.hits_and_loads().1.to_string()
            ),
        ),
        None => eprintln!(
            "[census] assets t={at:.3} decoded_images=unavailable images_resident={}",
            image_assets.len()
        ),
    }
}

/// Install the presentation-side censuses.
///
/// Adds `RenderDiagnosticsPlugin` when the render app exists and nothing has
/// added it. A `--features profile` build gets it from `bevy_render`; this
/// gives other profiling builds the same per-pass rows.
pub struct PresentationCensusPlugin;

impl Plugin for PresentationCensusPlugin {
    fn build(&self, app: &mut App) {
        // Registered only when the census is enabled. See
        // `ambition_dev_tools::runtime_census`. These systems have no place in a
        // shipped frame's schedule.
        if RuntimeCensus::from_env().enabled() {
            app.add_systems(
                Last,
                (
                    report_view_census,
                    report_draw_census,
                    report_visual_quality_census,
                    report_render_target_census,
                    report_render_pass_census,
                    report_asset_census,
                    report_presentation_churn_census,
                ),
            );
            #[cfg(feature = "portal_render")]
            app.add_systems(Last, report_portal_census);
        }

        // `bevy_render` installs this under `bevy/trace_tracy` (`--features
        // profile`), and adding it twice panics. Otherwise it is absent, so add
        // it when the census is on and nobody else has. Read the environment,
        // not the resource, so this does not depend on plugin order.
        if RuntimeCensus::from_env().enabled()
            && !app.is_plugin_added::<bevy::render::diagnostic::RenderDiagnosticsPlugin>()
        {
            app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_most_specific_marker_wins() {
        // A portal rig also has an image target; without this rule it reads as
        // a plain offscreen camera.
        assert_eq!(
            classify_camera(true, false, false, false, true),
            CameraRole::PortalCapture
        );
        // The HUD camera is a Camera2d on a window target, like the gameplay
        // camera; only its marker separates them.
        assert_eq!(
            classify_camera(false, true, false, false, false),
            CameraRole::Hud
        );
        assert_eq!(
            classify_camera(false, false, true, false, false),
            CameraRole::MainGameplay
        );
        assert_eq!(
            classify_camera(false, false, true, true, false),
            CameraRole::LocalView
        );
        assert_eq!(
            classify_camera(false, false, false, false, true),
            CameraRole::Offscreen
        );
        assert_eq!(
            classify_camera(false, false, false, false, false),
            CameraRole::Other
        );
    }

    #[test]
    fn only_world_drawing_roles_count_toward_repeated_world_rendering() {
        // The count answers "how many times is this world drawn this frame". A
        // HUD overlay is not a draw of it.
        assert!(CameraRole::MainGameplay.renders_world());
        assert!(CameraRole::LocalView.renders_world());
        assert!(CameraRole::PortalCapture.renders_world());
        assert!(!CameraRole::Hud.renders_world());
        assert!(!CameraRole::Offscreen.renders_world());
        assert!(!CameraRole::Other.renders_world());
    }
}
