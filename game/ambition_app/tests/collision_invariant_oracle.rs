//! Collision-invariant oracle — a fuzz-driven *diagnostic* that surfaces the
//! out-of-bounds / clipped-into-a-wall movement bugs.
//!
//! The existing `fuzz_random_walker` asserts only "no panic / no NaN / alive"
//! and *deliberately permits* collision violations. This harness adds the
//! missing per-step invariant oracle on top of the same deterministic
//! `SandboxSim`: each tick it reads the player's live AABB and the room's
//! collision world and flags
//!   - **EmbeddedInSolid** — the player center sits inside a Solid block (the
//!     "teleported into a wall" / clipped-through signature),
//!   - **OutOfBounds** — the player center left the world bounds (above the
//!     ceiling is the bug Jon hit flying up; below the floor is usually a legit
//!     gap fall — the catalog labels the side so a human can tell them apart),
//!   - **Teleport** — a single-tick position jump larger than any legit move
//!     (blink is 150px, so the 250px threshold only catches genuine pops).
//!
//! **Why this is a diagnostic, not a hard CI gate** (deliberate, see the
//! `tech-debt-log` OOB entry + the "no-shadow assertion" TODO): OOB-via-authored-
//! gaps is *expected* in some rooms, and embed/teleport are exactly the OOB bugs
//! Jon has explicitly deferred to a non-autonomous fixing session. A hard assert
//! would either false-positive on gap rooms or red-light CI on a known-deferred
//! bug. So `collision_oracle_smoke` only proves the *harness* runs (and prints a
//! report); the comprehensive `collision_oracle_full_sweep` is `#[ignore]`d and
//! run on demand to produce the repro catalog:
//!
//! ```text
//! cargo test -p ambition_app --test collision_invariant_oracle \
//!     -- --ignored --nocapture
//! ```
//!
//! Each flagged step prints `(room, seed, tick, pos)` so it reproduces through
//! `cargo run -p ambition_app --bin rl_random_walker -- <STEPS> <SEED>` after a `--start-room`.

use ambition::engine_core as ae;
use ambition::engine_core::RoomGeometry;
use ambition_app::rl_sim::TimestepMode;
use ambition_app::{RandomWalkPolicy, SandboxSim, SandboxSimOptions};

// --- the oracle ---

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// §6.1 invariant 1 — center inside a Solid/BlinkWall **after carve
    /// subtraction**. A carved hole is not solid, so a body sunk into a portal
    /// aperture is legal and this test no longer sees the block it went through.
    EmbeddedInSolid,
    OutOfBoundsAbove,
    OutOfBoundsBelow,
    OutOfBoundsSide,
    Teleport,
    /// §6.1 invariant 4 — `NaN`/`inf` in pos or vel. Folded in from
    /// `fuzz_random_walker`, which asserts it but does not catalog it.
    NonFinite,
    /// §6.1 invariant 6 — a body ended BELOW a one-way it was supported by last
    /// frame, with no drop-through intent and no reset/room change. Admission is
    /// one-directional; silent fall-through is the historical bug.
    OneWayFallThrough,
    /// §6.1 invariant 2 — a straddling body's center sits inside AUTHORED solid
    /// material that is not part of the carve of the portal it straddles. The
    /// §7.6 class: it got into the wall through some other hole.
    StraddleOutsideCarve,
    /// §6.1 invariant 5 — two Class-B remaps applied to one body in one frame
    /// (§3.2's ordering invariant). A re-ordering bug, not a tolerated race.
    DoubleClassBRemap,
}

impl Kind {
    /// The §6.1 invariant number this kind reports. Part of the pinned minimum
    /// trace payload: `(seed, room id, tick, invariant #, body)`.
    fn invariant(self) -> u8 {
        match self {
            Kind::EmbeddedInSolid => 1,
            // The OOB family and Teleport are both invariant 3's margin test
            // (center outside the world AABB) split by side, plus the legacy
            // single-tick-jump probe that predates the numbering.
            Kind::OutOfBoundsAbove | Kind::OutOfBoundsBelow | Kind::OutOfBoundsSide => 3,
            Kind::Teleport => 3,
            Kind::NonFinite => 4,
            Kind::OneWayFallThrough => 6,
            Kind::StraddleOutsideCarve => 2,
            Kind::DoubleClassBRemap => 5,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Kind::EmbeddedInSolid => "EMBEDDED-IN-SOLID",
            Kind::OutOfBoundsAbove => "OOB-ABOVE-CEILING",
            Kind::OutOfBoundsBelow => "OOB-BELOW-FLOOR",
            Kind::OutOfBoundsSide => "OOB-SIDE",
            Kind::Teleport => "TELEPORT",
            Kind::NonFinite => "NON-FINITE-POS-OR-VEL",
            Kind::OneWayFallThrough => "ONE-WAY-FALL-THROUGH",
            Kind::StraddleOutsideCarve => "STRADDLE-OUTSIDE-CARVE",
            Kind::DoubleClassBRemap => "DOUBLE-CLASS-B-REMAP",
        }
    }
}

struct Violation {
    room: String,
    seed: u64,
    tick: u64,
    kind: Kind,
    pos: (f32, f32),
    detail: String,
    /// For an OOB: did the player cross a Solid boundary wall to get out
    /// (`Some(true)` = a real clip-through bug) or walk off an open edge
    /// (`Some(false)` = level-authoring, the edge is just open)? `None` for the
    /// embed/teleport kinds, which aren't boundary-relative.
    through_wall: Option<bool>,
    /// §6.2's pinned minimum payload: the `GeoId` of the geometry the invariant
    /// NAMES — the embedding block, the violated one-way. `None` where the
    /// invariant names no geometry (OOB, teleport, non-finite).
    geo: Option<ae::GeoId>,
}

/// True if `(x, y)` lies inside any Solid block — used to probe whether a Solid
/// boundary wall sits just inside the edge the player went OOB through.
fn point_in_solid(blocks: &[SolidBlock], x: f32, y: f32) -> bool {
    blocks
        .iter()
        .any(|b| x >= b.aabb.min.x && x <= b.aabb.max.x && y >= b.aabb.min.y && y <= b.aabb.max.y)
}

/// True if the center is past the world bounds (the same test the OOB check uses).
fn is_oob(pos: (f32, f32), world: (f32, f32)) -> bool {
    pos.0 < -OOB_MARGIN
        || pos.0 > world.0 + OOB_MARGIN
        || pos.1 < -OOB_MARGIN
        || pos.1 > world.1 + OOB_MARGIN
}

/// Margin (px) the player center must be *past* a face before we call it
/// embedded — keeps sub-pixel contact drift from false-positiving.
const EMBED_MARGIN: f32 = 2.0;
/// How far the center must clear the world bound before it's flagged OOB —
/// generous so edge-hugging at a legit exit doesn't trip.
const OOB_MARGIN: f32 = 16.0;
/// A single-tick jump past this is a genuine pop (blink carries 150px, dash less,
/// so this never fires on a legitimate ability).
const TELEPORT_PX: f32 = 250.0;

/// One embeddable block: its durable identity and its box.
#[derive(Clone)]
struct SolidBlock {
    geo: ae::GeoId,
    aabb: ae::Aabb,
}

/// **The COMPOSED collision world's** embeddable blocks — §6.1 invariant 1's
/// "after carve subtraction (the composed world is the truth — a carved hole is
/// not solid)".
///
/// This is the CC3 delta. The harness reads the canonical session-root `RoomGeometry`, the AUTHORED
/// geometry, and so reported a body sunk into a portal aperture as embedded in the
/// wall the portal had punched through. Composing the carves first makes the
/// invariant-1 exemption for a straddling body FALL OUT of the geometry instead of
/// needing a `PortalTransit` special case: the block it is "inside" no longer
/// exists there.
///
/// `BlinkWall` joins `Solid`, per the invariant's own wording. One-ways never do:
/// overlapping a one-way is explicitly legal (§6.1 "Explicitly legal").
fn solid_blocks(sim: &SandboxSim) -> Vec<SolidBlock> {
    let Some(room) =
        ambition::platformer::lifecycle::session_world_component::<RoomGeometry>(sim.world())
    else {
        return Vec::new();
    };
    let carves: Vec<ae::Aabb> = sim
        .world()
        .get_resource::<ambition::actors::features::FeatureEcsWorldOverlay>()
        .map(|o| o.portal_carves.clone())
        .unwrap_or_default();
    let composed = ambition::world::collision::world_with_portal_carves(&room.0, &carves);
    composed
        .blocks
        .iter()
        .filter(|b| {
            matches!(
                b.kind,
                ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
            )
        })
        .map(|b| SolidBlock {
            geo: b.id.clone(),
            aabb: b.aabb,
        })
        .collect()
}

/// The one-way platforms of the room the sim is in, with their identities. Read
/// from the AUTHORED geometry: a portal never carves a one-way (only solid host
/// kinds are carved for a body's benefit, and a one-way is not a host).
fn one_ways(sim: &SandboxSim) -> Vec<SolidBlock> {
    let Some(room) =
        ambition::platformer::lifecycle::session_world_component::<RoomGeometry>(sim.world())
    else {
        return Vec::new();
    };
    room.0
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, ae::BlockKind::OneWay))
        .map(|b| SolidBlock {
            geo: b.id.clone(),
            aabb: b.aabb,
        })
        .collect()
}

/// The room's AUTHORED (uncarved) solid material. Invariant 2 needs it: the
/// composed world used by invariant 1 has EVERY portal's carve subtracted, so a
/// body that slipped into the host wall through a *different* portal's hole
/// reads as "not in a solid" there. Against the authored wall it reads as what
/// it is.
fn authored_solid_blocks(sim: &SandboxSim) -> Vec<SolidBlock> {
    let Some(room) =
        ambition::platformer::lifecycle::session_world_component::<RoomGeometry>(sim.world())
    else {
        return Vec::new();
    };
    room.0
        .blocks
        .iter()
        .filter(|b| {
            matches!(
                b.kind,
                ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
            )
        })
        .map(|b| SolidBlock {
            geo: b.id.clone(),
            aabb: b.aabb,
        })
        .collect()
}

/// The player's entity, so the Class-B ledger can be read per body.
fn player_entity(sim: &mut SandboxSim) -> Option<ambition::bevy::prelude::Entity> {
    use ambition::bevy::prelude::{Entity, With};
    let mut q = sim
        .world_mut()
        .query_filtered::<Entity, With<ambition::actors::actor::PrimaryPlayer>>();
    let world = sim.world();
    q.iter(world).next()
}

/// **Invariant 2's read-model row.** The carve volume of the portal the player
/// is currently straddling, if it is straddling one.
///
/// `docs/planning/engine/collision-and-ccd.md` §6.1 said this needed "a
/// read-model row" that did not exist. It did exist: `PortalTransit.straddling`
/// has always named the channel — what was missing was a caller. The hole is the
/// same `pieces::carve_hole` that `publish_portal_carves` pushes, so the oracle
/// tests against the exact geometry the sim carved.
#[cfg(feature = "portal")]
fn straddled_carve(sim: &mut SandboxSim) -> Option<ae::Aabb> {
    use ambition::bevy::prelude::With;
    use ambition::portal::{find_portal, PlacedPortal, PortalTransit};

    let channel = {
        let mut q = sim
            .world_mut()
            .query_filtered::<&PortalTransit, With<ambition::actors::actor::PrimaryPlayer>>();
        let world = sim.world();
        q.iter(world).next().map(|t| t.straddling)?
    };
    let portals: Vec<PlacedPortal> = {
        let mut q = sim.world_mut().query::<&PlacedPortal>();
        let world = sim.world();
        q.iter(world).cloned().collect()
    };
    let enter = find_portal(&portals, channel)?;
    Some(ambition::portal::pieces::carve_hole(&enter.aperture()))
}

#[cfg(not(feature = "portal"))]
fn straddled_carve(_sim: &mut SandboxSim) -> Option<ae::Aabb> {
    None
}

/// Every Class-B remap the engine applied to `body` on the tick that just ran
/// (`collision-and-ccd.md` §3.2), newest last. Empty when the ledger is absent.
///
/// This is invariant 5's countable event — and it is also what finally retires
/// the TELEPORT false positive: a body the transit authority legally warped did
/// not "pop".
fn class_b_remaps(
    sim: &SandboxSim,
    body: Option<ambition::bevy::prelude::Entity>,
) -> Vec<ambition::platformer::class_b::ClassBRemap> {
    let (Some(log), Some(body)) = (
        sim.world()
            .get_resource::<ambition::platformer::class_b::ClassBRemapLog>(),
        body,
    ) else {
        return Vec::new();
    };
    log.kinds_for(body).collect()
}

/// The player's live body: center, velocity, half-extent. The oracle needs the
/// half-height to know where its FEET are, and the velocity for invariant 4.
fn player_body(sim: &mut SandboxSim) -> Option<(ae::Vec2, ae::Vec2, ae::Vec2)> {
    use ambition::bevy::prelude::With;
    let mut q = sim.world_mut().query_filtered::<
        &ambition::actors::actor::BodyKinematics,
        With<ambition::actors::actor::PrimaryPlayer>,
    >();
    let world = sim.world();
    q.iter(world).next().map(|k| (k.pos, k.vel, k.size * 0.5))
}

/// Which one-way (if any) the body is STANDING ON this frame: its feet are within
/// a contact epsilon of the platform's top, its center is above that top, and it
/// is horizontally over the platform.
fn supported_one_way(
    center: ae::Vec2,
    half: ae::Vec2,
    platforms: &[SolidBlock],
) -> Option<SolidBlock> {
    /// y grows downward; feet are the +gravity face (default gravity is +y).
    const CONTACT_EPS: f32 = 2.0;
    let feet_y = center.y + half.y;
    platforms
        .iter()
        .find(|p| {
            center.x > p.aabb.min.x
                && center.x < p.aabb.max.x
                && center.y < p.aabb.min.y
                && (feet_y - p.aabb.min.y).abs() <= CONTACT_EPS
        })
        .cloned()
}

/// `room id -> its LoadingZone (edge-exit / door) AABBs`, loaded once from the
/// LDtk project. An OOB that lands at one of these is the player legitimately
/// leaving through an opening, not clipping a solid boundary — the cross-check
/// that turns the raw OOB-SIDE noise into "OOB through a wall with no exit".
fn load_loading_zones() -> std::collections::HashMap<String, Vec<ae::Aabb>> {
    let mut map = std::collections::HashMap::new();
    let Ok(project) = load_project_for_test() else {
        return map;
    };
    let Ok(room_set) = project.to_room_set() else {
        return map;
    };
    for room in &room_set.rooms {
        map.insert(
            room.id.clone(),
            room.loading_zones.iter().map(|z| z.aabb).collect(),
        );
    }
    map
}

/// Everything invariants 2 and 5 need about this tick's Class-B activity, bundled
/// so `check_step` stays under a readable arity.
#[derive(Default)]
struct TransitContext<'a> {
    /// The AUTHORED (uncarved) solid material — invariant 2's reference wall.
    authored: &'a [SolidBlock],
    /// The carve volume of the portal the body straddles, if it straddles one.
    straddled_carve: Option<ae::Aabb>,
    /// The Class-B remaps applied to this body on this tick, in order.
    remaps: &'a [ambition::platformer::class_b::ClassBRemap],
}

/// Check one post-tick observation against the invariants. `teleport_from` is
/// the prior tick's center for the teleport test — passed as `None` whenever the
/// room changed or the player respawned this tick (a door load or a death→spawn
/// is a *legitimate* large jump, not a pop), so teleport only fires on a genuine
/// same-room in-place warp. Embed/OOB always run on the current pos.
#[allow(clippy::too_many_arguments)]
fn check_step(
    room: &str,
    seed: u64,
    tick: u64,
    pos: (f32, f32),
    vel: Option<(f32, f32)>,
    teleport_from: Option<(f32, f32)>,
    world_size: (f32, f32),
    blocks: &[SolidBlock],
    loading_zones: &[ae::Aabb],
    // The one-way this body stood on LAST frame, if any, plus whether it pressed
    // down this frame (drop-through intent) and whether a reset/room change
    // occurred. Invariant 6 needs all three.
    one_way_ctx: Option<(&SolidBlock, bool, bool)>,
    transit: &TransitContext<'_>,
    suppressed: &mut u32,
) -> Vec<Violation> {
    let mut out = Vec::new();
    let (px, py) = pos;

    // §6.1 invariant 4 — NaN/inf in pos or vel. `fuzz_random_walker` asserts it;
    // the oracle CATALOGS it, because a non-finite body makes every other
    // invariant meaningless and the repro line is what a fixer needs.
    let vel_finite = vel.is_none_or(|(vx, vy)| vx.is_finite() && vy.is_finite());
    if !px.is_finite() || !py.is_finite() || !vel_finite {
        out.push(Violation {
            room: room.to_string(),
            seed,
            tick,
            kind: Kind::NonFinite,
            pos,
            detail: format!("pos=({px:?},{py:?}) vel={vel:?}"),
            through_wall: None,
            geo: None,
        });
        return out; // every downstream test is meaningless on a non-finite body
    }

    // §6.1 invariant 1 — center strictly inside a Solid/BlinkWall by > EMBED_MARGIN,
    // in the COMPOSED world (carves already subtracted by `solid_blocks`).
    for b in blocks {
        let a = b.aabb;
        if px > a.min.x + EMBED_MARGIN
            && px < a.max.x - EMBED_MARGIN
            && py > a.min.y + EMBED_MARGIN
            && py < a.max.y - EMBED_MARGIN
        {
            out.push(Violation {
                room: room.to_string(),
                seed,
                tick,
                kind: Kind::EmbeddedInSolid,
                pos,
                detail: format!(
                    "center inside solid [{:.0},{:.0}]..[{:.0},{:.0}]",
                    a.min.x, a.min.y, a.max.x, a.max.y
                ),
                through_wall: None,
                geo: Some(b.geo.clone()),
            });
            break; // one embed report per step is enough
        }
    }

    // §6.1 invariant 2 — straddle-outside-carve. A body mid-transit legitimately
    // has its center behind the plane, INSIDE the carved aperture. What is never
    // legal is being inside the host material anywhere ELSE: it means the body
    // reached the wall's interior through some other opening (the §7.6 class).
    //
    // Invariant 1 cannot see this. It tests the COMPOSED world, where every
    // portal's carve has been subtracted — including the one the body did not
    // enter through. Invariant 2 tests the AUTHORED wall against the ONE carve
    // the body is entitled to.
    if let Some(carve) = transit.straddled_carve {
        let inside_own_carve =
            px >= carve.min.x && px <= carve.max.x && py >= carve.min.y && py <= carve.max.y;
        if !inside_own_carve {
            for b in transit.authored {
                let a = b.aabb;
                if px > a.min.x + EMBED_MARGIN
                    && px < a.max.x - EMBED_MARGIN
                    && py > a.min.y + EMBED_MARGIN
                    && py < a.max.y - EMBED_MARGIN
                {
                    out.push(Violation {
                        room: room.to_string(),
                        seed,
                        tick,
                        kind: Kind::StraddleOutsideCarve,
                        pos,
                        detail: format!(
                            "straddling, but center is in solid [{:.0},{:.0}]..[{:.0},{:.0}] \
                             outside its carve [{:.0},{:.0}]..[{:.0},{:.0}]",
                            a.min.x,
                            a.min.y,
                            a.max.x,
                            a.max.y,
                            carve.min.x,
                            carve.min.y,
                            carve.max.x,
                            carve.max.y
                        ),
                        through_wall: None,
                        geo: Some(b.geo.clone()),
                    });
                    break;
                }
            }
        }
    }

    // §6.1 invariant 5 — at most one Class-B remap per body per frame (§3.2).
    // The ledger is the countable event; two entries is the violation, and their
    // ORDER says which kind of bug: a stronger authority applying second means a
    // sweep sample was not reset, a weaker one means the schedule is misordered.
    if transit.remaps.len() > 1 {
        let first = transit.remaps[0];
        let second = transit.remaps[1];
        out.push(Violation {
            room: room.to_string(),
            seed,
            tick,
            kind: Kind::DoubleClassBRemap,
            pos,
            detail: format!(
                "{} then {} ({})",
                first.label(),
                second.label(),
                if second.wins_over(first) {
                    "priority-correct order, so the FIRST remap should have been \
                     voided by the sample reset (§3.1 rule 2)"
                } else {
                    "priority-INVERTED: the weaker authority ran last and won"
                }
            ),
            through_wall: None,
            geo: None,
        });
    }

    // §6.1 invariant 6 — one-way fall-through. Admission is one-directional: a
    // body supported by a one-way last frame may leave upward or sideways, and may
    // drop through ON PURPOSE, but must never end below it silently.
    if let Some((platform, pressed_down, remapped)) = one_way_ctx {
        let top = platform.aabb.min.y;
        // y grows downward, so "below the platform" is a larger y.
        if !pressed_down && !remapped && py > top + EMBED_MARGIN {
            out.push(Violation {
                room: room.to_string(),
                seed,
                tick,
                kind: Kind::OneWayFallThrough,
                pos,
                detail: format!(
                    "stood on one-way top y={top:.0} last tick, now center y={py:.0} \
                     with no drop-through intent"
                ),
                through_wall: None,
                geo: Some(platform.geo.clone()),
            });
        }
    }

    // 2. Out of bounds: center clearly outside [0, world] (+Y is down).
    let (ww, wh) = world_size;
    let kind = if py < -OOB_MARGIN {
        Some(Kind::OutOfBoundsAbove)
    } else if py > wh + OOB_MARGIN {
        Some(Kind::OutOfBoundsBelow)
    } else if px < -OOB_MARGIN || px > ww + OOB_MARGIN {
        Some(Kind::OutOfBoundsSide)
    } else {
        None
    };
    if let Some(kind) = kind {
        // An OOB at an authored exit (edge-exit / door) is legit traversal, not a
        // clip — the player is leaving through the opening. Expand the zone so a
        // center a half-body past the edge still reads as "at the exit".
        const EXIT_PAD: f32 = 48.0;
        let at_exit = loading_zones.iter().any(|z| {
            px > z.min.x - EXIT_PAD
                && px < z.max.x + EXIT_PAD
                && py > z.min.y - EXIT_PAD
                && py < z.max.y + EXIT_PAD
        });
        // Only count a FRESH crossing (inside last tick → outside now). Staying
        // outside (drift under gravity) just continues the same event — it would
        // double-count AND mis-flag the through-wall probe at a drifted-to coord
        // (the exact false positive the under_town_pipes trace exposed).
        let was_inside = teleport_from.is_some_and(|prev| !is_oob(prev, (ww, wh)));
        if !was_inside {
            // Drift continuation or a room transition — not a new boundary cross.
        } else if at_exit {
            *suppressed += 1;
        } else {
            // Probe just inside the crossed edge: a Solid there means the player
            // clipped THROUGH a boundary wall (a real bug); nothing there means
            // the edge is simply open (level-authoring — a pit, a sky boundary).
            const PROBE: f32 = 8.0;
            let through_wall = match kind {
                Kind::OutOfBoundsAbove => point_in_solid(blocks, px, PROBE),
                Kind::OutOfBoundsBelow => point_in_solid(blocks, px, wh - PROBE),
                Kind::OutOfBoundsSide if px < 0.0 => point_in_solid(blocks, PROBE, py),
                Kind::OutOfBoundsSide => point_in_solid(blocks, ww - PROBE, py),
                _ => false,
            };
            out.push(Violation {
                room: room.to_string(),
                seed,
                tick,
                kind,
                pos,
                detail: format!("world [{ww:.0}x{wh:.0}]"),
                through_wall: Some(through_wall),
                geo: None,
            });
        }
    }

    // 3. Teleport: a single-tick jump no legit move can produce. The caller passes
    // `None` across room loads / respawns; the Class-B ledger covers the rest —
    // §6.1's "Explicitly legal" list names *the transfer frame's position jump*,
    // and a body that any Class-B authority remapped this tick took exactly that.
    // Before the ledger existed this probe reported every portal transit as a
    // 290px TELEPORT. That was the one false positive in the 64,800-frame sweep.
    if let Some((qx, qy)) = teleport_from.filter(|_| transit.remaps.is_empty()) {
        let d = ((px - qx).powi(2) + (py - qy).powi(2)).sqrt();
        if d > TELEPORT_PX {
            out.push(Violation {
                room: room.to_string(),
                seed,
                tick,
                kind: Kind::Teleport,
                pos,
                detail: format!("jumped {d:.0}px from ({qx:.0},{qy:.0})"),
                through_wall: None,
                geo: None,
            });
        }
    }
    out
}

/// Run one (start_room, seed) episode of `steps` ticks, collecting violations.
/// Violations are labelled with the room the player is actually in (not the
/// start room) so a transition mid-episode attributes correctly. Returns
/// `(violations, steps_actually_run, oob_suppressed_at_authored_exits)`.
fn run_episode(
    start_room: &str,
    seed: u64,
    steps: u64,
    zones: &std::collections::HashMap<String, Vec<ae::Aabb>>,
) -> (Vec<Violation>, u64, u32) {
    let opts = SandboxSimOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_start_room(start_room);
    let Ok(mut sim) = SandboxSim::new_with_options(opts) else {
        return (Vec::new(), 0, 0);
    };
    let mut policy = RandomWalkPolicy::traversal_stress(seed);
    let first = sim.observation();
    let mut prev_pos = first.player_pos;
    let mut prev_room = first.active_room.clone();
    let mut prev_resets = first.resets;
    // Invariant 6's carry: the one-way the body stood on at the END of last tick.
    let mut prev_support: Option<SolidBlock> = None;
    let mut violations = Vec::new();
    let mut ran = 0;
    let mut suppressed = 0;
    let empty: Vec<ae::Aabb> = Vec::new();
    for _ in 0..steps {
        let action = policy.act();
        // Drop-through INTENT is a held descend axis — the same threshold the
        // body-mode/possession gestures use. Sampled from the action we are about
        // to submit, because that is the input the tick will consume.
        let pressed_down = action.move_y > 0.35;
        let obs = sim.step(action);
        ran += 1;
        let blocks = solid_blocks(&sim);
        let authored = authored_solid_blocks(&sim);
        let platforms = one_ways(&sim);
        let body = player_body(&mut sim);
        let subject = player_entity(&mut sim);
        let carve = straddled_carve(&mut sim);
        let remaps = class_b_remaps(&sim, subject);
        let room_zones = zones.get(&obs.active_room).unwrap_or(&empty);
        // A door load or a death→spawn respawn is a legit large jump — only feed
        // the teleport test a prior pos when neither happened this tick. It is
        // ALSO invariant 6's "Class-B remap" escape hatch: a body the engine
        // teleported did not fall through anything. `remaps` is the same escape
        // hatch made precise (§3.2) — the observation-derived `transitioned` flag
        // stays because a room load also swaps the geometry the carry refers to.
        let transitioned = obs.active_room != prev_room || obs.resets != prev_resets;
        let teleport_from = (!transitioned).then_some(prev_pos);
        let remapped = transitioned || !remaps.is_empty();
        let one_way_ctx = prev_support.as_ref().map(|p| (p, pressed_down, remapped));
        let transit_ctx = TransitContext {
            authored: &authored,
            straddled_carve: carve,
            remaps: &remaps,
        };
        violations.extend(check_step(
            &obs.active_room,
            seed,
            obs.tick,
            obs.player_pos,
            body.map(|(_, v, _)| (v.x, v.y)),
            teleport_from,
            obs.world_size,
            &blocks,
            room_zones,
            one_way_ctx,
            &transit_ctx,
            &mut suppressed,
        ));
        // Carry this tick's support forward. A room change invalidates it (the
        // platform belongs to the room we left).
        prev_support = if transitioned {
            None
        } else {
            body.and_then(|(pos, _, half)| supported_one_way(pos, half, &platforms))
        };
        prev_pos = obs.player_pos;
        prev_room = obs.active_room.clone();
        prev_resets = obs.resets;
    }
    (violations, ran, suppressed)
}

/// Group violations into a stable, diff-friendly report: per (room, kind) a
/// count plus the first repro. `suppressed` = OOB events that landed at an
/// authored exit (legit traversal, filtered out of the catalog).
fn format_report(
    violations: &[Violation],
    episodes: u64,
    total_steps: u64,
    suppressed: u32,
) -> String {
    use std::collections::BTreeMap;
    // A label that folds in the through-wall classification so a genuine
    // clip-through (bug) buckets separately from an open-edge walk-off (design).
    let label = |v: &Violation| -> String {
        match v.through_wall {
            // A Solid sits just inside the crossed edge. SUSPECT clip-through —
            // but a heuristic: the player could also have left through a gap and
            // drifted (while OOB) to a coordinate that happens to be walled. The
            // `?` flags "investigate", not "confirmed bug".
            Some(true) => format!("{} [past-solid?]", v.kind.label()),
            Some(false) => format!("{} (open edge)", v.kind.label()),
            None => v.kind.label().to_string(),
        }
    };
    let mut buckets: BTreeMap<(String, String), (u64, &Violation)> = BTreeMap::new();
    for v in violations {
        buckets
            .entry((v.room.clone(), label(v)))
            .and_modify(|(n, _)| *n += 1)
            .or_insert((1, v));
    }
    // Split the OOB by whether a Solid sat at the crossed edge: `past_solid` are
    // the SUSPECT clip-throughs to investigate (vs walking off an open edge,
    // which is level-authoring, not a physics bug).
    let past_solid = violations
        .iter()
        .filter(|v| v.through_wall == Some(true))
        .count();
    let mut s = String::new();
    s.push_str(&format!(
        "\n=== collision-invariant oracle: {episodes} episodes, {total_steps} steps, {} violations ({suppressed} OOB suppressed at authored exits; {past_solid} OOB ended up PAST a Solid at the crossed edge — suspect clips, investigate) ===\n",
        violations.len()
    ));
    if buckets.is_empty() {
        s.push_str("  no invariant violations — clean sweep.\n");
        return s;
    }
    for ((room, kind), (count, first)) in &buckets {
        // §6.2's pinned MINIMUM payload: (seed, room id, tick, invariant #, body)
        // + the GeoId of the geometry the invariant names, where it names one.
        // Richer channels (sweep samples, Class-B events, portal crossings) JOIN
        // this line in the slice that creates them — never speculatively.
        let geo = match &first.geo {
            Some(id) => format!(" geo={:?}#{}", id.source, id.index),
            None => String::new(),
        };
        s.push_str(&format!(
            "  {room:28} {kind:28} x{count:<4} first: inv={} seed={} tick={} pos=({:.0},{:.0}){geo} {}\n",
            first.kind.invariant(),
            first.seed,
            first.tick,
            first.pos.0,
            first.pos.1,
            first.detail
        ));
    }
    s
}

/// The through-wall classifier: an OOB past a Solid boundary wall is a real
/// clip-through; an OOB off an open edge is level-authoring. (PROBE-based, so it
/// reads the same boundary the player crossed.)
#[test]
fn oob_classifies_through_wall_vs_open_edge() {
    let world = (100.0, 100.0); // ww = wh = 100
    let mut supp = 0;
    let oob_right = (130.0, 50.0); // center well past the right edge (> ww + margin)

    // A Solid boundary wall at x[92,100]: being at x=130 means clipping through it.
    let walled = [SolidBlock {
        geo: ae::GeoId::anon(),
        aabb: ae::Aabb::new(ae::Vec2::new(96.0, 50.0), ae::Vec2::new(4.0, 50.0)),
    }];
    let v = check_step(
        "r",
        1,
        1,
        oob_right,
        None,
        Some((90.0, 50.0)),
        world,
        &walled,
        &[],
        None,
        &TransitContext::default(),
        &mut supp,
    );
    let side = v.iter().find(|x| matches!(x.kind, Kind::OutOfBoundsSide));
    assert_eq!(
        side.and_then(|x| x.through_wall),
        Some(true),
        "past a boundary wall = clip-through (bug)"
    );

    // No wall at the right edge — the player walked off an open edge (design).
    let v = check_step(
        "r",
        1,
        1,
        oob_right,
        None,
        Some((90.0, 50.0)),
        world,
        &[],
        &[],
        None,
        &TransitContext::default(),
        &mut supp,
    );
    let side = v.iter().find(|x| matches!(x.kind, Kind::OutOfBoundsSide));
    assert_eq!(
        side.and_then(|x| x.through_wall),
        Some(false),
        "open edge = not a clip (level-authoring)"
    );

    // Drift: already OOB last tick (prev x=120 is also past the edge) → the same
    // event continuing, not a fresh crossing → NOT recorded. This is the gate that
    // kills the under_town_pipes [past-solid?] false positives.
    let v = check_step(
        "r",
        1,
        1,
        oob_right,
        None,
        Some((120.0, 50.0)),
        world,
        &walled,
        &[],
        None,
        &TransitContext::default(),
        &mut supp,
    );
    assert!(
        v.iter().all(|x| !matches!(x.kind, Kind::OutOfBoundsSide)),
        "a drift continuation (already OOB last tick) is not a new OOB event"
    );
}

/// Diagnostic: trace the player trajectory around the worst `[past-solid?]` OOB
/// (under_town_pipes seed=1, ~tick 99) to settle clip-through vs drift-after-gap.
/// A real clip shows a single big jump from inside (x>0) to outside (x<0) crossing
/// a Solid face; drift shows the center easing left over many ticks (already OOB).
/// Run: `... --test collision_invariant_oracle trace_oob -- --ignored --nocapture`.
#[test]
#[ignore = "diagnostic trace — run with --ignored --nocapture"]
fn trace_oob_under_town_pipes() {
    let opts = SandboxSimOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_start_room("under_town_pipes");
    let mut sim = SandboxSim::new_with_options(opts).expect("sim");
    let mut policy = RandomWalkPolicy::traversal_stress(1);
    let mut prev = sim.observation().player_pos;
    for tick in 1..=110u64 {
        let action = policy.act();
        let obs = sim.step(action);
        let (px, py) = obs.player_pos;
        if (90..=105).contains(&tick) {
            let dx = px - prev.0;
            // Probe the left edge at this y (where the classifier flagged a Solid).
            let left_solid = point_in_solid(&solid_blocks(&sim), 8.0, py);
            eprintln!(
                "tick {tick}: pos=({px:.0},{py:.0}) dx={dx:+.0} left_edge_solid@y={left_solid} room={}",
                obs.active_room
            );
        }
        prev = (px, py);
    }
}

/// Smoke test: proves the oracle harness runs end-to-end and prints a report.
/// Does NOT assert zero violations (embed/teleport/OOB are the deferred bugs +
/// gap rooms — see the module docs). Fast: a couple seeds on the cold-launch room.
#[test]
fn collision_oracle_smoke() {
    let zones = load_loading_zones();
    let mut all = Vec::new();
    let mut total_steps = 0;
    let mut episodes = 0;
    let mut suppressed = 0;
    // The empty start_room string means "keep the LDtk-authored start room".
    for seed in [1_u64, 7] {
        let (mut v, ran, supp) = run_episode("", seed, 200, &zones);
        assert!(ran > 0, "the oracle episode must actually step the sim");
        total_steps += ran;
        episodes += 1;
        suppressed += supp;
        all.append(&mut v);
    }
    eprintln!("{}", format_report(&all, episodes, total_steps, suppressed));
    // Harness liveness only — the sim stepped without panicking across the run.
    assert_eq!(episodes, 2);
}

/// Comprehensive on-demand catalog: every room, several seeds, longer episodes.
/// `#[ignore]` so it never gates CI (it WILL surface the deferred OOB bugs); run
/// it to produce the repro list for the non-autonomous OOB-fixing session.
#[test]
#[ignore = "diagnostic catalog — run with --ignored --nocapture; surfaces deferred OOB bugs"]
fn collision_oracle_full_sweep() {
    let rooms = SandboxSim::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("SandboxSim::new should succeed")
        .room_ids();
    assert!(
        !rooms.is_empty(),
        "no rooms — the sweep would pass vacuously"
    );

    let zones = load_loading_zones();
    let seeds = [1_u64, 42, 2026];
    let mut all = Vec::new();
    let mut total_steps = 0;
    let mut episodes = 0;
    let mut suppressed = 0;
    for room in &rooms {
        for &seed in &seeds {
            let (mut v, ran, supp) = run_episode(room, seed, 300, &zones);
            total_steps += ran;
            episodes += 1;
            suppressed += supp;
            all.append(&mut v);
        }
    }
    eprintln!("{}", format_report(&all, episodes, total_steps, suppressed));
    eprintln!(
        "swept {} rooms x {} seeds; see per-(room,kind) first-repro above.",
        rooms.len(),
        seeds.len()
    );
}

/// Load the game's merged LDtk project the way a sim entry point does:
/// install the world manifest first — post-R3.2 the engine ships no worlds
/// and panics without a provider-owned manifest.
fn load_project_for_test() -> Result<ambition::actors::ldtk_world::LdtkProject, String> {
    ambition_content::worlds::install();
    ambition::actors::ldtk_world::LdtkProject::load_default_for_dev()
}

/// §6.1 invariant 4 — a non-finite body is CATALOGED, and short-circuits the rest
/// (every geometric test is meaningless once pos or vel is `NaN`).
#[test]
fn a_non_finite_body_reports_invariant_4_and_nothing_else() {
    let mut supp = 0;
    let v = check_step(
        "r",
        1,
        1,
        (f32::NAN, 50.0),
        Some((0.0, 0.0)),
        None,
        (100.0, 100.0),
        &[],
        &[],
        None,
        &TransitContext::default(),
        &mut supp,
    );
    assert_eq!(v.len(), 1, "exactly one violation: {:?}", v.len());
    assert!(matches!(v[0].kind, Kind::NonFinite));
    assert_eq!(v[0].kind.invariant(), 4);

    // A non-finite VELOCITY counts too, even with a finite position.
    let v = check_step(
        "r",
        1,
        1,
        (50.0, 50.0),
        Some((f32::INFINITY, 0.0)),
        None,
        (100.0, 100.0),
        &[],
        &[],
        None,
        &TransitContext::default(),
        &mut supp,
    );
    assert_eq!(v.len(), 1);
    assert!(matches!(v[0].kind, Kind::NonFinite));
}

/// §6.1 invariant 6 — one-way admission is one-directional.
///
/// The historical bug is a SILENT fall-through. Dropping through on purpose, and
/// being remapped by the engine (a room load / respawn — the Class-B escape
/// hatch), are both legal and must not fire.
#[test]
fn a_silent_one_way_fall_through_reports_invariant_6() {
    let mut supp = 0;
    let world = (200.0, 200.0);
    // A one-way whose TOP is y=100 (y grows downward), spanning x[40,160].
    let platform = SolidBlock {
        geo: ae::GeoId::anon(),
        aabb: ae::Aabb::new(ae::Vec2::new(100.0, 110.0), ae::Vec2::new(60.0, 10.0)),
    };
    let below = (100.0, 130.0); // center well under the platform's top

    let fire = |pressed_down: bool, remapped: bool, supp: &mut u32| {
        check_step(
            "r",
            1,
            1,
            below,
            Some((0.0, 50.0)),
            None,
            world,
            &[],
            &[],
            Some((&platform, pressed_down, remapped)),
            &TransitContext::default(),
            supp,
        )
    };

    let v = fire(false, false, &mut supp);
    let hit = v
        .iter()
        .find(|x| matches!(x.kind, Kind::OneWayFallThrough))
        .expect("a body that stood on the one-way and ended below it, silently");
    assert_eq!(hit.kind.invariant(), 6);
    assert_eq!(
        hit.geo,
        Some(ae::GeoId::anon()),
        "§6.2: the dump names the geometry the invariant names"
    );

    // Dropping through ON PURPOSE is the feature, not the bug.
    assert!(fire(true, false, &mut supp)
        .iter()
        .all(|x| !matches!(x.kind, Kind::OneWayFallThrough)));

    // A Class-B remap (room load / respawn) did not "fall" the body through.
    assert!(fire(false, true, &mut supp)
        .iter()
        .all(|x| !matches!(x.kind, Kind::OneWayFallThrough)));

    // Still standing on it: no violation.
    let v = check_step(
        "r",
        1,
        1,
        (100.0, 80.0),
        Some((0.0, 0.0)),
        None,
        world,
        &[],
        &[],
        Some((&platform, false, false)),
        &TransitContext::default(),
        &mut supp,
    );
    assert!(v.iter().all(|x| !matches!(x.kind, Kind::OneWayFallThrough)));
}

/// §6.1 invariant 1's carve subtraction, at the level `check_step` sees it: a
/// carved block is simply ABSENT from the composed world, so a body sunk into a
/// portal aperture reports nothing. `solid_blocks` performs the subtraction
/// (`world_with_portal_carves`); this pins that the embed test respects it.
#[test]
fn a_body_inside_a_carved_hole_is_not_embedded() {
    let mut supp = 0;
    let world = (200.0, 200.0);
    // The composed world after a portal carved the middle out of a wall: the
    // wall survives as two pieces, and the body sits in the gap between them.
    let carved = [
        SolidBlock {
            geo: ae::GeoId::anon(),
            aabb: ae::Aabb::new(ae::Vec2::new(100.0, 40.0), ae::Vec2::new(20.0, 40.0)),
        },
        SolidBlock {
            geo: ae::GeoId::anon(),
            aabb: ae::Aabb::new(ae::Vec2::new(100.0, 160.0), ae::Vec2::new(20.0, 40.0)),
        },
    ];
    let in_the_hole = (100.0, 100.0);
    let v = check_step(
        "r",
        1,
        1,
        in_the_hole,
        Some((0.0, 0.0)),
        None,
        world,
        &carved,
        &[],
        None,
        &TransitContext::default(),
        &mut supp,
    );
    assert!(
        v.iter().all(|x| !matches!(x.kind, Kind::EmbeddedInSolid)),
        "a carved hole is not solid — a straddling body's center legitimately \
         sits there, and the exemption falls out of the geometry"
    );

    // ...but the UNCARVED wall would have reported it.
    let solid = [SolidBlock {
        geo: ae::GeoId::anon(),
        aabb: ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(20.0, 100.0)),
    }];
    let v = check_step(
        "r",
        1,
        1,
        in_the_hole,
        Some((0.0, 0.0)),
        None,
        world,
        &solid,
        &[],
        None,
        &TransitContext::default(),
        &mut supp,
    );
    assert!(
        v.iter().any(|x| matches!(x.kind, Kind::EmbeddedInSolid)),
        "the same point inside an UNCARVED wall is invariant 1"
    );
}

/// §6.1 **invariant 2** — a straddling body's center is only allowed inside the
/// host wall where ITS portal carved a hole.
///
/// This is the case invariant 1 structurally cannot see. Invariant 1 tests the
/// COMPOSED world, in which every portal's carve has been subtracted; a body
/// standing in a *different* portal's hole reads as "not in a solid" there. So
/// the two probes disagree by construction, and that disagreement is the §7.6
/// class: the body reached the wall's interior through an opening it never
/// entered.
#[test]
fn a_straddling_body_inside_the_wall_but_outside_its_own_carve_is_invariant_2() {
    let world = (400.0, 400.0);
    let mut supp = 0;
    // One thick wall spanning x∈[100,200]. Two holes punched through it: the one
    // this body straddles (y∈[0,50]) and another, far away (y∈[300,350]).
    let wall = SolidBlock {
        geo: ae::GeoId::anon(),
        aabb: ae::aabb_from_min_size(ae::Vec2::new(100.0, 0.0), ae::Vec2::new(100.0, 400.0)),
    };
    let own_carve = ae::aabb_from_min_size(ae::Vec2::new(100.0, 0.0), ae::Vec2::new(100.0, 50.0));
    let authored = [wall];

    let fire = |pos: (f32, f32), supp: &mut u32| {
        check_step(
            "r",
            1,
            1,
            pos,
            Some((0.0, 0.0)),
            None,
            world,
            // The COMPOSED world: both holes subtracted, so nothing here is solid
            // at either hole and invariant 1 stays silent for both positions.
            &[],
            &[],
            None,
            &TransitContext {
                authored: &authored,
                straddled_carve: Some(own_carve),
                remaps: &[],
            },
            supp,
        )
    };

    // Inside the wall, but in the hole this body straddles — legal, and the whole
    // reason invariant 1 exempts a transiting body.
    assert!(
        fire((150.0, 25.0), &mut supp)
            .iter()
            .all(|x| !matches!(x.kind, Kind::StraddleOutsideCarve)),
        "a body in its OWN aperture is mid-transit, not embedded"
    );

    // Inside the wall, in the OTHER portal's hole. Invariant 1 is blind (the
    // composed world carved it away); invariant 2 is not.
    let v = fire((150.0, 325.0), &mut supp);
    let hit = v
        .iter()
        .find(|x| matches!(x.kind, Kind::StraddleOutsideCarve))
        .expect("straddling + inside authored solid + outside own carve");
    assert_eq!(hit.kind.invariant(), 2);
    assert_eq!(
        hit.geo,
        Some(ae::GeoId::anon()),
        "§6.2: the dump names the wall it is embedded in"
    );

    // Not straddling anything: invariant 2 has nothing to say, even in the wall.
    let v = check_step(
        "r",
        1,
        1,
        (150.0, 325.0),
        Some((0.0, 0.0)),
        None,
        world,
        &[],
        &[],
        None,
        &TransitContext {
            authored: &authored,
            straddled_carve: None,
            remaps: &[],
        },
        &mut supp,
    );
    assert!(v
        .iter()
        .all(|x| !matches!(x.kind, Kind::StraddleOutsideCarve)));
}

/// §6.1 **invariant 5** — at most one Class-B remap per body per frame (§3.2).
///
/// The ledger is a `Vec`, so the probe is trivial; what earns its keep is the
/// DETAIL line, which reads the pair's priority order and names the bug class.
#[test]
fn two_class_b_remaps_in_one_frame_is_invariant_5_and_the_order_names_the_bug() {
    use ambition::platformer::class_b::ClassBRemap;
    let world = (400.0, 400.0);
    let mut supp = 0;

    let fire = |remaps: &[ClassBRemap], supp: &mut u32| {
        check_step(
            "r",
            1,
            1,
            (50.0, 50.0),
            Some((0.0, 0.0)),
            None,
            world,
            &[],
            &[],
            None,
            &TransitContext {
                authored: &[],
                straddled_carve: None,
                remaps,
            },
            supp,
        )
    };

    // One remap per frame is the contract holding.
    assert!(fire(&[ClassBRemap::PortalTransit], &mut supp).is_empty());

    // Portal transit then room transition: the doctrine's own tie-break says the
    // room transition wins, and it DID run last — so the bug is that the portal's
    // remap also applied. §3.1 rule 2's sample reset should have voided it.
    let v = fire(
        &[ClassBRemap::PortalTransit, ClassBRemap::RoomTransition],
        &mut supp,
    );
    let hit = v
        .iter()
        .find(|x| matches!(x.kind, Kind::DoubleClassBRemap))
        .expect("two remaps in one frame");
    assert_eq!(hit.kind.invariant(), 5);
    assert!(
        hit.detail.contains("voided by the sample reset"),
        "detail must name the bug class, got: {}",
        hit.detail
    );

    // Death then portal transit: a corpse cannot warp. The weaker authority ran
    // last and won — a schedule ordering bug, a different fix.
    let v = fire(
        &[ClassBRemap::DeathOrReset, ClassBRemap::PortalTransit],
        &mut supp,
    );
    let hit = v
        .iter()
        .find(|x| matches!(x.kind, Kind::DoubleClassBRemap))
        .expect("two remaps in one frame");
    assert!(
        hit.detail.contains("priority-INVERTED"),
        "detail must name the bug class, got: {}",
        hit.detail
    );
}

/// The TELEPORT probe's Class-B exemption — the fix for the one false positive
/// in CC3's 64,800-frame sweep. §6.1's "Explicitly legal" list names *the
/// transfer frame's position jump*; the ledger is how the oracle finally knows a
/// transfer happened.
#[test]
fn a_class_b_remap_exempts_the_frames_position_jump_from_the_teleport_probe() {
    use ambition::platformer::class_b::ClassBRemap;
    let world = (4000.0, 4000.0);
    let mut supp = 0;
    let from = Some((100.0, 100.0));
    let to = (900.0, 100.0); // an 800px jump: far past TELEPORT_PX

    let fire = |remaps: &[ClassBRemap], supp: &mut u32| {
        check_step(
            "r",
            1,
            1,
            to,
            Some((0.0, 0.0)),
            from,
            world,
            &[],
            &[],
            None,
            &TransitContext {
                authored: &[],
                straddled_carve: None,
                remaps,
            },
            supp,
        )
    };

    let v = fire(&[], &mut supp);
    assert!(
        v.iter().any(|x| matches!(x.kind, Kind::Teleport)),
        "an unexplained 800px jump is still a pop"
    );

    let v = fire(&[ClassBRemap::PortalTransit], &mut supp);
    assert!(
        v.iter().all(|x| !matches!(x.kind, Kind::Teleport)),
        "the transfer frame's position jump is explicitly legal (§6.1)"
    );
}
