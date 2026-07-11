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
//! [`StartingCharacter`] is the STARTUP SELECTION resource. At spawn
//! ([`crate::session::setup`]) the chosen id is both overlaid onto the body
//! (moveset + name) AND recorded as the canonical [`WornCharacter`] identity
//! component ON the player entity. From then on the entity's component — not
//! this resource — is the single source both gameplay and presentation derive
//! from: [`apply_worn_character_gameplay`] re-applies the kit on any change, and
//! the reusable `ambition_render` binder installs the sprite from the same
//! identity. Presentation no longer reads this app-local resource.

use bevy::ecs::resource::Resource;
use bevy::ecs::system::{Commands, Query};
use bevy::prelude::{Changed, Entity, Name};

use ambition_characters::actor::WornCharacter;
use ambition_characters::brain::ActionSet;

use crate::combat::moveset::{build_actor_moveset, ActorMoveset};
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

/// **The gameplay OVERLAY a body derives from wearing `character_id`.** The ONE
/// authority both the spawn bundle
/// ([`crate::avatar::PlayerSimulationBundle::from_scratch_as_character`]) and the
/// runtime re-wear system ([`apply_worn_character_gameplay`]) apply, so for a KNOWN
/// character they can never disagree on its name + kit. Applied in place:
///
/// * the display [`Name`] for any id that authors one;
/// * for a KNOWN NON-PROTAGONIST character, its authored [`ActionSet`] + the melee
///   [`ActorMoveset`] derived from it — the worn character's ActionSet IS the kit.
///
/// It deliberately does NOT touch the kit for the PROTAGONIST (content default) or
/// an UNKNOWN id: the protagonist's kit is the code-side `AbilitySet` dev-toggle
/// kit and an unknown id keeps the playable default — both are established by
/// `from_scratch` and cannot be rebuilt from the catalog here. **Consequence
/// (known limitation):** a *runtime* re-wear FROM a known character TO the default
/// or an unknown id leaves the prior character's kit in place (the code-side
/// default kit is not reconstructible without persisting the `AbilitySet`). Fully
/// closing that requires modeling the default kit as data; tracked in the plan.
pub fn apply_worn_character_overlay(
    name: &mut Name,
    action_set: &mut ActionSet,
    moveset: &mut ActorMoveset,
    character_id: &str,
) {
    if let Some(display) = crate::character_roster::display_name_for_character_id(character_id) {
        *name = Name::new(display);
    }
    if character_id == crate::character_roster::default_character_id() {
        return;
    }
    if let Some(character_set) =
        crate::character_roster::default_action_set_for_character_id(character_id)
    {
        *moveset = ActorMoveset(
            build_actor_moveset(None, character_set.melee.as_ref(), None).unwrap_or_default(),
        );
        *action_set = character_set;
    }
}

/// **Derive a body's gameplay from its worn identity, at spawn and on re-wear.**
///
/// Runs whenever a player's [`WornCharacter`] is added or changes (Bevy's `Changed`
/// filter covers both). Applies [`apply_worn_character_overlay`] (name + authored
/// kit — the SAME overlay the spawn bundle uses) then the movement identity via
/// [`apply_worn_motion_model`]. See the overlay's docs for the protagonist/unknown
/// limitation.
pub fn apply_worn_character_gameplay(
    mut commands: Commands,
    mut worn: Query<
        (
            Entity,
            &WornCharacter,
            &mut Name,
            &mut ActionSet,
            &mut ActorMoveset,
        ),
        Changed<WornCharacter>,
    >,
) {
    for (entity, character, mut name, mut action_set, mut moveset) in &mut worn {
        let id = character.id();
        apply_worn_character_overlay(&mut name, &mut action_set, &mut moveset, id);
        // Movement identity: insert SurfaceMomentum for a momentum character,
        // else REMOVE any stale model so a re-wear never rides a chain the new
        // character can't (the render-refresh clobber gotcha in reverse).
        apply_worn_motion_model(&mut commands, entity, id);
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

    /// **S1: gameplay configuration is DERIVED from the worn identity, at spawn
    /// (Added) and on any later re-wear (Changed).** A body carrying only the
    /// `WornCharacter` identity plus the mutable gameplay components has its name
    /// and movement identity re-derived by `apply_worn_character_gameplay`.
    #[test]
    fn gameplay_derives_from_worn_identity_at_add_and_on_change() {
        use crate::combat::moveset::ActorMoveset;
        use ambition_characters::brain::ActionSet;
        use bevy::prelude::*;

        // Pin the installed default so the protagonist branch is deterministic.
        crate::character_roster::install_default_character_id("player");

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, apply_worn_character_gameplay);

        // Spawn wearing the momentum speedster.
        let e = app
            .world_mut()
            .spawn((
                WornCharacter::new("sanic"),
                Name::new("unset"),
                ActionSet::default(),
                ActorMoveset(Default::default()),
            ))
            .id();
        app.update();

        // Movement identity (SurfaceMomentum) + name are derived from "sanic".
        assert!(
            matches!(
                app.world().get::<MotionModel>(e),
                Some(MotionModel::SurfaceMomentum(_))
            ),
            "wearing the momentum character derives SurfaceMomentum"
        );
        assert_eq!(
            app.world().get::<Name>(e).unwrap().as_str(),
            "Sanic",
            "the display name is derived from the worn identity"
        );

        // Re-wear the protagonist through the supported path (mutate the
        // identity). Downstream observes the change: the stale momentum model is
        // removed and the name follows.
        *app.world_mut().get_mut::<WornCharacter>(e).unwrap() = WornCharacter::new("player");
        app.update();
        assert!(
            app.world().get::<MotionModel>(e).is_none(),
            "re-wearing a non-momentum character removes the stale movement model"
        );
        assert_eq!(
            app.world().get::<Name>(e).unwrap().as_str(),
            "Player",
            "the display name follows the new worn identity"
        );
    }

    /// **S1 poison / non-vacuity:** with NO change to `WornCharacter`, the derive
    /// system does not fire, so a hand-set movement model is left untouched. This
    /// proves the assertion above is driven by the `Changed` edge, not by the
    /// system running unconditionally every frame.
    #[test]
    fn derive_system_only_fires_on_identity_change() {
        use crate::combat::moveset::ActorMoveset;
        use ambition_characters::brain::ActionSet;
        use bevy::prelude::*;

        crate::character_roster::install_default_character_id("player");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, apply_worn_character_gameplay);
        let e = app
            .world_mut()
            .spawn((
                WornCharacter::new("sanic"),
                Name::new("unset"),
                ActionSet::default(),
                ActorMoveset(Default::default()),
            ))
            .id();
        app.update(); // Added → derives SurfaceMomentum for sanic.
        assert!(app.world().get::<MotionModel>(e).is_some());

        // No identity change: subsequent frames must not re-run the wear. Prove it
        // by clobbering the model and confirming the un-changed system leaves it.
        app.world_mut().entity_mut(e).remove::<MotionModel>();
        app.update();
        assert!(
            app.world().get::<MotionModel>(e).is_none(),
            "with no WornCharacter change the derive system must not re-fire"
        );
    }

    /// **The full KIT (ActionSet + moveset), not just name/movement, follows a
    /// re-wear between two KNOWN characters** — the reviewer-flagged gap. Wearing
    /// the pirate gives its authored pistol; re-wearing the goblin replaces it with
    /// the goblin's kit, leaving no stale pirate pistol behind.
    #[test]
    fn worn_kit_fully_follows_a_known_character_rewear() {
        use crate::combat::moveset::ActorMoveset;
        use ambition_characters::brain::{ActionSet, RangedActionSpec};
        use bevy::prelude::*;

        crate::character_roster::install_default_character_id("player");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, apply_worn_character_gameplay);
        let e = app
            .world_mut()
            .spawn((
                WornCharacter::new("npc_pirate_admiral"),
                Name::new("unset"),
                ActionSet::default(),
                ActorMoveset(Default::default()),
            ))
            .id();
        app.update();
        assert!(
            matches!(
                app.world().get::<ActionSet>(e).unwrap().ranged,
                Some(RangedActionSpec::Pistol { .. })
            ),
            "wearing the pirate derives its authored pistol into the ActionSet"
        );

        // Re-wear a DIFFERENT known character: the kit fully swaps — no stale pistol.
        *app.world_mut().get_mut::<WornCharacter>(e).unwrap() = WornCharacter::new("goblin");
        app.update();
        assert!(
            !matches!(
                app.world().get::<ActionSet>(e).unwrap().ranged,
                Some(RangedActionSpec::Pistol { .. })
            ),
            "re-wearing the goblin replaces the pirate's kit — no stale ActionSet"
        );
        assert_eq!(app.world().get::<Name>(e).unwrap().as_str(), "Goblin");
    }

    /// **Honest limitation:** a runtime re-wear FROM a known character TO the
    /// content default (protagonist) does NOT rebuild the code-side AbilitySet kit
    /// — it leaves the prior kit, because that kit is not reconstructible from the
    /// catalog. Pinned so the limitation is explicit rather than hidden behind an
    /// "always total" claim. (Fully closing it means modeling the default kit as
    /// data; tracked in the plan.)
    #[test]
    fn runtime_rewear_to_the_default_keeps_the_prior_kit_documented_gap() {
        use crate::combat::moveset::ActorMoveset;
        use ambition_characters::brain::{ActionSet, RangedActionSpec};
        use bevy::prelude::*;

        crate::character_roster::install_default_character_id("player");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, apply_worn_character_gameplay);
        let e = app
            .world_mut()
            .spawn((
                WornCharacter::new("npc_pirate_admiral"),
                Name::new("unset"),
                ActionSet::default(),
                ActorMoveset(Default::default()),
            ))
            .id();
        app.update();

        // Re-wear the default: name follows, but the pistol kit is NOT rebuilt.
        *app.world_mut().get_mut::<WornCharacter>(e).unwrap() = WornCharacter::new("player");
        app.update();
        assert_eq!(app.world().get::<Name>(e).unwrap().as_str(), "Player");
        assert!(
            matches!(
                app.world().get::<ActionSet>(e).unwrap().ranged,
                Some(RangedActionSpec::Pistol { .. })
            ),
            "documented gap: the default's code-side kit is not rebuilt at runtime, \
             so the prior pistol lingers (spawn uses the code kit instead)"
        );
    }
}
