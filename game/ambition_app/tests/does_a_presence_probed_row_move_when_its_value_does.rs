//! S7 — does one of the 25 sharpest unchecksummed rows actually differ at a
//! frame compared twice?
//!
//! S7 narrows the 99 rows outside the session checksum to **25** by intersecting
//! three conditions: no value projection, read by an unfiltered per-tick query,
//! and carrying a float-bearing field. Twelve of the 25 are mutably borrowed in
//! production. `item.ground_item` is the largest writer of all of them, with
//! seven `&mut GroundItem` sites, and it carries `pos`, `vel` and `half_extent`
//! as `Vec2` — a moving physical object whose position decides whether a body
//! can pick it up.
//!
//! ⛔⛤ **THE INSTRUMENT S8 USED IS STRUCTURALLY BLIND TO EVERY ONE OF THE 25,
//! AND THAT IS WHY THIS FILE HAD TO ADD A ROAD RATHER THAN POINT AT ONE.**
//! `RollbackRestoreAudit` compares `probes.census_all(world)`, and a row
//! registered through `rollback_component_clone` gets
//! `ChecksumProbe::presence_for::<T>` — whose census is `census_presence`, which
//! hard-codes `xor: 0`. It counts carriers. A float can drift by any amount in
//! every carrier and the census is byte-identical. So "point the existing audit
//! at the 25" was not a plan; it was a plan to re-measure the carrier count.
//!
//! ⇒ `RollbackChecksumProbes::strengthen_with` is the road: it replaces a
//! presence probe with a VALUE projection at runtime, which the audit then reads.
//! Strength is owned by the probe collection, not by the registration site.
//! ⚠ It had to be added because there was no way to strengthen a DIAGNOSTIC
//! without editing the registration, which changes that row's `detail`, which
//! changes `schema_dump()`, which changes `compute_schema_fingerprint` — a
//! purely local diagnostic property welded to peer-visible identity. `Q122`.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};

type GroundItem = ambition_platformer2d::item::GroundItem;

/// ⛔⛤ **THE ROOM IS `blink_run` AND THE FIRST RUN OF THIS FILE PROVES WHY.**
/// Pointed at `combat_calibration_lab` — the room every other rollback arm uses
/// — the strengthened probe reported `carriers=0` at every one of 40 steps, 148
/// saves, 108 replay-comparable, and *"no component changed across a save/load of
/// the same frame"*. That room authors no ground item. A clean divergence report
/// over a population of ZERO is byte-identical to a clean one over a population
/// that moved, and the only reason the first run was not read as an answer is
/// that the probe printed the carrier count beside it.
///
/// `blink_run` authors exactly one ground item, which is what makes "the authored
/// object" an unambiguous phrase here as it is in `a_dropped_item_falls`.
const ROOM: &str = "blink_run";

/// Steps taken AFTER the release, while the object is falling — so every one of
/// them is a frame at which `pos` and `vel` are genuinely different from the
/// last. An item at rest carries a float that does not move, and a probe over it
/// answers about rest.
const FALLING_STEPS: usize = 24;

/// ⛔ THE WHOLE VALUE, NOT ONE FIELD. Every float `GroundItem` carries goes in,
/// by `to_bits`, so the fold is exact rather than tolerant: the question is
/// whether two saves of the same frame agree bit for bit, and a rounding drift
/// is precisely the thing a tolerance would hide.
///
/// ⚠ `spec` is folded through its `Debug` rather than a field walk, because what
/// this arm needs is "did anything about this carrier change", not an attribution
/// to a field. A `Debug` string is a stable function of the value within one
/// build, which is all a same-process resimulation comparison requires — it is
/// NOT peer-stable, and nothing here reaches a peer checksum.
fn whole_ground_item(item: &GroundItem) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for f in [
        item.pos.x,
        item.pos.y,
        item.vel.x,
        item.vel.y,
        item.half_extent.x,
        item.half_extent.y,
    ] {
        f.to_bits().hash(&mut hasher);
    }
    format!("{:?}", item.spec).hash(&mut hasher);
    hasher.finish()
}

/// The CONTROL projection: it reads the carrier and returns a constant. Folded
/// over the same carriers at the same frames, it must report NO divergence
/// however much the values move — so a divergence under `whole_ground_item` is
/// attributable to the VALUE and not to the strengthening itself, to carriers
/// appearing, or to the audit's fold.
fn a_constant_per_carrier(_item: &GroundItem) -> u64 {
    1
}

fn sim_in(room: &str) -> Platformer2dSimHarness {
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(room)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| ambition_app::rl_sim::ambition_sim_composition(app, options),
    )
    .expect("the sync-test harness builds")
}

fn sim() -> Platformer2dSimHarness {
    sim_in(ROOM)
}

/// Strengthen `GroundItem`'s probe and turn the audit on. Returns whether the
/// probe was found, which the caller must assert: `strengthen_with` refuses an
/// unregistered type rather than inventing coverage.
fn strengthen_and_audit<T: bevy::prelude::Component>(
    sim: &mut Platformer2dSimHarness,
    projection: fn(&T) -> u64,
) -> bool {
    let found = sim
        .world_mut()
        .resource_mut::<ambition_platformer2d::rollback::RollbackChecksumProbes>()
        .strengthen_with::<T>(projection);
    sim.world_mut()
        .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());
    found
}

fn carriers<T: bevy::prelude::Component>(sim: &mut Platformer2dSimHarness) -> usize {
    sim.world_mut().query::<&T>().iter(sim.world()).count()
}

/// What one measurement answers. Every field except `diverging` is a floor the
/// caller must clear before `diverging` means anything.
struct Reading {
    /// Divergences the audit recorded whose type name is the subject's.
    diverging: usize,
    /// How many frames GGRS saved twice. Zero means nothing was compared.
    resimulations: usize,
    /// The smallest carrier count seen across the window. Zero means the whole
    /// reading is about an empty population.
    smallest_carriers: usize,
    /// How many distinct censuses the probe took AT THE FRAMES THE AUDIT
    /// COMPARED. One means the comparison had nothing to disagree about.
    censuses_at_compared_frames: usize,
}

/// Strengthen `T`'s probe, run `prepare`, step `steps` times taking `action`, and
/// report what the audit saw.
///
/// ⛔ `prepare` is what puts the subject's value in motion, and it is per-subject
/// because there is no generic way to do it: a ground item has to be dropped, an
/// animation timer has to be armed. A measurement with no `prepare` measures rest.
fn measure<T: bevy::prelude::Component>(
    room: &str,
    steps: usize,
    prepare: impl FnOnce(&mut Platformer2dSimHarness),
    action: fn() -> AgentAction,
    projection: fn(&T) -> u64,
) -> Reading {
    let mut sim = sim_in(room);
    assert!(
        strengthen_and_audit::<T>(&mut sim, projection),
        "`strengthen_with` found no probe for {} — it refuses an unregistered \
         type rather than inventing coverage, so this is a registration question \
         and not a result",
        std::any::type_name::<T>()
    );
    prepare(&mut sim);
    let mut smallest = usize::MAX;
    for _ in 0..steps {
        sim.step(action());
        smallest = smallest.min(carriers::<T>(&mut sim));
    }
    let audit = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
    let wanted = std::any::type_name::<T>();
    let reading = Reading {
        diverging: audit
            .divergences
            .iter()
            .filter(|d| d.type_name == wanted)
            .count(),
        resimulations: audit.resimulations,
        smallest_carriers: smallest,
        censuses_at_compared_frames: audit.distinct_censuses_across_compared_frames_of::<T>(),
    };
    println!(
        "   {wanted}: {} distinct census(es) at the compared frames; carriers >= {}; {}",
        reading.censuses_at_compared_frames,
        reading.smallest_carriers,
        audit.coverage()
    );
    reading
}

/// The four floors, asserted together so no arm can forget one. `subject` names
/// what is being measured, because the messages are read without the call site.
fn the_reading_is_about_the_subject(reading: &Reading, subject: &str) {
    assert!(
        reading.smallest_carriers >= 1,
        "{subject}: no entity carried the component at some step of the window, \
         so the divergence count is a reading about an empty population. The \
         first run of this file reported a perfectly clean audit over \
         `carriers=0`, in a room that authors no ground item"
    );
    assert!(
        reading.resimulations > 0,
        "{subject}: GGRS never saved the same frame twice, so nothing was compared"
    );
    assert!(
        reading.censuses_at_compared_frames > 1,
        "{subject}: the probe took {} distinct census(es) at the frames the audit \
         COMPARED — so either the subject held one value at every one of them or \
         the projection is constant, and in both cases the comparison had nothing \
         to disagree about. `resimulations > 0` does not imply this: the window is \
         long and the compared frames are few",
        reading.censuses_at_compared_frames
    );
}

/// The authored object, taken up by the pressed pickup a player actually uses,
/// then released with `grab_pressed`. Both halves are the production road —
/// `a_dropped_item_falls` pins them — and the release is what puts the value in
/// motion, which is the premise this whole file rests on.
fn release_the_authored_object(sim: &mut Platformer2dSimHarness) -> bevy::prelude::Entity {
    type ItemCustody = ambition_platformer2d::held_items::ItemCustody;
    let lying: Vec<(bevy::prelude::Entity, (f32, f32))> = {
        let mut query = sim
            .world_mut()
            .query::<(bevy::prelude::Entity, &GroundItem, &ItemCustody)>();
        let mut found: Vec<_> = query
            .iter(sim.world())
            .filter(|(_, _, custody)| custody.in_world())
            .map(|(entity, ground, _)| (entity, (ground.pos.x, ground.pos.y)))
            .collect();
        found.sort_by_key(|(entity, _)| *entity);
        found
    };
    assert_eq!(
        lying.len(),
        1,
        "'{ROOM}' should author exactly one ground item lying in the world; \
         without one this file measures a population of zero and reports it clean"
    );
    let (item, (x, y)) = lying[0];
    sim.teleport_player((x, y));
    sim.step(AgentAction {
        attack: true,
        ..AgentAction::default()
    });
    sim.step(AgentAction::default());
    assert!(
        matches!(
            sim.world().get::<ItemCustody>(item),
            Some(ItemCustody::Held { .. })
        ),
        "the pressed pickup should have taken custody of the authored object"
    );
    sim.step_frame(ambition_platformer2d::engine_core::ControlFrame {
        grab_pressed: true,
        ..Default::default()
    });
    let released = sim
        .world()
        .get::<GroundItem>(item)
        .expect("the object is still a ground item");
    assert!(
        released.vel.y > 0.0,
        "a Z-drop launches at zero and picks up gravity on the release step; \
         this object left the hand doing {:?}, so nothing below is in motion",
        released.vel
    );
    item
}

/// ⛔⛤ **THE ANSWER FOR ONE OF THE 25, ASSERTED — AND ITS FOUR ANTI-VACUITY
/// FLOORS, WHICH ARE MOST OF THE ARM.**
///
/// The question S7 leaves open is whether a row that is outside the peer checksum
/// and read every tick and carries a float actually DIFFERS at a frame compared
/// twice. `the_reading_is_about_the_subject` holds three of the four floors; the
/// fourth is `strengthen_with` returning `true`, inside `measure`.
#[test]
fn a_falling_ground_item_reproduces_its_value_across_every_resimulation() {
    let reading = measure(
        ROOM,
        FALLING_STEPS,
        |sim| {
            release_the_authored_object(sim);
        },
        AgentAction::default,
        whole_ground_item,
    );
    the_reading_is_about_the_subject(&reading, "a falling ground item");
    assert_eq!(
        reading.diverging, 0,
        "a falling ground item's pos/vel/half_extent did NOT reproduce across a \
         resimulation of the same frame. That is a value outside the peer \
         checksum, read by an unfiltered per-tick query, drifting locally — \
         which is the S7 finding stated as a defect rather than a ranking"
    );
}

/// ⛔ **THE CONTROL, AND IT IS THE ABSENCE OF THE SUBJECT RATHER THAN A DIFFERENT
/// INSTANCE OF IT.** Identical room, identical release, identical audit,
/// identical carriers — and a projection that reads the carrier and returns a
/// constant. Its census must therefore take exactly ONE value at every compared
/// frame — the floor the arm above requires to be greater than one — and it must
/// report no divergence.
///
/// ⇒ Without this, `diverging == 0` above is equally well explained by a
/// strengthened probe that never actually observes the value.
#[test]
fn a_constant_projection_folds_to_one_value_and_reports_nothing() {
    let reading = measure(
        ROOM,
        FALLING_STEPS,
        |sim| {
            release_the_authored_object(sim);
        },
        AgentAction::default,
        a_constant_per_carrier,
    );
    assert!(reading.smallest_carriers >= 1, "the control ran over no carriers");
    assert!(reading.resimulations > 0, "the control compared nothing");
    assert_eq!(
        reading.censuses_at_compared_frames, 1,
        "the control projection returns a constant per carrier, so across a \
         window with a constant carrier count its census must take exactly one \
         value; {} means it is reading something it should not be",
        reading.censuses_at_compared_frames
    );
    assert_eq!(
        reading.diverging, 0,
        "a constant-per-carrier fold reported a GroundItem divergence, so the \
         divergence is about the carrier POPULATION and not the value — which \
         would make the arm above unable to tell the two apart"
    );
}

// ---------------------------------------------------------------------------
// SUBJECT 2 — `actor.animation_facts`, the sharpest member of the 25.
//
// ⛔⛤ THE REPOSITORY ALREADY SAYS WHAT THIS IS, BESIDE THE CODE THAT WRITES IT.
// `advance_body_anim_overlays`' own doc: *"Measured before the move:
// `BodyAnimFacts` is rollback-registered as `actor.animation_facts`, so
// everything here writes CANONICAL SIMULATION STATE RESTORED ON EVERY REWIND —
// this must run in the deterministic sim, and must not be reclassified as
// presentation on the strength of the field names."*
//
// ⇒ So: f32 timers, DECAYED BY `frame_dt` every tick by a sim system, restored on
// every rewind, declared canonical by the code that writes them — and outside the
// checksum two peers compare. Float accumulation is the exact failure mode a
// per-tick decay has, and a carrier count cannot see any of it.
//
// ⚠ Its own doc also states the limit on how bad a drift here would be: *"What it
// does NOT do is affect simulation GEOMETRY: authored attack volumes resolve
// against an animation row chosen by `attack_intent_animation(intent)`, a match
// on the attack INTENT, which never consults these timers."* That bounds the
// consequence; it does not make the value reproduce.

type BodyAnimFacts = ambition_platformer2d::characters::actor::BodyAnimFacts;

/// The room the other rollback arms use. It has a player, which is what carries
/// the animation facts.
const ACTOR_ROOM: &str = "combat_calibration_lab";

/// ⚠ **LONG, AND THE CADENCE BELOW IS TIGHT, BECAUSE THE VALUE IS NON-ZERO FOR
/// ONLY A FRAME OR TWO PER LANDING.** Measured by the probe in this file:
/// `land_anim_timer` reads `0.323` on the frame after a touchdown and is back to
/// `0.000` within about two frames. A jump every twenty steps put the subject in
/// motion at roughly one frame in ten, and the audit compares a minority of the
/// frames it saves — so the two windows can miss each other entirely, which is
/// what the compared-frames floor exists to report.
const DECAY_STEPS: usize = 120;

/// ⛔ EVERY FLOAT, BY `to_bits`, plus the bools — the question is bit-for-bit
/// agreement between two saves of one frame, and a rounding drift is precisely
/// what a tolerance would hide.
fn whole_anim_facts(anim: &BodyAnimFacts) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("{anim:?}").hash(&mut hasher);
    hasher.finish()
}

fn a_constant_per_actor(_anim: &BodyAnimFacts) -> u64 {
    1
}

/// ⛔⛤ **THE VERB IS JUMP, NOT ATTACK, AND THE PROBE IN THIS FILE IS WHY.**
/// Measured across 60 steps of each of four inputs: `attack` held, `attack`
/// pressed on the 1-in-12 edge that
/// `a_move_keeps_its_occurrence_across_a_rewind` uses to start several moves,
/// jump-and-land, and run-and-jump. Under both attack inputs every field of
/// every one of the four carriers read **exactly `0.000` at every step** —
/// `slash_anim_timer`, `land_anim_timer`, `dash_startup_timer`,
/// `shoot_anim_timer`. Only a landing moved anything.
///
/// ⇒ The first version of the arm below held `attack` for 40 steps and got 148
/// saves, 108 replay-comparable, **36 compared**, `carriers >= 4` — and ONE
/// distinct census. Read as a result that is *"`BodyAnimFacts` reproduces across
/// 36 comparisons"*. It is "nothing moved", and only the compared-frames floor
/// says which.
fn landing_repeatedly() -> AgentAction {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    AgentAction {
        move_x: 1.0,
        right_pressed: true,
        jump: n % 8 == 0,
        jump_held: n % 8 < 3,
        ..AgentAction::default()
    }
}

fn attacking() -> AgentAction {
    AgentAction {
        attack: true,
        ..AgentAction::default()
    }
}

/// ⚠ THE FIRST WINDOW FOR THIS SUBJECT DID NOT MOVE THE SUBJECT, and the floor
/// caught it rather than a reading of the output. Holding `attack: true` for 40
/// steps gave 148 saves, 108 replay-comparable, 36 compared, carriers >= 4 — and
/// ONE distinct census at the compared frames. Without the floor that is
/// "`BodyAnimFacts` reproduces across 36 comparisons"; with it, it is "nothing
/// moved". This probe prints the values so the window can be chosen from them.
#[test]
#[ignore = "PROBE, print-only: what the animation facts actually do under an input"]
fn probe_what_the_animation_facts_do() {
    for (label, action) in [
        ("attack held", attacking as fn() -> AgentAction),
        ("attack pressed 1-in-12, the cadence that starts moves", edged_attack as fn() -> AgentAction),
        ("jump and land", jumping as fn() -> AgentAction),
        ("move and jump", running_and_jumping as fn() -> AgentAction),
    ] {
        let mut sim = sim_in(ACTOR_ROOM);
        println!("── {label}");
        for step in 0..60 {
            sim.step(action());
            let mut query = sim.world_mut().query::<&BodyAnimFacts>();
            let facts: Vec<String> = query
                .iter(sim.world())
                .map(|anim| {
                    format!(
                        "slash={:.3} land={:.3} dash={:.3} shoot={:.3}",
                        anim.slash_anim_timer,
                        anim.land_anim_timer,
                        anim.dash_startup_timer,
                        anim.shoot_anim_timer
                    )
                })
                .collect();
            if facts.iter().any(|f| !f.contains("slash=0.000 land=0.000 dash=0.000 shoot=0.000"))
                || step % 12 == 0
            {
                println!("   step={step:>2} {facts:?}");
            }
        }
    }
}

/// The cadence `a_move_keeps_its_occurrence_across_a_rewind` uses to start
/// several moves: a press EDGE every twelfth frame, because a held button starts
/// one move and not several.
fn edged_attack() -> AgentAction {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    if n % 12 == 0 {
        AgentAction {
            attack: true,
            ..AgentAction::default()
        }
    } else {
        AgentAction::default()
    }
}

/// `land_anim_timer` is armed by ground contact, so a jump is the verb that
/// moves it — a different field of the same component, reached by a different road.
fn jumping() -> AgentAction {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    AgentAction {
        jump: n % 20 == 0,
        jump_held: n % 20 < 4,
        ..AgentAction::default()
    }
}

fn running_and_jumping() -> AgentAction {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    AgentAction {
        move_x: 1.0,
        right_pressed: true,
        jump: n % 20 == 0,
        jump_held: n % 20 < 4,
        ..AgentAction::default()
    }
}

#[test]
fn decaying_animation_timers_reproduce_across_every_resimulation() {
    let reading = measure(
        ACTOR_ROOM,
        DECAY_STEPS,
        |_| {},
        landing_repeatedly,
        whole_anim_facts,
    );
    the_reading_is_about_the_subject(&reading, "decaying animation timers");
    assert_eq!(
        reading.diverging, 0,
        "`BodyAnimFacts` did NOT reproduce across a resimulation of the same \
         frame. Its own writer's doc calls it canonical simulation state restored \
         on every rewind, and it is outside the checksum two peers compare, so a \
         drift here is both mechanical and invisible to a peer"
    );
}

/// The same control shape as for the ground item: same room, same input, a
/// projection that returns a constant per carrier.
#[test]
fn a_constant_projection_over_actors_folds_to_one_value_and_reports_nothing() {
    let reading = measure(
        ACTOR_ROOM,
        DECAY_STEPS,
        |_| {},
        landing_repeatedly,
        a_constant_per_actor,
    );
    assert!(reading.smallest_carriers >= 1, "the control ran over no carriers");
    assert!(reading.resimulations > 0, "the control compared nothing");
    assert_eq!(
        reading.censuses_at_compared_frames, 1,
        "the control returns a constant per carrier; {} distinct census(es) \
         means the actor POPULATION moved during the window, which would make \
         the arm above unable to separate a value change from a carrier change",
        reading.censuses_at_compared_frames
    );
    assert_eq!(reading.diverging, 0, "the constant control reported a divergence");
}
