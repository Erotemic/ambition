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

/// Pin the rest row of each body whose character states one. (sim)
///
/// A body is left as it is when another row is pinned on it. A body whose
/// character states no rest row and that still has one pinned by this system
/// (it wore such a look and changed) is unpinned.
pub fn pin_rest_rows(
    tick: Res<SimTick>,
    catalog: Res<CharacterCatalog>,
    mut bodies: Query<(&WornCharacter, Option<&mut PinnedRow>, Option<&RestRowPinned>, Entity)>,
    mut commands: Commands,
) {
    let elapsed = tick.get() as f32 * TICK_S;
    for (worn, pinned, ours, entity) in &mut bodies {
        match (catalog.rest_row(worn.id()), pinned) {
            // A body that no conductor draws has no pin yet: it gets one.
            (Some(row), None) => {
                let mut pin = PinnedRow::default();
                pin.pin(&[row], elapsed, true);
                commands.entity(entity).try_insert((pin, RestRowPinned));
            }
            (Some(row), Some(mut pinned)) if ours.is_some() || !pinned.is_pinned() => {
                pinned.pin(&[row], elapsed, true);
                if ours.is_none() {
                    commands.entity(entity).try_insert(RestRowPinned);
                }
            }
            (Some(_), Some(_)) => {}
            (None, pinned) => {
                if ours.is_some() {
                    if let Some(mut pinned) = pinned {
                        pinned.clear();
                    }
                    commands.entity(entity).try_remove::<RestRowPinned>();
                }
            }
        }
    }
}

/// The pin on this body is its rest row, put there by [`pin_rest_rows`].
///
/// Not snapshot state: it is derived again on the next tick from the worn
/// character and the pin, as the pin is.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct RestRowPinned;

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
}
