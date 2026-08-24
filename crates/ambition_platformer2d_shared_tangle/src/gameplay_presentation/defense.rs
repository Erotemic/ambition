//! Route-authored presentation policy for body-defense cues.
//!
//! Gameplay owns hit eligibility. This module only names semantic presentation
//! causes and lets the active route opt those causes into shared engine effects.
//! Character/content effects remain independent, so they compose with shared
//! cues instead of being suppressed by renderer special cases.

use std::collections::BTreeMap;

use bevy::prelude::Resource;

/// Semantic reasons a presentation consumer may want to visualize.
///
/// These are presentation-facing categories, not a second hit-eligibility
/// model. `ambition_sim_view` projects the canonical simulation state into this
/// vocabulary after the damage gate has already answered whether the body can
/// be hit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DefenseCueCauses(u32);

impl DefenseCueCauses {
    pub const NONE: Self = Self(0);

    /// A transformation policy currently makes the body untouchable.
    pub const TRANSFORMING: Self = Self(1 << 0);
    /// A power/super state currently makes the body untouchable.
    pub const EMPOWERED: Self = Self(1 << 1);
    /// Scripted gameplay currently makes the body untouchable.
    pub const SCRIPTED: Self = Self(1 << 2);
    /// An authored move window currently grants intangibility.
    pub const MOVE_IFRAME: Self = Self(1 << 3);
    /// Roll/air-dodge intangibility.
    pub const DODGE: Self = Self(1 << 4);
    /// Ledge intangibility.
    pub const LEDGE: Self = Self(1 << 5);
    /// Ledge/getup option intangibility.
    pub const GETUP: Self = Self(1 << 6);
    /// Perfect-shield/parry invulnerability.
    pub const PARRY: Self = Self(1 << 7);
    /// Post-hit or other `BodyCombat` timed damage i-frames.
    pub const DAMAGE_IFRAME: Self = Self(1 << 8);
    /// Match/ruleset respawn protection.
    pub const RESPAWN: Self = Self(1 << 9);

    /// The engine's ordinary defensive i-frame vocabulary.
    ///
    /// Deliberately excludes transformation, empowerment and scripted
    /// invulnerability: those are states that commonly have their own content
    /// presentation. A route may opt them into a shared cue explicitly.
    pub const SHARED_IFRAMES: Self = Self(
        Self::MOVE_IFRAME.0
            | Self::DODGE.0
            | Self::LEDGE.0
            | Self::GETUP.0
            | Self::PARRY.0
            | Self::DAMAGE_IFRAME.0
            | Self::RESPAWN.0,
    );

    pub const ALL: Self = Self(
        Self::TRANSFORMING.0
            | Self::EMPOWERED.0
            | Self::SCRIPTED.0
            | Self::SHARED_IFRAMES.0,
    );

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn bits(self) -> u32 {
        self.0
    }
}

/// Shared engine-supported defense effects selected by a route.
///
/// Content-specific effects do not need to be represented here. A character
/// quasar, outline or particle effect may read its own semantic state and thus
/// compose naturally with these shared cues.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DefensePresentationCues {
    pub blink: bool,
}

/// Which semantic defense causes opt into each shared presentation effect.
///
/// There is one selector per effect rather than one policy branch that decides
/// all rendering. A future shared outline or ghost effect can be another field
/// without changing hit eligibility or character-owned presentation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DefensePresentationPolicy {
    pub blink_on: DefenseCueCauses,
}

impl DefensePresentationPolicy {
    /// No shared defense effects. This is the route-selection default.
    pub const fn none() -> Self {
        Self {
            blink_on: DefenseCueCauses::NONE,
        }
    }

    /// Opt ordinary defensive i-frames into the shared blink.
    ///
    /// Power, transformation and scripted invulnerability are intentionally not
    /// included; a route can add any of those explicitly with [`Self::with_blink`].
    pub const fn shared_iframe_blink() -> Self {
        Self {
            blink_on: DefenseCueCauses::SHARED_IFRAMES,
        }
    }

    /// Add semantic causes to the shared blink for this route.
    pub const fn with_blink(mut self, causes: DefenseCueCauses) -> Self {
        self.blink_on = self.blink_on.union(causes);
        self
    }

    /// Remove semantic causes from the shared blink for this route.
    pub const fn without_blink(mut self, causes: DefenseCueCauses) -> Self {
        self.blink_on = self.blink_on.without(causes);
        self
    }

    pub const fn resolve(self, causes: DefenseCueCauses) -> DefensePresentationCues {
        DefensePresentationCues {
            blink: self.blink_on.intersects(causes),
        }
    }
}

/// Route-keyed defense presentation declarations.
#[derive(Resource, Default)]
pub struct DefensePresentationCatalog {
    by_route: BTreeMap<String, DefensePresentationPolicy>,
}

impl DefensePresentationCatalog {
    pub fn insert(&mut self, route_id: impl Into<String>, policy: DefensePresentationPolicy) {
        self.by_route.insert(route_id.into(), policy);
    }

    pub fn get(&self, route_id: &str) -> Option<&DefensePresentationPolicy> {
        self.by_route.get(route_id)
    }

    pub fn is_empty(&self) -> bool {
        self.by_route.is_empty()
    }

    pub fn routes(&self) -> impl Iterator<Item = &str> {
        self.by_route.keys().map(String::as_str)
    }
}

/// The active route's shared defense presentation policy.
///
/// Defaults to no shared effect while no declaring gameplay route is active.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveDefensePresentationPolicy(pub DefensePresentationPolicy);

impl Default for ActiveDefensePresentationPolicy {
    fn default() -> Self {
        Self(DefensePresentationPolicy::none())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_iframe_policy_does_not_claim_content_owned_invulnerability() {
        let policy = DefensePresentationPolicy::shared_iframe_blink();
        assert!(!policy.resolve(DefenseCueCauses::EMPOWERED).blink);
        assert!(!policy.resolve(DefenseCueCauses::TRANSFORMING).blink);
        assert!(!policy.resolve(DefenseCueCauses::SCRIPTED).blink);
        assert!(policy.resolve(DefenseCueCauses::MOVE_IFRAME).blink);
        assert!(policy.resolve(DefenseCueCauses::DODGE).blink);
        assert!(policy.resolve(DefenseCueCauses::RESPAWN).blink);
    }

    #[test]
    fn effects_are_opt_in_and_causes_compose() {
        let policy = DefensePresentationPolicy::shared_iframe_blink()
            .with_blink(DefenseCueCauses::EMPOWERED);

        assert!(policy.resolve(DefenseCueCauses::EMPOWERED).blink);
        assert!(
            policy
                .resolve(DefenseCueCauses::EMPOWERED.union(DefenseCueCauses::DODGE))
                .blink
        );

        let content_owned = policy.without_blink(DefenseCueCauses::EMPOWERED);
        assert!(!content_owned.resolve(DefenseCueCauses::EMPOWERED).blink);
        assert!(
            content_owned
                .resolve(DefenseCueCauses::EMPOWERED.union(DefenseCueCauses::DODGE))
                .blink,
            "one content-owned cause must not swallow a simultaneous shared iframe"
        );
    }

    #[test]
    fn route_default_is_explicitly_opt_in() {
        let active = ActiveDefensePresentationPolicy::default();
        assert!(!active.0.resolve(DefenseCueCauses::DODGE).blink);
    }
}
