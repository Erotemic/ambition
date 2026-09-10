#![cfg(feature = "rl_sim")]

//! ⛔⛔ **THE MEASUREMENT THAT DECIDES WHETHER THE STRICT PASS CAN BE TURNED ON.**
//!
//! `TechniqueSupport::admit` refuses an unknown key, a paramless key given
//! params, and params that do not hydrate. It has NO production caller over
//! authored effects — and `MoveSpec::effect_refs`, A11b's exhaustive visitor,
//! has none outside tests either. Wiring the two together at the preparation
//! barrier is what moves a misspelled key from a mid-fight `warn!` to a refusal.
//!
//! ⚠ IT CANNOT SIMPLY BE SWITCHED ON. If the shipped content names a key nothing
//! declares, the pass would refuse real moves at startup. So this asks the
//! question FIRST, as a test, against the built app: run every authored effect
//! in the composition through the real support table and report every refusal.
//!
//! ⭐ AND IT IS THE EVENTUAL GUARD, not scaffolding. Once the production pass
//! lands at the barrier, this is the fixture that says the corpus still passes
//! it — which is the acceptance row the owner document asks for
//! ("validate the actual prepared corpus before reporting completion").
//!
//! ⛔ A GREP WAS THE WRONG INSTRUMENT and this file exists because of it: a
//! regex over `EffectRef::new("…")` in three directories found one production
//! key (`pogo_bounce`) and three test-only ones, and had no way to see a key
//! reached through a flow node, an on-hit payload or a sustained window. The
//! visitor sees all four sites by destructuring without `..`.

use ambition_app::{Platformer2dSimHarness, TimestepMode};
use ambition_app::AmbitionSim;

/// ⛔⛔ **THE SHIPPED COMPOSITION CLOSED ITS BARRIER THROUGH THE CHECKED ROAD.**
///
/// The admitting fold and the unchecked backstop are two `PreStartup` systems
/// racing a `finalized` flag, so the ORDER is the entire guarantee: if the
/// backstop wins, the cast publishes with no admission at all.
///
/// ⚠ **AND THE GUARD BELOW CANNOT SEE THAT**, which is why this one exists. It
/// re-derives `admit_at` itself against `InstalledTechniques` and would pass
/// identically in a world where the barrier never consulted them — a green
/// result about a road that did not run. Asking which road CLOSED it is a
/// different question from asking whether the corpus would be admitted.
///
/// ⭐ Written the day the ordering edge was respelled: the runtime used to say
/// `.before(ambition_characters::prepared::close_preparation_barrier)`, a foreign
/// crate ordering against another's private function path, and it now orders
/// against the published `PreparationBarrier` set. The guarantee is identical
/// and the edge is a line in a composition — so it needs a witness that fails
/// when the line goes, rather than a comment saying it matters.
#[test]
fn the_shipped_composition_closes_its_barrier_through_the_checked_road() {
    let sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    assert!(
        ambition_platformer2d::characters::prepared::barrier_closed_with_admission(sim.world()),
        "the unchecked backstop folded the cast before the admitting barrier \
         could, so every authored effect in this composition was published \
         WITHOUT being checked against the techniques it installs — and nothing \
         else in this suite can tell the difference"
    );
}

#[test]
fn every_authored_effect_in_the_shipped_composition_is_admitted() {
    use ambition_platformer2d::combat::technique::InstalledTechniques;

    let sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let world = sim.world();
    let installed = &world
        .get_resource::<InstalledTechniques>()
        .expect("the composition declares its techniques")
        .0;
    let prepared = world
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
        .expect("the cast is prepared at the barrier");

    let mut refusals: Vec<String> = Vec::new();
    let mut effects = 0usize;
    let mut moves = 0usize;
    for (id, definition) in prepared.iter() {
        // ⚠ `kit`'s moveset, not `authored_moveset`: the kit ALWAYS carries one
        // (derived from the action set when the character authored no
        // timelines), so this is the full set of effects a body wearing this
        // character can actually reach. `authored_moveset` answers a different
        // question — "did this character say what its moves ARE?" — and using it
        // here would silently skip every derived repertoire.
        let Some(moveset) = definition.kit.projectable_moveset() else {
            continue;
        };
        for mv in &moveset.moves {
            moves += 1;
            for (site, effect) in mv.effect_refs() {
                effects += 1;
                // ⛔ THE SITE, or this guard is WEAKER THAN PRODUCTION. The
                // barrier calls `admit_at(Some(&site), ..)`; a fixture calling
                // bare `admit` skips the delivery check entirely and would pass
                // a corpus the shipped composition refuses — a guard that agrees
                // with itself rather than with the road.
                if let Err(refusal) = installed.admit_at(Some(&site), effect) {
                    refusals.push(format!("{id} / {} / {site:?}: {refusal}", mv.id));
                }
            }
        }
    }

    // ⛔ THE PREMISE. Zero refusals over zero effects is the vacuous pass this
    // whole family of guards keeps producing; say what was actually examined.
    assert!(
        moves > 0 && effects > 0,
        "walked {moves} move(s) and {effects} authored effect(s) — nothing was \
         examined, so 'no refusals' is a statement about the harness"
    );

    assert!(
        refusals.is_empty(),
        "{} authored effect(s) of {effects} across {moves} move(s) are NOT \
         admitted by the shipped support table. Each is a move that plays and \
         does nothing at runtime:\n    {}",
        refusals.len(),
        refusals.join("\n    ")
    );
}

/// ⛔⛔⛔ **THE REAL RUNNER'S ORDERING, WHICH IS THE ONE THE SHIPPED GAME USES —
/// AND IT IS THE OPPOSITE OF THE ONE EVERY OTHER TEST IN THIS FILE EXERCISES.**
///
/// GPT review, 2026-09-10: the guard above is green *because* it drives the app
/// by hand. `Platformer2dSimHarness`' first tick is a direct `App::update()`,
/// which **does not run plugin `finish()`** — so the checked `PreStartup` system
/// wins a race it loses in production.
///
/// Bevy's runner does `finish()` → `cleanup()` → first `update()`, and
/// `game/ambition_app/src/app/cli.rs` reaches `app.run()` for both the windowed
/// and the browser composition. ⇒ In the shipped game the order is:
///
/// 1. the runtime builds its installed-technique support table;
/// 2. `CharacterPreparationPlugin::finish()` calls
///    `finalize_prepared_cast(world, None)` — the UNCHECKED road, and `None`
///    selects "admit everything" rather than "this composition supports
///    nothing";
/// 3. that sets `finalized`, permanently;
/// 4. the first update begins, `PreStartup` runs, and the checked system finds
///    an already-closed barrier and does nothing.
///
/// ⚠ **THE PLUGIN'S OWN COMMENT SAYS "whichever trigger fires first wins and the
/// other is a no-op", AND THAT IS THE DEFECT RATHER THAN A MITIGATION** — in
/// production the same one always fires first, and it is the unchecked one.
///
/// ⇒ This drives the lifecycle Bevy drives. **A guard that exercises the
/// convenient ordering is a guard for a program nobody runs.**
#[test]
fn the_barrier_closes_through_the_checked_road_under_the_real_lifecycle() {
    let mut app = ambition_app::app::build_visible_app(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
    );
    // ⭐ THE THREE CALLS `App::run` MAKES, IN ITS ORDER. Not `update()` alone,
    // which is what every hand-driven fixture in this repository does and what
    // hid this for the life of the packet.
    app.finish();
    app.cleanup();
    app.update();

    assert!(
        ambition_platformer2d::characters::prepared::barrier_closed_with_admission(app.world()),
        "under the REAL runner ordering — `finish()`, `cleanup()`, `update()` — \
         the unchecked backstop folded the cast before the admitting barrier \
         could, so every authored effect in the SHIPPED game is published \
         without being checked against the techniques the composition installs. \
         The sibling guard above passes only because a hand-driven `update()` \
         never runs `finish()`."
    );
}

/// ⛔⛔ **WHAT THE BARRIER ACTUALLY WITHHELD, READ FROM THE ARTIFACT RATHER THAN
/// RE-DERIVED — AND NOTHING IN THIS SUITE ASKED THAT UNTIL NOW.**
///
/// `every_authored_effect_in_the_shipped_composition_is_admitted` above walks
/// the prepared registry and asks `admit_at` again itself. ⇒ **It answers "would
/// this cast admit cleanly", not "did the barrier refuse anything"**, and the
/// queue records exactly that about it: *"the app-level admission guard
/// re-derives `admit_at` itself against `InstalledTechniques`, so it passes
/// whether or not the barrier ever consulted them."*
///
/// `AuthoredEffectRefusals` exists so a refusal is an inspectable fact rather
/// than a log line, and until this test **nothing outside a rollback waiver list
/// ever read it.** ⭐ ASK THE ARTIFACT, NOT THE FUNCTION.
///
/// ⚠ **AND IT IS NOT THE SAME QUESTION AS THE GUARD ABOVE, WHICH IS WHY BOTH
/// EXIST.** A withheld definition is ABSENT from the registry, so a walk OVER
/// the registry cannot see it — the one road that would notice is the one that
/// records what was removed.
#[test]
fn the_shipped_composition_withheld_nothing_at_its_barrier() {
    let sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let refusals = sim
        .world()
        .get_resource::<ambition_platformer2d::characters::prepared::AuthoredEffectRefusals>()
        .map(|r| r.0.clone())
        .unwrap_or_default();

    println!(
        "[admission] the shipped composition withheld {} definition(s)",
        refusals.len()
    );
    for refusal in &refusals {
        println!("[admission]   withheld: {refusal:?}");
    }

    assert!(
        refusals.is_empty(),
        "the barrier WITHHELD {} definition(s) from the shipped composition: \
         {refusals:?}. Each one is a character whose moves are absent from the \
         prepared registry — every consumer reads it as 'no such character'. ⚠ \
         The sibling guard above cannot see this: it walks the registry and a \
         withheld definition is not IN the registry.",
        refusals.len()
    );
}
