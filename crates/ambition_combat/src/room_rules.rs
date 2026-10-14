//! The `room_rules` authored-content schema: the per-game rules of a game's own
//! rooms, as data in its pack.
//!
//! [`DeathRules`], [`StrikeWeightRules`] and [`LandingLagRules`] are rules
//! that each game states for its own rooms (ADR 0033). They were Rust
//! constants in each game's plugin, so a change to one number (a feel call
//! such as Ambition's landing-lag share) was a relink of the host and a
//! restart. In a pack source, the edit is a reload.
//!
//! The data says WHAT the rules are. The composition says WHICH rooms they
//! govern: the game that installs the pack passes the [`RulesScope`] to
//! [`declare_room_rules`], because the same pack is a hosted game in one binary
//! and the whole binary in another.
//!
//! A field that is absent declares nothing, and the engine's answer for an
//! undeclared rule applies (no interlude and no level reset; no heavy hits; the
//! move's own landing lag).

use std::sync::Arc;

use ambition_content_pack::{
    CapabilityId, ContentSchemaHandler, DiagnosticCode, FacetOutcome, FacetSource,
    RuntimeDisposition, SchemaId, SchemaRegistration, SchemaVersion,
};
use bevy::prelude::*;
use serde::Deserialize;

use crate::death_rules::DeathRules;
use crate::landing_lag::LandingLagRules;
use crate::scoped_rules::{DeclareRulesExt as _, DeclaredRules, RulesScope};
use crate::strike_weight::StrikeWeightRules;

/// The capability that owns this schema.
pub const ROOM_RULES_CAPABILITY: &str = "combat";

/// The authored FILE kind: one game's rules for its own rooms.
pub const ROOM_RULES_SCHEMA: &str = "room_rules";

/// The schema version this handler reads.
pub const ROOM_RULES_VERSION: SchemaVersion = SchemaVersion(1);

/// The authored document. Each field is one rule, absent when the game states
/// none.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct AuthoredRoomRules {
    /// Seconds a death holds before the level goes back to how it started,
    /// when no participant remains. Absent: a death resets nothing.
    #[serde(default)]
    replay_level_after_death_s: Option<f32>,
    /// A hit that deals at least this damage lands heavy. Absent: no hit
    /// lands heavy.
    #[serde(default)]
    heavy_hit_damage: Option<i32>,
    /// What each move's authored landing lag is multiplied by. Absent: the
    /// move's own lag.
    #[serde(default)]
    landing_lag_share: Option<f32>,
}

/// One game's rules for its own rooms, lowered from its pack.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoomRules {
    pub death: Option<DeathRules>,
    pub strike_weight: Option<StrikeWeightRules>,
    pub landing_lag: Option<LandingLagRules>,
}

impl AuthoredRoomRules {
    /// Each value the runtime cannot use, named by its field.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if let Some(seconds) = self.replay_level_after_death_s {
            if !(seconds.is_finite() && seconds >= 0.0) {
                problems.push(format!(
                    "`replay_level_after_death_s` is {seconds}; it is seconds, finite and not negative"
                ));
            }
        }
        if let Some(damage) = self.heavy_hit_damage {
            if damage < 1 {
                problems.push(format!(
                    "`heavy_hit_damage` is {damage}; a hit that deals no damage cannot land heavy, so it is at least 1"
                ));
            }
        }
        if let Some(share) = self.landing_lag_share {
            if !(share.is_finite() && share >= 0.0) {
                problems.push(format!(
                    "`landing_lag_share` is {share}; it multiplies each move's landing lag, so it is finite and not negative"
                ));
            }
        }
        problems
    }

    fn lowered(&self) -> RoomRules {
        RoomRules {
            death: self.replay_level_after_death_s.map(DeathRules::replay_level_after),
            strike_weight: self.heavy_hit_damage.map(StrikeWeightRules::heavy_at),
            landing_lag: self.landing_lag_share.map(LandingLagRules::scaled_by),
        }
    }
}

struct RoomRulesSchema;

impl ContentSchemaHandler for RoomRulesSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let authored = match ron::from_str::<AuthoredRoomRules>(facet.text) {
            Ok(authored) => authored,
            Err(error) => {
                let code = match error.code {
                    ron::error::Error::NoSuchStructField { .. } => DiagnosticCode::UnknownField,
                    _ => DiagnosticCode::MalformedSource,
                };
                out.report(facet.diagnostic(code, format!("{error}")));
                return;
            }
        };
        let id = facet.content_id_in(ROOM_RULES_SCHEMA, "rules");
        out.define(id.clone(), format!("{authored:?}"));
        for problem in authored.problems() {
            out.report(
                facet
                    .diagnostic(DiagnosticCode::MalformedSource, problem)
                    .about(id.clone()),
            );
        }
        if !out.failed() {
            out.lower(authored.lowered());
        }
    }
}

/// The validated rules, or `None` when the pack has no `room_rules` source.
pub fn lowered_room_rules(pack: &ambition_content_pack::PreparedContentPack) -> Option<&RoomRules> {
    pack.lowered::<RoomRules>(&SchemaId::new(ROOM_RULES_SCHEMA))
}

pub fn room_rules_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(ROOM_RULES_SCHEMA),
        version: ROOM_RULES_VERSION,
        capability: CapabilityId::new(ROOM_RULES_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "One game's rules for its own rooms: the death replay, the heavy-hit line \
              and the landing-lag share. An absent field declares nothing.",
        handler: Arc::new(RoomRulesSchema),
    }
}

/// Declare each rule that `rules` states, for the rooms of `scope`.
pub fn declare_room_rules(app: &mut App, scope: RulesScope, rules: RoomRules) {
    if let Some(death) = rules.death {
        app.declare_rules(scope, death);
    }
    if let Some(strike_weight) = rules.strike_weight {
        app.declare_rules(scope, strike_weight);
    }
    if let Some(landing_lag) = rules.landing_lag {
        app.declare_rules(scope, landing_lag);
    }
}

/// Replace this scope's declaration of each rule with what `rules` states. A
/// rule that `rules` does not state is removed from the scope: absent in the
/// candidate means remove, as for every family a reload publishes.
pub fn restate_room_rules(world: &mut World, scope: RulesScope, rules: Option<&RoomRules>) {
    let rules = rules.copied().unwrap_or_default();
    world
        .get_resource_or_insert_with(DeclaredRules::<DeathRules>::default)
        .restate(scope, rules.death);
    world
        .get_resource_or_insert_with(DeclaredRules::<StrikeWeightRules>::default)
        .restate(scope, rules.strike_weight);
    world
        .get_resource_or_insert_with(DeclaredRules::<LandingLagRules>::default)
        .restate(scope, rules.landing_lag);
}

#[cfg(test)]
mod tests;
