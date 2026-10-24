//! The `worn_gear` schema: gear that a body wears on an anchor of its art (a
//! space helmet on its head), as data in a game's pack.
//!
//! A row is keyed by the gear id. The gear is an item that is drawn on the
//! wearer's anchor part, and not a part of one character's art. So one row
//! fits many wearers: `fits` gives, for each wearer sheet, the track that is
//! the anchor and how the pictures sit on it. Content declares the rows; a
//! game or a boss conductor puts the gear on a body (its equipment row goes
//! into the body's `WornEquipment`). See `docs/systems/worn-gear.md`.

use std::collections::BTreeMap;

use ambition_content_pack::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind.
pub const WORN_GEAR_SCHEMA: &str = "worn_gear";

/// The part of a wearer that a gear attaches to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum AuthoredGearAnchor {
    /// The head.
    #[default]
    Head,
}

/// How a gear sits on one wearer sheet.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredGearFit {
    /// The track of the sheet's part flipbook that is the anchor part.
    #[serde(default = "head_track")]
    pub track: String,
    /// The size of the pictures, as a part of the row's `size`.
    #[serde(default = "one")]
    pub scale: f32,
    /// The move of the pictures from the anchor's pivot, in world units, in
    /// the frame of the body: +x is the way it faces, +y is up.
    #[serde(default)]
    pub offset: (f32, f32),
    /// The turn of the pictures on the anchor, in degrees. A positive turn
    /// lifts the side of the pictures that is to the way the body faces.
    #[serde(default)]
    pub turn: f32,
}

fn head_track() -> String {
    "head".to_owned()
}

fn one() -> f32 {
    1.0
}

/// One gear.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredWornGear {
    /// The part of the wearer that the gear attaches to.
    #[serde(default)]
    pub anchor: AuthoredGearAnchor,
    /// The exclusive equipment slot (`head`): a body wears one gear in a slot.
    pub slot: String,
    /// The picture drawn under the body, as the asset server names it.
    #[serde(default)]
    pub back: Option<String>,
    /// The picture drawn over the anchor part.
    #[serde(default)]
    pub front: Option<String>,
    /// The size of the pictures in world units, at a fit scale of 1.
    pub size: (f32, f32),
    /// The point of the pictures that is on the anchor's pivot, from 0 to 1
    /// on each axis, from the top left.
    pub socket: (f32, f32),
    /// How the gear sits on each wearer, by the wearer's sheet target
    /// (`player_robot_v3`). A wearer with no fit does not show the gear.
    #[serde(default)]
    pub fits: BTreeMap<String, AuthoredGearFit>,
}

/// A game's worn gear, by gear id.
pub type WornGearRows = BTreeMap<String, AuthoredWornGear>;

/// Each row the game cannot put on a body or draw, named by its id and field.
fn problems(rows: &WornGearRows) -> Vec<String> {
    let mut problems = Vec::new();
    for (id, gear) in rows {
        if gear.slot.trim().is_empty() {
            problems.push(format!("`{id}`: the `slot` is empty"));
        }
        let pictures = [&gear.back, &gear.front];
        if pictures.iter().all(|picture| picture.is_none()) {
            problems.push(format!("`{id}`: it has no `back` and no `front`; give it one picture or both"));
        }
        if pictures.iter().any(|picture| picture.as_deref().is_some_and(|path| path.trim().is_empty())) {
            problems.push(format!("`{id}`: a picture path is empty"));
        }
        let (width, height) = gear.size;
        if !(width.is_finite() && width > 0.0 && height.is_finite() && height > 0.0) {
            problems.push(format!("`{id}`: `size` is {:?}; each side is finite and more than 0", gear.size));
        }
        let (u, v) = gear.socket;
        if !((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v)) {
            problems.push(format!("`{id}`: `socket` is {:?}; each axis is from 0 to 1", gear.socket));
        }
        for (sheet, fit) in &gear.fits {
            if fit.track.trim().is_empty() {
                problems.push(format!("`{id}`: the fit for `{sheet}` has an empty `track`"));
            }
            if !(fit.scale.is_finite() && fit.scale > 0.0) {
                problems.push(format!("`{id}`: the fit for `{sheet}` has `scale` {}; it is finite and more than 0", fit.scale));
            }
            if !(fit.offset.0.is_finite() && fit.offset.1.is_finite() && fit.turn.is_finite()) {
                problems.push(format!("`{id}`: the fit for `{sheet}` has an `offset` or a `turn` that is not finite"));
            }
        }
    }
    problems
}

/// The schema.
pub fn worn_gear_schema() -> SchemaRegistration {
    rules_schema::<WornGearRows>(
        WORN_GEAR_SCHEMA,
        1,
        super::ITEMS_CAPABILITY,
        "Gear a body wears on an anchor of its art (a helmet on its head): its \
         equipment slot, the pictures under and over the anchor, their size and \
         socket, and how it fits each wearer sheet.",
        problems,
    )
}

/// The rows the pack states, or `None` when it has no `worn_gear` source.
pub fn lowered_worn_gear(pack: &PreparedContentPack) -> Option<&WornGearRows> {
    lowered_rules::<WornGearRows>(pack, WORN_GEAR_SCHEMA)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELMET: &str = r#"{
        "space_helmet": (
            slot: "head",
            back: Some("sprites/space_helmet_back.png"),
            front: Some("sprites/space_helmet_front.png"),
            size: (64.0, 64.0),
            socket: (0.5, 0.58),
            fits: {
                "player_robot_v3": (scale: 1.0),
                "gnu_ton_scholar_boss": (track: "head_face", scale: 0.9, offset: (2.0, -1.0), turn: 4.0),
            },
        ),
    }"#;

    /// The shipped shape reads, and a fit that names only its scale anchors
    /// to the `head` track.
    #[test]
    fn a_helmet_row_reads_and_a_fit_defaults_to_the_head_track() {
        let rows: WornGearRows = ron::from_str(HELMET).expect("the row reads");
        assert!(problems(&rows).is_empty(), "{:?}", problems(&rows));
        let helmet = &rows["space_helmet"];
        assert_eq!(helmet.anchor, AuthoredGearAnchor::Head);
        assert_eq!(helmet.fits["player_robot_v3"].track, "head");
        assert_eq!(helmet.fits["player_robot_v3"].offset, (0.0, 0.0));
        assert_eq!(helmet.fits["gnu_ton_scholar_boss"].track, "head_face");
    }

    /// A row with no picture, a socket off the picture, or a fit at no size
    /// is named by its id and field.
    #[test]
    fn a_row_the_game_cannot_draw_is_named_by_its_id_and_field() {
        let mut rows: WornGearRows = ron::from_str(HELMET).unwrap();
        let helmet = rows.get_mut("space_helmet").unwrap();
        helmet.back = None;
        helmet.front = None;
        helmet.socket = (0.5, 1.2);
        helmet.fits.get_mut("player_robot_v3").unwrap().scale = 0.0;
        let found = problems(&rows);
        for field in ["no `back` and no `front`", "`socket`", "`player_robot_v3` has `scale`"] {
            assert!(found.iter().any(|problem| problem.starts_with("`space_helmet`") && problem.contains(field)), "{field}: {found:?}");
        }
        assert!(ron::from_str::<WornGearRows>(&HELMET.replace("socket:", "sokcet:")).is_err(), "a typo of a field is refused");
    }
}
