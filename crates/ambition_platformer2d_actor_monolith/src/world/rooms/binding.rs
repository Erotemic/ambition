//! Resolve `RoomSpec` references before construction mutates the world.
//!
//! Patrol paths, characters, and similar references that would otherwise fall
//! back are resolved in one pass and reported through `BindingReport`. References
//! whose own construction already refuses invalid ids remain with their owning
//! domain. Resolving the engine room IR rather than a specific authoring file
//! gives every content backend the same validation.

use ambition_platformer2d_shared_tangle::binding::{
    BindingLedger, BindingReport, Namespace, Ref, Resolver,
};

use ambition_platformer2d_world::rooms::RoomSpec;

/// The kinematic paths a room declares. Room-scoped: `patrol_a` in one room has
/// nothing to do with `patrol_a` in another.
pub struct KinematicPathId;

impl Namespace for KinematicPathId {
    const NAME: &'static str = "kinematic path";
}

/// The character archetypes a catalog knows.
pub struct CharacterId;

impl Namespace for CharacterId {
    const NAME: &'static str = "character";
}

/// The held-item specs the item registry knows.
///
/// Not swept here. A ground item naming an unregistered held item does not
/// degrade — `authored_ground_item_requests` refuses the room outright — so the
/// namespace exists for that refusal to name what WAS available, and reporting
/// it a second time here would only disagree with it.
pub struct HeldItemId;

impl Namespace for HeldItemId {
    const NAME: &'static str = "held item";
}

/// The resolvers a room sweep has on hand.
///
/// Paths come from the room itself, so they are always checked, and
/// [`RoomBindings::checked`] says which namespaces were — "we did not look" must
/// not read like "we looked and it was fine", which is the whole failure this
/// boundary exists to prevent.
///
/// THE CHARACTER NAMESPACE LEFT WITH THE ARCHETYPE ROSTER (AC6). It resolved each
/// `EnemySpawn`'s BRAIN KEY against the roster's keys, because the lookup behind that key could
/// not fail: a misspelling became the generic `combatant` body wearing the right name, and this
/// sweep was the only place that could see it.
#[derive(Default)]
pub struct RoomBindings;

impl RoomBindings {
    /// Which namespaces this sweep can actually decide, in report order. A caller
    /// that wants full coverage asserts on this rather than on an empty report.
    pub fn checked(&self) -> Vec<&'static str> {
        vec![KinematicPathId::NAME]
    }

    /// Resolve every reference `room` declares that this sweep can decide.
    ///
    /// One pass, one report: a room with a bad patrol path AND a bad character
    /// archetype names both, so fixing content is one edit session rather than a
    /// sequence of run-crash-fix cycles.
    pub fn sweep(&self, room: &RoomSpec) -> BindingReport {
        let mut ledger = BindingLedger::new();
        let paths = room_paths(room);
        // Two paths answering to one spelling is not a resolution failure — the
        // first one wins — but the second is unreachable and its author does not
        // know that.
        ledger.note_duplicates(&paths, format!("room `{}` paths", room.id));

        for enemy in &room.enemy_spawns {
            match &enemy.payload.brain {
                ambition_entity_catalog::placements::CharacterBrain::Patrol {
                    path_id: Some(path_id),
                } => {
                    ledger.resolve(
                        &paths,
                        &Ref::new(path_id),
                        format!("patrol brain of `{}`", enemy.id),
                    );
                }
                _ => {}
            }
        }

        // THE ENEMY BRAIN WAS NEVER THE ONLY REFERENCE THAT SHRUGS. Three
        // roads resolve a path id against this same table by string equality and
        // all three fall through to `None` in silence — an enemy's patrol brain,
        // an NPC placement's `patrol_path_id`, and a hazard's motion `path_id`.
        // Only the first was swept, so the module's own thesis ("only references
        // that shrug belong here") was two-thirds unimplemented, and the NPC case
        // is the quietest of the three: an NPC with an unresolvable path still
        // patrols its home±radius lane, so it moves, just not along the waypoints
        // somebody drew.
        for placement in &room.placements {
            match &placement.schema {
                ambition_entity_catalog::placements::PlacementSchema::Interactable(
                    interactable,
                ) => {
                    if let ambition_entity_catalog::placements::InteractionKindSpec::Npc {
                        patrol_path_id: Some(path_id),
                        ..
                    } = &interactable.kind
                    {
                        // resolved UNTRIMMED, because the NPC lowering road
                        // compares untrimmed (the enemy road trims at conversion,
                        // the hazard road trims at lookup). Predicting the runtime
                        // matters more than being tidy: trimming here would call a
                        // padded id healthy while the body found nothing. Blank is
                        // skipped — that is "no path authored", not a typo.
                        if !path_id.trim().is_empty() {
                            ledger.resolve(
                                &paths,
                                &Ref::new(path_id),
                                format!("npc patrol of `{}`", placement.id.as_str()),
                            );
                        }
                    }
                }
                ambition_entity_catalog::placements::PlacementSchema::Hazard(hazard) => {
                    // The hazard road trims before looking up, so resolve trimmed.
                    if let Some(path_id) = hazard
                        .path_id
                        .as_deref()
                        .map(str::trim)
                        .filter(|path_id| !path_id.is_empty())
                    {
                        ledger.resolve(
                            &paths,
                            &Ref::new(path_id),
                            format!("motion path of hazard `{}`", placement.id.as_str()),
                        );
                    }
                }
                _ => {}
            }
        }

        ledger.finish()
    }
}

/// The room's paths, under every spelling each answers to.
///
/// `KinematicPathSpec::matches_id` also accepts the normalized display-name
/// slug, so the resolver must consume the exact same alias set — otherwise this
/// sweep reports references that the runtime resolves perfectly well, which is
/// worse than not checking.
fn room_paths(room: &RoomSpec) -> Resolver<KinematicPathId> {
    Resolver::with_aliases(
        room.kinematic_paths
            .iter()
            .enumerate()
            .flat_map(|(slot, path)| {
                path.resolution_aliases()
                    .map(move |alias| (alias.into_owned(), slot))
            }),
    )
}
