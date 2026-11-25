//! Wearing by request: a conversation's `<<transform "id">>` makes the
//! character being talked to wear another catalog character.
//!
//! A character with a transform is one character with more than one look
//! (Jeff Hinter and his armor, Mary-O and her grown forms, Sanic and Super
//! Sanic). Each look is a catalog row with its own sheet, and the body wears
//! one of them at a time ([`CharacterForm`]). A game changes the worn row by
//! its own rules (a power-up, a technique). This request is the same change,
//! asked for by a conversation: the hall of characters uses it to show each
//! look of a character on the one body that stands there.
//!
//! The request changes [`CharacterForm`] and asks for the template to be
//! applied again ([`RecharacterizeBody`]). It does not change what the body
//! is to talk to: its conversation stays the one authored on its placement.
//!
//! When the sheet of the new look has a `transform` row, the request also
//! asks for a transform beat ([`TransformBeatRequested`]) as long as that
//! row, so the body plays its change. A sheet with no such row changes at
//! once.
//!
//! ⭐ ADMITTED AGAINST THE SESSION'S CAST (FORM-REBIND). The look and its
//! sheet are resolved in the cast this session was prepared with
//! ([`SessionCast`](ambition_characters::prepared::SessionCast)), before the
//! form is written or a beat is asked for. A content reload can publish a
//! newer catalog while the session still plays the older cast; a look only
//! the newer one has is refused here, whole, and the beat never reads the
//! newer sheet while the mechanics apply the older form.

use ambition_platformer2d_core::replay_messages::ReplayMessageReader;
use ambition_characters::actor::{RecharacterizeBody, CharacterForm};
use ambition_entity_catalog::CharacterId;
use ambition_platformer2d_shared_tangle::lifecycle::{LiveBodies, LiveBodyId};
use ambition_sprite_sheet::character::sheets::AuthoredSheets;
use ambition_sprite_sheet::character::CharacterAnim;
use bevy::prelude::*;

use crate::body::transform_beat::{TransformBeatPolicy, TransformBeatRequested};

/// A conversation asked one body to wear a catalog character, by live
/// identity.
///
/// Routed by identity for the reason [`crate::body::ChallengeRequested`]
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
/// character the session's prepared cast does not hold is refused with a log
/// and changes nothing: no form is written and no beat is asked for, so the
/// body and its active beats are as they were. A body that already wears the
/// character is left as it is.
///
/// The body is untouchable while it plays the `transform` row of its new
/// look (`arrival_beat`), read from the sheet the admitted cast names.
pub fn apply_wear_requests(sprite_content: bevy::prelude::Res<ambition_sprite_sheet::SpriteContent>,
    mut commands: Commands,
    mut requests: ReplayMessageReader<WearRequested>,
    ids: LiveBodies,
    cast: ambition_characters::prepared::SessionCast,
    authored: Option<Res<AuthoredSheets>>,
    mut worn: Query<&mut CharacterForm>,
) {
    for request in requests.read() {
        let Some(entity) = ids.entity_of(&request.body) else {
            warn!(
                target: "crate::body::wear",
                "a transform of {} names no live body; ignoring",
                request.body,
            );
            continue;
        };
        let Some(admitted) = cast.get().and_then(|cast| cast.get(request.character.as_str())) else {
            warn!(
                target: "crate::body::wear",
                "the session's cast has no character `{}`; refusing the transform of {}",
                request.character.as_str(),
                request.body,
            );
            continue;
        };
        let Ok(mut worn) = worn.get_mut(entity) else {
            warn!(
                target: "crate::body::wear",
                "{} wears no character; ignoring the transform",
                request.body,
            );
            continue;
        };
        if worn.character() == &request.character {
            continue;
        }
        *worn = CharacterForm(request.character.clone());
        let mut body = commands.entity(entity);
        body.try_insert(RecharacterizeBody);
        if let Some(beat) = arrival_beat(&sprite_content, authored.as_deref(), admitted.sheet.as_deref()) {
            body.try_insert(TransformBeatRequested(beat));
        }
    }
}

/// The beat of a body's arrival in a look: the `transform` row of the sheet
/// the admitted cast names for it (`sheet`), for as long as the row takes to
/// draw. The row's timing does not depend on the gameplay tuning a catalog row
/// adds to a sheet (collision scale, sample inset, feet anchor).
///
/// `None` when the sheet has no row of its own for [`CharacterAnim::Transform`]
/// (a fallback row is not a change to play), or when the look names no sheet.
fn arrival_beat(sprite_content: &ambition_sprite_sheet::SpriteContent,
    authored: Option<&AuthoredSheets>,
    sheet: Option<&str>,
) -> Option<TransformBeatPolicy> {
    let spec = ambition_sprite_sheet::character::sheets::try_load_spec_for_target_authored(
        sprite_content,
        authored?,
        sheet?,
        &Default::default(),
    )?;
    spec.row_for_anim(CharacterAnim::Transform)?;
    Some(TransformBeatPolicy {
        duration: spec.clip_seconds(CharacterAnim::Transform),
        anim: CharacterAnim::Transform,
        clock_scale: 1.0,
        untouchable: true,
        clip: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::actor::character_catalog::{CharacterCatalog, CharacterCatalogData};
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

    /// The prepared cast of `names`, as a session would freeze it.
    fn cast_of(app: &mut App, names: &[&str]) {
        for name in names {
            ambition_characters::prepared::stage_authored_character(
                app,
                ambition_characters::actor::definition::CharacterDefinition::new(*name, *name, "fixture"),
                &Default::default(),
            )
            .expect("the fixture character stages");
        }
        ambition_characters::prepared::close_preparation_barrier(app.world_mut());
    }

    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(ambition_sprite_sheet::test_support::shared_sprites());
        app.insert_resource(catalog());
        cast_of(&mut app, &["scholar", "scholar_armored"]);
        app.add_message::<WearRequested>();
        app.add_systems(Update, apply_wear_requests);
        app
    }

    /// FORM-REBIND: a look the published catalog has but the running
    /// session's cast does not is refused whole. The control is the same
    /// request with no session running, which the published cast admits.
    #[test]
    fn a_look_only_a_newer_cast_has_is_refused_before_anything_changes() {
        let frozen = {
            let mut older = App::new();
            cast_of(&mut older, &["scholar"]);
            older.world().resource::<ambition_characters::prepared::PreparedCharacterRegistry>().clone()
        };
        let worn_after = |session: bool| {
            let mut app = app();
            if session {
                app.insert_resource(ambition_characters::prepared::ActiveSessionCast(Some(frozen.clone())));
            }
            let body = app.world_mut().spawn((SimId::placement("exhibit"), CharacterForm::new("scholar"))).id();
            ask(&mut app, "exhibit", "scholar_armored");
            app.update();
            let asked = app.world().get::<RecharacterizeBody>(body).is_some()
                || app.world().get::<TransformBeatRequested>(body).is_some();
            (app.world().get::<CharacterForm>(body).unwrap().id().to_string(), asked)
        };
        assert_eq!(worn_after(false), ("scholar_armored".to_string(), true), "control: the published cast admits the look");
        assert_eq!(
            worn_after(true),
            ("scholar".to_string(), false),
            "a look the session's cast does not hold changed the body or asked for its template or beat"
        );
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
            .spawn((SimId::placement("exhibit"), CharacterForm::new("scholar")))
            .id();
        let worn = |app: &App| app.world().get::<CharacterForm>(body).unwrap().id().to_string();
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
