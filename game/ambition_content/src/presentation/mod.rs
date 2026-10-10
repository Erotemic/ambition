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
        fsm_sauce::install(app);
        mockingbird_sky::install(app);
        room_look::install(app);
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
}
