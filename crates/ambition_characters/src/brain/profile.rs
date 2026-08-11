//! **A reusable autonomous-controller profile** — how a non-human participant
//! CHOOSES, separated from what the body IS.
//!
//! ```text
//! CharacterDefinition   what this character is and can do   (body)
//! BrainProfile          how an autonomous driver decides    (this file)
//! SpawnContext          what is true about this instance    (placement)
//! ```
//!
//! ⭐ **this is the second of Jon's three authorities**
//! (`docs/planning/overnight-campaign-2026-08-11.md`), and the one the enemy
//! archetype hid best. `ArchetypeSpec` states a body's health, mass, hurtboxes
//! and repertoire in the same row as its aggro radius and its Smash tactics, so
//! two characters that fight alike could not share a policy without also sharing
//! a body — which is why `medium_striker` exists as a whole-body archetype worn
//! by several different creatures.
//!
//! ⛔ **a controller supplies INTENT and never manufactures a capability.** The
//! `smash_can_*` trio below is the one place that rule is currently broken, and
//! it is broken deliberately and visibly: those three are a MIRROR of the body's
//! `can_blink` / `can_fly` / `can_shield`, projected so the Smash brain knows
//! which options are worth attempting. Their deletion is item 21 of the
//! campaign — the brain should ask the body's `CombatCapabilities` instead of
//! carrying a copy that can disagree with it.
//!
//! ## Why the numbers here are DISTANCES and not SPEEDS
//!
//! A profile authors *"notice at 220px, commit at 36px"* and never *"chase at
//! 110px/s"*. Absolute locomotion is the BODY's: §4.7 has the brain→body seam
//! carrying normalized effort, and the archetype vocabulary already respects it
//! (`run_speed` on the body, `patrol_effort`/`chase_effort` as fractions of it).
//! The catalog's older `brain_presets` rows do author absolute speeds
//! (`MeleeBrute(chase_speed: 110.0, ..)`), which is the same fork seen from the
//! other side and is the thing to fix when the two vocabularies merge — not a
//! precedent to copy.

use super::CharacterBrainTemplate;

/// Default melee smash hit-band (px) for a profile that authors none.
pub const DEFAULT_SMASH_HIT_BAND: f32 = 36.0;

fn default_smash_hit_band() -> f32 {
    DEFAULT_SMASH_HIT_BAND
}

fn default_turns_at_walls() -> bool {
    true
}

/// The middle rung: a profile that names the fighter template and nothing else
/// gets a fighter rather than a refusal.
fn default_fighter_level() -> u8 {
    5
}

/// An unhurried patrol: half the body's top speed. The number the runtime
/// hard-coded before a profile could state one.
fn default_patrol_effort() -> f32 {
    0.5
}

/// A chase is everything the body has, which is what makes noticing you matter.
fn default_chase_effort() -> f32 {
    1.0
}

/// **How an autonomous participant driving this body decides what to do.**
///
/// Reusable across characters by design: several distinct bodies may name the
/// same profile, and the same body may be driven by a different one in a
/// different context. Nothing here says what the body can physically do.
///
/// ⛔⛔ **AND NOTHING HERE SAYS WHO THIS BODY'S ENEMIES ARE.** This answers *how
/// aggressively do I close, how far away do I notice, which attack do I prefer,
/// how do I patrol* — never *is that a target*. Hostility is a relationship, and
/// a relationship belongs to the PLACEMENT and the session: `SpawnDisposition`,
/// factions, teams. The driver is handed eligible targets and decides what to do
/// about them.
///
/// ⛔ **`attacks_player` used to live here and was deleted 2026-08-11** (Jon's
/// redirect §6). It came across from `ArchetypeSpec`, where body and AI and
/// social role were one thing, and it was wrong twice over: it is not a policy,
/// and it is player-centric vocabulary in the one type that must never be. The
/// giant GNU was its motivating case and it is the proof it was unnecessary —
/// the mount is a `StandStill` driver with a zero aggro radius, so its POLICY
/// already said it never seeks anybody, and its placement now says `Peaceful`
/// because that is where "this creature is not your enemy" belongs.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct BrainProfile {
    /// Which motion / decision policy the brain instantiates.
    pub template: CharacterBrainTemplate,
    /// Distance (px) at which this driver notices a target.
    #[serde(default)]
    pub aggro_radius: f32,
    /// Distance (px) at which this driver commits to an attack.
    #[serde(default)]
    pub attack_range: f32,
    /// **Simple walkers turn away from a semantic side contact.**
    ///
    /// Control policy, consumed by the Patrol/Wanderer templates through
    /// `BrainSnapshot` — the movement kernel only publishes the wall-contact
    /// fact and never changes facing. It lives here rather than on the body so
    /// that human control, a Smash fighter brain, scripted control and a future
    /// remote/RL controller do not inherit a hidden locomotion policy merely by
    /// inhabiting the same body.
    #[serde(default = "default_turns_at_walls")]
    pub turns_at_walls: bool,
    /// **How hard this driver walks while patrolling**, as a fraction of the
    /// body's own top speed — §4.7's normalized effort, and the reason a
    /// profile never authors px/s.
    ///
    /// ⛔ **the runtime hard-coded `0.5` and `1.0`** in `new_character_in`, so a
    /// character-first body could not be an ambler or a sprinter: every
    /// migrated creature patrolled at exactly half pace whatever its archetype
    /// row had said. `pirate_shark_rider` authors 0.4783 and `medium_striker`
    /// 0.44 — numbers that were tuned, and that a migration would have silently
    /// rounded to one shared value.
    #[serde(default = "default_patrol_effort")]
    pub patrol_effort: f32,
    /// The same, for a committed chase.
    #[serde(default = "default_chase_effort")]
    pub chase_effort: f32,
    /// Which rung of the fighter ladder a [`CharacterBrainTemplate::Fighter`]
    /// driver plays at — difficulty, which is a controller fact. Ignored by
    /// every other template.
    #[serde(default = "default_fighter_level")]
    pub fighter_level: u8,
    /// Smash-template hit band (px) — the radius the driver closes to before
    /// emitting a melee attack.
    #[serde(default = "default_smash_hit_band")]
    pub smash_hit_band: f32,
    /// Smash-template heavy base: longer reach + slower chase
    /// (`SmashCfg::BRUTE_DEFAULT`) vs the lighter striker default.
    #[serde(default)]
    pub smash_heavy: bool,
    /// Smash-template dash-to-close: a richer option set that dashes to close a
    /// large gap.
    #[serde(default)]
    pub smash_dash_to_close: bool,
    /// Smash-template **duelist neutral game** (`SmashCfg::DUELIST_DEFAULT`
    /// base): footsies, neutral hops and a real spacing/retreat rhythm instead
    /// of the grunt's close-and-camp.
    #[serde(default)]
    pub smash_duelist: bool,
    /// ⛔ **mirror of the body's `can_blink`** — the driver's *attempt* side of a
    /// capability the body *enforces*. Campaign item 21 deletes it.
    #[serde(default)]
    pub smash_can_blink: bool,
    /// ⛔ mirror of the body's `can_fly`. See [`Self::smash_can_blink`].
    #[serde(default)]
    pub smash_can_fly: bool,
    /// ⛔ mirror of the body's `can_shield`. See [`Self::smash_can_blink`].
    #[serde(default)]
    pub smash_can_shield: bool,
    /// When provoked from peaceful, force an aggressive MeleeBrute policy with
    /// at least this aggro radius. `None` = use the template's default
    /// aggressive brain.
    #[serde(default)]
    pub provoke_forced_brute_min_aggro: Option<f32>,
}

impl Default for BrainProfile {
    fn default() -> Self {
        Self {
            template: CharacterBrainTemplate::MeleeBrute,
            aggro_radius: 0.0,
            attack_range: 0.0,
            turns_at_walls: true,
            patrol_effort: default_patrol_effort(),
            chase_effort: default_chase_effort(),
            fighter_level: default_fighter_level(),
            smash_hit_band: DEFAULT_SMASH_HIT_BAND,
            smash_heavy: false,
            smash_dash_to_close: false,
            smash_duelist: false,
            smash_can_blink: false,
            smash_can_fly: false,
            smash_can_shield: false,
            provoke_forced_brute_min_aggro: None,
        }
    }
}

impl BrainProfile {
    /// Default melee smash hit-band (px) when a profile authors none — the same
    /// constant as [`DEFAULT_SMASH_HIT_BAND`], reachable through the type for
    /// call sites that already hold it.
    pub const DEFAULT_SMASH_HIT_BAND: f32 = DEFAULT_SMASH_HIT_BAND;

    /// A profile that only names its template — every policy knob at its
    /// default. The shape a peaceful or scripted body wants.
    pub fn from_template(template: CharacterBrainTemplate) -> Self {
        Self {
            template,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A profile authored with only a template is complete**, and its
    /// defaults are the ones the runtime used to hard-code.
    ///
    /// ⚠ the poison this guards: `#[serde(default)]` on `turns_at_walls` or
    /// `fighter_level` would silently author `false` / `0` — a walker that
    /// stops turning at walls, and a fighter refused for being off the ladder.
    /// Both were live defaults elsewhere before this type existed.
    #[test]
    fn a_template_only_profile_keeps_the_runtime_defaults() {
        let profile: BrainProfile = ron::from_str("(template: Wanderer)")
            .expect("a profile may author nothing but its template");
        assert_eq!(profile.template, CharacterBrainTemplate::Wanderer);
        assert!(
            profile.turns_at_walls,
            "a walker that authors nothing still turns at walls"
        );
        assert_eq!(profile.fighter_level, 5, "the middle rung, not rung zero");
        assert_eq!(profile.smash_hit_band, DEFAULT_SMASH_HIT_BAND);
        assert_eq!(profile.patrol_effort, 0.5, "the runtime's old hard-code");
        assert_eq!(profile.chase_effort, 1.0);
    }

    /// **An authored value beats the default**, so the test above is not
    /// measuring a struct that ignores its input.
    #[test]
    fn authored_policy_wins_over_the_default() {
        let profile: BrainProfile = ron::from_str(
            "(template: Smash, aggro_radius: 220.0, attack_range: 36.0, \
             turns_at_walls: false, fighter_level: 9, smash_duelist: true, \
             patrol_effort: 0.4783, chase_effort: 0.8)",
        )
        .expect("the authored form parses");
        assert_eq!(profile.aggro_radius, 220.0);
        assert_eq!(profile.attack_range, 36.0);
        assert!(!profile.turns_at_walls);
        assert_eq!(profile.fighter_level, 9);
        assert!(profile.smash_duelist);
        assert_eq!(profile.patrol_effort, 0.4783, "a tuned amble survives");
        assert_eq!(profile.chase_effort, 0.8);
    }

    /// **A misspelled knob is a REFUSAL, not a silent no-op.**
    ///
    /// The same contract `ArchetypeSpec` learned the hard way: without
    /// `deny_unknown_fields` an authored `agro_radius` compiles clean and the
    /// mechanic never fires, which looks identical to authoring nothing.
    #[test]
    fn an_unknown_knob_is_rejected() {
        let parsed: Result<BrainProfile, _> =
            ron::from_str("(template: Wanderer, agro_radius: 220.0)");
        assert!(
            parsed.is_err(),
            "a misspelled policy knob must fail to parse, not vanish"
        );
    }
}
