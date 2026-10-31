//! Beat sounds read the active cast across publication and activation.

use super::ActorDecisionFrames;
use ambition_characters::actor::character_catalog::{CharacterCatalog, CharacterCatalogFragment};
use ambition_characters::actor::WornCharacter;
use ambition_characters::brain::BrainBeat;
use ambition_characters::prepared::{CharacterCatalogAppExt, PreparedCharacterRegistry};
use ambition_characters::session_cast::ActiveSessionCast;
use ambition_platformer2d_core as ae;
use ambition_sfx::{BodyPresentationSource, OwnedSfxMessage, SfxId, SfxMessage};
use bevy::prelude::*;

fn prepared_voice(prefix: &str) -> (CharacterCatalog, PreparedCharacterRegistry) {
    let text = format!(r#"(
        brain_presets: {{ "idle": StandStill }},
        action_set_presets: {{ "peaceful": (move_style: Walk) }},
        characters: {{ "alpha": (
            display_name: "Alpha", manifest: "alpha_spritesheet.ron",
            tier: MainHall, body_kind: Standard, composition: None,
            default_brain: "idle", default_action_set: "peaceful", tags: [],
            cues: (notice: Some("{prefix}.notice"), tell: Some("{prefix}.tell")),
        ) }},
    )"#);
    let mut app = App::new();
    app.register_character_catalog_fragment(
        CharacterCatalogFragment::from_ron("fixture", Some("alpha"), &text).unwrap(),
    );
    app.finish();
    (app.world().resource::<CharacterCatalog>().clone(),
     app.world().resource::<PreparedCharacterRegistry>().clone())
}

fn heard(app: &mut App) -> Vec<SfxId> {
    app.update();
    app.world_mut().resource_mut::<Messages<OwnedSfxMessage>>().drain().map(|message| {
        assert_eq!(message.source, "fixture-voice".into());
        match message.request {
            SfxMessage::Play { id, pos } => {
                assert_eq!(pos, ae::Vec2::new(12.0, 34.0));
                id
            }
            other => panic!("expected a beat sound, got {other:?}"),
        }
    }).collect()
}

#[test]
fn beat_sounds_change_at_cast_activation_and_keep_the_bodys_source() {
    let (_, old) = prepared_voice("old");
    let (new_catalog, new) = prepared_voice("new");
    let mut app = App::new();
    app.add_message::<OwnedSfxMessage>();
    app.insert_resource(ActiveSessionCast(Some(old)));
    app.add_systems(Update, crate::body::beat_cues::play_beat_cues);
    let body = app.world_mut().spawn((
        WornCharacter::new("alpha"),
        ae::BodyKinematics { pos: ae::Vec2::new(12.0, 34.0), ..Default::default() },
        BodyPresentationSource("fixture-voice".into()),
    )).id();
    app.insert_resource(ActorDecisionFrames {
        beats: vec![(body, BrainBeat::Notice), (body, BrainBeat::Tell)],
        ..Default::default()
    });
    let old_cues = vec![SfxId::new("old.notice"), SfxId::new("old.tell")];
    let new_cues = vec![SfxId::new("new.notice"), SfxId::new("new.tell")];
    assert_eq!(heard(&mut app), old_cues, "a frozen cast needs no mutable catalog");

    app.insert_resource(new_catalog);
    app.insert_resource(new.clone());
    assert_eq!(heard(&mut app), old_cues, "publication must not change live sounds");
    app.insert_resource(ActiveSessionCast(Some(new)));
    assert_eq!(heard(&mut app), new_cues);
    app.insert_resource(ActiveSessionCast(None));
    assert!(heard(&mut app).is_empty(), "a session with no cast cannot use the published one");
    app.world_mut().remove_resource::<ActiveSessionCast>();
    assert_eq!(heard(&mut app), new_cues, "outside a session the published cast applies");
}
