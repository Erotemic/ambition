//! The verbs a body's worn equipment withholds, as a ceiling on its abilities
//! (ABILITY-VARIANTS 3).
//!
//! A row can stand for a lower level of an ability the body's own kit has
//! whole: Precision Blink's level 1 is the quick blink without the aimed one.
//! The row names those verbs (`EquipmentRow::withholds`). This source writes
//! the worn set's withheld verbs into the body's `AbilityContributions` each
//! tick, and withdraws the entry when no worn row withholds anything, so the
//! kit itself is never edited and a row taken off gives its verbs back.

use bevy::prelude::*;

use ambition_characters::equipment::WornEquipment;
use ambition_platformer2d_core::{AbilityContribution, AbilityContributions, AbilitySet};

/// The worn equipment's key in a body's `AbilityContributions`.
pub const EQUIPMENT_WITHHELD_VERBS: &str = "equipment.withheld_verbs";

/// The ceiling the rows of `worn` put on a body: every verb but the ones some
/// row withholds, or `None` when no row withholds a verb. A verb name the
/// ability set does not know withholds nothing; the item schema refuses it at
/// admission, so it reaches here only from a row built in code.
pub fn worn_ceiling(worn: &WornEquipment) -> Option<AbilitySet> {
    let mut ceiling = AbilitySet::ALL;
    let mut withholds = false;
    for verb in worn.rows.iter().flat_map(|row| &row.withholds) {
        if let Some(allowed) = ceiling.verb_mut(verb) {
            *allowed = false;
            withholds = true;
        }
    }
    withholds.then_some(ceiling)
}

/// Each wearer's body takes the ceiling of what it wears.
pub fn withhold_the_worn_verbs(mut bodies: Query<(&WornEquipment, &mut AbilityContributions)>) {
    for (worn, mut contributions) in &mut bodies {
        let wanted = worn_ceiling(worn).map(AbilityContribution::Ceiling);
        if contributions.get(EQUIPMENT_WITHHELD_VERBS) == wanted {
            continue;
        }
        match wanted {
            Some(ceiling) => contributions.set(EQUIPMENT_WITHHELD_VERBS, ceiling),
            None => contributions.clear(EQUIPMENT_WITHHELD_VERBS),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::equipment::EquipmentRow;

    /// A worn row that withholds the aimed blink takes it from a kit that has
    /// it and leaves the quick blink; taking the row off gives it back.
    #[test]
    fn a_worn_row_withholds_its_verbs_until_it_is_taken_off() {
        let mut kit = AbilitySet::default();
        kit.blink = true;
        kit.precision_blink = true;
        let mut app = App::new();
        app.add_systems(Update, withhold_the_worn_verbs);
        let level_one = EquipmentRow {
            id: "blink_1".into(),
            exclusive_slot: Some("blink".into()),
            withholds: vec!["precision_blink".into()],
            ..Default::default()
        };
        let body = app.world_mut().spawn((WornEquipment::new(vec![level_one]), AbilityContributions::default())).id();
        let verbs = |app: &App| app.world().get::<AbilityContributions>(body).unwrap().apply(kit);

        app.update();
        assert!(verbs(&app).blink && !verbs(&app).precision_blink, "level 1 blinks, without the aimed blink");

        app.world_mut().get_mut::<WornEquipment>(body).unwrap().unequip("blink_1");
        app.update();
        assert_eq!(verbs(&app), kit, "the row off, the kit is whole again");
    }
}
