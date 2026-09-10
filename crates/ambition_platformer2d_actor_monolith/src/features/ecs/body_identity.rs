//! ⛔⛔ **A DAMAGEABLE BODY THAT REACHES THE WORLD WITHOUT A STABLE IDENTITY IS
//! A CONSTRUCTION FAILURE, AND UNTIL NOW NOTHING SAID SO.**
//!
//! The projectile contact protocol calls a missing target identity a
//! construction/verification failure rather than a sort fallback — so the
//! invariant belongs where bodies are BUILT, and the resolver can only report
//! it. `construction/mod.rs` mints `SimId::placement(..)` for enemies, bosses,
//! giants, hands, shrines, riders and summons; **what nothing established is
//! whether any damageable body arrives without one.**
//!
//! ⚠ **WHY THIS IS A SYSTEM AND NOT A TEST, and it is the whole design.** A test
//! can only census the roads its fixture drives —
//! `every_damageable_body_is_identified` drives three of the seven and says so.
//! **This observes the INSERTION, so it is exhaustive by construction rather
//! than by discipline**: a road nobody thought to drive still passes through
//! here, and a road added next year does too. There is no list to keep in sync.
//! Same shape as `canonical_f32_bits`' non-finite observer, which this borrows
//! deliberately: correct a place that already sees every case, rather than
//! building a second walk beside it.
//!
//! ⚠ **IT COUNTS RATHER THAN PANICS, AND THE REASON IS NOT TIMIDITY.** A body
//! could in principle be damageable for a frame before its identity lands, and a
//! panic would make that a crash rather than a finding. **MEASURED 2026-09-10
//! before this was written: zero unidentified across 120 frame-samples in a
//! sandbox that drove the placement, enemy and boss roads — `SimId` arrives WITH
//! the body, never after.** So the counter is expected to stay at zero, and the
//! test arm that reads it is the assertion. If a future road does mint late, this
//! reports it instead of taking the game down.

use ambition_combat::components::{ActorFaction, CenteredAabb};
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;

/// Damageable bodies observed without a [`SimId`], and the total observed.
///
/// ⭐ **BOTH NUMBERS, because the offender count alone cannot tell a clean run
/// from an unobserved one.** A composition that never installs this system, or a
/// run in which no body is ever built, reports zero offenders and reads exactly
/// like a healthy one. `observed` is the anti-vacuity floor a reader checks
/// first.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct BodyIdentityCensus {
    /// Damageable bodies that became damageable with no `SimId`.
    pub unidentified: u64,
    /// Damageable bodies observed at all. **Read this first.**
    pub observed: u64,
    /// ⛔⛔ WHO, not just how many — and the first version of this resource
    /// carried only the count while its own comment said *"a census that says
    /// 'one' and cannot say WHICH sends the reader to re-derive the population
    /// by hand"*. It named the offender through `error!`, into a log the sim
    /// harness does not install a subscriber for, **so the name went nowhere and
    /// the count arrived alone.** A report whose only channel is a log is a
    /// report a headless caller cannot read.
    pub first_unidentified: Option<String>,
}

/// Observe every body as it BECOMES damageable.
///
/// ⚠ THE FILTER IS THE SUBTLE PART. `CenteredAabb` and `ActorFaction` arrive from
/// SEPARATE inserts — that is the row's own reason a bundle-shaped static scan
/// cannot answer this — so neither `Added` alone is the moment a body becomes
/// damageable. `Or<(Added<A>, Added<B>)>` with `With<>` on both fires on the tick
/// the SECOND one lands, whichever that is, and fires once.
pub fn observe_damageable_body_identity(
    mut census: ResMut<BodyIdentityCensus>,
    bodies: Query<
        (Entity, Option<&SimId>, Option<&Name>),
        (
            With<CenteredAabb>,
            With<ActorFaction>,
            Or<(Added<CenteredAabb>, Added<ActorFaction>)>,
        ),
    >,
) {
    for (entity, sim_id, name) in &bodies {
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
                "a damageable body reached the world with no `SimId`: {entity} \
                 ({}). The contact protocol calls a missing target identity a \
                 CONSTRUCTION failure rather than a sort fallback — whatever \
                 built this has to mint one. The projectile resolver's own \
                 `debug_assert` cannot see it: that flags only a COINCIDENT \
                 pair, so a lone unidentified body passes it in silence.",
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
