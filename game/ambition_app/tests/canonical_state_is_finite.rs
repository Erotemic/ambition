//! ⛔⛔ **A NON-FINITE VALUE IN ROLLBACK-CANONICAL STATE IS NORMALISED SO THE
//! DESYNC CHECK CANNOT SEE IT — which is why nobody caught this in a year of
//! rollback work.**
//!
//! `canonical_f32_bits` already asks `is_nan()`, and it asks in order to collapse
//! every NaN to ONE bit pattern **so that two peers' checksums agree**. The one
//! mechanism that exists to notice two peers diverging has been made blind to
//! this specific poison, deliberately, for a good reason. ⇒ Two peers can hold a
//! NaN in the same canonical field and agree perfectly about it forever.
//!
//! And it does not misbehave once. `f32::clamp` returns NaN for a NaN input, so a
//! single such value poisons a meter permanently, every later comparison against
//! it is false, and it is snapshotted and restored across every rewind. The
//! failure and the healthy state are indistinguishable to everything that looks.
//!
//! ⭐ **THE OBSERVER IS NOT A NEW GUARD.** It is the smallest possible correction
//! to a function that already computes the answer and throws it away — and it is
//! exhaustive BY CONSTRUCTION rather than by discipline: passing through
//! `canonical_f32_bits` is what MAKES a value canonical, so a type that gains a
//! canonical float is observed automatically. There is no field walk to keep in
//! sync. The alternative measured first was a per-type walk across the
//! hand-written `SnapshotState` impls, each free to walk three of five fields —
//! **98 of them at 2026-09-10**, and the count is the point rather than the
//! number: it moved from 93 to 98 during the day this was written, which is
//! exactly what a walk kept by discipline has to keep up with.
//!
//! ⚠ **WHAT THIS DOES NOT COVER, and this file will not claim otherwise:
//! values that are never ENCODED.** RE-DERIVED 2026-09-17 against
//! `rollback_schema_baseline.txt` at schema v198 (491 rows): **169 are
//! `component-clone` and call `encode` on nothing**, and splitting them by
//! their `detail`:
//!
//! * **96 carry no probe at all** — "bevy_ggrs clone snapshot; not in the
//!   session checksum". A non-finite float in one of these is seen by nothing.
//! * **59 carry a localization VALUE probe** and are still outside the session
//!   checksum.
//! * 14 carry an entity-remapping probe (8 handle, 5 set, 1 keyed map).
//!
//! ⛔⛤ **THE PREVIOUS READING OF THIS WAS "108 name another authoritative
//! projection that covers them; 59 say outright they are not in the session
//! checksum", and the two numbers ARE NOT COMPARABLE WITH THE ONES ABOVE.** The
//! `detail` wording for an unhashed clone changed at schema v194 — it used to
//! claim the value was "checksummed by some other authoritative projection",
//! which neither the registrar nor its caller could establish — so the split was
//! measured against a sentence that no longer exists. Counted by what the rows
//! say today, **155 of the 169 state they are outside the session checksum**,
//! not 59. ⇒ "The canonical state is finite" is NOT what a green run here means,
//! and it covers less than this paragraph used to imply.
//!
//! ⚠ `inf` / `-inf` are COUNTED but not canonicalised by the encoder. Counting
//! without changing the encoding is deliberate: whether an infinity should
//! collapse the way a NaN does changes what two peers agree about, which is a
//! maintainer's decision.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions};
use ambition_platformer2d::engine_core::snapshot::non_finite;

/// Floor on the number of finite canonical floats a 30-frame sync-test window
/// encodes. MEASURED 2026-09-10 at **116,280** (3,876 per frame); the floor is set
/// two orders of magnitude below the reading because its job is to catch the
/// encoder going SILENT, not to pin the schema's size.
const ENCODED_FLOAT_FLOOR: u64 = 1_000;

/// ⭐ A SYNC-TEST SESSION, because that is what makes `encode` run at all. GGRS
/// saves and loads every frame under `SyncTest`, and a canonical component's save
/// IS its `SnapshotState::encode`. A sandbox with no rollback session encodes
/// nothing and would report a serene zero.
fn sync_test_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default().with_sync_test_rollback_settings(4, 10),
    )
    .expect("sandbox sim builds with a sync-test rollback session")
}

fn armed_steps(sim: &mut Platformer2dSimHarness, steps: usize) -> (u64, u64) {
    let armed = non_finite::arm();
    for _ in 0..steps {
        sim.step(AgentAction::default());
    }
    armed.observed()
}

#[test]
fn no_canonical_float_encoded_by_a_sync_test_window_is_non_finite() {
    let mut sim = sync_test_sim();
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let (non_finite_seen, finite_seen) = armed_steps(&mut sim, 30);

    // ⛔⛔ THE FLOOR FIRST, AND IT IS THE LOAD-BEARING ARM. A window that encoded
    // NO floats reports zero offenders and reads exactly like a healthy one. It
    // is not hypothetical: the first version of the observer used a thread-local
    // and measured EXACTLY ZERO, because bevy runs the encoding systems on
    // task-pool threads and not on the thread the harness is stepped from. This
    // arm is what said so instead of printing a clean bill of health.
    assert!(
        finite_seen >= ENCODED_FLOAT_FLOOR,
        "the observer saw {finite_seen} finite canonical floats over 30 sync-test \
         frames, against a floor of {ENCODED_FLOAT_FLOOR} and a measurement of \
         116,280. A window that encodes nothing reports zero non-finite values and \
         is indistinguishable from a healthy one, so this arm refuses the reading \
         rather than the tree — check that the harness really started a rollback \
         session and that canonical encoding still reaches this counter"
    );
    assert_eq!(
        non_finite_seen, 0,
        "{non_finite_seen} NON-FINITE canonical floats were encoded while this \
         window was armed (out of {finite_seen} finite ones). A NaN reaching \
         canonical state is invisible to the desync check by design — \
         `canonical_f32_bits` collapses every NaN to one bit pattern SO THAT two \
         peers agree — and `f32::clamp` returns NaN for a NaN input, so the value \
         poisons its field permanently and survives every rewind. ⚠ THE COUNTER IS \
         PROCESS-WIDE: the offender may have been encoded by another test's sim \
         running concurrently, so read this as `a` non-finite canonical float, not \
         as `this` sim's"
    );
}

/// ⭐⭐ **THE POSITIVE CONTROL, and without it a green run above is a claim about
/// the instrument.** The observer counting zero and the observer not running are
/// the same reading.
///
/// ⛔⛔ **THE POISON HAS TO BE INJECTED FROM INSIDE THE SIMULATION, and finding
/// that out is half the value of this arm.** The obvious control — write a NaN
/// into `BodyKinematics.pos.x` through `world_mut()` between two `step` calls —
/// counts NOTHING, and it is not because the observer is broken. It is because
/// GGRS ROLLS THE WRITE BACK: a sync-test session restores the frame from its own
/// saved snapshot before advancing, so an out-of-band write to canonical state is
/// erased by the exact mechanism under test. MEASURED: `pos.x` was `NaN` before
/// the step and `950` after it, with zero non-finite encodes in between.
///
/// ⇒ Any future test that poisons rollback-canonical state from outside the
/// schedule is testing GGRS's restore, not its own subject. This one runs the
/// write as a system inside [`GgrsSchedule`], where the sim itself would produce
/// such a value.
#[test]
fn the_observer_fires_when_a_canonical_float_goes_non_finite() {
    use ambition_platformer2d::engine_core::BodyKinematics;
    use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
    use ambition_platformer2d::rollback::GgrsSchedule;
    use bevy::prelude::*;

    fn poison_the_primary_body(mut bodies: Query<&mut BodyKinematics, PrimaryPlayerOnly>) {
        for mut kin in &mut bodies {
            kin.pos.x = f32::NAN;
        }
    }

    let mut sim = sync_test_sim();
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    sim.app_mut()
        .add_systems(GgrsSchedule, poison_the_primary_body);

    // ⚠ ONE FRAME. The point is that the ENCODER sees the value, not that the sim
    // survives it.
    let (non_finite_seen, finite_seen) = armed_steps(&mut sim, 1);

    assert!(
        non_finite_seen > 0,
        "a NaN written into `BodyKinematics.pos.x` — a `component-canonical` \
         field whose codec puts `pos` through `put_f32` — by a system running \
         inside the rollback schedule was encoded, and the observer counted none \
         out of {finite_seen} finite floats. The guard above is therefore \
         measuring its own instrument rather than the tree"
    );
}
