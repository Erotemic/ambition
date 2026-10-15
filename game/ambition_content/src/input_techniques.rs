//! Ambition's motion techniques: the pack's `data/motion_techniques.ron`.
//!
//! The named fighting-game gestures the player kit uses (the quarter-circle
//! `qcf`, its keyboard-friendly grace form `qcf_grace`, and the half-circle
//! `hcf`) are rows of that file (schema `motion_techniques`, owned by the
//! projectile capability). This registers them into the App's
//! [`MotionTechniqueCatalog`](ambition_projectiles::MotionTechniqueCatalog);
//! the player fire system asks the catalog by id whether each fired, and a
//! content reload restates Ambition's rows (`reload::publish_motion_techniques`).

use ambition_projectiles::MotionTechniqueAppExt;
use bevy::prelude::{App, Resource};

/// The technique ids Ambition's pack registered: the rows a reload restates,
/// and no other provider's.
#[derive(Resource, Clone, Debug, Default)]
pub(crate) struct AmbitionMotionTechniques(pub(crate) Vec<String>);

/// Register every technique the selected pack states into the App-local
/// catalog.
pub(super) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    let techniques =
        ambition_projectiles::content_schema::lowered_motion_techniques(&pack).cloned().unwrap_or_default();
    for (id, technique) in &techniques {
        app.register_motion_technique(id.clone(), technique.clone());
    }
    app.insert_resource(AmbitionMotionTechniques(techniques.into_keys().collect()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_projectiles::{MotionDirection, MotionInputBuffer, MotionTechniqueCatalog};
    use MotionDirection::{Down, DownLeft, DownRight, Left, Right};

    fn buffer_of(dirs: &[MotionDirection]) -> MotionInputBuffer {
        let mut buf = MotionInputBuffer::new(1.0);
        let mut t = 0.0;
        for &dir in dirs {
            buf.push(dir, t);
            t += 0.04;
        }
        buf
    }

    #[test]
    fn ambition_registers_the_player_kit_techniques() {
        let mut app = App::new();
        register(&mut app);
        let catalog = app.world().resource::<MotionTechniqueCatalog>();
        for id in ["qcf", "qcf_grace", "hcf"] {
            assert!(catalog.get(id).is_some(), "{id} must be registered");
        }
        assert!(catalog.get("dragon_punch").is_none());
    }

    #[test]
    fn registered_qcf_and_hcf_detect_their_gestures() {
        let mut app = App::new();
        register(&mut app);
        let catalog = app.world().resource::<MotionTechniqueCatalog>();

        let qcf_right = buffer_of(&[Down, DownRight, Right]);
        assert_eq!(catalog.detect("qcf", &qcf_right), Some(1.0));

        // Half-circle right→left ends on Left (facing -1) but fires facing +1.
        let hcf = buffer_of(&[Right, DownRight, Down, DownLeft, Left]);
        assert_eq!(catalog.detect("hcf", &hcf), Some(1.0));
    }

    /// The techniques are the pack's rows: an edit is the lowered technique,
    /// and a technique no input can fire is refused at admission with its id
    /// named.
    #[test]
    fn the_motion_techniques_are_the_packs_rows() {
        use ambition_projectiles::content_schema::lowered_motion_techniques;
        let row = r#""qcf_grace": (patterns: [[Down, Right], [Down, Left]]),"#;
        let with = |replacement: &'static str| {
            crate::pack::compile_pack_with(move |path, text| {
                if path != "data/motion_techniques.ron" {
                    return text;
                }
                assert_eq!(text.matches(row).count(), 1, "the probe edits one row");
                text.replace(row, replacement)
            })
        };
        let edited = with(r#""qcf_grace": (patterns: [[Down, Right]]),"#).expect("an edited row compiles");
        let techniques = lowered_motion_techniques(&edited).expect("the edited pack lowers its techniques");
        assert_eq!(techniques["qcf_grace"].patterns, vec![vec![Down, Right]]);

        let refused = format!("{:?}", with(r#""qcf_grace": (patterns: []),"#).expect_err("a technique with no patterns"));
        assert!(refused.contains("`qcf_grace` has no patterns"), "{refused}");
    }

    /// A reload restates Ambition's rows and refuses another provider's id.
    #[test]
    fn a_reload_restates_ambitions_techniques_and_no_other_providers() {
        let mut app = App::new();
        register(&mut app);
        app.register_motion_technique("dragon_punch", ambition_projectiles::MotionTechnique::new(vec![vec![Right, Down, DownRight]]));
        let ours = app.world().resource::<AmbitionMotionTechniques>().0.clone();
        let mut next = ambition_projectiles::content_schema::lowered_motion_techniques(crate::pack::shipped()).cloned().unwrap();
        next.remove("qcf_grace");
        let mut catalog = app.world_mut().resource_mut::<MotionTechniqueCatalog>();
        let owned = catalog.restate(ours.iter().map(String::as_str), &next).expect("Ambition's own ids");
        assert!(catalog.get("qcf_grace").is_none() && catalog.get("dragon_punch").is_some());
        next.insert("dragon_punch".into(), ambition_projectiles::MotionTechnique::new(vec![vec![Down]]));
        assert!(catalog.restate(owned.iter().map(String::as_str), &next).is_err(), "another provider's id");
    }
}
