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
pub mod dialog;

use bevy::prelude::{App, Plugin};

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
        deep_dream::install(app);
    }
}
