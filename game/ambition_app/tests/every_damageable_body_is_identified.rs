//! ⛔⛔ **IS EVERY DAMAGEABLE BODY IDENTIFIED WHERE IT IS BUILT?** (D-DAMAGEABLE-BODY-IDENTITY)
//!
//! The projectile contact protocol calls a missing target identity a
//! CONSTRUCTION failure rather than a sort fallback, so the invariant belongs
//! where bodies are built and the resolver can only report it.
//! `construction/mod.rs` mints an identity for enemies, bosses, giants, hands,
//! shrines, riders and summons — **so the placed roads are covered, and what has
//! never been established is whether any damageable body reaches the world
//! without one.**
//!
//! ⚠ NOT ALL SEVEN MINT `SimId::placement`, and this file said they did until
//! 2026-09-10. Hands take `SimId::spawned(host, ordinal)` and summons take
//! `SimId::spawned(summoner, sequence)`, which spell `{parent}/{n}`. The
//! difference is not cosmetic: it is why a floor on these roads has to compare
//! ids EXACTLY rather than by substring.
//!
//! ⛔ AND THE SEVEN IDENTITY-MINTING ROADS ARE NOT THE SEVEN DAMAGEABLE ROADS.
//! `construct_shrine` inserts `Name` + `HealShrine{pos, half_extent}` and no
//! `CenteredAabb`/`ActorFaction` pair at all, so a shrine is not a body the
//! strike road can reach and no floor over it can ever pass. Minting an
//! identity and producing a damageable body are different properties, and
//! shrines are where they come apart.
//!
//! ⚠ **A STATIC SCAN CANNOT ANSWER THIS, and the row says why:** bodies receive
//! `CenteredAabb` and `ActorFaction` from separate inserts, so a bundle-shaped
//! scanner reports 2 of 2 and is describing its own method rather than the tree.
//! ⇒ The instrument is a RUNTIME census over the population the strike road
//! actually queries.
//!
//! ⚠ **AND THE EXISTING CHECK IS A PARTIAL CENSUS.** The projectile resolver's
//! `debug_assert` (`c2188fa7a`) flags only the COINCIDENT pair — two candidates
//! at an identical distance with no identity to break the tie — so **a lone
//! unidentified body passes it in silence.** This counts the population instead
//! of waiting for two of them to collide.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::combat::components::{ActorFaction, CenteredAabb};
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::*;

/// Frames to let the shipped room finish constructing its cast.
const SETTLE_FRAMES: usize = 120;

/// ⭐⭐ THE ANTI-VACUITY FLOOR, AND IT FIRED ON THE FIRST RUN — which is the
/// reason the fixture below spawns anything at all.
///
/// A census that finds NO damageable bodies reports zero unidentified ones and
/// reads exactly like a healthy tree. **MEASURED 2026-09-10: the sandbox world
/// alone, 120 frames in, holds TWO** — so a census of it would have printed
/// `0 without a SimId` over a population of two and read as a clean bill of
/// health for the whole tree. ⇒ The floor refused the READING rather than the
/// tree, and the answer was to exercise more construction roads rather than to
/// lower it.
/// ⛔ IT IS 1, NOT THE MEASURED POPULATION, AND THAT IS THE WHOLE POINT OF A
/// FLOOR. This read `4` against a measurement of exactly 4 on 2026-09-10 — a
/// ratchet sitting on its ceiling, which fires on the first change in either
/// direction and teaches the next reader to bump the number. An anti-vacuity
/// floor answers ONE question: did the subject exist at all. ⇒ `1` answers it,
/// and the PER-ROAD floors below do the discriminating work a bigger number
/// only pretends to do. The measured total is PRINTED and named in the failure
/// message instead, so a drop from many to one is visible without an assertion
/// that manufactures red for a content edit.
const DAMAGEABLE_FLOOR: usize = 1;

/// **Every body the strike road can hit carries a stable identity.**
///
/// ⚠ THE POPULATION IS THE STRIKE ROAD'S OWN. `StrikeVictim` requires exactly
/// `CenteredAabb` + `ActorFaction`; every other field is optional with a defined
/// absence semantic. So this queries those two and nothing narrower — a filter
/// of my own choosing would be a census of my opinion about who can be hit.
#[test]
fn every_body_the_strike_road_can_hit_has_a_stable_identity() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    for _ in 0..SETTLE_FRAMES {
        sim.step(AgentAction::default());
    }

    // ⭐ DRIVE THE ROADS THE ROW NAMES, rather than censusing whatever the
    // sandbox happens to hold. `construction/mod.rs` mints an identity for
    // enemies, bosses, giants, hands, shrines, riders and summons; the question
    // is whether a damageable body reaches the world WITHOUT one, and that is
    // only askable of roads something actually travels. See the header: two of
    // the seven mint `SimId::spawned`, and shrines are not damageable at all.
    //
    // ⚠ THIS IS A DELIBERATE POPULATION AND THE TEST SAYS SO. It is not "every
    // body the game can build" — it is the enemy road, the boss road and the
    // sandbox's own cast. A road nobody drives here is not covered, and adding
    // one is how this census grows.
    sim.spawn_enemy_character_at(
        "census_enemy",
        "Census Enemy",
        (240.0, 200.0),
        (22.0, 39.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Custom(
            "pirate_raider".to_string(),
        ),
        "npc_pirate_raider",
    );
    sim.spawn_boss_at(
        "census_boss",
        "Census Boss",
        (320.0, 200.0),
        (60.0, 40.0),
        ambition_platformer2d::entity_catalog::placements::BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );
    // ⭐⭐ SAMPLED EVERY FRAME, NOT ONCE AT THE END — because a body that is
    // damageable for three frames before its identity arrives is exactly the
    // defect this row is about, and an end-state reading cannot see it.
    //
    // ⛔ IT ALSO ANSWERS A QUESTION THE END-STATE VERSION HAD TO ASSUME:
    // whether `SimId` lands with the body or after it. If it ever lagged, this
    // counter would be non-zero while the final census read clean, and the two
    // disagreeing is the finding. They agree, so the identity arrives with the
    // body — which is what makes a build-site assertion safe to consider.
    let mut ever_unidentified = 0usize;
    let mut worst_frame: Option<(usize, String)> = None;
    for frame in 0..SETTLE_FRAMES {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let mut sample = world.query_filtered::<
            (Entity, Option<&Name>),
            (
                bevy::prelude::With<CenteredAabb>,
                bevy::prelude::With<ActorFaction>,
                bevy::prelude::Without<SimId>,
            ),
        >();
        for (entity, name) in sample.iter(world) {
            ever_unidentified += 1;
            if worst_frame.is_none() {
                worst_frame = Some((
                    frame,
                    match name {
                        Some(name) => format!("{entity} ({name})"),
                        None => format!("{entity}"),
                    },
                ));
            }
        }
    }

    let world = sim.world_mut();
    let mut q = world.query::<(Entity, &CenteredAabb, &ActorFaction, Option<&SimId>, Option<&Name>)>();
    let mut total = 0usize;
    let mut unidentified: Vec<String> = Vec::new();
    let mut identities: Vec<String> = Vec::new();
    for (entity, _aabb, _faction, sim_id, name) in q.iter(world) {
        total += 1;
        match sim_id {
            Some(sim_id) => identities.push(sim_id.as_str().to_string()),
            None => unidentified.push(match name {
                Some(name) => format!("{entity} ({name})"),
                None => format!("{entity} (no Name either)"),
            }),
        }
    }

    assert_eq!(
        ever_unidentified, 0,
        "a damageable body was seen WITHOUT a `SimId` on {ever_unidentified} \
         frame-samples across the {SETTLE_FRAMES} settling frames after the \
         spawns — first at {worst_frame:?}. ⚠ The end-of-run census below may \
         still read clean, and the two disagreeing is the finding: it would mean \
         identity arrives AFTER the body becomes damageable, so there is a window \
         in which the strike road can reach a body it cannot name."
    );
    identities.sort();
    println!(
        "[identity] {total} damageable bodies, {} without a `SimId`; identified: {identities:?}",
        unidentified.len()
    );
    for row in &unidentified {
        println!("[identity]   unidentified: {row}");
    }

    // ⛔⛔ THE PER-ROAD FLOOR, BEFORE THE TOTAL — because a total hides one road
    // collapsing behind the others. If `spawn_boss_at` silently stopped
    // producing a damageable body, the total would still clear its floor on the
    // sandbox's own cast plus the enemy, and this census would report a clean
    // bill of health for a road it no longer travels.
    // ⛔⛔ EXACT EQUALITY, NEVER `contains`. `SimId::spawned(host, n)` spells
    // `{host}/{n}`, so a giant's hand id CONTAINS its host's id — a substring
    // floor over this id space is satisfied by host-only, by hands-only and by
    // both, which is every world it was meant to tell apart. These two roads
    // are not prefixes of each other, so `contains` held here BY COINCIDENCE OF
    // THE IDS PICKED; the idiom is the trap and it is gone.
    for road in ["census_enemy", "census_boss"] {
        let want = SimId::placement(road);
        assert!(
            identities.iter().any(|id| id.as_str() == want.as_str()),
            "the `{road}` construction road produced no damageable body, so this \
             census did not travel it. Identities seen: {identities:?}. ⚠ A TOTAL \
             WOULD HAVE HIDDEN THIS: the sandbox's own cast plus the other spawn \
             clears the floor below without either road being exercised."
        );
    }
    assert!(
        total >= DAMAGEABLE_FLOOR,
        "the census found {total} damageable bodies against a floor of \
         {DAMAGEABLE_FLOOR}. A world with no damageable bodies reports zero \
         unidentified ones and is indistinguishable from a healthy one, so this \
         arm refuses the READING rather than the tree — check that the room \
         still constructs its cast within {SETTLE_FRAMES} frames"
    );
    // ⭐⭐ AND THE COMPOSITION'S OWN OBSERVER, WHICH IS NOT LIMITED TO THE ROADS
    // THIS FIXTURE DRIVES. Everything above censuses what these three roads
    // produced; `BodyIdentityCensus` is fed by a system watching the INSERTION,
    // so it sees every road the run travelled including ones nobody named here.
    //
    // ⚠ READ `observed` FIRST. A composition that never installed the system
    // reports zero offenders and reads exactly like a healthy one — this is the
    // arm that tells the two apart, and the only reason the number beside it
    // means anything.
    let census = sim
        .world_mut()
        .resource::<ambition_platformer2d::actors::features::BodyIdentityCensus>()
        .clone();
    println!(
        "[identity] composition observer: {} bodies observed at insertion, {} unidentified",
        census.observed, census.unidentified
    );
    assert!(
        census.observed > 0,
        "the composition's `BodyIdentityCensus` observed NO body becoming \
         damageable across this run, so its `unidentified: {}` is a reading about \
         the observer rather than about the tree — check that \
         `install_body_identity_census` is still wired into the sim schedule",
        census.unidentified
    );
    assert_eq!(
        census.unidentified, 0,
        "{} damageable bodies became damageable with no `SimId`, seen by the \
         COMPOSITION rather than by this fixture — so at least one is on a \
         construction road this test does not drive. The error log names each.",
        census.unidentified
    );

    assert!(
        unidentified.is_empty(),
        "{} of {total} damageable bodies reached the world with no `SimId`: \
         {unidentified:?}. The contact protocol calls a missing target identity \
         a CONSTRUCTION failure rather than a sort fallback — the resolver can \
         only report it, so whatever built these has to mint one. ⚠ The \
         projectile resolver's own `debug_assert` cannot see this: it flags only \
         a COINCIDENT pair, so a lone unidentified body passes it in silence.",
        unidentified.len()
    );
}


// ───────────────────────────────────────────────────────────────────────────
// THE ROADS THE CENSUS ABOVE DOES NOT TRAVEL.
//
// ⛔ FIVE ROADS, NOT SEVEN, AND THE MISSING TWO ARE A FINDING RATHER THAN A GAP.
// `construction/mod.rs` mints an identity on seven roads, and the row that
// opened this asked for all seven to be driven. Shrines CANNOT be: see the
// header — `construct_shrine` never inserts the damageable pair, so a floor
// over shrines can never pass and the road is out of this population by
// construction. Riders mint no body of their own either; a rider IS an enemy
// body wearing a Mount relation, so its floor has to assert the RELATION.
// ───────────────────────────────────────────────────────────────────────────

/// Every damageable body's identity, and the ones that have none.
///
/// ⚠ THE POPULATION IS `StrikeVictim`'S OWN PAIR and nothing narrower. A filter
/// of my own choosing would be a census of my opinion about who can be hit.
fn damageable_census(world: &mut World) -> (Vec<String>, Vec<String>) {
    let mut q = world.query_filtered::<(Entity, Option<&SimId>, Option<&Name>), (
        With<CenteredAabb>,
        With<ActorFaction>,
    )>();
    let mut identified = Vec::new();
    let mut unidentified = Vec::new();
    for (entity, sim_id, name) in q.iter(world) {
        match sim_id {
            Some(id) => identified.push(id.as_str().to_string()),
            None => unidentified.push(match name {
                Some(name) => format!("{entity} ({name})"),
                None => format!("{entity} (no Name either)"),
            }),
        }
    }
    identified.sort();
    (identified, unidentified)
}

/// **The giant road builds a host AND two hands, and each carries its own id.**
///
/// ⛔⛔ THIS DRIVES AN AUTHORED ROOM, AND THE FIRST VERSION DROVE A PROGRAMMATIC
/// SPAWN AND MEASURED NOTHING. `spawn_enemy_character_at("npc_giant_gnu")`
/// produces NO ENTITY AT ALL — measured 2026-09-10: zero bodies, zero entities
/// carrying that id, zero entities named "giant". It is not a defect.
/// `apply_spawn_actor_requests` calls `reject_runtime_giant` and refuses a
/// `"giant"`-class spec on purpose, because the programmatic road does not lower
/// through the construction planner and so cannot mint the host + two hand rows;
/// refusing beats emitting a handless host. ⇒ **The giant cluster is reachable
/// only from a road that goes through the planner**, which is an authored room.
///
/// ⚠ AND THE REFUSAL IS INVISIBLE HERE. `reject_runtime_giant` reports through
/// `bevy::log::warn!`, and this harness installs no subscriber — so the spawn
/// call returns, nothing is built, and nothing is said. A report whose only
/// channel is a log is a report a headless caller cannot read.
///
/// ⭐ THE ASSERTION IS THE CLUSTER SHAPE, NOT AN AUTHORED ID. It looks for any
/// host `H` with BOTH `H/0` and `H/1` damageable and identified, so renaming the
/// placement cannot silently empty this test — and it is exactly the property
/// `giant_cluster_rows` claims: one host, two limbs, joined and identified.
///
/// ⚠ THE EDIT THAT MAKES THIS FALSE: lowering a limbed host through the plain
/// enemy branch, or dropping either hand row. Both leave a host in the census
/// with no `H/0`+`H/1` pair, and the search below finds no cluster.
#[test]
fn the_giant_road_builds_a_host_and_two_identified_hands() {
    const ARENA: &str = "gnu_ton_arena";
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED, not tolerant: the tolerant road falls back to the authored
        // start room, which has no giant, and this test would be green having
        // measured a room it never meant to visit.
        .with_required_start_room(ARENA);
    let mut sim = Platformer2dSimHarness::new_with_options(opts)
        .expect("the gnu-ton arena builds headlessly");
    for _ in 0..SETTLE_FRAMES {
        sim.step(AgentAction::default());
    }

    let (identified, unidentified) = damageable_census(sim.world_mut());
    let clusters: Vec<&String> = identified
        .iter()
        .filter(|host| {
            let left = format!("{host}/0");
            let right = format!("{host}/1");
            identified.iter().any(|id| *id == left) && identified.iter().any(|id| *id == right)
        })
        .collect();

    // ⭐ PRINTED, NOT ASSERTED. The count is a property of the room's cast, so
    // pinning it would break on any content edit; printing makes a drop visible
    // without manufacturing red.
    println!(
        "[giant] {} damageable bodies in `{ARENA}`, {} host+2-hand cluster(s): {clusters:?}",
        identified.len(),
        clusters.len()
    );
    println!("[giant] identities: {identified:?}");

    assert!(
        !clusters.is_empty(),
        "no damageable body in `{ARENA}` has BOTH an `H/0` and an `H/1` sibling, \
         so the giant road built no host+hands cluster and every identity \
         assertion here is vacuous. Identities: {identified:?}. ⚠ A HOST ALONE \
         WOULD NOT CLEAR THIS, and that is the point: a `\"giant\"`-class spec \
         whose character stops resolving lowers as an ORDINARY ENEMY — one body, \
         no hands, no rig — and a floor that only asked for the host would pass \
         on a road it no longer travels."
    );
    assert!(
        unidentified.is_empty(),
        "{} damageable bodies reached the world with no `SimId`: {unidentified:?}",
        unidentified.len()
    );
}

/// **The rider road: a mounted pair, both damageable, both identified.**
///
/// ⛔ THE FLOOR IS THE RELATION, NOT A BODY COUNT. `attach_authored_mount_links`
/// creates no body — it hangs an `ambition.mount` relation on a request that the
/// ENEMY road already built. So "two damageable bodies exist in this room" is a
/// floor on the enemy road wearing a different name. `wire_mount` inserts
/// `RidingOn{mount}` on the rider and the saddle on the mount, and `verify_mount`
/// reads exactly that — so `RidingOn` is the production seam this road can be
/// anchored to.
///
/// ⚠ THE EDIT THAT MAKES THIS FALSE: dropping the `RidingOn` insert in
/// `wire_mount`, or letting a mount link name a body that was never built.
#[test]
fn the_rider_road_builds_a_mounted_pair_each_identified() {
    const LOOKOUT: &str = "pirate_sky_lookout";
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED, not tolerant: the tolerant road falls back to the authored
        // start room and would run this whole test in a room with no mount link,
        // green, having measured nothing.
        .with_required_start_room(LOOKOUT);
    let mut sim = Platformer2dSimHarness::new_with_options(opts)
        .expect("the pirate sky lookout builds headlessly");
    for _ in 0..SETTLE_FRAMES {
        sim.step(AgentAction::default());
    }

    let pairs: Vec<(String, String)> = {
        let world = sim.world_mut();
        let mut riders = world.query_filtered::<(&SimId, &ambition_platformer2d::mount::RidingOn), (
            With<CenteredAabb>,
            With<ActorFaction>,
        )>();
        let raw: Vec<(String, Entity)> = riders
            .iter(world)
            .map(|(id, riding)| (id.as_str().to_string(), riding.mount))
            .collect();
        raw.into_iter()
            .map(|(rider, mount)| {
                let named = world
                    .get::<SimId>(mount)
                    .map(|id| id.as_str().to_string())
                    .unwrap_or_else(|| format!("{mount} (NO SimId)"));
                (rider, named)
            })
            .collect()
    };

    let (identified, unidentified) = damageable_census(sim.world_mut());
    println!(
        "[rider] {} damageable bodies, {} mounted pair(s): {pairs:?}",
        identified.len(),
        pairs.len()
    );

    assert!(
        !pairs.is_empty(),
        "no damageable body in `{LOOKOUT}` carries `RidingOn`, so the rider road \
         was not travelled and every identity assertion below is vacuous. \
         Identities seen: {identified:?}"
    );
    for (rider, mount) in &pairs {
        assert!(
            !mount.contains("NO SimId"),
            "rider `{rider}` rides {mount}, which carries no identity — the \
             relation was wired to a body construction never named"
        );
        assert!(
            identified.iter().any(|id| id == mount),
            "rider `{rider}` names mount `{mount}`, which is not a DAMAGEABLE \
             body. A mount the strike road cannot reach is not the pair this \
             road claims to build. Identities: {identified:?}"
        );
    }
    assert!(
        unidentified.is_empty(),
        "{} damageable bodies with no `SimId`: {unidentified:?}",
        unidentified.len()
    );
}

/// **The summon road: a runtime minion, descended from its summoner's identity.**
///
/// ⚠ THIS DRIVES THE SUMMON EXECUTOR AND ITS CONSTRUCTION ROAD, NOT THE ABILITY
/// THAT EMITS ONE. The message is written directly, so a green here says nothing
/// about whether any shipped technique still asks for a summon.
///
/// ⛔ THE FLOOR EARNS ITS PLACE BECAUSE THIS ROAD CAN PRODUCE NOTHING, SILENTLY.
/// `apply_summon_effects` drops the whole batch with `requests.clear(); return;`
/// when no session scope is active, and skips an emitter it cannot resolve. Both
/// leave a world with zero unidentified bodies, which reads exactly like success.
/// ⚠ These are CANDIDATE failure modes read from the source, not measured ones.
/// (A fourth I proposed — an emitter with a `SimId` but no `SimIdCounter` — is
/// impossible: `SimId` declares `#[require(SimIdCounter)]`.)
#[test]
fn the_summon_road_builds_an_identified_minion() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    for _ in 0..SETTLE_FRAMES {
        sim.step(AgentAction::default());
    }

    let (summoner, summoner_id) = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<(Entity, &SimId), (With<CenteredAabb>, With<ActorFaction>)>();
        q.iter(world)
            .next()
            .map(|(entity, id)| (entity, id.as_str().to_string()))
            .expect("the settled sandbox holds an identified damageable body to summon from")
    };
    let (before, _) = damageable_census(sim.world_mut());

    sim.world_mut()
        .write_message(ambition_platformer2d::vfx::EffectRequest {
            owner: summoner,
            effect: ambition_platformer2d::vfx::Effect::Summon(
                ambition_platformer2d::vfx::SummonSpec {
                    id: "census_summon".to_string(),
                    name: "Census Summon".to_string(),
                    pos: Vec2::new(300.0, 200.0),
                    half_size: Vec2::new(12.0, 12.0),
                    character_id: "npc_burning_flying_shark".to_string(),
                    encounter_id: "census_summon_encounter".to_string(),
                    faction: ambition_platformer2d::vfx::HitSide::Enemy,
                    ridden_by_summoner: None,
                    health: None,
                    keeps_contact_damage: false,
                },
            ),
        });
    sim.step_n(AgentAction::default(), 30);

    let (after, unidentified) = damageable_census(sim.world_mut());
    // ⭐ A SPAWNED ID IS `{parent}/{n}` BY CONSTRUCTION, so the prefix is the
    // exact shape of the claim rather than a loose substring search: the
    // sequence number is minted from the summoner's counter and is not knowable
    // here, but the parentage is.
    let descended: Vec<&String> = after
        .iter()
        .filter(|id| id.starts_with(&format!("{summoner_id}/")))
        .collect();
    println!(
        "[summon] summoner={summoner_id}; {} -> {} damageable bodies; descended: {descended:?}",
        before.len(),
        after.len()
    );

    assert!(
        !descended.is_empty(),
        "the summon road produced no damageable body descended from \
         `{summoner_id}`. The census went {} -> {} bodies. ⚠ This road drops a \
         whole batch silently when no session scope is active, so a run with \
         zero unidentified bodies and zero summons reads exactly like success — \
         which is why this floor is here. Identities: {after:?}",
        before.len(),
        after.len()
    );
    assert!(
        unidentified.is_empty(),
        "{} damageable bodies with no `SimId`: {unidentified:?}",
        unidentified.len()
    );
}
