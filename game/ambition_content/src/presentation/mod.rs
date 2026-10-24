//! Content-owned presentation plugins — named Ambition looks layered onto the
//! reusable renderer's PUBLIC seams.
//!
//! The reusable `ambition_render` crate names no Ambition content: it exposes
//! [`ambition_render::rendering::ActorOverlaySet`] (a positioned, session-gated
//! set inside the presentation visual-sync chain) and the generic visual marker
//! components and dialogue-presenter lifecycle seams; this module supplies the
//! named passes that decorate specific Ambition actors and the game's concrete
//! opaque portrait dialogue box. Adding another surreal look or changing product
//! UI means editing THIS crate, never the reusable renderer.
//!
//! Visible builds only: the app adds [`AmbitionPresentationPlugin`] beside the
//! renderer's presentation plugins. Headless builds never mount it, exactly as
//! they never mount the renderer.

pub mod deep_dream;
pub mod fsm_sauce;
pub mod gate_terminal;
pub mod inugami_field;
pub mod mockingbird_sky;
pub mod room_look;
pub mod dialog;
pub mod vanity_card_made_this_meme;

use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, SessionScopedEntity, SessionSpawnScope};
use bevy::prelude::{App, Plugin};

/// The scope an overlay of a visual is spawned under: the session and the live
/// room of the visual it decorates.
///
/// An overlay is a different entity from its visual, so it gets no stamp from
/// it. Without the room stamp the camera of each live room draws the overlay,
/// and room retirement does not remove it with its room.
pub(crate) fn overlay_scope_of(owner: Option<&SessionScopedEntity>, room: Option<&InRoomInstance>) -> SessionSpawnScope {
    SessionSpawnScope::new(owner.map(|owner| owner.0)).in_room(room.map(|room| room.0))
}

/// Installs every named Ambition presentation pass: the one concrete dialogue
/// presenter plus actor overlays. Add AFTER (or beside)
/// `ambition_render::rendering::PresentationVisualAnimationPlugin` — the actor
/// systems live in the renderer's public [`ActorOverlaySet`] seam, while the
/// dialogue presenter claims `DialogPresentationSet` independently.
///
/// [`ActorOverlaySet`]: ambition_render::rendering::ActorOverlaySet
pub struct AmbitionPresentationPlugin;

impl Plugin for AmbitionPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(dialog::AmbitionDialogUiPlugin);
        app.add_plugins(vanity_card_made_this_meme::MadeThisMemeCardPlugin);
        deep_dream::install(app);
        gate_terminal::install(app);
        fsm_sauce::install(app);
        inugami_field::install(app);
        mockingbird_sky::install(app);
        room_look::install(app);
        // Idle barks fire on the pack's cadence (`boss_idle_bark`) while a
        // boss is in an attacking phase, so the scholar feels alive between
        // strikes. Ambition's bosses, so only in Ambition's rooms.
        {
            use ambition_platformer2d::runtime::ScopedSystemsAppExt as _;
            use bevy::prelude::IntoScheduleConfigs as _;
            app.add_scoped_systems(
                bevy::prelude::Update,
                crate::AMBITION_RULES,
                crate::bosses::tick_boss_idle_barks
                    .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, SpawnSessionScopedExt};
    use bevy::prelude::*;

    /// An overlay is in the room of the visual it decorates, and in no room
    /// when that visual is in none.
    #[test]
    fn an_overlay_is_stamped_with_the_room_of_its_visual() {
        let mut world = World::new();
        let room = InRoomInstance(LiveRoomInstance::ACTIVATION);
        let (in_room, in_none) = {
            let mut commands = world.commands();
            (
                commands.spawn_session_scoped(overlay_scope_of(None, Some(&room)), Name::new("overlay")).id(),
                commands.spawn_session_scoped(overlay_scope_of(None, None), Name::new("overlay")).id(),
            )
        };
        world.flush();
        assert_eq!(world.get::<InRoomInstance>(in_room).map(|stamp| stamp.0), Some(room.0));
        assert!(world.get::<InRoomInstance>(in_none).is_none());
    }

    /// Each shader in `src/presentation/shaders` is served as the file is
    /// saved, at the path its material names, so a shader edit costs a
    /// restart and no build of this crate.
    #[test]
    fn each_shader_is_served_as_its_file_is_saved() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<bevy::shader::Shader>();
        deep_dream::install(&mut app);
        fsm_sauce::install(&mut app);
        mockingbird_sky::install(&mut app);
        room_look::install(&mut app);
        let source = app.world().resource::<AssetServer>().get_source("embedded").unwrap().reader();
        let shaders = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/presentation/shaders");
        let mut files: Vec<_> = std::fs::read_dir(&shaders).unwrap().map(|entry| entry.unwrap().path()).collect();
        files.sort();
        assert!(files.len() >= 9, "premise: the shaders are on disk: {files:?}");
        for file in files {
            let name = file.file_name().unwrap().to_str().unwrap();
            let served = std::path::PathBuf::from("ambition_content/presentation/shaders").join(name);
            let bytes = bevy::tasks::block_on(async {
                let mut reader = source.read(&served).await.unwrap_or_else(|error| panic!("{name} is not served: {error}"));
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes).await.unwrap();
                bytes
            });
            assert!(bytes == std::fs::read(&file).unwrap(), "{name} is served, but not as its file is saved");
        }
    }
}
