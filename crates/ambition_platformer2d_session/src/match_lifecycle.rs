//! Installation of match construction and lifecycle systems.

use bevy::prelude::*;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

/// Install match activation, elapsed time, and object retirement.
pub struct MatchLifecyclePlugin;

impl Plugin for MatchLifecyclePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.init_resource::<ambition_match::live_match_clock::LiveMatchTicks>()
            .init_resource::<ambition_match::seating::SessionMatchOrdinal>()
            .add_systems(
                sim,
                // Before the character projection in the same phase, so a fighter seated this tick
                // wears its moveset and silhouette on the tick it appears rather than the one after
                // — the difference between a fighter that can be hit on frame one and one that is
                // briefly a bare rectangle. PREPARE, then ACTIVATE, chained on one tick. Two
                // systems rather than one because they answer different questions and only one of
                // them may fail: preparation resolves every permanent question against the
                // character authorities, and activation builds the cast from the answer without
                // consulting any authority at all. GGRS said so directly — *"sync-test checksum
                // mismatch at frames [10, 11, 12]"* — which is this repo's own "a derive's MEMO is
                // rollback state" trap: a value that GATES behaviour is not a cache.
                //
                // Activation stays here because building bodies is simulation,
                // and it replays correctly precisely because the plan it reads
                // was decided outside the window and cannot have changed.
                // Each only with its input: a plan to build, a match to release.
                (
                    crate::match_activation::activate_the_prepared_match
                        .run_if(resource_exists::<ambition_match::PreparedMatch>),
                    // The ceremony's other end: the tick the hold comes off.
                    // Chained after activation so a match whose ruleset declares
                    // NO countdown still releases on the tick it is built — the
                    // behaviour every match had before ceremonies existed.
                    crate::match_activation::release_the_opening_hold.run_if(resource_exists::<ambition_match::ActiveMatch>),
                )
                    .chain()
                    // Preparation needs an ASSEMBLED content composition, and a
                    // bare engine App legitimately has none. This is a run
                    // condition rather than an `Option<Res<..>>` parameter
                    // deliberately: `engine.character-authority-is-app-local`
                    // forbids making the character authority optional, and it is
                    // right to. "Not part of this composition" and "optional
                    // here" are different claims, and only the first is true.
                    //
                    // it gates on what preparation actually REQUIRES — the
                    // catalog its `Res<CharacterCatalog>` would panic without.
                    // Until AC6 it gated on the enemy archetype roster instead: a
                    // table about hostile bodies, standing in for "content was
                    // assembled" because it happened to be assembled at the same
                    // moment. The proxy went with the ontology; the requirement
                    // was always this one.
                    .run_if(resource_exists::<
                        ambition_characters::actor::character_catalog::CharacterCatalog,
                    >)
                    .in_set(ambition_platformer2d_shared_tangle::schedule::PlayerInputSet::CharacterProjection)
                    .before(ambition_platformer2d_actor_monolith::character_runtime::project_prepared_character_definitions),
            )
            .add_systems(
                sim,
                // HOW LONG THIS MATCH HAS BEEN FOUGHT, counted once for every
                // consumer. `WorldPrep` because both readers are downstream —
                // the timeout in `Combat` and the item cadence in the item
                // chain — so they see THIS tick's count rather than last
                // tick's, and neither has to state an ordering against the
                // other to agree about the number.
                //
                // ⛔ DELIBERATELY UNGATED on `gameplay_allowed`: the clock's own
                // answer to a stopped world is `sim_dt == 0`, which is the
                // condition itself rather than a schedule-level proxy for it,
                // and a gate here would be a second opinion that could disagree.
                ambition_match::live_match_clock::count_the_live_match_ticks
                    .run_if(resource_exists::<ambition_match::ActiveMatch>)
                    .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::WorldPrep),
            )
            .add_systems(
                sim,
                // What a match created ends with it. In `CombatSet::Trigger`,
                // the earliest combat phase, so no fighter can trip a mine that
                // an earlier match left behind.
                (ambition_match::stamp_credit_stand_ins, ambition_match::sweep_objects_from_ended_matches)
                    .in_set(ambition_platformer2d_shared_tangle::schedule::CombatSet::Trigger),
            )
            .add_systems(
                Update,
                (
                    // The same requirement as the activation registration above,
                    // named the same way.
                    crate::match_activation::prepare_the_match
                        .run_if(resource_exists::<ambition_characters::actor::character_catalog::CharacterCatalog>)
                        // A match is prepared from a published roster; with none,
                        // there is nothing to prepare.
                        .run_if(resource_exists::<ambition_match::MatchParticipantRoster>),
                    // The camera's answer for a match with no local participant.
                    // In `Update` beside preparation because it is a projection
                    // FOR presentation, not simulation state.
                    crate::match_activation::declare_the_match_cast_as_the_view.run_if(
                        |active: Option<Res<ambition_match::ActiveMatch>>,
                         framed: Option<Res<ambition_platformer2d_shared_tangle::markers::FramedCast>>| {
                            active.is_some() || framed.is_some_and(|framed| !framed.0.is_empty())
                        },
                    ),
                ),
            );
    }
}
