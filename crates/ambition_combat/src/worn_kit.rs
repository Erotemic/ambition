//! The kit a body wears: what a character id resolves to when a body puts it on.
//!
//! One resolver for spawn, runtime re-wear and match seating, so no two roads can
//! disagree about what a character IS. Every field is a deterministic function of
//! the identity, the body's `AbilitySet`, and (only for a seated fighter) the
//! stage's borrowed repertoire — never of the body's prior kit.
//!
//! This is content compilation, and it used to live in the actor kernel's
//! `avatar` module. The kernel now consumes the [`WornKit`] value and writes it
//! onto components; it no longer decides what a character's kit is.

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::brain::action_set::IdentityKit;
use ambition_characters::brain::{ActionSet, RangedExecution};
use ambition_characters::prepared::{
    overlay_authored_moves, PreparedCharacterRegistry, PreparedKit,
};
use ambition_entity_catalog::MovesetContract;

use crate::components::CombatKit;
use crate::moveset::{apply_player_robot_slash_sfx, build_actor_moveset};

/// What a body carries once it wears a character.
#[derive(Clone, Debug)]
pub struct WornKit {
    /// The prepared display name, else the catalog's, else the id itself — an
    /// unknown id is shown as the id so the problem is visible.
    pub display_name: String,
    pub action_set: ActionSet,
    pub moveset: MovesetContract,
    /// The un-granted baseline the brain reads: the action set and the moveset
    /// it was built beside, before equipment and granted verbs overlay them.
    pub identity: IdentityKit,
    /// The durable innate baseline capabilities are reconstructed from. Built
    /// from the same action set, so the two can never disagree.
    pub combat_kit: CombatKit,
    /// HOW the resolved persona fires. The kernel's ECS derive synchronizes the
    /// charge marker and its mutable state from this.
    pub execution: RangedExecution,
}

impl WornKit {
    /// Resolve the kit `character_id` puts on a body whose own capabilities are
    /// `base_abilities`.
    ///
    /// - a prepared `Authored` row: its action set, narrowed by `gated_by` to
    ///   what is unlocked, with the moveset preparation derived;
    /// - a prepared `Unauthored` row: the host-code kit built from the body,
    ///   firing through the charge path;
    /// - an unprepared catalog row: the catalog's default action set, or a safe
    ///   peaceful kit when that row's preset does not resolve (malformed
    ///   content is reported, never promoted to host privileges);
    /// - an unknown id: the host-code compatibility kit.
    ///
    /// A MATCH OUTRANKS THE PERSONA, and only a match: `match_kit` is a rule of
    /// the stage the fighter stands on, not another opinion about who the
    /// character is, so it replaces the action set outright. How the borrower
    /// FIRES is still the character's own fact — a robot seated with a stage's
    /// generic set still charges if the robot charges — and a character's own
    /// authored timelines still overlay the borrowed set's derived moves.
    pub fn resolve(
        catalog: &CharacterCatalog,
        registry: Option<&PreparedCharacterRegistry>,
        character_id: &str,
        base_abilities: ambition_platformer2d_core::AbilitySet,
        match_kit: Option<&ActionSet>,
    ) -> Self {
        let prepared = registry.and_then(|registry| registry.get(character_id));
        let display_name = prepared
            .map(|prepared| prepared.display_name.as_str())
            .or_else(|| catalog.display_name(character_id))
            .unwrap_or(character_id)
            .to_string();

        let (set, derived, execution) = if let Some(kit) = match_kit {
            let execution = prepared.map_or(RangedExecution::MovesetVerb, |prepared| {
                prepared.ranged_execution
            });
            let authored = prepared.and_then(|prepared| prepared.authored_moveset.clone());
            let derived = derive_persona_moveset(kit, execution, authored);
            (kit.clone(), derived, execution)
        } else {
            match prepared.map(|prepared| &prepared.kit) {
                Some(PreparedKit::Authored {
                    action_set,
                    moveset,
                }) => (
                    action_set.gated_by(base_abilities),
                    moveset.clone(),
                    prepared.map_or(RangedExecution::MovesetVerb, |prepared| {
                        prepared.ranged_execution
                    }),
                ),
                Some(PreparedKit::Unauthored { authored_moveset }) => {
                    let set = default_player_action_set(base_abilities);
                    let execution = RangedExecution::ChargedProjectile;
                    let derived = derive_persona_moveset(&set, execution, authored_moveset.clone());
                    (set, derived, execution)
                }
                None => {
                    let catalog_knows_it = catalog.knows(character_id);
                    let authored = catalog.build_default_action_set(character_id);
                    if catalog_knows_it && authored.is_none() {
                        bevy::log::error!(
                            "worn character '{character_id}' has a catalog row whose \
                             default_action_set does not resolve; installing a safe peaceful kit"
                        );
                    } else if !catalog_knows_it {
                        bevy::log::warn_once!(
                            "worn character id '{character_id}' is not in the catalog; wearing \
                             the code-side compatibility kit and showing the id as the display \
                             name"
                        );
                    }
                    let (set, execution) =
                        resolve_playable_action_set(catalog_knows_it, authored, base_abilities);
                    let derived = derive_persona_moveset(&set, execution, None);
                    (set, derived, execution)
                }
            }
        };
        Self {
            display_name,
            combat_kit: CombatKit::from_action_set(&set),
            identity: IdentityKit::of(set.clone(), derived.clone()),
            moveset: derived,
            action_set: set,
            execution,
        }
    }
}

/// Derive a persona's moves from its action set, given HOW it fires.
///
/// Under `ChargedProjectile` the charge mechanic already owns the ranged press,
/// so the ranged preset contributes no move and the kit wears the robot blade's
/// sound family; under `MovesetVerb` the ranged preset IS the ranged verb. The
/// `authored` contract overlays the derivation ([`overlay_authored_moves`]) —
/// after the blade stamp, so authored cues are never retargeted.
///
/// `pub` for fixtures: a body's swing is built HERE, at spawn, from its action
/// set, so a harness that mutates `ActionSet.melee` afterwards changes nothing
/// the runtime reads.
pub fn derive_persona_moveset(
    set: &ActionSet,
    execution: RangedExecution,
    authored: Option<MovesetContract>,
) -> MovesetContract {
    let (ranged, special) = match execution {
        RangedExecution::ChargedProjectile => (None, set.special.as_ref()),
        RangedExecution::MovesetVerb => (set.ranged.as_ref(), set.special.as_ref()),
    };
    let mut derived =
        build_actor_moveset(None, set.melee.as_ref(), ranged, special).unwrap_or_default();
    if execution.charges_projectiles() {
        apply_player_robot_slash_sfx(&mut derived);
    }
    overlay_authored_moves(derived, authored)
}

/// Resolve a playable action set for an id the prepared registry does not hold.
///
/// A known row whose preset does not resolve is malformed content: the startup
/// validator reports it, and the runtime stays peaceful rather than granting the
/// host kit. Only an UNKNOWN id gets the compatibility fallback — a defined
/// answer for an id nobody wrote down.
pub fn resolve_playable_action_set(
    catalog_knows_it: bool,
    authored: Option<ActionSet>,
    base_abilities: ambition_platformer2d_core::AbilitySet,
) -> (ActionSet, RangedExecution) {
    if catalog_knows_it {
        (
            authored.unwrap_or_else(ActionSet::peaceful),
            RangedExecution::MovesetVerb,
        )
    } else {
        (
            default_player_action_set(base_abilities),
            RangedExecution::ChargedProjectile,
        )
    }
}

/// The host-code action set, derived from a body's `AbilitySet`:
///
/// - `melee = Some(Swipe)` iff `abilities.attack` — with NO windup: the hand the
///   player is holding comes out on the press, unlike a Striker it is meant to
///   read;
/// - `ranged = Some(bolt)` always — the fireball path is itself gated by
///   `projectile`, and there is no separate ability flag for it;
/// - `special = Some(Special("bubble_shield"))` iff `abilities.shield`.
///
/// The resolver emits no request for a capability the body lacks, so effects
/// consumers can read the set as "what this body can actually do right now".
pub fn default_player_action_set(abilities: ambition_platformer2d_core::AbilitySet) -> ActionSet {
    use ambition_characters::brain::{
        MeleeActionSpec, MoveStyleSpec, RangedActionSpec, SpecialActionSpec, SwipeSpec,
    };
    ActionSet {
        melee: abilities
            .attack
            .then_some(MeleeActionSpec::Swipe(SwipeSpec {
                windup_s: 0.0,
                active_s: 0.10,
                recover_s: 0.18,
                damage: 1,
                reach_px: 36.0,
            })),
        ranged: Some(RangedActionSpec::bolt(600.0, 1)),
        move_style: MoveStyleSpec::Walk,
        special: abilities
            .shield
            .then_some(SpecialActionSpec::Special("bubble_shield".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A typo in a known catalog row is content corruption, not permission to
    /// gain the host protagonist's code kit: the fallback is deliberately inert.
    #[test]
    fn malformed_authored_resolution_is_safe_peaceful_not_host_code() {
        let (set, execution) = resolve_playable_action_set(
            true,
            None,
            ambition_platformer2d_core::AbilitySet::sandbox_all(),
        );
        assert!(set.melee.is_none());
        assert!(set.ranged.is_none());
        assert!(set.special.is_none());
        assert_eq!(execution, RangedExecution::MovesetVerb);
    }

    /// An id nobody wrote down wears the host kit and is shown as itself.
    #[test]
    fn an_unknown_id_wears_the_host_kit_under_its_own_name() {
        let catalog = CharacterCatalog::empty();
        let kit = WornKit::resolve(
            &catalog,
            None,
            "nobody",
            ambition_platformer2d_core::AbilitySet::sandbox_all(),
            None,
        );
        assert_eq!(kit.display_name, "nobody");
        assert_eq!(kit.execution, RangedExecution::ChargedProjectile);
        assert!(kit.action_set.melee.is_some());
        // The charge path owns the ranged press: no ranged move is derived.
        assert!(!kit
            .moveset
            .verbs
            .contains_key(ambition_entity_catalog::RANGED_VERB));
        assert_eq!(kit.combat_kit, CombatKit::from_action_set(&kit.action_set));
    }
}
