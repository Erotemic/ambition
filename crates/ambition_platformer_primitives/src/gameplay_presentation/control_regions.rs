//! Where on-screen controls and HUD actually go.
//!
//! A fixed-aspect profile that "reserves" surround for controls is lying
//! unless something places the controls there. This module is the one resolved
//! source of truth for that placement: producers publish what they NEED
//! ([`ControlFootprints`]), the pure resolver decides where it FITS, and the
//! touch/HUD presenters consume the answer. No presenter infers margins from
//! the window on its own, so the rendered node, the hit region, and the
//! reserved area cannot disagree.
//!
//! # The fallback ladder
//!
//! A 4:3 viewport does not leave usable side surround on every display. At
//! 1920x1200 each side is 160px, while the action cluster wants 233px — so a
//! profile cannot simply assert that controls live in the surround. The ladder
//! is explicit, deterministic, and published as
//! [`ResolvedControlRegions::placement`] so diagnostics and tests can see which
//! rung was taken:
//!
//! 1. [`ControlPlacement::ReservedSurround`] — every cluster at its preferred
//!    size, entirely outside the gameplay rectangle;
//! 2. [`ControlPlacement::CompactSurround`] — every cluster still reserved, but
//!    at least one shrunk toward its minimum usable size;
//! 3. [`ControlPlacement::HybridSurround`] — one side reserved, the other
//!    overlaying gameplay;
//! 4. [`ControlPlacement::Overlay`] — ordinary corner-anchored overlay, which
//!    is what every game got before reserved surrounds existed.
//!
//! Never silently overlap gameplay while claiming reserved placement: an
//! overlapping cluster is reported as `reserved: false`, and the placement rung
//! says so.

use ambition_engine_core as ae;
use bevy::prelude::Resource;

use super::{NamedScreenRect, ScreenRect, SurroundRegion};

/// What one control cluster needs, in logical pixels.
///
/// `minimum` is a USABILITY floor, not a geometric one: below it the control
/// still draws but its touch targets stop being reliably hittable, so the
/// resolver prefers overlaying gameplay to shrinking past it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlFootprint {
    pub preferred: ae::Vec2,
    pub minimum: ae::Vec2,
}

impl ControlFootprint {
    pub fn new(preferred: ae::Vec2, minimum: ae::Vec2) -> Self {
        Self {
            preferred: preferred.max(ae::Vec2::ZERO),
            minimum: minimum.max(ae::Vec2::ZERO).min(preferred.max(ae::Vec2::ZERO)),
        }
    }

    /// A cluster that must not be scaled at all.
    pub fn fixed(size: ae::Vec2) -> Self {
        Self::new(size, size)
    }

    /// The largest uniform scale that fits this footprint inside `available`,
    /// or `None` if even the minimum does not fit.
    fn scale_within(self, available: ae::Vec2) -> Option<f32> {
        if self.preferred.x <= 0.0 || self.preferred.y <= 0.0 {
            return None;
        }
        let fit = (available.x / self.preferred.x).min(available.y / self.preferred.y);
        let scale = fit.min(1.0);
        let scaled = self.preferred * scale;
        (scaled.x >= self.minimum.x && scaled.y >= self.minimum.y).then_some(scale)
    }
}

/// What the on-screen controls need this frame.
///
/// Published by whatever draws them (the touch presenter today). Absent slots
/// simply are not placed, so a session with no virtual controls costs nothing
/// and reserves nothing.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct ControlFootprints {
    pub movement: Option<ControlFootprint>,
    pub primary_actions: Option<ControlFootprint>,
    pub system_controls: Option<ControlFootprint>,
}

impl ControlFootprints {
    pub fn is_empty(&self) -> bool {
        self.movement.is_none() && self.primary_actions.is_none() && self.system_controls.is_none()
    }
}

/// Whether a profile wants its controls kept out of the gameplay rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlPlacementPolicy {
    /// Corner-anchored overlay on the gameplay view. The behavior every game
    /// had before reserved surrounds existed.
    #[default]
    Overlay,
    /// Place controls in the surround when the resolved layout leaves enough
    /// of it, falling back down the ladder when it does not.
    PreferSurround,
}

/// Which rung of the fallback ladder the resolved layout took.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlPlacement {
    /// Nothing published a footprint, so nothing was placed.
    #[default]
    NoControls,
    /// Every cluster reserved, every cluster at preferred size.
    ReservedSurround,
    /// Every cluster reserved, at least one shrunk toward its minimum.
    CompactSurround,
    /// At least one cluster reserved, at least one overlaying gameplay.
    HybridSurround,
    /// Ordinary corner-anchored overlay.
    Overlay,
}

impl ControlPlacement {
    /// Whether every placed cluster is outside the gameplay rectangle.
    pub fn is_fully_reserved(self) -> bool {
        matches!(self, Self::ReservedSurround | Self::CompactSurround)
    }
}

/// How a cluster came to be where it is.
///
/// Distinct from [`PlacedControl::reserved`], which is the GEOMETRIC fact.
/// The two can disagree honestly: on a very wide display an overlay-anchored
/// cluster in the display corner may happen to miss the gameplay rectangle
/// entirely. The ladder rung describes the strategy that ran, so it must be
/// decided by the strategy, not rediscovered from the result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlAnchor {
    /// Placed inside a reserved surround column.
    Surround,
    /// Corner-anchored on the device-safe display.
    Overlay,
}

/// One cluster's resolved placement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedControl {
    pub rect: ScreenRect,
    pub anchor: ControlAnchor,
    /// True when this cluster is entirely outside the gameplay rectangle.
    /// A presenter may draw such a cluster without any gameplay-dimming
    /// treatment; a cluster that is not reserved sits over the world.
    pub reserved: bool,
    /// `rect.size() / footprint.preferred`. Presenters scale their internal
    /// layout by this, so the rendered control and its hit regions shrink
    /// together instead of drifting apart.
    pub scale: f32,
}

/// THE resolved control and HUD placement.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResolvedControlRegions {
    pub placement: ControlPlacement,
    pub movement: Option<PlacedControl>,
    pub primary_actions: Option<PlacedControl>,
    pub system_controls: Option<PlacedControl>,
    /// Surround area left over for HUD once controls have taken theirs.
    pub hud: Vec<NamedScreenRect>,
}

/// Place controls and HUD against a resolved layout.
pub(super) fn resolve_control_regions(
    display_safe: ScreenRect,
    gameplay: ScreenRect,
    surround: &[NamedScreenRect],
    footprints: ControlFootprints,
    policy: ControlPlacementPolicy,
) -> ResolvedControlRegions {
    if footprints.is_empty() {
        return ResolvedControlRegions {
            placement: ControlPlacement::NoControls,
            hud: hud_regions(surround, &[]),
            ..ResolvedControlRegions::default()
        };
    }

    let side = |region| {
        surround
            .iter()
            .find(|named| named.region == region)
            .map(|named| named.rect)
            .filter(|_| policy == ControlPlacementPolicy::PreferSurround)
    };

    // Thumbs rest at the bottom, so a reserved cluster hugs the bottom of its
    // surround column rather than centering in a tall bar.
    let movement = footprints
        .movement
        .and_then(|footprint| place_in_column(footprint, side(SurroundRegion::Left)))
        .or_else(|| {
            footprints
                .movement
                .map(|footprint| overlay(footprint, display_safe, Corner::BottomLeft))
        });
    let primary_actions = footprints
        .primary_actions
        .and_then(|footprint| place_in_column(footprint, side(SurroundRegion::Right)))
        .or_else(|| {
            footprints
                .primary_actions
                .map(|footprint| overlay(footprint, display_safe, Corner::BottomRight))
        });

    // System chrome prefers the TOP of the right surround, above the action
    // cluster it shares that column with.
    let system_controls = footprints
        .system_controls
        .and_then(|footprint| {
            let column = side(SurroundRegion::Right)?;
            let free = match primary_actions {
                Some(actions) if actions.reserved => ScreenRect {
                    min: column.min,
                    max: ae::Vec2::new(column.max.x, actions.rect.min.y),
                },
                _ => column,
            };
            place_in_column_at_top(footprint, free)
        })
        .or_else(|| {
            footprints
                .system_controls
                .map(|footprint| overlay(footprint, display_safe, Corner::TopRight))
        });

    let placed: Vec<PlacedControl> = [movement, primary_actions, system_controls]
        .into_iter()
        .flatten()
        .collect();
    let taken: Vec<ScreenRect> = placed.iter().map(|control| control.rect).collect();

    ResolvedControlRegions {
        placement: classify(&placed),
        movement,
        primary_actions,
        system_controls,
        hud: hud_regions(surround, &taken),
    }
    .with_reserved_recomputed(gameplay)
}

impl ResolvedControlRegions {
    /// `reserved` answers the ONE question that matters to a participant: does
    /// this control sit on top of the world? It is measured against the actual
    /// gameplay rectangle rather than assumed from the branch that placed it,
    /// so an overlay-anchored cluster that happens to miss the world reports
    /// the truth. The ladder rung stays a property of the STRATEGY.
    fn with_reserved_recomputed(mut self, gameplay: ScreenRect) -> Self {
        for control in [
            &mut self.movement,
            &mut self.primary_actions,
            &mut self.system_controls,
        ]
        .into_iter()
        .flatten()
        {
            control.reserved = !control.rect.overlaps(gameplay);
        }
        let placed: Vec<PlacedControl> = [self.movement, self.primary_actions, self.system_controls]
            .into_iter()
            .flatten()
            .collect();
        self.placement = classify(&placed);
        self
    }

    /// Every placed cluster, for callers that just want to iterate.
    pub fn placed(&self) -> impl Iterator<Item = PlacedControl> + '_ {
        [self.movement, self.primary_actions, self.system_controls]
            .into_iter()
            .flatten()
    }
}

fn classify(placed: &[PlacedControl]) -> ControlPlacement {
    if placed.is_empty() {
        return ControlPlacement::NoControls;
    }
    let reserved = placed
        .iter()
        .filter(|control| control.anchor == ControlAnchor::Surround)
        .count();
    let compact = placed.iter().any(|control| control.scale < 1.0 - 1e-4);
    if reserved == placed.len() {
        if compact {
            ControlPlacement::CompactSurround
        } else {
            ControlPlacement::ReservedSurround
        }
    } else if reserved > 0 {
        ControlPlacement::HybridSurround
    } else {
        ControlPlacement::Overlay
    }
}

/// Fit a cluster into the bottom of a surround column.
fn place_in_column(footprint: ControlFootprint, column: Option<ScreenRect>) -> Option<PlacedControl> {
    let column = column?;
    let scale = footprint.scale_within(column.size())?;
    let size = footprint.preferred * scale;
    let min = ae::Vec2::new(
        column.min.x + (column.width() - size.x) * 0.5,
        column.max.y - size.y,
    );
    Some(PlacedControl {
        rect: ScreenRect::from_min_size(min, size),
        anchor: ControlAnchor::Surround,
        reserved: true,
        scale,
    })
}

/// Fit a cluster into the top of a surround column.
fn place_in_column_at_top(
    footprint: ControlFootprint,
    column: ScreenRect,
) -> Option<PlacedControl> {
    let scale = footprint.scale_within(column.size())?;
    let size = footprint.preferred * scale;
    let min = ae::Vec2::new(column.min.x + (column.width() - size.x) * 0.5, column.min.y);
    Some(PlacedControl {
        rect: ScreenRect::from_min_size(min, size),
        anchor: ControlAnchor::Surround,
        reserved: true,
        scale,
    })
}

#[derive(Clone, Copy)]
enum Corner {
    BottomLeft,
    BottomRight,
    TopRight,
}

/// The pre-existing behavior: full-size, anchored to a device-safe corner.
fn overlay(footprint: ControlFootprint, safe: ScreenRect, corner: Corner) -> PlacedControl {
    let size = footprint.preferred;
    let min = match corner {
        Corner::BottomLeft => ae::Vec2::new(safe.min.x, safe.max.y - size.y),
        Corner::BottomRight => ae::Vec2::new(safe.max.x - size.x, safe.max.y - size.y),
        Corner::TopRight => ae::Vec2::new(safe.max.x - size.x, safe.min.y),
    };
    PlacedControl {
        rect: ScreenRect::from_min_size(min, size),
        anchor: ControlAnchor::Overlay,
        reserved: false,
        scale: 1.0,
    }
}

/// Surround area not taken by a control, as HUD placement zones.
///
/// Only the vertical remainder of each surround region is reported: a control
/// hugging the bottom of a tall column leaves a usable band above it, which is
/// exactly where a fixed-aspect game wants its score and lives.
fn hud_regions(surround: &[NamedScreenRect], taken: &[ScreenRect]) -> Vec<NamedScreenRect> {
    let mut out = Vec::new();
    for named in surround {
        let mut rect = named.rect;
        for occupied in taken {
            if !rect.overlaps(*occupied) {
                continue;
            }
            let above = occupied.min.y - rect.min.y;
            let below = rect.max.y - occupied.max.y;
            rect = if above >= below {
                ScreenRect {
                    min: rect.min,
                    max: ae::Vec2::new(rect.max.x, occupied.min.y),
                }
            } else {
                ScreenRect {
                    min: ae::Vec2::new(rect.min.x, occupied.max.y),
                    max: rect.max,
                }
            };
        }
        if rect.width() > 1.0 && rect.height() > 1.0 {
            out.push(NamedScreenRect {
                region: named.region,
                rect,
            });
        }
    }
    out
}
