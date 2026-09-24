//! Live resolved visual-quality resource.
//!
//! Settings persist the user's profile/custom table in gameplay-core. The render
//! side mirrors that into one resource every visual subsystem can read.

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::view::Msaa;

use ambition_persistence::settings::{DetectedGpuClass, UserSettings, VisualQualityProfile};

pub use ambition_persistence::settings::ResolvedVisualQuality;

/// The resource and the system that keeps it current, together.
///
/// `sync_resolved_visual_quality` is the only reader of `UserSettings` into
/// `ResolvedVisualQuality`, so it must be installed wherever the resource is.
///
/// Idempotent (`is_unique() -> false` plus a marker), because two plugins
/// need it: `PlatformerPresentationPlugin` for a demo, and
/// `SessionRoomVisualsPlugin` for the shipped host, which adds only that one.
pub struct VisualQualityPlugin;

/// Present once [`VisualQualityPlugin`] has built.
#[derive(Resource)]
struct VisualQualityInstalled;

impl Plugin for VisualQualityPlugin {
    fn is_unique(&self) -> bool {
        false
    }

    fn build(&self, app: &mut App) {
        if app.world().contains_resource::<VisualQualityInstalled>() {
            return;
        }
        app.insert_resource(VisualQualityInstalled);
        log_quality_profile_override();
        app.init_resource::<ResolvedVisualQuality>();
        // Once, in `PostStartup`, after the settings file loads.
        // `load_settings_at_startup` runs in `Startup` and replaces the whole
        // `UserSettings` resource with the file's contents. Seeding earlier would
        // be overwritten, and the file's `hardware_seeded: false` restored, so an
        // existing install would never be seeded. `PostStartup` still precedes the
        // first `Update`, so the pair below reads the seeded tier before frame
        // one.
        app.add_systems(PostStartup, seed_visual_quality_from_adapter);
        app.add_systems(
            Update,
            (sync_resolved_visual_quality, sync_raster_budget).chain(),
        );
        // This bridge reads visual quality and writes portal presentation
        // quality, so register it only when the destination resource exists.
        #[cfg(feature = "portal_render")]
        app.add_systems(
            Update,
            sync_portal_quality_budget.run_if(
                resource_exists::<ambition_portal2d_presentation::PortalCaptureQualityBudget>,
            ),
        );
    }
}

/// Translate the graphics API's adapter class into the tier policy's own
/// vocabulary.
///
/// This function only translates. Which tier a hardware class starts on is
/// decided in `ambition_persistence`, beside the tiers, where it is testable
/// without a GPU. This is the only place that names `wgpu::DeviceType`.
fn detected_gpu_class(device_type: wgpu::DeviceType) -> DetectedGpuClass {
    match device_type {
        wgpu::DeviceType::DiscreteGpu => DetectedGpuClass::Discrete,
        wgpu::DeviceType::IntegratedGpu => DetectedGpuClass::Integrated,
        wgpu::DeviceType::VirtualGpu => DetectedGpuClass::Virtual,
        wgpu::DeviceType::Cpu => DetectedGpuClass::Cpu,
        wgpu::DeviceType::Other => DetectedGpuClass::Other,
    }
}

/// Seed the visual quality tier from the renderer's adapter, once, on a
/// profile the player has not changed.
///
/// A detected default is a first-run seed, never a per-boot override:
/// deciding every launch would undo the settings menu. Both guards are in
/// [`VisualQualitySettings::seed_from_hardware`] (a persisted
/// `hardware_seeded` flag, and the profile still at the default), so this
/// system only supplies the adapter class and reports the result.
///
/// Needed because `default_visual_quality_profile()` decides by target OS, so
/// every desktop starts at `High`, even on an integrated GPU (Intel HD 630
/// measured p50 51 ms at High).
///
/// Every parameter is optional. A headless composition has no
/// `RenderAdapterInfo` and a fixture may have no `UserSettings`; a `Res` that
/// matches nothing panics in parameter validation.
pub fn seed_visual_quality_from_adapter(
    adapter: Option<Res<bevy::render::renderer::RenderAdapterInfo>>,
    settings: Option<ResMut<UserSettings>>,
) {
    let (Some(adapter), Some(mut settings)) = (adapter, settings) else {
        return;
    };
    if settings.video.quality.hardware_seeded {
        return;
    }
    let class = detected_gpu_class(adapter.device_type);
    let name = adapter.name.clone();
    match settings.video.quality.seed_from_hardware(class) {
        Some(profile) => info!(
            "visual quality seeded to `{}` for a {:?} adapter ({name}); \
             this is a FIRST-RUN default and the settings menu owns it from here",
            profile.label(),
            class,
        ),
        None => debug!("visual quality left as-is for a {class:?} adapter ({name})"),
    }
}

/// Log once, at startup, that a forced profile is active. Also log when the
/// value is set but not understood, which would otherwise look like the
/// override working.
fn log_quality_profile_override() {
    let Ok(raw) = std::env::var(ambition_persistence::settings::QUALITY_PROFILE_ENV) else {
        return;
    };
    if raw.trim().is_empty() {
        return;
    }
    match VisualQualityProfile::from_label(&raw) {
        Some(profile) => info!(
            "visual quality forced to `{}` by {}; the settings menu cannot change it this run",
            profile.label(),
            ambition_persistence::settings::QUALITY_PROFILE_ENV,
        ),
        None => warn!(
            "{}={raw:?} is not a profile; using the saved setting instead. \
             Expected one of: potato, low, medium, high, ultra",
            ambition_persistence::settings::QUALITY_PROFILE_ENV,
        ),
    }
}

pub fn sync_resolved_visual_quality(
    settings: Option<Res<UserSettings>>,
    mut resolved: ResMut<ResolvedVisualQuality>,
) {
    let Some(settings) = settings else {
        return;
    };
    let next = ResolvedVisualQuality::from_settings(&settings);
    if *resolved != next {
        *resolved = next;
    }
}

#[cfg(feature = "portal_render")]
pub fn sync_portal_quality_budget(
    quality: Res<ResolvedVisualQuality>,
    mut portal_budget: ResMut<ambition_portal2d_presentation::PortalCaptureQualityBudget>,
) {
    let next = ambition_portal2d_presentation::PortalCaptureQualityBudget {
        max_resolution: quality.budget.portal.max_resolution,
        texels_per_world_px: quality.budget.portal.texels_per_world_px,
        recursion_depth: quality.budget.portal.recursion_depth,
        max_active_captures: quality.budget.portal.max_active_captures,
        max_updates_per_frame: quality.budget.portal.max_updates_per_frame,
        min_refresh_interval_s: quality.budget.portal.min_refresh_interval_s,
        include_parallax: quality.budget.portal.include_parallax,
    };
    if *portal_budget != next {
        *portal_budget = next;
    }
}

/// Apply the [`RasterBudget`](ambition_persistence::settings::RasterBudget): the
/// DPI-scale cap on the window, and MSAA on every camera that draws to it.
///
/// These are the two costs that scale with screen area. Other budget knobs
/// trade scene detail; these trade fragments, and without a discrete GPU the
/// fragments are the frame. (Example: a 1600x900 window on a 2x Wayland
/// session rasterized at 3200x1800 on an Intel HD 630, at p50 ~50 ms.)
///
/// Capture cameras are not touched. `ambition_render::capture` sets
/// `Msaa::Off` on the image targets it adopts, and a blanket write would undo
/// it. Only window-target cameras are in scope.
///
/// The scale cap is a request. `set_scale_factor_override` asks winit for a
/// smaller buffer; whether the compositor upscales (wanted) or shrinks the
/// window depends on the platform. To check, read
/// `render/upscaling/fragment_shader_invocations` in a profiling bundle: it
/// equals the framebuffer's pixel count. If capping does not reduce it, the
/// cap did not apply, and the next step is an explicit reduced render target.
pub fn sync_raster_budget(
    quality: Res<ResolvedVisualQuality>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(Entity, &RenderTarget, Option<&Msaa>), With<Camera>>,
    mut commands: Commands,
) {
    let raster = &quality.budget.raster;

    for mut window in &mut windows {
        // Read through `Deref` so an unchanged window is not marked changed;
        // only the assignment below uses `DerefMut`.
        let reported = window.resolution.base_scale_factor();
        let desired = raster.effective_scale_factor(reported);
        if window.resolution.scale_factor_override() != desired {
            window.resolution.set_scale_factor_override(desired);
        }
    }

    let desired = match raster.sanitized_msaa_samples() {
        1 => Msaa::Off,
        2 => Msaa::Sample2,
        8 => Msaa::Sample8,
        _ => Msaa::Sample4,
    };
    for (entity, target, current) in &cameras {
        if !matches!(target, RenderTarget::Window(_)) {
            continue;
        }
        if current != Some(&desired) {
            commands.entity(entity).insert(desired);
        }
    }
}

/// The hardware seed against the schedule that actually runs it.
///
/// Integration tests on purpose. `seed_from_hardware` has unit tests beside
/// the tiers, but those pass whether or not the seed reaches a real settings
/// file. The risk here is ordering, which only the real schedule shows.
#[cfg(test)]
mod seed_schedule_tests {
    use super::*;
    use ambition_persistence::settings::persistence::{save_settings, settings_path_under};
    use ambition_persistence::settings::seed_profile_for_gpu;
    use ambition_persistence::{PersistenceRoot, PersistenceSchedulePlugin};

    /// A `RenderAdapterInfo` for a machine that is not present. Every field but
    /// `device_type` is inert here; the seed reads the class and the name.
    fn adapter(
        device_type: wgpu::DeviceType,
        name: &str,
    ) -> bevy::render::renderer::RenderAdapterInfo {
        bevy::render::renderer::RenderAdapterInfo(bevy::render::renderer::WgpuWrapper::new(
            wgpu::AdapterInfo {
                name: name.to_string(),
                vendor: 0,
                device: 0,
                device_type,
                driver: String::new(),
                driver_info: String::new(),
                backend: wgpu::Backend::Noop,
                // wgpu 29 added these; `AdapterInfo` has no `Default`, so they are
                // written out. Inert: the seed reads only `device_type` and `name`.
                device_pci_bus_id: String::new(),
                subgroup_min_size: 0,
                subgroup_max_size: 0,
                transient_saves_memory: false,
            },
        ))
    }

    /// The app a player actually boots: settings load from disk, and the render
    /// seam seeds the tier.
    fn booted_app(root: PersistenceRoot, device_type: wgpu::DeviceType) -> App {
        let mut app = App::new();
        app.insert_resource(root);
        app.init_resource::<UserSettings>();
        // The schedule plugin installs the save systems too, and they need their
        // resource. The shipped composition inits both; doing so here keeps the
        // test on the real plugin.
        app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
        app.add_plugins(PersistenceSchedulePlugin);
        app.add_plugins(VisualQualityPlugin);
        app.insert_resource(adapter(device_type, "a machine that is not here"));
        app
    }

    /// An install from before the seed has a settings file with no
    /// `hardware_seeded` key, so serde gives `false`, and that file loads in
    /// `Startup`. The seed must still apply after the load.
    #[test]
    fn an_existing_settings_file_still_receives_its_first_run_seed() {
        let root = PersistenceRoot::isolated();
        let path = settings_path_under(&root.0);
        let mut stored = UserSettings::default();
        // Proof the file was loaded; otherwise a pass could mean the load did
        // nothing.
        stored.audio.master_volume = 0.37;
        assert!(
            !stored.video.quality.hardware_seeded,
            "a file written before the seed existed has not been seeded; \
             the arm is meaningless if the fixture is already seeded"
        );
        save_settings(&path, &stored).expect("the fixture settings file is written");

        let mut app = booted_app(root, wgpu::DeviceType::Cpu);
        app.update();

        let settings = app.world().resource::<UserSettings>();
        assert_eq!(
            settings.audio.master_volume, 0.37,
            "the stored file was not loaded at all, so this test proves nothing"
        );
        assert_eq!(
            settings.video.quality.profile,
            seed_profile_for_gpu(ambition_persistence::settings::DetectedGpuClass::Cpu),
            "an existing install on a software rasteriser kept its OS-decided \
             default tier: the seed ran before the file that overwrote it"
        );
        assert!(
            settings.video.quality.hardware_seeded,
            "the seed did not record that it ran, so it would be re-examined \
             every boot forever"
        );
    }

    /// A player who already chose a tier keeps it.
    #[test]
    fn a_chosen_tier_in_an_existing_file_survives_the_seed() {
        let root = PersistenceRoot::isolated();
        let path = settings_path_under(&root.0);
        let mut stored = UserSettings::default();
        stored.video.quality.profile = VisualQualityProfile::Ultra;
        save_settings(&path, &stored).expect("the fixture settings file is written");

        let mut app = booted_app(root, wgpu::DeviceType::Cpu);
        app.update();

        let settings = app.world().resource::<UserSettings>();
        assert_eq!(
            settings.video.quality.profile,
            VisualQualityProfile::Ultra,
            "the seed overrode a tier the player had chosen"
        );
        assert!(
            settings.video.quality.hardware_seeded,
            "the attempt must be recorded even when it declines to move anything, \
             or this player is re-examined on every launch"
        );
    }

    /// A fresh install with no file must still seed.
    #[test]
    fn a_fresh_install_with_no_settings_file_is_seeded() {
        let root = PersistenceRoot::isolated();
        assert!(
            !settings_path_under(&root.0).exists(),
            "an isolated root must start with no settings file"
        );

        let mut app = booted_app(root, wgpu::DeviceType::IntegratedGpu);
        app.update();

        let settings = app.world().resource::<UserSettings>();
        assert_eq!(
            settings.video.quality.profile,
            seed_profile_for_gpu(ambition_persistence::settings::DetectedGpuClass::Integrated),
        );
    }

    /// The migration reaches disk. `hardware_seeded` stops the seed from
    /// re-examining a player every boot only if the settings writer commits it:
    /// the seed writes the resource, and the `Update` writer persists it.
    #[test]
    fn the_seed_is_persisted_so_it_is_not_re_examined_next_boot() {
        let root = PersistenceRoot::isolated();
        let path = settings_path_under(&root.0);
        save_settings(&path, &UserSettings::default()).expect("fixture written");
        assert!(
            !ambition_persistence::settings::persistence::load_settings(&path)
                .video
                .quality
                .hardware_seeded,
            "the fixture on disk must start un-seeded"
        );

        let mut app = booted_app(root, wgpu::DeviceType::Cpu);
        app.update();

        let on_disk = ambition_persistence::settings::persistence::load_settings(&path);
        assert!(
            on_disk.video.quality.hardware_seeded,
            "the seed moved the live resource but never reached the file, so the \
             next boot re-examines this player and the flag is decorative"
        );
    }

    /// The seed reaches the resource the renderer reads, not just the settings
    /// it was written into. `sync_resolved_visual_quality` runs in `Update`,
    /// after every startup schedule, so the first frame renders at the seeded
    /// tier.
    #[test]
    fn the_first_frame_resolves_the_seeded_tier() {
        // The forced-profile env var wins over settings by design, which would
        // make this arm read the override instead of the seed.
        if std::env::var(ambition_persistence::settings::QUALITY_PROFILE_ENV).is_ok() {
            return;
        }
        let root = PersistenceRoot::isolated();
        let path = settings_path_under(&root.0);
        save_settings(&path, &UserSettings::default()).expect("fixture written");

        let mut app = booted_app(root, wgpu::DeviceType::Cpu);
        app.update();

        assert_eq!(
            app.world().resource::<ResolvedVisualQuality>().profile,
            seed_profile_for_gpu(ambition_persistence::settings::DetectedGpuClass::Cpu),
            "the seed moved the setting but the first frame still resolved the \
             un-seeded tier"
        );
    }
}
