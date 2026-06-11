//! Runtime selection between the compiled-in portal transit **visual
//! effects**, for live A/B comparison and profiling (the view windows cost
//! extra render passes; on constrained targets the host needs to measure that
//! against the cheap legacy masks, in the SAME session).
//!
//! Each effect is also a cargo feature (`effect_view_cones` /
//! `effect_transit_masks`), so a build can ship one, both, or neither; the
//! cycle only ever offers what was compiled (plus `Off` — the bare exit copy —
//! as the profiling baseline). The host surfaces [`PortalEffectSelection`] in
//! its developer menu (in Ambition: the Developer screen's "Portal FX" row).

use bevy::prelude::*;

/// One portal transit visual effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortalVisualEffect {
    /// No effect beyond the always-on exit copy of the transiting body — the
    /// profiling baseline.
    Off,
    /// The render-to-texture view windows (viewer-gated cones).
    #[cfg(feature = "effect_view_cones")]
    ViewCones,
    /// The legacy opaque "feet in, feet out" mask boxes over the body slices.
    #[cfg(feature = "effect_transit_masks")]
    TransitMasks,
}

impl PortalVisualEffect {
    /// Every effect compiled into this build, in cycle order. `Off` is always
    /// available (it is the A/B baseline).
    pub const fn compiled() -> &'static [Self] {
        &[
            #[cfg(feature = "effect_view_cones")]
            Self::ViewCones,
            #[cfg(feature = "effect_transit_masks")]
            Self::TransitMasks,
            Self::Off,
        ]
    }

    /// Display label for dev menus / logs.
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off (copy only)",
            #[cfg(feature = "effect_view_cones")]
            Self::ViewCones => "View Cones",
            #[cfg(feature = "effect_transit_masks")]
            Self::TransitMasks => "Transit Masks",
        }
    }
}

/// Which portal visual effect is live right now. The host's developer menu
/// cycles it; systems for inactive effects stand down (the view-cone renderer
/// despawns its capture rigs entirely, so an A/B profile sees the true cost
/// delta, not a hidden-but-still-rendering pass).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortalEffectSelection {
    pub active: PortalVisualEffect,
}

impl Default for PortalEffectSelection {
    fn default() -> Self {
        Self {
            // First compiled effect (view cones when present), else Off.
            active: PortalVisualEffect::compiled()[0],
        }
    }
}

impl PortalEffectSelection {
    /// Advance to the next/previous compiled effect (`dir < 0` ⇒ previous) —
    /// the dev-menu cycle.
    pub fn cycle(&mut self, dir: i32) {
        let all = PortalVisualEffect::compiled();
        let i = all.iter().position(|e| *e == self.active).unwrap_or(0) as i32;
        let n = all.len() as i32;
        let next = if dir < 0 {
            (i + n - 1) % n
        } else {
            (i + 1) % n
        };
        self.active = all[next as usize];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Off` is always offered, the cycle visits every compiled effect, and a
    /// full lap returns to the start.
    #[test]
    fn cycle_covers_all_compiled_effects() {
        let all = PortalVisualEffect::compiled();
        assert!(all.contains(&PortalVisualEffect::Off));
        let mut sel = PortalEffectSelection::default();
        let start = sel.active;
        let mut seen = vec![sel.active];
        for _ in 1..all.len() {
            sel.cycle(1);
            assert!(!seen.contains(&sel.active), "no repeats inside one lap");
            seen.push(sel.active);
        }
        sel.cycle(1);
        assert_eq!(sel.active, start, "a full lap returns to the start");
        sel.cycle(-1);
        assert_eq!(sel.active, *seen.last().unwrap(), "reverse steps back");
    }
}
