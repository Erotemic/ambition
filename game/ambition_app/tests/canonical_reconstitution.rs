// Drives the real Ambition app, which needs the RL stepping API.
#![cfg(feature = "rl_sim")]
//! Every lifecycle path that rebuilds a room must rebuild the SAME room.
//!
//! `docs/planning/engine/construction-and-reconstitution.md` states the model:
//!
//! ```text
//! prepared immutable content + durable facts + retention policy
//!     -> canonical construction plan
//!     -> { new session | room transition | same-room replay | durable restore }
//! ```
//!
//! The lifecycle paths are allowed to RETAIN different populations. They are not
//! allowed to have different ideas of what the authored population IS. So the
//! instrument here is a census of the authored authoritative population plus the
//! state a fresh construction gives it, taken at the same maturity on every arm.
//!
//! ⛔ A census that only counted identities would pass against the defect this
//! file exists to find: the same-room replay never re-spawns anything, so its
//! identity set survives by accident while the STATE those identities carry is
//! whatever a hand-kept reset list remembered to restore.

use std::collections::{BTreeMap, BTreeSet};

use crate::common::{base, fixed_60hz_room_options, fixed_60hz_room_sim};

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::platformer::lifecycle::RoomScopedEntity;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::*;

/// The room every arm reconstructs. Chosen because it authors a population with
/// several distinct families (enemies, pickups, a switch, a breakable platform,
/// a kinematic path) rather than an empty corridor; the premise asserts in
/// [`replaying_a_room_rebuilds_what_entering_it_builds`] keep that true.
const ROOM: &str = "combat_calibration_lab";

/// Frames of simulation between a room's CONSTRUCTION and its census.
///
/// ⭐ AGE SINCE CONSTRUCTION, NOT SINCE BOOT. A budget counted from the start of
/// each arm samples the two rooms at different ages, because a replay commits
/// its room several frames in. Each arm counts the live frames its OWN room has
/// simulated, from the step that built it — long enough for a brain to have
/// chosen a direction, so a reconstruction that hands back a stale one is
/// visible.
///
/// Two rooms of the same age stand at the same positions to the bit, so
/// positions compare exactly. A position that differs is a room that is older
/// or younger, or one that was built differently; see
/// [`settle_after_construction`].
const MATURITY: usize = 8;

// ── the instrument ───────────────────────────────────────────────────────────

/// What a reconstruction owes ONE authored authoritative entity.
///
/// Every fact compares exactly — see [`assert_same_population`].
#[derive(Clone, Debug, Default, PartialEq)]
struct Facts {
    /// Where the thing is. A body's own kinematics when it has them, otherwise
    /// the feature's authored box.
    ///
    /// ⛔ NOT BOTH. `CenteredAabb` on a BODY is a read model that a later system
    /// syncs from the kinematics, so a freshly committed room carries the
    /// authored box for a frame while the body already stands somewhere else.
    /// Censusing both made that lag look like a 13px reconstruction defect.
    at: Option<Vec2>,
    /// Everything a reconstruction must get EXACTLY right: facing, health,
    /// disposition, breakable state, collected/opened/respawning markers,
    /// switch position. Sorted, so the census is a canonical document.
    exact: Vec<String>,
}

/// The authored authoritative population, keyed by `SimId`.
type Census = BTreeMap<String, Facts>;

/// Shortest text that reads back to the same bits, because positions compare
/// exactly and a rounded row could show two different values as one.
fn place(v: Vec2) -> String {
    format!("({},{})", v.x, v.y)
}

/// The authored room-scoped authoritative population and its reconstructed state.
///
/// Facts are collected family by family and merged under the entity's `SimId`,
/// because no single Bevy query can name them all and a per-family query keeps
/// each fact optional — an entity that lacks a family contributes no row for it.
fn census(sim: &mut Platformer2dSimHarness) -> Census {
    let mut out: Census = BTreeMap::new();

    // Identity: room-scoped authored roots. Everything below joins to this.
    {
        let mut q = sim
            .world_mut()
            .query_filtered::<&SimId, With<RoomScopedEntity>>();
        let world = sim.world();
        for sim_id in q.iter(world) {
            out.entry(sim_id.as_str().to_string()).or_default();
        }
    }

    // Bodies: position, facing, health.
    {
        let mut q = sim.world_mut().query::<(
            &SimId,
            &ae::BodyKinematics,
            Option<&ambition_platformer2d::actor::BodyHealth>,
        )>();
        let world = sim.world();
        let rows: Vec<_> = q
            .iter(world)
            .map(|(id, kin, health)| {
                (
                    id.as_str().to_string(),
                    kin.pos,
                    format!(
                        "facing={:.0} hp={}",
                        kin.facing,
                        health
                            .map(|h| format!("{}/{}", h.health.current, h.health.max))
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                )
            })
            .collect();
        for (id, pos, fact) in rows {
            if let Some(facts) = out.get_mut(&id) {
                facts.at = Some(pos);
                facts.exact.push(fact);
            }
        }
    }

    // Features: the authored box, for the things that have no body of their own.
    {
        let mut q = sim.world_mut().query::<(
            &SimId,
            &ambition_platformer2d::combat::components::CenteredAabb,
        )>();
        let world = sim.world();
        let rows: Vec<_> = q
            .iter(world)
            .map(|(id, aabb)| (id.as_str().to_string(), aabb.center))
            .collect();
        for (id, center) in rows {
            if let Some(facts) = out.get_mut(&id) {
                facts.at.get_or_insert(center);
            }
        }
    }

    // Feature dispositions a reconstruction claims to restore.
    {
        use ambition_platformer2d::item_runtime::pickups::Collected;
use ambition_platformer2d::world_items::containers::Opened;
use ambition_platformer2d::breakables::BreakableFeature;
use ambition_platformer2d::world_returns::RespawnTimer;
use ambition_platformer2d::combat::components::{
            ActorDisposition,
        };
        let mut q = sim.world_mut().query::<(
            &SimId,
            Option<&Collected>,
            Option<&Opened>,
            Option<&BreakableFeature>,
            Option<&RespawnTimer>,
            Option<&ActorDisposition>,
        )>();
        let world = sim.world();
        let rows: Vec<_> = q
            .iter(world)
            .map(|(id, collected, opened, breakable, respawn, disposition)| {
                let mut facts = Vec::new();
                if collected.is_some() {
                    facts.push("collected".to_string());
                }
                if opened.is_some() {
                    facts.push("opened".to_string());
                }
                if let Some(breakable) = breakable {
                    facts.push(format!(
                        "breakable={:?} hp={}",
                        breakable.state().state,
                        breakable.state().health.current
                    ));
                }
                if respawn.is_some() {
                    facts.push("respawning".to_string());
                }
                if let Some(disposition) = disposition {
                    facts.push(format!("disposition={disposition:?}"));
                }
                (id.as_str().to_string(), facts)
            })
            .collect();
        for (id, new_facts) in rows {
            if let Some(facts) = out.get_mut(&id) {
                facts.exact.extend(new_facts);
            }
        }
    }

    // Switches.
    {
        let mut q = sim.world_mut().query::<(
            &SimId,
            &ambition_platformer2d::encounter::switches::SwitchFeature,
        )>();
        let world = sim.world();
        let save = world.resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>().data();
        let rows: Vec<_> = q
            .iter(world)
            .map(|(id, switch)| {
                let on = save.switch(&switch.activation.id);
                (id.as_str().to_string(), format!("switch_on={on}"))
            })
            .collect();
        for (id, fact) in rows {
            if let Some(facts) = out.get_mut(&id) {
                facts.exact.push(fact);
            }
        }
    }

    for facts in out.values_mut() {
        facts.exact.sort();
    }
    out
}

impl Facts {
    fn render(&self) -> String {
        format!(
            "at={} {}",
            self.at.map(place).unwrap_or_else(|| "-".to_string()),
            self.exact.join(" | ")
        )
    }
}

fn render(census: &Census) -> String {
    census
        .iter()
        .map(|(id, facts)| format!("{id}\t{}", facts.render()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The one comparison, with a diff a reader can act on.
fn assert_same_population(what: &str, expected: &Census, actual: &Census) {
    let mut lines = Vec::new();
    for id in expected
        .keys()
        .chain(actual.keys())
        .collect::<BTreeSet<_>>()
    {
        match (expected.get(id), actual.get(id)) {
            (Some(a), Some(b)) if a == b => {}
            (Some(a), Some(b)) => lines.push(format!(
                "  ~ {id}\n      fresh:  {}\n      after:  {}",
                a.render(),
                b.render()
            )),
            (Some(a), None) => lines.push(format!("  - {id}  (fresh had: {})", a.render())),
            (None, Some(b)) => lines.push(format!("  + {id}  (only after: {})", b.render())),
            (None, None) => unreachable!(),
        }
    }
    assert!(
        lines.is_empty(),
        "{what} did not reconstruct the population a fresh entry builds.\n\
         Each `~` row is a fact one lifecycle path restores and the other does not.\n{}",
        lines.join("\n")
    );
}

// ── driving the lifecycle paths ──────────────────────────────────────────────

/// Every room-scoped authoritative entity, by ECS identity.
///
/// A rebuild despawns and respawns, so this set CHANGES the frame a
/// construction commit lands — which is how each arm finds its own commit
/// without a frame budget guessed from outside.
fn population_entities(sim: &mut Platformer2dSimHarness) -> BTreeSet<Entity> {
    let mut q = sim
        .world_mut()
        .query_filtered::<Entity, (With<RoomScopedEntity>, With<SimId>)>();
    let world = sim.world();
    q.iter(world).collect()
}

/// Step until a room construction lands, then age it by [`MATURITY`].
///
/// `previous` is the population the caller is replacing; for a fresh boot that
/// is the empty set. Returns the aged population's identities so a following
/// operation can wait for ITS commit.
fn settle_after_construction(
    sim: &mut Platformer2dSimHarness,
    previous: &BTreeSet<Entity>,
) -> BTreeSet<Entity> {
    // Aged in LIVE ticks the room has simulated, counted from the step that
    // built it. A frozen tick moves nothing, and a rebuild's commit is
    // followed by ticks on which the transition holds the sim clock at zero.
    // A room that already stands was built by the caller's last step: the
    // harness constructor's first step (SimTick 0) for a boot, the crossing
    // step for a walk. That step is its first tick of age when it ran live.
    let live = |sim: &Platformer2dSimHarness| {
        sim.world()
            .resource::<ambition_platformer2d::platformer::time::SimDt>()
            .get()
            > 0.0
    };
    let mut aged = 0;
    let standing = population_entities(sim);
    if !standing.is_empty() && &standing != previous {
        aged += usize::from(live(sim));
    } else {
        let mut built = false;
        for _ in 0..120 {
            sim.step(base());
            let now = population_entities(sim);
            if !now.is_empty() && &now != previous {
                built = true;
                break;
            }
        }
        assert!(
            built,
            "no room construction landed within 120 frames, so this arm never got \
             the population it is about to census"
        );
        aged += usize::from(live(sim));
    }
    let commit_tick = sim_tick(sim);
    for _ in 0..MATURITY * 4 {
        if aged == MATURITY {
            break;
        }
        sim.step(base());
        aged += usize::from(live(sim));
    }
    assert_eq!(aged, MATURITY, "the sim clock never ran long enough to age this arm");
    // Printed for the intermittent red in the leaving arm (one EnemySpawn a
    // frame's walk ahead after re-entry). libtest shows it only on failure,
    // so a red run names the ticks its two samples were taken at.
    eprintln!(
        "settle: commit seen at SimTick {commit_tick}, census at SimTick {}",
        sim_tick(sim)
    );
    population_entities(sim)
}

fn sim_tick(sim: &Platformer2dSimHarness) -> u64 {
    sim.world()
        .resource::<ambition_platformer2d::time::SimTick>()
        .0
}

/// A fresh boot, settled to the same age as every other arm.
fn enter_the_room() -> (Platformer2dSimHarness, BTreeSet<Entity>) {
    let mut sim = fixed_60hz_room_sim(ROOM);
    let live = settle_after_construction(&mut sim, &BTreeSet::new());
    (sim, live)
}

/// The active room's authored spawn — where every rebuild puts its subject.
fn room_spawn(sim: &mut Platformer2dSimHarness) -> Vec2 {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ae::RoomGeometry, With<ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot>>();
    q.iter(world)
        .next()
        .expect("an active session publishes its room geometry")
        .0
        .spawn
}

fn player_pos(sim: &mut Platformer2dSimHarness) -> Vec2 {
    let mut q = sim
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
    let world = sim.world();
    q.iter(world)
        .next()
        .expect("the hosted app has a primary player")
        .pos
}

/// Disturb the room the way a failed attempt does: break what breaks, hurt what
/// can be hurt, collect what can be collected, and move the body off spawn.
/// Returns the number of facts it actually changed. Switches are not flipped: a
/// switch's state is saved progress, which a replay keeps.
///
/// ⭐ It mutates the world DIRECTLY rather than playing the game, because the
/// question is what a rebuild restores, not whether a fight is winnable. Every
/// mutation is one a real attempt can produce.
fn disturb_the_room(sim: &mut Platformer2dSimHarness) -> usize {
    use ambition_platformer2d::item_runtime::pickups::Collected;
use ambition_platformer2d::breakables::BreakableFeature;

    let mut changed = 0;

    // Hurt every non-player body.
    {
        let mut q = sim
            .world_mut()
            .query_filtered::<&mut ambition_platformer2d::actor::BodyHealth, (
                With<RoomScopedEntity>,
                Without<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
            )>();
        let world = sim.world_mut();
        for mut health in q.iter_mut(world) {
            if health.health.current > 1 {
                health.health.current = 1;
                changed += 1;
            }
        }
    }
    // Break every breakable.
    {
        let mut q = sim.world_mut().query::<&mut BreakableFeature>();
        let world = sim.world_mut();
        for mut breakable in q.iter_mut(world) {
            // Through the domain's own verb, so the fixture cannot invent a
            // state the game never produces.
            let mut state = breakable.state().clone();
            let hp = state.health.max;
            let _broke = state.apply_damage(hp);
            *breakable = BreakableFeature::new(state);
            changed += 1;
        }
    }
    // Collect every collectible.
    {
        let mut q = sim.world_mut().query_filtered::<Entity, (
            With<ambition_platformer2d::platformer::lifecycle::FeatureSimEntity>,
            Without<Collected>,
        )>();
        let world = sim.world_mut();
        let entities: Vec<Entity> = q.iter(world).collect();
        for entity in entities {
            if world
                .get::<ambition_platformer2d::item::GroundItem>(entity)
                .is_some()
            {
                world.entity_mut(entity).insert(Collected);
                changed += 1;
            }
        }
    }
    changed
}

fn replay_the_room(sim: &mut Platformer2dSimHarness, live: &BTreeSet<Entity>) -> BTreeSet<Entity> {
    replay_the_room_because(
        sim,
        live,
        ambition_platformer2d::session::reset::RoomReplayRequested::manual(),
    )
}

/// A replay for a stated REASON.
///
/// ⭐ `RoomResetReason` is `{PlayerDeath, Manual}` and the distinction is not
/// decoration — `RoomReplayAdmitted`'s own doc records that policy already
/// differs by it: *"a death preserves the player's placed gun portals, a
/// deliberate retry clears them."* So "does the attempt's residue survive" is a
/// question this codebase already answers per reason, and a measurement that
/// only ever passes `manual()` cannot see that axis.
fn replay_the_room_because(
    sim: &mut Platformer2dSimHarness,
    live: &BTreeSet<Entity>,
    request: ambition_platformer2d::session::reset::RoomReplayRequested,
) -> BTreeSet<Entity> {
    sim.world_mut().write_message(request);
    settle_after_construction(sim, live)
}

// ── leaving and coming back ──────────────────────────────────────────────────

/// Walk the controlled body out of the active room through the authored zone
/// that leads to `target`, and return the room it arrived in.
///
/// The zone is chosen by asking the room graph where each one actually GOES —
/// `transition_for_player` is the resolver the crossing itself uses — because a
/// room with two exits makes "the first one" a coin flip, and the way back is
/// not the way out.
///
/// Held interact covers every activation kind: a `Door` needs it, `Walk` and
/// `EdgeExit` fire on contact regardless.
fn walk_to(sim: &mut Platformer2dSimHarness, target: &str) -> String {
    let before = sim.observation().active_room.clone();
    let zone = {
        let world = sim.world_mut();
        let live_definition = ambition_platformer2d::world::rooms::sole_live_room_definition(world)
            .expect("the session has a live room");
        let mut q = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = q
            .iter(world)
            .next()
            .expect("the session has an active room set");
        let mut reachable: Vec<String> = Vec::new();
        let mut chosen = None;
        for zone in &room_set.spec(live_definition).loading_zones {
            // The zone's own box as the body's box: a zero-length path from the
            // centre of a rectangle is inside that rectangle, so this asks the
            // resolver about exactly this zone.
            let Some(transition) = room_set.transition_for_player(live_definition, zone.aabb, ae::Vec2::ZERO, true)
            else {
                continue;
            };
            let Some(destination) = room_set.rooms.get(transition.target_room) else {
                continue;
            };
            reachable.push(destination.id.clone());
            if destination.id == target {
                chosen = Some(zone.clone());
                break;
            }
        }
        chosen.unwrap_or_else(|| {
            panic!("'{before}' has no zone leading to '{target}'; it reaches {reachable:?}")
        })
    };
    let center = zone.aabb.center();
    sim.teleport_player((center.x, center.y));
    for _ in 0..120 {
        let room = sim
            .step(AgentAction {
                interact: true,
                interact_held: true,
                ..base()
            })
            .active_room;
        if room != before {
            return room;
        }
    }
    panic!(
        "held interact inside the '{}' zone of '{before}' for 120 frames and the \
         room never changed",
        zone.name
    );
}

/// The room `ROOM`'s first authored exit leads to. Discovered rather than
/// hard-coded, so re-authoring the level moves this test instead of breaking it.
fn a_neighbour_of_the_room(sim: &mut Platformer2dSimHarness) -> String {
    let world = sim.world_mut();
    let live_definition = ambition_platformer2d::world::rooms::sole_live_room_definition(world)
        .expect("the session has a live room");
    let mut q = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
    let room_set = q
        .iter(world)
        .next()
        .expect("the session has an active room set");
    for zone in &room_set.spec(live_definition).loading_zones {
        let Some(transition) = room_set.transition_for_player(live_definition, zone.aabb, ae::Vec2::ZERO, true)
        else {
            continue;
        };
        if let Some(destination) = room_set.rooms.get(transition.target_room) {
            if destination.id != ROOM {
                return destination.id.clone();
            }
        }
    }
    panic!("'{ROOM}' has no authored exit, so it cannot be left and re-entered")
}

// ── the cases ────────────────────────────────────────────────────────────────

/// Case 1: leaving a room and coming back rebuilds it, and the collision
/// overlay leaves with the room it describes.
///
/// The reference arm. A transition has always gone through canonical
/// construction, so this is what the replay is being held to.
///
/// The overlay is geometry of one room, rebuilt at the head of each tick. A room
/// commit changes the room geometry partway through a tick, so every system
/// after the commit on that tick composes the NEW room with whatever the
/// overlay still holds. That must not be the walls of the room just left: the
/// commit retracts them in the same transaction (AP48). The sample is taken on
/// the commit tick, before the next rebuild, because that is the only moment
/// the defect exists; the tick after, the arrival room's own contributions are
/// back.
#[test]
fn leaving_a_room_and_returning_rebuilds_what_entering_it_built() {
    let (mut sim, live) = enter_the_room();
    let fresh = census(&mut sim);
    let built_here = overlay_geometry(&sim);

    disturb_the_room(&mut sim);
    let neighbour = a_neighbour_of_the_room(&mut sim);
    let away = walk_to(&mut sim, &neighbour);
    assert_eq!(away, neighbour, "the fixture never actually left '{ROOM}'");
    let live = settle_after_construction(&mut sim, &live);
    let left_behind = overlay_geometry(&sim);
    let only_there: BTreeSet<_> = left_behind.difference(&built_here).cloned().collect();
    assert!(
        !only_there.is_empty(),
        "'{neighbour}' puts no geometry in the collision overlay that '{ROOM}' does \
         not also have, so this test cannot see a stale overlay"
    );

    let back = walk_to(&mut sim, ROOM);
    assert_eq!(back, ROOM, "the fixture did not get back to '{ROOM}'");
    let on_the_commit_tick = overlay_geometry(&sim);
    let stale: Vec<_> = on_the_commit_tick.intersection(&only_there).collect();
    assert!(
        stale.is_empty(),
        "on the tick that committed '{ROOM}', the collision overlay still holds \
         geometry of '{neighbour}', the room just left: {stale:#?}"
    );
    settle_after_construction(&mut sim, &live);

    assert_eq!(
        overlay_geometry(&sim),
        built_here,
        "after the rebuild the overlay must describe '{ROOM}' as it did when the \
         room was first entered"
    );
    let returned = census(&mut sim);
    assert_same_population("leaving and re-entering", &fresh, &returned);
}

/// Every piece of geometry the collision overlay holds, one line each, so two
/// samples compare as sets and a failure names the pieces.
fn overlay_geometry(sim: &Platformer2dSimHarness) -> BTreeSet<String> {
    let overlay = ambition_platformer2d::session::sole_live_room_component::<ambition_platformer2d::world::FeatureEcsWorldOverlay,
    >(sim
        .world()).expect("the live room has a collision overlay");
    let aabb = |kind: &str, a: &ae::Aabb| {
        format!(
            "{kind} ({:.1},{:.1})..({:.1},{:.1})",
            a.min.x, a.min.y, a.max.x, a.max.y
        )
    };
    let mut pieces = BTreeSet::new();
    pieces.extend(overlay.blocks.iter().map(|b| aabb("block", &b.aabb)));
    pieces.extend(overlay.gate_solids.iter().map(|b| aabb("gate", &b.aabb)));
    pieces.extend(overlay.portal_holes.iter().map(|hole| aabb("portal hole", &hole.face.bounds())),
    );
    pieces.extend(overlay.climbable_carves.iter().map(|a| aabb("climb carve", a)),
    );
    pieces.extend(overlay.water_regions.iter().map(|w| aabb("water", &w.aabb)));
    pieces.extend(
        overlay
            .removed_block_names
            .iter()
            .map(|name| format!("removed {name}")),
    );
    pieces
}

/// The primary body's coins.
fn wallet_balance(sim: &mut Platformer2dSimHarness) -> i32 {
    let mut q = sim
        .world_mut()
        .query_filtered::<&ambition_platformer2d::characters::actor::BodyWallet, With<
            ambition_platformer2d::platformer::markers::PrimaryPlayer,
        >>();
    let world = sim.world();
    let wallets: Vec<i32> = q.iter(world).map(|wallet| wallet.balance).collect();
    assert_eq!(wallets.len(), 1, "exactly one primary player carries a wallet");
    wallets[0]
}

/// Case 2: a same-room replay rebuilds what entering the room builds, and does
/// it twice in a row.
///
/// ⛔ THE CASE THE HAND-KEPT RESET LEDGER COULD NOT HOLD. Every fact the ledger
/// forgot to restore is one `~` row in the failure.
///
/// One session carries the replay's other obligations too, each with its own
/// assert:
///
/// - the HOME AVATAR is not retired with the room and comes back standing at the
///   room's own spawn (the body being DRIVEN is
///   [`a_replay_follows_the_body_you_are_actually_driving`]'s claim);
/// - what the ATTEMPT created does not survive it and does not join the
///   reconstructed population. The residue is placed directly: what matters is
///   that it carries the marker the shipped drop road applies
///   (`SpawnedThisAttempt`, see `damage_drops::drop_currency_coin`) and no room
///   scope, so only the attempt-residue leg of the retention policy can remove
///   it;
/// - session progress survives: coins held during the attempt are not refunded
///   or confiscated. Disturbed on the live `BodyWallet`, because the save's
///   wallet is a projection `items::persist` overwrites from it every frame.
#[test]
fn replaying_a_room_rebuilds_what_entering_it_builds() {
    let (mut sim, live) = enter_the_room();
    let fresh = census(&mut sim);

    // Two empty censuses are equal, so the population must be worth comparing.
    assert!(
        fresh.len() >= 4,
        "'{ROOM}' authors only {} room-scoped authoritative roots, which is too \
         thin a population to prove anything about reconstruction:\n{}",
        fresh.len(),
        render(&fresh)
    );
    let with_facts = fresh
        .values()
        .filter(|facts| !facts.exact.is_empty())
        .count();
    assert!(
        with_facts >= 3,
        "the census found roots but no reconstructable STATE on them, so it \
         would agree with any rebuild at all:\n{}",
        render(&fresh)
    );

    let coins_before = wallet_balance(&mut sim);
    {
        let mut q = sim
            .world_mut()
            .query_filtered::<&mut ambition_platformer2d::characters::actor::BodyWallet, With<
                ambition_platformer2d::platformer::markers::PrimaryPlayer,
            >>();
        let world = sim.world_mut();
        for mut wallet in q.iter_mut(world) {
            wallet.balance += 137;
        }
    }
    let residue = sim
        .world_mut()
        .spawn(ambition_platformer2d::platformer::lifecycle::SpawnedThisAttempt)
        .id();
    let changed = disturb_the_room(&mut sim);
    assert!(
        changed >= 2,
        "the disturbance changed only {changed} facts, so the replay has almost \
         nothing to restore and this arm cannot fail honestly"
    );
    for _ in 0..MATURITY {
        sim.step(base());
    }
    let disturbed = census(&mut sim);
    assert_ne!(
        fresh, disturbed,
        "the disturbance left no trace in the census, so the instrument cannot \
         see what a replay would have to undo"
    );
    let coins_held = wallet_balance(&mut sim);
    assert_ne!(coins_held, coins_before, "the wallet disturbance did not land");

    let live = replay_the_room(&mut sim, &live);
    let replayed = census(&mut sim);
    assert_same_population("a same-room replay", &fresh, &replayed);

    assert!(
        sim.world().get_entity(residue).is_err(),
        "the previous attempt's residue is still lying in the rebuilt room"
    );
    assert_eq!(
        wallet_balance(&mut sim),
        coins_held,
        "a replay is not a refund: coins held during the attempt are session \
         progress and must survive it"
    );

    let survivors = {
        let mut q = sim
            .world_mut()
            .query_filtered::<Entity, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
        q.iter(sim.world()).count()
    };
    assert_eq!(
        survivors, 1,
        "a replay must leave exactly one primary body playing the room, not {survivors}"
    );
    // ⭐ AT THE ROOM'S SPAWN, not "wherever it was". A replay arrives the body
    // the way every other rebuild does, and the arrival is validated against the
    // rebuilt geometry, so the honest claim is where it ENDS. A body still
    // standing where the last attempt left it would be tens to hundreds of
    // pixels out; a validated arrival is a few.
    let spawn = room_spawn(&mut sim);
    let after = player_pos(&mut sim);
    assert!(
        after.distance(spawn) < 16.0,
        "a replay left the home avatar at {after:?}, not at the room spawn {spawn:?}"
    );

    disturb_the_room(&mut sim);
    replay_the_room(&mut sim, &live);
    let replayed_twice = census(&mut sim);
    assert_same_population("a second replay", &fresh, &replayed_twice);
}

/// Case 3: RETENTION, the leg the room census cannot see.
///
/// An authored object in a body's hands rides through a replay for exactly the
/// reason it rides through a door: it is `RoomScopedEntity` but not
/// `RoomResident`. And because the reconstruction is prepared against what the
/// world REMEMBERS, the room it came from does not author a second copy of it.
///
/// ⛔ TWO ASSERTIONS, AND EACH CATCHES A DIFFERENT WRONG POLICY. Retiring every
/// `RoomScopedEntity` destroys the held object; preparing the plan against no
/// durable facts re-authors it, and then two live things stand behind one
/// `SimId`. A test that checked only one of those passes against the other.
#[test]
fn an_object_in_your_hands_survives_a_replay_and_is_not_re_authored() {
    const CARRY_ROOM: &str = "blink_run";
    use ambition_platformer2d::held_items::ItemCustody;

    let mut sim = fixed_60hz_room_sim(CARRY_ROOM);
    settle_after_construction(&mut sim, &BTreeSet::new());

    // The room's single authored ground item.
    let (item, authored) = {
        let mut q = sim.world_mut().query::<(Entity, &SimId, &ItemCustody)>();
        let found: Vec<(Entity, SimId)> = q
            .iter(sim.world())
            .filter(|(_, _, custody)| custody.in_world())
            .map(|(entity, id, _)| (entity, id.clone()))
            .collect();
        assert_eq!(
            found.len(),
            1,
            "'{CARRY_ROOM}' must author exactly one ground item for this case to \
             have anything to carry"
        );
        found[0].clone()
    };

    let pos = sim
        .world()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(item)
        .expect("the authored occurrence is a ground item")
        .pos;
    sim.teleport_player((pos.x, pos.y));
    sim.step(AgentAction {
        attack: true,
        ..base()
    });
    sim.step(base());
    let holder = match sim.world().get::<ItemCustody>(item).copied() {
        Some(ItemCustody::Held { holder }) => holder,
        other => panic!("the pressed pickup should have taken custody, got {other:?}"),
    };

    // ⛔ A FIXED BUDGET HERE, not `settle_after_construction`. That helper waits
    // for the room-scoped population to CHANGE, and this room's authored
    // population is the one object now in the body's hands — which is exactly
    // what a correct replay does not rebuild. The wait would never end.
    sim.world_mut().write_message(
        ambition_platformer2d::session::reset::RoomReplayRequested::manual());
    for _ in 0..30 {
        sim.step(base());
    }

    assert!(
        sim.world().get_entity(item).is_ok(),
        "the replay destroyed the object in the body's hands — it retired every \
         room-scoped entity instead of every room RESIDENT"
    );
    assert!(
        matches!(
            sim.world().get::<ItemCustody>(item).copied(),
            Some(ItemCustody::Held { holder: h }) if h == holder
        ),
        "the object survived the replay but left the hand that was holding it"
    );

    let mut q = sim.world_mut().query::<(Entity, &SimId)>();
    let live_copies: Vec<Entity> = q
        .iter(sim.world())
        .filter(|(_, id)| *id == &authored)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(
        live_copies.len(),
        1,
        "the replay authored a second occurrence of `{authored}` into the room \
         it was carried out of, so two live things now stand behind one identity"
    );
}

// ── the durable road ─────────────────────────────────────────────────────────

/// Boot a fresh session with a supplied save file and wait for the production
/// restore latch.
///
/// ⭐ THE ONE MANUFACTURED BEAT IS THE BOOT. A load is two facts inside one
/// process — the save resource holds the file's bytes and `SaveRestored` is
/// false because nothing has applied them — and `load_save_at_startup` produces
/// exactly that pair. Every system that then runs is the shipped one, in its
/// shipped order.
/// Frames a boot takes before the save is written in, and after.
///
/// ⭐ FIXED, AND THE SAME ON EVERY DURABLE ARM. A load performs no room
/// construction of its own when the file carries no rows, so
/// `settle_after_construction` has nothing to wait for here; frame alignment is
/// what makes these censuses comparable instead.
const BEFORE_LOAD: usize = 8;
const AFTER_LOAD: usize = 90;

fn boot_with_save(
    room: &str,
    file: &ambition_platformer2d::persistence::save_data::AmbitionGameSaveData,
) -> Platformer2dSimHarness {
    use ambition_platformer2d::persistence::save::AmbitionGameSave;
    use ambition_platformer2d::platformer::lifecycle::SaveRestored;

    let mut sim = fixed_60hz_room_sim(room);
    for _ in 0..BEFORE_LOAD {
        sim.step(base());
    }
    sim.world_mut().resource_mut::<AmbitionGameSave>().0 = file.clone();
    sim.world_mut().resource_mut::<SaveRestored>().0 = false;
    for _ in 0..AFTER_LOAD {
        sim.step(base());
    }
    assert!(
        sim.world().resource::<SaveRestored>().0,
        "the load must have LANDED — a latch still false means it returned early \
         and everything below is measuring a load that never happened"
    );
    sim
}

/// A room that authors occurrence-bearing objects, so a durable "it is lying
/// somewhere else" row has something to be about. [`ROOM`] is chosen for its
/// population; this one is chosen for its objects.
const DURABLE_ROOM: &str = "central_hub_complex";

/// Case 4: a durable fact the room must ACT on reaches every road that builds
/// the room, and no road builds the room wrong first and corrects it later.
///
/// The file says one of the room's own authored occurrences is lying next door.
/// Construction has to suppress it, because minting a second one puts two live
/// things behind one identity. Three roads build the room under that file:
///
/// - a BOOT with the file, the way the binary does: bytes in the world before
///   the session activates. Sampled EVERY FRAME, not at the ends: a load that
///   built its first room with no occurrence continuity and corrected it
///   afterwards holds the object for a stretch of ticks in which combat,
///   pickups and encounters all run ungated, and both endpoints answer no
///   while the middle answers yes;
/// - the file written into a RUNNING session (the correction road);
/// - walking out of that session's room and back into it.
///
/// ⛔ THE PREMISE: with an EMPTY file the room DOES author the object, so
/// "never present" is a claim about the row and not about a room that has no
/// such object.
#[test]
fn a_relocated_occurrence_is_never_authored_by_a_load_or_a_re_entry() {
    use ambition_platformer2d::persistence::save_data::{
        PersistedOccurrence, PersistedWhereabouts,
    };
    use ambition_platformer2d::platformer::lifecycle::SaveRestored;

    let (relocatable, neighbour, empty_file) = relocatable_occurrence();

    let mut relocated_file = empty_file.clone();
    relocated_file.set_durable_horizon(vec![PersistedOccurrence::new(
        relocatable.clone(),
        PersistedWhereabouts::Placed {
            room: neighbour.clone(),
            x: 200,
            y: 200,
        },
    )], Vec::new(),
    );

    // The BOOT road.
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(DURABLE_ROOM).with_save(relocated_file.clone()),
    )
    .expect("the harness boots with a save file");
    let mut present_at: Vec<usize> = Vec::new();
    for frame in 0..AFTER_LOAD {
        if live_ids(&mut sim).contains(&relocatable) {
            present_at.push(frame);
        }
        sim.step(base());
    }
    assert!(
        sim.world().resource::<SaveRestored>().0,
        "the load never landed, so this measured a load that did not happen"
    );
    drop(sim);

    let mut plain = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(DURABLE_ROOM).with_save(empty_file),
    )
    .expect("the harness boots with a save file");
    for _ in 0..AFTER_LOAD {
        plain.step(base());
    }
    assert!(
        live_ids(&mut plain).contains(&relocatable),
        "'{DURABLE_ROOM}' does not author `{relocatable}` even with an empty file, \
         so suppressing it proves nothing"
    );
    drop(plain);

    assert!(
        present_at.is_empty(),
        "the load authored `{relocatable}` into '{DURABLE_ROOM}' on {} of the {AFTER_LOAD} \
         frames after the file said it is lying in '{neighbour}' — frames {:?}. \
         A population that is corrected later is a population that EXISTED, and \
         everything gameplay does in that window it does to two live things \
         behind one identity.",
        present_at.len(),
        &present_at[..present_at.len().min(12)]
    );

    // The CORRECTION road.
    let mut loaded = boot_with_save(DURABLE_ROOM, &relocated_file);
    let after_load = census(&mut loaded);
    assert!(
        !after_load.contains_key(&relocatable),
        "the load into a running session authored `{relocatable}` into \
         '{DURABLE_ROOM}' even though the file says it is lying in '{neighbour}' — \
         two live things behind one identity.\nCensus was:\n{}",
        render(&after_load)
    );

    // The RE-ENTRY road, under the same saved facts.
    let live = population_entities(&mut loaded);
    let away = walk_to(&mut loaded, &neighbour);
    assert_eq!(away, neighbour, "the fixture never actually left '{DURABLE_ROOM}'");
    let live = settle_after_construction(&mut loaded, &live);
    assert_eq!(walk_to(&mut loaded, DURABLE_ROOM), DURABLE_ROOM);
    settle_after_construction(&mut loaded, &live);
    let after_walk = census(&mut loaded);
    assert!(
        !after_walk.contains_key(&relocatable),
        "walking back into '{DURABLE_ROOM}' authored `{relocatable}`, which the \
         file says is lying in '{neighbour}'"
    );
}

/// The room Case 5's checkpoint resumes into. It authors an occurrence-bearing
/// object, which the case's premise asserts before anything else.
const RESUME_ROOM: &str = "blink_run";

/// Case 5: a save carrying BOTH a cross-room checkpoint and an occurrence row
/// lands BOTH.
///
/// ⛔⛔ TWO CHECKPOINT ROADS WANT THE SAME ONE-SLOT LIFECYCLE COMMIT, and the
/// collision is real: `restore_checkpoint_on_session_start` records a transition
/// to the checkpoint's room on the first tick a body exists;
/// `resume_at_checkpoint_on_reset` records one too, from the `ResetToCheckpoint`
/// the durable chain writes when the file carries rows. The slot is
/// EARLIEST-STICKY, so the second gets `AlreadyPending`, writes no
/// `RoomReplayAdmitted`, and the reset message is drained either way.
///
/// ⭐ BENIGN FOR THIS SHAPE. Both roads ask for the SAME destination and
/// arrival, so whichever wins takes the session to the same place; the only
/// thing the loser drops is the replay announcement, whose consumer is the
/// attempt-residue sweep, and a session start has no previous attempt to sweep.
/// Poisoning EITHER road alone leaves this green: they are redundant, not
/// cooperating. Poisoning both reddens it on the active-room assertion, so it
/// pins the end-to-end contract that two roads happen to implement.
///
/// ⛔ THE ROW MUST BE ABOUT THE ROOM WE RESUME INTO, not the one we open in. A
/// row naming an object the destination never authors is suppressed by
/// arithmetic rather than by the ledger, and stays green with both legs
/// poisoned.
#[test]
fn a_save_with_a_checkpoint_and_an_occurrence_lands_both() {
    use ambition_platformer2d::persistence::save_data::{
        AmbitionGameSaveData, PersistedCheckpoint, PersistedOccurrence, PersistedWhereabouts,
    };
    use ambition_platformer2d::platformer::lifecycle::SaveRestored;

    // ⛔ THE PREMISE, and the object it names: with an empty file the resume
    // room DOES author it, so the suppression below is the ledger's doing and
    // not the room's.
    let resumed_object = {
        let mut plain = Platformer2dSimHarness::new_with_options(
            fixed_60hz_room_options(RESUME_ROOM).with_save(AmbitionGameSaveData::default()),
        )
        .expect("the harness boots with a save file");
        for _ in 0..AFTER_LOAD {
            plain.step(base());
        }
        let mut q = plain.world_mut().query::<(
            &SimId,
            &ambition_platformer2d::held_items::ItemCustody)>();
        let mut ids: Vec<String> = q
            .iter(plain.world())
            .map(|(id, _)| id.as_str().to_string())
            .collect();
        ids.sort();
        ids.into_iter().next().unwrap_or_else(|| {
            panic!(
                "'{RESUME_ROOM}' authors no occurrence-bearing object even with an \
                 empty file, so suppressing one proves nothing"
            )
        })
    };

    // The checkpoint names a room other than the one the session opens in,
    // which is the only shape that makes `restore_checkpoint_on_session_start`
    // take the slot at all.
    let mut file = AmbitionGameSaveData::default();
    file.set_checkpoint(PersistedCheckpoint::new(RESUME_ROOM.to_string(), 200, 200));
    file.set_durable_horizon(vec![PersistedOccurrence::new(
        resumed_object.clone(),
        PersistedWhereabouts::Placed {
            room: DURABLE_ROOM.to_string(),
            x: 200,
            y: 200,
        },
    )], Vec::new(),
    );

    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(DURABLE_ROOM).with_save(file),
    )
    .expect("the harness boots with a save file");
    for _ in 0..AFTER_LOAD {
        sim.step(base());
    }

    assert!(
        sim.world().resource::<SaveRestored>().0,
        "the load never landed"
    );
    assert_eq!(
        active_room(&mut sim),
        RESUME_ROOM,
        "the checkpoint names '{RESUME_ROOM}', so that is where the session must \
         resume — whichever road won the one lifecycle slot"
    );
    // ⭐ AND THE LEDGER ROW SURVIVED THE COLLISION. `resumed_object` is one of
    // the RESUME room's own authored objects, and the file says it is lying back
    // in the room the session opened in, so the room we resumed into must not
    // author it.
    assert!(
        !live_ids(&mut sim).contains(&resumed_object),
        "resuming into '{RESUME_ROOM}' authored `{resumed_object}`, which the \
         file says is lying in '{DURABLE_ROOM}'"
    );
}

/// The id of the room the session is standing in.
fn active_room(sim: &mut Platformer2dSimHarness) -> String {
    let world = sim.world_mut();
    let live_definition = ambition_platformer2d::world::rooms::sole_live_room_definition(world)
        .expect("the session has a live room");
    let mut q = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
    q.iter(world)
        .next()
        .expect("the session has an active room set")
        .spec(live_definition)
        .id
        .clone()
}

/// Every `SimId` alive in the world right now.
fn live_ids(sim: &mut Platformer2dSimHarness) -> BTreeSet<String> {
    let mut q = sim.world_mut().query::<&SimId>();
    let world = sim.world();
    q.iter(world).map(|id| id.as_str().to_string()).collect()
}

/// The first occurrence-bearing object `DURABLE_ROOM` authors, a room it has an
/// authored exit to, and a save file of a session that touched nothing.
fn relocatable_occurrence() -> (
    String,
    String,
    ambition_platformer2d::persistence::save_data::AmbitionGameSaveData,
) {
    use ambition_platformer2d::persistence::save::AmbitionGameSave;

    let room = DURABLE_ROOM;
    let mut played = fixed_60hz_room_sim(room);
    for _ in 0..(BEFORE_LOAD + AFTER_LOAD) {
        played.step(base());
    }
    let empty_file = played.world().resource::<AmbitionGameSave>().data().clone();

    let relocatable = {
        let mut q = played.world_mut().query::<(
            &SimId,
            &ambition_platformer2d::held_items::ItemCustody)>();
        let world = played.world();
        let mut ids: Vec<String> = q
            .iter(world)
            .map(|(id, _)| id.as_str().to_string())
            .collect();
        ids.sort();
        ids.into_iter().next().unwrap_or_else(|| {
            panic!(
                "'{room}' authors no occurrence-bearing object, so this case \
                 has nothing to relocate"
            )
        })
    };
    let neighbour = {
        let world = played.world_mut();
        let live_definition = ambition_platformer2d::world::rooms::sole_live_room_definition(world)
            .expect("the session has a live room");
        let mut q = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = q
            .iter(world)
            .next()
            .expect("the session has an active room set");
        room_set
            .spec(live_definition).loading_zones
            .iter()
            .filter_map(|zone| {
                room_set
                    .transition_for_player(live_definition, zone.aabb, ae::Vec2::ZERO, true)
                    .and_then(|t| room_set.rooms.get(t.target_room))
                    .map(|spec| spec.id.clone())
            })
            .find(|id| id.as_str() != room)
            .unwrap_or_else(|| panic!("'{room}' has no authored exit"))
    };
    (relocatable, neighbour, empty_file)
}

/// ⛔ THE BODY YOU ARE DRIVING, NOT THE ONE YOU LEFT AT HOME.
///
/// `RoomReplayRequested`'s contract says "the controlled player". While
/// possessing an actor, `PrimaryPlayer` is the home avatar the player is NOT
/// driving — so a replay that queried it reset the wrong body and named the
/// wrong subject, while the possessed body carried the previous attempt's state
/// through custody.
#[test]
fn a_replay_follows_the_body_you_are_actually_driving() {
    const POSSESSION_ROOM: &str = "vertical_shaft";
    use ambition_platformer2d::platformer::markers::ControlledSubject;

    let mut sim = fixed_60hz_room_sim(POSSESSION_ROOM);
    let (actor, _) = crate::common::possess_the_authored_enemy(&mut sim);
    // The helper stops the frame possession COMMITS; the seat projection that
    // publishes  runs after it, so let the frame finish.
    for _ in 0..4 {
        sim.step(base());
    }
    // ⛔ THE DRIVEN BODY IS WHAT THE REPLAY FOLLOWS, so the fixture asserts which
    // body that IS before asserting anything about the replay. A fixture that
    // assumed the possessed entity and the controlled subject were the same
    // would be measuring its own assumption.
    let driven = sim
        .world()
        .resource::<ControlledSubject>()
        .0
        .expect("possession leaves somebody driving");
    assert_eq!(
        driven, actor,
        "setup: possession must move the primary seat onto the possessed actor, \
         or `ControlledSubject` is not the seam a replay should follow"
    );

    // Put the driven body somewhere it plainly is not going to be after a reset.
    let spawn = room_spawn(&mut sim);
    let away = spawn + Vec2::new(0.0, -260.0);
    {
        let world = sim.world_mut();
        let mut q = world.query::<(
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel,
        )>();
        let (mut cluster_item, mut model) = q
            .get_mut(world, actor)
            .expect("the possessed actor still has a body");
        let mut clusters = cluster_item.as_clusters_mut();
        ae::movement::transit_body(
            &mut model,
            &mut clusters,
            away,
            ae::movement::TransitVelocity::Zero,
        );
    }
    // ⭐ AND WOUND IT, so the case asserts an OUTCOME the player would notice
    // and not only a position.
    //
    // ⚠ THIS ASSERTION IS NOT ATTRIBUTABLE TO ONE SYSTEM. Poisoned to reset a
    // deliberately DIFFERENT body, the driven body still came back at full
    // health — so something else on the rebuild road (the room reconstruction,
    // or the save sync that follows it) delivers it too. What the POSITION
    // assertion below pins, and what nothing else delivers, is the ADMISSION's
    // choice of subject: naming the home avatar there leaves the driven body
    // where the last attempt left it.
    {
        let world = sim.world_mut();
        let mut health = world
            .get_mut::<ambition_platformer2d::actor::BodyHealth>(actor)
            .expect("the driven body has a health meter");
        health.health.current = 1;
    }
    sim.step(base());

    sim.world_mut().write_message(
        ambition_platformer2d::session::reset::RoomReplayRequested::manual());
    for _ in 0..20 {
        sim.step(base());
    }

    let health = sim
        .world()
        .get::<ambition_platformer2d::actor::BodyHealth>(actor)
        .expect("the driven body survives its own replay");
    assert_eq!(
        health.health.current, health.health.max,
        "the replay left the DRIVEN body carrying the last attempt's damage"
    );

    let landed = sim
        .world()
        .get::<ae::BodyKinematics>(actor)
        .map(|kin| kin.pos)
        .expect("the driven body survives its own replay");
    assert!(
        landed.distance(spawn) < landed.distance(away),
        "the replay left the DRIVEN body at {landed:?} — nearer where the last \
         attempt left it ({away:?}) than the room spawn ({spawn:?}). The reset \
         went to the home avatar instead of the body being played."
    );
}

/// ⛔⛔ THE SECOND AUTHORITY THE POPULATION CENSUS CANNOT SEE.
///
/// Every case above is keyed by `SimId` over `RoomScopedEntity` and compares
/// ECS state. The durable save appears in this file only as a FIXTURE. ⇒ A
/// lifecycle path that leaves a stale durable fact produces a room whose
/// population is **identical by construction** and whose DOORS and DIALOGUE
/// differ, because eight of the engine's nine published conditions read the
/// save or the live state mirroring it — `world.flag_set`, `world.switch_on`,
/// `inventory.holds`, `custody.is_held`, `encounter.cleared`, `boss.cleared`,
/// `quest.active`, `wallet.can_afford`.
///
/// ⭐ DESTRUCTURED, NOT FIELD-LISTED, and that is the whole point of doing it
/// here. `reset_cut_rope_attempt_on_replay` clears its durable facts by hand
/// (three when this was written; ONE since 2026-09-05, when a flag with no
/// reader was removed — the count shrinking on its own is the argument, not a
/// counter-example),
/// and a hand-kept list grows only when somebody notices — the property that
/// made `reset_ecs_room_features` a second constructor with sixteen queries.
/// This comparison cannot fall behind: adding a field to
/// `AmbitionGameSaveData` fails to compile until it is classified.
fn durable_families_that_differ(
    before: &ambition_platformer2d::persistence::save_data::AmbitionGameSaveData,
    after: &ambition_platformer2d::persistence::save_data::AmbitionGameSaveData,
) -> Vec<&'static str> {
    // ⛔ THE DESTRUCTURE IS STILL THE GUARD -- it now lives on the type, in
    // `ambition_persistence`, because the fact fields are sealed and only that
    // crate can destructure them. It also got stronger in the move: both sides
    // are destructured now, where this copy destructured only `before`.
    before.families_that_differ(after)
}

fn durable_facts(
    sim: &Platformer2dSimHarness,
) -> ambition_platformer2d::persistence::save_data::AmbitionGameSaveData {
    use ambition_platformer2d::persistence::save::AmbitionGameSave;
    sim.world().resource::<AmbitionGameSave>().data().clone()
}

/// Which durable save families a replay changes, printed for the open question
/// of which attempt-scoped facts a replay owns. The wallet (a live component
/// the save mirrors) and a flag (the save is its authority) are disturbed
/// first, so the report covers both kinds of plumbing. That session progress
/// survives a replay is asserted in
/// [`replaying_a_room_rebuilds_what_entering_it_builds`].
#[test]
#[ignore = "playtest: prints a census for a policy nobody has ruled on; it asserts nothing about the replay"]
fn a_replay_reports_the_durable_families_it_touches() {
    use ambition_platformer2d::persistence::save::AmbitionGameSave;

    let (mut sim, live) = enter_the_room();
    let fresh = durable_facts(&sim);
    {
        let mut q = sim
            .world_mut()
            .query_filtered::<&mut ambition_platformer2d::characters::actor::BodyWallet, With<
                ambition_platformer2d::platformer::markers::PrimaryPlayer,
            >>();
        let world = sim.world_mut();
        for mut wallet in q.iter_mut(world) {
            wallet.balance += 137;
        }
    }
    // Let the mirror carry the live balance into the save.
    sim.step(base());
    {
        let mut save = sim.world_mut().resource_mut::<AmbitionGameSave>();
        save.data_mut()
            .set_flag("a_flag_set_during_this_attempt", true);
    }
    let disturbed = durable_facts(&sim);
    let landed = durable_families_that_differ(&fresh, &disturbed);
    assert!(
        landed.contains(&"wallet") && landed.contains(&"flags"),
        "the disturbance must land in BOTH families or the report says nothing; \
         it landed in {landed:?}"
    );

    replay_the_room(&mut sim, &live);
    let after = durable_facts(&sim);
    eprintln!(
        "[durable-census] a replay of `{ROOM}` changed these durable families: {:?}",
        durable_families_that_differ(&disturbed, &after)
    );
}

/// Prints whether a boss defeat written into the save BY HAND (a defeat from
/// before the checkpoint baseline) survives a replay, for one cut-rope boss and
/// one `BossSpawn` boss. Q56 (BOSS-REPLAY-RETRACTION) ruled that a defeat after
/// the last checkpoint is retracted and one before it is kept;
/// `boss_replay_retraction.rs` asserts both halves.
#[test]
#[ignore = "playtest: a census printed for the Q56 ruling; boss_replay_retraction.rs asserts the rule"]
fn how_many_boss_families_retract_their_defeat_on_a_replay() {
    use ambition_platformer2d::boss_encounter::BossConfig;
    use ambition_platformer2d::persistence::save::AmbitionGameSave;
    use ambition_platformer2d::platformer::authored_logic::{
        AuthoredArg, ConditionCatalog, ConditionId, ConditionOutcome,
    };

    // Two boss families, one room each: each costs a room boot.
    const ROOMS: [&str; 2] = ["you_have_to_cut_the_rope", "mockingbird_sky"];

    let mut report: Vec<String> = Vec::new();
    for room in ROOMS {
        let mut sim = fixed_60hz_room_sim(room);
        let live = settle_after_construction(&mut sim, &BTreeSet::new());

        let placements: Vec<String> = {
            let mut q = sim.world_mut().query::<&BossConfig>();
            let world = sim.world();
            q.iter(world).map(|config| config.id.clone()).collect()
        };
        assert!(
            !placements.is_empty(),
            "`{room}` is in this list because it authors a boss; it authored none, \
             so this arm is measuring nothing"
        );

        let id = ConditionId::parse("boss.cleared").expect("well-formed id");
        for placement in placements {
            {
                let mut save = sim.world_mut().resource_mut::<AmbitionGameSave>();
                save.data_mut().set_boss(
                    placement.clone(),
                    ambition_platformer2d::persistence::save_data::PersistedEncounterState::Cleared,
                );
            }
            replay_the_room(&mut sim, &live);
            let after = {
                let world = sim.world_mut();
                world.resource_scope::<ConditionCatalog, _>(|world, catalog| {
                    catalog.evaluate(world, &id, &[AuthoredArg::Name(placement.clone())], &ambition_platformer2d::platformer::authored_logic::AuthoredAsk::new("probe", "a test",
                        ),
                    )
                })
            };
            report.push(format!(
                "{room}/{placement}: after a replay `boss.cleared` = {}",
                match after {
                    ConditionOutcome::Satisfied => "STILL CLEARED",
                    ConditionOutcome::NotSatisfied(_) => "retracted",
                    ConditionOutcome::Unanswerable(_) => "unanswerable",
                }
            ));
        }
    }
    eprintln!("[boss-retraction-census]\n  {}", report.join("\n  "));
    assert_eq!(
        report.len(),
        2,
        "one line per authored boss placement across the two rooms: {report:#?}"
    );
}

/// Prints whether a death and a deliberate retry treat a boss defeat written
/// into the save by hand the same way, in the cut-rope room. The rule for each
/// reason is asserted in `boss_replay_retraction.rs` (a replay, and a death in
/// this room and in another).
#[test]
#[ignore = "playtest: a census printed for the Q56 ruling on replay reasons; boss_replay_retraction.rs asserts the rule"]
fn does_a_death_retract_a_boss_defeat_the_same_way_a_retry_does() {
    use ambition_platformer2d::boss_encounter::BossConfig;
    use ambition_platformer2d::persistence::save::AmbitionGameSave;
    use ambition_platformer2d::platformer::authored_logic::{
        AuthoredArg, ConditionCatalog, ConditionId, ConditionOutcome,
    };
    use ambition_platformer2d::session::reset::RoomReplayRequested;

    // A room with one authored boss.
    const ROOM: &str = "you_have_to_cut_the_rope";

    let id = ConditionId::parse("boss.cleared").expect("well-formed id");
    let mut report: Vec<String> = Vec::new();

    for (label, request) in [
        ("a deliberate retry", RoomReplayRequested::manual()),
        ("a player death", RoomReplayRequested::player_death()),
    ] {
        let mut sim = fixed_60hz_room_sim(ROOM);
        let live = settle_after_construction(&mut sim, &BTreeSet::new());

        let placements: Vec<String> = {
            let mut q = sim.world_mut().query::<&BossConfig>();
            let world = sim.world();
            q.iter(world).map(|config| config.id.clone()).collect()
        };
        assert!(
            !placements.is_empty(),
            "`{ROOM}` authored no boss, so this arm is measuring nothing"
        );

        for placement in &placements {
            {
                let mut save = sim.world_mut().resource_mut::<AmbitionGameSave>();
                save.data_mut().set_boss(
                    placement.clone(),
                    ambition_platformer2d::persistence::save_data::PersistedEncounterState::Cleared,
                );
            }
        }
        replay_the_room_because(&mut sim, &live, request);
        for placement in &placements {
            let after = {
                let world = sim.world_mut();
                world.resource_scope::<ConditionCatalog, _>(|world, catalog| {
                    catalog.evaluate(world, &id, &[AuthoredArg::Name(placement.clone())], &ambition_platformer2d::platformer::authored_logic::AuthoredAsk::new("probe", "a test",
                        ),
                    )
                })
            };
            report.push(format!(
                "{placement}: after {label}, `boss.cleared` = {}",
                match after {
                    ConditionOutcome::Satisfied => "STILL CLEARED",
                    ConditionOutcome::NotSatisfied(_) => "retracted",
                    ConditionOutcome::Unanswerable(_) => "unanswerable",
                }
            ));
        }
    }

    eprintln!("[boss-retraction-by-reason]\n  {}", report.join("\n  "));
    assert_eq!(
        report.len(),
        2,
        "one line per reason for the single cut-rope placement: {report:#?}"
    );
}

/// ⭐⭐ EVERY SHIPPED BOSS GATE NAMES A SHIPPED BOSS PLACEMENT.
///
/// Boss progress is keyed by the stable AUTHORED placement id (Jon, 2026-09-05,
/// Q57). For each id a shipped `boss_cleared(...)` asks about, a shipped room
/// must place a boss under exactly that id, or the gate can never open.
///
/// ⛔⛔ NEITHER SIDE IS SPELLED HERE. The id comes from the SHIPPED DIALOGUE and
/// the placement from the lowered SHIPPED WORLDS. A test that seeded
/// `set_boss("mockingbird")` and asked `boss_cleared("mockingbird")` agreed with
/// itself under a key production never wrote. The rest of the road is owned
/// below this binary: construction builds a boss under its placement id
/// (`ActorConstructionParams::AuthoredBoss`), and `boss.cleared` answers from
/// the save row under that id (`ambition_boss_encounter::conditions::tests`).
#[test]
fn every_shipped_boss_gate_names_a_shipped_boss_placement() {
    let asked_ids: BTreeSet<String> = ambition_content::dialogue::yarn::yarn_sources()
        .iter()
        .flat_map(|(_, text)| ambition_content::dialogue::yarn::executable_regions(text))
        .filter_map(|(_, region)| {
            let rest = region.split_once("boss_cleared(")?.1;
            let inner = rest.trim_start().strip_prefix('"')?;
            let close = inner.find('"')?;
            Some(inner[..close].to_string())
        })
        .collect();
    assert!(!asked_ids.is_empty(), "the shipped corpus gates on at least one boss");

    let world_manifest = ambition_content::worlds::world_manifest();
    let project = ambition_platformer2d::ldtk_map::LdtkProject::load_default_for_dev(&world_manifest)
        .expect("the shipped worlds load");
    let rooms = project
        .to_room_set(&world_manifest, &ambition_app::composed_ldtk_vocabulary())
        .expect("the shipped worlds lower");

    let unplaced: Vec<&String> = asked_ids
        .iter()
        .filter(|asked| {
            !rooms
                .rooms
                .iter()
                .any(|room| room.boss_spawns.iter().any(|boss| boss.id == **asked))
        })
        .collect();
    assert!(
        unplaced.is_empty(),
        "the shipped dialogue asks `boss_cleared(..)` about {unplaced:?}, and no shipped \
         room places a boss under that id: the gate can never open"
    );
}
