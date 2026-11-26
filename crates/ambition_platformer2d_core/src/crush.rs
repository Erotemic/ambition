//! Crushing policy (crushing-and-obstruction-response.md, stage C2): how
//! hard a moving surface pushes, how a body gives way, and the game's rule
//! for both, merged most specific first.
//!
//! The data says what a response may do; the stages that act on it read it
//! here (C3: a driver stalls; C4: a body gives way). The defaults reproduce
//! the engine before any response: every surface keeps its schedule, no body
//! deforms, and a one-way support lets go of a body it pinches.

use std::collections::BTreeMap;

/// How a moving surface meets a body it would overconstrain.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SurfaceAuthority {
    /// It stalls on any obstruction (a cardboard lift, a lift with a safety
    /// stop): it keeps the part of its move that overconstrains no body.
    Yielding,
    /// It pushes on until the overconstraint reaches `max_overconstraint_px`
    /// or its force `max_force`, then stalls.
    Limited { max_overconstraint_px: f32, max_force: f32 },
    /// It keeps its schedule (a train, a crusher); the body takes the whole
    /// response.
    Unstoppable,
}

/// The axes along which a body may squash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompressAxes {
    /// Along the crush axis, whichever it is.
    Crush,
    /// Only along the body's own up.
    Vertical,
}

/// A body that squashes before it fails.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Compressibility {
    pub axes: CompressAxes,
    /// The smallest scale along the crush axis, in (0, 1].
    pub min_scale: f32,
    /// How fast it recovers once the pressure lifts (scale per second).
    pub recover_per_s: f32,
    /// Reaching `min_scale` is not death.
    pub survivable: bool,
}

/// What a body does when it cannot (further) compress.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FailureResponse {
    /// Damage per pixel of overconstraint, capped per tick.
    Damage { per_px: f32, per_tick_cap: f32 },
    /// It is destroyed.
    Destroy,
    /// It is ejected along the least overconstrained free axis.
    Displace,
    /// It does not deform and takes nothing: the pinch is reported, as before.
    Rigid,
}

/// How a body resists being crushed.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BodyResistance {
    #[serde(default)]
    pub compress: Option<Compressibility>,
    pub failure: FailureResponse,
}

impl BodyResistance {
    pub const RIGID: Self = Self { compress: None, failure: FailureResponse::Rigid };
}

/// The game's rule, under every surface and body that says nothing.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameCrushPolicy {
    pub default_authority: SurfaceAuthority,
    pub default_resistance: BodyResistance,
    /// A one-way support that pinches a body lets go of it (Mary-O), rather
    /// than crushing it.
    pub one_way_yields: bool,
}

impl Default for GameCrushPolicy {
    fn default() -> Self {
        Self { default_authority: SurfaceAuthority::Unstoppable, default_resistance: BodyResistance::RIGID,
            one_way_yields: true }
    }
}

/// The three levels of crushing policy: the game's rule, and the surfaces
/// and bodies it names. A surface or body's own authored value, when it has
/// one, is more specific still.
#[derive(bevy_ecs::prelude::Resource, Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CrushPolicy {
    #[serde(default)]
    pub game: GameCrushPolicy,
    /// By surface name (a moving platform's authored name).
    #[serde(default)]
    pub surfaces: BTreeMap<String, SurfaceAuthority>,
    /// By body key (a character's catalog id).
    #[serde(default)]
    pub bodies: BTreeMap<String, BodyResistance>,
}

/// Why a crushing policy was refused.
#[derive(Clone, Debug, PartialEq)]
pub enum CrushPolicyError {
    UnknownSurface(String),
    UnknownBody(String),
    /// A number out of its range (`what` names it).
    Invalid { what: String },
}

impl std::fmt::Display for CrushPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSurface(name) => write!(f, "the crush policy names an unknown surface `{name}`"),
            Self::UnknownBody(name) => write!(f, "the crush policy names an unknown body `{name}`"),
            Self::Invalid { what } => write!(f, "the crush policy has an invalid {what}"),
        }
    }
}

impl std::error::Error for CrushPolicyError {}

impl CrushPolicy {
    /// The authority of a surface: its own, the one the policy names it
    /// with, or the game's.
    pub fn authority(&self, surface: &str, own: Option<SurfaceAuthority>) -> SurfaceAuthority {
        own.or_else(|| self.surfaces.get(surface).copied()).unwrap_or(self.game.default_authority)
    }

    /// The resistance of a body: its own, the one the policy names it with,
    /// or the game's.
    pub fn resistance(&self, body: &str, own: Option<BodyResistance>) -> BodyResistance {
        own.or_else(|| self.bodies.get(body).copied()).unwrap_or(self.game.default_resistance)
    }

    /// Refuse a policy that names a surface or body the game does not have,
    /// or holds a number out of range.
    pub fn validate<'a>(
        &self,
        known_surfaces: impl IntoIterator<Item = &'a str>,
        known_bodies: impl IntoIterator<Item = &'a str>,
    ) -> Result<(), CrushPolicyError> {
        let surfaces: Vec<&str> = known_surfaces.into_iter().collect();
        let bodies: Vec<&str> = known_bodies.into_iter().collect();
        if let Some(name) = self.surfaces.keys().find(|name| !surfaces.contains(&name.as_str())) {
            return Err(CrushPolicyError::UnknownSurface(name.clone()));
        }
        if let Some(name) = self.bodies.keys().find(|name| !bodies.contains(&name.as_str())) {
            return Err(CrushPolicyError::UnknownBody(name.clone()));
        }
        let authorities = std::iter::once(("game default", &self.game.default_authority))
            .chain(self.surfaces.iter().map(|(name, authority)| (name.as_str(), authority)));
        for (name, authority) in authorities {
            authority.validate().map_err(|what| CrushPolicyError::Invalid { what: format!("{what} of `{name}`") })?;
        }
        let resistances = std::iter::once(("game default", &self.game.default_resistance))
            .chain(self.bodies.iter().map(|(name, resistance)| (name.as_str(), resistance)));
        for (name, resistance) in resistances {
            resistance.validate().map_err(|what| CrushPolicyError::Invalid { what: format!("{what} of `{name}`") })?;
        }
        Ok(())
    }
}

impl SurfaceAuthority {
    /// The overconstraint a surface pushes a body into before it stalls:
    /// none for a yielding one, its limit for a limited one, any for an
    /// unstoppable one.
    pub fn allowance_px(self) -> Option<f32> {
        match self {
            Self::Yielding => Some(0.0),
            Self::Limited { max_overconstraint_px, .. } => Some(max_overconstraint_px),
            Self::Unstoppable => None,
        }
    }

    fn validate(self) -> Result<(), &'static str> {
        match self {
            Self::Limited { max_overconstraint_px, max_force }
                if !(max_overconstraint_px.is_finite() && max_overconstraint_px >= 0.0
                    && max_force.is_finite() && max_force >= 0.0) => Err("limit"),
            _ => Ok(()),
        }
    }
}

impl BodyResistance {
    fn validate(self) -> Result<(), &'static str> {
        if let Some(compress) = self.compress {
            if !(compress.min_scale.is_finite() && compress.min_scale > 0.0 && compress.min_scale <= 1.0) {
                return Err("minimum scale");
            }
            if !(compress.recover_per_s.is_finite() && compress.recover_per_s >= 0.0) {
                return Err("recovery");
            }
        }
        match self.failure {
            FailureResponse::Damage { per_px, per_tick_cap }
                if !(per_px.is_finite() && per_px >= 0.0 && per_tick_cap.is_finite() && per_tick_cap >= 0.0) => Err("damage"),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> CrushPolicy {
        CrushPolicy {
            game: GameCrushPolicy { default_authority: SurfaceAuthority::Limited { max_overconstraint_px: 4.0, max_force: 10.0 },
                ..GameCrushPolicy::default() },
            surfaces: BTreeMap::from([("lift".to_string(), SurfaceAuthority::Yielding)]),
            bodies: BTreeMap::from([("slime".to_string(), BodyResistance { compress: Some(Compressibility {
                axes: CompressAxes::Crush, min_scale: 0.35, recover_per_s: 2.0, survivable: true }),
                failure: FailureResponse::Rigid })]),
        }
    }

    /// The defaults are the engine before any response: surfaces keep their
    /// schedule, bodies are rigid, a one-way lets go.
    #[test]
    fn the_default_policy_changes_nothing() {
        let policy = CrushPolicy::default();
        assert_eq!(policy.authority("anything", None), SurfaceAuthority::Unstoppable);
        assert_eq!(policy.resistance("anyone", None), BodyResistance::RIGID);
        assert!(policy.game.one_way_yields);
    }

    /// Most specific first: a surface's own authority over the one the policy
    /// names it with, over the game's; the same for a body.
    #[test]
    fn the_most_specific_level_wins() {
        let policy = policy();
        assert_eq!(policy.authority("lift", Some(SurfaceAuthority::Unstoppable)), SurfaceAuthority::Unstoppable);
        assert_eq!(policy.authority("lift", None), SurfaceAuthority::Yielding);
        assert_eq!(policy.authority("press", None), policy.game.default_authority);
        let own = BodyResistance { compress: None, failure: FailureResponse::Destroy };
        assert_eq!(policy.resistance("slime", Some(own)), own);
        assert!(policy.resistance("slime", None).compress.is_some());
        assert_eq!(policy.resistance("scout", None), BodyResistance::RIGID);
    }

    /// A policy that names a surface or body the game does not have is
    /// refused, as is a number out of its range.
    #[test]
    fn a_policy_naming_an_unknown_surface_or_body_is_refused() {
        let policy = policy();
        assert_eq!(policy.validate(["lift"], ["slime"]), Ok(()));
        assert_eq!(policy.validate(["press"], ["slime"]), Err(CrushPolicyError::UnknownSurface("lift".into())));
        assert_eq!(policy.validate(["lift"], ["scout"]), Err(CrushPolicyError::UnknownBody("slime".into())));
        let mut bad = policy.clone();
        bad.surfaces.insert("lift".into(), SurfaceAuthority::Limited { max_overconstraint_px: f32::NAN, max_force: 1.0 });
        assert!(matches!(bad.validate(["lift"], ["slime"]), Err(CrushPolicyError::Invalid { .. })));
        let mut bad = policy;
        bad.bodies.get_mut("slime").unwrap().compress.as_mut().unwrap().min_scale = 0.0;
        assert!(matches!(bad.validate(["lift"], ["slime"]), Err(CrushPolicyError::Invalid { .. })));
    }
}
