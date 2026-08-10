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
use bevy::prelude::{Component, Entity, Has, Name, Res, With};

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

/// **Does this session build a home body at all?**
///
/// ⛔ **it did not used to be a question, and that was an engine assumption
/// nobody had written down.** Every platformer session constructed a privileged
/// primary body: `simulation_world` always returned one and
/// `SessionBuildResult.player` was an `Entity`, not an option. A MATCH is not
/// that shape — it realizes its own cast from a roster — so the engine handed
/// one an extra controllable actor it had no use for, and match seating grew a
/// whole adoption path to reinterpret that body as a fighter. Every symptom of
/// Jon's 2026-08-06 report came out of the reinterpretation.
///
/// ⚠ **NOT an `Option<StartingCharacter>`, and not an empty id.** An empty
/// [`StartingCharacter::character_id`] already means *"wear the provider-relative
/// default"* — absence is taken, and a second meaning on the same emptiness is
/// exactly how a silent default becomes a silent bug.
///
/// ⚠ **and it is a different question from `starting_character`**, which is why
/// both exist. That field also names the experience's catalog DEFAULT — the id a
/// worn body falls back to, which a match experience legitimately still has even
/// though it builds no body of its own.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub enum InitialBodyPolicy {
    /// Spawn the session's home body wearing this character. Every exploration
    /// experience, and the default.
    SpawnCharacter(StartingCharacter),
    /// **Build no session body.** The experience realizes whatever actors it
    /// needs — a match builds its cast from a prepared roster — and there is no
    /// privileged avatar for anything to adopt, re-dress, or follow by accident.
    NoInitialBody,
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

// NOTE (2026-07-05): the old `overlay_character_moveset` fallback — empty worn
// slots kept the player's swipe/bolt/shield — is GONE. Wearing is possession
// semantics: the worn character's authored ActionSet IS the kit (Jon's Sanic
// report: a peaceful speedster must not secretly shoot the robot's fireballs).
// A protagonist whose kit is a runtime `AbilitySet` concern opts its ROW into
// `PlayableKitSource::HostCode` (the kit is rebuilt from the body's persisted
// `AbilitySet`); the DEFAULT is that the row's authored kit wins — being the
// content default no longer implies "keep the host's hardcoded kit" (2026-07-11).

/// The movement policy for `character_id`, DEFINITION first, catalog second.
///
/// The action set's precedence rule applied to the third leg of the kit: a
/// character that authored how it moves outranks the row that guessed. `None` on
/// the definition means the author said nothing and the catalog stands, which is
/// every character that has not authored one (campaign R-a, landed 2026-07-28).
///
/// A separate function rather than a parameter on the catalog-only one because
/// three call sites legitimately have no registry — a from-scratch bundle
/// predates the world, and two tests build a catalog alone — and threading an
/// `Option<&Registry>` through them would make "there is no registry here" and
/// "the registry had nothing" the same call.
pub fn motion_model_spec_for_character(
    registry: Option<&crate::character_runtime::PreparedCharacterRegistry>,
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
/// ⚠ `None` is not a default here, it is an ANSWER. The marker component's
/// presence means "this body's tuning is authored rather than the shared dev
/// tuning", so a character that authored none must produce `None` and have the
/// marker REMOVED — otherwise a re-wear from an authored feel back to the
/// sandbox protagonist never returns the body to the live inspector sliders.
pub fn movement_tuning_for_character(
    registry: Option<&crate::character_runtime::PreparedCharacterRegistry>,
    catalog: &CharacterCatalog,
    character_id: &str,
) -> Option<ambition_platformer2d_core::MovementTuning> {
    match registry.and_then(|registry| registry.get(character_id)) {
        // A prepared character's `None` is an ANSWER — "no authored feel" — and
        // asking the catalog after it would re-open a question the barrier
        // already closed. This used to be an `.or_else` chain, which is exactly
        // the shape that made the seated and worn paths disagree.
        Some(prepared) => prepared.movement_tuning,
        None => catalog.axis_tuning(character_id),
    }
}

pub fn motion_model_spec_for_character_id(
    catalog: &CharacterCatalog,
    character_id: &str,
) -> ambition_platformer2d_core::MotionModelSpec {
    match catalog.momentum_params(character_id) {
        Some(params) => ambition_platformer2d_core::MotionModelSpec::SurfaceMomentum(params),
        None => ambition_platformer2d_core::MotionModelSpec::AxisSwept(
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
    registry: Option<&crate::character_runtime::PreparedCharacterRegistry>,
    catalog: &CharacterCatalog,
    character_id: &str,
    current: &mut MotionModel,
) {
    ambition_platformer2d_core::switch_motion_model(
        current,
        motion_model_spec_for_character(registry, catalog, character_id),
    );
}

/// The host code kit's moves: built from its action set, wearing the robot
/// blade's sound family, unless the character brought its own timelines.
///
/// The blade's SFX belongs to the code-built kit rather than to a character named
/// "player" — whoever wears that kit is swinging that blade — and an AUTHORED
/// moveset brings its own cues, which is why the override lands after the stamp
/// rather than before it.
///
/// `special` IS folded in here and nowhere else. On this kit the special is a
/// capability marker (`bubble_shield`) with no authored move behind it; an
/// authored persona drives its special through its own path, so folding a
/// generic shell move there would make one press fire two things.
/// **HOW a body fires — the single fact four decisions used to make separately.**
///
/// A body that fires has exactly one mechanism for it, and which one it is
/// decides more than it looks:
///
/// * whether the action set's `ranged` preset folds into the derived moveset,
/// * whether its `special` preset does,
/// * whether the protagonist's blade SFX is stamped over the derived melee,
/// * and whether the body carries `ChargesProjectiles` and its projectile state.
///
/// All four are the same question, and until this enum they were asked in three
/// places in two spellings: a `bool` named for the fourth, and two hard-coded
/// arguments (`None` for ranged here, `None` for special at the catalog site)
/// each carrying a comment explaining that it was the opposite of the other. A
/// fifth consultation could have been written in a third spelling without
/// contradicting anything, which is what the queue meant by warning that a new
/// name is only worth having if it becomes the SOLE switch.
///
/// It is not a component. `ChargesProjectiles` is the runtime marker and stays
/// exactly as it was; this is the derivation-time decision that installs it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RangedExecution {
    /// The host's chargeable-projectile mechanic owns the ranged press.
    ///
    /// The compat kit every unknown id and every `HostCode` row wears. Its
    /// ranged verb is NOT a moveset move — folding one in would make one press
    /// do two things, which is the test that went red when the queue's H2 first
    /// tried passing `set.ranged` unconditionally.
    HostCharge,
    /// A moveset verb derived from the action set's own `ranged` preset.
    ///
    /// What content-authored personas use. They have no charge mechanic and no
    /// shell `special` marker, so their special is authored into their moves
    /// rather than folded from the set.
    MovesetVerb,
}

impl RangedExecution {
    /// Whether a body executing this way carries the host charge capability.
    pub fn charges_projectiles(self) -> bool {
        matches!(self, Self::HostCharge)
    }
}

/// Derive a persona's moves from its action set, given HOW it fires.
///
/// The `authored` contract, when present, replaces the derivation outright: a
/// character that authored its own moveset said something more specific than
/// anything derivable from presets.
///
/// ⚠ **`pub` for FIXTURES, and the reason is worth a line.** A body's swing is
/// built HERE, at spawn, from its action set — so a harness that mutates
/// `ActionSet.melee` afterwards changes nothing the runtime reads, which is a
/// silent no-op a test cannot see. The rollback exit oracle needs to state the
/// swing its route walks with (its frame counts are measured against one), and
/// the only honest way to do that is to rebuild the moveset the same way the
/// spawn did.
pub fn derive_persona_moveset(
    set: &ActionSet,
    execution: RangedExecution,
    authored: Option<ambition_entity_catalog::MovesetContract>,
) -> ambition_entity_catalog::MovesetContract {
    let (ranged, special) = match execution {
        // The charge mechanic already owns the ranged press; `special` is the
        // shell marker this kit's moves are built from.
        RangedExecution::HostCharge => (None, set.special.as_ref()),
        // Symmetrically: the ranged preset IS the ranged verb — and the special
        // preset IS the special verb.
        //
        // ⚠ this arm passed `None` for special, on the reasoning that an authored
        // persona puts its special into its authored MOVES. That holds only when
        // it authored a moveset, and the API does not require one: a character
        // with `action_set.special = Some(..)` and no moveset advertised a
        // signature move the brain would press with no timeline to run — H2's
        // defect one field over (GPT 5.6, 2026-07-29). `authored` still
        // overrides the whole derivation below, so a persona that DID author its
        // moves is unaffected.
        RangedExecution::MovesetVerb => (set.ranged.as_ref(), set.special.as_ref()),
    };
    let mut derived =
        build_actor_moveset(None, set.melee.as_ref(), ranged, special).unwrap_or_default();
    if execution.charges_projectiles() {
        crate::combat::moveset::apply_player_robot_slash_sfx(&mut derived);
    }
    authored.unwrap_or(derived)
}

/// Resolve a playable ActionSet without collapsing an invalid authored row into
/// the privileged host-code fallback. The returned [`RangedExecution`] says how
/// the body fires.
fn resolve_playable_action_set(
    source: Option<ambition_characters::actor::character_catalog::PlayableKitSource>,
    authored: Option<ActionSet>,
    base_abilities: ambition_platformer2d_core::AbilitySet,
) -> (ActionSet, RangedExecution) {
    use ambition_characters::actor::character_catalog::PlayableKitSource;

    match source {
        Some(PlayableKitSource::HostCode) => (
            crate::avatar::bundles::default_player_action_set(base_abilities),
            RangedExecution::HostCharge,
        ),
        Some(PlayableKitSource::Authored) => {
            // A known authored row with a missing preset is malformed content.
            // The startup validator reports it; runtime remains fail-safe and
            // peaceful rather than silently granting the host protagonist kit.
            (
                authored.unwrap_or_else(ActionSet::peaceful),
                RangedExecution::MovesetVerb,
            )
        }
        None => (
            // Unknown ids use one explicit compatibility fallback. This is
            // intentionally distinct from a known-but-invalid Authored row.
            crate::avatar::bundles::default_player_action_set(base_abilities),
            RangedExecution::HostCharge,
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
/// Returns HOW the resolved persona fires ([`RangedExecution`]); the ECS derive
/// system synchronizes the charge marker and its mutable state from that.
pub fn apply_worn_character_overlay(
    catalog: &CharacterCatalog,
    // C3: the prepared registry, consulted for the moveset. Threaded THROUGH this
    // one construction rather than written over its result downstream — the action
    // set, the identity baseline and the moveset have to be built together, or
    // equipment reconciliation re-derives from a baseline that disagrees with the
    // moveset actually on the body (GPT 5.6, 2026-07-27).
    registry: Option<&crate::character_runtime::PreparedCharacterRegistry>,
    name: &mut Name,
    action_set: &mut ActionSet,
    moveset: &mut ActorMoveset,
    identity: &mut ambition_characters::brain::action_set::IdentityKit,
    // OPTIONAL because not every body has a durable baseline to keep in step: a
    // seated fighter carries `CombatKit` (seating seeds it), the plain player
    // bundle does not. `None` is "this body has no second baseline", which is a
    // different claim from "leave it stale" — and the type says which.
    combat_kit: Option<&mut crate::combat::components::CombatKit>,
    character_id: &str,
    base_abilities: ambition_platformer2d_core::AbilitySet,
    // See `MatchParticipant::action_set`.
    match_kit: Option<&ActionSet>,
) -> RangedExecution {
    // NAME. A known row supplies a display name; an unknown id becomes its own
    // label — deterministic and never stale, and a legible diagnostic that a body
    // is wearing an id the catalog does not know.
    // The REGISTRY first, then the catalog, then the id.
    //
    // Registration is the newer authority and a registered-only character has no
    // catalog row at all — so asking the catalog first named those bodies after
    // their raw id. That did not matter while this ran only for the worn player,
    // whose id is always a catalog one; it started mattering the moment seated
    // fighters began coming through here, and every versus fighter is
    // registered-only (Phase B, 2026-07-29).
    let display = registry
        .and_then(|registry| registry.get(character_id))
        .map(|prepared| prepared.display_name.as_str())
        .or_else(|| catalog.display_name(character_id));
    match display {
        Some(display) => *name = Name::new(display.to_string()),
        None => *name = Name::new(character_id.to_string()),
    }

    apply_worn_character_kit(
        catalog,
        registry,
        action_set,
        moveset,
        identity,
        combat_kit,
        character_id,
        base_abilities,
        match_kit,
    )
}

/// **The kit this body's MATCH gave it**, if it is in one.
///
/// A body with no `MatchSeat` is not in a match and keeps its authored persona,
/// which is every other body in every game. ⚠ keyed by SEAT rather than by
/// character id, because a mirror match is legal: two seats may wear one
/// character and a per-character lookup would give them the same kit by
/// accident rather than by decision.
fn match_kit_for_seat<'a>(
    roster: Option<&'a crate::character_runtime::MatchParticipantRoster>,
    seat: Option<&crate::character_runtime::MatchSeat>,
) -> Option<&'a ActionSet> {
    roster?.participants.get(seat?.0)?.action_set.as_ref()
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
    registry: Option<&crate::character_runtime::PreparedCharacterRegistry>,
    action_set: &mut ActionSet,
    moveset: &mut ActorMoveset,
    identity: &mut ambition_characters::brain::action_set::IdentityKit,
    // The DURABLE half of the same baseline, when this body has one. See the
    // publication point below.
    combat_kit: Option<&mut crate::combat::components::CombatKit>,
    character_id: &str,
    base_abilities: ambition_platformer2d_core::AbilitySet,
    // **What the MATCH says this fighter fights with**, if a match said anything.
    // See `MatchParticipant::action_set`.
    match_kit: Option<&ActionSet>,
) -> RangedExecution {
    let prepared = registry.and_then(|registry| registry.get(character_id));

    // **A REGISTERED character's kit is already decided.** (H1, 2026-07-29)
    //
    // This function used to perform the fold itself — definition first, catalog
    // row second, `Some(empty)` outranking the row exactly as a filled set does.
    // Every word of that precedence still holds; it just happens at the
    // preparation barrier now, for the whole cast at once, which is what lets the
    // SEATED path get the same answer. That path has no catalog to fold with, so
    // for a day the two disagreed about the same character.
    //
    // The catalog arm below is not a fallback for a prepared character. It serves
    // ids that are in the catalog and were never registered — most of the legacy
    // cast — which have no prepared value to disagree with.
    // ⭐ **A MATCH OUTRANKS THE PERSONA, and only a match.** Checked first rather
    // than folded, because the roster is not another opinion about who the
    // character IS — it is a rule of the stage they are standing on, exactly like
    // `fighter_abilities`. A crossover grid borrows Alice, whose row says
    // `peaceful` and is RIGHT about her: she was authored to stand in a room and
    // talk. The stage is the only thing that may say otherwise, and it may not
    // say it by editing her row.
    //
    // ⚠ still ONE writer, and it falls through to the SAME publication below.
    // Seating deliberately does not author moves — its own comment says *"that is
    // `WornCharacter`'s job… seating must not author a second opinion about
    // them"* — so the override is consulted here, where the persona is derived,
    // and the identity baseline, the moveset and the durable combat kit are still
    // built together by the one path that knows they have to agree.
    let (set, derived, execution) =
        if let Some(kit) = match_kit {
            let execution = RangedExecution::MovesetVerb;
            let derived = derive_persona_moveset(kit, execution, None);
            (kit.clone(), derived, execution)
        } else {
            match prepared.map(|prepared| &prepared.kit) {
                Some(crate::character_runtime::PreparedKit::Authored {
                    action_set,
                    moveset,
                }) => (
                    action_set.clone(),
                    moveset.clone(),
                    // The charge mechanic is the CODE-SIDE compat kit's, and a character
                    // whose capabilities content decided is not wearing that kit.
                    RangedExecution::MovesetVerb,
                ),
                Some(crate::character_runtime::PreparedKit::HostCode { authored_moveset }) => {
                    let set = crate::avatar::bundles::default_player_action_set(base_abilities);
                    let execution = RangedExecution::HostCharge;
                    let derived = derive_persona_moveset(&set, execution, authored_moveset.clone());
                    (set, derived, execution)
                }
                None => {
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
                    // ONE call for both kits now. The two arms this replaced differed
                    // only in which presets they folded and whether they stamped the
                    // blade SFX — which is precisely what `RangedExecution` decides, so
                    // they were the same call written twice with the answer inlined.
                    let (set, execution) =
                        resolve_playable_action_set(source, authored, base_abilities);
                    let derived = derive_persona_moveset(&set, execution, None);
                    (set, derived, execution)
                }
            }
        };
    // Publish what IDENTITY alone derived, before any equipment overlay. This is
    // the baseline `reconcile_equipment_grants` re-derives the live kit from, which
    // is what makes a granted verb revocable: without it, a consumed or downgraded
    // row could not take its verb back, because the live set no longer remembers
    // which half of it came from the body and which from a row.
    // **AND `CombatKit`, WHICH IS THE SAME BASELINE.**
    //
    // `ActionSet` is the hot per-frame resolver; `CombatKit` is the DURABLE
    // source of the same capability, and its own doc says so — "what the actor
    // can do innately, before current held-item overlays are applied". Several
    // subsystems reconstruct an `ActionSet` from it rather than reading the live
    // one: `apply_catalog_mode` on a brain command, the mount pair, autonomous
    // reconciliation.
    //
    // Seating seeds it from `ActionSet::default()` — an empty kit, matching what
    // an enemy spawn does before its archetype fills one in — and this writer
    // then installed the real action set, moveset and identity kit and left the
    // durable one at the placeholder. So a seated fighter could act through its
    // live `ActionSet` and then LOSE its innate attacks the moment anything
    // rebuilt them from the stale baseline (GPT 5.6, 2026-07-29).
    //
    // That is precisely the split this campaign exists to remove: one identity,
    // one writer, every derived baseline published together. Equipment stays an
    // overlay — `CombatKit` is the INNATE kit, so it is built from the identity's
    // set exactly as `IdentityKit` is, and a granted verb is layered over it
    // rather than baked into it.
    if let Some(combat_kit) = combat_kit {
        *combat_kit = crate::combat::components::CombatKit::from_action_set(&set);
    }
    *identity =
        ambition_characters::brain::action_set::IdentityKit::of(set.clone(), derived.clone());
    *moveset = ActorMoveset(derived);
    *action_set = set;
    execution
}

fn sync_charge_projectile_capability(
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
    // Optional, like every other reader of it: a composition with no registered
    // characters is the ordinary case and must not require the resource.
    registry: Option<Res<crate::character_runtime::PreparedCharacterRegistry>>,
    // **What the MATCH decided**, when one is running. `Option` because most
    // compositions are not a match, which is the ordinary case rather than a
    // degraded one.
    roster: Option<Res<crate::character_runtime::MatchParticipantRoster>>,
    mut commands: Commands,
    mut worn: Query<(
        Entity,
        Ref<WornCharacter>,
        &mut Name,
        &mut ActionSet,
        &mut ActorMoveset,
        &mut ambition_characters::brain::action_set::IdentityKit,
        // The DURABLE capability baseline, on the bodies that carry one — a
        // seated fighter does, the plain player bundle does not. It is published
        // WITH the identity kit rather than beside it, because a second baseline
        // updated separately is a second baseline that can be stale.
        Option<&mut crate::combat::components::CombatKit>,
        Ref<crate::actor::BodyAbilities>,
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
        Option<&crate::features::Mass>,
        // The knockback weight's live carrier. ⚠ `Option` because a body that
        // never fights carries no `CombatTuning`; ⛔ and this path may only
        // WRITE its field, never insert or remove the component — see
        // `apply_to_body`.
        Option<&mut crate::combat::CombatTuning>,
        Has<ambition_projectiles::PlayerProjectileState>,
        // What THIS system last applied to this body. See [`PersonaBaseline`]:
        // the change-detection filter that used to live here could not see a
        // cast replacement, because a replacement changes nothing on a body.
        Option<&PersonaBaseline>,
        // Which seat this body holds, if it is in a match at all.
        Option<&crate::character_runtime::MatchSeat>,
    )>,
) {
    use ambition_characters::actor::character_catalog::PlayableKitSource;

    for (
        entity,
        character,
        mut name,
        mut action_set,
        mut moveset,
        mut identity,
        mut combat_kit,
        abilities,
        mut motion_model,
        mut health,
        mass,
        mut combat_tuning,
        has_projectile_state,
        baseline,
        seat,
    ) in &mut worn
    {
        let id = character.id();
        let generation = registry
            .as_deref()
            .map(crate::character_runtime::PreparedCharacterRegistry::generation)
            .unwrap_or_default();
        // A body needs the whole persona re-derived when its identity changed OR
        // when the cast it was built from is no longer the cast that exists. The
        // `Or<(Changed<..>, Changed<..>)>` query filter that used to stand here
        // could only ever express the first, which left every live body wearing a
        // retired kit after a hot reload.
        let stale_cast =
            baseline.is_none_or(|baseline| baseline.id != id || baseline.generation != generation);
        if character.is_changed() || stale_cast {
            let execution = apply_worn_character_overlay(
                &catalog,
                registry.as_deref(),
                &mut name,
                &mut action_set,
                &mut moveset,
                &mut identity,
                combat_kit.as_deref_mut(),
                id,
                abilities.abilities,
                // **The kit this MATCH gave the seat**, when this body is in one.
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

            // **WHAT THE BODY PHYSICALLY IS follows the character it wears.**
            //
            // A re-wear is character REPLACEMENT — one of the boundaries a body
            // may legitimately be told its maximum health and mass — so a
            // possession or a character swap moves those numbers to the new
            // identity's instead of leaving the previous one's on the body.
            //
            // ⚠ `Replacement`, not `Construction`, and the difference is a real
            // gameplay rule: the damage a body has taken is the BODY's and
            // survives the swap, clamped under the new maximum. Refilling here
            // would make wearing a character mid-round a free heal. Geometry is
            // deliberately not applied — see [`BaselineBoundary::Replacement`].
            //
            // ⚠ **and what the INCOMING character does not author is retracted,
            // not inherited.** `standing` is the body's own physical answer,
            // captured the first time any persona was projected onto it and
            // carried forward since. Without it, absence read as "keep what is
            // there" — so wearing a 2.0-mass 60-health duelist and then a persona
            // that authors neither left the body at 2.0 and 60, and every later
            // swap accumulated instead of replacing (GPT 5.6, 2026-07-30). The
            // same defect appeared when a hot-reloaded generation dropped an
            // override from `Some` to `None`.
            let incoming = registry
                .as_deref()
                .and_then(|registry| registry.get(id))
                .map(crate::character_runtime::PhysicalBaseline::of);
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
                    crate::character_runtime::BaselineBoundary::Replacement,
                    &mut commands.entity(entity),
                    health.as_deref_mut(),
                    combat_tuning.as_deref_mut(),
                    None,
                    crate::character_runtime::PhysicalRetraction::resolve(incoming, displaced),
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
            // **DEATH TRAITS follow the character too** (D73 phase 1), on the
            // same insert-or-retract rule as the feel marker directly above and
            // for the same reason: absence is an ANSWER. Wearing a sandbag and
            // then a duelist must leave a killable duelist, not an unkillable
            // one — and until a character could author these at all, the only
            // bodies that had them were archetype-built, so a seated fighter or
            // a worn player had no death traits whatever the character was.
            //
            // ⛔⛔ **RETRACT BY RESETTING, NEVER BY REMOVING** — and the first
            // version of this removed. `CombatCapabilities` is a REQUIRED member
            // of `ActorClusterQueryData`, so a body without it silently leaves
            // the actor cluster query altogether: it stops being simulated as an
            // actor at all. Sixteen versus/smash integration tests went red at
            // once, reporting *"player one swung twelve times and the other
            // fighter is still on 52/52 HP"* — a body nothing could hit because
            // nothing was stepping it. ⭐ an absent component is not the same
            // statement as a default one, and here only the second is legal.
            //
            // ⚠ **and the reset is conditional on the PREVIOUS persona having
            // claimed these**, because construction owns a body's traits too:
            // `ActorClusterSeed::into_components` spawns every clustered actor
            // with capabilities from its archetype. Resetting unconditionally
            // would strip an exploding mite the moment anything wore a character
            // on it. Same shape as the health/mass displacement rule directly
            // above, kept narrow deliberately.
            let authored = registry
                .as_deref()
                .and_then(|registry| registry.get(id))
                .and_then(|prepared| prepared.combat_capabilities.clone());
            let previous_authored = baseline
                .zip(registry.as_deref())
                .and_then(|(baseline, registry)| registry.get(&baseline.id))
                .is_some_and(|previous| previous.combat_capabilities.is_some());
            match authored {
                Some(capabilities) => {
                    commands.entity(entity).try_insert(capabilities);
                }
                None if previous_authored => {
                    commands
                        .entity(entity)
                        .try_insert(crate::combat::CombatCapabilities::default());
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
                // ⚠ for a body CONSTRUCTED wearing a character (a seated fighter),
                // what this displaces is what construction built — which includes
                // that character's authored numbers. Deliberate, and the honest
                // meaning of the record: it is the body as it entered the world,
                // not an archetype default this path never sees.
                displaced,
            });
            continue;
        }

        if abilities.is_changed() {
            let source = catalog.playable_kit_source(id);
            if matches!(source, Some(PlayableKitSource::HostCode)) || source.is_none() {
                let execution = apply_worn_character_kit(
                    &catalog,
                    registry.as_deref(),
                    &mut action_set,
                    &mut moveset,
                    &mut identity,
                    combat_kit.as_deref_mut(),
                    id,
                    abilities.abilities,
                    match_kit_for_seat(roster.as_deref(), seat),
                );
                sync_charge_projectile_capability(
                    &mut commands,
                    entity,
                    execution,
                    has_projectile_state,
                );
            }
        }
    }
}

/// **What cast a body's PERSONA was built from.**
///
/// The record `apply_worn_character_gameplay` keeps of its own work: the id it
/// last applied and the registry generation it read. Nothing else writes it.
///
/// It exists because a cast replacement changes nothing ON a body — not its worn
/// character, not its abilities — so the derive's change-detection filter could
/// not see one, and a rebalanced or hot-reloaded character left every live body
/// wearing the retired kit. `project_prepared_character_definitions` DID notice,
/// and stamped its own marker with the new generation regardless, which made the
/// failure worse than a missed update: the body recorded that it was current, so
/// nothing would ever revisit it (GPT 5.6, 2026-07-29).
///
/// ⚠ deliberately a SECOND marker rather than sharing
/// [`ProjectedCharacterKit`](crate::character_runtime::ProjectedCharacterKit).
/// That one is the projection's record of the body facts IT grants — authored
/// hurtboxes, movement feel, motion model. Two writers sharing one "already done"
/// record is how one of them ends up certifying work the other has not performed,
/// which is precisely the defect this fixes. One writer, one record, each stamped
/// only after its own work is applied.
/// ⚠ it also carries `displaced`, which is NOT a memo — it is the only surviving
/// record of what a persona took from this body, and retraction is driven from
/// it. `Eq` is gone with it: a mass is an `f32`.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PersonaBaseline {
    pub id: String,
    pub generation: crate::character_runtime::CharacterCatalogGeneration,
    /// See [`DisplacedPhysicals`](crate::character_runtime::DisplacedPhysicals).
    /// Grows once per field, at the first persona that overrides that field, and
    /// is carried forward verbatim through every later re-wear.
    pub displaced: crate::character_runtime::DisplacedPhysicals,
}

/// Gate the raw player-control frame by the effective worn kit before any body
/// or effects system consumes it.
///
/// `ActionSet` already gates the generic message resolver, but several legacy
/// player-body paths still read `ActorControl` directly: the movement engine's
/// attack recoil/slash limb, bubble shield, and the chargeable projectile input.
/// Clearing those verbs here makes a peaceful authored persona peaceful in
/// behavior, not merely in its nominal `ActionSet`.
/// **The set `gate_worn_player_control` runs in.**
///
/// ⛔ `ambition_demo_sanic` BRACKETS this function by name — one `.before`, one
/// `.after` — reaching through the facade (`actors::avatar::…`) for an engine
/// leaf. Bracketing is the shape that makes a leaf pin hardest to remove later:
/// two edges that only make sense as a pair, expressed against a name neither
/// side owns.
///
/// ⚠ ONE member, and this one has the subtlest reason of the set family.
/// `gate_worn_player_control` sits inside `PlayerInputSet::ControlGate` with
/// `sustain_bubble_shield` chained after it, and that neighbour exists precisely
/// to run AFTER the gate ("after the gate, which keeps the persona's shield verb
/// alive"). A set spanning both would let a consumer's `.before(set)` land ahead
/// of a system the gate is supposed to feed.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WornControlGateSet;

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
    use ambition_characters::action_scheme::{derive_action_scheme, resolve_control_slots};
    use ambition_characters::actor::character_catalog::PlayableKitSource;
    use ambition_characters::brain::SpecialActionSpec;

    for (
        worn,
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

/// Raise (and sustain) a `"bubble_shield"` persona's guard through the ONE shield
/// path (`shield_held` → `resolve_shield`) — so pressing Special actually deploys
/// the bubble shield instead of playing a bare animation.
///
/// Two conditions force `shield_held`, and together they eliminate the one-tick
/// lag the folded special would otherwise have:
///
/// - **The press tick.** The special MOVE is triggered later in the tick
///   (`trigger_moveset_moves` runs in `Combat`, after `PlayerInput`/`WorldPrep`),
///   so on the tick Special is pressed there is no `MovePlayback` yet. Reading the
///   `special_pressed` edge here — in `PlayerInput`, before the `WorldPrep` kernel
///   bridge — raises the guard the SAME tick the button goes down.
/// - **The move's duration.** Once the move is playing, its `id` equals the body's
///   `ActionSet.special` key (that is how [`build_actor_moveset`] folds the marker
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
        &mut ambition_characters::brain::ActorControl,
    )>,
) {
    use ambition_characters::brain::SpecialActionSpec;
    for (actions, playback, mut control) in &mut bodies {
        let Some(SpecialActionSpec::Special(key)) = actions.special.as_ref() else {
            continue;
        };
        if key != "bubble_shield" {
            continue;
        }
        let pressed = control.0.special_pressed;
        let playing = playback.is_some_and(|p| p.spec.id == *key);
        if pressed || playing {
            control.0.shield_held = true;
        }
    }
}

#[cfg(test)]
mod tests;
