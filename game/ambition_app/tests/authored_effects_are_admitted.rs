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
                if let Err(refusal) = installed.admit(effect) {
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
