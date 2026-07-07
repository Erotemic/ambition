//! Which character the local player STARTS as.
//!
//! The player entity is a *control box*: it carries `Brain::Player(slot)`, the
//! home-body integration loop, the player markers, and the full traversal
//! ability kit. WHICH character that box *wears* — its sprite, its combat
//! moveset, and its name — is chosen by the [`StartingCharacter`] resource.
//! With no override the resource is EMPTY and resolves (at spawn) to the
//! CONTENT-installed default character (C2) — the engine names no specific
//! character — so an untouched build spawns exactly as it did before.
//!
//! This is the runtime seam behind Jon's polish-list ask: *"swap my starting
//! character for PCA or a pirate ... just spawn the character and make its
//! brain the keyboard input."* Possession
//! ([`crate::abilities::traversal::possession`]) already proves
//! `Brain::Player` drives ANY body; this makes the *starting* body a choice
//! too, without unifying the (deferred) player-vs-actor integration paths.
//!
//! Reads happen at session setup in two halves — the simulation side
//! ([`crate::session::setup`]) overlays the moveset + name; the presentation
//! side (`ambition_app::app::scene_setup`) binds the sprite sheet.

use bevy::ecs::resource::Resource;
use bevy::ecs::system::Commands;
use bevy::prelude::Entity;

use crate::features::{MomentumMotion, MotionModel};

/// The catalog `character_id` the local player spawns as.
///
/// Read at session setup by both the simulation (moveset + name) and
/// presentation (sprite) halves. An EMPTY `character_id` means "no override —
/// wear the content-installed default" ([`crate::character_roster::default_character_id`]);
/// [`Default`] is exactly that. The engine names no specific character (C2):
/// which row is the default is CONTENT's choice, resolved lazily at spawn.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct StartingCharacter {
    /// A `character_catalog.ron` row id, or EMPTY for the content default.
    /// Ids without a renderable sheet still spawn a controllable player (the
    /// sprite falls back to the colored rectangle) — the sim side never depends
    /// on presentation.
    pub character_id: String,
}

impl StartingCharacter {
    pub fn new(character_id: impl Into<String>) -> Self {
        Self {
            character_id: character_id.into(),
        }
    }

    /// True when the player spawns as the canonical protagonist (no override) —
    /// an empty id routes through the untouched `from_scratch` bundle.
    pub fn is_default(&self) -> bool {
        self.character_id.is_empty()
    }

    /// The concrete catalog id to wear: the explicit override, or the
    /// content-installed default when unset. Resolve at spawn time, never at
    /// resource init (the content default installs at the catalog choke point).
    pub fn effective_id(&self) -> &str {
        if self.character_id.is_empty() {
            crate::character_roster::default_character_id()
        } else {
            &self.character_id
        }
    }
}

// The curated PLAYABLE cast (which catalog ids the character-select surface
// cycles) is CONTENT — it lives in `ambition_content::character_catalog`
// (`PLAYABLE_ROSTER` / `next_playable`), beside the catalog data it indexes
// (R3.2, residue #10). This module keeps only the engine machinery: the
// StartingCharacter resource + the moveset overlay.

// NOTE (2026-07-05): the old `overlay_character_moveset` fallback — empty worn
// slots kept the player's swipe/bolt/shield — is GONE. Wearing is possession
// semantics: the worn character's authored ActionSet IS the kit (Jon's Sanic
// report: a peaceful speedster must not secretly shoot the robot's fireballs).
// The protagonist keeps its code-side kit via the `from_scratch` path.

/// Apply the worn character's MOVEMENT IDENTITY to an already-spawned body
/// (Q16 §S2): if the character authors surface-momentum params, insert
/// `MotionModel::SurfaceMomentum`; otherwise **REMOVE** any `MotionModel` the
/// body carried so it falls back to the axis-swept path.
///
/// The explicit removal is the point: wearing is a re-parametrisation of ONE
/// box (`Brain::Player` never moves), so a re-wear must not leave a stale
/// momentum model riding a chain the new character can't — the render-refresh
/// clobber gotcha in reverse. `momentum_params_for_character_id` is the single
/// source of truth, so the player-wear seam and the actor spawn path can never
/// disagree on which characters ride surfaces.
pub fn apply_worn_motion_model(commands: &mut Commands, entity: Entity, character_id: &str) {
    match crate::character_roster::momentum_params_for_character_id(character_id) {
        Some(params) => {
            commands
                .entity(entity)
                .insert(MotionModel::SurfaceMomentum(MomentumMotion::new(params)));
        }
        None => {
            commands.entity(entity).remove::<MotionModel>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_unset_and_is_default() {
        // No override: an empty id routes to the untouched `from_scratch` path.
        // The concrete row is CONTENT's (`effective_id` resolves it at spawn);
        // the engine bakes in no character name.
        let sc = StartingCharacter::default();
        assert!(sc.character_id.is_empty());
        assert!(sc.is_default());
        // `effective_id` resolves to a real catalog row (the content-installed
        // default, or the first row as fallback) — never empty, never a name
        // the ENGINE baked in.
        let eff = sc.effective_id();
        assert!(!eff.is_empty());
        assert!(crate::character_roster::catalog()
            .characters
            .contains_key(eff));
    }

    #[test]
    fn wearing_sanic_inserts_momentum_then_unwearing_removes_it() {
        // Q16 test (c): wearing a momentum character makes the box ride
        // surfaces; re-wearing a non-momentum character REMOVES the model so a
        // stale MotionModel never rides a chain the new character can't (the
        // render-refresh clobber gotcha in reverse). Removal restores the
        // axis-swept path byte-for-byte — the absence of the component IS the
        // default.
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let entity = app.world_mut().spawn_empty().id();

        // Wear Sanic → SurfaceMomentum inserted with the authored fast profile.
        let mut queue = bevy::ecs::world::CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, app.world());
            apply_worn_motion_model(&mut commands, entity, "sanic");
        }
        queue.apply(app.world_mut());
        match app.world().get::<MotionModel>(entity) {
            Some(MotionModel::SurfaceMomentum(m)) => {
                assert_eq!(m.params.top_speed, 1200.0, "Sanic's authored top speed");
            }
            other => panic!("expected SurfaceMomentum after wearing Sanic, got {other:?}"),
        }

        // Re-wear the protagonist (axis-swept) → the model is removed entirely.
        let mut queue = bevy::ecs::world::CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, app.world());
            apply_worn_motion_model(&mut commands, entity, "player");
        }
        queue.apply(app.world_mut());
        assert!(
            app.world().get::<MotionModel>(entity).is_none(),
            "unwearing a momentum character restores the axis-swept path (no MotionModel)"
        );
    }

    #[test]
    fn non_default_id_is_not_default() {
        assert!(!StartingCharacter::new("goblin").is_default());
    }
}
