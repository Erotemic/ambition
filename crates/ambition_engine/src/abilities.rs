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

use serde::{Deserialize, Serialize};

/// A set of optional player capabilities.
///
/// This is intentionally a plain data struct. Later we can load it from RON,
/// JSON, a save file, an AI-generated spec, or an in-game upgrade graph without
/// changing the movement simulation API.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Double-tap down while airborne starts fast-fall. Holding down alone does
    /// not fast-fall, so down+attack/pogo can remain a natural input.
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
    /// Free-flight sandbox/test movement mode. When toggled on, movement input
    /// applies acceleration toward a terminal velocity instead of normal
    /// ground/air platformer steering.
    pub fly: bool,
    /// Short-range teleport. Quick release blinks immediately along input/facing.
    pub blink: bool,
    /// Upgrade for blink: holding the blink button enters aim/bullet-time mode
    /// and releases to blink to a more deliberate destination.
    pub precision_blink: bool,
    /// Allow blink pathing through soft blink gates. The destination must still
    /// be open space; this only permits crossing selected wall volumes.
    pub blink_through_soft_walls: bool,
    /// Allow blink pathing through hard blink gates. This is intentionally a
    /// separate future upgrade so some walls can remain meaningful blockers.
    pub blink_through_hard_walls: bool,
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
    /// Snap onto ledges while wall-sliding and pull-up to the platform
    /// above. Gated as a separate ability so the early game can ship
    /// without it and a mid-game upgrade or piece of gear can light it
    /// up. Movement integration reads `Player::abilities.ledge_grab`
    /// before running the snap probe, so disabling this turns the
    /// mechanic off entirely.
    #[serde(default)]
    pub ledge_grab: bool,
    /// Active swim controls inside any `WaterRegion`: jump becomes a
    /// swim impulse, the player can rise with repeated presses, and
    /// surface exit is allowed. Without this flag the player drowns
    /// on water contact (movement triggers a respawn). Source of the
    /// region — IntGrid `Water` cells or entity `WaterVolume` — is
    /// abstracted by `World::water_at`.
    #[serde(default)]
    pub swim: bool,
    /// Glide / cape / slow-fall: holding the jump button while
    /// airborne and falling caps the fall speed at
    /// `MovementTuning::glide_fall_speed` instead of `max_fall_speed`.
    /// Cancels on ground / dash / blink / jump release. Cheap held
    /// ability that pairs well with `wall_jump` and `double_jump` for
    /// long-distance platforming. No resource cost in the v1 — that
    /// can land later as a `hover_fuel` `ResourceMeter` tap.
    #[serde(default)]
    pub glide: bool,
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
            fly: false,
            blink: false,
            precision_blink: false,
            blink_through_soft_walls: false,
            blink_through_hard_walls: false,
            attack: false,
            pogo: false,
            directional_primary: false,
            directional_special: false,
            rebound: false,
            reset: true,
            ledge_grab: false,
            swim: false,
            glide: false,
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
            fly: true,
            blink: true,
            precision_blink: true,
            blink_through_soft_walls: true,
            blink_through_hard_walls: true,
            attack: true,
            pogo: true,
            directional_primary: true,
            directional_special: true,
            rebound: true,
            reset: true,
            ledge_grab: true,
            swim: true,
            glide: true,
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
            fly: true,
            blink: true,
            precision_blink: true,
            blink_through_soft_walls: true,
            blink_through_hard_walls: false,
            attack: true,
            pogo: true,
            directional_primary: true,
            directional_special: true,
            rebound: true,
            reset: true,
            // ledge grab + swim + glide are mid-game upgrades; not
            // part of the "sane subset" early-game baseline.
            ledge_grab: false,
            swim: false,
            glide: false,
        }
    }

    /// Number of air jumps granted by the active ability set.
    pub const fn air_jump_count(self, tuning_air_jumps: u8) -> u8 {
        if self.double_jump {
            tuning_air_jumps
        } else {
            0
        }
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
        if self.fly && !self.move_horizontal {
            warnings.push("fly is enabled but move_horizontal is disabled");
        }
        if self.precision_blink && !self.blink {
            warnings.push("precision_blink is enabled but blink is disabled");
        }
        if self.blink_through_soft_walls && !self.blink {
            warnings.push("blink_through_soft_walls is enabled but blink is disabled");
        }
        if self.blink_through_hard_walls && !self.blink_through_soft_walls {
            warnings.push("blink_through_hard_walls is enabled without blink_through_soft_walls");
        }
        if self.directional_special && !self.blink {
            warnings
                .push("directional_special currently has no concrete verb unless blink is enabled");
        }
        if self.pogo && !self.attack {
            warnings.push("pogo is enabled but attack is disabled");
        }
        if self.glide && !self.jump {
            warnings.push("glide is enabled but jump is disabled (the trigger is hold-jump)");
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
        assert!(AbilitySet::sandbox_all()
            .compatibility_warnings()
            .is_empty());
    }

    #[test]
    fn dependent_abilities_report_warnings() {
        let mut abilities = AbilitySet::basic();
        abilities.double_dash = true;
        abilities.wall_climb = true;
        abilities.precision_blink = true;
        abilities.blink_through_soft_walls = true;
        let warnings = abilities.compatibility_warnings();
        assert!(warnings.iter().any(|w| w.contains("double_dash")));
        assert!(warnings.iter().any(|w| w.contains("wall_climb")));
        assert!(warnings.iter().any(|w| w.contains("precision_blink")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("blink_through_soft_walls")));
    }

    #[test]
    fn glide_without_jump_warns() {
        let mut abilities = AbilitySet::basic();
        abilities.glide = true;
        abilities.jump = false;
        let warnings = abilities.compatibility_warnings();
        assert!(warnings.iter().any(|w| w.contains("glide")));
    }

    #[test]
    fn dash_charge_count_respects_double_dash() {
        let mut abilities = AbilitySet::basic();
        abilities.dash = true;
        assert_eq!(abilities.dash_charge_count(), 1);
        abilities.double_dash = true;
        assert_eq!(abilities.dash_charge_count(), 2);
        abilities.dash = false;
        assert_eq!(abilities.dash_charge_count(), 0);
    }

    #[test]
    fn air_jump_count_zero_without_double_jump() {
        let mut abilities = AbilitySet::basic();
        assert_eq!(abilities.air_jump_count(2), 0);
        abilities.double_jump = true;
        assert_eq!(abilities.air_jump_count(2), 2);
    }

    #[test]
    fn sane_subset_passes_compatibility() {
        // Same contract as sandbox_all: no warnings on a curated set.
        assert!(AbilitySet::sane_subset()
            .compatibility_warnings()
            .is_empty());
    }
}
