//! A light about an actor: what a glow is, and which bodies a route makes glow.
//!
//! Presentation only. A glow is not rollback state and has no effect on play.
//! [`ActorGlow`] is a component on a body's visual (the entity that carries
//! its sprite); the renderer draws it and keeps it at the body's drawn place.
//! Any game, content system or catalog facet can put one on a visual.
//!
//! A route can also say "the bodies the players drive glow"
//! ([`PlayerGlowCatalog`], selected into [`ActivePlayerGlow`]), with the same
//! route-selection shape as [`super::DefensePresentationCatalog`]: one binary
//! can host more than one game, so build order cannot be the authority.
//!
//! ⛔ A glow is LIGHT. It is never dark and it is never pinned at the feet:
//! a dark soft ellipse under a body is a drop shadow, which the art style
//! forbids. The renderer draws [`ActorGlow::color`] as a hue at full
//! brightness, so no value of it can darken what is behind the body.

use std::collections::BTreeMap;

use bevy::prelude::{Component, Resource};

/// Where a glow is drawn in depth, against its body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GlowLayer {
    /// Behind the body: a halo that its silhouette stands in front of.
    #[default]
    Behind,
    /// Over the body: a wash of light on it. Under every cue drawn on the
    /// body (the hit flash), so a strike still reads.
    Over,
}

/// How a glow is drawn. Each look is a separate cost on a device.
///
/// This enum is the seam for a better look. A new look is a new variant and
/// a new arm in the renderer; the data that says "this body glows" does not
/// change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GlowLook {
    /// One sprite of a soft round picture, tinted, alpha-blended. It batches
    /// with the other sprites and needs no shader: every quality tier draws it.
    #[default]
    SoftDisc,
}

/// A slow rise and fall of a glow's intensity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlowPulse {
    /// One cycle, in sim ticks. Sim-derived, so a capture, a replay and the
    /// screen show the same phase.
    pub period_ticks: u32,
    /// How much of the intensity the pulse takes away at its low point, `0..=1`.
    pub depth: f32,
}

/// A light about one actor. Put it on the actor's visual entity.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ActorGlow {
    /// The hue of the light, sRGB `0..=1`. Only the hue counts: the renderer
    /// scales it to full brightness, and black is drawn as white.
    pub color: [f32; 3],
    /// The radius of the light, in heights of the body's drawn frame.
    pub radius: f32,
    /// The opacity of the light at its middle, `0..=1`.
    pub intensity: f32,
    /// Behind the body or over it.
    pub layer: GlowLayer,
    /// The middle of the light from the middle of the drawn frame, in heights
    /// of that frame, `+y` up. `x` is toward the body's facing.
    pub offset: [f32; 2],
    pub pulse: Option<GlowPulse>,
    pub look: GlowLook,
}

impl ActorGlow {
    /// A white light behind the body, the given radius and intensity.
    pub const fn soft(radius: f32, intensity: f32) -> Self {
        Self {
            color: [1.0, 1.0, 1.0],
            radius,
            intensity,
            layer: GlowLayer::Behind,
            offset: [0.0, 0.0],
            pulse: None,
            look: GlowLook::SoftDisc,
        }
    }

    /// The light that marks the body a player drives: pale, faint, behind
    /// it, and a little larger than its frame, so the body is easy to find in
    /// a busy room and its silhouette stays the same.
    pub const fn player_highlight() -> Self {
        Self {
            color: [0.80, 0.92, 1.0],
            ..Self::soft(0.85, 0.42)
        }
    }

    pub const fn with_color(mut self, color: [f32; 3]) -> Self {
        self.color = color;
        self
    }

    pub const fn with_layer(mut self, layer: GlowLayer) -> Self {
        self.layer = layer;
        self
    }

    pub const fn with_offset(mut self, offset: [f32; 2]) -> Self {
        self.offset = offset;
        self
    }

    pub const fn with_pulse(mut self, pulse: GlowPulse) -> Self {
        self.pulse = Some(pulse);
        self
    }

    /// The intensity on sim tick `tick`, with the pulse applied.
    pub fn intensity_at(&self, tick: u64) -> f32 {
        let base = self.intensity.clamp(0.0, 1.0);
        let Some(pulse) = self.pulse.filter(|pulse| pulse.period_ticks > 0) else {
            return base;
        };
        let phase = (tick % u64::from(pulse.period_ticks)) as f32 / pulse.period_ticks as f32;
        // 1 at the top of the cycle, 0 at its low point.
        let wave = 0.5 + 0.5 * (phase * std::f32::consts::TAU).cos();
        base * (1.0 - pulse.depth.clamp(0.0, 1.0) * (1.0 - wave))
    }

    /// The hue at full brightness: its largest channel is 1. Black, or a hue
    /// that is not a number, is white.
    pub fn light_color(&self) -> [f32; 3] {
        let [r, g, b] = self.color.map(|channel| if channel.is_finite() { channel.max(0.0) } else { 0.0 });
        let peak = r.max(g).max(b);
        if peak <= f32::EPSILON {
            return [1.0, 1.0, 1.0];
        }
        [r / peak, g / peak, b / peak]
    }
}

/// Route-keyed declarations of the glow on the bodies the players drive.
#[derive(Resource, Default)]
pub struct PlayerGlowCatalog {
    by_route: BTreeMap<String, ActorGlow>,
}

impl PlayerGlowCatalog {
    pub fn insert(&mut self, route_id: impl Into<String>, glow: ActorGlow) {
        self.by_route.insert(route_id.into(), glow);
    }

    pub fn remove(&mut self, route_id: &str) -> Option<ActorGlow> {
        self.by_route.remove(route_id)
    }

    pub fn get(&self, route_id: &str) -> Option<&ActorGlow> {
        self.by_route.get(route_id)
    }
}

/// The active route's glow on the bodies the players drive. `None` while no
/// route that declares one is active: no body glows by default.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct ActivePlayerGlow(pub Option<ActorGlow>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glow_is_never_dark() {
        assert_eq!(ActorGlow::soft(1.0, 1.0).with_color([0.0, 0.0, 0.0]).light_color(), [1.0, 1.0, 1.0]);
        let dim_red = ActorGlow::soft(1.0, 1.0).with_color([0.2, 0.05, 0.0]).light_color();
        assert_eq!(dim_red[0], 1.0, "a dim hue is drawn at full brightness");
        assert!((dim_red[1] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn a_pulse_falls_by_its_depth_and_comes_back() {
        let glow = ActorGlow::soft(1.0, 0.8).with_pulse(GlowPulse { period_ticks: 60, depth: 0.5 });
        assert!((glow.intensity_at(0) - 0.8).abs() < 1e-6);
        assert!((glow.intensity_at(30) - 0.4).abs() < 1e-6);
        assert!((glow.intensity_at(60) - 0.8).abs() < 1e-6);
        assert_eq!(ActorGlow::soft(1.0, 0.8).intensity_at(30), 0.8, "no pulse, no change");
    }
}
