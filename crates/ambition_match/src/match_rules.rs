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
use ambition_characters::smash_fighter::FighterBodyAuthoring;
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
    /// The game's default fighter body, as a patch over the engine's
    /// (`DEFAULT_TUNING`). See [`MatchRules::base_body`].
    pub base_body: Option<FighterBodyAuthoring>,
    pub fighter_contact: Option<AuthoredContact>,
    /// How tightly the camera frames the fighters. `None` keeps the engine's
    /// exploration framing.
    pub cast_framing: Option<AuthoredCastFraming>,
}

/// See `CastFramingTuning`: the view a cast standing together may shrink the
/// stage view to (a fraction), and the room around the cast's box (px).
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredCastFraming {
    pub min_view_scale: f32,
    pub margin: f32,
    /// The factor on `margin` for 1, 2, 3 and 4+ members (a tuple of four).
    /// Absent: 1 for every count.
    #[serde(default)]
    pub margin_by_count: Option<(f32, f32, f32, f32)>,
    /// The share of the view kept below the cast. Absent: none.
    #[serde(default)]
    pub floor_share: f32,
    /// The pan rate (Hz). Absent: the engine's.
    #[serde(default)]
    pub pan_hz: Option<f32>,
    /// The zoom-out rate (Hz). Absent: the view opens at once.
    #[serde(default)]
    pub zoom_out_hz: Option<f32>,
    /// The zoom-in rate (Hz). Absent: the engine's.
    #[serde(default)]
    pub zoom_in_hz: Option<f32>,
    /// How far past the room the view may look. Absent: it stops at the room.
    #[serde(default)]
    pub bounds: Option<AuthoredCameraBounds>,
    /// The bubble a member wholly off the view is shown in. Absent: none.
    #[serde(default)]
    pub bubble: Option<AuthoredCastBubble>,
}

/// [`CastBubble`], as a pack writes it:
/// `(diameter_px: 96.0, span: 150.0, rings: ((1.0, 0.3, 0.3, 1.0), ...four...), backdrop: (0.05, 0.06, 0.12, 0.85))`.
///
/// [`CastBubble`]: ambition_platformer2d_shared_tangle::markers::CastBubble
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredCastBubble {
    pub diameter_px: f32,
    pub span: f32,
    /// One ring colour per seat, four seats.
    pub rings: [(f32, f32, f32, f32); 4],
    pub backdrop: (f32, f32, f32, f32),
}

/// [`CastCameraBounds`], as a pack writes it: `(side: 280.0, rise: 170.0, fall: 100.0)`.
///
/// [`CastCameraBounds`]: ambition_platformer2d_shared_tangle::markers::CastCameraBounds
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredCameraBounds {
    pub side: f32,
    pub rise: f32,
    pub fall: f32,
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
            // `MatchRules` holds the resolved body, not the patch it came from.
            base_body: None,
            fighter_contact: rules.fighter_contact.map(|contact| AuthoredContact { resistance: contact.resistance }),
            cast_framing: rules.cast_framing.map(|framing| AuthoredCastFraming {
                min_view_scale: framing.min_view_scale,
                margin: framing.margin,
                margin_by_count: Some({
                    let [a, b, c, d] = framing.margin_by_count;
                    (a, b, c, d)
                }),
                floor_share: framing.floor_share,
                pan_hz: Some(framing.pan_hz),
                zoom_out_hz: framing.zoom_out_hz,
                zoom_in_hz: Some(framing.zoom_in_hz),
                bounds: framing.bounds.map(|bounds| AuthoredCameraBounds {
                    side: bounds.side,
                    rise: bounds.rise,
                    fall: bounds.fall,
                }),
                bubble: framing.bubble.map(|bubble| AuthoredCastBubble {
                    diameter_px: bubble.diameter_px,
                    span: bubble.span,
                    rings: bubble.rings.map(|[r, g, b, a]| (r, g, b, a)),
                    backdrop: {
                        let [r, g, b, a] = bubble.backdrop;
                        (r, g, b, a)
                    },
                }),
            }),
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
        rules.base_body = self
            .base_body
            .map(|body| body.over(ambition_platformer2d_core::DEFAULT_TUNING));
        rules.fighter_contact = self.fighter_contact.map(|contact| BodyContact { resistance: contact.resistance });
        rules.cast_framing = self.cast_framing.map(|framing| {
            use ambition_platformer2d_shared_tangle::markers::{CastBubble, CastCameraBounds, CastFramingTuning};
            let engine = CastFramingTuning::default();
            CastFramingTuning {
                min_view_scale: framing.min_view_scale,
                margin: framing.margin,
                margin_by_count: framing
                    .margin_by_count
                    .map_or(engine.margin_by_count, |(a, b, c, d)| [a, b, c, d]),
                floor_share: framing.floor_share,
                pan_hz: framing.pan_hz.unwrap_or(engine.pan_hz),
                zoom_out_hz: framing.zoom_out_hz,
                zoom_in_hz: framing.zoom_in_hz.unwrap_or(engine.zoom_in_hz),
                bounds: framing.bounds.map(|bounds| CastCameraBounds {
                    side: bounds.side,
                    rise: bounds.rise,
                    fall: bounds.fall,
                }),
                bubble: framing.bubble.map(|bubble| CastBubble {
                    diameter_px: bubble.diameter_px,
                    span: bubble.span,
                    rings: bubble.rings.map(|(r, g, b, a)| [r, g, b, a]),
                    backdrop: {
                        let (r, g, b, a) = bubble.backdrop;
                        [r, g, b, a]
                    },
                }),
            }
        });
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
                ("directional_air_dodge_time", body.directional_air_dodge_time),
                ("directional_air_dodge_endlag", body.directional_air_dodge_endlag),
                ("air_dodge_landing_lag", body.air_dodge_landing_lag),
                ("directional_air_dodge_landing_lag", body.directional_air_dodge_landing_lag),
                ("dodge_roll_endlag", body.dodge_roll_endlag),
                ("dodge_stale_step", body.dodge_stale_step),
                ("dodge_stale_floor", body.dodge_stale_floor),
                ("dodge_stale_recovery", body.dodge_stale_recovery),
                ("untechable_launch_speed", body.untechable_launch_speed),
                ("evade_cancel_tail", body.evade_cancel_tail),
                ("tumble_speed", body.tumble_speed),
                ("spot_dodge_time", body.spot_dodge_time),
                ("spot_dodge_endlag", body.spot_dodge_endlag),
                ("launch_decay", body.launch_decay),
                ("sdi_step", body.sdi_step),
                ("asdi_step", body.asdi_step),
                ("jab_lock_speed", body.jab_lock_speed),
                ("crouch_speed_frac", body.crouch_speed_frac),
                ("initial_dash_time", body.initial_dash_time),
                ("dash_deflection", body.dash_deflection),
                ("coyote_time", body.coyote_time),
                ("turnaround_time", body.turnaround_time),
                ("teeter_margin", body.teeter_margin),
            ] {
                amount(&format!("body.{field}"), value);
            }
        }
        if let Some(contact) = &self.fighter_contact {
            amount("fighter_contact.resistance", contact.resistance);
        }
        if let Some(share) = self.cast_framing.map(|framing| framing.floor_share) {
            if !(share.is_finite() && (0.0..0.9).contains(&share)) {
                problems.push(format!("`cast_framing.floor_share` is {share}; it is a share of the view, 0 to below 0.9"));
            }
        }
        if let Some(bubble) = self.cast_framing.and_then(|framing| framing.bubble) {
            for (field, value) in [("diameter_px", bubble.diameter_px), ("span", bubble.span)] {
                if !(value.is_finite() && value > 0.0) {
                    problems.push(format!("`cast_framing.bubble.{field}` is {value}; it is finite and above 0"));
                }
            }
        }
        if let Some(body) = &self.base_body {
            body.problems(&mut problems);
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
