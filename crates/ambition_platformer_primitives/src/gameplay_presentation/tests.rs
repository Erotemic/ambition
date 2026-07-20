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
        control_footprints: ControlFootprints::default(),
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
    ScreenOccluder::anchored(
        ScreenOcclusionPurpose::VirtualMovementStick,
        ScreenAnchor::BottomLeft,
        ae::Vec2::splat(24.0),
        ae::Vec2::splat(size),
    )
    .self_resolved(ScreenRect::from_min_size(
        ae::Vec2::ZERO,
        ae::Vec2::new(2400.0, 1080.0),
    ))
    .expect("an anchored occluder resolves itself")
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

    let a = ScreenOccluder::anchored(
        ScreenOcclusionPurpose::VirtualMovementStick,
        ScreenAnchor::BottomLeft,
        ae::Vec2::splat(24.0),
        ae::Vec2::splat(300.0),
    )
    .self_resolved(bounds).expect("anchored");
    let b = ScreenOccluder::anchored(
        ScreenOcclusionPurpose::VirtualActionCluster,
        ScreenAnchor::BottomRight,
        ae::Vec2::splat(16.0),
        ae::Vec2::new(320.0, 300.0),
    )
    .self_resolved(bounds).expect("anchored");
    let c = ScreenOccluder::anchored(
        ScreenOcclusionPurpose::PersistentHud,
        ScreenAnchor::TopLeft,
        ae::Vec2::splat(8.0),
        ae::Vec2::new(280.0, 90.0),
    )
    .self_resolved(bounds).expect("anchored");

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
        ScreenOccluder::anchored(purpose, anchor, ae::Vec2::ZERO, ae::Vec2::new(600.0, 340.0))
            .self_resolved(bounds)
            .expect("anchored")
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

/// Anchored occupancy — the escape hatch for producers with no `bevy_ui`
/// node — still tracks the display edge it was authored against.
#[test]
fn anchored_occluders_follow_their_display_corner() {
    let occluder = ScreenOccluder::anchored(
        ScreenOcclusionPurpose::VirtualActionCluster,
        ScreenAnchor::BottomRight,
        ae::Vec2::new(10.0, 20.0),
        ae::Vec2::new(200.0, 100.0),
    );

    let small = occluder
        .self_resolved(ScreenRect::from_min_size(
            ae::Vec2::ZERO,
            ae::Vec2::new(1000.0, 500.0),
        ))
        .expect("anchored");
    assert_eq!(small.rect.max, ae::Vec2::new(990.0, 480.0));
    assert_eq!(small.rect.min, ae::Vec2::new(790.0, 380.0));

    let large = occluder
        .self_resolved(ScreenRect::from_min_size(
            ae::Vec2::ZERO,
            ae::Vec2::new(2000.0, 1000.0),
        ))
        .expect("anchored");
    assert_eq!(large.rect.max, ae::Vec2::new(1990.0, 980.0));
    assert_eq!(large.rect.size(), small.rect.size());
}

/// Padding expands the reserved region on every side, whichever geometry the
/// rectangle came from.
#[test]
fn occluder_padding_expands_the_reserved_rect() {
    let rect = ScreenRect::from_min_size(ae::Vec2::splat(40.0), ae::Vec2::splat(80.0));
    let bare = ScreenOccluder::contextual_action().from_rect(rect);
    let padded = ScreenOccluder::contextual_action()
        .with_padding(ae::Vec2::new(12.0, 6.0))
        .from_rect(rect);

    assert_eq!(padded.rect.min, bare.rect.min - ae::Vec2::new(12.0, 6.0));
    assert_eq!(padded.rect.max, bare.rect.max + ae::Vec2::new(12.0, 6.0));
}

/// The DEFAULT geometry is the computed UI layout, so an ordinary producer
/// restates nothing: purpose in, layout-derived rectangle out.
#[test]
fn computed_ui_occupancy_comes_from_the_layout() {
    let occluder = ScreenOccluder::action_controls();
    assert_eq!(occluder.geometry, OccluderGeometry::ComputedUi);
    assert!(
        occluder
            .self_resolved(ScreenRect::from_min_size(
                ae::Vec2::ZERO,
                ae::Vec2::splat(1000.0)
            ))
            .is_none(),
        "a computed-UI occluder has no self-known rectangle",
    );

    // Physical pixels on a 2x display convert to logical, centre to corner.
    let occlusion = occluder
        .from_computed_ui(ae::Vec2::new(400.0, 200.0), ae::Vec2::new(1000.0, 600.0), 0.5)
        .expect("a sized node yields occupancy");
    assert_eq!(
        occlusion.rect,
        ScreenRect::from_min_size(ae::Vec2::new(400.0, 250.0), ae::Vec2::new(200.0, 100.0)),
    );

    assert!(
        occluder
            .from_computed_ui(ae::Vec2::ZERO, ae::Vec2::splat(100.0), 1.0)
            .is_none(),
        "a zero-sized node cannot occlude anything",
    );
}

// ---------------------------------------------------------------------------
// Resolved control + HUD placement
// ---------------------------------------------------------------------------
//
// A profile that reserves surround for controls is only telling the truth if
// the controls are actually placed there. These pin the fallback ladder across
// the display matrix, including the displays where a 4:3 viewport does NOT
// leave enough surround.

/// The real touch-HUD footprints, mirrored from `ambition_touch_input::layout`
/// so the ladder is exercised at the sizes that actually ship. Minimums are the
/// usability floor: below them the smallest touch circle drops under ~40 logical
/// px and stops being reliably hittable.
fn touch_footprints() -> ControlFootprints {
    ControlFootprints {
        // Not compactible: the stick's art is owned by `virtual_joystick`, so
        // shrinking its node without its art would reintroduce exactly the
        // drawn-vs-tappable drift this work removes.
        movement: Some(ControlFootprint::fixed(ae::Vec2::splat(210.0))),
        primary_actions: Some(ControlFootprint::new(
            ae::Vec2::new(233.0, 234.4),
            ae::Vec2::new(233.0, 234.4) * 0.893,
        )),
        system_controls: Some(ControlFootprint::fixed(ae::Vec2::new(162.6, 78.0))),
    }
}

fn resolve_controls(
    display: ae::Vec2,
    insets: ScreenInsets,
) -> ResolvedGameplayPresentation {
    resolve_gameplay_presentation(GameplayPresentationInput {
        display_px: display,
        safe_area_insets: insets,
        profile: profiles::fixed_four_by_three().for_environment(PresentationEnvironment::Desktop),
        occlusions: &[],
        control_footprints: touch_footprints(),
    })
}

/// Whatever rung is chosen, the controls stay inside the device-safe area and
/// the two thumb clusters never sit on top of each other.
#[test]
fn resolved_controls_stay_safe_and_disjoint_on_every_display() {
    let asymmetric_left = ScreenInsets::new(96.0, 0.0, 24.0, 48.0);
    let asymmetric_right = ScreenInsets::new(0.0, 96.0, 24.0, 48.0);

    for &(name, w, h) in DISPLAYS {
        for (inset_name, insets) in [
            ("none", ScreenInsets::ZERO),
            ("left-cutout", asymmetric_left),
            ("right-cutout", asymmetric_right),
        ] {
            let resolved = resolve_controls(ae::Vec2::new(w, h), insets);
            let safe = resolved.display_safe_rect;
            let controls = &resolved.controls;

            for placed in controls.placed() {
                assert!(
                    placed.rect.min.x >= safe.min.x - 0.01
                        && placed.rect.min.y >= safe.min.y - 0.01
                        && placed.rect.max.x <= safe.max.x + 0.01
                        && placed.rect.max.y <= safe.max.y + 0.01,
                    "{name}/{inset_name}: control {:?} escaped the safe area {safe:?}",
                    placed.rect,
                );
                assert!(placed.scale > 0.0 && placed.scale <= 1.0);
            }

            if let (Some(movement), Some(actions)) = (controls.movement, controls.primary_actions) {
                assert!(
                    !movement.rect.overlaps(actions.rect),
                    "{name}/{inset_name}: thumb clusters overlap each other",
                );
            }
        }
    }
}

/// The load-bearing honesty property: when the layout CLAIMS a reserved
/// placement, no control is on top of the world. This is the bug the review
/// found — decorative sidebars while the controls still anchored to the window.
#[test]
fn a_reserved_placement_never_overlaps_gameplay() {
    for &(name, w, h) in DISPLAYS {
        let resolved = resolve_controls(ae::Vec2::new(w, h), ScreenInsets::ZERO);
        let gameplay = resolved.gameplay_rect;

        if resolved.controls.placement.is_fully_reserved() {
            for placed in resolved.controls.placed() {
                assert!(
                    placed.reserved && !placed.rect.overlaps(gameplay),
                    "{name}: {:?} claims a reserved placement but covers gameplay {gameplay:?}",
                    resolved.controls.placement,
                );
            }
        }
        // Every SURROUND-anchored cluster must clear the world — that is the
        // placement math under test, not a restatement of the definition.
        for placed in resolved.controls.placed() {
            if placed.anchor == ControlAnchor::Surround {
                assert!(
                    !placed.rect.overlaps(gameplay),
                    "{name}: a surround-anchored control covers gameplay {gameplay:?}",
                );
            }
            assert_eq!(
                placed.reserved,
                !placed.rect.overlaps(gameplay),
                "{name}: `reserved` must report the geometric truth",
            );
        }
    }
}

/// A 20:9 phone leaves 480px of side surround, which fits both thumb clusters
/// at full size. This is the case the Mary-O profile is FOR.
#[test]
fn a_wide_display_reserves_both_thumb_clusters_at_full_size() {
    let resolved = resolve_controls(ae::Vec2::new(2400.0, 1080.0), ScreenInsets::ZERO);
    assert_eq!(resolved.controls.placement, ControlPlacement::ReservedSurround);

    let movement = resolved.controls.movement.expect("movement placed");
    let actions = resolved.controls.primary_actions.expect("actions placed");
    assert_eq!(movement.scale, 1.0);
    assert_eq!(actions.scale, 1.0);
    assert!(movement.rect.max.x <= resolved.gameplay_rect.min.x + 0.01, "movement is left of play");
    assert!(actions.rect.min.x >= resolved.gameplay_rect.max.x - 0.01, "actions are right of play");
}

/// The review's exact counterexample. At 1920x1200 a 4:3 viewport leaves 160px
/// per side, which holds neither cluster above its usability floor. The
/// documented answer is the ordinary overlay, reported as such — never a
/// reserved claim over controls that are actually covering the world.
#[test]
fn insufficient_space_selects_the_documented_fallback() {
    let resolved = resolve_controls(ae::Vec2::new(1920.0, 1200.0), ScreenInsets::ZERO);
    assert_eq!(resolved.gameplay_rect.size(), ae::Vec2::new(1600.0, 1200.0));
    assert_eq!(
        resolved.surround_rect(SurroundRegion::Left).map(|r| r.width()),
        Some(160.0),
    );

    assert_eq!(resolved.controls.placement, ControlPlacement::Overlay);
    for placed in resolved.controls.placed() {
        assert_eq!(placed.anchor, ControlAnchor::Overlay);
        assert_eq!(placed.scale, 1.0, "an overlaid cluster is not shrunk");
    }
}

/// Rung 2: the action cluster compacts into a column too narrow for it at full
/// size, while the stick still fits. Both stay off the world.
#[test]
fn a_slightly_narrow_surround_compacts_rather_than_overlaying() {
    // 1440 gameplay + 220 per side.
    let resolved = resolve_controls(ae::Vec2::new(1880.0, 1080.0), ScreenInsets::ZERO);
    assert_eq!(
        resolved.surround_rect(SurroundRegion::Left).map(|r| r.width()),
        Some(220.0),
    );
    assert_eq!(resolved.controls.placement, ControlPlacement::CompactSurround);

    let movement = resolved.controls.movement.expect("movement placed");
    let actions = resolved.controls.primary_actions.expect("actions placed");
    assert_eq!(movement.scale, 1.0, "the stick is never compacted");
    assert!(actions.scale < 1.0 && actions.scale >= 0.893, "actions compacted to the floor");
    for placed in resolved.controls.placed() {
        assert!(placed.reserved, "a compact placement is still a reserved one");
    }
}

/// Rung 3: a column that holds the compacted action cluster but not the stick.
#[test]
fn a_column_that_fits_only_one_cluster_goes_hybrid() {
    // 1440 gameplay + 209 per side: below the stick's fixed 210, above the
    // action cluster's 208.1 floor.
    let resolved = resolve_controls(ae::Vec2::new(1858.0, 1080.0), ScreenInsets::ZERO);
    assert_eq!(resolved.controls.placement, ControlPlacement::HybridSurround);

    let movement = resolved.controls.movement.expect("movement placed");
    let actions = resolved.controls.primary_actions.expect("actions placed");
    assert_eq!(movement.anchor, ControlAnchor::Overlay);
    assert_eq!(actions.anchor, ControlAnchor::Surround);
    assert!(actions.reserved);
}

/// A display with no surround at all degrades to the pre-existing overlay,
/// which is the behavior every game had before reserved surrounds existed.
#[test]
fn a_display_with_no_surround_falls_back_to_overlay() {
    let resolved = resolve_controls(ae::Vec2::new(1024.0, 768.0), ScreenInsets::ZERO);
    assert!(!resolved.has_surround(), "4:3 on 4:3 leaves nothing");
    assert_eq!(resolved.controls.placement, ControlPlacement::Overlay);
    for placed in resolved.controls.placed() {
        assert!(!placed.reserved);
        assert_eq!(placed.scale, 1.0);
    }
}

/// A profile that does not ask for reserved controls keeps them overlaid even
/// when surround exists — controls and HUD are separate axes.
#[test]
fn an_overlay_policy_ignores_available_surround() {
    let profile = GameplayPresentationProfile::fixed_aspect(4.0, 3.0)
        .with_control_placement(ControlPlacementPolicy::Overlay);
    let resolved = resolve_gameplay_presentation(GameplayPresentationInput {
        display_px: ae::Vec2::new(2400.0, 1080.0),
        safe_area_insets: ScreenInsets::ZERO,
        profile: &profile,
        occlusions: &[],
        control_footprints: touch_footprints(),
    });

    assert!(resolved.has_surround(), "the surround is there to be ignored");
    assert_eq!(resolved.controls.placement, ControlPlacement::Overlay);
}

/// A session with no virtual controls places nothing and says so, rather than
/// reporting an overlay that does not exist.
#[test]
fn no_published_footprints_places_nothing() {
    let resolved = resolve_gameplay_presentation(GameplayPresentationInput {
        display_px: ae::Vec2::new(2400.0, 1080.0),
        safe_area_insets: ScreenInsets::ZERO,
        profile: profiles::fixed_four_by_three().for_environment(PresentationEnvironment::Desktop),
        occlusions: &[],
        control_footprints: ControlFootprints::default(),
    });
    assert_eq!(resolved.controls.placement, ControlPlacement::NoControls);
    assert_eq!(resolved.controls.placed().count(), 0);
}

/// HUD keeps a usable band in the surround above a bottom-hugging control.
#[test]
fn hud_zones_survive_alongside_reserved_controls() {
    let resolved = resolve_controls(ae::Vec2::new(2400.0, 1080.0), ScreenInsets::ZERO);
    assert_eq!(resolved.controls.placement, ControlPlacement::ReservedSurround);

    let hud = &resolved.controls.hud;
    assert!(!hud.is_empty(), "PreferSurround HUD must get placement zones");
    for zone in hud {
        for placed in resolved.controls.placed() {
            assert!(
                !zone.rect.overlaps(placed.rect),
                "HUD zone {:?} collides with a control {:?}",
                zone.rect,
                placed.rect,
            );
        }
        assert!(
            !zone.rect.overlaps(resolved.gameplay_rect),
            "a surround HUD zone must not cover gameplay",
        );
    }
}

/// An asymmetric cutout moves the safe area, and the controls move with it —
/// they anchor to the SAFE display, not the raw one.
#[test]
fn controls_follow_an_asymmetric_safe_area() {
    let bare = resolve_controls(ae::Vec2::new(1024.0, 768.0), ScreenInsets::ZERO);
    let inset = resolve_controls(
        ae::Vec2::new(1024.0, 768.0),
        ScreenInsets::new(80.0, 0.0, 0.0, 40.0),
    );

    let bare_movement = bare.controls.movement.expect("movement placed");
    let inset_movement = inset.controls.movement.expect("movement placed");
    assert_eq!(bare_movement.rect.min.x, 0.0);
    assert_eq!(inset_movement.rect.min.x, 80.0, "left cutout pushes the stick right");
    assert!(
        inset_movement.rect.max.y < bare_movement.rect.max.y,
        "a bottom inset lifts the stick off the gesture bar",
    );
}

/// Occlusion composition must be invariant under EVERY input permutation, not
/// just a few sampled ones.
///
/// Carving is sequential, so the canonical sort is what makes the result
/// order-independent — and a sort key is only canonical if it is complete.
/// This fixture is the counterexample the incomplete key admitted: several
/// rectangles sharing a bottom-left corner (and several sharing a bottom-right
/// one) with DIFFERENT extents. Under a purpose+min key those compare equal, a
/// stable sort preserves ECS iteration order, and the composed safe region
/// depends on which entity happened to be spawned first.
#[test]
fn occlusion_composition_is_invariant_under_every_permutation() {
    let profile = GameplayPresentationProfile::full_bleed()
        .with_occlusion_aware_framing(SoftFramingProfile::platformer());
    let display = ae::Vec2::new(2400.0, 1080.0);

    let rect = |min: ae::Vec2, size: ae::Vec2| ScreenRect::from_min_size(min, size);
    let occlusion = |purpose, min, size| ScreenOcclusion {
        purpose,
        rect: rect(min, size),
    };

    // Three share the corner (300, 700); two more share (1900, 640). Same
    // purpose, same minimum, different maximum — invisible to the old key.
    let corner = ae::Vec2::new(300.0, 700.0);
    let far_corner = ae::Vec2::new(1900.0, 640.0);
    let set = [
        occlusion(
            ScreenOcclusionPurpose::VirtualMovementStick,
            corner,
            ae::Vec2::new(260.0, 300.0),
        ),
        occlusion(
            ScreenOcclusionPurpose::VirtualMovementStick,
            corner,
            ae::Vec2::new(420.0, 180.0),
        ),
        occlusion(
            ScreenOcclusionPurpose::VirtualMovementStick,
            corner,
            ae::Vec2::new(180.0, 360.0),
        ),
        occlusion(
            ScreenOcclusionPurpose::VirtualActionCluster,
            far_corner,
            ae::Vec2::new(340.0, 300.0),
        ),
        occlusion(
            ScreenOcclusionPurpose::VirtualActionCluster,
            far_corner,
            ae::Vec2::new(240.0, 420.0),
        ),
    ];

    let baseline = resolve_gameplay_presentation(GameplayPresentationInput {
        display_px: display,
        safe_area_insets: ScreenInsets::ZERO,
        profile: &profile,
        occlusions: &set,
        control_footprints: ControlFootprints::default(),
    })
    .subject_safe_rect;

    // The fixture must actually carve, or invariance would be trivial.
    let unoccluded = resolve_gameplay_presentation(GameplayPresentationInput {
        display_px: display,
        safe_area_insets: ScreenInsets::ZERO,
        profile: &profile,
        occlusions: &[],
        control_footprints: ControlFootprints::default(),
    })
    .subject_safe_rect;
    assert_ne!(baseline, unoccluded, "the fixture must actually shrink the region");

    // All 120 orderings of five occluders.
    let mut indices = [0usize, 1, 2, 3, 4];
    let mut permutations = 0usize;
    permute(&mut indices, 0, &mut |order| {
        permutations += 1;
        let permuted: Vec<ScreenOcclusion> = order.iter().map(|&i| set[i]).collect();
        let resolved = resolve_gameplay_presentation(GameplayPresentationInput {
            display_px: display,
            safe_area_insets: ScreenInsets::ZERO,
            profile: &profile,
            occlusions: &permuted,
            control_footprints: ControlFootprints::default(),
        })
        .subject_safe_rect;
        assert_eq!(
            resolved, baseline,
            "ordering {order:?} composed a different safe region",
        );
    });
    assert_eq!(permutations, 120, "every ordering must have been tried");
}

fn permute(items: &mut [usize], start: usize, visit: &mut impl FnMut(&[usize])) {
    if start == items.len() {
        visit(items);
        return;
    }
    for i in start..items.len() {
        items.swap(start, i);
        permute(items, start + 1, visit);
        items.swap(start, i);
    }
}
