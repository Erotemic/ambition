//! Ambition's gate portals (the pack's `data/gate_portals.ron`): the intro's
//! gate and the Math gate pair, by the loading zone each one holds.
//!
//! This registers the rows into the App's [`GatePortalRegistry`]; a content
//! reload restates Ambition's rows (`reload::publish_gate_portals`) and no
//! other provider's.

use ambition_platformer2d::world::rooms::{GatePortalRegistry, GatePortalRows, GATE_PORTALS_SCHEMA};
use bevy::prelude::{App, Resource};

/// The portal zones Ambition's pack registered: the rows a reload restates,
/// and no other provider's.
#[derive(Resource, Clone, Debug, Default)]
pub(crate) struct AmbitionGatePortals(pub(crate) Vec<String>);

/// The gate portals a pack states.
pub(crate) fn lowered_gate_portals(pack: &ambition_content_pack::PreparedContentPack) -> GatePortalRows {
    ambition_content_pack::lowered_rules::<GatePortalRows>(pack, GATE_PORTALS_SCHEMA).cloned().unwrap_or_default()
}

/// Register every gate portal the selected pack states.
pub(super) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    let rows = lowered_gate_portals(&pack);
    let world = app.world_mut();
    // A refusal is a content bug: another provider's portal holds the zone.
    // Logged, not panicked: a missing portal leaves a world that still runs.
    match world.get_resource_or_init::<GatePortalRegistry>().restate([], &rows) {
        Ok(zones) => {
            world.insert_resource(AmbitionGatePortals(zones));
        }
        Err(problem) => bevy::log::error!("Ambition's gate portals were refused: {problem}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The portals are the pack's rows: the App registers each row the file
    /// states, an edit is the lowered row, and a row the runtime cannot use
    /// is refused at admission with its zone and field named.
    #[test]
    fn the_gate_portals_are_the_packs_rows() {
        let mut app = App::new();
        register(&mut app);
        let registry = app.world().resource::<GatePortalRegistry>();
        let shipped = lowered_gate_portals(crate::pack::shipped());
        assert!(!shipped.is_empty(), "the pack states its gate portals");
        for (zone, portal) in &shipped {
            assert_eq!(registry.iter().find(|(known, _)| *known == zone).map(|(_, p)| p), Some(portal), "`{zone}`");
        }
        assert_eq!(app.world().resource::<AmbitionGatePortals>().0, shipped.keys().cloned().collect::<Vec<_>>());

        let switch = r#"switch_id: "intro_portal_switch","#;
        let with = |replacement: &'static str| {
            crate::pack::compile_pack_with(move |path, text| {
                if path != "data/gate_portals.ron" {
                    return text;
                }
                assert_eq!(text.matches(switch).count(), 1, "the probe edits one switch");
                text.replace(switch, replacement)
            })
        };
        let edited = with(r#"switch_id: "another_switch","#).expect("an edited row compiles");
        assert_eq!(lowered_gate_portals(&edited)["intro_portal_zone"].switch_id, "another_switch");
        let refused = format!("{:?}", with(r#"switch_id: "","#).expect_err("a portal with no switch"));
        assert!(refused.contains("`intro_portal_zone.switch_id` is empty"), "{refused}");
        assert!(refused.contains("gate_portals.ron"), "{refused}");
    }

    /// Each row names what the maps hold: its zone and its two props are in
    /// a world file under these names, and its switch too. A rename on one
    /// side only would leave a gate that never opens.
    #[test]
    fn each_gate_portal_names_what_a_world_file_holds() {
        let worlds = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/worlds");
        let texts: Vec<String> = std::fs::read_dir(&worlds)
            .expect("the world files are readable")
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "ldtk"))
            .map(|path| std::fs::read_to_string(path).expect("a world file is readable"))
            .collect();
        for (zone, portal) in &lowered_gate_portals(crate::pack::shipped()) {
            for name in [zone, &portal.switch_id, &portal.portal_sprite_name, &portal.ring_sprite_name] {
                let quoted = format!("\"{name}\"");
                assert!(texts.iter().any(|text| text.contains(&quoted)), "`{name}` ({zone}) is in no world file");
            }
        }
    }

    /// A reload restates Ambition's portals and refuses another provider's
    /// zone.
    #[test]
    fn a_reload_restates_ambitions_gate_portals_and_no_other_providers() {
        let mut app = App::new();
        register(&mut app);
        app.world_mut()
            .resource_mut::<GatePortalRegistry>()
            .try_register("their_zone", "s", "Their Portal", "Their Ring")
            .unwrap();
        let ours = app.world().resource::<AmbitionGatePortals>().0.clone();
        let mut next = lowered_gate_portals(crate::pack::shipped());
        next.remove("math_gate_return_zone");
        let mut registry = app.world_mut().resource_mut::<GatePortalRegistry>();
        let owned = registry.restate(ours.iter().map(String::as_str), &next).expect("Ambition's own zones");
        assert!(!registry.is_portal("math_gate_return_zone") && registry.is_portal("their_zone"));
        let stolen = next.values().next().unwrap().clone();
        next.insert("their_zone".into(), stolen);
        assert!(registry.restate(owned.iter().map(String::as_str), &next).is_err(), "another provider's zone");
    }
}
