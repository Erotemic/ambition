//! Ambition's cast voiceprints: the pack's `data/dialogue_voiceprints.ron`.
//!
//! The file is the cast's one list (schema `dialogue_voiceprints`, owned by
//! the audio capability). Admission builds the catalog the dialogue runtime
//! resolves by, and a content reload restates it (`reload::publish_voice_catalog`).

use bevy::prelude::*;

/// Install the voice catalog the selected pack states. A pack that states
/// none installs none, and every line takes the generic blip, as after a
/// reload that removes the file.
pub(super) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    if let Some(catalog) = ambition_audio::content_schema::lowered_voice_catalog(&pack) {
        app.insert_resource(catalog);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_sfx::voice::DialogueVoiceCatalog;
    use ambition_sfx::SfxId;

    fn catalog() -> DialogueVoiceCatalog {
        let mut app = App::new();
        register(&mut app);
        app.world_mut().remove_resource::<DialogueVoiceCatalog>().unwrap()
    }

    /// The cast speaks with the voices its file states, through an exact key,
    /// an alias and a dialogue id.
    #[test]
    fn flagship_exact_and_alias_names_keep_their_authored_voiceprints() {
        let catalog = catalog();
        assert_eq!(catalog.resolve("Bob", ""), Some(SfxId::from_static("dialogue.blip.bob")));
        assert_eq!(catalog.resolve("Shadow Oni Leader", ""), Some(SfxId::from_static("dialogue.blip.ninja")));
        assert_eq!(catalog.resolve("", "hall_npc_oiler"), Some(SfxId::from_static("dialogue.blip.oiler")));
        assert_eq!(catalog.resolve("Iron Mary", ""), Some(SfxId::from_static("dialogue.blip.pirate")));
    }

    #[test]
    fn registration_is_idempotent_for_repeated_content_plugin_installation() {
        let mut app = App::new();
        register(&mut app);
        register(&mut app);
        assert_eq!(
            app.world().resource::<DialogueVoiceCatalog>().resolve("Alice", ""),
            Some(SfxId::from_static("dialogue.blip.alice")),
        );
    }
}
