//! Apply worn character mechanics to live bodies.

use bevy::ecs::system::{Commands, Local, Query};
use bevy::prelude::{Component, Entity, Has, Name, Res, With};

use ambition_characters::actor::WornCharacter;
use ambition_characters::brain::{ActionSet, RangedExecution};

use ambition_body_seed::PersonaBaseline;
use ambition_combat::moveset::ActorMoveset;
use ambition_combat::worn_kit::WornKit;
use ambition_platformer2d_core::movement::MotionModel;






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

    /// The home body's base set when it wears a character whose prepared
    /// definition authors `authored`: that set, else every intrinsic verb but
    /// Morph Ball, then this experience's grants and permits over it. The one
    /// answer for a body that is built and for a body that changes form, so a
    /// form brings its own abilities (Q159) and the experience's grants (Morph
    /// Ball in Ambition) stay.
    pub fn base_for(&self, authored: Option<ambition_platformer2d_core::AbilitySet>) -> ambition_platformer2d_core::AbilitySet {
        self.apply(authored.unwrap_or(ambition_platformer2d_core::AbilitySet {
            // An unauthored body gets every intrinsic verb. Morph Ball is
            // progression an experience grants, so a fallback never carries it.
            morph: false,
            ..ambition_platformer2d_core::AbilitySet::sandbox_all()
        }))
    }
}



// NOTE: the old `overlay_character_moveset` fallback — empty worn slots kept the player's
// swipe/bolt/shield — is GONE, and being the content default no longer implies "keep the
// host's hardcoded kit": the row's authored kit wins.
//
// ⛔⛤ **A ROW CANNOT OPT INTO THE HOST KIT BY NAME, AND THIS SAID IT COULD**
// until 2026-09-19 — it told an author to mark the row `PlayableKitSource::HostCode`,
// a spelling `prepared.rs` deleted once the enum turned out to hold one variant
// and to be answering catalog MEMBERSHIP all along. An instruction nobody can
// carry out is worse than a missing one, because it reads as a supported route.
// An id the prepared cast does not hold has no kit and is not worn (Q103): see
// `wear_character`.

/// The movement policy for `character_id`: its prepared definition's.
///
/// The barrier prepares every catalog row (AP30) and folds the row's motion
/// model into it, so the registry answers for every character the catalog
/// knows. An id it does not hold gets the default axis-swept model.
pub fn motion_model_spec_for_character(
    registry: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    character_id: &str,
) -> ambition_platformer2d_core::MotionModelSpec {
    registry
        .and_then(|registry| registry.get(character_id))
        .map_or_else(
            || ambition_platformer2d_core::MotionModelSpec::AxisSwept(Default::default()),
            |prepared| prepared.motion_model,
        )
}

/// The movement FEEL for `character_id`: its prepared definition's.
///
/// `None` is not a default here, it is an ANSWER. The marker component's
/// presence means "this body's tuning is authored rather than the shared dev
/// tuning", so a character that authored none must produce `None` and have the
/// marker REMOVED — otherwise a re-wear from an authored feel back to the
/// sandbox protagonist never returns the body to the live inspector sliders.
pub fn movement_tuning_for_character(
    registry: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    character_id: &str,
) -> Option<ambition_platformer2d_core::MovementTuning> {
    registry?.get(character_id)?.movement_tuning
}

/// Apply the worn character's movement identity to an already-spawned body.
///
/// Every movable body already carries one explicit model. This operation only
/// changes that policy; it never removes the component or uses absence as an
/// axis-swept sentinel.
pub fn apply_worn_motion_model(
    registry: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    commands: &mut Commands,
    entity: Entity,
    character_id: &str,
) {
    let mut model = MotionModel::default();
    model.apply_spec(motion_model_spec_for_character(registry, character_id));
    commands.entity(entity).insert(model);
}

/// Synchronize movement identity without discarding live solver state when the
/// selected policy is unchanged. A same-model refresh updates only parameters;
/// a cross-model transition preserves every shared body fact and initializes
/// ONLY destination-private state — through the one kernel transition seam.
fn sync_worn_motion_model_preserving_state(
    registry: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    character_id: &str,
    current: &mut MotionModel,
) {
    ambition_platformer2d_core::switch_motion_model(
        current,
        motion_model_spec_for_character(registry, character_id),
    );
}

/// The gameplay overlay a body derives from wearing `character_id`.
///
/// This is the single resolver used by both spawn and runtime re-wear. Every
/// field it writes is a deterministic function of the identity (and, for a
/// seat, the match's kit) — never of the body's abilities or its prior kit.
/// See [`WornKit::of`] for the arms.
///
/// Returns HOW the resolved persona fires ([`RangedExecution`]); the ECS derive
/// system synchronizes the charge marker and its mutable state from that.
/// `None` when the cast does not hold `character_id`: nothing is written (see
/// [`wear_character`]).
pub fn apply_worn_character_overlay(
    registry: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    name: &mut Name,
    action_set: &mut ActionSet,
    moveset: &mut ActorMoveset,
    identity: &mut ambition_characters::brain::action_set::IdentityKit,
    character_id: &str,
    terms: ambition_combat::worn_kit::SeatTerms<'_>,
) -> Option<RangedExecution> {
    let execution = wear_character(registry, name, identity, character_id, terms)?;
    // Construction: nothing is worn or held yet, so the live pair is the
    // identity's own fold, published with it.
    let live = ambition_characters::repertoire::effective_repertoire(
        identity,
        None,
        ambition_characters::repertoire::Hand::Empty,
    );
    *action_set = live.action_set;
    *moveset = ActorMoveset(live.moveset);
    Some(execution)
}

/// Put `character_id` on a body: its display name and its identity baseline.
///
/// Writes only the fold's INPUT. The live `ActionSet` + `ActorMoveset` are
/// `reconcile_effective_repertoire`'s to derive from this baseline together
/// with worn equipment and the hand, so a body re-wearing a character while
/// holding something is never published empty-handed.
///
/// ⛔ `None` WHEN THE CAST DOES NOT HOLD `character_id` (Q103): nothing is
/// written, and the caller reports the refusal. A character admitted into
/// simulation was prepared for that generation; there is no default kit and
/// no name made from the id.
pub fn wear_character(
    registry: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    name: &mut Name,
    identity: &mut ambition_characters::brain::action_set::IdentityKit,
    character_id: &str,
    terms: ambition_combat::worn_kit::SeatTerms<'_>,
) -> Option<RangedExecution> {
    let prepared = registry?.get(character_id)?;
    let kit = WornKit::of(prepared, terms);
    *name = Name::new(prepared.display_name.clone());
    *identity = kit.identity;
    Some(kit.execution)
}

/// What the match says about the kit this seat wears, or the terms of a body
/// in no match.
fn seat_terms_for<'a>(
    roster: Option<&'a ambition_match::MatchParticipantRoster>,
    seat: Option<&ambition_match::MatchSeat>,
) -> ambition_combat::worn_kit::SeatTerms<'a> {
    let (Some(roster), Some(seat)) = (roster, seat) else {
        return ambition_combat::worn_kit::SeatTerms::default();
    };
    let action_set = roster
        .participants
        .get(seat.0)
        .and_then(|participant| participant.action_set.as_ref());
    roster.rules.seat_terms(action_set)
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
/// kit, projectile capability, and movement identity, all read from the
/// prepared cast. In particular, an authored Sanic keeps the persistent `MomentumMotion.state`
/// it accumulated while riding a surface.
pub fn apply_worn_character_gameplay(
    // The running session's frozen cast (`SessionCast`), not the published
    // one: a reload publishes the App cast before the session that runs it is
    // activated, and a body re-derived from it in that window plays a cast its
    // session was not prepared against. MEASURED 2026-10-01: with the reload's
    // claim emptied, the frozen cast said 5 HP, the App said 9, and the live
    // goblins had 9 until this read the frozen cast.
    cast: ambition_characters::prepared::SessionCast,
    // What the MATCH decided, when one is running. `Option` because most
    // compositions are not a match, which is the ordinary case rather than a
    // degraded one.
    roster: Option<Res<ambition_match::MatchParticipantRoster>>,
    // What the running experience grants a home body, so a home body that
    // changes form gets the new form's abilities under the same grants.
    homes: Query<(&ambition_platformer2d_shared_tangle::lifecycle::SessionRoot, &HomeBodyAbilities)>,
    active: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
    mut commands: Commands,
    // The refusals already reported for a body that stays stale: such a body
    // is asked again every tick, and it is reported once for each generation.
    // Only the log reads this.
    mut reported: Local<std::collections::HashSet<(Entity, ambition_characters::prepared::CharacterCatalogGeneration)>>,
    // The repertoire-bearing bodies: the live pair this system does not write
    // is the fold's, and a body without it has nothing for the fold to publish.
    mut worn: Query<
        (
        Entity,
        // Written only to put back the previous character when a wear is
        // refused.
        &mut WornCharacter,
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
            // A home body's base abilities, rewritten at a change of form.
            Option<&mut ambition_platformer2d_core::AbilityBase>,
            Has<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
        ),
        ),
        (With<ActionSet>, With<ActorMoveset>),
    >,
) {
    let registry = cast.get();
    let home = active
        .as_deref()
        .and_then(ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope::current)
        .and_then(|session| homes.iter().find(|(root, _)| root.0 == session))
        .map(|(_, home)| *home);
    for (
        entity,
        mut character,
        mut name,
        mut identity,
        mut motion_model,
        mut health,
        mass,
        mut combat_tuning,
        has_projectile_state,
        (baseline, seat, recharacterize, ability_base, is_home_body),
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
            let Some(execution) = wear_character(
                registry,
                &mut name,
                &mut identity,
                id,
                // The terms this MATCH gave the seat, when this body is in one:
                // its borrowed kit and which damage its moves deal. A body with
                // no `MatchSeat` is not in a match and keeps its authored
                // persona, which is every other body in every game.
                seat_terms_for(roster.as_deref(), seat),
            ) else {
                // ⛔ REFUSED (Q103): the cast this session was prepared with
                // does not hold the id. Nothing of the body changes, and it
                // wears again the character last applied to it
                // (`PersonaBaseline::id`), so its id and its kit stay one
                // answer. A body with no baseline keeps the id it was built
                // with, and is asked again when a cast holds it.
                let previous = baseline
                    .map(|baseline| baseline.id.as_str())
                    .filter(|previous| *previous != id);
                if recharacterize || reported.insert((entity, generation)) {
                    bevy::log::error!(
                        "body {entity:?} does not wear '{id}': it is not a character of the \
                         prepared cast; it keeps {}",
                        previous.map_or_else(|| "what it has".to_string(), |previous| format!("'{previous}'")),
                    );
                }
                if let Some(previous) = previous {
                    *character = WornCharacter::new(previous);
                }
                continue;
            };
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
            // THE FORM'S ABILITIES (Q159): a home body is built with the base
            // `HomeBodyAbilities::base_for` gives, and a change of form gives it
            // again for the new form. A match seat's abilities are the match's,
            // and what progression lends rides `AbilityContributions`, so
            // neither moves here.
            if let (true, None, Some(home), Some(mut base)) = (is_home_body, seat, home, ability_base) {
                let abilities = home.base_for(registry.as_deref().and_then(|registry| registry.get(id)).and_then(|prepared| prepared.abilities));
                if base.abilities != abilities {
                    base.abilities = abilities;
                }
            }

            // Movement identity is identity-derived, not ability-derived. Only
            // a wear/re-wear may replace the model; doing this for a live
            // ability edit would reset SurfaceMomentum's persistent riding
            // state to Airborne.
            sync_worn_motion_model_preserving_state(
                registry,
                id,
                &mut motion_model,
            );

            // Per-character axis FEEL rides a marker component: presence means
            // "this body's tuning is authored, not the shared F3 dev tuning".
            // Insert it when the worn identity authors a tuning, remove it when
            // it does not — so a re-wear from an authored feel back to the
            // sandbox protagonist returns the body to the live inspector sliders.
            match movement_tuning_for_character(registry, id) {
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
                .zip(registry)
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
/// way to raise the ONE shield. Runs after [`crate::control::gate_body_control`], which
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
        if key != ambition_characters::brain::action_set::BUBBLE_SHIELD_SPECIAL {
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
