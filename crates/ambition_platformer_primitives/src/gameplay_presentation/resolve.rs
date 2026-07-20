//! The pure layout resolver.
//!
//! Depends on nothing but geometry and policy: no window, no rendering, no
//! touch input, no content, no provider. Given a display size, safe-area
//! insets, one profile, and whatever occupancy is currently published, it
//! answers where gameplay renders and where the subject should stay.

use ambition_engine_core as ae;

use super::{
    AspectRatio, FixedAspectFit, GameplayPresentationProfile, GameplayViewportPolicy,
    NamedScreenRect, ResolvedGameplayPresentation, ScreenInsets,
    ScreenOcclusion, ScreenRect, SurroundRegion,
};

/// Pure input bundle for [`resolve_gameplay_presentation`].
pub struct GameplayPresentationInput<'a> {
    /// Physical display size in pixels.
    pub display_px: ae::Vec2,
    /// Platform safe-area insets. Zero when the host cannot supply them.
    pub safe_area_insets: ScreenInsets,
    pub profile: &'a GameplayPresentationProfile,
    /// Currently published occupancy, already resolved to display pixels.
    /// Order does not matter — the resolver sorts internally so the same set
    /// always composes the same region.
    pub occlusions: &'a [ScreenOcclusion],
}

/// Resolve the gameplay presentation layout.
///
/// The subject-safe region is conceptually:
///
/// ```text
/// authored framing region
/// ∩ gameplay viewport
/// ∩ device safe area
/// − active critical occlusions
/// ```
pub fn resolve_gameplay_presentation(
    input: GameplayPresentationInput<'_>,
) -> ResolvedGameplayPresentation {
    let display_px = input.display_px.max(ae::Vec2::ONE);
    let display_rect = ScreenRect::from_min_size(ae::Vec2::ZERO, display_px);

    // Insets that would collapse the display are ignored rather than obeyed:
    // a bad platform report must not black out the game.
    let inset_rect = display_rect.inset(input.safe_area_insets);
    let display_safe_rect = if inset_rect.width() < 1.0 || inset_rect.height() < 1.0 {
        display_rect
    } else {
        inset_rect
    };

    let gameplay_rect = match input.profile.viewport {
        GameplayViewportPolicy::FullBleed => display_safe_rect,
        GameplayViewportPolicy::FixedAspect { aspect, fit } => {
            fit_fixed_aspect(display_safe_rect, aspect, fit)
        }
    };

    let soft_framing = input.profile.framing.profile();
    let subject_safe_rect = match soft_framing {
        None => gameplay_rect,
        Some(profile) => {
            let authored = profile
                .safe_region
                .resolve(gameplay_rect)
                .intersect(display_safe_rect);
            if input.profile.framing.consumes_occlusions() {
                let floor = (gameplay_rect.size() * profile.min_region_fraction.max(ae::Vec2::ZERO))
                    .min(authored.size());
                carve_occlusions(authored, input.occlusions, floor)
            } else {
                authored
            }
        }
    };

    ResolvedGameplayPresentation {
        display_rect,
        display_safe_rect,
        gameplay_rect,
        subject_safe_rect,
        subject_safe_region: subject_safe_rect.normalized_within(gameplay_rect),
        soft_framing,
        surround: input.profile.surround,
        hud: input.profile.hud,
        surround_rects: surround_rects(display_safe_rect, gameplay_rect),
        occlusions: input.occlusions.to_vec(),
    }
}

/// Fit `aspect` inside `safe`, preserving the ratio exactly.
///
/// A display wider than the requested aspect pillarboxes; a narrower one
/// letterboxes. The result is always fully inside `safe`.
fn fit_fixed_aspect(safe: ScreenRect, aspect: AspectRatio, fit: FixedAspectFit) -> ScreenRect {
    let target = aspect.ratio();
    let safe_size = safe.size();
    if safe_size.x <= 0.0 || safe_size.y <= 0.0 {
        return safe;
    }
    let size = if safe_size.x / safe_size.y > target {
        // Wider than requested: height is the binding constraint.
        ae::Vec2::new(safe_size.y * target, safe_size.y)
    } else {
        ae::Vec2::new(safe_size.x, safe_size.x / target)
    };
    let slack = (safe_size - size).max(ae::Vec2::ZERO);
    let min = match fit {
        FixedAspectFit::Center => safe.min + slack * 0.5,
        FixedAspectFit::Top => ae::Vec2::new(safe.min.x + slack.x * 0.5, safe.min.y),
        FixedAspectFit::Bottom => ae::Vec2::new(safe.min.x + slack.x * 0.5, safe.min.y + slack.y),
    };
    ScreenRect::from_min_size(min, size)
}

/// Reduce `region` so it clears every occlusion that reserves subject space.
///
/// The public contract permits real rectangles, but the *product* stays one
/// rectangle: a region is what a camera deadzone can consume. Each occluder is
/// cleared by insetting whichever single side costs the least area, so a
/// corner thumbstick trims a corner-ward strip rather than an entire half of
/// the display.
///
/// Carving stops rather than shrinking the region below `floor` — dense
/// occupancy must degrade to "the subject may overlap a control", never to
/// "the camera pins the subject to a sliver".
fn carve_occlusions(
    region: ScreenRect,
    occlusions: &[ScreenOcclusion],
    floor: ae::Vec2,
) -> ScreenRect {
    let mut ordered: Vec<&ScreenOcclusion> = occlusions
        .iter()
        .filter(|occlusion| occlusion.purpose.reserves_subject_space())
        .collect();
    // Stable composition: the same set of regions must always produce the same
    // result regardless of the order entities happened to be iterated in.
    ordered.sort_by(|a, b| {
        a.purpose
            .cmp(&b.purpose)
            .then_with(|| a.rect.min.x.total_cmp(&b.rect.min.x))
            .then_with(|| a.rect.min.y.total_cmp(&b.rect.min.y))
    });

    let mut region = region;
    for occlusion in ordered {
        region = carve_one(region, occlusion.rect, floor);
    }
    region
}

fn carve_one(region: ScreenRect, occluder: ScreenRect, floor: ae::Vec2) -> ScreenRect {
    if occluder.is_empty() || !region.overlaps(occluder) {
        return region;
    }

    // How far each side must move inward to clear the occluder.
    let candidates = [
        (occluder.max.x - region.min.x, Side::Left),
        (region.max.x - occluder.min.x, Side::Right),
        (occluder.max.y - region.min.y, Side::Top),
        (region.max.y - occluder.min.y, Side::Bottom),
    ];
    let Some(&(cost, side)) = candidates
        .iter()
        .filter(|(cost, _)| *cost > 0.0)
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
    else {
        return region;
    };

    let carved = match side {
        Side::Left => ScreenRect {
            min: ae::Vec2::new(region.min.x + cost, region.min.y),
            max: region.max,
        },
        Side::Right => ScreenRect {
            min: region.min,
            max: ae::Vec2::new(region.max.x - cost, region.max.y),
        },
        Side::Top => ScreenRect {
            min: ae::Vec2::new(region.min.x, region.min.y + cost),
            max: region.max,
        },
        Side::Bottom => ScreenRect {
            min: region.min,
            max: ae::Vec2::new(region.max.x, region.max.y - cost),
        },
    };

    let size = carved.size();
    if size.x < floor.x || size.y < floor.y {
        region
    } else {
        carved
    }
}

#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

/// The display regions left over outside the gameplay rectangle.
///
/// Left/right span the full safe height; top/bottom span only the gameplay
/// rectangle's width, so the four regions never overlap.
fn surround_rects(safe: ScreenRect, gameplay: ScreenRect) -> Vec<NamedScreenRect> {
    let mut out = Vec::new();
    let mut push = |region: SurroundRegion, rect: ScreenRect| {
        if rect.width() > 0.5 && rect.height() > 0.5 {
            out.push(NamedScreenRect { region, rect });
        }
    };

    push(
        SurroundRegion::Left,
        ScreenRect::from_corners(safe.min, ae::Vec2::new(gameplay.min.x, safe.max.y)),
    );
    push(
        SurroundRegion::Right,
        ScreenRect::from_corners(ae::Vec2::new(gameplay.max.x, safe.min.y), safe.max),
    );
    push(
        SurroundRegion::Top,
        ScreenRect::from_corners(
            ae::Vec2::new(gameplay.min.x, safe.min.y),
            ae::Vec2::new(gameplay.max.x, gameplay.min.y),
        ),
    );
    push(
        SurroundRegion::Bottom,
        ScreenRect::from_corners(
            ae::Vec2::new(gameplay.min.x, gameplay.max.y),
            ae::Vec2::new(gameplay.max.x, safe.max.y),
        ),
    );
    out
}
