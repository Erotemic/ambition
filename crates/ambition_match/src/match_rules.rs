//! The `match_rules` authored-content schema: the rules a game's matches play
//! under, as data in its pack.
//!
//! The stock count, the clocks, the percent pool, the fighter body and the
//! ability floor and ceiling of a match were Rust constants in each game, so
//! a feel call (a turnaround frame, a dodge's staling) was a relink and a
//! restart. In a pack source, the edit is played at the next start.
//!
//! The field names are [`MatchRules`]' own. A field that is absent is the
//! engine's answer ([`MatchRules::default`]). What the data cannot name stays
//! with the game: the damage table its moves deal, the resources a seat holds
//! and the item spawns.

use std::sync::Arc;

use ambition_content_pack::{
    CapabilityId, ContentSchemaHandler, DiagnosticCode, FacetOutcome, FacetSource,
    RuntimeDisposition, SchemaId, SchemaRegistration, SchemaVersion,
};
use ambition_platformer2d_core::{MatchAbilities, MatchBody};
use ambition_platformer2d_shared_tangle::body::BodyContact;
use serde::Deserialize;

use crate::MatchRules;

/// The capability that owns this schema.
pub const MATCH_RULES_CAPABILITY: &str = "match";

/// The authored FILE kind: the rules of one game's matches.
pub const MATCH_RULES_SCHEMA: &str = "match_rules";

/// The schema version this handler reads.
pub const MATCH_RULES_VERSION: SchemaVersion = SchemaVersion(1);

/// The rules of a game's matches, as its pack states them. Each field is the
/// [`MatchRules`] field of the same name.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AuthoredMatchRules {
    pub stocks: Option<u32>,
    pub stock_respawn_seconds: f32,
    pub health_pool: Option<i32>,
    pub opens_suspended: bool,
    pub opening_countdown_ticks: u32,
    pub time_limit_ticks: u32,
    pub abilities: Option<MatchAbilities>,
    pub body: Option<MatchBody>,
    pub fighter_contact: Option<AuthoredContact>,
}

/// [`BodyContact`], as a pack writes it: `(resistance: 0.85)`.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredContact {
    pub resistance: f32,
}

impl Default for AuthoredMatchRules {
    fn default() -> Self {
        let rules = MatchRules::default();
        Self {
            stocks: rules.stocks,
            stock_respawn_seconds: rules.stock_respawn_seconds,
            health_pool: rules.health_pool,
            opens_suspended: rules.opens_suspended,
            opening_countdown_ticks: rules.opening_countdown_ticks,
            time_limit_ticks: rules.time_limit_ticks,
            abilities: rules.abilities,
            body: rules.body,
            fighter_contact: rules.fighter_contact.map(|contact| AuthoredContact { resistance: contact.resistance }),
        }
    }
}

impl AuthoredMatchRules {
    /// Write each rule this states on `rules`. The fields it cannot state
    /// (the damage table, the resources, the item spawns) are left as they are.
    pub fn apply_to(&self, rules: &mut MatchRules) {
        rules.stocks = self.stocks;
        rules.stock_respawn_seconds = self.stock_respawn_seconds;
        rules.health_pool = self.health_pool;
        rules.opens_suspended = self.opens_suspended;
        rules.opening_countdown_ticks = self.opening_countdown_ticks;
        rules.time_limit_ticks = self.time_limit_ticks;
        rules.abilities = self.abilities;
        rules.body = self.body;
        rules.fighter_contact = self.fighter_contact.map(|contact| BodyContact { resistance: contact.resistance });
    }

    /// Each value the runtime cannot use, named by its field.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.stocks == Some(0) {
            problems.push("`stocks` is 0; a stock match gives each fighter at least one".to_string());
        }
        if let Some(pool) = self.health_pool.filter(|pool| *pool < 1) {
            problems.push(format!("`health_pool` is {pool}; it is the denominator of percent, at least 1"));
        }
        let mut amount = |field: &str, value: f32| {
            if !(value.is_finite() && value >= 0.0) {
                problems.push(format!("`{field}` is {value}; it is finite and not negative"));
            }
        };
        amount("stock_respawn_seconds", self.stock_respawn_seconds);
        if let Some(body) = &self.body {
            for (field, value) in [
                ("slash_recoil", body.slash_recoil),
                ("jump_squat_time", body.jump_squat_time),
                ("air_dodge_time", body.air_dodge_time),
                ("air_dodge_speed", body.air_dodge_speed),
                ("air_dodge_endlag", body.air_dodge_endlag),
                ("dodge_roll_endlag", body.dodge_roll_endlag),
                ("dodge_stale_step", body.dodge_stale_step),
                ("dodge_stale_floor", body.dodge_stale_floor),
                ("dodge_stale_recovery", body.dodge_stale_recovery),
                ("untechable_launch_speed", body.untechable_launch_speed),
                ("evade_cancel_tail", body.evade_cancel_tail),
                ("tumble_speed", body.tumble_speed),
                ("spot_dodge_time", body.spot_dodge_time),
                ("sdi_step", body.sdi_step),
                ("asdi_step", body.asdi_step),
                ("jab_lock_speed", body.jab_lock_speed),
                ("crouch_speed_frac", body.crouch_speed_frac),
                ("initial_dash_time", body.initial_dash_time),
                ("initial_dash_speed", body.initial_dash_speed),
                ("turnaround_time", body.turnaround_time),
                ("teeter_margin", body.teeter_margin),
            ] {
                amount(&format!("body.{field}"), value);
            }
        }
        if let Some(contact) = &self.fighter_contact {
            amount("fighter_contact.resistance", contact.resistance);
        }
        problems
    }
}

struct MatchRulesSchema;

impl ContentSchemaHandler for MatchRulesSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let authored = match facet.parse::<AuthoredMatchRules>() {
            Ok(authored) => authored,
            Err(diagnostic) => {
                out.report(diagnostic);
                return;
            }
        };
        let id = facet.content_id_in(MATCH_RULES_SCHEMA, "rules");
        out.define(id.clone(), format!("{authored:?}"));
        for problem in authored.problems() {
            out.report(
                facet
                    .diagnostic(DiagnosticCode::MalformedSource, problem)
                    .about(id.clone()),
            );
        }
        if !out.failed() {
            out.lower(authored);
        }
    }
}

/// The validated rules, or `None` when the pack has no `match_rules` source.
pub fn lowered_match_rules(pack: &ambition_content_pack::PreparedContentPack) -> Option<&AuthoredMatchRules> {
    pack.lowered::<AuthoredMatchRules>(&SchemaId::new(MATCH_RULES_SCHEMA))
}

pub fn match_rules_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(MATCH_RULES_SCHEMA),
        version: MATCH_RULES_VERSION,
        capability: CapabilityId::new(MATCH_RULES_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "The rules of one game's matches: stocks, clocks, the percent pool, the \
              fighter body, the ability floor and ceiling, fighter contact. An absent \
              field is the engine's answer.",
        handler: Arc::new(MatchRulesSchema),
    }
}

#[cfg(test)]
mod tests;
