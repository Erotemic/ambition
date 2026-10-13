//! The Noodly Temple: the level of the Flying Spaghetti Monster.
//!
//! Seven rooms round one hall in the `boss` biome
//! (`tools/ambition_ldtk_tools/specs/noodly_temple/make_specs.py`). The way to
//! the monster is shut by doors, and each door has a key. The rooms, the
//! keys, the doors, the levers and the enemies are authored in the map, and
//! the two fights in `data/encounters/goblin_encounter.ron`. What the map
//! cannot author is here: the sheets that its keys and its doors name.
//!
//! A key is a `PickupSpawn` that sets a flag (`sprite` names its sheet). A
//! door is a `LockWall` that stands until its `gated_by` is true (`look`
//! names its sheet).

use ambition_platformer2d::sprite_sheet::character::SheetTuning;
use ambition_platformer2d_actor_monolith::assets::game_assets::{PropSheetSource, PropSheetsAppExt};
use bevy::prelude::*;

/// The sauces: a key and a door for each. The colander is the last.
pub const SAUCES: [&str; 4] = ["marinara", "pesto", "parmesan", "colander"];

/// The doors that no key opens: the grille of a fight, the sluices of the
/// cistern (by the number of the wheel; `_shut` is a sluice its wheel shuts)
/// and the seal of a bell.
pub const OTHER_DOORS: [&str; 6] = [
    "temple_gate_iron",
    "temple_gate_sluice_1",
    "temple_gate_sluice_2",
    "temple_gate_sluice_3",
    "temple_gate_sluice_1_shut",
    "temple_gate_bell",
];

/// The sheet of the key of `sauce`, and the renderer target of it.
pub fn key_sheet(sauce: &str) -> String {
    format!("temple_key_{sauce}")
}

/// The sheet of the door that the key of `sauce` opens.
pub fn door_sheet(sauce: &str) -> String {
    format!("temple_gate_{sauce}")
}

/// The flag that the key of `sauce` sets, and that its doors ask.
pub fn key_flag(sauce: &str) -> String {
    format!("noodly_key_{sauce}")
}

pub struct NoodlyTemplePlugin;

impl Plugin for NoodlyTemplePlugin {
    fn build(&self, app: &mut App) {
        // A key is drawn a little larger than its box, as a pickup is. A door
        // fills the box of its wall.
        for sauce in SAUCES {
            let key = key_sheet(sauce);
            app.register_prop_sheet(key.clone(), PropSheetSource::SpriteFolder { target: key, tuning: SheetTuning::new(1.4, 0) });
            let door = door_sheet(sauce);
            app.register_prop_sheet(door.clone(), PropSheetSource::SpriteFolder { target: door, tuning: SheetTuning::new(1.0, 0) });
        }
        for door in OTHER_DOORS {
            app.register_prop_sheet(door, PropSheetSource::SpriteFolder { target: door.to_string(), tuning: SheetTuning::new(1.0, 0) });
        }
    }
}
