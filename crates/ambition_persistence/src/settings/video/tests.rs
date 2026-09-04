//! Unit tests for video settings: shader-strength clamping and related nudges.

use super::*;

#[test]
fn screen_shader_strength_stays_clamped() {
    let mut shaders = ScreenShaderSettings::default();
    shaders.nudge_strength(10.0);
    assert_eq!(shaders.strength, 1.0);
    shaders.nudge_strength(-10.0);
    assert_eq!(shaders.strength, 0.0);
}

#[test]
fn flash_intensity_cycles() {
    let order = [
        FlashIntensity::On,
        FlashIntensity::Reduced,
        FlashIntensity::Off,
        FlashIntensity::On,
    ];
    let mut current = order[0];
    for expected in &order[1..] {
        current = current.next();
        assert_eq!(current, *expected);
    }
}

#[test]
fn colorblind_mode_cycles_through_all() {
    let mut visited = std::collections::HashSet::new();
    let mut cur = ColorblindMode::Off;
    for _ in 0..ColorblindMode::ALL.len() {
        visited.insert(cur);
        cur = cur.next();
    }
    assert_eq!(visited.len(), ColorblindMode::ALL.len());
}

#[test]
fn flash_multiplier_clamps() {
    assert_eq!(FlashIntensity::On.multiplier(), 1.0);
    assert_eq!(FlashIntensity::Off.multiplier(), 0.0);
    assert!(FlashIntensity::Reduced.multiplier() > 0.0);
    assert!(FlashIntensity::Reduced.multiplier() < 1.0);
}

#[test]
fn flash_intensity_cycles_through_all() {
    let mut visited: Vec<FlashIntensity> = Vec::new();
    let mut cur = FlashIntensity::On;
    for _ in 0..FlashIntensity::ALL.len() {
        if !visited.contains(&cur) {
            visited.push(cur);
        }
        cur = cur.next();
    }
    assert_eq!(visited.len(), FlashIntensity::ALL.len());
}

#[test]
fn camera_zoom_preset_scales_are_positive_finite() {
    for preset in CameraZoomPreset::ALL {
        let scale = preset.scale();
        assert!(scale > 0.0 && scale.is_finite());
    }
}

#[test]
fn camera_zoom_preset_cycles_through_all() {
    let mut visited: Vec<CameraZoomPreset> = Vec::new();
    let mut cur = CameraZoomPreset::Combat;
    for _ in 0..CameraZoomPreset::ALL.len() {
        if !visited.contains(&cur) {
            visited.push(cur);
        }
        cur = cur.next();
    }
    assert_eq!(visited.len(), CameraZoomPreset::ALL.len());
}

#[test]
fn camera_aspect_policy_cycles_through_all() {
    let mut visited: Vec<CameraAspectPolicy> = Vec::new();
    let mut cur = CameraAspectPolicy::FitDesign;
    for _ in 0..CameraAspectPolicy::ALL.len() {
        if !visited.contains(&cur) {
            visited.push(cur);
        }
        cur = cur.next();
    }
    assert_eq!(visited.len(), CameraAspectPolicy::ALL.len());
}

#[test]
fn combat_framing_biases_up_without_horizontal_tap_lookahead() {
    let (dx, dy) = CameraFramingPreset::Combat.target_offset(800.0, 450.0, 1.0);
    assert_eq!(dx, 0.0);
    assert!(dy < 0.0);
    let (dx_left, _) = CameraFramingPreset::Combat.target_offset(800.0, 450.0, -1.0);
    assert_eq!(dx_left, 0.0);
}

#[test]
fn flash_intensity_prev_next_round_trip() {
    let f = FlashIntensity::Reduced;
    assert_eq!(f.next().prev(), f);
}

#[test]
fn visual_quality_profile_table_matches_android_starting_budget() {
    let low = VisualQualityBudget::for_profile(VisualQualityProfile::Low);
    assert_eq!(low.portal.max_resolution, 384);
    assert_eq!(low.portal.recursion_depth, 0);
    assert!(!low.portal.include_parallax);
    assert_eq!(low.sprites.resolution_scale, TextureResolutionScale::Half);
    assert_eq!(low.parallax.max_layers, Some(2));

    let ultra = VisualQualityBudget::for_profile(VisualQualityProfile::Ultra);
    assert_eq!(ultra.portal.max_active_captures, 4);
    assert_eq!(ultra.backgrounds.max_texture_resolution, 4096);
    assert_eq!(ultra.particles.max_particles, 1024);
}

#[test]
fn potato_profile_strips_everything_to_the_bare_minimum() {
    let potato = VisualQualityBudget::for_profile(VisualQualityProfile::Potato);
    // Smallest possible portal capture, no recursion / parallax, throttled hard.
    assert_eq!(potato.portal.max_resolution, 128);
    assert_eq!(potato.portal.recursion_depth, 0);
    assert!(!potato.portal.include_parallax);
    assert!(potato.portal.min_refresh_interval_s > 0.0);
    // Tiniest textures for sprites + backgrounds.
    assert_eq!(
        potato.sprites.resolution_scale,
        TextureResolutionScale::Potato
    );
    assert!(potato.sprites.prefer_scaled_variants);
    assert_eq!(
        potato.backgrounds.resolution_scale,
        TextureResolutionScale::Potato
    );
    // Parallax off, shaders off, almost no particles.
    assert!(!potato.parallax.enabled);
    assert_eq!(potato.shaders.screen_shader_scale, 0.0);
    assert!(!potato.shaders.allow_expensive_materials);
    assert!(potato.particles.max_particles <= 32);

    // Potato is the floor: it is no heavier than Low on the levers that matter.
    let low = VisualQualityBudget::for_profile(VisualQualityProfile::Low);
    assert!(potato.portal.max_resolution <= low.portal.max_resolution);
    assert!(potato.particles.max_particles <= low.particles.max_particles);
}

#[test]
fn visual_quality_profile_cycles_through_all_including_potato() {
    assert!(VisualQualityProfile::ALL.contains(&VisualQualityProfile::Potato));
    let mut visited: Vec<VisualQualityProfile> = Vec::new();
    let mut cur = VisualQualityProfile::High;
    for _ in 0..VisualQualityProfile::ALL.len() {
        if !visited.contains(&cur) {
            visited.push(cur);
        }
        cur = cur.next();
    }
    assert_eq!(visited.len(), VisualQualityProfile::ALL.len());
    // next/prev round-trips around the new first variant.
    assert_eq!(
        VisualQualityProfile::Potato.next().prev(),
        VisualQualityProfile::Potato
    );
}

#[test]
fn custom_visual_quality_resolves_to_stored_budget() {
    let mut settings = VisualQualitySettings::default();
    settings.profile = VisualQualityProfile::Custom;
    settings.custom.portal.max_resolution = 333;
    assert_eq!(settings.resolved_budget().portal.max_resolution, 333);
}

#[test]
fn texture_resolution_scale_owns_variant_folder_names() {
    assert_eq!(
        TextureResolutionScale::Half.asset_subdir("custom_sprites"),
        "custom_sprites_0_5x"
    );
    assert_eq!(
        TextureResolutionScale::Quarter.asset_subdir("sprites"),
        "sprites_0_25x"
    );
    assert_eq!(
        TextureResolutionScale::Half.parallax_subdir(),
        "backgrounds/parallax_layers_0_5x"
    );
    assert_eq!(
        TextureResolutionScale::Full.asset_subdir("sprites"),
        "sprites"
    );
    assert_eq!(
        TextureResolutionScale::Potato.asset_subdir("sprites"),
        "sprites_potato"
    );
    assert_eq!(
        TextureResolutionScale::Potato.parallax_subdir(),
        "backgrounds/parallax_layers_potato"
    );
    assert_eq!(
        TextureResolutionScale::Potato.asset_id_suffix(),
        Some("potato")
    );
    // Every below-Full tier is a generated variant; Full is never in the list.
    assert_eq!(TextureResolutionScale::MANIFEST_VARIANTS.len(), 3);
    assert!(TextureResolutionScale::MANIFEST_VARIANTS.contains(&TextureResolutionScale::Potato));
    assert!(!TextureResolutionScale::MANIFEST_VARIANTS.contains(&TextureResolutionScale::Full));
}

// ── Raster budget ────────────────────────────────────────────────────────────
// The two knobs that scale with SCREEN AREA. Everything asserted here is about
// not surprising a machine that can afford the work.

#[test]
fn high_and_ultra_raster_budgets_are_todays_behaviour() {
    // ⛔ THIS IS THE NO-KNEECAP GUARD. `max_scale_factor: None` honours the
    // compositor and `msaa_samples: 4` is Bevy's own default, so a machine on
    // High or Ultra rasterises exactly what it did before the budget existed.
    // If either value changes here, capable hardware got quietly downgraded.
    for profile in [VisualQualityProfile::High, VisualQualityProfile::Ultra] {
        let raster = VisualQualityBudget::for_profile(profile).raster;
        assert_eq!(raster.max_scale_factor, None, "{profile:?} must not cap DPI scale");
        assert_eq!(raster.sanitized_msaa_samples(), 4, "{profile:?} must keep 4x MSAA");
    }
}

#[test]
fn the_cheaper_tiers_cap_dpi_scale_and_drop_msaa() {
    for profile in
        [VisualQualityProfile::Potato, VisualQualityProfile::Low, VisualQualityProfile::Medium]
    {
        let raster = VisualQualityBudget::for_profile(profile).raster;
        assert_eq!(raster.max_scale_factor, Some(1.0), "{profile:?} should cap DPI scale at 1x");
        assert_eq!(raster.sanitized_msaa_samples(), 1, "{profile:?} should run without MSAA");
    }
}

#[test]
fn capping_the_scale_factor_never_raises_it() {
    // ⭐ A CAP, NOT A SETTING. The failure this guards is a 1x laptop being
    // told to rasterise at 2x because a tier "sets" the scale factor.
    let capped = RasterBudget { max_scale_factor: Some(1.0), msaa_samples: 1 };
    assert_eq!(capped.effective_scale_factor(2.0), Some(1.0), "2x display is brought down");
    assert_eq!(capped.effective_scale_factor(1.0), None, "1x display is left alone");
    assert_eq!(capped.effective_scale_factor(0.75), None, "below the cap is left alone");

    let uncapped = RasterBudget { max_scale_factor: None, msaa_samples: 4 };
    assert_eq!(uncapped.effective_scale_factor(2.0), None, "no cap means no override");
}

#[test]
fn msaa_sample_counts_round_down_to_something_bevy_accepts() {
    // A hand-edited config is the expected source of a bad value here, and the
    // safe direction to resolve one is DOWNWARD — never hand a struggling
    // machine more samples than it asked for.
    let samples = |n| RasterBudget { max_scale_factor: None, msaa_samples: n }
        .sanitized_msaa_samples();
    assert_eq!(samples(0), 1, "0 is off, not a crash");
    assert_eq!(samples(1), 1);
    assert_eq!(samples(2), 2);
    assert_eq!(samples(4), 4);
    assert_eq!(samples(8), 8);
    assert_eq!(samples(3), 2, "rounds down");
    assert_eq!(samples(6), 4, "rounds down");
    assert_eq!(samples(16), 8, "clamped to the highest Bevy names");
}

#[test]
fn profile_labels_round_trip_through_from_label() {
    // The string a diagnostic prints must be the string a config file accepts,
    // or the error message teaches the wrong spelling.
    for profile in VisualQualityProfile::ALL {
        if profile == VisualQualityProfile::Custom {
            continue;
        }
        assert_eq!(
            VisualQualityProfile::from_label(profile.label()),
            Some(profile),
            "{profile:?} should parse back from its own label",
        );
    }
}

#[test]
fn from_label_tolerates_hand_editing_but_not_custom() {
    assert_eq!(VisualQualityProfile::from_label("  MEDIUM \n"), Some(VisualQualityProfile::Medium));
    assert_eq!(VisualQualityProfile::from_label("Low"), Some(VisualQualityProfile::Low));
    // ⛔ `custom` means "use the budget stored in the user's settings", which a
    // boot override cannot supply. Accepting it would boot High wearing another
    // name.
    assert_eq!(VisualQualityProfile::from_label("custom"), None);
    // A typo must not resolve to a tier nobody chose.
    assert_eq!(VisualQualityProfile::from_label("mediun"), None);
    assert_eq!(VisualQualityProfile::from_label(""), None);
}

// ⚠ These mutate process-wide environment, so they run in ONE test with the
// variables cleared between cases. Split into separate `#[test]`s they would
// race each other under the default parallel harness and fail intermittently —
// which is worse than the bug they are meant to catch.
#[test]
fn raster_env_overrides_apply_on_top_of_the_tier() {
    let base = || RasterBudget { max_scale_factor: None, msaa_samples: 4 };
    let clear = || {
        unsafe {
            std::env::remove_var(MAX_SCALE_FACTOR_ENV);
            std::env::remove_var(MSAA_ENV);
        }
    };

    clear();
    assert_eq!(base().with_env_overrides(), base(), "no variables set changes nothing");

    unsafe { std::env::set_var(MAX_SCALE_FACTOR_ENV, "1.0") };
    assert_eq!(base().with_env_overrides().max_scale_factor, Some(1.0));

    // Turning the cap OFF must be expressible, or a config can only ever make
    // the picture cheaper and never restore it.
    unsafe { std::env::set_var(MAX_SCALE_FACTOR_ENV, "none") };
    assert_eq!(base().with_env_overrides().max_scale_factor, None);

    // ⛔ A value that cannot be read is IGNORED, never defaulted: substituting a
    // number nobody typed makes a typo look like a successful experiment.
    for junk in ["", "  ", "abc", "-1", "0", "nan", "inf"] {
        unsafe { std::env::set_var(MAX_SCALE_FACTOR_ENV, junk) };
        let got = RasterBudget { max_scale_factor: Some(2.0), msaa_samples: 4 }
            .with_env_overrides()
            .max_scale_factor;
        assert_eq!(got, Some(2.0), "{junk:?} should leave the tier's value alone");
    }

    clear();
    unsafe { std::env::set_var(MSAA_ENV, "1") };
    assert_eq!(base().with_env_overrides().sanitized_msaa_samples(), 1);
    unsafe { std::env::set_var(MSAA_ENV, "8") };
    assert_eq!(base().with_env_overrides().sanitized_msaa_samples(), 8);
    unsafe { std::env::set_var(MSAA_ENV, "banana") };
    assert_eq!(base().with_env_overrides().sanitized_msaa_samples(), 4, "junk keeps the tier");
    clear();
}

#[test]
fn settings_ron_without_raster_still_parses() {
    // ⛔ THE REGRESSION THIS GUARDS COST A REAL SETTINGS FILE. `custom` is
    // serialised into the user's `settings.ron`; a budget field added later is
    // absent from every file written before it existed. Without `serde(default)`
    // that is not a missing field — the WHOLE settings parse fails and the user
    // silently loses audio, video, gameplay and controls together.
    let without_raster = r#"(
        portal: (max_resolution: 1024, texels_per_world_px: 1.0, recursion_depth: 1,
                 max_active_captures: 2, max_updates_per_frame: 2,
                 min_refresh_interval_s: 0.0, include_parallax: true),
        sprites: (resolution_scale: Full, prefer_scaled_variants: false),
        backgrounds: (resolution_scale: Full, max_texture_resolution: 2048,
                      prefer_scaled_variants: false),
        parallax: (enabled: true, max_layers: None, resolution_scale: Full),
        shaders: (screen_shader_scale: 1.0, allow_expensive_materials: true),
        particles: (max_particles: 4096, spawn_rate_scale: 1.0),
    )"#;
    let parsed: VisualQualityBudget =
        ron::from_str(without_raster).expect("a pre-raster budget must still deserialize");

    // ⭐ AND IT MUST DESERIALIZE TO WHAT THE ENGINE DID BEFORE THE FIELD EXISTED.
    // The user never chose a raster budget, so they must not be handed a cheaper
    // one by surprise on upgrade.
    assert_eq!(parsed.raster.max_scale_factor, None, "must still honour the compositor");
    assert_eq!(parsed.raster.sanitized_msaa_samples(), 4, "must still be Bevy's default MSAA");
    assert_eq!(parsed.raster, RasterBudget::default());
}

/// ⭐⭐ THE TIER IS SEEDED FROM WHAT THE MACHINE RENDERS WITH, not from its OS.
///
/// `default_visual_quality_profile` decides by target OS, so every desktop
/// boots `High` — including `calculex`, whose renderer is an Intel HD 630 and
/// which measured p50 51.0ms (~19.6 FPS) there. The OS was never the thing that
/// made it slow.
#[test]
fn the_seed_tier_follows_the_adapter_and_not_the_operating_system() {
    use super::{seed_profile_for_gpu, DetectedGpuClass, VisualQualityProfile};

    assert_eq!(
        seed_profile_for_gpu(DetectedGpuClass::Discrete),
        VisualQualityProfile::High,
        "a discrete card is what the High tier was authored against"
    );
    assert_eq!(
        seed_profile_for_gpu(DetectedGpuClass::Integrated),
        VisualQualityProfile::Medium,
        "an IGP shares system memory and must not start where a discrete card does"
    );
    assert_eq!(
        seed_profile_for_gpu(DetectedGpuClass::Virtual),
        VisualQualityProfile::Medium,
        "a paravirtualised adapter is an IGP's problem wearing a guest's clothes"
    );

    // ⛔ NO GPU IS NOT A WEAK GPU. A software rasteriser pays its fill cost on
    // the same cores running the sim, so the tier that merely trims effects is
    // the wrong answer for it.
    assert_eq!(
        seed_profile_for_gpu(DetectedGpuClass::Cpu),
        VisualQualityProfile::Potato,
        "llvmpipe/lavapipe must start at the cheapest tier there is"
    );

    // ⛔ AND AN ADAPTER WE CANNOT CLASSIFY KEEPS THE EXISTING DEFAULT. Booting an
    // unrecognised machine into a degraded tier makes "we did not recognise
    // your GPU" indistinguishable from "your GPU is bad".
    assert_eq!(
        seed_profile_for_gpu(DetectedGpuClass::Other),
        super::default_visual_quality_profile(),
        "an unknown adapter must not be guessed downward"
    );
}

/// ⛔⛔ A SEED IS NOT AN OVERRIDE. Re-deciding the tier each launch would
/// silently undo the settings menu — a player who chose High on an integrated
/// laptop would be put back on Medium every boot with nothing admitting why.
///
/// The seam that enforces this is serde: `VisualQualitySettings::profile`
/// carries `#[serde(default = "default_visual_quality_profile")]`, so a STORED
/// profile is read back verbatim and the default function is not consulted at
/// all. This pins that, so a future refactor that "helpfully" re-seeds on load
/// fails here rather than in a player's settings menu.
#[test]
fn a_stored_profile_survives_a_reload_and_is_never_re_seeded() {
    use super::{VisualQualityProfile, VisualQualitySettings};

    let stored = r#"(profile: High)"#;
    let parsed: VisualQualitySettings = ron::from_str(stored)
        .expect("a stored quality block must deserialize");
    assert_eq!(
        parsed.profile,
        VisualQualityProfile::High,
        "the player's own choice must come back exactly as they left it"
    );

    // And the absent case is the ONLY one the default may answer.
    let unset = r#"()"#;
    let seeded: VisualQualitySettings =
        ron::from_str(unset).expect("a settings block with no profile must still load");
    assert_eq!(
        seeded.profile,
        super::default_visual_quality_profile(),
        "only a settings file that never stored a profile may take a default"
    );
}

/// The seed runs ONCE, and never against a tier the player owns.
#[test]
fn the_hardware_seed_fires_once_and_respects_a_chosen_tier() {
    use super::{DetectedGpuClass, VisualQualityProfile, VisualQualitySettings};

    // First run on an integrated laptop: seeded down, and recorded.
    let mut fresh = VisualQualitySettings::default();
    assert!(!fresh.hardware_seeded, "a fresh settings block has not been seeded");
    assert_eq!(
        fresh.seed_from_hardware(DetectedGpuClass::Integrated),
        Some(VisualQualityProfile::Medium),
        "an IGP must not start where a discrete card does"
    );
    assert!(fresh.hardware_seeded);

    // ⛔ AND NEVER AGAIN. A second boot must not re-decide, even if the player
    // has since moved the tier UP — that is exactly the silent override this
    // whole mechanism is written to avoid.
    fresh.profile = VisualQualityProfile::High;
    assert_eq!(
        fresh.seed_from_hardware(DetectedGpuClass::Integrated),
        None,
        "a seeded install is never re-seeded"
    );
    assert_eq!(
        fresh.profile,
        VisualQualityProfile::High,
        "the player's later choice survives the next launch"
    );

    // An existing install where the player already chose a tier is left alone
    // on its one and only seeding pass.
    let mut chosen = VisualQualitySettings {
        profile: VisualQualityProfile::Potato,
        ..Default::default()
    };
    assert_eq!(
        chosen.seed_from_hardware(DetectedGpuClass::Discrete),
        None,
        "a tier the player moved off the default is theirs, not the seed's"
    );
    assert_eq!(chosen.profile, VisualQualityProfile::Potato);
    assert!(chosen.hardware_seeded, "the attempt is recorded so it is not retried each boot");
}

/// ⭐ THE DEFAULT FRAMING IS CALIBRATED, NOT CHOSEN BY EYE — so it gets a test
/// that states the arithmetic, because the number is meaningless without it.
///
/// Jon asked for a more zoomed-in default "for smash too" and named
/// `pointed_polygon` as the reference. That character is `body_kind: Standard`,
/// which authors a standing height of **48.0 world units**
/// (`ambition_characters::actor::character_catalog::entry`), so the ratio a
/// player actually sees is `48 / base_view.y`.
///
/// The target is Smash-like readability: a medium fighter in a neutral 1v1
/// reads at roughly 14–16% of screen height. `Duel` puts a standard humanoid at
/// exactly 15.0%.
///
/// ⛔⛔ THIS TEST DOES NOT NOTICE A CHANGE TO `Standard`'s STANDING HEIGHT, and
/// it used to claim it did. This crate cannot see `ambition_characters`, so the
/// 48.0 below is a COPY — measured 2026-09-04 by moving the real authority to
/// 50.0: this file stayed green (120 passed) while the cross-domain guard went
/// red. A guard that duplicates the value it guards cannot fail for the reason
/// it exists.
/// ⇒ What this test checks is the ARITHMETIC of the preset, against a stated
/// reference height. The cross-domain calibration is
/// `ambition_sim_view::camera_snapshot::default_framing_calibration_tests`,
/// which depends on both crates and reads both authorities for real.
#[test]
fn the_default_framing_puts_a_standard_humanoid_at_fifteen_percent_of_screen_height() {
    /// A COPY of `BodyKind::Standard::default_standing_height()`, and the
    /// reason this test cannot be the calibration guard: this crate does not
    /// depend on the characters crate, so nothing here can read the real one.
    const STANDARD_STANDING_HEIGHT: f32 = 48.0;

    let (_, view_h) = CameraZoomPreset::default().base_view();
    let ratio = STANDARD_STANDING_HEIGHT / view_h;

    assert!(
        (ratio - 0.15).abs() < 0.005,
        "the default framing must put a standard humanoid at ~15% of screen height \
         (Smash-like readability); got {:.1}% at a {view_h}-unit view",
        ratio * 100.0,
    );
}

/// Non-vacuity for the test above, and a record of what the change actually did:
/// the previous default was materially wider than the target.
#[test]
fn the_previous_default_was_wider_than_the_readability_target() {
    const STANDARD_STANDING_HEIGHT: f32 = 48.0;

    let (_, combat_h) = CameraZoomPreset::Combat.base_view();
    let combat_ratio = STANDARD_STANDING_HEIGHT / combat_h;

    assert!(
        combat_ratio < 0.11,
        "the `Combat` framing this replaced sat at {:.1}%, below even the \
         most zoomed-OUT end of Smash's normal dynamic range (~11%) — which is \
         why the game read as further away than a fighting game should",
        combat_ratio * 100.0,
    );
}
