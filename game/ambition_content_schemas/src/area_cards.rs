//! The `area_cards` schema: the areas of Ambition's world, by the rooms each
//! holds, as data in its pack (`assets/data/area_cards.ron`).
//!
//! Entering a room of another area shows that area's card: its name across
//! the view, its subtitle under it. A room in no area shows nothing and
//! leaves the last area standing.

use std::collections::BTreeMap;

use ambition_content_pack::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind.
pub const AREA_CARDS_SCHEMA: &str = "area_cards";

/// One area: its card and its rooms.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AreaCardRow {
    pub name: String,
    #[serde(default)]
    pub subtitle: Option<String>,
    /// Room ids. One that ends in `*` is every room whose id starts with
    /// what comes before it. A room named exactly wins over a prefix, and a
    /// longer prefix over a shorter one.
    pub rooms: Vec<String>,
}

/// Each row the cards cannot show, named by its index. `&Vec` because
/// `rules_schema` passes `&T` and the file is a list.
#[allow(clippy::ptr_arg)]
fn problems(areas: &Vec<AreaCardRow>) -> Vec<String> {
    let mut problems = Vec::new();
    let mut owner: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, area) in areas.iter().enumerate() {
        if area.name.trim().is_empty() {
            problems.push(format!("`[{index}]` has no name"));
        }
        if area.rooms.is_empty() {
            problems.push(format!("`[{index}]` (`{}`) holds no room", area.name));
        }
        for room in &area.rooms {
            if room.trim_end_matches('*').trim().is_empty() {
                problems.push(format!("`[{index}]` (`{}`) names an empty room", area.name));
            } else if let Some(first) = owner.insert(room.as_str(), index) {
                problems.push(format!("`{room}` is in `[{first}]` and `[{index}]`; a room is in one area"));
            }
        }
    }
    problems
}

/// The schema, for Ambition's pack.
pub fn area_cards_schema() -> SchemaRegistration {
    rules_schema::<Vec<AreaCardRow>>(
        AREA_CARDS_SCHEMA,
        1,
        crate::AMBITION_CAPABILITY,
        "Ambition's areas: the card shown on entering each, and the rooms it holds.",
        problems,
    )
}

/// The areas `pack` states, if it states any.
pub fn lowered_area_cards(pack: &PreparedContentPack) -> Option<&Vec<AreaCardRow>> {
    lowered_rules::<Vec<AreaCardRow>>(pack, AREA_CARDS_SCHEMA)
}
