//! **Derivations FROM a character sheet.** The sheet vocabulary itself —
//! `CharacterAnim`, `SheetRecord`, `SpritePosedBody`, the baked registry —
//! belongs to `ambition_sprite_sheet` and is named from there. What lives here
//! is the half that needs gameplay state to answer:
//!
//! - [`anim`] — the one shared `pick_body_anim` priority ladder over a
//!   `BodyAnimView`, and the thin per-body adapters that build it
//!   ([`pick_player_anim`], [`pick_actor_anim`] + [`ActorAnimState`]);
//! - [`posed_body`] — the sheet as the AUTHORITY for an actor's collision box,
//!   sprite quad and quad placement, resolved per pose, plus the sim pass
//!   ([`SpritePosedBodyPlugin`]) that keeps a body equal to the pose it shows;
//! - [`attack_hitbox`] — the manifest attack rect mapped into world space, so
//!   the box you author IS the gameplay damage box.
//!
//! ## Why it is its own crate
//!
//! It was `ambition_platformer2d_actor_monolith::character_sprites::{anim,
//! posed_body, attack_hitbox}` until 2026-08-09, and the four steps that made
//! this move mechanical are recorded in `docs/planning/engine/decomposition.md`.
//! The load-bearing property is the DIRECTION: **the actor crate does not
//! depend on this one.** Every path here resolves into `ambition_sprite_sheet`,
//! `ambition_combat`, `ambition_characters` or `ambition_platformer2d_core` —
//! all of which already sit below the actor crate — and the only thing the
//! actor crate used to give back was one `add_systems` line, which is now
//! [`SpritePosedBodyPlugin`].
//!
//! ⛔ **that one line is the whole design, so do not "simplify" it away.** A
//! carve that leaves the registration behind puts this crate BETWEEN
//! `ambition_combat` and the actor crate: the owner then depends on it, an edit
//! here still rebuilds the actor crate and everything above it (isolation runs
//! one direction only), and the workspace's longest serial compile chain grows
//! by one. Measured both ways before the move — see the D33 row in
//! `docs/planning/queue-72h-2026-08-08.md`.

mod anim;
mod attack_hitbox;
mod posed_body;

pub use anim::{body_state_clip, pick_actor_anim, pick_player_anim, ActorAnimState};
pub use attack_hitbox::authored_attack_volume_resolver;
pub use posed_body::{
    authored_body_pixel_size, posed_body_geometry, sync_sprite_posed_bodies, PosedBodyGeometry,
};

use bevy::prelude::{App, IntoScheduleConfigs, Plugin};

use ambition_platformer2d_shared_tangle::schedule::{SimScheduleExt, WorldPrepSet};

/// **Install [`sync_sprite_posed_bodies`].**
///
/// A body whose SHEET authors its geometry adopts the box for the pose it is
/// showing, BEFORE the movement phase sweeps that box. Engine-owned so every
/// game gets it: the `SpritePosedBody` component is the opt-in, and a content
/// crate that adds the component without a system to honour it would silently
/// get nothing. The content rule that PINS the pose runs after movement (it
/// classifies contacts against resolved positions), so the box trails the pin
/// by one tick — see [`posed_body`] for why that is the right way round.
///
/// ⚠ this used to be two lines inside the actor crate's
/// `WorldPrepSchedulePlugin`. It is a plugin so that the registration lives
/// with the system rather than one crate up — see the module docs for what the
/// alternative costs.
pub struct SpritePosedBodyPlugin;

impl Plugin for SpritePosedBodyPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            sync_sprite_posed_bodies.in_set(WorldPrepSet::BeforeIntegrate),
        );
    }
}
