//! Swallows fetch coconuts from the primeval trail and release them over prey.
//!
//! The rule is a script the game ships (`rhai/swallows.rhai`, runtime
//! authoring audit P4): one invocation for each room's swallows
//! (`group_think`), which steer through the control gate and take and let go
//! through the item owner. The swallows carry the catalog tag `swallow`.

use bevy::prelude::*;

pub const COCONUT: &str = "coconut";

/// The swallows' rule, as the game ships it.
pub const SWALLOWS_SCRIPT: &str = include_str!("../rhai/swallows.rhai");

pub struct PrimevalPlugin;

impl Plugin for PrimevalPlugin {
    fn build(&self, app: &mut App) {
        ambition_platformer2d::extension::add_shipped_script(
            app,
            "game/ambition_content/rhai/swallows.rhai",
            SWALLOWS_SCRIPT,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The value of the script's constant `name` (`const NAME = 1.5;`).
    fn constant(name: &str) -> f32 {
        let prefix = format!("const {name} = ");
        let line = SWALLOWS_SCRIPT
            .lines()
            .find_map(|line| line.strip_prefix(&prefix))
            .unwrap_or_else(|| panic!("the script states {name}"));
        line.trim_end_matches(';').parse().unwrap_or_else(|_| panic!("{name} is a number: {line}"))
    }

    /// The bird lets go no lower over its target than it carries the
    /// coconut, or it would never let go; and the fall from there is long
    /// enough to hurt: the thrown-item rule's bar for a coconut is a speed,
    /// and a fall of `LET_GO_OVER` at the gravity a loose item falls with is
    /// faster.
    #[test]
    fn a_coconut_let_go_arrives_faster_than_it_hurts_at() {
        let (over, carried) = (constant("LET_GO_OVER"), constant("CARRY_OVER"));
        assert!(over <= carried, "it lets go no lower than {over} and carries at {carried}: it never lets go");
        let bar = crate::tuning::shipped()
            .thrown_impacts
            .get(COCONUT)
            .expect("a coconut hurts what it strikes")
            .min_speed;
        let arrives = (2.0 * ambition_held_items::GROUND_ITEM_GRAVITY * over).sqrt();
        assert!(arrives > bar, "a coconut that falls {over} arrives at {arrives}, under its bar of {bar}");
    }
}
