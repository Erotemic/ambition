//! Reusable autonomous-controller policy.
//!
//! A [`BrainProfile`] describes how an autonomous controller chooses; body
//! capabilities remain authoritative on the body. A profile may express normalized
//! effort and spatial thresholds, but it must not manufacture abilities or duplicate
//! body limits such as absolute movement speed.
//!
//! The same profile can therefore drive different bodies while respecting each
//! body's live capabilities and movement parameters.

use super::CharacterBrainTemplate;

/// A developer's standing override of what every AUTHORED actor's brain is.
///
/// ⭐⭐ THE INVERSION D33 ASKED FOR: *the sim reads a SESSION-OWNED override that
/// the dev tool WRITES, never the dev crate itself.* Until 2026-09-02 the
/// simulation called `ambition_dev_tools::brain_override::forced_profile()` and
/// `forced_preset()` while BUILDING A LIVE BRAIN — the actor kernel reaching UP
/// into a developer crate to decide what the world contains, which is the
/// authority that carve exists to remove. `ClockScaleRequest` had already shown
/// the shape for slow-motion; this is the same move for brains.
///
/// ⛔ ABSENT AND DEFAULT BOTH MEAN "THE AUTHOR DECIDES", which is exactly what an
/// unset environment variable meant before. A composition with no developer
/// tools installs nothing and every placement resolves its own brain.
///
/// ⚠ AND IT STOPS BEING A HIDDEN PROCESS INPUT. The old road was a `OnceLock`
/// over `std::env::var`, resolved once per PROCESS: a fixture could not set it,
/// two tests in one binary could not disagree about it, and the value steering a
/// run appeared in no snapshot. A resource is ordinary state — a test sets it,
/// and what a measurement was taken under is visible to the census that prints
/// it.
///
/// ⛔ IT CHANGES THE ROOM, AND THAT IS THE POINT — see
/// `ambition_dev_tools::brain_override`, which still owns the environment names
/// and the reason the knob exists. This type owns only the VALUE.
#[derive(bevy::prelude::Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct AuthoredBrainOverride {
    /// Force every authored actor's brain PRESET. `None` = the placement decides.
    pub preset: Option<String>,
    /// Force every authored actor's autonomous PROFILE. `None` = the author decides.
    ///
    /// ⭐ A SECOND FIELD AND NOT A MODE OF THE FIRST, because the preset road
    /// cannot reach the brains that perceive: every catalog preset lowers to an
    /// arm `tick_simple_state_machine` answers, and that takes no `WorldView`.
    /// `Fighter` is reachable only through a character's autonomous profile.
    pub profile: Option<String>,
}

impl AuthoredBrainOverride {
    /// The preset every authored actor is forced to, or `None`.
    pub fn preset(&self) -> Option<&str> {
        self.preset.as_deref()
    }

    /// The autonomous profile every authored actor is forced to, or `None`.
    pub fn profile(&self) -> Option<&str> {
        self.profile.as_deref()
    }
}

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

/// How an autonomous participant driving this body decides what to do.
///
/// Reusable across characters by design: several distinct bodies may name the
/// same profile, and the same body may be driven by a different one in a
/// different context. Nothing here says what the body can physically do.
///
///  AND NOTHING HERE SAYS WHO THIS BODY'S ENEMIES ARE. This answers *how
/// aggressively do I close, how far away do I notice, which attack do I prefer,
/// how do I patrol* — never *is that a target*. Hostility is a relationship, and
/// a relationship belongs to the PLACEMENT and the session: `SpawnDisposition`,
/// factions, teams. The driver is handed eligible targets and decides what to do
/// about them.
///
/// The giant GNU was its motivating case and it is the proof it was unnecessary — the mount is
/// a `StandStill` driver with a zero aggro radius, so its POLICY already said it never seeks
/// anybody, and its placement now says `Peaceful` because that is where "this creature is not
/// your enemy" belongs.
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
    /// Simple walkers turn away from a semantic side contact.
    ///
    /// Control policy, consumed by the Patrol/Wanderer templates through
    /// `BrainSnapshot` — the movement kernel only publishes the wall-contact
    /// fact and never changes facing. It lives here rather than on the body so
    /// that human control, a Smash fighter brain, scripted control and a future
    /// remote/RL controller do not inherit a hidden locomotion policy merely by
    /// inhabiting the same body.
    #[serde(default = "default_turns_at_walls")]
    pub turns_at_walls: bool,
    /// Patrol effort as a fraction of the body's own top speed. Profiles
    /// author normalized effort rather than world-space speed.
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
    pub smash_sprint_to_close: bool,
    /// Smash-template duelist neutral game (`SmashCfg::DUELIST_DEFAULT`
    /// base): footsies, neutral hops and a real spacing/retreat rhythm instead
    /// of the grunt's close-and-camp.
    #[serde(default)]
    pub smash_duelist: bool,
    /// When provoked from peaceful, force an aggressive MeleeBrute policy with
    /// at least this aggro radius. `None` = use the template's default
    /// aggressive brain.
    #[serde(default)]
    pub provoke_forced_brute_min_aggro: Option<f32>,
    /// How often this driver commits to a swing — `ENEMY_ATTACK_COOLDOWN *
    /// attack_cooldown_mult` paces the brain's next attack.
    ///
    /// It reads as a body number and is not one, by exactly the argument already made for
    /// `aggro_radius` and `attack_range`: a radius at which a driver notices, a range at which it
    /// commits, and a rate at which it commits again are all decisions about how to PLAY a body,
    /// and a human or scripted controller in the same body must not inherit them.
    #[serde(default = "default_attack_cooldown_mult")]
    pub attack_cooldown_mult: f32,
}

/// Serde/`Default` value for [`BrainProfile::attack_cooldown_mult`]: unscaled.
pub fn default_attack_cooldown_mult() -> f32 {
    1.0
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
            smash_sprint_to_close: false,
            smash_duelist: false,
            provoke_forced_brute_min_aggro: None,
            attack_cooldown_mult: default_attack_cooldown_mult(),
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

    /// A template-only profile is complete and receives the runtime defaults.
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

    /// An authored value beats the default, so the test above is not
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

    /// A misspelled knob is a REFUSAL, not a silent no-op.
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
