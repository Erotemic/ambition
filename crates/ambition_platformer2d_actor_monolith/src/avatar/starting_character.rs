//! Which character the local player STARTS as.
//!
//! The player entity is a *control box*: it carries `DrivingParticipant(slot)`, the
//! home-body integration loop, the player markers, and the full traversal
//! ability kit. WHICH character that box *wears* — its sprite, its combat
//! moveset, and its name — is chosen by the session-owned [`StartingCharacter`] component.
//! With no override the component is EMPTY and resolves (at spawn) to the
//! CONTENT-installed default character (C2) — the engine names no specific
//! character — so an untouched build spawns exactly as it did before.
//!
//! [`StartingCharacter`] is the session-owned startup selection. At spawn
//! ([`crate::session::setup`]) the chosen id is both overlaid onto the body
//! (moveset + name) AND recorded as the canonical [`WornCharacter`] identity
//! component ON the player entity. From then on the entity's component — not
//! this component — is the single source both gameplay and presentation derive
//! from: [`apply_worn_character_gameplay`] re-applies the kit on any change, and
//! the reusable `ambition_render` binder installs the sprite from the same
//! identity. Presentation reads the same session-owned identity rather than process state.

use bevy::ecs::change_detection::Ref;
use bevy::ecs::system::{Commands, Query};
use bevy::prelude::{Component, Entity, Has, Name, Res, With};

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::actor::WornCharacter;
use ambition_characters::brain::{ActionSet, RangedExecution};

use ambition_body_seed::PersonaBaseline;
use ambition_combat::moveset::ActorMoveset;
use ambition_combat::worn_kit::WornKit;
use ambition_platformer2d_core::movement::MotionModel;

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
    ///
    /// typed (P0.3): this is the RUNTIME seam of the same rule the prepared
    /// registry and the match participant already hold — an id is an id, and a
    /// display name passed here is a mistake the compiler can catch. empty
    /// still means *the content default*; `CharacterId` carries an empty string
    /// as happily as `String` did, so that meaning is unchanged (see
    /// `is_content_default` below, which is the ONE reader of the emptiness).
    pub character_id: ambition_entity_catalog::CharacterId,
}

impl StartingCharacter {
    pub fn new(character_id: impl Into<ambition_entity_catalog::CharacterId>) -> Self {
        Self {
            character_id: character_id.into(),
        }
    }

    /// True when the player spawns as the canonical protagonist (no override) —
    /// an empty id routes through the untouched `from_scratch` bundle.
    pub fn is_default(&self) -> bool {
        self.character_id.as_str().is_empty()
    }

    /// The concrete catalog id to wear: the explicit override, or the
    /// content-installed default when unset. Resolve at spawn time, never at
    /// component construction (the content default installs at the catalog choke point).
    pub fn effective_id<'a>(&'a self, default_character_id: &'a str) -> &'a str {
        if self.character_id.as_str().is_empty() {
            default_character_id
        } else {
            self.character_id.as_str()
        }
    }
}

/// Does this session build a home body at all?
///
/// A MATCH is not that shape — it realizes its own cast from a roster — so the engine handed
/// one an extra controllable actor it had no use for, and match seating grew a whole adoption
/// path to reinterpret that body as a fighter.
///
/// NOT an `Option<StartingCharacter>`, and not an empty id. An empty
/// [`StartingCharacter::character_id`] already means *"wear the provider-relative
/// default"* — absence is taken, and a second meaning on the same emptiness is
/// exactly how a silent default becomes a silent bug.
///
/// and it is a different question from `starting_character`, which is why
/// both exist. That field also names the experience's catalog DEFAULT — the id a
/// worn body falls back to, which a match experience legitimately still has even
/// though it builds no body of its own.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub enum InitialBodyPolicy {
    /// Spawn the session's home body wearing this character. Every exploration
    /// experience, and the default.
    SpawnCharacter(StartingCharacter),
    /// Build no session body. The experience realizes whatever actors it
    /// needs — a match builds its cast from a prepared roster — and there is no
    /// privileged avatar for anything to adopt, re-dress, or follow by accident.
    NoInitialBody,
}

/// What the session's home body HOLDS, as the experience declared it — the
/// resources it is built with and resets to (Ambition's Mana pool, say).
///
/// Empty is a complete answer: a home body that holds nothing. It sits beside
/// [`InitialBodyPolicy`] on the session root for the same reason that does —
/// an activation input the experience authors, read once when the body is
/// built — and is a separate component because a match, which builds no home
/// body, has nothing to say here.
///
/// It holds the PREPARED bank, so a bad declaration is refused where the
/// experience wrote it and setup only clones a finished answer.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct HomeBodyResources(Option<ambition_platformer2d_core::resources::ActorResources>);

impl HomeBodyResources {
    pub fn declared(
        declarations: &[ambition_resource_spec::ResourceDeclaration],
    ) -> Result<Self, ambition_platformer2d_core::resources::ResourceLayoutError> {
        ambition_platformer2d_core::resources::ActorResources::declared(declarations).map(Self)
    }

    /// The bank the home body is built holding, at its declared start.
    pub fn bank(&self) -> Option<&ambition_platformer2d_core::resources::ActorResources> {
        self.0.as_ref()
    }
}

/// What the experience GRANTS and PERMITS its home body, over the verbs the worn
/// character authors: `effective = (authored ∪ granted) ∩ permitted`, the rule
/// a match applies to its seats ([`ambition_platformer2d_core::MatchAbilities`]).
///
/// This is where a progression verb such as Morph Ball comes from: the
/// Ambition experience grants it to its home body, so the same character
/// seated in another experience does not have it. The default grants nothing
/// and permits everything — the character's own kit, unchanged. Read once, when
/// setup builds the home body.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct HomeBodyAbilities(pub ambition_platformer2d_core::MatchAbilities);

impl Default for HomeBodyAbilities {
    fn default() -> Self {
        Self(ambition_platformer2d_core::MatchAbilities::at_most(
            ambition_platformer2d_core::AbilitySet::sandbox_all(),
        ))
    }
}

impl HomeBodyAbilities {
    /// Grant `kit` on top of whatever the worn character authors.
    pub const fn granting(kit: ambition_platformer2d_core::AbilitySet) -> Self {
        Self(ambition_platformer2d_core::MatchAbilities {
            granted: kit,
            permitted: ambition_platformer2d_core::AbilitySet::sandbox_all(),
        })
    }

    /// The home body's intrinsic set for a character that authors `authored`.
    pub fn apply(&self, authored: ambition_platformer2d_core::AbilitySet) -> ambition_platformer2d_core::AbilitySet {
        self.0.apply(Some(authored))
    }
}

impl Default for InitialBodyPolicy {
    fn default() -> Self {
        Self::SpawnCharacter(StartingCharacter::default())
    }
}

impl InitialBodyPolicy {
    /// The character the home body wears, or `None` when there is no home body.
    pub fn starting_character(&self) -> Option<&StartingCharacter> {
        match self {
            Self::SpawnCharacter(character) => Some(character),
            Self::NoInitialBody => None,
        }
    }

    pub fn spawns_a_body(&self) -> bool {
        matches!(self, Self::SpawnCharacter(_))
    }
}

// The curated PLAYABLE cast (which catalog ids the character-select surface
// cycles) is CONTENT — it lives in `ambition_content::character_catalog`
// (`PLAYABLE_ROSTER` / `next_playable`), beside the catalog data it indexes
// (R3.2, residue #10). This module keeps only the engine machinery: the
// StartingCharacter component + the moveset overlay.

// NOTE: the old `overlay_character_moveset` fallback — empty worn slots kept the player's
// swipe/bolt/shield — is GONE, and being the content default no longer implies "keep the
// host's hardcoded kit": the row's authored kit wins.
//
// ⛔⛤ **A ROW CANNOT OPT INTO THE HOST KIT BY NAME, AND THIS SAID IT COULD**
// until 2026-09-19 — it told an author to mark the row `PlayableKitSource::HostCode`,
// a spelling `prepared.rs` deleted once the enum turned out to hold one variant
// and to be answering catalog MEMBERSHIP all along. An instruction nobody can
// carry out is worse than a missing one, because it reads as a supported route.
// A body is rebuilt from its persisted `AbilitySet` exactly when the catalog
// does not know its id; `resolve_playable_action_set` owns that rule and is the
// only place that should state it.

/// The movement policy for `character_id`, DEFINITION first, catalog second.
///
/// A separate function rather than a parameter on the catalog-only one because
/// three call sites legitimately have no registry — a from-scratch bundle
/// predates the world, and two tests build a catalog alone — and threading an
/// `Option<&Registry>` through them would make "there is no registry here" and
/// "the registry had nothing" the same call.
pub fn motion_model_spec_for_character(
    registry: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    catalog: &CharacterCatalog,
    character_id: &str,
) -> ambition_platformer2d_core::MotionModelSpec {
    registry
        .and_then(|registry| registry.get(character_id))
        .map(|prepared| prepared.motion_model)
        // The catalog is consulted only for an id NOTHING registered. A prepared
        // character already folded its row in at the barrier, so falling back
        // here for one would be the displaced authority getting a second vote.
        .unwrap_or_else(|| motion_model_spec_for_character_id(catalog, character_id))
}

/// Resolve the state-free movement policy a CATALOG ROW authors.
///
/// The active experience owns the character catalog. Movement identity must be
/// resolved from that App-local catalog rather than from Ambition's built-in
/// roster, so standalone experiences such as Sanic can author their own policy
/// without process-global registration.
///
/// Prefer [`motion_model_spec_for_character`] wherever a registry is in hand: a
/// definition that authored a motion model outranks the row.
/// The movement FEEL for `character_id`, DEFINITION first, catalog second.
///
/// Companion to [`motion_model_spec_for_character`]: that one picks the solver,
/// this one supplies its numbers. Same precedence rule, and the same reason for
/// being a separate function from the catalog-only lookup.
///
/// `None` is not a default here, it is an ANSWER. The marker component's
/// presence means "this body's tuning is authored rather than the shared dev
/// tuning", so a character that authored none must produce `None` and have the
/// marker REMOVED — otherwise a re-wear from an authored feel back to the
/// sandbox protagonist never returns the body to the live inspector sliders.
pub fn movement_tuning_for_character(
    registry: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    catalog: &CharacterCatalog,
    character_id: &str,
) -> Option<ambition_platformer2d_core::MovementTuning> {
    match registry.and_then(|registry| registry.get(character_id)) {
        Some(prepared) => prepared.movement_tuning,
        None => catalog.axis_tuning(character_id),
    }
}

/// The body of this function was eighteen lines reading nothing but the catalog and
/// `ambition_platformer2d_core` — both visible from `ambition_characters` — so it was a catalog
/// question written next to its first caller. It is `CharacterCatalog:motion_model_spec` now,
/// and character PREPARATION asks the catalog directly instead of reaching up into
/// `crate:avatar`, which is one of the two obstacles keeping the authoritative character model
/// inside this monolith.
pub fn motion_model_spec_for_character_id(
    catalog: &CharacterCatalog,
    character_id: &str,
) -> ambition_platformer2d_core::MotionModelSpec {
    catalog.motion_model_spec(character_id)
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
    registry: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    catalog: &CharacterCatalog,
    character_id: &str,
    current: &mut MotionModel,
) {
    ambition_platformer2d_core::switch_motion_model(
        current,
        motion_model_spec_for_character(registry, catalog, character_id),
    );
}

/// The gameplay overlay a body derives from wearing `character_id`.
///
/// This is the single resolver used by both spawn and runtime re-wear. Every
/// field it writes is a deterministic function of the identity (and, for a
/// seat, the match's kit) — never of the body's abilities or its prior kit.
/// See [`WornKit::resolve`] for the arms.
///
/// Returns HOW the resolved persona fires ([`RangedExecution`]); the ECS derive
/// system synchronizes the charge marker and its mutable state from that.
pub fn apply_worn_character_overlay(
    catalog: &CharacterCatalog,
    registry: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    name: &mut Name,
    action_set: &mut ActionSet,
    moveset: &mut ActorMoveset,
    identity: &mut ambition_characters::brain::action_set::IdentityKit,
    character_id: &str,
    match_kit: Option<&ActionSet>,
) -> RangedExecution {
    let execution = wear_character(catalog, registry, name, identity, character_id, match_kit);
    // Construction: nothing is worn or held yet, so the live pair is the
    // identity's own fold, published with it.
    let live = ambition_characters::repertoire::effective_repertoire(
        identity,
        None,
        ambition_characters::repertoire::Hand::Empty,
    );
    *action_set = live.action_set;
    *moveset = ActorMoveset(live.moveset);
    execution
}

/// Put `character_id` on a body: its display name and its identity baseline.
///
/// Writes only the fold's INPUT. The live `ActionSet` + `ActorMoveset` are
/// `reconcile_effective_repertoire`'s to derive from this baseline together
/// with worn equipment and the hand, so a body re-wearing a character while
/// holding something is never published empty-handed.
pub fn wear_character(
    catalog: &CharacterCatalog,
    registry: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    name: &mut Name,
    identity: &mut ambition_characters::brain::action_set::IdentityKit,
    character_id: &str,
    match_kit: Option<&ActionSet>,
) -> RangedExecution {
    let kit = WornKit::resolve(catalog, registry, character_id, match_kit);
    // Prepared name, else the catalog's, else the id itself, so an unknown id is
    // shown as the id and the problem stays visible.
    *name = Name::new(
        registry
            .and_then(|registry| registry.get(character_id))
            .map(|prepared| prepared.display_name.as_str())
            .or_else(|| catalog.display_name(character_id))
            .unwrap_or(character_id)
            .to_string(),
    );
    *identity = kit.identity;
    kit.execution
}

fn match_kit_for_seat<'a>(
    roster: Option<&'a ambition_match::MatchParticipantRoster>,
    seat: Option<&ambition_match::MatchSeat>,
) -> Option<&'a ActionSet> {
    roster?.participants.get(seat?.0)?.action_set.as_ref()
}

pub fn sync_charge_projectile_capability(
    commands: &mut Commands,
    entity: Entity,
    execution: RangedExecution,
    has_projectile_state: bool,
) {
    // This kit refresh is deferred, and a session-scoped body can be despawned
    // by session teardown in the same frame its worn identity last changed; the
    // `try_` variants apply the capability iff the entity is still alive rather
    // than erroring on a torn-down entity.
    let mut entity_commands = commands.entity(entity);
    if execution.charges_projectiles() {
        entity_commands.try_insert(ambition_characters::brain::ChargesProjectiles);
        if !has_projectile_state {
            entity_commands.try_insert(ambition_projectiles::PlayerProjectileState::default());
        }
    } else {
        entity_commands.try_remove::<ambition_characters::brain::ChargesProjectiles>();
        entity_commands.try_remove::<ambition_projectiles::PlayerProjectileState>();
    }
}

/// Derive a body's gameplay from its worn identity and host ability source.
///
/// An identity change refreshes the complete persona: display name, effective
/// kit, projectile capability, and movement identity. An ability-only change is
/// narrower: only the compatibility kit an id the catalog does not know
/// receives depends on `BodyAbilities`, so only that kit and its projectile
/// capability are rebuilt. ⚠ That gate is `!catalog.knows(id)` rather than an
/// inspection of the prepared kit, and the two cannot diverge: the only other
/// route to `PreparedKit::Unauthored` is preparation with NO catalog, and this
/// system takes `Res<CharacterCatalog>` unconditionally, so it does not run in
/// a composition that has none.
/// In particular, an authored Sanic keeps the persistent `MomentumMotion.state`
/// it accumulated while riding a surface.
pub fn apply_worn_character_gameplay(
    catalog: Res<CharacterCatalog>,
    // Optional, like every other reader of it: a composition with no registered
    // characters is the ordinary case and must not require the resource.
    registry: Option<Res<ambition_characters::prepared::PreparedCharacterRegistry>>,
    // What the MATCH decided, when one is running. `Option` because most
    // compositions are not a match, which is the ordinary case rather than a
    // degraded one.
    roster: Option<Res<ambition_match::MatchParticipantRoster>>,
    mut commands: Commands,
    // The repertoire-bearing bodies: the live pair this system does not write
    // is the fold's, and a body without it has nothing for the fold to publish.
    mut worn: Query<
        (
        Entity,
        Ref<WornCharacter>,
        &mut Name,
        &mut ambition_characters::brain::action_set::IdentityKit,
        // The one transition seam (`switch_motion_model`): a cross-model
        // re-wear initializes destination-private state inside the new
        // variant value; no cluster is touched (ADR 0024).
        &mut MotionModel,
        // The physical baseline's live half. `Option` because not every worn body
        // carries a health pool, and "this path cannot write it" is a different
        // claim from "leave it alone".
        Option<&mut ambition_characters::actor::BodyHealth>,
        // The physical baseline's other live half, read-only: it is WRITTEN
        // through commands, and read here only to capture what the body weighed
        // before any persona spoke for it.
        Option<&ambition_platformer2d_shared_tangle::body::Mass>,
        // The knockback weight's live carrier. `Option` because a body that
        // never fights carries no `CombatTuning`; and this path may only
        // WRITE its field, never insert or remove the component — see
        // `apply_to_body`.
        Option<&mut ambition_combat::CombatTuning>,
        Has<ambition_projectiles::PlayerProjectileState>,
        // What THIS system last applied to this body.
        (
            Option<&PersonaBaseline>,
            // Which seat this body holds, if it is in a match at all.
            Option<&ambition_match::MatchSeat>,
            Has<ambition_characters::actor::RecharacterizeBody>,
        ),
        ),
        (With<ActionSet>, With<ActorMoveset>),
    >,
) {
    for (
        entity,
        character,
        mut name,
        mut identity,
        mut motion_model,
        mut health,
        mass,
        mut combat_tuning,
        has_projectile_state,
        (baseline, seat, recharacterize),
    ) in &mut worn
    {
        let id = character.id();
        let generation = registry
            .as_deref()
            .map(ambition_characters::prepared::PreparedCharacterRegistry::generation)
            .unwrap_or_default();
        // Re-derive automatically only when the prepared cast generation changed
        // or the body lacks a baseline. Identity changes request recharacterization
        // explicitly rather than relying on Bevy change ticks.
        let stale_cast = baseline.is_none_or(|baseline| baseline.generation != generation);

        // NOT `character.is_changed()`. A body is re-derived because
        // somebody ASKED (`RecharacterizeBody`), or because the cast it was
        // built from no longer exists (`stale_cast`, which is a rollback-state
        // test rather than a change tick and therefore survives a rewind).
        //
        // What the edge DID carry was a hidden dependency on a tick that does not rewind.
        if recharacterize {
            // CONSUMED, so a request is one application rather than a
            // state a body gets stuck re-deriving in.
            commands
                .entity(entity)
                .try_remove::<ambition_characters::actor::RecharacterizeBody>();
        }
        if recharacterize || stale_cast {
            let execution = wear_character(
                &catalog,
                registry.as_deref(),
                &mut name,
                &mut identity,
                id,
                // The kit this MATCH gave the seat, when this body is in one.
                // A body with no `MatchSeat` is not in a match and keeps its
                // authored persona, which is every other body in every game.
                match_kit_for_seat(roster.as_deref(), seat),
            );
            sync_charge_projectile_capability(
                &mut commands,
                entity,
                execution,
                has_projectile_state,
            );

            // Character replacement updates authored physical baselines without
            // healing accumulated damage. Unauthored incoming fields retract to
            // the body's saved standing baseline rather than inheriting the prior persona.
            let incoming = registry
                .as_deref()
                .and_then(|registry| registry.get(id))
                .map(ambition_body_seed::PhysicalBaseline::of);
            // What a persona has taken from this body, extended with whatever
            // the INCOMING one is about to take. Recorded before the write, and
            // only for fields no persona has claimed yet.
            let displaced = baseline
                .map(|baseline| baseline.displaced)
                .unwrap_or_default()
                .displace(
                    incoming,
                    health.as_deref().map(|health| health.health.max),
                    mass.map(|mass| mass.0),
                    combat_tuning.as_deref().map(|tuning| tuning.weight),
                );
            if let Some(physical) = incoming {
                physical.apply_to_body(
                    ambition_body_seed::BaselineBoundary::Replacement,
                    &mut commands.entity(entity),
                    health.as_deref_mut(),
                    combat_tuning.as_deref_mut(),
                    None,
                    ambition_body_seed::PhysicalRetraction::resolve(incoming, displaced),
                );
            }

            // Movement identity is identity-derived, not ability-derived. Only
            // a wear/re-wear may replace the model; doing this for a live
            // ability edit would reset SurfaceMomentum's persistent riding
            // state to Airborne.
            sync_worn_motion_model_preserving_state(
                registry.as_deref(),
                &catalog,
                id,
                &mut motion_model,
            );

            // Per-character axis FEEL rides a marker component: presence means
            // "this body's tuning is authored, not the shared F3 dev tuning".
            // Insert it when the worn identity authors a tuning, remove it when
            // it does not — so a re-wear from an authored feel back to the
            // sandbox protagonist returns the body to the live inspector sliders.
            match movement_tuning_for_character(registry.as_deref(), &catalog, id) {
                Some(tuning) => {
                    commands
                        .entity(entity)
                        .try_insert(ambition_platformer2d_core::AuthoredMovementTuning(tuning));
                }
                None => {
                    commands
                        .entity(entity)
                        .try_remove::<ambition_platformer2d_core::AuthoredMovementTuning>();
                }
            }
            // RETRACT BY RESETTING, NEVER BY REMOVING — and the first
            // version of this removed. `CombatCapabilities` is a REQUIRED member
            // of `ActorClusterQueryData`, so a body without it silently leaves
            // the actor cluster query altogether: it stops being simulated as an
            // actor at all. Sixteen versus/smash integration tests went red at
            // once, reporting *"player one swung twelve times and the other
            // fighter is still on 52/52 HP"* — a body nothing could hit because
            // nothing was stepping it. an absent component is not the same
            // statement as a default one, and here only the second is legal.
            //
            // and the reset is conditional on the PREVIOUS persona having
            // claimed these, because construction owns a body's traits too:
            // `ActorClusterSeed::into_components` spawns every clustered actor
            // with capabilities from its archetype. Resetting unconditionally
            // would strip an exploding mite the moment anything wore a character
            // on it. Same shape as the health/mass displacement rule directly
            // above, kept narrow deliberately.
            let authored = registry
                .as_deref()
                .and_then(|registry| registry.get(id))
                .and_then(|prepared| prepared.death_traits.as_ref())
                .map(ambition_combat::CombatCapabilities::from);
            let previous_authored = baseline
                .zip(registry.as_deref())
                .and_then(|(baseline, registry)| registry.get(&baseline.id))
                .is_some_and(|previous| previous.death_traits.is_some());
            match authored {
                Some(capabilities) => {
                    commands.entity(entity).try_insert(capabilities);
                }
                None if previous_authored => {
                    commands
                        .entity(entity)
                        .try_insert(ambition_combat::CombatCapabilities::default());
                }
                None => {}
            }
            // LAST, and that ordering is the point: the record says the baseline
            // HAS been applied, so it must not be written by anything that has
            // not applied it — including this system on an early return.
            commands.entity(entity).try_insert(PersonaBaseline {
                id: id.to_string(),
                generation,
                // Extended above, never re-read: a field already displaced keeps
                // the value it displaced, so a second swap still retracts to the
                // BODY rather than to the first character.
                //
                // for a body CONSTRUCTED wearing a character (a seated fighter),
                // what this displaces is what construction built — which includes
                // that character's authored numbers. Deliberate, and the honest
                // meaning of the record: it is the body as it entered the world,
                // not an archetype default this path never sees.
                displaced,
            });
            continue;
        }
    }
}

/// Gate the raw player-control frame by the effective worn kit before direct body/effect
/// consumers read it. `sustain_bubble_shield` must run after this gate.
///
/// TODO(compat-remove): migrate the remaining direct `ActorControl` consumers to the
/// `ActionSet`-gated semantic path, then remove this raw-frame compatibility gate.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WornControlGateSet;

pub fn gate_worn_player_control(
    mut players: Query<
        (
            &ActionSet,
            // The body's live combat/ability authorities — the SAME inputs the
            // control-prompt read-model derives its labels from. The gate resolves
            // them through the shared `derive_action_scheme` so what a slot GATES
            // here and what the prompt SHOWS are one derivation (no UI drift).
            &ambition_platformer2d_core::BodyAbilities,
            Option<&ActorMoveset>,
            Option<&ambition_characters::action_scheme::ActorTechniques>,
            &mut ambition_characters::control::ActorControl,
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
        (
            With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            With<WornCharacter>,
        ),
    >,
) {
    use ambition_characters::action_scheme::{derive_action_scheme, resolve_control_slots};

    for (
        actions,
        abilities,
        moveset,
        techniques,
        mut control,
        mut tech_edges,
        has_charge_marker,
        holds_item,
    ) in &mut players
    {
        // THE shared resolver — byte-identical to the call the ControlPrompt
        // producer makes on the same immediate authorities.
        let scheme = derive_action_scheme(
            &abilities.abilities,
            moveset.map(|m| &m.0),
            Some(actions),
            techniques.map_or(&[], |t| t.0.as_slice()),
        );

        // Per-slot dispatch: route every technique to its sanctioned edge, strip
        // the verbs the scheme doesn't own (Attack/Special/Projectile), and keep
        // the moveset `Move`s. A technique-bearing body always has
        // `ResolvedTechniqueEdges` (required by `ActorTechniques`), so nothing is
        // dropped for a missing sink; the local fallback only ever backs a body
        // with no techniques (nothing routes into it).
        let mut fallback_edges =
            ambition_characters::action_scheme::ResolvedTechniqueEdges::default();
        let edges = tech_edges.as_deref_mut().unwrap_or(&mut fallback_edges);
        let unroutable = resolve_control_slots(&scheme, &mut control.0, edges, holds_item);
        debug_assert!(
            unroutable.is_empty(),
            "action scheme declared a technique on a slot the combat gate cannot route \
             yet (needs the Phase-3 kernel re-key): {unroutable:?}",
        );

        // the question now lives with every other slot's, inside
        // `resolve_control_slots` above: the Shield slot is present iff the body
        // has the shield ability, and the held-item exception (shield+attack is
        // the throw gesture) moved there with it. `sustain_bubble_shield` runs
        // AFTER this system and still forces the guard up for the folded
        // bubble-shield special, so a special MAY raise a guard — it is simply no
        // longer the only way any body has.

        // HOW THIS BODY FIRES is its `ChargesProjectiles` marker, installed
        // from the character's `ranged_execution` by every road that builds or
        // re-wears it; a body without it has no charge path to feed.
        if !has_charge_marker {
            control.0.projectile_pressed = false;
            control.0.projectile_held = false;
            control.0.projectile_released = false;
        }
    }
}

/// Raise (and sustain) a `"bubble_shield"` persona's guard through the ONE shield
/// path (`shield_held` → `resolve_shield`) — so pressing Special actually deploys
/// the bubble shield instead of playing a bare animation.
///
/// Two conditions force `shield_held`, and together they eliminate the one-tick
/// lag the folded special would otherwise have:
///
/// - The press tick. The special MOVE is triggered later in the tick
///   (`trigger_moveset_moves` runs in `Combat`, after `PlayerInput`/`WorldPrep`),
///   so on the tick Special is pressed there is no `MovePlayback` yet. Reading the
///   `special_pressed` edge here — in `PlayerInput`, before the `WorldPrep` kernel
///   bridge — raises the guard the SAME tick the button goes down.
/// - The move's duration. Once the move is playing, its `id` equals the body's
///   `ActionSet.special` key (that is how `build_actor_moveset` folds the marker
///   in), so a `bubble_shield` persona keeps the guard up BY IDENTITY for as long
///   as the move plays — no per-body wiring.
///
/// The on-screen Special button reads the SAME scheme, so it cannot advertise a
/// shield the body won't raise. Forcing `shield_held` (rather than poking
/// [`ambition_platformer2d_core::body_clusters::BodyShieldState`] directly) keeps the
/// kernel's parry-window / dash-gating rules uniform — the special is just another
/// way to raise the ONE shield. Runs after [`gate_worn_player_control`], which
/// keeps the persona's shield verb and (as a `Move`) its `special_pressed` alive.
pub fn sustain_bubble_shield(
    mut bodies: Query<(
        &ActionSet,
        Option<&ambition_combat::moveset::MovePlayback>,
        &mut ambition_characters::control::ActorControl,
        // WHICH SPECIAL THIS PRESS MEANS. See the directional note below.
        Option<&ambition_combat::moveset::ActorMoveset>,
        Option<&ambition_platformer2d_core::BodyKinematics>,
        Option<&ambition_platformer2d_core::BodyGroundState>,
    )>,
) {
    use ambition_characters::brain::SpecialActionSpec;
    for (actions, playback, mut control, moveset, kin, ground) in &mut bodies {
        let Some(SpecialActionSpec::Special(key)) = actions.special.as_ref() else {
            continue;
        };
        if key != "bubble_shield" {
            continue;
        }
        let playing = playback.is_some_and(|p| p.spec.id == *key);
        // ⛔⛔ A RAW SPECIAL EDGE IS NOT A BUBBLE-SHIELD PRESS, and reading it as
        // one broke every OTHER special the body owns (D253). `player_robot_v3` names `bubble_shield` as its body-kit
        // special AND authors a full directional repertoire — rocket dash,
        // phase shift, the stabilizers. This ran in `PlayerInputSet::ControlGate`
        // on the bare `special_pressed` edge, so EVERY special press raised the
        // guard first: grounded shield plus a direction is an evade, airborne is
        // an air dodge, and by the time `Combat` asked which special was meant
        // the move was refused from the state this layer had just created. One
        // shared authority error, reported as five broken moves.
        //
        // ⭐ SO ASK, DO NOT GUESS — with the SAME two calls the resolver uses.
        // `attack_dir_from_axis` is the one place a facing is folded into an
        // aim, and `move_for_directional_verb` is the one place a direction
        // picks a special. Reimplementing either here would replace one
        // authority problem with two.
        //
        // ⭐ A BODY WITH NO REPERTOIRE STILL SHIELDS ON ANY PRESS. That is the
        // ordinary body kit this compatibility layer was written for, where
        // Special means exactly one thing and a direction cannot select
        // anything else.
        let means_the_bubble = match moveset {
            None => true,
            Some(moveset) => {
                let direction = ambition_combat::moveset::attack_dir_from_axis(
                    control.0.attack_axis,
                    kin.map_or(1.0, |kin| kin.facing),
                );
                let grounded = ground.is_none_or(|ground| ground.on_ground);
                moveset
                    .0
                    .move_for_directional_verb(
                        ambition_combat::moveset::SPECIAL_VERB,
                        direction,
                        grounded,
                    )
                    .is_none_or(|spec| spec.id == *key)
            }
        };
        if (control.0.special_pressed && means_the_bubble) || playing {
            control.0.shield_held = true;
        }
    }
}

#[cfg(test)]
mod tests;
