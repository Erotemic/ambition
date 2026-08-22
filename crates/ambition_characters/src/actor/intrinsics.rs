//! What a character's BODY is, as authored data — the facts that are true
//! of the creature however it is spawned and whoever drives it.
//!
//! ```text
//! CharacterLocomotion   how this body MOVES     run speed, style, surface cling
//! ContactDamage         does touching it hurt   strength, amount
//! ```
//!
//! neither is a controller fact. How FAST a body can go is a capability;
//! how fast it CHOOSES to go while patrolling is `BrainProfile`'s
//! `patrol_effort`, expressed as a fraction of this. The archetype vocabulary
//! already separates them exactly this way (`run_speed` vs `patrol_effort`), and
//! that separation is the thing worth carrying across rather than the row.

use crate::brain::MoveStyleSpec;

/// How this body moves under its own power.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterLocomotion {
    /// Ground-run capability, px/s — the fastest this body can locomote,
    /// and the only absolute speed a character authors.
    ///
    /// §4.7: locomotion crosses the brain→body seam as normalized effort. This
    /// is the body half of that sentence — a driver emits a throttle in `[0,1]`
    /// and the integrator resolves `locomotion * run_speed`, so a possessed
    /// crawler and a patrolling one share one top speed and differ only in how
    /// hard each asks for it.
    pub run_speed: f32,
    /// Locomotion STYLE — walk, crawl, hover, the shape of the gait.
    pub move_style: MoveStyleSpec,
    /// Walks surfaces hugging the surface normal: a wall/ceiling crawler
    /// with ledge-aware patrol, rather than a body that falls off ceilings.
    #[serde(default)]
    pub surface_walker: bool,
    /// Surface-walker only: a hit knocks this body off its surface — it loses
    /// cling and falls for a moment before re-attaching. `false` keeps a
    /// crawler holding on when struck.
    #[serde(default)]
    pub cling_breaks_on_hit: bool,
    /// Whether ordinary locomotion ignores gravity before any ability input.
    /// This is baseline body behavior, not the capability to fly. `None` means
    /// the character is silent; `Some(false)` explicitly authors a grounded
    /// baseline. Preparation resolves silence before spawning the body.
    #[serde(default)]
    pub baseline_free_flight: Option<bool>,
}

impl Default for CharacterLocomotion {
    fn default() -> Self {
        Self {
            // zero, not a guessed default, and that is deliberate: a
            // character that authors a locomotion block and forgets its speed
            // should stand still visibly rather than inherit somebody's idea of
            // a walk. There is no "ordinary" run speed for a body that could be
            // a mite or a giant.
            run_speed: 0.0,
            move_style: MoveStyleSpec::Walk,
            surface_walker: false,
            cling_breaks_on_hit: false,
            // SILENT, not grounded — see the field.
            baseline_free_flight: None,
        }
    }
}

/// What this body can be RIDDEN as, and what it can ride (ADR 0020).
///
/// A shark is rideable because of what a shark IS, and a pirate can board one for the same kind of
/// reason; neither is a decision the placement or the driver makes.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterMount {
    /// The mount CLASS this body offers when ridden. `None` = not a mount.
    ///
    /// Content-defined (`"shark"`, `"mech"`): a rider may board this body only
    /// if its own [`Self::pilotable_classes`] contains this string.
    #[serde(default)]
    pub class: Option<String>,
    /// The mount classes this body may PILOT. Empty = it rides nothing.
    #[serde(default)]
    pub pilotable_classes: Vec<String>,
    /// Damage this MOUNT splashes onto its rider when it dies. `None` = the
    /// rider drops unharmed, which is the ordinary dismount; `Some(n)` is a
    /// mech exploding under whoever was driving it.
    #[serde(default)]
    pub death_splash: Option<i32>,
}

/// Touching this body hurts.
///
/// Absent (`None` on a definition) means it does not, which is the ordinary
/// case: most characters harm only through their moves.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactDamage {
    /// Knockback strength of the contact — how hard being touched throws you.
    pub strength: f32,
    /// Damage dealt by the contact.
    pub amount: i32,
}

impl Default for ContactDamage {
    fn default() -> Self {
        Self {
            strength: 0.5,
            amount: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A locomotion block authors its speed or gets nothing, so a missing
    /// number is visible as a body that does not move rather than as one that
    /// moves like something else.
    #[test]
    fn an_unauthored_run_speed_is_zero_rather_than_a_guess() {
        assert_eq!(CharacterLocomotion::default().run_speed, 0.0);
        let crawler: CharacterLocomotion =
            ron::from_str("(run_speed: 36.0, move_style: Slither, surface_walker: true)")
                .expect("the authored form parses");
        assert_eq!(crawler.run_speed, 36.0);
        assert!(crawler.surface_walker);
        assert!(!crawler.cling_breaks_on_hit, "unauthored, so it holds on");
    }

    /// A misspelled knob is a refusal, the same contract `ArchetypeSpec` and
    /// `BrainProfile` carry: without it, `surfacewalker: true` compiles clean
    /// and the crawler falls off the ceiling with nothing to read.
    #[test]
    fn an_unknown_knob_is_rejected() {
        let parsed: Result<CharacterLocomotion, _> =
            ron::from_str("(run_speed: 36.0, move_style: Walk, surfacewalker: true)");
        assert!(parsed.is_err());
        let parsed: Result<ContactDamage, _> = ron::from_str("(strength: 0.5, amount: 1, x: 2)");
        assert!(parsed.is_err());
    }
}
