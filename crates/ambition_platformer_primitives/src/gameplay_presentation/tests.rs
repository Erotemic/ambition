//! Pure layout oracles for the presentation resolver.
//!
//! These pin acceptance oracles 1, 2, 5, 7, 8 and 12 from
//! `docs/planning/triage/gameplay-presentation-profiles.md` at the only level
//! where they are cheap: no window, no app, no game.

use super::*;
use ambition_engine_core as ae;

/// The required display matrix: 4:3, 16:9, 16:10, 19.5:9, 20:9 — landscape.
const DISPLAYS: &[(&str, f32, f32)] = &[
    ("4:3", 1024.0, 768.0),
    ("16:9", 1920.0, 1080.0),
    ("16:10", 1680.0, 1050.0),
    ("19.5:9", 2340.0, 1080.0),
    ("20:9", 2400.0, 1080.0),
];

fn resolve(
    display: ae::Vec2,
    insets: ScreenInsets,
    profile: &GameplayPresentationProfile,
    occlusions: &[ScreenOcclusion],
) -> ResolvedGameplayPresentation {
    resolve_gameplay_presentation(GameplayPresentationInput {
        display_px: display,
        safe_area_insets: insets,
        profile,
        occlusions,
    })
}

fn four_three() -> GameplayPresentationProfile {
    *profiles::fixed_four_by_three().for_environment(PresentationEnvironment::Desktop)
}

/// Oracle 1 — a fixed-aspect gameplay rectangle preserves the requested aspect
/// inside the device-safe rectangle, on every display in the matrix and with
/// asymmetric insets.
#[test]
fn fixed_aspect_preserves_ratio_inside_the_safe_rect() {
    let profile = four_three();
    let insets = ScreenInsets::new(48.0, 12.0, 24.0, 60.0);

    for &(name, w, h) in DISPLAYS {
        for insets in [ScreenInsets::ZERO, insets] {
            let resolved = resolve(ae::Vec2::new(w, h), insets, &profile, &[]);
            let gameplay = resolved.gameplay_rect;
            let ratio = gameplay.width() / gameplay.height();
            assert!(
                (ratio - 4.0 / 3.0).abs() < 1e-3,
                "{name} @ {insets:?}: gameplay aspect {ratio} is not 4:3 ({gameplay:?})",
            );

            let safe = resolved.display_safe_rect;
            assert!(
                gameplay.min.x >= safe.min.x - 1e-3
                    && gameplay.min.y >= safe.min.y - 1e-3
                    && gameplay.max.x <= safe.max.x + 1e-3
                    && gameplay.max.y <= safe.max.y + 1e-3,
                "{name}: gameplay {gameplay:?} escaped the safe rect {safe:?}",
            );
        }
    }
}

/// A display wider than the requested aspect pillarboxes; a narrower one
/// letterboxes; an exact match fills the safe display.
#[test]
fn fixed_aspect_pillarboxes_when_wide_and_letterboxes_when_narrow() {
    let profile = four_three();

    let wide = resolve(ae::Vec2::new(2400.0, 1080.0), ScreenInsets::ZERO, &profile, &[]);
    assert!(wide.gameplay_rect.height() == 1080.0, "wide display should be height-bound");
    assert!(
        wide.surround_rect(SurroundRegion::Left).is_some()
            && wide.surround_rect(SurroundRegion::Right).is_some(),
        "a wide display must pillarbox",
    );
    assert!(
        wide.surround_rect(SurroundRegion::Top).is_none()
            && wide.surround_rect(SurroundRegion::Bottom).is_none(),
    );

    // Narrower than 4:3 (a portrait-ish window) letterboxes instead.
    let narrow = resolve(ae::Vec2::new(600.0, 800.0), ScreenInsets::ZERO, &profile, &[]);
    assert!(narrow.gameplay_rect.width() == 600.0, "narrow display should be width-bound");
    assert!(
        narrow.surround_rect(SurroundRegion::Top).is_some()
            && narrow.surround_rect(SurroundRegion::Bottom).is_some(),
        "a narrow display must letterbox",
    );

    let exact = resolve(ae::Vec2::new(1024.0, 768.0), ScreenInsets::ZERO, &profile, &[]);
    assert_eq!(exact.gameplay_rect, exact.display_safe_rect);
    assert!(!exact.has_surround(), "a 4:3 display leaves no surround");
}

/// Oracle 2 — full-bleed mode uses the full safe display, and leaves no
/// surround to draw.
#[test]
fn full_bleed_uses_the_whole_safe_display() {
    let profile = GameplayPresentationProfile::full_bleed();
    let insets = ScreenInsets::new(30.0, 0.0, 0.0, 18.0);

    for &(name, w, h) in DISPLAYS {
        let resolved = resolve(ae::Vec2::new(w, h), insets, &profile, &[]);
        assert_eq!(
            resolved.gameplay_rect, resolved.display_safe_rect,
            "{name}: full bleed must equal the safe display",
        );
        assert!(!resolved.has_surround(), "{name}: full bleed has no surround");
        assert_eq!(resolved.gameplay_rect.min, ae::Vec2::new(30.0, 0.0));
        assert_eq!(resolved.gameplay_rect.max, ae::Vec2::new(w, h - 18.0));
    }
}

/// A hostile safe-area report must not black the game out.
#[test]
fn degenerate_safe_area_insets_fall_back_to_the_display() {
    let profile = GameplayPresentationProfile::full_bleed();
    let resolved = resolve(
        ae::Vec2::new(1280.0, 720.0),
        ScreenInsets::new(900.0, 900.0, 500.0, 500.0),
        &profile,
        &[],
    );
    assert_eq!(resolved.display_safe_rect, resolved.display_rect);
}

/// The surround regions tile the leftover display without overlapping the
/// gameplay rectangle or each other.
#[test]
fn surround_regions_do_not_overlap_gameplay() {
    let profile = four_three();
    let resolved = resolve(ae::Vec2::new(2400.0, 1080.0), ScreenInsets::ZERO, &profile, &[]);

    let gameplay = resolved.gameplay_rect;
    for named in &resolved.surround_rects {
        assert!(
            !named.rect.overlaps(gameplay),
            "{:?} {:?} overlaps gameplay {gameplay:?}",
            named.region,
            named.rect,
        );
    }

    let surround_area: f32 = resolved.surround_rects.iter().map(|n| n.rect.area()).sum();
    assert!(
        (surround_area + gameplay.area() - resolved.display_safe_rect.area()).abs() < 1.0,
        "surround + gameplay must tile the safe display",
    );
}

/// Normal framing leaves the whole gameplay rectangle available — there is no
/// hidden deadzone when a game declares nothing.
#[test]
fn normal_framing_protects_the_whole_gameplay_rect() {
    let profile = GameplayPresentationProfile::full_bleed();
    let resolved = resolve(ae::Vec2::new(1920.0, 1080.0), ScreenInsets::ZERO, &profile, &[]);
    assert_eq!(resolved.subject_safe_region, NormalizedScreenRegion::FULL);
    assert!(resolved.soft_framing.is_none());
}

fn stick(size: f32) -> ScreenOcclusion {
    ScreenOccluder::new(
        ScreenOcclusionPurpose::VirtualMovementStick,
        ScreenAnchor::BottomLeft,
        ae::Vec2::splat(24.0),
        ae::Vec2::splat(size),
    )
    .resolve(ScreenRect::from_min_size(
        ae::Vec2::ZERO,
        ae::Vec2::new(2400.0, 1080.0),
    ))
}

/// Oracle 7 — occlusion-aware framing keeps the subject-safe region clear of
/// controls, and does so by trimming the corner-ward strip rather than half
/// the display.
#[test]
fn occlusion_aware_framing_carves_the_cheapest_side() {
    let profile = GameplayPresentationProfile::full_bleed()
        .with_occlusion_aware_framing(SoftFramingProfile::platformer());
    let display = ae::Vec2::new(2400.0, 1080.0);
    let occlusion = stick(600.0);

    let resolved = resolve(display, ScreenInsets::ZERO, &profile, &[occlusion]);
    assert!(
        !resolved.subject_safe_rect.overlaps(occlusion.rect),
        "safe region {:?} still overlaps the stick {:?}",
        resolved.subject_safe_rect,
        occlusion.rect,
    );

    // Exactly ONE edge moves, and it is the cheapest one. On a 20:9 display a
    // bottom-left stick is far cheaper to clear sideways than by surrendering
    // the vertical band the actor actually jumps through.
    let baseline = resolve(display, ScreenInsets::ZERO, &profile, &[]).subject_safe_rect;
    let carved = resolved.subject_safe_rect;
    assert!(carved.min.x > baseline.min.x, "the left edge should move in");
    assert_eq!(carved.max.x, baseline.max.x);
    assert_eq!(carved.min.y, baseline.min.y);
    assert_eq!(
        carved.max.y, baseline.max.y,
        "clearing sideways is cheaper than raising the bottom here",
    );

    // ...and "cheapest" is literal: no other single-side carve that clears the
    // stick would have cost less area.
    let alternatives = [
        ScreenRect { min: baseline.min, max: ae::Vec2::new(occlusion.rect.min.x, baseline.max.y) },
        ScreenRect { min: ae::Vec2::new(baseline.min.x, occlusion.rect.max.y), max: baseline.max },
        ScreenRect { min: baseline.min, max: ae::Vec2::new(baseline.max.x, occlusion.rect.min.y) },
    ];
    for alternative in alternatives {
        assert!(
            carved.area() >= alternative.area() - 1e-3,
            "carve {carved:?} lost more area than {alternative:?}",
        );
    }
}

/// Soft framing that is not occlusion-aware ignores occupancy entirely —
/// that is the whole difference between the two policies.
#[test]
fn soft_safe_region_ignores_occlusions() {
    let occlusion = stick(600.0);
    let display = ae::Vec2::new(2400.0, 1080.0);

    let soft = GameplayPresentationProfile::full_bleed()
        .with_soft_framing(SoftFramingProfile::platformer());
    let aware = GameplayPresentationProfile::full_bleed()
        .with_occlusion_aware_framing(SoftFramingProfile::platformer());

    let soft = resolve(display, ScreenInsets::ZERO, &soft, &[occlusion]);
    let aware = resolve(display, ScreenInsets::ZERO, &aware, &[occlusion]);

    assert_eq!(
        soft.subject_safe_rect,
        resolve(
            display,
            ScreenInsets::ZERO,
            &GameplayPresentationProfile::full_bleed()
                .with_soft_framing(SoftFramingProfile::platformer()),
            &[],
        )
        .subject_safe_rect,
    );
    assert_ne!(soft.subject_safe_rect, aware.subject_safe_rect);
}

/// Purposes that do not reserve subject space never shrink framing.
#[test]
fn non_reserving_purposes_do_not_carve() {
    let profile = GameplayPresentationProfile::full_bleed()
        .with_occlusion_aware_framing(SoftFramingProfile::platformer());
    let display = ae::Vec2::new(2400.0, 1080.0);

    let chrome = ScreenOcclusion {
        purpose: ScreenOcclusionPurpose::SystemMenuControl,
        rect: stick(600.0).rect,
    };
    assert_eq!(
        resolve(display, ScreenInsets::ZERO, &profile, &[chrome]).subject_safe_rect,
        resolve(display, ScreenInsets::ZERO, &profile, &[]).subject_safe_rect,
    );
}

/// The composed region must not depend on the order occupancy was published
/// in — entity iteration order is not stable, and a camera that jitters with
/// it would be untraceable.
#[test]
fn occlusion_composition_is_order_independent() {
    let profile = GameplayPresentationProfile::full_bleed()
        .with_occlusion_aware_framing(SoftFramingProfile::platformer());
    let display = ae::Vec2::new(2400.0, 1080.0);
    let bounds = ScreenRect::from_min_size(ae::Vec2::ZERO, display);

    let a = ScreenOccluder::new(
        ScreenOcclusionPurpose::VirtualMovementStick,
        ScreenAnchor::BottomLeft,
        ae::Vec2::splat(24.0),
        ae::Vec2::splat(300.0),
    )
    .resolve(bounds);
    let b = ScreenOccluder::new(
        ScreenOcclusionPurpose::VirtualActionCluster,
        ScreenAnchor::BottomRight,
        ae::Vec2::splat(16.0),
        ae::Vec2::new(320.0, 300.0),
    )
    .resolve(bounds);
    let c = ScreenOccluder::new(
        ScreenOcclusionPurpose::PersistentHud,
        ScreenAnchor::TopLeft,
        ae::Vec2::splat(8.0),
        ae::Vec2::new(280.0, 90.0),
    )
    .resolve(bounds);

    let forward = resolve(display, ScreenInsets::ZERO, &profile, &[a, b, c]).subject_safe_rect;
    let reverse = resolve(display, ScreenInsets::ZERO, &profile, &[c, b, a]).subject_safe_rect;
    let shuffled = resolve(display, ScreenInsets::ZERO, &profile, &[b, c, a]).subject_safe_rect;
    assert_eq!(forward, reverse);
    assert_eq!(forward, shuffled);
}

/// Dense occupancy degrades to overlap, never to a collapsed region.
#[test]
fn carving_stops_at_the_minimum_region() {
    let profile = GameplayPresentationProfile::full_bleed()
        .with_occlusion_aware_framing(SoftFramingProfile::platformer());
    let display = ae::Vec2::new(1280.0, 720.0);
    let bounds = ScreenRect::from_min_size(ae::Vec2::ZERO, display);

    // Four huge controls, one per corner: clearing them all is impossible.
    let hogs: Vec<ScreenOcclusion> = [
        (ScreenAnchor::BottomLeft, ScreenOcclusionPurpose::VirtualMovementStick),
        (ScreenAnchor::BottomRight, ScreenOcclusionPurpose::VirtualActionCluster),
        (ScreenAnchor::TopLeft, ScreenOcclusionPurpose::PersistentHud),
        (ScreenAnchor::TopRight, ScreenOcclusionPurpose::ContextualAction),
    ]
    .into_iter()
    .map(|(anchor, purpose)| {
        ScreenOccluder::new(purpose, anchor, ae::Vec2::ZERO, ae::Vec2::new(600.0, 340.0))
            .resolve(bounds)
    })
    .collect();

    let resolved = resolve(display, ScreenInsets::ZERO, &profile, &hogs);
    let floor = resolved.gameplay_rect.size() * SoftFramingProfile::platformer().min_region_fraction;
    let size = resolved.subject_safe_rect.size();
    assert!(
        size.x >= floor.x && size.y >= floor.y,
        "region {size:?} collapsed below the floor {floor:?}",
    );
}

/// Oracle 12 — the presets differ by configuration, and each one's declared
/// intent survives environment selection.
#[test]
fn presets_declare_the_three_motivating_profiles() {
    let ambition = profiles::adaptive_platformer();
    assert_eq!(
        ambition.for_environment(PresentationEnvironment::Desktop).framing,
        SubjectFramingPolicy::Normal,
        "oracle 6: Ambition desktop retains normal framing",
    );
    assert!(ambition
        .for_environment(PresentationEnvironment::TouchPrimary)
        .framing
        .consumes_occlusions());
    assert_eq!(
        ambition.for_environment(PresentationEnvironment::Desktop).viewport,
        GameplayViewportPolicy::FullBleed,
    );

    let sanic = profiles::high_speed_full_bleed();
    for environment in [
        PresentationEnvironment::Desktop,
        PresentationEnvironment::TouchPrimary,
        PresentationEnvironment::Handheld,
    ] {
        let profile = sanic.for_environment(environment);
        assert_eq!(profile.viewport, GameplayViewportPolicy::FullBleed);
        assert!(
            profile.framing.profile().is_some(),
            "oracle 8: Sanic uses soft framing in {environment:?}",
        );
    }
    assert!(sanic
        .for_environment(PresentationEnvironment::TouchPrimary)
        .framing
        .consumes_occlusions());

    let mary_o = profiles::fixed_four_by_three();
    for environment in [
        PresentationEnvironment::Desktop,
        PresentationEnvironment::TouchPrimary,
        PresentationEnvironment::Handheld,
    ] {
        let profile = mary_o.for_environment(environment);
        assert!(
            matches!(
                profile.viewport,
                GameplayViewportPolicy::FixedAspect { aspect, .. }
                    if (aspect.ratio() - 4.0 / 3.0).abs() < 1e-6
            ),
            "Mary O must be 4:3 in {environment:?}",
        );
        assert_eq!(profile.hud, HudLayoutPolicy::PreferSurround);
    }
}

/// Touch-primary Mary O pins the gameplay rectangle to the top so the vertical
/// slack collects under it, where thumbs are.
#[test]
fn touch_primary_fixed_aspect_pins_to_the_top() {
    let profiles = profiles::fixed_four_by_three();
    // Taller than 4:3 so there IS vertical slack to place.
    let display = ae::Vec2::new(900.0, 900.0);

    let desktop = resolve(
        display,
        ScreenInsets::ZERO,
        profiles.for_environment(PresentationEnvironment::Desktop),
        &[],
    );
    let touch = resolve(
        display,
        ScreenInsets::ZERO,
        profiles.for_environment(PresentationEnvironment::TouchPrimary),
        &[],
    );

    assert_eq!(touch.gameplay_rect.min.y, 0.0);
    assert!(desktop.gameplay_rect.min.y > 0.0);
    assert_eq!(touch.gameplay_rect.size(), desktop.gameplay_rect.size());
    assert!(
        touch.surround_rect(SurroundRegion::Bottom).unwrap().height()
            > desktop.surround_rect(SurroundRegion::Bottom).unwrap().height(),
    );
}

/// An undeclared environment falls back to `default` rather than to engine
/// defaults — a game that declares only `default` gets it everywhere.
#[test]
fn environments_fall_back_to_the_declared_default() {
    let profiles = GameplayPresentationProfiles::uniform(
        GameplayPresentationProfile::fixed_aspect(16.0, 9.0),
    );
    for environment in [
        PresentationEnvironment::Desktop,
        PresentationEnvironment::TouchPrimary,
        PresentationEnvironment::Handheld,
    ] {
        assert_eq!(profiles.for_environment(environment), &profiles.default);
    }
}

/// Anchored occupancy tracks the display edge it was authored against.
#[test]
fn anchored_occluders_follow_their_display_corner() {
    let occluder = ScreenOccluder::new(
        ScreenOcclusionPurpose::VirtualActionCluster,
        ScreenAnchor::BottomRight,
        ae::Vec2::new(10.0, 20.0),
        ae::Vec2::new(200.0, 100.0),
    );

    let small = occluder.resolve(ScreenRect::from_min_size(
        ae::Vec2::ZERO,
        ae::Vec2::new(1000.0, 500.0),
    ));
    assert_eq!(small.rect.max, ae::Vec2::new(990.0, 480.0));
    assert_eq!(small.rect.min, ae::Vec2::new(790.0, 380.0));

    let large = occluder.resolve(ScreenRect::from_min_size(
        ae::Vec2::ZERO,
        ae::Vec2::new(2000.0, 1000.0),
    ));
    assert_eq!(large.rect.max, ae::Vec2::new(1990.0, 980.0));
    assert_eq!(large.rect.size(), small.rect.size());
}

/// Padding expands the reserved region on every side.
#[test]
fn occluder_padding_expands_the_reserved_rect() {
    let bounds = ScreenRect::from_min_size(ae::Vec2::ZERO, ae::Vec2::new(1000.0, 500.0));
    let bare = ScreenOccluder::new(
        ScreenOcclusionPurpose::ContextualAction,
        ScreenAnchor::TopLeft,
        ae::Vec2::splat(40.0),
        ae::Vec2::splat(80.0),
    );
    let padded = bare.with_padding(ae::Vec2::new(12.0, 6.0));

    let bare = bare.resolve(bounds).rect;
    let padded = padded.resolve(bounds).rect;
    assert_eq!(padded.min, bare.min - ae::Vec2::new(12.0, 6.0));
    assert_eq!(padded.max, bare.max + ae::Vec2::new(12.0, 6.0));
}
