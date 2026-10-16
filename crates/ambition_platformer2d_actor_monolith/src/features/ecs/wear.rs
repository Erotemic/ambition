//! Wearing by request: a conversation's `<<transform "id">>` makes the
//! character being talked to wear another catalog character.
//!
//! A character with a transform is one character with more than one look
//! (Jeff Hinter and his armor, Mary-O and her grown forms, Sanic and Super
//! Sanic). Each look is a catalog row with its own sheet, and the body wears
//! one of them at a time ([`WornCharacter`]). A game changes the worn row by
//! its own rules (a power-up, a technique). This request is the same change,
//! asked for by a conversation: the hall of characters uses it to show each
//! look of a character on the one body that stands there.
//!
//! The request changes [`WornCharacter`] and asks for the template to be
//! applied again ([`RecharacterizeBody`]). It does not change what the body
//! is to talk to: its conversation stays the one authored on its placement.
//!
//! When the sheet of the new look has a `transform` row, the request also
//! asks for a transform beat ([`TransformBeatRequested`]) as long as that
//! row, so the body plays its change. A sheet with no such row changes at
//! once.

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::actor::{RecharacterizeBody, WornCharacter};
use ambition_entity_catalog::CharacterId;
use ambition_platformer2d_shared_tangle::lifecycle::{LiveBodies, LiveBodyId};
use ambition_sprite_sheet::character::sheets::AuthoredSheets;
use ambition_sprite_sheet::character::{catalog_join, CharacterAnim};
use bevy::prelude::*;

use crate::features::transform_beat::{TransformBeatPolicy, TransformBeatRequested};

/// A conversation asked one body to wear a catalog character, by live
/// identity.
///
/// Routed by identity for the reason [`crate::features::ChallengeRequested`]
/// is: the narrative ledger releases it again on every replay of its tick, and
/// an `Entity` is not stable across that.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct WearRequested {
    /// The body that changes: the one being talked to.
    pub body: LiveBodyId,
    /// The catalog row it wears from now on.
    pub character: CharacterId,
}

/// Wear the character a conversation asked for. (sim)
///
/// A request that names no live body, a body that wears nothing, or a
/// character the catalog does not have is refused with a log and changes
/// nothing. A body that already wears the character is left as it is.
///
/// The body is untouchable while it plays the `transform` row of its new
/// look ([`arrival_beat`]).
pub fn apply_wear_requests(
    mut commands: Commands,
    mut requests: MessageReader<WearRequested>,
    ids: LiveBodies,
    catalog: Res<CharacterCatalog>,
    authored: Option<Res<AuthoredSheets>>,
    mut worn: Query<&mut WornCharacter>,
) {
    for request in requests.read() {
        let Some(entity) = ids.entity_of(&request.body) else {
            warn!(
                target: "crate::features::wear",
                "a transform of {} names no live body; ignoring",
                request.body,
            );
            continue;
        };
        if catalog.get(request.character.as_str()).is_none() {
            warn!(
                target: "crate::features::wear",
                "the catalog has no character `{}`; ignoring the transform of {}",
                request.character.as_str(),
                request.body,
            );
            continue;
        }
        let Ok(mut worn) = worn.get_mut(entity) else {
            warn!(
                target: "crate::features::wear",
                "{} wears no character; ignoring the transform",
                request.body,
            );
            continue;
        };
        if worn.character() == &request.character {
            continue;
        }
        *worn = WornCharacter(request.character.clone());
        let mut body = commands.entity(entity);
        body.try_insert(RecharacterizeBody);
        if let Some(beat) = arrival_beat(authored.as_deref(), &catalog, request.character.as_str()) {
            body.try_insert(TransformBeatRequested(beat));
        }
    }
}

/// The beat of a body's arrival in the look `character`: the `transform` row
/// of the look's sheet, for as long as the row takes to draw.
///
/// `None` when the sheet has no row of its own for [`CharacterAnim::Transform`]
/// (a fallback row is not a change to play), or when no sheet is known for the
/// character.
fn arrival_beat(
    authored: Option<&AuthoredSheets>,
    catalog: &CharacterCatalog,
    character: &str,
) -> Option<TransformBeatPolicy> {
    let spec = catalog_join::sheet_for_character_id_from_data(authored?, catalog.data(), character)?;
    spec.row_for_anim(CharacterAnim::Transform)?;
    Some(TransformBeatPolicy {
        duration: spec.clip_seconds(CharacterAnim::Transform),
        anim: CharacterAnim::Transform,
        clock_scale: 1.0,
        untouchable: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::actor::character_catalog::CharacterCatalogData;
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    /// A character with one other look, and a character that is no look of it.
    fn catalog() -> CharacterCatalog {
        let row = |name: &str, form_of: &str| {
            format!(
                r#""{name}": (
                    display_name: "{name}",
                    manifest: "sprites/{name}.ron",
                    tier: MainHall,
                    body_kind: Standard,
                    composition: None,
                    default_brain: "idle",
                    default_action_set: "peaceful",
                    {form_of}
                ),"#
            )
        };
        let data: CharacterCatalogData = ron::from_str(&format!(
            r#"(
                brain_presets: {{ "idle": StandStill }},
                action_set_presets: {{ "peaceful": (move_style: Walk) }},
                characters: {{ {} {} }},
            )"#,
            row("scholar", ""),
            row("scholar_armored", r#"form_of: Some("scholar"),"#),
        ))
        .expect("the fixture catalog parses");
        CharacterCatalog::from_data(data)
    }

    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(catalog());
        app.add_message::<WearRequested>();
        app.add_systems(Update, apply_wear_requests);
        app
    }

    fn ask(app: &mut App, body: &str, character: &str) {
        app.world_mut().write_message(WearRequested {
            body: LiveBodyId::new(SimId::placement(body), None),
            character: CharacterId::new(character),
        });
    }

    /// A conversation's transform puts the other look on the body and asks
    /// for the template again; a second request for the same look asks for
    /// nothing; a look the catalog does not have changes nothing.
    #[test]
    fn a_wear_request_changes_the_worn_character_and_asks_for_its_template() {
        let mut app = app();
        let body = app
            .world_mut()
            .spawn((SimId::placement("exhibit"), WornCharacter::new("scholar")))
            .id();
        let worn = |app: &App| app.world().get::<WornCharacter>(body).unwrap().id().to_string();
        let asked = |app: &App| app.world().get::<RecharacterizeBody>(body).is_some();

        // Control: with no request the body keeps its look.
        app.update();
        assert_eq!(worn(&app), "scholar");
        assert!(!asked(&app));

        ask(&mut app, "exhibit", "scholar_armored");
        app.update();
        assert_eq!(worn(&app), "scholar_armored");
        assert!(asked(&app), "the template of the new look was not asked for");

        // The same look again: nothing to apply.
        app.world_mut().entity_mut(body).remove::<RecharacterizeBody>();
        ask(&mut app, "exhibit", "scholar_armored");
        app.update();
        assert!(!asked(&app), "a body that already wears the look was re-derived");

        // A look the catalog does not have.
        ask(&mut app, "exhibit", "scholar_in_a_hat");
        app.update();
        assert_eq!(worn(&app), "scholar_armored");
        assert!(!asked(&app));

        // And the way back.
        ask(&mut app, "exhibit", "scholar");
        app.update();
        assert_eq!(worn(&app), "scholar");
        assert_eq!(app.world().resource::<CharacterCatalog>().form_of("scholar_armored"), Some("scholar"));
        assert_eq!(
            app.world().resource::<CharacterCatalog>().forms_of("scholar").collect::<Vec<_>>(),
            vec!["scholar_armored"]
        );
    }
}
