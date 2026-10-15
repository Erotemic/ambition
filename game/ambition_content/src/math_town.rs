//! The town of the Math dimension, and the gate pair that leads to it.
//!
//! The Math dimension is its own world file, `math.ldtk` (one world file is
//! one dimension). Its rooms are written by
//! `tools/ambition_ldtk_tools/specs/math_town/make_specs.py`: the gate square
//! (`math_gate_hall`) and the street under the terraces (`math_town_street`).
//! What the map cannot author is here: the two gates' rows in the
//! [`GatePortalRegistry`].
//!
//! The gate pair is one link with one switch. Gate 7 of the Home gate station
//! (`gate_stack_lower`) and the gate on the Math square both follow
//! [`MATH_GATE_SWITCH_ID`], which stands by the Home gate: switched on there,
//! both portals boot, so a traveller who went through can come back.

use ambition_platformer2d::world::rooms::GatePortalRegistry;
use ambition_platformer2d_actor_monolith::assets::game_assets::{PropSheetSource, PropSheetsAppExt};
use bevy::prelude::*;

/// The `Prop` kind of the stand a gate ring sits in, and its renderer target:
/// a plinth with stairs at each end and two pylons that hold the ring. Its
/// stairs are walked on zero-speed one-way platforms the room places.
pub const GATE_CRADLE_KIND: &str = "gate_cradle";

/// The house fronts of the Math town (`targets/props/math_town_facades.py`):
/// decoration `Prop` kinds, each its own sheet. A balcony or roof a body can
/// stand on is a one-way platform the room places where the art draws it.
pub const HOUSE_KINDS: [&str; 5] = [
    "math_house_rose",
    "math_house_dome",
    "math_house_tower",
    "math_house_shop",
    "math_house_arcade",
];

/// The switch both gates of the pair follow (a `Switch` in `gate_stack_lower`).
pub const MATH_GATE_SWITCH_ID: &str = "math_gate_switch";

/// One gate of the pair: its door, and the names of its ring and portal
/// props (the gate visuals find a gate's props by these names, across rooms,
/// so each gate has its own).
pub struct MathGate {
    pub zone_id: &'static str,
    pub ring_name: &'static str,
    pub portal_name: &'static str,
}

/// Gate 7 in the Home gate station, to the Math square.
pub const HOME_GATE: MathGate = MathGate {
    zone_id: "math_gate_home_zone",
    ring_name: "Math Gate Ring (Home)",
    portal_name: "Math Gate Portal (Home)",
};

/// The gate on the Math square, back to the Home gate station.
pub const MATH_GATE: MathGate = MathGate {
    zone_id: "math_gate_return_zone",
    ring_name: "Math Gate Ring (Math)",
    portal_name: "Math Gate Portal (Math)",
};

pub struct MathTownPlugin;

impl Plugin for MathTownPlugin {
    fn build(&self, app: &mut App) {
        // The cradle and the house fronts are scenery: their sheets load with
        // the rest of the art.
        for kind in std::iter::once(GATE_CRADLE_KIND).chain(HOUSE_KINDS) {
            app.register_prop_sheet(
                kind,
                PropSheetSource::SpriteFolder {
                    target: kind.to_string(),
                    tuning: ambition_platformer2d::sprite_sheet::character::SheetTuning::new(1.0, 0),
                },
            );
        }
        let registry = &mut *app.world_mut().get_resource_or_init::<GatePortalRegistry>();
        for gate in [HOME_GATE, MATH_GATE] {
            // A refusal is a content bug: another portal already claimed the
            // zone. Logged, not panicked: a missing gate leaves a world that
            // still runs.
            if let Err(conflict) =
                registry.try_register(gate.zone_id, MATH_GATE_SWITCH_ID, gate.portal_name, gate.ring_name)
            {
                bevy::log::error!("a gate of the Math dimension was refused: {conflict}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The registry rows name what the maps hold: each gate's door and its
    /// two props are in the world file its room is in, under these names. A
    /// rename on one side only would leave a gate that never opens.
    #[test]
    fn each_gate_names_a_door_and_props_its_world_holds() {
        let worlds = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/worlds");
        for (gate, file) in [(HOME_GATE, "intro.ldtk"), (MATH_GATE, "math.ldtk")] {
            let text = std::fs::read_to_string(worlds.join(file)).expect("the world file is readable");
            for name in [gate.zone_id, gate.ring_name, gate.portal_name] {
                assert!(text.contains(&format!("\"{name}\"")), "`{name}` is not in {file}");
            }
        }
        let intro = std::fs::read_to_string(worlds.join("intro.ldtk")).expect("intro.ldtk is readable");
        assert!(intro.contains(&format!("\"{MATH_GATE_SWITCH_ID}\"")), "the gate switch is not in intro.ldtk");
    }

    #[test]
    fn both_gates_are_registered_on_the_one_switch() {
        let mut app = App::new();
        app.add_plugins(MathTownPlugin);
        let registry = app.world().resource::<GatePortalRegistry>();
        for gate in [HOME_GATE, MATH_GATE] {
            assert!(registry.is_portal(gate.zone_id), "`{}` is not a registered gate", gate.zone_id);
        }
    }
}
