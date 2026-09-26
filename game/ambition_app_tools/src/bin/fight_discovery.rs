//! Fight discovery: run a boss fight headlessly and say what happens in it, in
//! numbers.
//!
//! A fight is MEASURED before it is polished. This drives the player against a
//! boss in its real arena under a few policies (stand still, random input, a
//! scripted aggressor), records every tick through the one combat serializer
//! ([`CombatObservation`]), and writes:
//!
//! * a CHOREOGRAPHY — the boss's beats in order: the move, its telegraph, its
//!   strike, and the rest that follows. Read off [`BossAttackState`], the boss's
//!   published live move, so it describes any pattern- or conductor-driven boss
//!   that publishes one, not only GNU-ton;
//! * a `FightReport`-shaped summary (`docs/planning/engine/boss-design.md` §4):
//!   outcome, time to kill, hits taken and which move dealt them, threat
//!   density per phase, punish windows against punishes landed, and dead time;
//! * FINDINGS — the holes those numbers point at, each naming the rule it
//!   applies. They are prompts for a designer, not a verdict: the bands are
//!   first guesses until a fight Jon rates calibrates them.
//!
//! ⛔ Nothing here resolves a volume. Threat and punishability come from the
//! observation rows, which come from `CombatGeometryView`; health and the live
//! move are read off the components the game itself reads.
//!
//! ```text
//! cargo run -p ambition_app_tools --release --bin fight_discovery -- \
//!     --room gnu_ton_arena [--policy all|sandbag|random|aggressor] \
//!     [--seconds 180] [--seeds 2] [--out target/fight_discovery]
//! ```

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use ambition_app::rl_sim::{
    AgentAction, AgentObservation, AmbitionSim, Platformer2dSimHarness,
    Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::boss_encounter::{BossConfig, BossEncounter};
use ambition_platformer2d::characters::actor::{BodyHealth, LimbRig};
use ambition_platformer2d::mount::RidingOn;
use ambition_platformer2d::characters::brain::{BossAttackProfile, BossAttackState};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_sim_harness::combat_observation::{sim_id_of, CombatObservation, ScenarioRoles};
use bevy::prelude::{Entity, World};

const DT: f32 = 1.0 / 60.0;

// ── Options ──────────────────────────────────────────────────────────────────

struct Options {
    room: String,
    boss: Option<String>,
    policies: Vec<Policy>,
    seconds: f32,
    seeds: u64,
    out: PathBuf,
}

fn parse_options() -> Options {
    let mut args = std::env::args().skip(1);
    let mut room = None;
    let mut boss = None;
    let mut policy = "all".to_string();
    let mut seconds = 180.0;
    let mut seeds = 2;
    let mut out = None;
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| usage(&format!("{arg} needs a value")));
        match arg.as_str() {
            "--room" => room = Some(value()),
            "--boss" => boss = Some(value()),
            "--policy" => policy = value(),
            "--seconds" => seconds = value().parse().unwrap_or_else(|_| usage("--seconds is a number")),
            "--seeds" => seeds = value().parse().unwrap_or_else(|_| usage("--seeds is a count")),
            "--out" => out = Some(PathBuf::from(value())),
            "-h" | "--help" => usage(""),
            other => usage(&format!("unknown argument {other}")),
        }
    }
    let room = room.unwrap_or_else(|| usage("--room is required"));
    let policies = match policy.as_str() {
        "all" => vec![Policy::Sandbag, Policy::Random, Policy::Aggressor],
        word => vec![Policy::parse(word).unwrap_or_else(|| usage(&format!("unknown policy {word}")))],
    };
    let out = out.unwrap_or_else(|| PathBuf::from("target/fight_discovery").join(&room));
    Options { room, boss, policies, seconds, seeds, out }
}

fn usage(problem: &str) -> ! {
    if !problem.is_empty() {
        eprintln!("fight_discovery: {problem}");
    }
    eprintln!(
        "usage: fight_discovery --room <arena> [--boss <id>] [--policy all|sandbag|random|aggressor] \
         [--seconds N] [--seeds N] [--out DIR]"
    );
    std::process::exit(2);
}

// ── Player policies ──────────────────────────────────────────────────────────

/// Who holds the controller. Floors and a crude opponent, not a player model:
/// the fighter brain driving the player seat is the next rung (BD6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    /// No input: what the boss does to someone who does nothing.
    Sandbag,
    /// Seeded random stick, jump and attack.
    Random,
    /// Walks at the nearest thing it can hurt, jumps at it, swings and pogos.
    Aggressor,
}

impl Policy {
    fn parse(word: &str) -> Option<Self> {
        match word {
            "sandbag" => Some(Self::Sandbag),
            "random" => Some(Self::Random),
            "aggressor" => Some(Self::Aggressor),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Sandbag => "sandbag",
            Self::Random => "random",
            Self::Aggressor => "aggressor",
        }
    }
}

struct Driver {
    policy: Policy,
    rng: u64,
    tick: u64,
    held_x: f32,
    jump_hold: u32,
}

impl Driver {
    fn new(policy: Policy, seed: u64) -> Self {
        Self { policy, rng: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1, tick: 0, held_x: 0.0, jump_hold: 0 }
    }

    fn next(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f32 / (1u64 << 53) as f32
    }

    fn act(&mut self, obs: &AgentObservation, aim: Option<ae::Vec2>) -> AgentAction {
        self.tick += 1;
        let mut action = AgentAction::default();
        match self.policy {
            Policy::Sandbag => {}
            Policy::Random => {
                if self.tick % 12 == 1 {
                    self.held_x = [-1.0, 0.0, 1.0][(self.next() * 3.0) as usize % 3];
                    if self.next() < 0.25 {
                        self.jump_hold = 18;
                        action.jump = true;
                    }
                    if self.next() < 0.35 {
                        action.attack = true;
                        action.down_pressed = self.next() < 0.2;
                    }
                }
                steer(&mut action, self.held_x);
            }
            Policy::Aggressor => {
                let player = ae::Vec2::new(obs.player_pos.0, obs.player_pos.1);
                if let Some(aim) = aim {
                    let d = aim - player;
                    steer(&mut action, if d.x.abs() > 48.0 { d.x.signum() } else { 0.0 });
                    let close = d.x.abs() < 90.0 && d.y.abs() < 110.0;
                    if close && self.tick % 10 == 1 {
                        action.attack = true;
                    }
                    if d.y < -80.0 && obs.on_ground && self.jump_hold == 0 {
                        self.jump_hold = 22;
                        action.jump = true;
                    }
                    // Above it and falling: pogo.
                    if !obs.on_ground && d.y > 30.0 && d.x.abs() < 60.0 && self.tick % 8 == 1 {
                        action.down_pressed = true;
                        action.move_y = 1.0;
                        action.attack = true;
                    }
                }
            }
        }
        if self.jump_hold > 0 {
            self.jump_hold -= 1;
            action.jump_held = true;
        }
        action.attack_held = action.attack;
        action
    }
}

fn steer(action: &mut AgentAction, x: f32) {
    action.move_x = x;
    action.left_pressed = x < 0.0;
    action.right_pressed = x > 0.0;
}

// ── One tick of the record ───────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Telegraph,
    Strike,
}

#[derive(Clone, Debug)]
struct Tick {
    t: f32,
    phase: String,
    beat: Option<(String, Stage)>,
    boss_hp: i32,
    boss_alive: bool,
    player_hp: i32,
    player_resets: u32,
    /// Bounding area (px²) of every harmful volume the boss side owns now.
    threat_area: f32,
    /// A harmful boss-side volume overlaps the player now (by the engine's own
    /// intersection, `overlaps`).
    threat_on_player: bool,
    /// A boss-side body offers a hurtbox now.
    punishable: bool,
    /// Which boss-side bodies offer a hurtbox now (their sim ids).
    hittable: Vec<String>,
    /// Where the player should swing: the nearest boss-side hurtbox, else the boss.
    aim: Option<ae::Vec2>,
}

fn profile_name(profile: &BossAttackProfile) -> String {
    match profile {
        BossAttackProfile::Strike(key) | BossAttackProfile::Special(key) => key.clone(),
    }
}

fn find_boss(world: &mut World, wanted: Option<&str>) -> Option<Entity> {
    let mut query = world.query::<(Entity, &BossConfig)>();
    query
        .iter(world)
        .find(|(_, config)| wanted.is_none_or(|id| config.behavior.id == id))
        .map(|(entity, _)| entity)
}

/// The boss's side of the fight: the boss, what it rides, and the limbs of
/// either (GNU-ton's fists are the gnu's limbs and own their own strikes).
///
/// ⛔ The observation's `subject_owned` role covers what the BOSS owns; a limb
/// is its own body and owns its own hitbox, so without this its strikes read as
/// scenery and a fight that hits you with its fists reads as threat-free.
/// Each entry is (sim id, the role a reader is told: `boss`, `mount`,
/// `limb:<slot>`).
fn boss_side(world: &World, boss: Entity) -> Vec<(String, String)> {
    let mount = world.get::<RidingOn>(boss).map(|riding| riding.mount);
    let mut bodies = vec![(boss, "boss".to_string())];
    bodies.extend(mount.map(|m| (m, "mount".to_string())));
    for host in [Some(boss), mount].into_iter().flatten() {
        if let Some(rig) = world.get::<LimbRig>(host) {
            bodies.extend(rig.limbs.iter().map(|(slot, limb)| (*limb, format!("limb:{slot}"))));
        }
    }
    bodies
        .into_iter()
        .filter_map(|(entity, name)| sim_id_of(world, entity).map(|id| (id, name)))
        .collect()
}

fn find_player(world: &mut World) -> Option<Entity> {
    let mut query = world.query_filtered::<Entity, PrimaryPlayerOnly>();
    query.iter(world).next()
}

fn observe(world: &mut World, boss: Entity, player: Entity, obs: &AgentObservation, t: f32) -> Tick {
    let beat = world.get::<BossAttackState>(boss).and_then(|state| {
        state
            .active_profile
            .as_ref()
            .map(|p| (profile_name(p), Stage::Strike))
            .or_else(|| state.telegraph_profile.as_ref().map(|p| (profile_name(p), Stage::Telegraph)))
    });
    let phase = world
        .get::<BossEncounter>(boss)
        .map_or_else(|| "?".to_string(), |encounter| format!("{:?}", encounter.encounter_phase()));
    let (boss_hp, boss_alive) = world
        .get::<BodyHealth>(boss)
        .map_or((0, false), |health| (health.current(), health.alive()));
    let boss_pos = world.get::<ae::BodyKinematics>(boss).map(|kin| kin.pos);
    let player_id = sim_id_of(world, player);
    let side = boss_side(world, boss);
    let roles = ScenarioRoles::of(Some(boss), Some(player)).resolve(world);
    let rows = CombatObservation::capture(world, &roles).to_json();

    let on_boss_side = |row: &serde_json::Value, id_key: &str| {
        matches!(row["role"].as_str(), Some("subject" | "subject_owned"))
            || row[id_key].as_str().is_some_and(|id| side.iter().any(|(s, _)| s == id))
    };
    let mut threat_area = 0.0;
    let mut threat_on_player = false;
    for strike in rows["strikes"].as_array().into_iter().flatten() {
        if !on_boss_side(strike, "owner_id") || strike["damage"].as_i64().unwrap_or(0) <= 0 {
            continue;
        }
        let half = &strike["half"];
        threat_area += 4.0 * half[0].as_f64().unwrap_or(0.0) as f32 * half[1].as_f64().unwrap_or(0.0) as f32;
        let overlaps = strike["overlaps"].as_array().into_iter().flatten();
        threat_on_player |= overlaps.filter_map(|v| v.as_str()).any(|id| Some(id) == player_id.as_deref());
    }
    let player_pos = ae::Vec2::new(obs.player_pos.0, obs.player_pos.1);
    let mut punishable = false;
    let mut hittable = Vec::new();
    let mut aim: Option<ae::Vec2> = None;
    // Body rows are flat: `hurtboxes` and `hurtbox_source` sit beside `role`.
    for body in rows["bodies"].as_array().into_iter().flatten() {
        if !on_boss_side(body, "id") || body["hurtbox_source"].as_str() == Some("intangible") {
            continue;
        }
        for hurtbox in body["hurtboxes"].as_array().into_iter().flatten() {
            let Some(pos) = hurtbox["pos"].as_array() else { continue };
            punishable = true;
            if let Some(id) = body["id"].as_str() {
                let name = side.iter().find(|(s, _)| s == id).map_or(id, |(_, name)| name.as_str());
                if !hittable.iter().any(|h| h == name) {
                    hittable.push(name.to_string());
                }
            }
            let at = ae::Vec2::new(pos[0].as_f64().unwrap_or(0.0) as f32, pos[1].as_f64().unwrap_or(0.0) as f32);
            if aim.is_none_or(|best| at.distance(player_pos) < best.distance(player_pos)) {
                aim = Some(at);
            }
        }
    }
    Tick {
        t,
        phase,
        beat,
        boss_hp,
        boss_alive,
        player_hp: obs.hp,
        player_resets: obs.resets,
        threat_area,
        threat_on_player,
        punishable,
        hittable,
        aim: aim.or(boss_pos),
    }
}

// ── The run ──────────────────────────────────────────────────────────────────

struct Run {
    policy: Policy,
    seed: u64,
    arena_area: f32,
    ticks: Vec<Tick>,
    outcome: &'static str,
}

fn run(options: &Options, policy: Policy, seed: u64) -> Run {
    let harness = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(options.room.clone());
    let mut sim = Platformer2dSimHarness::new_with_options(harness)
        .unwrap_or_else(|error| panic!("the arena `{}` builds headlessly: {error}", options.room));
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let world = sim.world_mut();
    let boss = find_boss(world, options.boss.as_deref())
        .unwrap_or_else(|| panic!("no boss{} in `{}`", options.boss.as_ref().map_or(String::new(), |b| format!(" `{b}`")), options.room));
    let player = find_player(world).expect("the arena seats a player");
    let first = sim.observation();
    let arena_area = first.world_size.0 * first.world_size.1;
    let mut driver = Driver::new(policy, seed);
    let mut ticks = Vec::new();
    let mut aim = None;
    let mut outcome = "timeout";
    let frames = (options.seconds / DT) as usize;
    for frame in 0..frames {
        let action = driver.act(&sim.observation(), aim);
        let obs = sim.step(action);
        if obs.active_room != options.room {
            outcome = "left_the_arena";
            break;
        }
        let tick = observe(sim.world_mut(), boss, player, &obs, frame as f32 * DT);
        aim = tick.aim;
        let (boss_down, player_down) = (!tick.boss_alive, tick.player_resets > first.resets || tick.player_hp <= 0);
        ticks.push(tick);
        if boss_down {
            outcome = "boss_defeated";
            break;
        }
        if player_down {
            outcome = "player_defeated";
            break;
        }
    }
    Run { policy, seed, arena_area, ticks, outcome }
}

// ── What the record says ─────────────────────────────────────────────────────

/// One occurrence of a move: its telegraph, its strike, and the quiet after it.
#[derive(Clone, Debug)]
struct Beat {
    profile: String,
    phase: String,
    start: f32,
    telegraph_s: f32,
    strike_s: f32,
    rest_after_s: f32,
    /// Something other than the boss's own next move ended it: the record
    /// stopped (a kill, the time cap) or the encounter changed phase inside it,
    /// which restarts the script. Its strike or rest is a cut-off, not a length
    /// the pattern chose, so the per-move medians leave it out.
    cut: bool,
}

/// Cut the record into beats. A beat starts when the live move changes or when
/// a strike gives way to a telegraph (the same move again is a new beat).
fn beats(ticks: &[Tick]) -> Vec<Beat> {
    let mut out: Vec<Beat> = Vec::new();
    let mut last: Option<(String, Stage)> = None;
    for tick in ticks {
        match (&tick.beat, &last) {
            (Some((profile, stage)), previous) => {
                let fresh = match previous {
                    None => true,
                    Some((was, was_stage)) => {
                        was != profile || (*was_stage == Stage::Strike && *stage == Stage::Telegraph)
                    }
                };
                if fresh {
                    // The phase changing as the next move starts ends the one
                    // before it too: the new phase's script started the move.
                    if let Some(before) = out.last_mut() {
                        before.cut |= before.phase != tick.phase;
                    }
                    out.push(Beat {
                        profile: profile.clone(),
                        phase: tick.phase.clone(),
                        start: tick.t,
                        telegraph_s: 0.0,
                        strike_s: 0.0,
                        rest_after_s: 0.0,
                        cut: false,
                    });
                }
                let beat = out.last_mut().expect("a beat was just started");
                beat.cut |= beat.phase != tick.phase;
                match stage {
                    Stage::Telegraph => beat.telegraph_s += DT,
                    Stage::Strike => beat.strike_s += DT,
                }
            }
            (None, _) => {
                if let Some(beat) = out.last_mut() {
                    beat.rest_after_s += DT;
                    beat.cut |= beat.phase != tick.phase;
                }
            }
        }
        last = tick.beat.clone();
    }
    // The record ended inside the last beat: whatever it measured was cut off.
    if let Some(last) = out.last_mut() {
        last.cut = true;
    }
    out
}

/// Maximal runs of ticks where `pred` holds, as (start index, end index exclusive).
fn runs(ticks: &[Tick], pred: impl Fn(&Tick) -> bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, tick) in ticks.iter().enumerate() {
        match (pred(tick), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, ticks.len()));
    }
    out
}

fn median(mut values: Vec<f32>) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f32::total_cmp);
    values[values.len() / 2]
}

struct Summary {
    json: serde_json::Value,
    beats: Vec<Beat>,
}

fn summarize(run: &Run) -> Summary {
    let ticks = &run.ticks;
    let beats = beats(ticks);
    let duration = ticks.len() as f32 * DT;
    // Hits taken and dealt, as health drops between ticks; a hit taken is
    // charged to the move live on that tick.
    let mut hits_taken = 0;
    let mut damage_taken = 0;
    let mut damage_sources: BTreeMap<String, i32> = BTreeMap::new();
    let mut hits_dealt = 0;
    let mut damage_dealt = 0;
    // Where the player's damage landed: the move live when the boss lost
    // health, or the rest after the last one, and the phase it fell in. A phase
    // that loses its health far faster than the others is one the player skips.
    let mut damage_windows: BTreeMap<String, i32> = BTreeMap::new();
    let mut hit_sizes: Vec<i32> = Vec::new();
    let mut hp_lost_by_phase: BTreeMap<String, i32> = BTreeMap::new();
    let mut last_move: Option<&str> = None;
    for pair in ticks.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if let Some((profile, _)) = &a.beat {
            last_move = Some(profile.as_str());
        }
        if b.player_hp < a.player_hp {
            hits_taken += 1;
            damage_taken += a.player_hp - b.player_hp;
            let source = b.beat.as_ref().or(a.beat.as_ref()).map_or("between_moves", |(p, _)| p.as_str());
            *damage_sources.entry(source.to_string()).or_default() += a.player_hp - b.player_hp;
        }
        if b.boss_hp < a.boss_hp {
            hits_dealt += 1;
            damage_dealt += a.boss_hp - b.boss_hp;
            hit_sizes.push(a.boss_hp - b.boss_hp);
            let window = match b.beat.as_ref().or(a.beat.as_ref()) {
                Some((profile, _)) => profile.clone(),
                None => format!("after {}", last_move.unwrap_or("nothing")),
            };
            *damage_windows.entry(window).or_default() += a.boss_hp - b.boss_hp;
            *hp_lost_by_phase.entry(b.phase.clone()).or_default() += a.boss_hp - b.boss_hp;
        }
    }
    // Threat density and time, per phase.
    let mut per_phase: BTreeMap<String, (f32, f32, f32)> = BTreeMap::new();
    for tick in ticks {
        let entry = per_phase.entry(tick.phase.clone()).or_default();
        entry.0 += DT;
        entry.1 += tick.threat_area / run.arena_area.max(1.0);
        entry.2 += if tick.threat_area > 0.0 { 1.0 } else { 0.0 };
    }
    let phases: serde_json::Map<String, serde_json::Value> = per_phase
        .iter()
        .map(|(phase, (seconds, density, threatened))| {
            let n = (seconds / DT).max(1.0);
            (
                phase.clone(),
                serde_json::json!({
                    "seconds": seconds,
                    "threat_density": density / n,
                    "threatened_fraction": threatened / n,
                    "boss_hp_lost": hp_lost_by_phase.get(phase).copied().unwrap_or(0),
                }),
            )
        })
        .collect();
    // Punish windows: runs where the boss side offers a hurtbox. Converted when
    // the boss lost health inside one.
    let windows = runs(ticks, |t| t.punishable);
    let converted = windows
        .iter()
        .filter(|(s, e)| ticks[*s].boss_hp > ticks[e.saturating_sub(1).max(*s)].boss_hp)
        .count();
    let punish_seconds: f32 = windows.iter().map(|(s, e)| (e - s) as f32 * DT).sum();
    // Dead time: no move live and nothing harmful out.
    let dead = ticks.iter().filter(|t| t.beat.is_none() && t.threat_area <= 0.0).count() as f32 * DT;
    // Exposure: seconds a harmful boss-side volume overlapped the player. Far
    // more exposure than hits means the i-frames or hit rules are absorbing it.
    let exposed = ticks.iter().filter(|t| t.threat_on_player).count() as f32 * DT;
    // How much of the fight each boss-side body can be hit. A body hittable
    // all fight long is not a punish window; it is no defence.
    let mut hittable: BTreeMap<String, f32> = BTreeMap::new();
    for tick in ticks {
        for id in &tick.hittable {
            *hittable.entry(id.clone()).or_default() += DT / duration.max(DT);
        }
    }
    let json = serde_json::json!({
        "policy": run.policy.as_str(),
        "seed": run.seed,
        "outcome": run.outcome,
        "duration_s": duration,
        "time_to_kill_s": (run.outcome == "boss_defeated").then_some(duration),
        "hits_taken": hits_taken,
        "damage_taken": damage_taken,
        "damage_sources": damage_sources,
        "hits_dealt": hits_dealt,
        "damage_dealt": damage_dealt,
        "damage_windows": damage_windows,
        "hit_sizes": hit_sizes,
        "boss_hp": [ticks.first().map_or(0, |t| t.boss_hp), ticks.last().map_or(0, |t| t.boss_hp)],
        "player_hp": [ticks.first().map_or(0, |t| t.player_hp), ticks.iter().map(|t| t.player_hp).min().unwrap_or(0), ticks.last().map_or(0, |t| t.player_hp)],
        "phases": phases,
        "punish_windows": windows.len(),
        "punish_windows_converted": converted,
        "punish_seconds": punish_seconds,
        "dead_time_s": dead,
        "exposed_s": exposed,
        "hittable_fraction": hittable,
        "beats": beats.len(),
    });
    Summary { json, beats }
}

// ── Findings ─────────────────────────────────────────────────────────────────

/// One move's shape across every run.
struct MoveShape {
    /// Occurrences, cut ones included.
    beats: usize,
    /// Of those, the ones something else ended (see [`Beat::cut`]).
    cut: usize,
    /// Medians over the WHOLE beats only; `None` when every beat was cut.
    telegraph: Option<f32>,
    strike: Option<f32>,
    rest: Option<f32>,
    phases: Vec<String>,
}

/// Per-move shape across every run.
fn move_table(all: &[(Run, Summary)]) -> BTreeMap<String, MoveShape> {
    let mut grouped: BTreeMap<String, (Vec<&Beat>, Vec<String>)> = BTreeMap::new();
    for (_, summary) in all {
        for beat in &summary.beats {
            let entry = grouped.entry(beat.profile.clone()).or_default();
            entry.0.push(beat);
            if !entry.1.contains(&beat.phase) {
                entry.1.push(beat.phase.clone());
            }
        }
    }
    grouped
        .into_iter()
        .map(|(profile, (beats, phases))| {
            let whole: Vec<&&Beat> = beats.iter().filter(|b| !b.cut).collect();
            let of = |f: fn(&Beat) -> f32| {
                (!whole.is_empty()).then(|| median(whole.iter().map(|b| f(b)).collect()))
            };
            let shape = MoveShape {
                beats: beats.len(),
                cut: beats.len() - whole.len(),
                telegraph: of(|b| b.telegraph_s),
                strike: of(|b| b.strike_s),
                rest: of(|b| b.rest_after_s),
                phases,
            };
            (profile, shape)
        })
        .collect()
}

/// The holes the numbers point at. Each names its rule; thresholds are first
/// guesses (boss-design.md §3/§4) until a rated fight calibrates them.
fn findings(all: &[(Run, Summary)]) -> Vec<String> {
    let mut out = Vec::new();
    let push = |out: &mut Vec<String>, line: String| {
        if !out.contains(&line) {
            out.push(line);
        }
    };
    for (profile, shape) in move_table(all) {
        // Only whole beats say what the pattern chose.
        let (Some(tel), Some(strike), Some(rest)) = (shape.telegraph, shape.strike, shape.rest) else {
            continue;
        };
        let n = shape.beats - shape.cut;
        if tel > 0.0 && tel < 0.35 {
            push(&mut out, format!("`{profile}`: telegraph {tel:.2}s (x{n}) is under 0.35s — hard to read before it lands (telegraph grammar)."));
        }
        if tel == 0.0 {
            push(&mut out, format!("`{profile}`: no telegraph at all (x{n}) — nothing to read (telegraph grammar)."));
        }
        if rest < 0.2 && strike > 0.0 {
            push(&mut out, format!("`{profile}`: {rest:.2}s of rest after its strike (x{n}) — no punish window follows it (commitment rule)."));
        }
        if rest > 2.5 {
            push(&mut out, format!("`{profile}`: {rest:.2}s of quiet after it (x{n}) — dead air (pacing)."));
        }
    }
    for (run, summary) in all {
        let j = &summary.json;
        // Per policy, not per seed: the same finding from two seeds is one finding.
        let label = run.policy.as_str().to_string();
        let duration = j["duration_s"].as_f64().unwrap_or(0.0) as f32;
        let dead = j["dead_time_s"].as_f64().unwrap_or(0.0) as f32;
        if duration > 0.0 && dead / duration > 0.3 {
            push(&mut out, format!("{label}: {:.0}% of the fight is dead time (no move, no threat).", 100.0 * dead / duration));
        }
        let windows = j["punish_windows"].as_u64().unwrap_or(0);
        let converted = j["punish_windows_converted"].as_u64().unwrap_or(0);
        if run.policy == Policy::Aggressor && windows > 0 && converted * 4 < windows {
            push(&mut out, format!("{label}: converted {converted} of {windows} punish windows — the windows may not be reachable or long enough."));
        }
        let taken = j["damage_taken"].as_i64().unwrap_or(0).max(1) as f32;
        for (source, damage) in j["damage_sources"].as_object().into_iter().flatten() {
            let share = damage.as_i64().unwrap_or(0) as f32 / taken;
            if share > 0.5 && taken >= 3.0 {
                push(&mut out, format!("{label}: `{source}` dealt {:.0}% of all damage taken (damage-source diversity).", share * 100.0));
            }
        }
        for (body, fraction) in j["hittable_fraction"].as_object().into_iter().flatten() {
            if fraction.as_f64().unwrap_or(0.0) > 0.9 {
                push(&mut out, format!("{label}: `{body}` can be hit {:.0}% of the fight — no defence, so every window is a punish window (commitment rule).", 100.0 * fraction.as_f64().unwrap_or(0.0)));
            }
        }
        if run.policy == Policy::Aggressor && run.outcome == "boss_defeated" && duration < 30.0 {
            push(&mut out, format!("{label}: a scripted swinger kills it in {duration:.1}s — faster than the choreography can show itself (time-to-kill band)."));
        }
        if run.policy == Policy::Sandbag && run.outcome == "timeout" {
            let start = j["player_hp"][0].as_f64().unwrap_or(0.0) as f32;
            let rate = j["damage_taken"].as_f64().unwrap_or(0.0) as f32 / duration.max(DT);
            if rate > 0.0 && start / rate > 120.0 {
                push(&mut out, format!("{label}: it deals {:.1} HP/min to a player who does nothing; killing their {start:.0} HP would take {:.0}s (threat rule).", rate * 60.0, start / rate));
            }
        }
        if run.policy == Policy::Sandbag && run.outcome == "timeout" && j["hits_taken"].as_i64() == Some(0) {
            push(&mut out, format!("{label}: a player who does nothing is never hit — the fight does not come to them."));
        }
        let phases = j["phases"].as_object().cloned().unwrap_or_default();
        let densities: Vec<(String, f64)> = phases
            .iter()
            .map(|(p, v)| (p.clone(), v["threat_density"].as_f64().unwrap_or(0.0)))
            .collect();
        for pair in densities.windows(2) {
            if pair[1].1 + 1e-6 < pair[0].1 && pair[0].0.starts_with("Phase") && pair[1].0.starts_with("Phase") {
                push(&mut out, format!("{label}: threat density falls from {} ({:.3}) to {} ({:.3}) — the fight should escalate.", pair[0].0, pair[0].1, pair[1].0, pair[1].1));
            }
        }
    }
    out
}

// ── Report ───────────────────────────────────────────────────────────────────

fn report(options: &Options, all: &[(Run, Summary)]) -> String {
    let mut md = String::new();
    let _ = writeln!(md, "# Fight discovery: `{}`\n", options.room);
    let _ = writeln!(md, "Generated by `fight_discovery` ({} s cap, {} seed(s) per policy). Numbers are measured; findings are heuristics with named rules.\n", options.seconds, options.seeds);
    let _ = writeln!(md, "## Runs\n");
    let _ = writeln!(md, "| policy | seed | outcome | duration | hits taken | damage dealt | boss hp | punish windows (converted) | dead time |");
    let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|");
    for (run, summary) in all {
        let j = &summary.json;
        let _ = writeln!(
            md,
            "| {} | {} | {} | {:.1}s | {} | {} | {} → {} | {} ({}) | {:.1}s |",
            run.policy.as_str(),
            run.seed,
            run.outcome,
            j["duration_s"].as_f64().unwrap_or(0.0),
            j["hits_taken"],
            j["damage_dealt"],
            j["boss_hp"][0],
            j["boss_hp"][1],
            j["punish_windows"],
            j["punish_windows_converted"],
            j["dead_time_s"].as_f64().unwrap_or(0.0),
        );
    }
    let _ = writeln!(md, "\n## Moves (all runs)\n");
    let _ = writeln!(
        md,
        "Medians are over WHOLE beats: one the record's end or a phase change cut off (`cut`) measured the interruption, not a length the pattern chose.\n"
    );
    let _ = writeln!(md, "| move | beats | telegraph (median) | strike (median) | rest after (median) | phases |");
    let _ = writeln!(md, "|---|---|---|---|---|---|");
    for (profile, shape) in move_table(all) {
        let secs = |v: Option<f32>| v.map_or("—".to_string(), |v| format!("{v:.2}s"));
        let beats = match shape.cut {
            0 => shape.beats.to_string(),
            cut => format!("{} ({cut} cut)", shape.beats),
        };
        let _ = writeln!(
            md,
            "| `{profile}` | {beats} | {} | {} | {} | {} |",
            secs(shape.telegraph),
            secs(shape.strike),
            secs(shape.rest),
            shape.phases.join(", ")
        );
    }
    let _ = writeln!(md, "\n## Threat by phase\n");
    let _ = writeln!(md, "| policy | seed | phase | seconds | threat density | threatened | boss hp lost |");
    let _ = writeln!(md, "|---|---|---|---|---|---|---|");
    for (run, summary) in all {
        for (phase, v) in summary.json["phases"].as_object().into_iter().flatten() {
            let _ = writeln!(
                md,
                "| {} | {} | {phase} | {:.1} | {:.4} | {:.0}% | {} |",
                run.policy.as_str(),
                run.seed,
                v["seconds"].as_f64().unwrap_or(0.0),
                v["threat_density"].as_f64().unwrap_or(0.0),
                100.0 * v["threatened_fraction"].as_f64().unwrap_or(0.0),
                v["boss_hp_lost"].as_i64().unwrap_or(0),
            );
        }
    }
    let _ = writeln!(md, "\n## How much of the fight each boss-side body can be hit\n");
    for (run, summary) in all {
        let list: Vec<String> = summary.json["hittable_fraction"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(k, v)| format!("`{k}` {:.0}%", 100.0 * v.as_f64().unwrap_or(0.0)))
            .collect();
        let _ = writeln!(md, "- {} seed {}: {}", run.policy.as_str(), run.seed, if list.is_empty() { "none".to_string() } else { list.join(", ") });
    }
    let _ = writeln!(md, "\n## Where the damage came from\n");
    for (run, summary) in all {
        let sources = summary.json["damage_sources"].as_object().cloned().unwrap_or_default();
        if !sources.is_empty() {
            let list: Vec<String> = sources.iter().map(|(k, v)| format!("`{k}` {v}")).collect();
            let _ = writeln!(md, "- {} seed {}: {}", run.policy.as_str(), run.seed, list.join(", "));
        }
    }
    let _ = writeln!(md, "\n## Where the player's damage landed\n");
    let _ = writeln!(md, "The move live when the boss lost health, or the rest after one.\n");
    for (run, summary) in all {
        let windows = summary.json["damage_windows"].as_object().cloned().unwrap_or_default();
        if !windows.is_empty() {
            let list: Vec<String> = windows.iter().map(|(k, v)| format!("`{k}` {v}")).collect();
            let _ = writeln!(md, "- {} seed {}: {}", run.policy.as_str(), run.seed, list.join(", "));
        }
    }
    // The choreography: the longest run shows the most of the boss's sequence.
    if let Some((run, summary)) = all.iter().max_by_key(|(r, _)| r.ticks.len()) {
        let _ = writeln!(md, "\n## Choreography ({} seed {}, {} beats)\n", run.policy.as_str(), run.seed, summary.beats.len());
        let _ = writeln!(md, "```text");
        let _ = writeln!(md, "   t(s)  phase        move                  telegraph  strike  rest-after");
        let mut phase = String::new();
        for beat in &summary.beats {
            if beat.phase != phase {
                phase = beat.phase.clone();
                let _ = writeln!(md, "  ── {phase} ──");
            }
            let _ = writeln!(
                md,
                "  {:6.1}  {:<11}  {:<20}  {:>8.2}s  {:>5.2}s  {:>8.2}s{}",
                beat.start,
                beat.phase,
                beat.profile,
                beat.telegraph_s,
                beat.strike_s,
                beat.rest_after_s,
                if beat.cut { "  (cut)" } else { "" }
            );
        }
        let _ = writeln!(md, "```");
    }
    let _ = writeln!(md, "\n## Findings\n");
    let found = findings(all);
    if found.is_empty() {
        let _ = writeln!(md, "- none of the heuristics fired");
    }
    for finding in found {
        let _ = writeln!(md, "- {finding}");
    }
    md
}

fn main() {
    let options = parse_options();
    std::fs::create_dir_all(&options.out).expect("the output directory can be created");
    let mut all = Vec::new();
    for policy in &options.policies {
        for seed in 0..options.seeds {
            eprintln!("fight_discovery: {} {} seed {seed}", options.room, policy.as_str());
            let run = run(&options, *policy, seed);
            let summary = summarize(&run);
            let path = options.out.join(format!("{}_seed{seed}.json", policy.as_str()));
            let beats: Vec<serde_json::Value> = summary
                .beats
                .iter()
                .map(|b| {
                    serde_json::json!({
                        "move": b.profile, "phase": b.phase, "t": b.start,
                        "telegraph_s": b.telegraph_s, "strike_s": b.strike_s, "rest_after_s": b.rest_after_s,
                        "cut": b.cut,
                    })
                })
                .collect();
            let body = serde_json::json!({ "summary": summary.json, "choreography": beats });
            std::fs::write(&path, serde_json::to_string_pretty(&body).expect("json")).expect("write run");
            eprintln!("  {} after {:.1}s", run.outcome, run.ticks.len() as f32 * DT);
            all.push((run, summary));
        }
    }
    let md = report(&options, &all);
    let path = options.out.join("report.md");
    std::fs::write(&path, &md).expect("write report");
    println!("{md}");
    eprintln!("fight_discovery: wrote {}", path.display());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(t: f32, beat: Option<(&str, Stage)>) -> Tick {
        Tick {
            t,
            phase: "Phase1".into(),
            beat: beat.map(|(p, s)| (p.to_string(), s)),
            boss_hp: 10,
            boss_alive: true,
            player_hp: 5,
            player_resets: 0,
            threat_area: 0.0,
            threat_on_player: false,
            punishable: false,
            hittable: Vec::new(),
            aim: None,
        }
    }

    #[test]
    fn the_same_move_twice_is_two_beats_and_the_quiet_belongs_to_the_one_before() {
        use Stage::*;
        let script = [
            Some(("slam", Telegraph)),
            Some(("slam", Strike)),
            None,
            Some(("slam", Telegraph)),
            Some(("slam", Strike)),
            Some(("sweep", Telegraph)),
        ];
        let ticks: Vec<Tick> = script.iter().enumerate().map(|(i, b)| tick(i as f32 * DT, *b)).collect();
        let found = beats(&ticks);
        let names: Vec<&str> = found.iter().map(|b| b.profile.as_str()).collect();
        assert_eq!(names, ["slam", "slam", "sweep"]);
        assert!((found[0].rest_after_s - DT).abs() < 1e-6, "one quiet tick after the first slam");
        assert_eq!(found[1].rest_after_s, 0.0, "the second slam runs straight into the sweep");
    }

    /// A beat ended by something other than the boss's next move measured the
    /// interruption. The fight ending on a kill read as "no rest after the
    /// pair slam" (0.00s, its strike cut at 0.72 of 2.0s), and a phase change
    /// restarting the script read the same way; both fired the commitment
    /// finding for a move whose authored rest is 0.8s.
    #[test]
    fn a_beat_the_record_or_a_phase_change_ends_is_cut_and_left_out_of_the_medians() {
        use Stage::*;
        let mut ticks = Vec::new();
        let mut push = |phase: &str, beat: Option<(&str, Stage)>| {
            let mut t = tick(ticks.len() as f32 * DT, beat);
            t.phase = phase.into();
            ticks.push(t);
        };
        // A whole slam: telegraph, strike, 3 quiet ticks, then the sweep.
        push("Phase1", Some(("slam", Telegraph)));
        push("Phase1", Some(("slam", Strike)));
        for _ in 0..3 {
            push("Phase1", None);
        }
        push("Phase1", Some(("sweep", Telegraph)));
        push("Phase1", Some(("sweep", Strike)));
        // The phase changes inside the sweep's rest — the change lands on a
        // quiet tick, so the sweep's last tick is already Phase2 (the shape
        // the pair slam had before Enrage in every run)…
        push("Phase1", None);
        push("Phase2", None);
        push("Phase2", Some(("slam", Telegraph)));
        // …and the record ends three ticks into a strike (longer than the whole
        // slam's one, so counting it would move the median).
        for _ in 0..3 {
            push("Phase2", Some(("slam", Strike)));
        }
        let found = beats(&ticks);
        let cut: Vec<(&str, bool)> = found.iter().map(|b| (b.profile.as_str(), b.cut)).collect();
        assert_eq!(cut, [("slam", false), ("sweep", true), ("slam", true)]);

        let run = Run { policy: Policy::Sandbag, seed: 0, arena_area: 1.0, ticks, outcome: "timeout" };
        let summary = Summary { json: serde_json::Value::Null, beats: found };
        let table = move_table(&[(run, summary)]);
        let slam = &table["slam"];
        assert_eq!((slam.beats, slam.cut), (2, 1));
        assert!((slam.rest.expect("one whole slam") - 3.0 * DT).abs() < 1e-6, "the whole slam's rest, not the cut one's 0");
        assert!((slam.strike.expect("one whole slam") - DT).abs() < 1e-6, "the whole slam's strike, not the cut one's");
        assert_eq!(table["sweep"].rest, None, "every sweep was cut: nothing to say about its rest");
    }
}
