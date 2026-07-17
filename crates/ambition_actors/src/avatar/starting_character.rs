//! Which character the local player STARTS as.
//!
//! The player entity is a *control box*: it carries `Brain::Player(slot)`, the
//! home-body integration loop, the player markers, and the full traversal
//! ability kit. WHICH character that box *wears* — its sprite, its combat
//! moveset, and its name — is chosen by the session-owned [`StartingCharacter`] component.
//! With no override the component is EMPTY and resolves (at spawn) to the
//! CONTENT-installed default character (C2) — the engine names no specific
//! character — so an untouched build spawns exactly as it did before.
//!
//! This is the runtime seam behind Jon's polish-list ask: *"swap my starting
//! character for PCA or a pirate ... just spawn the character and make its
//! brain the keyboard input."* Possession
//! ([`crate::abilities::traversal::possession`]) already proves
//! `Brain::Player` drives ANY body; this makes the *starting* body a choice
//! too without creating a character-specific movement route. The worn body
//! still enters the same frame-aware movement kernel as every other body.
//!
//! [`StartingCharacter`] is the session-owned startup selection. At spawn
//! ([`crate::session::setup`]) the chosen id is both overlaid onto the body
//! (moveset + name) AND recorded as the canonical [`WornCharacter`] identity
//! component ON the player entity. From then on the entity's component — not
//! this component — is the single source both gameplay and presentation derive
//! from: [`apply_worn_character_gameplay`] re-applies the kit on any change, and
//! the reusable `ambition_render` binder installs the sprite from the same
//! identity. Presentation reads the same session-owned identity rather than process state.

use bevy::ecs::change_detection::{DetectChanges, Ref};
use bevy::ecs::system::{Commands, Query};
use bevy::prelude::{Changed, Component, Entity, Has, Name, Or, Res, With};

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::actor::WornCharacter;
use ambition_characters::brain::ActionSet;

use crate::combat::moveset::{build_actor_moveset, ActorMoveset};
use crate::features::MotionModel;

/// The catalog `character_id` the local player spawns as.
///
/// Read at session setup by both the simulation (moveset + name) and
/// presentation (sprite) halves. An EMPTY `character_id` means "no override —
/// wear the provider-relative default supplied by the session builder.
/// [`Default`] is exactly that. The engine names no specific character (C2):
/// which row is the default is CONTENT's choice, resolved lazily at spawn.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
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
    /// component construction (the content default installs at the catalog choke point).
    pub fn effective_id<'a>(&'a self, default_character_id: &'a str) -> &'a str {
        if self.character_id.is_empty() {
            default_character_id
        } else {
            &self.character_id
        }
    }
}

// The curated PLAYABLE cast (which catalog ids the character-select surface
// cycles) is CONTENT — it lives in `ambition_content::character_catalog`
// (`PLAYABLE_ROSTER` / `next_playable`), beside the catalog data it indexes
// (R3.2, residue #10). This module keeps only the engine machinery: the
// StartingCharacter component + the moveset overlay.

// NOTE (2026-07-05): the old `overlay_character_moveset` fallback — empty worn
// slots kept the player's swipe/bolt/shield — is GONE. Wearing is possession
// semantics: the worn character's authored ActionSet IS the kit (Jon's Sanic
// report: a peaceful speedster must not secretly shoot the robot's fireballs).
// A protagonist whose kit is a runtime `AbilitySet` concern opts its ROW into
// `PlayableKitSource::HostCode` (the kit is rebuilt from the body's persisted
// `AbilitySet`); the DEFAULT is that the row's authored kit wins — being the
// content default no longer implies "keep the host's hardcoded kit" (2026-07-11).

/// Resolve the state-free movement policy authored by a character identity.
///
/// The active experience owns the character catalog. Movement identity must be
/// resolved from that App-local catalog rather than from Ambition's built-in
/// roster, so standalone experiences such as Sanic can author their own policy
/// without process-global registration.
pub fn motion_model_spec_for_character_id(
    catalog: &CharacterCatalog,
    character_id: &str,
) -> ambition_engine_core::MotionModelSpec {
    match catalog.momentum_params(character_id) {
        Some(params) => ambition_engine_core::MotionModelSpec::SurfaceMomentum(params),
        None => ambition_engine_core::MotionModelSpec::AxisSwept(
            // A character that authors its own axis feel seeds the model with it
            // so the FIRST frame is already correct (the live integrator then
            // refreshes from the body's `AuthoredMovementTuning` each tick); an
            // un-authored character starts from the shared default.
            catalog
                .axis_tuning(character_id)
                .map(|tuning| tuning.axis_swept_params())
                .unwrap_or_default(),
        ),
    }
}

/// Apply the worn character's movement identity to an already-spawned body.
///
/// Every movable body already carries one explicit model. This operation only
/// changes that policy; it never removes the component or uses absence as an
/// axis-swept sentinel.
pub fn apply_worn_motion_model(
    catalog: &CharacterCatalog,
    commands: &mut Commands,
    entity: Entity,
    character_id: &str,
) {
    let mut model = MotionModel::default();
    model.apply_spec(motion_model_spec_for_character_id(catalog, character_id));
    commands.entity(entity).insert(model);
}

/// Synchronize movement identity without discarding live solver state when the
/// selected policy is unchanged. A same-model refresh updates only parameters;
/// a cross-model transition preserves every shared body fact and initializes
/// ONLY destination-private state — through the one kernel transition seam.
fn sync_worn_motion_model_preserving_state(
    catalog: &CharacterCatalog,
    character_id: &str,
    current: &mut MotionModel,
) {
    ambition_engine_core::switch_motion_model(
        current,
        motion_model_spec_for_character_id(catalog, character_id),
    );
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
    catalog: &CharacterCatalog,
    name: &mut Name,
    action_set: &mut ActionSet,
    moveset: &mut ActorMoveset,
    character_id: &str,
    base_abilities: ambition_engine_core::AbilitySet,
) -> bool {
    // NAME. A known row supplies a display name; an unknown id becomes its own
    // label — deterministic and never stale, and a legible diagnostic that a body
    // is wearing an id the catalog does not know.
    match catalog.display_name(character_id) {
        Some(display) => *name = Name::new(display.to_string()),
        None => *name = Name::new(character_id.to_string()),
    }

    apply_worn_character_kit(catalog, action_set, moveset, character_id, base_abilities)
}

/// Refresh only the action/moveset portion of a playable persona.
///
/// Identity changes call this through [`apply_worn_character_overlay`]. A live
/// `BodyAbilities` edit calls it directly only for `HostCode` and unknown
/// compatibility identities, whose kits actually depend on those abilities.
/// Authored personas deliberately ignore that edge so an inspector edit cannot
/// reset their name, authored kit, or persistent movement state.
fn apply_worn_character_kit(
    catalog: &CharacterCatalog,
    action_set: &mut ActionSet,
    moveset: &mut ActorMoveset,
    character_id: &str,
    base_abilities: ambition_engine_core::AbilitySet,
) -> bool {
    let source = catalog.playable_kit_source(character_id);
    let authored = catalog.build_default_action_set(character_id);
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
    *moveset = ActorMoveset(
        build_actor_moveset(None, set.melee.as_ref(), None, set.special.as_ref())
            .unwrap_or_default(),
    );
    *action_set = set;
    charges_projectiles
}

fn sync_charge_projectile_capability(
    commands: &mut Commands,
    entity: Entity,
    charges_projectiles: bool,
    has_projectile_state: bool,
) {
    // This kit refresh is deferred, and a session-scoped body can be despawned
    // by session teardown in the same frame its worn identity last changed; the
    // `try_` variants apply the capability iff the entity is still alive rather
    // than erroring on a torn-down entity.
    let mut entity_commands = commands.entity(entity);
    if charges_projectiles {
        entity_commands.try_insert(ambition_characters::brain::ChargesProjectiles);
        if !has_projectile_state {
            entity_commands.try_insert(ambition_projectiles::PlayerProjectileState::default());
        }
    } else {
        entity_commands.try_remove::<ambition_characters::brain::ChargesProjectiles>();
        entity_commands.try_remove::<ambition_projectiles::PlayerProjectileState>();
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
    catalog: Res<CharacterCatalog>,
    mut commands: Commands,
    mut worn: Query<
        (
            Entity,
            Ref<WornCharacter>,
            &mut Name,
            &mut ActionSet,
            &mut ActorMoveset,
            Ref<crate::actor::BodyAbilities>,
            // The one transition seam (`switch_motion_model`): a cross-model
            // re-wear initializes destination-private state inside the new
            // variant value; no cluster is touched (ADR 0024).
            &mut MotionModel,
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
        mut motion_model,
        has_projectile_state,
    ) in &mut worn
    {
        let id = character.id();
        if character.is_changed() {
            let charges_projectiles = apply_worn_character_overlay(
                &catalog,
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
            sync_worn_motion_model_preserving_state(&catalog, id, &mut motion_model);

            // Per-character axis FEEL rides a marker component: presence means
            // "this body's tuning is authored, not the shared F3 dev tuning".
            // Insert it when the worn identity authors a tuning, remove it when
            // it does not — so a re-wear from an authored feel back to the
            // sandbox protagonist returns the body to the live inspector sliders.
            match catalog.axis_tuning(id) {
                Some(tuning) => {
                    commands
                        .entity(entity)
                        .try_insert(ambition_engine_core::AuthoredMovementTuning(tuning));
                }
                None => {
                    commands
                        .entity(entity)
                        .try_remove::<ambition_engine_core::AuthoredMovementTuning>();
                }
            }
            continue;
        }

        if abilities.is_changed() {
            let source = catalog.playable_kit_source(id);
            if matches!(source, Some(PlayableKitSource::HostCode)) || source.is_none() {
                let charges_projectiles = apply_worn_character_kit(
                    &catalog,
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
    catalog: Res<CharacterCatalog>,
    mut players: Query<
        (
            &WornCharacter,
            &ActionSet,
            // The body's live combat/ability authorities — the SAME inputs the
            // control-prompt read-model derives its labels from. The gate resolves
            // them through the shared `derive_action_scheme` so what a slot GATES
            // here and what the prompt SHOWS are one derivation (no UI drift).
            &crate::actor::BodyAbilities,
            Option<&ActorMoveset>,
            Option<&ambition_characters::action_scheme::ActorTechniques>,
            &mut ambition_characters::brain::ActorControl,
            // Sanctioned technique edges: when a slot resolves to `Technique`, the
            // gate routes the slot's device edge here (and clears the raw verb),
            // so a content technique reads THIS instead of intercepting a raw
            // combat press. `Option` — only technique-bearing bodies carry it.
            Option<&mut ambition_characters::action_scheme::ResolvedTechniqueEdges>,
            Has<ambition_characters::brain::ChargesProjectiles>,
            // Holding an item REPURPOSES the attack verb (the pickup stashes the
            // melee kit precisely so item-use fires instead), so the persona
            // gate must not eat melee/shield presses while an item is held —
            // by IDENTITY, not by racing the item systems in schedule order.
            Has<ambition_combat::held_items::HeldItem>,
        ),
        With<crate::actor::PlayerEntity>,
    >,
) {
    use ambition_characters::action_scheme::derive_action_scheme;
    use ambition_characters::actor::character_catalog::PlayableKitSource;
    use ambition_characters::brain::SpecialActionSpec;
    use ambition_engine_core::Edge;
    use ambition_entity_catalog::action_scheme::{ActionGate, ControlSlot};

    for (worn, actions, abilities, moveset, techniques, mut control, mut tech_edges, has_charge_marker, holds_item) in
        &mut players
    {
        // THE shared resolver — byte-identical to the call the ControlPrompt
        // producer makes on the same immediate authorities.
        let scheme = derive_action_scheme(
            &abilities.abilities,
            moveset.map(|m| &m.0),
            Some(actions),
            techniques.map_or(&[], |t| t.0.as_slice()),
        );
        if let Some(edges) = tech_edges.as_deref_mut() {
            edges.clear();
        }

        // Attack slot — resolve its gate:
        //  * `Technique(id)`: route the melee edge to the sanctioned technique
        //    edge and clear the raw verb (a plain melee edge is NO LONGER the
        //    content API — the technique fires only from its keyed edge).
        //  * absent: strip the melee verb (the persona-gate behavior, now keyed
        //    on scheme presence rather than a parallel `ActionSet.melee` check).
        //  * `Move`: keep it.
        match scheme.action_for_slot(ControlSlot::Attack).map(|a| &a.gate) {
            Some(ActionGate::Technique(id)) => {
                if let Some(edges) = tech_edges.as_deref_mut() {
                    edges.set(
                        id,
                        Edge {
                            pressed: control.0.melee_pressed,
                            held: false,
                            released: false,
                        },
                    );
                }
                control.0.melee_pressed = false;
                control.0.pogo_pressed = false;
                control.0.attack_axis = ambition_engine_core::Vec2::ZERO;
            }
            None if !holds_item => {
                control.0.melee_pressed = false;
                control.0.pogo_pressed = false;
                control.0.attack_axis = ambition_engine_core::Vec2::ZERO;
            }
            _ => {}
        }

        // Ranged (Projectile slot) — stripped iff the scheme lacks it.
        if !scheme.has_slot(ControlSlot::Projectile) && !holds_item {
            control.0.fire = None;
        }

        let allows_body_shield = matches!(
            actions.special.as_ref(),
            Some(SpecialActionSpec::Special(key)) if key == "bubble_shield"
        );
        // Shield+Attack is the universal "throw the held item" gesture, so a
        // held item keeps the shield verb alive too.
        if !allows_body_shield && !holds_item {
            control.0.shield_held = false;
        }

        // Use the row declaration as the same-tick source of truth. The marker is
        // synchronized by `apply_worn_character_gameplay`, but Commands are
        // deferred; consulting the identity prevents a one-tick projectile leak
        // on an Authored re-wear before that removal is applied.
        let source = catalog.playable_kit_source(worn.id());
        let allows_charge_projectiles =
            source == Some(PlayableKitSource::HostCode) || source.is_none();
        if !allows_charge_projectiles || !has_charge_marker {
            control.0.projectile_pressed = false;
            control.0.projectile_held = false;
            control.0.projectile_released = false;
        }
    }
}

/// While a body's folded `"bubble_shield"` special MOVE is playing, hold its guard
/// up through the ONE shield path (`shield_held` → `resolve_shield`) — so pressing
/// Special actually deploys the bubble shield instead of playing a bare animation.
///
/// The special move's `id` equals the body's `ActionSet.special` key (that is how
/// [`build_actor_moveset`] folds the marker in), so a `bubble_shield` persona
/// raises [`ambition_engine_core::body_clusters::BodyShieldState`] BY IDENTITY
/// while that move plays — no per-body wiring, and the on-screen Special button
/// (which reads the SAME scheme) cannot advertise a shield the body won't raise.
///
/// Runs in `PlayerInput` after [`gate_worn_player_control`] (which keeps a
/// `bubble_shield` persona's `shield_held` alive) and before the `WorldPrep`
/// kernel bridge, so the guard rises the same tick the kernel resolves it. It
/// forces `shield_held` rather than poking `BodyShieldState` directly, so the
/// kernel's parry-window/dash-gating rules apply uniformly — the special is just
/// another way to raise the ONE shield.
pub fn sustain_bubble_shield(
    mut bodies: Query<(
        &ActionSet,
        &ambition_combat::moveset::MovePlayback,
        &mut ambition_characters::brain::ActorControl,
    )>,
) {
    use ambition_characters::brain::SpecialActionSpec;
    for (actions, playback, mut control) in &mut bodies {
        let Some(SpecialActionSpec::Special(key)) = actions.special.as_ref() else {
            continue;
        };
        if key == "bubble_shield" && playback.spec.id == *key {
            control.0.shield_held = true;
        }
    }
}

#[cfg(test)]
mod tests;
