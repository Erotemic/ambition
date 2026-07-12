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

use bevy::ecs::change_detection::{DetectChanges, Ref};
use bevy::ecs::resource::Resource;
use bevy::ecs::system::{Commands, Query};
use bevy::prelude::{Changed, Entity, Has, Name, Or, With};

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

/// Resolve a playable ActionSet without collapsing an invalid authored row into
/// the privileged host-code fallback. The returned bool says whether the body
/// owns the host's chargeable-projectile capability.
fn resolve_playable_action_set(
    source: Option<ambition_characters::actor::character_catalog::PlayableKitSource>,
    authored: Option<ActionSet>,
    base_abilities: ambition_engine_core::AbilitySet,
) -> (ActionSet, bool) {
    use ambition_characters::actor::character_catalog::PlayableKitSource;

    match source {
        Some(PlayableKitSource::HostCode) => (
            crate::avatar::bundles::default_player_action_set(base_abilities),
            true,
        ),
        Some(PlayableKitSource::Authored) => {
            // A known authored row with a missing preset is malformed content.
            // The startup validator reports it; runtime remains fail-safe and
            // peaceful rather than silently granting the host protagonist kit.
            (authored.unwrap_or_else(ActionSet::peaceful), false)
        }
        None => (
            // Unknown ids use one explicit compatibility fallback. This is
            // intentionally distinct from a known-but-invalid Authored row.
            crate::avatar::bundles::default_player_action_set(base_abilities),
            true,
        ),
    }
}

/// **The gameplay overlay a body derives from wearing `character_id`.**
///
/// This is the single resolver used by both spawn and runtime re-wear. Every
/// field it writes is a deterministic function of the identity plus the body's
/// persisted `AbilitySet`, never of the prior ActionSet or moveset:
///
/// - known `Authored` row: use its resolved `default_action_set`; a malformed
///   missing preset receives a safe peaceful kit rather than host privileges;
/// - known `HostCode` row: rebuild the host kit from `base_abilities`;
/// - unknown id: install the explicit host-code compatibility fallback and name
///   the body after the id so the problem is visible.
///
/// Returns whether the resolved persona owns the host chargeable-projectile
/// capability; the ECS derive system synchronizes its marker and mutable state.
pub fn apply_worn_character_overlay(
    name: &mut Name,
    action_set: &mut ActionSet,
    moveset: &mut ActorMoveset,
    character_id: &str,
    base_abilities: ambition_engine_core::AbilitySet,
) -> bool {
    // NAME. A known row supplies a display name; an unknown id becomes its own
    // label — deterministic and never stale, and a legible diagnostic that a body
    // is wearing an id the catalog does not know.
    match crate::character_roster::display_name_for_character_id(character_id) {
        Some(display) => *name = Name::new(display),
        None => *name = Name::new(character_id.to_string()),
    }

    apply_worn_character_kit(action_set, moveset, character_id, base_abilities)
}

/// Refresh only the action/moveset portion of a playable persona.
///
/// Identity changes call this through [`apply_worn_character_overlay`]. A live
/// `BodyAbilities` edit calls it directly only for `HostCode` and unknown
/// compatibility identities, whose kits actually depend on those abilities.
/// Authored personas deliberately ignore that edge so an inspector edit cannot
/// reset their name, authored kit, or persistent movement state.
fn apply_worn_character_kit(
    action_set: &mut ActionSet,
    moveset: &mut ActorMoveset,
    character_id: &str,
    base_abilities: ambition_engine_core::AbilitySet,
) -> bool {
    let source = crate::character_roster::playable_kit_source_for_character_id(character_id);
    let authored = crate::character_roster::default_action_set_for_character_id(character_id);
    if matches!(
        source,
        Some(ambition_characters::actor::character_catalog::PlayableKitSource::Authored)
    ) && authored.is_none()
    {
        bevy::log::error!(
            "worn character '{character_id}' declares an Authored playable kit but its \
             default_action_set does not resolve; installing a safe peaceful kit"
        );
    } else if source.is_none() {
        bevy::log::warn_once!(
            "worn character id '{character_id}' is not in the catalog; wearing the \
             code-side compatibility kit and showing the id as the display name"
        );
    }

    let (set, charges_projectiles) = resolve_playable_action_set(source, authored, base_abilities);
    *moveset =
        ActorMoveset(build_actor_moveset(None, set.melee.as_ref(), None).unwrap_or_default());
    *action_set = set;
    charges_projectiles
}

fn sync_charge_projectile_capability(
    commands: &mut Commands,
    entity: Entity,
    charges_projectiles: bool,
    has_projectile_state: bool,
) {
    let mut entity_commands = commands.entity(entity);
    if charges_projectiles {
        entity_commands.insert(ambition_characters::brain::ChargesProjectiles);
        if !has_projectile_state {
            entity_commands.insert(ambition_projectiles::PlayerProjectileState::default());
        }
    } else {
        entity_commands.remove::<ambition_characters::brain::ChargesProjectiles>();
        entity_commands.remove::<ambition_projectiles::PlayerProjectileState>();
    }
}

/// **Derive a body's gameplay from its worn identity and host ability source.**
///
/// An identity change refreshes the complete persona: display name, effective
/// kit, projectile capability, and movement identity. An ability-only change is
/// narrower: only a `HostCode` or unknown compatibility kit depends on
/// `BodyAbilities`, so only that kit and its projectile capability are rebuilt.
/// In particular, an authored Sanic keeps the persistent `MomentumMotion.state`
/// it accumulated while riding a surface.
pub fn apply_worn_character_gameplay(
    mut commands: Commands,
    mut worn: Query<
        (
            Entity,
            Ref<WornCharacter>,
            &mut Name,
            &mut ActionSet,
            &mut ActorMoveset,
            Ref<crate::actor::BodyAbilities>,
            Has<ambition_projectiles::PlayerProjectileState>,
        ),
        Or<(Changed<WornCharacter>, Changed<crate::actor::BodyAbilities>)>,
    >,
) {
    use ambition_characters::actor::character_catalog::PlayableKitSource;

    for (
        entity,
        character,
        mut name,
        mut action_set,
        mut moveset,
        abilities,
        has_projectile_state,
    ) in &mut worn
    {
        let id = character.id();
        if character.is_changed() {
            let charges_projectiles = apply_worn_character_overlay(
                &mut name,
                &mut action_set,
                &mut moveset,
                id,
                abilities.abilities,
            );
            sync_charge_projectile_capability(
                &mut commands,
                entity,
                charges_projectiles,
                has_projectile_state,
            );

            // Movement identity is identity-derived, not ability-derived. Only
            // a wear/re-wear may replace the model; doing this for a live
            // ability edit would reset SurfaceMomentum's persistent riding
            // state to Airborne.
            apply_worn_motion_model(&mut commands, entity, id);
            continue;
        }

        if abilities.is_changed() {
            let source = crate::character_roster::playable_kit_source_for_character_id(id);
            if matches!(source, Some(PlayableKitSource::HostCode)) || source.is_none() {
                let charges_projectiles = apply_worn_character_kit(
                    &mut action_set,
                    &mut moveset,
                    id,
                    abilities.abilities,
                );
                sync_charge_projectile_capability(
                    &mut commands,
                    entity,
                    charges_projectiles,
                    has_projectile_state,
                );
            }
        }
    }
}

/// Gate the raw player-control frame by the effective worn kit before any body
/// or effects system consumes it.
///
/// `ActionSet` already gates the generic message resolver, but several legacy
/// player-body paths still read `ActorControl` directly: the movement engine's
/// attack recoil/slash limb, bubble shield, and the chargeable projectile input.
/// Clearing those verbs here makes a peaceful authored persona peaceful in
/// behavior, not merely in its nominal `ActionSet`.
pub fn gate_worn_player_control(
    mut players: Query<
        (
            &WornCharacter,
            &ActionSet,
            &mut ambition_characters::brain::ActorControl,
            Has<ambition_characters::brain::ChargesProjectiles>,
        ),
        With<crate::actor::PlayerEntity>,
    >,
) {
    use ambition_characters::actor::character_catalog::PlayableKitSource;
    use ambition_characters::brain::SpecialActionSpec;

    for (worn, actions, mut control, has_charge_marker) in &mut players {
        if actions.melee.is_none() {
            control.0.melee_pressed = false;
            control.0.pogo_pressed = false;
            control.0.attack_axis = ambition_engine_core::Vec2::ZERO;
        }
        if actions.ranged.is_none() {
            control.0.fire = None;
        }

        let allows_body_shield = matches!(
            actions.special.as_ref(),
            Some(SpecialActionSpec::Special(key)) if key == "bubble_shield"
        );
        if !allows_body_shield {
            control.0.shield_held = false;
        }

        // Use the row declaration as the same-tick source of truth. The marker is
        // synchronized by `apply_worn_character_gameplay`, but Commands are
        // deferred; consulting the identity prevents a one-tick projectile leak
        // on an Authored re-wear before that removal is applied.
        let source = crate::character_roster::playable_kit_source_for_character_id(worn.id());
        let allows_charge_projectiles =
            source == Some(PlayableKitSource::HostCode) || source.is_none();
        if !allows_charge_projectiles || !has_charge_marker {
            control.0.projectile_pressed = false;
            control.0.projectile_held = false;
            control.0.projectile_released = false;
        }
    }
}

#[cfg(test)]
mod tests;
