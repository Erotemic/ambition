//! Profiling-only presentation census: the views, targets and draw population
//! Ambition hands the renderer.
//!
//! `perf` can prove the renderer was hot and Tracy can name the pass; neither
//! can say the frame carried three world-rendering cameras and two portal
//! captures. These rows do, on the shared clock in
//! [`ambition_dev_tools::runtime_census`], so a slow interval reads against the
//! scene that produced it.
//!
//! Rows written here (one line each, `[census] <kind> t=<seconds> k=v ...`):
//!
//! - `views` — the per-frame rollup: how many cameras, how many active, how
//!   many draw the world, how many draw offscreen.
//! - `camera` — ONE row per active camera: identity, semantic role, target,
//!   resolution, order, render layers, and the view it presents.
//! - `draws` — sprite / text / mesh population and how much of it is visible.
//! - `portal` — capture rigs, their budget, and the resolution they capture at.
//! - `render_pass` — Bevy's own render diagnostics, when the backend supplies
//!   them.
//!
//! Everything is behind the same gate: without `AMBITION_PROFILE_CENSUS` each
//! system is one bool test per frame, and no per-entity iteration happens on a
//! frame that is not a sample frame.

use bevy::camera::visibility::{RenderLayers, ViewVisibility};
use bevy::camera::RenderTarget;
use bevy::diagnostic::DiagnosticsStore;
use bevy::prelude::*;

use ambition_dev_tools::runtime_census::RuntimeCensus;
use ambition_platformer2d_shared_tangle::camera_layers::{FrontHudCamera, MainCamera};
use ambition_sim_view::{LocalView, LocalViewId, PresentedForView, PresentsView};

/// What a camera is FOR, as far as the composition can say.
///
/// Roles are read off the markers the spawner already sets, not guessed from
/// geometry. A camera whose owner set no marker lands in [`Self::Other`] and
/// still reports its `Name`, which is the honest answer — better than a
/// confident inference from a component set that never meant to say this.
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

    /// Whether this role draws the simulated world, as opposed to overlaying
    /// it. The count of these is the number the "is the same world being
    /// rendered repeatedly" question turns on.
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

/// `RenderTarget` is a component, not a camera field: a camera without one
/// draws to the primary window, which is the common case and must not read as
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

/// Per-camera rows plus the rollup that summarizes them.
///
/// The population is cameras, not entities: a scene with a hundred thousand
/// sprites still has under a dozen of these, so a per-row line at 1 Hz is
/// cheaper than the rollup it feeds.
#[allow(clippy::type_complexity)]
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

    // ⛔⛔ THE PHASE SPLIT IS NOT TRUSTWORTHY WHILE ANYTHING RENDERS, and the
    // warning is emitted HERE — beside the evidence, in the same log a reader is
    // already scrolling — rather than left in a document nobody opens mid-run.
    //
    // `[census] phases` attributes WALL TIME between schedule markers. When the
    // render path blocks the main thread (submission, readback, a software
    // rasterizer), whichever phase happens to bracket that moment absorbs it.
    // Measured 2026-08-29: raising the render target from 320x240 to 1280x960
    // took `StateTransition` from 0.169ms to 1.822ms — a phase full of state
    // machinery, scaling with PIXELS — and every other phase moved with it. A
    // whole "StateTransition is 14% of a real room's frame" finding was built on
    // that and had to be retracted.
    //
    // ⚠ `fragment_shader_invocations = 0` DOES NOT MAKE IT SAFE: submission and
    // upscaling cost real time even when the opaque pass shades nothing.
    //
    // ⭐⭐ AND THE CAMERA COUNT IS THE WRONG QUESTION ON ITS OWN. A camera that
    // TARGETS the world is not the same as a render path that RUNS: the
    // `NoWindow` mode sets `backends: None`, which omits the RenderApp entirely
    // and draws nothing, yet still reports `world_rendering=1`. Warning on the
    // camera count alone therefore condemned windowless runs whose phase splits
    // are perfectly sound — measured 2026-08-29, when it talked its own author
    // out of a valid attribution of a Smash match.
    //
    // `RenderDevice` reaches the MAIN world only when the renderer actually
    // initialized, so it is the honest test for "is there a GPU behind this".
    let gpu = device.is_some();
    if gpu && (world_rendering > 0 || offscreen > 0) {
        eprintln!(
            "[census] phases_warning t={at:.3} untrustworthy=render_blocking \
             world_rendering={world_rendering} offscreen={offscreen} — `[census] phases` \
             attributes wall time between markers, so GPU blocking lands in whichever \
             phase brackets it. Trust phase splits only from a run with no rendering."
        );
    } else if !gpu {
        // ⭐ Say the POSITIVE case too. "No warning" is indistinguishable from
        // "the check never ran", and a reader deciding whether to trust a phase
        // split needs to see that the question was asked and answered.
        eprintln!(
            "[census] phases_trust t={at:.3} trustworthy=no_render_backend \
             world_rendering={world_rendering} offscreen={offscreen} — no `RenderDevice` in \
             the main world, so nothing is drawn and `[census] phases` is not absorbing \
             GPU time. Phase splits from this run are usable."
        );
    }
}

/// The draw population: how much there IS, and how much of it survived
/// visibility. A large gap between the two is work the scene created and the
/// renderer then threw away.
#[allow(clippy::type_complexity)]
pub fn report_draw_census(
    census: Res<RuntimeCensus>,
    sprites: Query<Option<&ViewVisibility>, With<Sprite>>,
    texts: Query<(), With<Text2d>>,
    projections: Query<(), With<PresentedForView>>,
) {
    let Some(at) = census.due() else {
        return;
    };
    let mut sprite_total = 0usize;
    let mut sprite_visible = 0usize;
    for visibility in &sprites {
        sprite_total += 1;
        if visibility.is_some_and(|visible| visible.get()) {
            sprite_visible += 1;
        }
    }
    eprintln!(
        "[census] draws t={at:.3} sprites={sprite_total} sprites_visible={sprite_visible} \
         text2d={} per_view_projections={}",
        texts.iter().count(),
        projections.iter().count(),
    );
}

/// How much presentation state is REWRITTEN each frame versus how much exists.
///
/// ⭐⭐ THE PROJECTION QUESTION, MADE MEASURABLE. A campaign measurement put
/// `ambition_render` at 99 systems in `Update` — the largest owner of the one
/// phase that is both ours and unexplained — and the open charge against a
/// simulation-to-presentation projection is that it rewrites state which did
/// not semantically change. Bevy's own extraction then pays for that churn
/// again downstream.
///
/// ⛔ IT COUNTS WHAT BEVY WILL BELIEVE, NOT WHAT ACTUALLY DIFFERS. `Changed<T>`
/// is set by any `DerefMut`, so a projection that writes an identical value
/// every frame reports as changed here — which is exactly the defect being
/// looked for, and exactly why a low number is a real acquittal while a high
/// number is only a suspicion. `changed == total` on a scene standing still is
/// the tell.
///
/// ⚠ Two bodies do not make a case either way. The number to watch is the ratio
/// on a scene with hundreds of sprites, which is the workload the whole campaign
/// was opened about.
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
/// Growth here across room transitions is the leak shape a frame-time graph
/// cannot show: capture textures that were replaced but never dropped keep
/// their bytes and stop being drawn, so nothing gets visibly worse until VRAM
/// runs out.
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
/// cost is expected: two rigs under a two-capture budget is the design, two
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
/// `RenderDiagnosticsPlugin` records `render/<pass>/elapsed_cpu` always, and
/// `elapsed_gpu` plus pipeline statistics only where the adapter supports
/// timestamp queries. A run that reports CPU rows and no GPU rows is a run
/// whose backend could not measure the GPU — the absence is a MEASUREMENT, so
/// the header row below states how many of each kind were found rather than
/// leaving a reader to wonder whether the pass was free.
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
/// The always-on `[image-census]` line reports a five-second delta; this row
/// reports the RUNNING TOTAL on the shared clock, so "did entering that room
/// decode another 200 MB" is a subtraction between two rows rather than a sum
/// over a log.
pub fn report_asset_census(
    census: Res<RuntimeCensus>,
    images: Option<Res<crate::asset_census::ImageCensus>>,
    image_assets: Res<Assets<Image>>,
) {
    let Some(at) = census.due() else {
        return;
    };
    match images {
        Some(images) => eprintln!(
            "[census] assets t={at:.3} decoded_images={} decoded_megapixels={:.1} \
             decoded_bytes={} images_resident={}",
            images.total_images(),
            images.total_megapixels(),
            images.total_bytes(),
            image_assets.len(),
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
/// added it already — a `--features profile` build gets it from `bevy_render`
/// itself, so this is the path that gives a non-Tracy profiling build the same
/// per-pass rows.
pub struct PresentationCensusPlugin;

impl Plugin for PresentationCensusPlugin {
    fn build(&self, app: &mut App) {
        // ⛔ Registered only when asked — see the note in
        // `ambition_dev_tools::runtime_census`. `due_at` is only set while the
        // census is enabled, so these could never have reported when off; they
        // simply had no business being in a shipped frame's schedule.
        if RuntimeCensus::from_env().enabled() {
            app.add_systems(
                Last,
                (
                    report_view_census,
                    report_draw_census,
                    report_render_target_census,
                    report_render_pass_census,
                    report_asset_census,
                    report_presentation_churn_census,
                ),
            );
            #[cfg(feature = "portal_render")]
            app.add_systems(Last, report_portal_census);
        }

        // `bevy_render` installs this itself under `bevy/trace_tracy`, which is
        // what `--features profile` turns on; adding it a second time panics.
        // Without that feature it is absent, and a profiling run still wants
        // per-pass timings -- so add it exactly when the census is on and
        // nobody else has. Read the environment rather than the resource: this
        // must not depend on whether the sim half of the census was built
        // first.
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
        // A portal rig also has an image target and would otherwise read as a
        // plain offscreen camera, losing the one fact that explains its cost.
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
        // The question the count answers is "how many times is this world
        // being drawn this frame" — a HUD overlay is not another draw of it.
        assert!(CameraRole::MainGameplay.renders_world());
        assert!(CameraRole::LocalView.renders_world());
        assert!(CameraRole::PortalCapture.renders_world());
        assert!(!CameraRole::Hud.renders_world());
        assert!(!CameraRole::Offscreen.renders_world());
        assert!(!CameraRole::Other.renders_world());
    }
}
