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
//! cargo test -p ambition_sandbox --test collision_invariant_oracle \
//!     -- --ignored --nocapture
//! ```
//!
//! Each flagged step prints `(room, seed, tick, pos)` so it reproduces through
//! `cargo run --bin rl_random_walker -- <STEPS> <SEED>` after a `--start-room`.

use ambition_sandbox::engine_core as ae;
use ambition_sandbox::rl_sim::TimestepMode;
use ambition_sandbox::{AgentAction, GameWorld, SandboxSim, SandboxSimOptions};

// --- deterministic policy (shared *shape* with fuzz_random_walker, not code) ---

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }
    fn unit(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32 + 1.0)
    }
    fn signed_unit(&mut self) -> f32 {
        2.0 * self.unit() - 1.0
    }
    fn chance(&mut self, p: f32) -> bool {
        self.unit() < p
    }
}

/// Random play biased toward *traversal stress* — more jumping, flying, blinking,
/// and vertical input than the base fuzzer, because the OOB pops Jon hit all
/// started with "fly up and move around on the ceiling". Pushing the player into
/// ceilings/corners is what makes the embed/teleport bugs reproduce.
fn random_action(rng: &mut Lcg, sticky_x: &mut f32) -> AgentAction {
    if rng.chance(0.08) {
        *sticky_x = if rng.chance(0.6) {
            if rng.chance(0.5) {
                1.0
            } else {
                -1.0
            }
        } else {
            rng.signed_unit()
        };
    }
    let jump = rng.chance(0.10);
    let up = rng.chance(0.20);
    AgentAction {
        move_x: *sticky_x,
        move_y: if up { -1.0 } else { rng.signed_unit() * 0.3 },
        up_pressed: up,
        down_pressed: rng.chance(0.08),
        jump,
        jump_held: jump || rng.chance(0.6),
        jump_released: false,
        dash: rng.chance(0.04),
        attack: rng.chance(0.01),
        blink: rng.chance(0.02),
        blink_held: false,
        blink_released: false,
        pogo: rng.chance(0.02),
        interact: false,
        projectile: false,
        projectile_held: false,
        projectile_released: false,
        // Toggle fly occasionally — being airborne over the ceiling is the
        // precondition for the reported pops.
        fly_toggle: rng.chance(0.03),
        reset: false,
        start: false,
        aim_x: rng.signed_unit(),
        aim_y: rng.signed_unit(),
    }
}

// --- the oracle ---

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    EmbeddedInSolid,
    OutOfBoundsAbove,
    OutOfBoundsBelow,
    OutOfBoundsSide,
    Teleport,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Kind::EmbeddedInSolid => "EMBEDDED-IN-SOLID",
            Kind::OutOfBoundsAbove => "OOB-ABOVE-CEILING",
            Kind::OutOfBoundsBelow => "OOB-BELOW-FLOOR",
            Kind::OutOfBoundsSide => "OOB-SIDE",
            Kind::Teleport => "TELEPORT",
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
}

/// True if `(x, y)` lies inside any Solid block — used to probe whether a Solid
/// boundary wall sits just inside the edge the player went OOB through.
fn point_in_solid(blocks: &[ae::Aabb], x: f32, y: f32) -> bool {
    blocks
        .iter()
        .any(|b| x >= b.min.x && x <= b.max.x && y >= b.min.y && y <= b.max.y)
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

/// The Solid (full-collision) block AABBs of the room the sim is currently in.
fn solid_blocks(sim: &SandboxSim) -> Vec<ae::Aabb> {
    let Some(world) = sim.world().get_resource::<GameWorld>() else {
        return Vec::new();
    };
    world
        .0
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, ae::BlockKind::Solid))
        .map(|b| b.aabb)
        .collect()
}

/// `room id -> its LoadingZone (edge-exit / door) AABBs`, loaded once from the
/// LDtk project. An OOB that lands at one of these is the player legitimately
/// leaving through an opening, not clipping a solid boundary — the cross-check
/// that turns the raw OOB-SIDE noise into "OOB through a wall with no exit".
fn load_loading_zones() -> std::collections::HashMap<String, Vec<ae::Aabb>> {
    use ambition_sandbox as sb;
    let mut map = std::collections::HashMap::new();
    let Ok(project) = sb::ldtk_world::LdtkProject::load_default_for_dev() else {
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

/// Check one post-tick observation against the invariants. `teleport_from` is
/// the prior tick's center for the teleport test — passed as `None` whenever the
/// room changed or the player respawned this tick (a door load or a death→spawn
/// is a *legitimate* large jump, not a pop), so teleport only fires on a genuine
/// same-room in-place warp. Embed/OOB always run on the current pos.
fn check_step(
    room: &str,
    seed: u64,
    tick: u64,
    pos: (f32, f32),
    teleport_from: Option<(f32, f32)>,
    world_size: (f32, f32),
    blocks: &[ae::Aabb],
    loading_zones: &[ae::Aabb],
    suppressed: &mut u32,
) -> Vec<Violation> {
    let mut out = Vec::new();
    let (px, py) = pos;
    if !px.is_finite() || !py.is_finite() {
        return out; // the base fuzzer owns the NaN-explosion assertion
    }

    // 1. Embedded in a Solid: center strictly inside a block by > EMBED_MARGIN.
    for b in blocks {
        if px > b.min.x + EMBED_MARGIN
            && px < b.max.x - EMBED_MARGIN
            && py > b.min.y + EMBED_MARGIN
            && py < b.max.y - EMBED_MARGIN
        {
            out.push(Violation {
                room: room.to_string(),
                seed,
                tick,
                kind: Kind::EmbeddedInSolid,
                pos,
                detail: format!(
                    "center inside solid [{:.0},{:.0}]..[{:.0},{:.0}]",
                    b.min.x, b.min.y, b.max.x, b.max.y
                ),
                through_wall: None,
            });
            break; // one embed report per step is enough
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
            });
        }
    }

    // 3. Teleport: a single-tick jump no legit move can produce (caller passes
    // None across room loads / respawns so those legit jumps don't count).
    if let Some((qx, qy)) = teleport_from {
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
    let mut rng = Lcg::new(seed);
    let mut sticky = 0.0_f32;
    let first = sim.observation();
    let mut prev_pos = first.player_pos;
    let mut prev_room = first.active_room.clone();
    let mut prev_resets = first.resets;
    let mut violations = Vec::new();
    let mut ran = 0;
    let mut suppressed = 0;
    let empty: Vec<ae::Aabb> = Vec::new();
    for _ in 0..steps {
        let action = random_action(&mut rng, &mut sticky);
        let obs = sim.step(action);
        ran += 1;
        let blocks = solid_blocks(&sim);
        let room_zones = zones.get(&obs.active_room).unwrap_or(&empty);
        // A door load or a death→spawn respawn is a legit large jump — only feed
        // the teleport test a prior pos when neither happened this tick.
        let transitioned = obs.active_room != prev_room || obs.resets != prev_resets;
        let teleport_from = (!transitioned).then_some(prev_pos);
        violations.extend(check_step(
            &obs.active_room,
            seed,
            obs.tick,
            obs.player_pos,
            teleport_from,
            obs.world_size,
            &blocks,
            room_zones,
            &mut suppressed,
        ));
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
        s.push_str(&format!(
            "  {room:28} {kind:28} x{count:<4} first: seed={} tick={} pos=({:.0},{:.0}) {}\n",
            first.seed, first.tick, first.pos.0, first.pos.1, first.detail
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
    let walled = [ae::Aabb::new(
        ae::Vec2::new(96.0, 50.0),
        ae::Vec2::new(4.0, 50.0),
    )];
    let v = check_step(
        "r",
        1,
        1,
        oob_right,
        Some((90.0, 50.0)),
        world,
        &walled,
        &[],
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
        Some((90.0, 50.0)),
        world,
        &[],
        &[],
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
        Some((120.0, 50.0)),
        world,
        &walled,
        &[],
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
    let mut rng = Lcg::new(1);
    let mut sticky = 0.0_f32;
    let mut prev = sim.observation().player_pos;
    for tick in 1..=110u64 {
        let action = random_action(&mut rng, &mut sticky);
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
