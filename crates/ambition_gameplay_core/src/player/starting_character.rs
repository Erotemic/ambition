//! Which character the local player STARTS as.
//!
//! The player entity is a *control box*: it carries `Brain::Player(slot)`, the
//! home-body integration loop, the player markers, and the full traversal
//! ability kit. WHICH character that box *wears* — its sprite, its combat
//! moveset, and its name — is chosen by the [`StartingCharacter`] resource.
//! The default is the canonical `player` (the robot protagonist), so an
//! untouched build spawns exactly as it did before this resource existed.
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

use ambition_characters::brain::ActionSet;

use crate::features::{MomentumMotion, MotionModel};

/// The catalog `character_id` the local player spawns as.
///
/// Read at session setup by both the simulation (moveset + name) and
/// presentation (sprite) halves. Defaults to [`Self::DEFAULT_ID`].
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct StartingCharacter {
    /// A `character_catalog.ron` row id. Ids without a renderable sheet still
    /// spawn a controllable player (the sprite falls back to the colored
    /// rectangle) — the sim side never depends on presentation.
    pub character_id: String,
}

impl StartingCharacter {
    /// The canonical protagonist id — the robot player. Selecting this is a
    /// no-op relative to the pre-feature spawn: [`is_default`](Self::is_default)
    /// routes it through the untouched `from_scratch` bundle.
    pub const DEFAULT_ID: &'static str = "player";

    pub fn new(character_id: impl Into<String>) -> Self {
        Self {
            character_id: character_id.into(),
        }
    }

    /// True when the player spawns as the canonical protagonist (no override).
    pub fn is_default(&self) -> bool {
        self.character_id == Self::DEFAULT_ID
    }
}

impl Default for StartingCharacter {
    fn default() -> Self {
        Self::new(Self::DEFAULT_ID)
    }
}

// The curated PLAYABLE cast (which catalog ids the character-select surface
// cycles) is CONTENT — it lives in `ambition_content::character_catalog`
// (`PLAYABLE_ROSTER` / `next_playable`), beside the catalog data it indexes
// (R3.2, residue #10). This module keeps only the engine machinery: the
// StartingCharacter resource + the moveset overlay.

/// Overlay a character's authored combat moveset onto the player's default kit.
///
/// The character's DEFINED slots win — a goblin swipes, a pirate fires a pistol
/// — while slots the character leaves empty fall back to the player kit, so a
/// peaceful character stays playable (you can still attack with the player's
/// default swipe / bolt). Locomotion style always comes from the character.
///
/// This is precisely "nothing changes except my abilities": the traversal kit
/// belongs to the player box; the melee / ranged / special read as the *worn*
/// character's whenever that character authored one.
pub fn overlay_character_moveset(player: ActionSet, character: ActionSet) -> ActionSet {
    ActionSet {
        move_style: character.move_style,
        melee: character.melee.or(player.melee),
        ranged: character.ranged.or(player.ranged),
        special: character.special.or(player.special),
    }
}

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
    use ambition_characters::brain::{MeleeActionSpec, MoveStyleSpec, RangedActionSpec, SwipeSpec};

    #[test]
    fn default_is_protagonist_and_is_default() {
        let sc = StartingCharacter::default();
        assert_eq!(sc.character_id, "player");
        assert!(sc.is_default());
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

    #[test]
    fn overlay_keeps_player_slots_when_character_is_peaceful() {
        // Player kit: swipe melee + bolt ranged. Peaceful character: all None,
        // Float locomotion. Overlay keeps the player's offense (still playable)
        // but adopts the character's locomotion.
        let player = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ranged: Some(RangedActionSpec::Bolt {
                speed: 600.0,
                damage: 1,
            }),
            move_style: MoveStyleSpec::Walk,
            special: None,
        };
        let peaceful = ActionSet {
            move_style: MoveStyleSpec::Float,
            ..Default::default()
        };
        let merged = overlay_character_moveset(player.clone(), peaceful);
        assert!(merged.melee.is_some(), "peaceful char keeps player melee");
        assert!(merged.ranged.is_some(), "peaceful char keeps player ranged");
        assert_eq!(
            merged.move_style,
            MoveStyleSpec::Float,
            "locomotion is the char's"
        );
    }

    #[test]
    fn overlay_lets_character_offense_win() {
        // Character authors a Lunge melee; it overrides the player's Swipe.
        let player = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ranged: None,
            move_style: MoveStyleSpec::Walk,
            special: None,
        };
        let character = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
                windup_s: 0.9,
                active_s: 0.1,
                recover_s: 0.5,
                damage: 3,
                reach_px: 50.0,
            })),
            ranged: None,
            move_style: MoveStyleSpec::WalkHeavy,
            special: None,
        };
        let merged = overlay_character_moveset(player, character);
        // The character's melee (damage 3) wins over the player's default.
        match merged.melee {
            Some(MeleeActionSpec::Swipe(spec)) => assert_eq!(spec.damage, 3),
            other => panic!("expected the character's Swipe, got {other:?}"),
        }
        assert_eq!(merged.move_style, MoveStyleSpec::WalkHeavy);
    }
}
