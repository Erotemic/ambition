//! Verify stable identities on damageable bodies.
//!
//! A damageable body has both `CenteredAabb` and `ActorFaction`. Some
//! construction paths mint `SimId` later in the same simulation tick.
//! This observer excludes bodies that became damageable during that tick,
//! then counts any still missing an identity. It observes all such bodies,
//! not only the ones a test fixture knows how to construct.
//! The census reports the problem without stopping the simulation.

use ambition_combat::components::CenteredAabb;
use ambition_characters::actor::ActorFaction;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;

/// Counts for damageable bodies checked after their identity-minting tick.
/// The `observed` count prevents an empty population from appearing clean.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct BodyIdentityCensus {
    /// Damageable bodies STILL carrying no `SimId` a tick after they became
    /// damageable — i.e. after the designated late-mint road has had both of its
    /// turns. **Not "unidentified at insertion", which counts the player.**
    pub unidentified: u64,
    /// Damageable-body observations that were actually JUDGED (a body that became
    /// damageable this tick is skipped, so it is not in here either). **Read this
    /// first**: it is the denominator, and zero means the observer saw nothing
    /// rather than that the tree is clean.
    pub observed: u64,
    /// The first observed body without identity, for headless diagnostics.
    pub first_unidentified: Option<String>,
}

/// Observe every damageable body that has had a tick to be identified.
///
/// ⚠ THE `Added` PAIR IS THE SUBTLE PART, and it is used to EXCLUDE rather than
/// to select. `CenteredAabb` and `ActorFaction` arrive from SEPARATE inserts —
/// the row's own reason a bundle-shaped static scan cannot answer this — so
/// neither `Added` alone is the moment a body becomes damageable.
/// `Or<(Added<A>, Added<B>)>` with `With<>` on both identifies the bodies that
/// became damageable THIS tick, and those are the ones the designated late-mint
/// road has not had its turn on yet.
pub fn observe_damageable_body_identity(
    mut census: ResMut<BodyIdentityCensus>,
    bodies: Query<(Entity, Option<&SimId>, Option<&Name>), (With<CenteredAabb>, With<ActorFaction>)>,
    became_damageable_this_tick: Query<
        Entity,
        (
            With<CenteredAabb>,
            With<ActorFaction>,
            Or<(Added<CenteredAabb>, Added<ActorFaction>)>,
        ),
    >,
) {
    for (entity, sim_id, name) in &bodies {
        // Not yet judged: `ensure_sim_id` runs within this same tick, twice, and
        // a body it is about to serve is not a body without identity.
        if became_damageable_this_tick.contains(entity) {
            continue;
        }
        census.observed += 1;
        if sim_id.is_none() {
            census.unidentified += 1;
            if census.first_unidentified.is_none() {
                census.first_unidentified = Some(format!(
                    "{entity} ({})",
                    name.map_or("no Name either", |name| name.as_str())
                ));
            }
            // ⚠ NAMED, not counted only. A census that says "one" and cannot say
            // WHICH sends the reader to re-derive the population by hand.
            error!(
                "a damageable body is STILL unidentified a tick after it became \
                 damageable: {entity} ({}). `ensure_sim_id` has had both of its \
                 turns and did not serve it, so no authored fact names it. The \
                 contact protocol calls a missing target identity a CONSTRUCTION \
                 failure rather than a sort fallback — whatever built this has to \
                 mint one. The projectile resolver's own `debug_assert` cannot \
                 see it: that flags only a COINCIDENT pair, so a lone \
                 unidentified body passes it in silence.",
                name.map_or("no Name either", |name| name.as_str())
            );
        }
    }
}

/// Install the census against the schedule the caller owns.
///
/// ⚠ The SCHEDULE stays the caller's: `app.sim_schedule()` differs by host, the
/// same reason `install_dismounted_rider_rebuild` takes one.
pub fn install_body_identity_census(
    app: &mut bevy::prelude::App,
    schedule: impl bevy::ecs::schedule::ScheduleLabel,
) {
    app.init_resource::<BodyIdentityCensus>();
    app.add_systems(schedule, observe_damageable_body_identity);
}

#[cfg(test)]
mod the_census_sees_a_body_become_damageable {
    use super::*;

    /// ⚠ RETURNS ITS OWN SUBJECT. A first version computed the expected entity
    /// id by building a SECOND app — ids are not stable across worlds, and it
    /// failed comparing `8v0` to `7v0`. The fixture has to hand back the thing
    /// it made.
    fn census_after(with_identity: bool) -> (BodyIdentityCensus, Entity) {
        let mut app = App::new();
        app.init_resource::<BodyIdentityCensus>();
        app.add_systems(Update, observe_damageable_body_identity);

        let mut body = app.world_mut().spawn(CenteredAabb::new(Vec2::ZERO, Vec2::splat(16.0)));
        if with_identity {
            body.insert(SimId::placement("census_subject"));
        }
        let body = body.id();
        // ⭐ THE SECOND HALF LANDS ON A LATER TICK, on purpose: this is the
        // shape the row says a static scan cannot see, and the filter has to
        // fire on the tick the pair COMPLETES rather than on either insert.
        app.update();
        app.world_mut().entity_mut(body).insert(ActorFaction::Enemy);
        // ⚠ TWO UPDATES AFTER THE PAIR COMPLETES, not one. The tick a body
        // becomes damageable is the tick the engine's designated late-mint road
        // still gets to serve it, so the observer SKIPS it; the judgement is on
        // the next tick. A fixture that reads after one update measures the skip.
        app.update();
        app.update();
        (app.world().resource::<BodyIdentityCensus>().clone(), body)
    }

    /// **A body that completes its damageable pair WITH an identity is observed
    /// and not flagged.**
    ///
    /// ⚠ THE EDIT THAT MAKES THIS FALSE is flagging on presence rather than
    /// absence, or counting a body twice.
    #[test]
    fn an_identified_body_is_observed_and_not_flagged() {
        assert_eq!(
            census_after(true).0,
            BodyIdentityCensus {
                unidentified: 0,
                observed: 1,
                first_unidentified: None
            },
            "an identified body must be OBSERVED (so the floor is real) and not \
             flagged"
        );
    }

    /// **And one WITHOUT an identity is flagged**, which is the arm that stops
    /// the one above from passing on a system that never runs.
    ///
    /// ⚠ THE EDIT THAT MAKES THIS FALSE is `return`ing from the system, or
    /// narrowing the filter to a single `Added` — the halves land on different
    /// ticks here precisely so that a single-`Added` filter misses the pair.
    #[test]
    fn an_unidentified_body_is_flagged_when_its_pair_completes() {
        let (census, subject) = census_after(false);
        assert_eq!(
            census,
            BodyIdentityCensus {
                unidentified: 1,
                observed: 1,
                first_unidentified: Some(format!("{subject} (no Name either)"))
            },
            "a body whose damageable pair completed with no `SimId` must be \
             flagged; if this reads 0 observed, the filter never fired and the \
             guard beside it proves nothing"
        );
    }
}
