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
// A protagonist whose kit is a runtime `AbilitySet` concern opts its ROW into
// `PlayableKitSource::HostCode` (the kit is rebuilt from the body's persisted
// `AbilitySet`); the DEFAULT is that the row's authored kit wins — being the
// content default no longer implies "keep the host's hardcoded kit" (2026-07-11).

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
/// runtime re-wear system ([`apply_worn_character_gameplay`]) apply, so spawn and
/// runtime can never disagree on what a character is. Every field it writes is
/// resolved from the identity plus the body's own persisted `AbilitySet` — NEVER
/// from the component's prior value — so wearing an id is TOTAL and deterministic
/// (a re-wear, a snapshot restore onto a survivor: same id → same result). Applied
/// in place:
///
/// * the display [`Name`]: a known row's `display_name`, else the id itself (so an
///   unknown id is a visible diagnostic, never a stale prior name);
/// * the [`ActionSet`] + the melee [`ActorMoveset`] derived from it, from ONE of
///   three deterministic sources:
///   - an [`Authored`](ambition_characters::actor::character_catalog::PlayableKitSource::Authored)
///     row (the general case) → its catalog `default_action_set` IS the kit;
///   - a [`HostCode`](ambition_characters::actor::character_catalog::PlayableKitSource::HostCode)
///     row (a protagonist whose combat is a runtime `AbilitySet` concern) →
///     rebuild the code kit from `base_abilities`;
///   - an UNKNOWN id → the same code kit (a defined fallback) plus a warning.
///
/// `base_abilities` is the body's capability set (its `BodyAbilities` at runtime,
/// the spawn `AbilitySet` at spawn) — the persisted source that makes the code
/// kit reconstructible, so being the content default no longer means "keep the
/// host's hardcoded kit" and there is no re-wear-to-default gap.
pub fn apply_worn_character_overlay(
    name: &mut Name,
    action_set: &mut ActionSet,
    moveset: &mut ActorMoveset,
    character_id: &str,
    base_abilities: ambition_engine_core::AbilitySet,
) {
    // NAME. A known row supplies a display name; an unknown id becomes its own
    // label — deterministic and never stale, and a legible diagnostic that a body
    // is wearing an id the catalog does not know.
    match crate::character_roster::display_name_for_character_id(character_id) {
        Some(display) => *name = Name::new(display),
        None => *name = Name::new(character_id.to_string()),
    }

    // KIT. Three deterministic sources, none of them the component's prior value:
    let set = if crate::character_roster::playable_kit_is_host_code(character_id) {
        // A protagonist whose combat is a runtime `AbilitySet` concern: rebuild
        // its code kit from the body's own capabilities (a `HostCode` row's
        // catalog action set describes its Hall/NPC face, not its playable kit).
        crate::avatar::bundles::default_player_action_set(base_abilities)
    } else if let Some(character_set) =
        crate::character_roster::default_action_set_for_character_id(character_id)
    {
        // The general case: the worn character's authored ActionSet IS the kit.
        character_set
    } else {
        // An unknown id (or a row whose preset is missing): fall back to the code
        // kit — a DEFINED profile, never arbitrary prior state — and say so.
        bevy::log::warn_once!(
            "worn character id '{character_id}' has no catalog action set; wearing \
             the code-side default kit and showing the id as the display name"
        );
        crate::avatar::bundles::default_player_action_set(base_abilities)
    };
    *moveset = ActorMoveset(build_actor_moveset(None, set.melee.as_ref(), None).unwrap_or_default());
    *action_set = set;
}

/// **Derive a body's gameplay from its worn identity, at spawn and on re-wear.**
///
/// Runs whenever a player's [`WornCharacter`] is added or changes (Bevy's `Changed`
/// filter covers both). Applies [`apply_worn_character_overlay`] (name + kit — the
/// SAME overlay the spawn bundle uses, fed the body's persisted [`BodyAbilities`]
/// so a `HostCode`/unknown re-wear rebuilds the code kit deterministically) then
/// the movement identity via [`apply_worn_motion_model`].
pub fn apply_worn_character_gameplay(
    mut commands: Commands,
    mut worn: Query<
        (
            Entity,
            &WornCharacter,
            &mut Name,
            &mut ActionSet,
            &mut ActorMoveset,
            &crate::actor::BodyAbilities,
        ),
        Changed<WornCharacter>,
    >,
) {
    for (entity, character, mut name, mut action_set, mut moveset, abilities) in &mut worn {
        let id = character.id();
        apply_worn_character_overlay(
            &mut name,
            &mut action_set,
            &mut moveset,
            id,
            abilities.abilities,
        );
        // Movement identity: insert SurfaceMomentum for a momentum character,
        // else REMOVE any stale model so a re-wear never rides a chain the new
        // character can't (the render-refresh clobber gotcha in reverse).
        apply_worn_motion_model(&mut commands, entity, id);
    }
}

#[cfg(test)]
mod tests;
