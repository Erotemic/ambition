//! A character whose rest is a row of its own: the catalog's `rest_row`.
//!
//! A sheet can hold more than one way to draw a character. The giant gnu's
//! sheet has the body the fight draws with no arms (its fists are bodies of
//! their own there), and rows with its arms on it. The look that is the giant
//! WHOLE is a catalog row that names the row with arms as its rest
//! (`rest_row`), and the Hall's stand wears that look.
//!
//! This pins that row on each body that wears such a character and that
//! nothing else has pinned. It is the channel a fight's conductor uses
//! ([`PinnedRow`]): a conductor that pins a row of its own wins, and the pin
//! is a function of the tick, so a rewind draws the same frame.

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::actor::WornCharacter;
use ambition_sprite_sheet::character::PinnedRow;
use ambition_time::SimTick;
use bevy::prelude::*;

/// The length of a tick, for the clock of a rest row (s).
const TICK_S: f32 = 1.0 / 60.0;

/// The name [`pin_rest_rows`] writes its pins under ([`PinnedRow::pin_as`]).
pub const REST_ROW_WRITER: &str = "rest_row";

/// Pin the rest row of each body whose character states one. (sim)
///
/// The rest row is a fallback. A body is left as it is while a pin that this
/// system did not write is held on it, whichever system pinned first and
/// whichever row it pinned: GNU-ton's choreography pins `armed_rest` with
/// its own clock, which is the row a rest row also names. When the other
/// system clears its pin, the rest row comes back. A body whose character
/// states no rest row and that still holds a pin this system wrote (it wore
/// such a look and changed) is unpinned.
///
/// ⛔ Whose pin it is was read from the row: the pin was this system's while
/// it held the row this system last wrote. A conductor that pinned that same
/// row lost its clock to this system on the next tick (review of
/// 2026-10-14). The pin now names its writer.
pub fn pin_rest_rows(
    tick: Res<SimTick>,
    catalog: Res<CharacterCatalog>,
    mut bodies: Query<(&WornCharacter, Option<&mut PinnedRow>, Entity)>,
    mut commands: Commands,
) {
    let elapsed = tick.get() as f32 * TICK_S;
    for (worn, pinned, entity) in &mut bodies {
        match (catalog.rest_row(worn.id()), pinned) {
            // A body that no conductor draws has no pin yet: it gets one.
            (Some(row), None) => {
                let mut pin = PinnedRow::default();
                pin.pin_as(REST_ROW_WRITER, &[row], elapsed, true);
                commands.entity(entity).try_insert(pin);
            }
            (Some(row), Some(mut pinned)) if pinned.written_by(REST_ROW_WRITER) || !pinned.is_pinned() => {
                pinned.pin_as(REST_ROW_WRITER, &[row], elapsed, true);
            }
            (Some(_), Some(_)) => {}
            (None, Some(mut pinned)) if pinned.written_by(REST_ROW_WRITER) => pinned.clear(),
            (None, _) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::actor::character_catalog::CharacterCatalogData;

    fn catalog() -> CharacterCatalog {
        let row = |name: &str, extra: &str| {
            format!(
                r#""{name}": (
                    display_name: "{name}",
                    spritesheet: "sprites/{name}.png",
                    manifest: "sprites/{name}.ron",
                    tier: MainHall,
                    body_kind: Standard,
                    composition: None,
                    default_brain: "idle",
                    default_action_set: "peaceful",
                    {extra}
                ),"#
            )
        };
        let data: CharacterCatalogData = ron::from_str(&format!(
            r#"(
                brain_presets: {{ "idle": StandStill }},
                action_set_presets: {{ "peaceful": (move_style: Walk) }},
                characters: {{ {} {} }},
            )"#,
            row("giant", ""),
            row("giant_whole", r#"rest_row: Some("armed_rest"),"#),
        ))
        .expect("the fixture catalog parses");
        CharacterCatalog::from_data(data)
    }

    /// A body that wears a character with a rest row has that row pinned; a
    /// row another system pinned first stays; and a body that changes to a
    /// character with no rest row is unpinned.
    #[test]
    fn a_rest_row_is_pinned_on_a_body_that_nothing_else_pins() {
        let mut app = App::new();
        app.insert_resource(catalog());
        app.init_resource::<SimTick>();
        app.add_systems(Update, pin_rest_rows);
        // A body no conductor draws has no pin of its own.
        let whole = app.world_mut().spawn(WornCharacter::new("giant_whole")).id();
        let plain = app.world_mut().spawn((WornCharacter::new("giant"), PinnedRow::default())).id();
        let mut conducted = PinnedRow::default();
        conducted.pin(&["buck"], 0.2, false);
        let fought = app.world_mut().spawn((WornCharacter::new("giant_whole"), conducted)).id();
        app.update();
        app.update();

        let rows = |app: &App, body: Entity| app.world().get::<PinnedRow>(body).expect("a pin").rows.clone();
        assert_eq!(rows(&app, whole), vec!["armed_rest".to_string()]);
        assert!(app.world().get::<PinnedRow>(whole).expect("a pin").looping);
        assert!(rows(&app, plain).is_empty(), "a character with no rest row was pinned");
        assert_eq!(rows(&app, fought), vec!["buck".to_string()], "a conductor's pin was replaced");

        // It changes to the look with no rest row: the pin goes with the look.
        *app.world_mut().get_mut::<WornCharacter>(whole).expect("the body") = WornCharacter::new("giant");
        app.update();
        assert!(rows(&app, whole).is_empty(), "the rest row of a look the body no longer wears stayed");
    }

    /// A conductor that pins `attack` while the fight asks it to, and clears
    /// its pin when the fight releases it.
    #[derive(Resource, Default)]
    struct Attacking(bool);

    fn conductor(attacking: Res<Attacking>, mut pins: Query<&mut PinnedRow, With<WornCharacter>>) {
        for mut pin in &mut pins {
            if attacking.0 {
                pin.pin(&["attack"], 0.0, false);
            } else if pin.rows == ["attack"] {
                pin.clear();
            }
        }
    }

    /// A conductor that pins the rest row's own row, with its own clock and
    /// no loop: the pin is the conductor's until it is released, and then the
    /// rest row comes back. Whose pin it is was read from the row, so the rest
    /// row took this pin over and wrote its own clock on it.
    #[test]
    fn a_conductor_that_pins_the_same_row_keeps_its_own_clock() {
        let mut app = App::new();
        app.insert_resource(catalog());
        app.init_resource::<SimTick>();
        app.add_systems(Update, pin_rest_rows);
        let body = app.world_mut().spawn(WornCharacter::new("giant_whole")).id();
        app.update();
        app.update();
        assert!(
            app.world().get::<PinnedRow>(body).expect("a pin").written_by(REST_ROW_WRITER),
            "the premise: the rest row is pinned"
        );

        app.world_mut().get_mut::<PinnedRow>(body).expect("a pin").pin(&["armed_rest"], 0.37, false);
        for tick in 1..4 {
            *app.world_mut().resource_mut::<SimTick>() = SimTick(tick * 30);
            app.update();
            let pin = app.world().get::<PinnedRow>(body).expect("a pin");
            assert_eq!(
                (pin.rows.as_slice(), pin.elapsed, pin.looping),
                (["armed_rest".to_string()].as_slice(), 0.37, false),
                "the rest row took over a conductor's pin of the same row"
            );
        }

        app.world_mut().get_mut::<PinnedRow>(body).expect("a pin").clear();
        app.update();
        assert!(
            app.world().get::<PinnedRow>(body).expect("a pin").written_by(REST_ROW_WRITER),
            "the rest row did not come back after the release"
        );
    }

    /// The rest row first, then a conductor's row, then the release: the
    /// conductor's row is drawn until it is released, and then the rest row
    /// comes back on the clock of the tick, in both orders of the two writers.
    /// A marker on the body once made the rest row write over a later pin.
    #[test]
    fn a_pin_made_after_the_rest_row_holds_until_it_is_released() {
        for conductor_runs_first in [true, false] {
            let mut app = App::new();
            app.insert_resource(catalog());
            app.init_resource::<SimTick>();
            app.init_resource::<Attacking>();
            if conductor_runs_first {
                app.add_systems(Update, (conductor, pin_rest_rows).chain());
            } else {
                app.add_systems(Update, (pin_rest_rows, conductor).chain());
            }
            let body = app.world_mut().spawn(WornCharacter::new("giant_whole")).id();
            let rows = |app: &App| app.world().get::<PinnedRow>(body).expect("a pin").rows.clone();
            app.update();
            app.update();
            assert_eq!(rows(&app), ["armed_rest"], "the premise: the rest row is pinned");

            app.world_mut().resource_mut::<Attacking>().0 = true;
            for _ in 0..3 {
                app.update();
                assert_eq!(rows(&app), ["attack"], "the rest row wrote over the conductor's pin (conductor first: {conductor_runs_first})");
            }

            app.world_mut().resource_mut::<Attacking>().0 = false;
            app.update();
            if !conductor_runs_first {
                // The conductor cleared after the rest row ran; the next tick restores it.
                app.update();
            }
            *app.world_mut().resource_mut::<SimTick>() = SimTick(120);
            app.update();
            let pin = app.world().get::<PinnedRow>(body).expect("a pin");
            assert_eq!(pin.rows, ["armed_rest"], "the rest row did not come back after the release");
            assert_eq!(pin.elapsed, 120.0 * TICK_S, "the rest row is not on the clock of the tick, so a rewind would draw another frame");
        }
    }
}
