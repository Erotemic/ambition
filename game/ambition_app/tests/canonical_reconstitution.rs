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

use crate::common::{base, fixed_60hz_room_sim};

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::platformer::lifecycle::RoomScopedEntity;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::*;

/// The room every arm reconstructs. Chosen because it authors a population with
/// several distinct families (enemies, pickups, a switch, a breakable platform,
/// a kinematic path) rather than an empty corridor; the fixture guard in
/// [`a_freshly_entered_room_is_a_population_worth_comparing`] keeps that true.
const ROOM: &str = "combat_calibration_lab";

/// Frames of simulation between a room's CONSTRUCTION and its census.
///
/// ⭐ AGE SINCE CONSTRUCTION, NOT SINCE BOOT. A budget counted from the start of
/// each arm samples the two rooms at different ages, because a replay commits
/// its room several frames in. Each arm waits for its OWN commit and then ages
/// by this many frames — long enough for a brain to have chosen a direction, so
/// a reconstruction that hands back a stale one is visible.
///
/// The sub-frame residue that no whole-frame budget can remove is absorbed by
/// [`POSITION_TOLERANCE`], and by nothing else.
const MATURITY: usize = 8;

// ── the instrument ───────────────────────────────────────────────────────────

/// What a reconstruction owes ONE authored authoritative entity.
///
/// Split into a position and a set of exact facts because the two compare
/// differently — see [`assert_same_population`].
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

/// How far two correctly-reconstructed bodies may stand apart.
///
/// ⭐ ONE FRAME OF THE FASTEST AUTHORED PATROL, and it buys exactly one thing:
/// a boot builds its room before the first frame while a rebuild commits partway
/// through one, so two identically-constructed populations get different
/// fractions of a frame of motion. Measured at 0.5px on this room's enemies.
///
/// ⛔ It is not a general slack. Every non-positional fact above is compared
/// EXACTLY, and the defect this file was written against showed up as 34.6px
/// plus a flipped facing — seventeen times this tolerance.
const POSITION_TOLERANCE: f32 = 2.0;

fn round(v: Vec2) -> String {
    format!("({:.1},{:.1})", v.x, v.y)
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
        use ambition_platformer2d::combat::components::{
            ActorDisposition, BreakableFeature, Collected, Opened, RespawnTimer,
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
                        breakable.breakable.state, breakable.breakable.health.current
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
            &ambition_platformer2d::encounter::switches::SwitchOn,
        )>();
        let world = sim.world();
        let rows: Vec<_> = q
            .iter(world)
            .map(|(id, on)| (id.as_str().to_string(), format!("switch_on={}", on.0)))
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
            self.at.map(round).unwrap_or_else(|| "-".to_string()),
            self.exact.join(" | ")
        )
    }

    /// Whether two reconstructions of the same identity agree. Positions within
    /// [`POSITION_TOLERANCE`]; everything else exactly.
    fn agrees_with(&self, other: &Self) -> bool {
        let placed = match (self.at, other.at) {
            (Some(a), Some(b)) => a.distance(b) <= POSITION_TOLERANCE,
            (None, None) => true,
            _ => false,
        };
        placed && self.exact == other.exact
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
            (Some(a), Some(b)) if a.agrees_with(b) => {}
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
        "no room construction landed within 120 frames, so this arm never got          the population it is about to census"
    );
    for _ in 0..MATURITY {
        sim.step(base());
    }
    population_entities(sim)
}

/// A fresh boot, settled to the same age as every other arm.
fn enter_the_room() -> (Platformer2dSimHarness, BTreeSet<Entity>) {
    let mut sim = fixed_60hz_room_sim(ROOM);
    let live = settle_after_construction(&mut sim, &BTreeSet::new());
    (sim, live)
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
/// can be hurt, flip what flips, collect what can be collected, and move the
/// body off spawn. Returns the number of facts it actually changed.
///
/// ⭐ It mutates the world DIRECTLY rather than playing the game, because the
/// question is what a rebuild restores, not whether a fight is winnable. Every
/// mutation is one a real attempt can produce.
fn disturb_the_room(sim: &mut Platformer2dSimHarness) -> usize {
    use ambition_platformer2d::combat::components::{BreakableFeature, Collected};
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
            let hp = breakable.breakable.health.max;
            breakable.breakable.apply_damage(hp);
            changed += 1;
        }
    }
    // Flip every switch.
    {
        let mut q = sim
            .world_mut()
            .query::<&mut ambition_platformer2d::encounter::switches::SwitchOn>();
        let world = sim.world_mut();
        for mut on in q.iter_mut(world) {
            on.0 = true;
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
    sim.world_mut()
        .write_message(ambition_platformer2d::actors::session::reset::RoomReplayRequested);
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
        let mut q = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = q
            .iter(world)
            .next()
            .expect("the session has an active room set");
        let mut reachable: Vec<String> = Vec::new();
        let mut chosen = None;
        for zone in room_set.active_loading_zones() {
            // The zone's own box as the body's box: a zero-length path from the
            // centre of a rectangle is inside that rectangle, so this asks the
            // resolver about exactly this zone.
            let Some(transition) = room_set.transition_for_player(zone.aabb, ae::Vec2::ZERO, true)
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
    let mut q = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
    let room_set = q
        .iter(world)
        .next()
        .expect("the session has an active room set");
    for zone in room_set.active_loading_zones() {
        let Some(transition) = room_set.transition_for_player(zone.aabb, ae::Vec2::ZERO, true)
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

/// Case 1: a freshly entered room is a population worth comparing.
///
/// ⛔ THE PREMISE GUARD. Every other case in this file compares two censuses;
/// two empty censuses are equal. This one refuses to let that happen silently.
#[test]
fn a_freshly_entered_room_is_a_population_worth_comparing() {
    let (mut sim, _) = enter_the_room();
    let fresh = census(&mut sim);
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
}

/// Case 2: leaving a room and coming back rebuilds it.
///
/// The reference arm. A transition has always gone through canonical
/// construction, so this is what the replay is being held to.
#[test]
fn leaving_a_room_and_returning_rebuilds_what_entering_it_built() {
    let (mut sim, live) = enter_the_room();
    let fresh = census(&mut sim);

    disturb_the_room(&mut sim);
    let neighbour = a_neighbour_of_the_room(&mut sim);
    let away = walk_to(&mut sim, &neighbour);
    assert_eq!(away, neighbour, "the fixture never actually left '{ROOM}'");
    let live = settle_after_construction(&mut sim, &live);

    let back = walk_to(&mut sim, ROOM);
    assert_eq!(back, ROOM, "the fixture did not get back to '{ROOM}'");
    settle_after_construction(&mut sim, &live);

    let returned = census(&mut sim);
    assert_same_population("leaving and re-entering", &fresh, &returned);
}

/// Case 3: a same-room replay rebuilds what entering the room builds.
///
/// ⛔ THE CASE THE HAND-KEPT RESET LEDGER COULD NOT HOLD. Every fact the ledger
/// forgot to restore is one `~` row in the failure.
#[test]
fn replaying_a_room_rebuilds_what_entering_it_builds() {
    let (mut sim, live) = enter_the_room();
    let fresh = census(&mut sim);

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

    replay_the_room(&mut sim, &live);
    let replayed = census(&mut sim);
    assert_same_population("a same-room replay", &fresh, &replayed);
}

/// Case 4: retention. The body playing the room is not retired with it, and it
/// comes back standing at the room's own spawn.
#[test]
fn a_replay_reconstructs_the_room_without_retiring_the_body_playing_it() {
    let (mut sim, live) = enter_the_room();

    disturb_the_room(&mut sim);
    replay_the_room(&mut sim, &live);

    let mut q = sim
        .world_mut()
        .query_filtered::<Entity, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>(
        );
    let world = sim.world();
    let survivors = q.iter(world).count();
    assert_eq!(
        survivors, 1,
        "a replay must leave exactly one primary body playing the room, not {survivors}"
    );

    // ⭐ AT THE ROOM'S SPAWN, not "wherever it was". A replay arrives the body
    // the way every other rebuild does, and the arrival is validated against the
    // rebuilt geometry — so the honest claim is where it ENDS, not that it never
    // moved. A body still standing where the last attempt left it would be tens
    // to hundreds of pixels out; a validated arrival is a few.
    let spawn = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&ae::RoomGeometry, With<ambition_platformer2d::platformer::lifecycle::SessionRoot>>();
        q.iter(world)
            .next()
            .expect("an active session publishes its room geometry")
            .0
            .spawn
    };
    let after = player_pos(&mut sim);
    assert!(
        after.distance(spawn) < 16.0,
        "a replay left the body at {after:?}, not at the room spawn {spawn:?}"
    );
}

/// Case 5: what the ATTEMPT created does not survive the attempt, and does not
/// join the reconstructed authored population.
///
/// ⚠ The residue is placed directly rather than earned in a fight. The claim
/// under test is about RECONSTRUCTION, not about the drop road: what matters is
/// that the marker is the one the shipped drop road applies
/// (`features::ecs::SpawnedThisAttempt`, see `damage_drops::drop_currency_coin`)
/// and that the entity carries no room scope — so only the attempt-residue leg
/// of the retention policy can remove it, and a rebuild that swept it by
/// accident would not prove anything.
#[test]
fn a_replay_does_not_adopt_what_the_attempt_created() {
    let (mut sim, live) = enter_the_room();
    let fresh = census(&mut sim);

    let residue = sim
        .world_mut()
        .spawn(ambition_platformer2d::actors::features::ecs::SpawnedThisAttempt)
        .id();
    assert!(
        sim.world().get_entity(residue).is_ok(),
        "the fixture failed to place the attempt residue it is about to look for"
    );

    replay_the_room(&mut sim, &live);

    assert!(
        sim.world().get_entity(residue).is_err(),
        "the previous attempt's residue is still lying in the rebuilt room"
    );
    let replayed = census(&mut sim);
    assert_same_population("a replay over attempt residue", &fresh, &replayed);
}

/// Case 4b: RETENTION, the leg the room census cannot see.
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
    use ambition_platformer2d::actors::items::pickup::ItemCustody;

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
        .get::<ambition_platformer2d::item::GroundItem>(item)
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
    sim.world_mut()
        .write_message(ambition_platformer2d::actors::session::reset::RoomReplayRequested);
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

/// Case 7: running one lifecycle path then another lands in the same room.
#[test]
fn running_one_lifecycle_path_then_another_lands_in_the_same_room() {
    let (mut sim, live) = enter_the_room();
    let fresh = census(&mut sim);

    disturb_the_room(&mut sim);
    let live = replay_the_room(&mut sim, &live);
    let replayed_once = census(&mut sim);

    disturb_the_room(&mut sim);
    replay_the_room(&mut sim, &live);
    let replayed_twice = census(&mut sim);

    assert_same_population("a first replay", &fresh, &replayed_once);
    assert_same_population("a second replay", &fresh, &replayed_twice);
}
