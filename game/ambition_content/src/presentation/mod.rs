//! Content-owned presentation plugins — named Ambition looks layered onto the
//! reusable renderer's PUBLIC seams.
//!
//! The reusable `ambition_render` crate names no Ambition content: it exposes
//! [`ambition_render::rendering::ActorOverlaySet`] (a positioned, session-gated
//! set inside the presentation visual-sync chain) and the generic visual marker
//! components; this module supplies the named passes that decorate specific
//! Ambition actors. Adding another surreal enemy look means editing THIS crate,
//! never the renderer (the engine-for-other-games test).
//!
//! Visible builds only: the app adds [`AmbitionPresentationPlugin`] beside the
//! renderer's presentation plugins. Headless builds never mount it, exactly as
//! they never mount the renderer.

pub mod deep_dream;

use bevy::prelude::{App, Plugin};

/// Installs every named Ambition presentation pass. Add AFTER (or beside)
/// `ambition_render::rendering::PresentationVisualAnimationPlugin` — the
/// systems here live in the renderer's public [`ActorOverlaySet`] seam, which
/// that plugin positions and gates.
///
/// [`ActorOverlaySet`]: ambition_render::rendering::ActorOverlaySet
pub struct AmbitionPresentationPlugin;

impl Plugin for AmbitionPresentationPlugin {
    fn build(&self, app: &mut App) {
        deep_dream::install(app);
    }
}
