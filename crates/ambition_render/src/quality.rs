//! Live resolved visual-quality resource.
//!
//! Settings persist the user's profile/custom table in gameplay-core. The render
//! side mirrors that into one resource every visual subsystem can read.

use bevy::prelude::*;
use bevy::camera::RenderTarget;
use bevy::render::view::Msaa;

use ambition_persistence::settings::{
    profile_override_from_env, DetectedGpuClass, UserSettings, VisualQualityBudget,
    VisualQualityProfile,
};

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ResolvedVisualQuality {
    pub profile: VisualQualityProfile,
    pub budget: VisualQualityBudget,
}

impl Default for ResolvedVisualQuality {
    fn default() -> Self {
        let (profile, mut budget) = match profile_override_from_env() {
            Some(forced) => (forced, VisualQualityBudget::for_profile(forced)),
            None => {
                let settings = ambition_persistence::settings::VisualQualitySettings::default();
                (settings.profile, settings.resolved_budget())
            }
        };
        budget.raster = budget.raster.with_env_overrides();
        Self { profile, budget }
    }
}

impl ResolvedVisualQuality {
    /// ⭐ THE BOOT OVERRIDE WINS, AND IT IS NOT WRITTEN BACK. `AMBITION_QUALITY_PROFILE`
    /// (which `run_game.sh` sets from the launcher config) forces the tier for
    /// the life of the process. It resolves the tier's own budget table rather
    /// than the user's stored `custom` one, so a forced Medium is the same
    /// Medium everywhere.
    ///
    /// ⚠ While an override is in force the settings menu cannot change quality:
    /// this runs every frame and will put the forced tier straight back. That is
    /// the intended behaviour of a forced profile, and the reason
    /// `log_quality_profile_override` says so once at startup rather than
    /// leaving someone to discover it by clicking.
    pub fn from_settings(settings: &UserSettings) -> Self {
        let (profile, mut budget) = match profile_override_from_env() {
            Some(forced) => (forced, VisualQualityBudget::for_profile(forced)),
            None => (settings.video.quality.profile, settings.video.quality.resolved_budget()),
        };
        budget.raster = budget.raster.with_env_overrides();
        Self { profile, budget }
    }
}

/// The resource and the system that keeps it true, together.
///
/// They were apart, and the half that MOVES was app-local. `ResolvedVisualQuality` was
/// initialised by `PlatformerPresentationPlugin` (and again by `game/ambition_app`), while
/// `sync_resolved_visual_quality` — the only thing that ever reads `UserSettings` into it — was
/// registered by the app alone.
///
/// Idempotent (`is_unique() -> false` plus a marker) because two plugins
/// legitimately need it: `PlatformerPresentationPlugin` for a demo, and
/// `SessionRoomVisualsPlugin` for the shipped host, which adds that one alone.
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
        // ⭐ STARTUP, AND ONCE — but AFTER the settings file lands.
        //
        // ⛔⛔ `PostStartup`, NOT `PreStartup`. `load_settings_at_startup` runs
        // in `Startup` and REPLACES the whole `UserSettings` resource with the
        // file's contents. Seeding before it meant the load overwrote the
        // seeded tier and restored the file's `hardware_seeded: false` — and
        // because this system is startup-only it never ran again. An existing
        // install on an integrated GPU, which is the exact machine the seed was
        // written for, therefore migrated on no boot, ever. A fresh install
        // (no file, the loader returns early) migrated fine, which is why the
        // unit tests on `seed_from_hardware` and a first-run play-through both
        // looked correct.
        //
        // `PostStartup` still precedes the first `Update`, so the pair below
        // reads the seeded tier into the resolved budget before frame one.
        app.add_systems(PostStartup, seed_visual_quality_from_adapter);
        app.add_systems(Update, (sync_resolved_visual_quality, sync_raster_budget).chain());
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
/// ⭐ THE TRANSLATION IS ALL THIS SEAM DOES. Which tier a class of hardware
/// should start on is `ambition_persistence`'s decision, next to the tiers it
/// decides between and testable without a GPU — which is the only way it is
/// testable on the machines it matters for. This function is the only place in
/// the codebase that names `wgpu::DeviceType`.
fn detected_gpu_class(device_type: wgpu::DeviceType) -> DetectedGpuClass {
    match device_type {
        wgpu::DeviceType::DiscreteGpu => DetectedGpuClass::Discrete,
        wgpu::DeviceType::IntegratedGpu => DetectedGpuClass::Integrated,
        wgpu::DeviceType::VirtualGpu => DetectedGpuClass::Virtual,
        wgpu::DeviceType::Cpu => DetectedGpuClass::Cpu,
        wgpu::DeviceType::Other => DetectedGpuClass::Other,
    }
}

/// Seed the visual quality tier from the adapter the renderer actually came up
/// on — ONCE, on a profile the player has not touched.
///
/// ⛔⛔ A DETECTED DEFAULT IS A FIRST-RUN SEED, NEVER A PER-BOOT OVERRIDE.
/// Re-deciding every launch would silently undo the settings menu. Both guards
/// live in [`VisualQualitySettings::seed_from_hardware`] — a persisted
/// `hardware_seeded` flag AND the profile still being the untouched default —
/// so this system only supplies the adapter class and reports what happened.
///
/// ⚠ WHY IT IS NEEDED: `default_visual_quality_profile()` decides by TARGET OS,
/// so every desktop booted `High`, including one whose renderer is an Intel
/// HD 630. Measured on `calculex` 2026-08-29: p50 51.0ms (~19.6 FPS) at High.
/// The OS was never the thing that made it slow.
///
/// ⛔ EVERY PARAM IS OPTIONAL AND THAT IS DELIBERATE. A headless composition has
/// no `RenderAdapterInfo` and a fixture may carry no `UserSettings`; a `Res` that
/// matches nothing is a system-param VALIDATION PANIC, not a skip. A world with
/// no renderer has no adapter to read, which is an ordinary state here.
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

/// Say once, at startup, that a forced profile is in force — and say it when the
/// value was set but not understood, which is the case that would otherwise look
/// exactly like the override working.
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
/// ⭐ THESE ARE THE TWO COSTS THAT SCALE WITH SCREEN AREA. Every other knob in
/// the quality budget trades away scene detail; these trade away fragments, and
/// on hardware without a discrete GPU the fragments are the frame. Measured on
/// `calculex` (Intel HD 630) 2026-08-29: a 1600x900 window on a 2x Wayland
/// session rasterised at 3200x1800, every full-screen pass reported exactly
/// 5,760,000 fragment invocations, and the frame sat at a p50 of ~50ms.
///
/// ⛔ CAPTURE CAMERAS ARE NOT TOUCHED. `ambition_render::capture` pins
/// `Msaa::Off` on the image targets it adopts, deliberately, and a blanket
/// write here would undo it. Only cameras whose target is a WINDOW are the
/// screen-area cost this budget is about.
///
/// ⚠ THE SCALE CAP IS A REQUEST, AND THE INSTRUMENT THAT CONFIRMS IT ALREADY
/// EXISTS. `set_scale_factor_override` asks winit for a smaller buffer; whether
/// a given compositor honours it by upscaling (what we want) rather than by
/// shrinking the window is a property of the platform, not of this code. The
/// check is one number in any profiling bundle:
/// `render/upscaling/fragment_shader_invocations` is the framebuffer's pixel
/// count exactly. If capping the scale does not divide it, the cap did not take
/// and the next lever is an explicit reduced render target — do not assume.
pub fn sync_raster_budget(
    quality: Res<ResolvedVisualQuality>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(Entity, &RenderTarget, Option<&Msaa>), With<Camera>>,
    mut commands: Commands,
) {
    let raster = &quality.budget.raster;

    for mut window in &mut windows {
        // Read through `Deref` so an unchanged window is not marked changed;
        // only the assignment below takes `DerefMut`.
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
/// ⭐ THESE ARE INTEGRATION TESTS ON PURPOSE. `seed_from_hardware` is unit
/// tested next to the tiers it decides between, and those tests pass whether or
/// not the seed ever reaches a real settings file — the whole defect this
/// module had was an ORDERING one, invisible to any test that calls the
/// function directly.
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
            },
        ))
    }

    /// The app a player actually boots: settings load from disk, and the render
    /// seam seeds the tier.
    fn booted_app(root: PersistenceRoot, device_type: wgpu::DeviceType) -> App {
        let mut app = App::new();
        app.insert_resource(root);
        app.init_resource::<UserSettings>();
        // The schedule plugin installs the SAVE systems beside the settings
        // ones and they take their resource non-optionally; the composition
        // that ships inits both. Naming them here keeps this test on the real
        // plugin rather than a hand-copied subset of its schedule.
        app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
        app.add_plugins(PersistenceSchedulePlugin);
        app.add_plugins(VisualQualityPlugin);
        app.insert_resource(adapter(device_type, "a machine that is not here"));
        app
    }

    /// ⛔⛔ THE CASE THE FEATURE WAS WRITTEN FOR. An install that predates the
    /// seed has a settings file on disk with no `hardware_seeded` key, so serde
    /// gives it `false` — and that file lands in `Startup`, after `PreStartup`.
    /// Seeding before the load meant the load overwrote the seed AND restored
    /// the un-seeded flag, and the startup-only system never ran again: the
    /// exact machine this was for migrated on no boot, ever.
    #[test]
    fn an_existing_settings_file_still_receives_its_first_run_seed() {
        let root = PersistenceRoot::isolated();
        let path = settings_path_under(&root.0);
        let mut stored = UserSettings::default();
        // Proof the file was really loaded — without it a green result could
        // just mean the load silently did nothing.
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

    /// The other half of the same order: a player who ALREADY chose a tier keeps
    /// it. Moving the seed after the load is what makes this arm reachable at
    /// all — before, the load always won by accident.
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

    /// A fresh install — no file — must still seed. This is the case the
    /// original `PreStartup` placement did get right, and moving the system
    /// must not lose it.
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

    /// The migration REACHES DISK. `hardware_seeded` is the guard that stops
    /// the seed re-examining a player every boot, and it only does that job if
    /// the settings writer commits it — the seed writes the resource, and the
    /// `Update` writer is what makes the answer durable.
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
