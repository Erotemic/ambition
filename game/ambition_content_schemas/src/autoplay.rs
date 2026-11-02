//! Authored routes for Ambition's development input driver.

use std::collections::BTreeMap;

use ambition_content_pack::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};
use serde::Deserialize;

pub const AUTOPLAY_SCHEMA: &str = "ambition_autoplay";
pub type Profiles = BTreeMap<String, Profile>;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub stops: Vec<Stop>,
    pub fight_at_stops: bool,
    pub repeat: bool,
    pub equip: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stop {
    pub room: String,
    pub way: Way,
    pub cross: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Way {
    Walk,
    Fly,
}

fn problems(profiles: &Profiles) -> Vec<String> {
    let mut problems = Vec::new();
    for (id, profile) in profiles {
        if id.trim().is_empty() || profile.stops.is_empty() {
            problems.push(format!(
                "autoplay `{id}` needs a name and at least one stop"
            ));
        }
        for (index, stop) in profile.stops.iter().enumerate() {
            if stop.room.trim().is_empty() {
                problems.push(format!("autoplay `{id}` stop {index} needs a room"));
            }
        }
        if profile
            .equip
            .as_ref()
            .is_some_and(|item| item.trim().is_empty())
        {
            problems.push(format!("autoplay `{id}` has an empty equipment id"));
        }
    }
    problems
}

pub fn autoplay_schema() -> SchemaRegistration {
    rules_schema::<Profiles>(
        AUTOPLAY_SCHEMA,
        1,
        crate::AMBITION_CAPABILITY,
        "Development routes, travel modes, and combat policy for Ambition autoplay.",
        problems,
    )
}

pub fn lowered_profiles(pack: &PreparedContentPack) -> Option<&Profiles> {
    lowered_rules(pack, AUTOPLAY_SCHEMA)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_route_needs_a_destination_before_it_can_drive_input() {
        let mut profiles = Profiles::from([(
            "trip".into(),
            Profile {
                stops: Vec::new(),
                fight_at_stops: false,
                repeat: true,
                equip: None,
            },
        )]);
        assert!(!problems(&profiles).is_empty());
        profiles.get_mut("trip").unwrap().stops.push(Stop {
            room: "hub".into(),
            way: Way::Walk,
            cross: false,
        });
        assert!(problems(&profiles).is_empty());
        profiles.get_mut("trip").unwrap().stops[0].room.clear();
        assert!(!problems(&profiles).is_empty());
    }
}
