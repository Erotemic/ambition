//! Ambition's prop sheets: the pack's `data/prop_sheets.ron`.
//!
//! A row is keyed by the `kind` a room's prop names (the Noodly Temple's keys
//! and doors, the Math town's fronts, the primeval palm, the Mockingbird's
//! shark), and loads that sheet with the rest of the art. This registers the
//! rows into the App's [`PropSheets`]; a content reload restates Ambition's
//! rows and loads their sheets again (`reload::publish_prop_sheets`).

use ambition_platformer2d_actor_monolith::assets::game_assets::PropSheetsAppExt;
use bevy::prelude::{App, Resource};

#[cfg(test)]
use ambition_platformer2d_actor_monolith::assets::game_assets::PropSheets;

/// The prop kinds Ambition's pack registered: the rows a reload restates, and
/// no other provider's.
#[derive(Resource, Clone, Debug, Default)]
pub(crate) struct AmbitionPropSheets(pub(crate) Vec<String>);

/// Register every prop sheet the selected pack states.
pub(super) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    let rows = ambition_sprite_sheet::prop_sheets::lowered_prop_sheets(&pack).cloned().unwrap_or_default();
    let kinds = app.register_prop_sheet_rows(&rows);
    app.insert_resource(AmbitionPropSheets(kinds));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_actor_monolith::assets::game_assets::PropSheetSource;
    use ambition_sprite_sheet::prop_sheets::lowered_prop_sheets;

    fn tuning_of(sheets: &PropSheets, kind: &str) -> Option<ambition_sprite_sheet::character::SheetTuning> {
        sheets.iter().find(|(known, _)| *known == kind).map(|(_, source)| match source {
            PropSheetSource::SpriteFolder { target, tuning } => {
                assert_eq!(target, kind, "a pack row's sheet target is its kind");
                *tuning
            }
            PropSheetSource::Catalog { .. } => panic!("`{kind}` is a pack row, not a catalog sheet"),
        })
    }

    /// The prop sheets are the pack's rows: the App registers each row the
    /// file states with its tuning, an edit is the lowered row, and a row the
    /// loader cannot draw is refused at admission with its kind named.
    #[test]
    fn the_prop_sheets_are_the_packs_rows() {
        let mut app = App::new();
        register(&mut app);
        let sheets = app.world().resource::<PropSheets>();
        let shipped = lowered_prop_sheets(crate::pack::shipped()).expect("the pack states its prop sheets");
        for (kind, row) in shipped {
            assert_eq!(tuning_of(sheets, kind), Some(row.tuning()), "`{kind}` is registered as the file states it");
        }
        assert_eq!(app.world().resource::<AmbitionPropSheets>().0, shipped.keys().cloned().collect::<Vec<_>>());

        let row = r#""temple_key_pesto": (scale: 1.4),"#;
        let with = |replacement: &'static str| {
            crate::pack::compile_pack_with(move |path, text| {
                if path != "data/prop_sheets.ron" {
                    return text;
                }
                assert_eq!(text.matches(row).count(), 1, "the probe edits one row");
                text.replace(row, replacement)
            })
        };
        let edited = with(r#""temple_key_pesto": (scale: 2.0),"#).expect("an edited row compiles");
        assert_eq!(lowered_prop_sheets(&edited).expect("the edited pack lowers its rows")["temple_key_pesto"].scale, 2.0);
        let refused = format!("{:?}", with(r#""temple_key_pesto": (scale: 0.0),"#).expect_err("a sheet drawn at no size"));
        assert!(refused.contains("`temple_key_pesto.scale` is 0"), "{refused}");
    }

    /// A reload restates Ambition's rows and refuses another provider's kind.
    #[test]
    fn a_reload_restates_ambitions_prop_sheets_and_no_other_providers() {
        let mut app = App::new();
        register(&mut app);
        app.register_prop_sheet(
            "intro_cart",
            PropSheetSource::SpriteFolder { target: "intro_cart".into(), tuning: Default::default() },
        );
        let ours = app.world().resource::<AmbitionPropSheets>().0.clone();
        let mut next = lowered_prop_sheets(crate::pack::shipped()).cloned().unwrap();
        next.remove("coconut_palm");
        let mut sheets = app.world_mut().resource_mut::<PropSheets>();
        let owned = sheets.restate(ours.iter().map(String::as_str), &next).expect("Ambition's own kinds");
        assert!(tuning_of(&sheets, "coconut_palm").is_none() && sheets.iter().any(|(kind, _)| kind == "intro_cart"));
        next.insert("intro_cart".into(), ambition_sprite_sheet::prop_sheets::AuthoredPropSheet { scale: 1.0, frame_sample_inset: 0 });
        assert!(sheets.restate(owned.iter().map(String::as_str), &next).is_err(), "another provider's kind");
    }
}
