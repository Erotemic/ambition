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

/// The active room's authored spawn — where every rebuild puts its subject.
fn room_spawn(sim: &mut Platformer2dSimHarness) -> Vec2 {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ae::RoomGeometry, With<ambition_platformer2d::platformer::lifecycle::SessionRoot>>();
    q.iter(world)
        .next()
        .expect("an active session publishes its room geometry")
        .0
        .spawn
}

/// Move the primary body through the movement authority.
fn displace_player(sim: &mut Platformer2dSimHarness, to: Vec2) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(
        ae::BodyClusterQueryData,
        &mut ambition_platformer2d::actor::MotionModel,
    ), With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
    let Some((mut cluster_item, mut model)) = q.iter_mut(world).next() else {
        panic!("gameplay has no primary player to displace");
    };
    let mut clusters = cluster_item.as_clusters_mut();
    ae::movement::transit_body(
        &mut model,
        &mut clusters,
        to,
        ae::movement::TransitVelocity::Zero,
    );
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
    sim.world_mut().write_message(
        ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual(),
    );
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

/// Case 4: the HOME AVATAR is not retired with the room, and comes back standing
/// at the room's own spawn.
///
/// ⚠ THIS IS ABOUT THE HOME AVATAR, and its name used to say "the body playing
/// the room" while querying only `PrimaryPlayer` — which is the home avatar and
/// not necessarily the body being driven. The claim it was overstating is
/// [`a_replay_follows_the_body_you_are_actually_driving`]'s.
#[test]
fn a_replay_leaves_the_home_avatar_standing_at_spawn() {
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

/// Case 6: RETENTION, the leg the room census cannot see.
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
    sim.world_mut().write_message(
        ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual(),
    );
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
    use ambition_platformer2d::actors::session::durable_horizon::SaveRestored;
    use ambition_platformer2d::persistence::save::AmbitionGameSave;

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

/// Case 7: a fresh-process load builds the room a session already standing in it
/// would build.
///
/// This is the fourth road in the model — new session, transition, replay,
/// durable restore — and the one that still builds its first room BEFORE the
/// save's occurrence facts are adopted, then corrects it. Measured 2026-08-30:
/// the correction lands on the same population, so the shape is a shape and not
/// a defect. This case is what keeps that true.
///
/// ⛔ THE PREMISE ARM IS NOT OPTIONAL. Comparing a load against a fresh entry
/// with an EMPTY save proves only that an empty file changes nothing. The second
/// arm carries a real durable deviation — an authored occurrence the file says is
/// lying in the room next door — and demands the loaded room and the re-entered
/// room agree about it.
#[test]
fn loading_a_save_builds_the_room_a_re_entry_builds() {
    use ambition_platformer2d::persistence::save::AmbitionGameSave;

    // ARM 1 — the empty file. A load of a world nobody has touched must leave the
    // room exactly as a session that never loaded anything has it.
    //
    // Aged to the same frame as the load, because a room whose enemies patrol is
    // only comparable to itself at the same age.
    let mut untouched = fixed_60hz_room_sim(ROOM);
    for _ in 0..(BEFORE_LOAD + AFTER_LOAD) {
        untouched.step(base());
    }
    let fresh = census(&mut untouched);
    let empty_file = untouched
        .world()
        .resource::<AmbitionGameSave>()
        .data()
        .clone();

    let mut loaded = boot_with_save(ROOM, &empty_file);
    assert_same_population("a fresh-process load", &fresh, &census(&mut loaded));
}

/// A room that authors occurrence-bearing objects, so a durable "it is lying
/// somewhere else" row has something to be about. [`ROOM`] is chosen for its
/// population; this one is chosen for its objects.
const DURABLE_ROOM: &str = "central_hub_complex";

/// Case 8: a durable fact the room must ACT on reaches both roads the same way.
///
/// The file says one of the room's own authored occurrences is lying next door.
/// Construction has to suppress it — minting a second one puts two live things
/// behind one identity — and it must not matter whether the room was built by a
/// boot or by walking back into it.
#[test]
fn a_relocated_occurrence_is_suppressed_by_a_load_and_by_a_re_entry_alike() {
    use ambition_platformer2d::persistence::save_data::{
        PersistedOccurrence, PersistedWhereabouts,
    };

    let (relocatable, neighbour, empty_file) = relocatable_occurrence();

    let mut relocated_file = empty_file.clone();
    relocated_file.occurrences = vec![PersistedOccurrence::new(
        relocatable.clone(),
        PersistedWhereabouts::Placed {
            room: neighbour.clone(),
            x: 200,
            y: 200,
        },
    )];

    // The BOOT road.
    let mut loaded = boot_with_save(DURABLE_ROOM, &relocated_file);
    let after_load = census(&mut loaded);
    assert!(
        !after_load.contains_key(&relocatable),
        "the load authored `{relocatable}` into '{DURABLE_ROOM}' even though the \
         file says it is lying in '{neighbour}' — two live things behind one \
         identity.\nCensus was:\n{}",
        render(&after_load)
    );

    // ⛔ THE PREMISE: without the row, the room DOES author it. Otherwise the
    // assertion above passes on a room that never had the object at all.
    let mut plain = boot_with_save(DURABLE_ROOM, &empty_file);
    assert!(
        census(&mut plain).contains_key(&relocatable),
        "'{DURABLE_ROOM}' does not author `{relocatable}` even with an empty file, \
         so suppressing it proves nothing"
    );

    // The RE-ENTRY road, under the same saved facts.
    let mut walked = boot_with_save(DURABLE_ROOM, &relocated_file);
    let live = population_entities(&mut walked);
    let away = walk_to(&mut walked, &neighbour);
    assert_eq!(away, neighbour);
    let live = settle_after_construction(&mut walked, &live);
    assert_eq!(walk_to(&mut walked, DURABLE_ROOM), DURABLE_ROOM);
    settle_after_construction(&mut walked, &live);

    let after_walk = census(&mut walked);
    assert!(
        !after_walk.contains_key(&relocatable),
        "walking back into '{DURABLE_ROOM}' authored `{relocatable}`, which the \
         file says is lying in '{neighbour}'"
    );
}

/// Case 8b: the room a load builds is built ONCE, and built right.
///
/// ⛔⛔ THIS IS THE ARM CASE 8 DOES NOT HAVE. Case 8 censuses after ninety
/// frames and finds the two roads agree — which establishes that the load
/// CONVERGES, and says nothing about the population it converges FROM. A load
/// used to construct its first room with no occurrence continuity at all and
/// correct it afterwards, so for a stretch of ticks the room held a live object
/// the file says is lying next door: two live things behind one identity, in a
/// window where combat, pickups and encounters all run ungated.
///
/// ⭐ SAMPLE EVERY FRAME, not the ends. "Does the population ever contain it"
/// is the question, and both endpoints answer no while the middle answers yes —
/// which is exactly the shape a two-endpoint test cannot see.
#[test]
fn a_load_never_authors_the_occurrence_it_is_about_to_suppress() {
    use ambition_platformer2d::actors::session::durable_horizon::SaveRestored;
    use ambition_platformer2d::persistence::save_data::{
        PersistedOccurrence, PersistedWhereabouts,
    };

    let (relocatable, neighbour, empty_file) = relocatable_occurrence();

    let mut relocated_file = empty_file.clone();
    relocated_file.occurrences = vec![PersistedOccurrence::new(
        relocatable.clone(),
        PersistedWhereabouts::Placed {
            room: neighbour.clone(),
            x: 200,
            y: 200,
        },
    )];

    // ⭐ BOOT WITH THE FILE, the way the binary does — bytes in the world before
    // the session activates. Writing the save into a RUNNING session measures
    // the correction road and can say nothing about what the first construction
    // knew.
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(DURABLE_ROOM).with_save(relocated_file),
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

    // ⛔ THE PREMISE: with an EMPTY file the room DOES author it, so "never
    // present" is a claim about the row and not about a room that has no such
    // object.
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
}

/// Case 8c: a save carrying BOTH a cross-room checkpoint and an occurrence row
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
/// ⭐ MEASURED, AND IT IS BENIGN FOR THIS SHAPE. Both roads ask for the SAME
/// destination and arrival, so whichever wins takes the session to the same
/// place; the only thing the loser drops is the replay announcement, whose
/// consumer is the attempt-residue sweep, and a session start has no previous
/// attempt to sweep. Poisoning EITHER road alone leaves this green — they are
/// redundant, not cooperating.
///
/// ⛔ SO THE POISON THAT REDDENS IT IS BOTH AT ONCE, and that is what makes this
/// a guard rather than a check that cannot fail: it pins the end-to-end
/// contract, which two roads happen to implement. Verified 2026-08-31 —
/// poisoning `routed_for` and the `ResetToCheckpoint` read together fails on the
/// active-room assertion.
///
/// ⛔ AND THE ROW MUST BE ABOUT THE ROOM WE RESUME INTO. The first version of
/// this case relocated an object of the room the session OPENS in, which the
/// destination never authors — so the suppression was guaranteed by arithmetic
/// and all three poisons left it green.
#[test]
fn a_save_with_a_checkpoint_and_an_occurrence_lands_both() {
    use ambition_platformer2d::actors::session::durable_horizon::SaveRestored;
    use ambition_platformer2d::persistence::save_data::{
        PersistedCheckpoint, PersistedOccurrence, PersistedWhereabouts,
    };

    // ⛔⛔ THE ROW MUST BE ABOUT THE ROOM WE RESUME INTO, not the one we open in.
    // A row naming an object the destination never authors is suppressed by
    // arithmetic rather than by the ledger, and the first version of this case
    // stayed green with BOTH legs poisoned because of exactly that.
    let (_, _, empty_file) = relocatable_occurrence();
    let (resume_room, resumed_object) = another_room_with_an_object();

    // The checkpoint names the NEIGHBOUR — a different room from the one the
    // session opens in — which is the only shape that makes
    // `restore_checkpoint_on_session_start` take the slot at all.
    let mut file = empty_file;
    file.checkpoint = Some(PersistedCheckpoint::new(resume_room.clone(), 200, 200));
    file.occurrences = vec![PersistedOccurrence::new(
        resumed_object.clone(),
        PersistedWhereabouts::Placed {
            room: DURABLE_ROOM.to_string(),
            x: 200,
            y: 200,
        },
    )];

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
        resume_room,
        "the checkpoint names '{resume_room}', so that is where the session must \
         resume — whichever road won the one lifecycle slot"
    );
    // ⭐ AND THE LEDGER ROW SURVIVED THE COLLISION. `resumed_object` is one of
    // the RESUME room's own authored objects, and the file says it is lying back
    // in the room the session opened in — so the room we resumed into must not
    // author it.
    assert!(
        !live_ids(&mut sim).contains(&resumed_object),
        "resuming into '{resume_room}' authored `{resumed_object}`, which the \
         file says is lying in '{DURABLE_ROOM}'"
    );

    // ⛔ THE PREMISE: with an empty file the resume room DOES author it, so the
    // suppression above is the ledger's doing and not the room's.
    let mut plain = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(&resume_room).with_save(Default::default()),
    )
    .expect("the harness boots with a save file");
    for _ in 0..AFTER_LOAD {
        plain.step(base());
    }
    assert!(
        live_ids(&mut plain).contains(&resumed_object),
        "'{resume_room}' does not author `{resumed_object}` even with an empty \
         file, so suppressing it proves nothing"
    );
}

/// A room OTHER than [`DURABLE_ROOM`] that authors an occurrence-bearing object,
/// and that object.
///
/// ⛔ SEARCHED, not hard-coded. The obvious candidate — the first room
/// `DURABLE_ROOM` has an exit to — turned out to author no such object, and a
/// case built on it would have asserted a suppression that arithmetic already
/// guaranteed.
fn another_room_with_an_object() -> (String, String) {
    let mut scout = fixed_60hz_room_sim(DURABLE_ROOM);
    let rooms: Vec<String> = {
        let world = scout.world_mut();
        let mut q = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        q.iter(world)
            .next()
            .expect("the session has an active room set")
            .rooms
            .iter()
            .map(|room| room.id.clone())
            .filter(|id| id != DURABLE_ROOM)
            .collect()
    };
    for room in rooms {
        let mut sim = fixed_60hz_room_sim(&room);
        for _ in 0..BEFORE_LOAD {
            sim.step(base());
        }
        let mut q = sim.world_mut().query::<(
            &SimId,
            &ambition_platformer2d::held_items::ItemCustody,
        )>();
        let world = sim.world();
        let mut ids: Vec<String> = q
            .iter(world)
            .map(|(id, _)| id.as_str().to_string())
            .collect();
        ids.sort();
        if let Some(object) = ids.into_iter().next() {
            return (room, object);
        }
    }
    panic!("no room besides '{DURABLE_ROOM}' authors an occurrence-bearing object");
}

/// The id of the room the session is standing in.
fn active_room(sim: &mut Platformer2dSimHarness) -> String {
    let world = sim.world_mut();
    let mut q = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
    q.iter(world)
        .next()
        .expect("the session has an active room set")
        .active_spec()
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
///
/// ⭐ ONE DEFINITION so the two durable cases cannot drift into relocating
/// different objects and calling it the same measurement.
fn relocatable_occurrence() -> (
    String,
    String,
    ambition_platformer2d::persistence::save_data::AmbitionGameSaveData,
) {
    relocatable_occurrence_in(DURABLE_ROOM)
}

/// [`relocatable_occurrence`] for any room, so a case can ask about the room it
/// RESUMES into rather than the one it opens in.
fn relocatable_occurrence_in(
    room: &str,
) -> (
    String,
    String,
    ambition_platformer2d::persistence::save_data::AmbitionGameSaveData,
) {
    use ambition_platformer2d::persistence::save::AmbitionGameSave;

    let mut played = fixed_60hz_room_sim(room);
    for _ in 0..(BEFORE_LOAD + AFTER_LOAD) {
        played.step(base());
    }
    let empty_file = played.world().resource::<AmbitionGameSave>().data().clone();

    let relocatable = {
        let mut q = played.world_mut().query::<(
            &SimId,
            &ambition_platformer2d::held_items::ItemCustody,
        )>();
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
        let mut q = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = q
            .iter(world)
            .next()
            .expect("the session has an active room set");
        room_set
            .active_loading_zones()
            .iter()
            .filter_map(|zone| {
                room_set
                    .transition_for_player(zone.aabb, ae::Vec2::ZERO, true)
                    .and_then(|t| room_set.rooms.get(t.target_room))
                    .map(|spec| spec.id.clone())
            })
            .find(|id| id.as_str() != room)
            .unwrap_or_else(|| panic!("'{room}' has no authored exit"))
    };
    (relocatable, neighbour, empty_file)
}

/// ⛔⛔ ZERO PARTIAL RESET. The one pending-lifecycle slot is earliest-sticky, so
/// a replay asked for while another lifecycle operation owns it does not happen
/// — and must therefore change NOTHING. Before admission was a decision, the
/// request reset the avatar, retired the previous attempt's residue and let
/// content clear its per-attempt state, and only then discovered the slot was
/// taken.
#[test]
fn a_replay_refused_by_the_lifecycle_slot_changes_nothing() {
    use ambition_platformer2d::actors::session::lifecycle_commit::{
        LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent,
    };

    let (mut sim, _) = enter_the_room();
    let fresh = census(&mut sim);

    // Attempt residue that only the replay's own sweep would take.
    let residue = sim
        .world_mut()
        .spawn(ambition_platformer2d::actors::features::ecs::SpawnedThisAttempt)
        .id();
    // Move the body off spawn, so a reset that ran would be visible.
    let displaced = {
        let spawn = room_spawn(&mut sim);
        let to = spawn + Vec2::new(240.0, 0.0);
        displace_player(&mut sim, to);
        sim.step(base());
        player_pos(&mut sim)
    };
    assert!(
        displaced.distance(room_spawn(&mut sim)) > 100.0,
        "the fixture must move the body off spawn before a refused reset can be \
         shown not to have moved it back"
    );

    // Another lifecycle operation takes the slot FIRST.
    sim.world_mut()
        .resource_mut::<PendingLifecycleCommit>()
        .pending = Some(
        ambition_platformer2d::actors::session::lifecycle_commit::PendingIntent {
            frame: 0,
            kind: LifecycleIntent::Transition(RoomTransitionIntent {
                subject: ambition_platformer2d::platformer::sim_id::SimId::placement(
                    "somebody-elses-crossing",
                ),
                target_room: ROOM.to_string(),
                arrival: Vec2::ZERO,
                edge_exit: false,
                zone_sfx: None,
            }),
        },
    );

    sim.world_mut().write_message(
        ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual(),
    );
    for _ in 0..8 {
        sim.step(base());
    }

    assert!(
        sim.world().get_entity(residue).is_ok(),
        "the refused replay retired the previous attempt's residue anyway"
    );
    let after = player_pos(&mut sim);
    assert!(
        after.distance(displaced) < 40.0,
        "the refused replay returned the body toward spawn anyway ({displaced:?} \
         -> {after:?}); the reset ran for an operation that never happened"
    );
    // ⛔ NO CENSUS COMPARISON HERE. This arm runs extra frames on purpose, and a
    // census compares two populations at the same AGE since construction — a
    // patrolling enemy that walked during those frames is not a reset. The two
    // assertions above are the claim: nothing the refused replay would have done
    // was done.
    let _ = fresh;
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
        ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual(),
    );
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

/// Case 9: running one lifecycle path then another lands in the same room.
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
/// here. `reset_cut_rope_attempt_on_replay` clears three durable facts by hand,
/// and a hand-kept list grows only when somebody notices — the property that
/// made `reset_ecs_room_features` a second constructor with sixteen queries.
/// This comparison cannot fall behind: adding a field to
/// `AmbitionGameSaveData` fails to compile until it is classified.
fn durable_families_that_differ(
    before: &ambition_platformer2d::persistence::save_data::AmbitionGameSaveData,
    after: &ambition_platformer2d::persistence::save_data::AmbitionGameSaveData,
) -> Vec<&'static str> {
    use ambition_platformer2d::persistence::save_data::AmbitionGameSaveData as D;
    // ⛔ THE DESTRUCTURE IS THE GUARD. Do not replace it with `before.field`
    // accesses: the compiler is what stops a fourteenth family from being added
    // without a decision about whether a replay may keep it.
    let D {
        version: _,
        encounters,
        switches,
        bosses,
        quests,
        flags,
        dialog_visits,
        items,
        wallet,
        inventory_saved,
        checkpoint,
        occurrences,
        custody,
        minted_items,
    } = before;
    let mut differ = Vec::new();
    let mut check = |name: &'static str, same: bool| {
        if !same {
            differ.push(name);
        }
    };
    check("encounters", *encounters == after.encounters);
    check("switches", *switches == after.switches);
    check("bosses", *bosses == after.bosses);
    check("quests", *quests == after.quests);
    check("flags", *flags == after.flags);
    check("dialog_visits", *dialog_visits == after.dialog_visits);
    check("items", *items == after.items);
    check("wallet", *wallet == after.wallet);
    check("inventory_saved", *inventory_saved == after.inventory_saved);
    check("checkpoint", *checkpoint == after.checkpoint);
    check("occurrences", *occurrences == after.occurrences);
    check("custody", *custody == after.custody);
    check("minted_items", *minted_items == after.minted_items);
    differ
}

fn durable_facts(
    sim: &Platformer2dSimHarness,
) -> ambition_platformer2d::persistence::save_data::AmbitionGameSaveData {
    use ambition_platformer2d::persistence::save::AmbitionGameSave;
    sim.world().resource::<AmbitionGameSave>().data().clone()
}

/// ⭐⭐ A REPLAY KEEPS SESSION PROGRESS — the durable half of "a replay
/// rebuilds rather than repairs", asserted on the side that is decidable today.
///
/// ⛔ COMPARING THE WHOLE SAVE WOULD BE THE WRONG TEST, and writing it that way
/// first is how this arm nearly went in backwards. A replay must NOT restore the
/// wallet, the quest ledger or the dialogue visit counts: those are session
/// progress, not attempt state, and asserting the two saves equal would pin a
/// policy nobody has decided and would red the moment a replay correctly KEPT
/// something.
///
/// ⚠ THE ATTEMPT SIDE IS DELIBERATELY NOT ASSERTED HERE, and the reason is a
/// scoping fact rather than caution. The one production consumer that clears
/// attempt state on replay — `reset_cut_rope_attempt_on_replay` — clears the
/// persisted record only for cut-rope placements PRESENT IN THE ROOM. So a test
/// that recorded a synthetic boss id and demanded a replay clear it would fail
/// against correct code, and would be pinning a policy nobody has written:
/// *which* attempt-scoped facts a replay owns is an open question, not a bug.
/// ⇒ What this arm does instead is REPORT the families a replay touches, so the
/// policy can be written from evidence. The report is an assertion-free
/// `eprintln!` on purpose: a number nobody has ruled on must not become a
/// ratchet by accident.
#[test]
fn a_replay_keeps_session_progress_and_reports_what_it_touches() {
    use ambition_platformer2d::persistence::save::AmbitionGameSave;

    let (mut sim, live) = enter_the_room();
    let fresh = durable_facts(&sim);

    // Session progress, earned during the attempt. A replay that refunded or
    // confiscated this would be a defect in the other direction.
    //
    // ⛔⛔ THE LIVE COMPONENT, NOT THE SAVE FIELD, and this arm's FIRST RUN
    // failed because it wrote the save. `AmbitionGameSaveData::wallet` is a
    // PROJECTION: `items::persist` mirrors `data.wallet = wallet.balance`
    // every frame the two differ, so a write to the save is overwritten from
    // the live `BodyWallet` on the next step — the test reported `0 vs 137`
    // and looked exactly like a replay confiscating the player's coins.
    // ⇒ Disturbing a projection and reading the result as a fact about the
    // subject is the failure this whole file exists to catch, and the
    // instrument committed it first.
    {
        let mut q = sim
            .world_mut()
            .query_filtered::<&mut ambition_platformer2d::characters::actor::BodyWallet, With<
                ambition_platformer2d::platformer::markers::PrimaryPlayer,
            >>();
        let world = sim.world_mut();
        let mut found = 0;
        for mut wallet in q.iter_mut(world) {
            wallet.balance += 137;
            found += 1;
        }
        assert_eq!(
            found, 1,
            "exactly one primary player carries the wallet this arm disturbs"
        );
    }
    // Let the mirror carry the live balance into the save before comparing.
    sim.step(base());
    // ⭐ A SECOND FAMILY, WITH THE OPPOSITE PLUMBING, so `[]` below is a claim
    // about the replay rather than about one field's mirror. `flags` has NO
    // live component behind it — `set_flag` writes the save and the save IS the
    // authority — so a direct write sticks where the wallet write did not.
    // ⇒ Two families reached by two different roads; a replay that quietly
    // rebuilt the whole save would show up in this one even if the mirror
    // happened to restore the other.
    {
        let mut save = sim.world_mut().resource_mut::<AmbitionGameSave>();
        save.data_mut()
            .set_flag("a_flag_set_during_this_attempt", true);
    }
    let disturbed = durable_facts(&sim);
    let landed = durable_families_that_differ(&fresh, &disturbed);
    assert!(
        landed.contains(&"wallet") && landed.contains(&"flags"),
        "the disturbance must land in BOTH families or this arm cannot fail \
         honestly; it landed in {landed:?}"
    );

    replay_the_room(&mut sim, &live);
    let after = durable_facts(&sim);

    assert_eq!(
        after.wallet, disturbed.wallet,
        "a replay is not a refund: coins held during the attempt are session \
         progress and must survive it"
    );
    eprintln!(
        "[durable-census] a replay of `{ROOM}` changed these durable families: {:?}",
        durable_families_that_differ(&disturbed, &after)
    );
}

/// ⭐⭐ THE ONE MECHANISM THAT RETRACTS ATTEMPT STATE, ASKED THROUGH THE ROAD A
/// GATE READS — the arm the durable census above says is missing.
///
/// The measurement above found that a replay changes ZERO durable families of
/// its own accord, so every attempt-scoped fact that must be retracted is
/// retracted by a CONTENT system that names it. Today there is exactly one:
/// `reset_cut_rope_attempt_on_replay`, which clears the persisted record for
/// cut-rope placements PRESENT IN THE ROOM on `RoomReplayAdmitted`.
///
/// ⛔ IT HAD NO END-TO-END ARM, and it is the only thing standing between a
/// retried fight and a permanently-open door. `boss.cleared` became a published
/// condition on 2026-09-04, so this record is now read by every `gated_by` and
/// every `<<if boss_cleared(...)>>`: a replay that left it set would rebuild the
/// boss — the population census would agree perfectly — while the world went on
/// believing the fight was won.
///
/// ⭐ ASKED THROUGH THE CATALOG, NOT THE SAVE FIELD, deliberately. Reading
/// `data.bosses` would prove the row changed; asking `boss.cleared` proves the
/// QUESTION a door asks changes with it, which is the property that matters and
/// the one a future refactor of the save shape could silently break.
#[test]
fn a_replay_retracts_the_boss_defeat_a_gate_would_have_read() {
    use ambition_platformer2d::boss_encounter::BossConfig;
    use ambition_platformer2d::persistence::save::AmbitionGameSave;
    use ambition_platformer2d::platformer::authored_logic::{
        AuthoredArg, ConditionCatalog, ConditionId, ConditionOutcome,
    };

    const BOSS_ROOM: &str = "you_have_to_cut_the_rope";
    let mut sim = fixed_60hz_room_sim(BOSS_ROOM);
    let live = settle_after_construction(&mut sim, &BTreeSet::new());

    // The placement id the room actually authored — not a synthetic one. The
    // production reset is scoped to placements present in the room, so a made-up
    // id would test nothing and would fail against correct code.
    let placement: String = {
        let mut q = sim.world_mut().query::<&BossConfig>();
        let world = sim.world();
        let ids: Vec<String> = q.iter(world).map(|config| config.id.clone()).collect();
        assert_eq!(
            ids.len(),
            1,
            "`{BOSS_ROOM}` is chosen because it authors exactly one boss; it \
             authored {ids:?}, so this arm no longer knows what it is asking about"
        );
        ids.into_iter().next().expect("one boss")
    };

    let ask = |sim: &mut Platformer2dSimHarness, placement: &str| -> ConditionOutcome {
        let id = ConditionId::parse("boss.cleared").expect("`boss.cleared` is a well-formed id");
        let world = sim.world_mut();
        world.resource_scope::<ConditionCatalog, _>(|world, catalog| {
            catalog.evaluate(world, &id, &[AuthoredArg::Name(placement.to_string())])
        })
    };

    assert!(
        matches!(ask(&mut sim, &placement), ConditionOutcome::NotSatisfied(_)),
        "a boss nobody has fought is not cleared; without this the arm below \
         cannot tell a retraction from a question that was never true"
    );

    // Record the defeat the victory beat records.
    {
        let mut save = sim.world_mut().resource_mut::<AmbitionGameSave>();
        save.data_mut().set_boss(
            placement.clone(),
            ambition_platformer2d::persistence::save_data::PersistedEncounterState::Cleared,
        );
    }
    assert_eq!(
        ask(&mut sim, &placement),
        ConditionOutcome::Satisfied,
        "the disturbance must actually make the gate's question answer YES, or \
         the replay below has nothing to retract"
    );

    replay_the_room(&mut sim, &live);

    assert!(
        matches!(ask(&mut sim, &placement), ConditionOutcome::NotSatisfied(_)),
        "the replay rebuilt the boss but left `boss.cleared` answering YES for \
         `{placement}`. The population census cannot see this — every entity \
         matches — while every door and dialogue branch gated on that boss stays \
         open for a fight that was undone."
    );
}

/// ⛔⛔ ONE BOSS FAMILY OF ELEVEN RETRACTS ITS DEFEAT ON A REPLAY. This arm
/// MEASURES the other ten rather than asserting a policy about them.
///
/// The shipped worlds author **eleven** `BossSpawn` placements — one in
/// `intro`, nine in `sandbox`, one in `you_have_to_cut_the_rope`. Exactly one
/// content system retracts a recorded defeat on `RoomReplayAdmitted`, and it is
/// scoped by name to cut-rope placements. ⇒ For the other ten, a defeat
/// recorded during an attempt survives the replay that undoes the attempt, and
/// `boss.cleared` — published 2026-09-04 — goes on answering YES to every
/// `gated_by` and `<<if boss_cleared(...)>>` for a fight the player is being
/// asked to fight again.
///
/// ⚠ WHETHER THAT IS A DEFECT IS A RULING, NOT A TEST, which is why this
/// reports instead of failing. A one-time story boss whose defeat SHOULD persist
/// across a room retry is a legitimate design; so is the opposite. Filed as a
/// maintainer decision. ⭐ What is NOT a matter of taste is that the two
/// behaviours are currently decided by which content author happened to write a
/// reset system, with nothing recording the choice — so this arm exists to make
/// the split visible and dated rather than to force it.
#[test]
fn how_many_boss_families_retract_their_defeat_on_a_replay() {
    use ambition_platformer2d::boss_encounter::BossConfig;
    use ambition_platformer2d::persistence::save::AmbitionGameSave;
    use ambition_platformer2d::platformer::authored_logic::{
        AuthoredArg, ConditionCatalog, ConditionId, ConditionOutcome,
    };

    // One room per behaviour: the family that retracts, and a family that has no
    // reset system of its own. Two points, not eleven, because each costs a room
    // boot and the question here is whether the SPLIT is real.
    const ROOMS: [&str; 2] = ["you_have_to_cut_the_rope", "mockingbird_arena"];

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
                    catalog.evaluate(world, &id, &[AuthoredArg::Name(placement.clone())])
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
