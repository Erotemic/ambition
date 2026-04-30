//! Optional movement/combat capabilities.
//!
//! Ambition is expected to have many upgrades, and the endgame sandbox should
//! usually run with everything enabled. The engine still needs the opposite:
//! small, explicit capability sets that can be tested in isolation. This file
//! is the vocabulary for that.
//!
//! The important rule is that an ability flag should answer "may this verb be
//! used at all?" Tuning values such as speed, duration, and charge counts live
//! in `MovementTuning`, while this module decides which groups of verbs exist.

/// A set of optional player capabilities.
///
/// This is intentionally a plain data struct. Later we can load it from RON,
/// JSON, a save file, an AI-generated spec, or an in-game upgrade graph without
/// changing the movement simulation API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbilitySet {
    /// Horizontal ground/air steering. Disabling this is mostly useful for
    /// tests or scripted story moments.
    pub move_horizontal: bool,
    /// Basic jump from ground/coyote time.
    pub jump: bool,
    /// Early jump release clips upward velocity.
    pub variable_jump: bool,
    /// One extra air jump in the current tuning pass.
    pub double_jump: bool,
    /// Holding down while airborne increases fall acceleration/speed.
    pub fast_fall: bool,
    /// Jumping from a wall contact.
    pub wall_jump: bool,
    /// Slow or stop wall sliding while pressing into a wall.
    pub wall_cling: bool,
    /// Climb upward/downward while clinging to a wall.
    pub wall_climb: bool,
    /// Aerial/ground dash.
    pub dash: bool,
    /// Upgrade that gives two dash charges before refresh.
    pub double_dash: bool,
    /// Short-range teleport. Quick release blinks immediately along input/facing.
    pub blink: bool,
    /// Upgrade for blink: holding the blink button enters aim/bullet-time mode
    /// and releases to blink to a more deliberate destination.
    pub precision_blink: bool,
    /// Generic slash/attack verb.
    pub attack: bool,
    /// Downward attack/pogo refresh verb.
    pub pogo: bool,
    /// Direction + primary attack can eventually produce distinct attacks.
    /// The first implementation still shares the same hitbox helper.
    pub directional_primary: bool,
    /// Direction + special/secondary can eventually produce distinct specials.
    /// Blink is the first concrete special in this category.
    pub directional_special: bool,
    /// Allow special world surfaces to apply an impulse.
    pub rebound: bool,
    /// Debug/sandbox reset. In the final game this may become a menu/system
    /// action rather than a player ability.
    pub reset: bool,
}

impl AbilitySet {
    /// Minimal movement for a first-room player.
    pub const fn basic() -> Self {
        Self {
            move_horizontal: true,
            jump: true,
            variable_jump: true,
            double_jump: false,
            fast_fall: false,
            wall_jump: false,
            wall_cling: false,
            wall_climb: false,
            dash: false,
            double_dash: false,
            blink: false,
            precision_blink: false,
            attack: false,
            pogo: false,
            directional_primary: false,
            directional_special: false,
            rebound: false,
            reset: true,
        }
    }

    /// Endgame sandbox defaults: every currently implemented verb is enabled.
    pub const fn sandbox_all() -> Self {
        Self {
            move_horizontal: true,
            jump: true,
            variable_jump: true,
            double_jump: true,
            fast_fall: true,
            wall_jump: true,
            wall_cling: true,
            wall_climb: true,
            dash: true,
            double_dash: true,
            blink: true,
            precision_blink: true,
            attack: true,
            pogo: true,
            directional_primary: true,
            directional_special: true,
            rebound: true,
            reset: true,
        }
    }

    /// A deliberately sane initial endgame subset.
    ///
    /// This is a smaller list than "all platformer abilities ever", but it is
    /// broad enough to exercise movement, wall routing, combat, and one special
    /// teleport verb.  The sandbox currently uses `sandbox_all`; tests and later
    /// story states can use this as a balanced default.
    pub const fn sane_subset() -> Self {
        Self {
            move_horizontal: true,
            jump: true,
            variable_jump: true,
            double_jump: true,
            fast_fall: true,
            wall_jump: true,
            wall_cling: true,
            wall_climb: true,
            dash: true,
            double_dash: true,
            blink: true,
            precision_blink: true,
            attack: true,
            pogo: true,
            directional_primary: true,
            directional_special: true,
            rebound: true,
            reset: true,
        }
    }

    /// Number of air jumps granted by the active ability set.
    pub const fn air_jump_count(self, tuning_air_jumps: u8) -> u8 {
        if self.double_jump { tuning_air_jumps } else { 0 }
    }

    /// Number of dash charges granted by the active ability set.
    pub const fn dash_charge_count(self) -> u8 {
        if !self.dash {
            0
        } else if self.double_dash {
            2
        } else {
            1
        }
    }

    /// Human-readable compatibility warnings.
    ///
    /// These are warnings, not hard errors. Some story/gameplay moments may
    /// intentionally enable a dependent ability without its normal prerequisite.
    pub fn compatibility_warnings(self) -> Vec<&'static str> {
        let mut warnings = Vec::new();
        if self.double_jump && !self.jump {
            warnings.push("double_jump is enabled but jump is disabled");
        }
        if self.wall_jump && !self.jump {
            warnings.push("wall_jump is enabled but jump is disabled");
        }
        if self.wall_climb && !self.wall_cling {
            warnings.push("wall_climb is enabled but wall_cling is disabled");
        }
        if self.double_dash && !self.dash {
            warnings.push("double_dash is enabled but dash is disabled");
        }
        if self.precision_blink && !self.blink {
            warnings.push("precision_blink is enabled but blink is disabled");
        }
        if self.directional_special && !self.blink {
            warnings.push("directional_special currently has no concrete verb unless blink is enabled");
        }
        if self.pogo && !self.attack {
            warnings.push("pogo is enabled but attack is disabled");
        }
        warnings
    }
}

impl Default for AbilitySet {
    fn default() -> Self {
        Self::basic()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_all_has_no_compatibility_warnings() {
        assert!(AbilitySet::sandbox_all().compatibility_warnings().is_empty());
    }

    #[test]
    fn dependent_abilities_report_warnings() {
        let mut abilities = AbilitySet::basic();
        abilities.double_dash = true;
        abilities.wall_climb = true;
        abilities.precision_blink = true;
        let warnings = abilities.compatibility_warnings();
        assert!(warnings.iter().any(|w| w.contains("double_dash")));
        assert!(warnings.iter().any(|w| w.contains("wall_climb")));
        assert!(warnings.iter().any(|w| w.contains("precision_blink")));
    }
}
