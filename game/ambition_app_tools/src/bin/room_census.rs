//! Room census: run an encounter room headlessly and say what its bodies do, in
//! numbers — the non-boss sibling of `fight_discovery`.
//!
//! A room of ordinary actors has no choreography to read, so this records what
//! a player would notice instead, per body:
//!
//! * WHERE IT SPENDS THE FIGHT: how high above the player, how often it is
//!   pressed against or buried in the ceiling;
//! * WHETHER IT DITHERS: how often its vertical velocity reverses, per second;
//! * WHAT IT SOUNDS LIKE: every [`SfxMessage`] the room emits, attributed to
//!   the nearest body, by kind, with the share that repeats within
//!   [`CHATTER_S`] of the same kind from the same body — a sound that stutters.
//!
//! ⛔ Attribution is by POSITION (a sound names where, not who). A body within
//! [`ATTRIBUTE_PX`] of the sound owns it; the player's own sounds are split out;
//! the rest is reported as `(elsewhere)`, never dropped.
//!
//! ```text
//! cargo run -p ambition_app_tools --release --bin room_census -- \
//!     --room pirate_sky_lookout [--policy all|sandbag|chase] [--seconds 60]
//! ```

use std::collections::BTreeMap;
use std::fmt::Write as _;

use ambition_app::rl_sim::{
    AgentAction, AgentObservation, AmbitionSim, Platformer2dSimHarness,
    Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
use ambition_platformer2d::actors::features::ecs::dormancy::Dormant;
use ambition_platformer2d::combat::components::{ActorFaction, FeatureId};
use ambition_platformer2d::combat::ActorTarget;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::mount::RidingOn;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::sfx::{OwnedSfxMessage, SfxMessage};
use bevy::ecs::message::{MessageCursor, Messages};
use bevy::prelude::{Entity, Has, World};

const DT: f32 = 1.0 / 60.0;
/// A sound within this of a body's centre is that body's.
const ATTRIBUTE_PX: f32 = 96.0;
/// The same kind of sound from the same body again within this is a stutter.
const CHATTER_S: f32 = 0.25;
/// A vertical velocity smaller than this has no direction worth counting.
const STILL_VY: f32 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    /// No input: what the room does to someone standing in it.
    Sandbag,
    /// Walks under the nearest hostile, jumps at it, swings.
    Chase,
}

impl Policy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Sandbag => "sandbag",
            Self::Chase => "chase",
        }
    }
}

struct Options {
    room: String,
    policies: Vec<Policy>,
    seconds: f32,
}

fn parse_options() -> Options {
    let mut args = std::env::args().skip(1);
    let (mut room, mut policy, mut seconds) = (None, "all".to_string(), 60.0);
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| usage(&format!("{arg} needs a value")));
        match arg.as_str() {
            "--room" => room = Some(value()),
            "--policy" => policy = value(),
            "--seconds" => seconds = value().parse().unwrap_or_else(|_| usage("--seconds is a number")),
            "-h" | "--help" => usage(""),
            other => usage(&format!("unknown argument {other}")),
        }
    }
    let policies = match policy.as_str() {
        "all" => vec![Policy::Sandbag, Policy::Chase],
        "sandbag" => vec![Policy::Sandbag],
        "chase" => vec![Policy::Chase],
        word => usage(&format!("unknown policy {word}")),
    };
    Options { room: room.unwrap_or_else(|| usage("--room is required")), policies, seconds }
}

fn usage(problem: &str) -> ! {
    if !problem.is_empty() {
        eprintln!("room_census: {problem}");
    }
    eprintln!("usage: room_census --room <room> [--policy all|sandbag|chase] [--seconds N]");
    std::process::exit(2);
}

/// What one body did over the run.
#[derive(Default)]
struct Body {
    who: String,
    rides: Option<String>,
    ticks: usize,
    at_ceiling: usize,
    buried: usize,
    /// Player y minus body y each tick: positive is above the player.
    above_player: Vec<f32>,
    distance: Vec<f32>,
    x: Vec<f32>,
    top_min: f32,
    vy_sign: f32,
    vy_flips: usize,
    alive_ticks: usize,
    dormant: usize,
    moving: usize,
    targeting: usize,
    /// Kind -> (count, stutters).
    sounds: BTreeMap<String, (usize, usize)>,
    last_sound: BTreeMap<String, f32>,
}

struct Observed {
    id: String,
    pos: ae::Vec2,
}

fn sound(message: &SfxMessage) -> (String, ae::Vec2) {
    match message {
        SfxMessage::Jump { pos } => ("jump".into(), *pos),
        SfxMessage::DoubleJump { pos } => ("double_jump".into(), *pos),
        SfxMessage::Dash { pos } => ("dash".into(), *pos),
        SfxMessage::Blink { pos, .. } => ("blink".into(), *pos),
        SfxMessage::Pogo { pos } => ("pogo".into(), *pos),
        SfxMessage::Land { pos } => ("land".into(), *pos),
        SfxMessage::Slash { pos } => ("slash".into(), *pos),
        SfxMessage::Hit { pos } => ("hit".into(), *pos),
        SfxMessage::Death { pos } => ("death".into(), *pos),
        SfxMessage::Reset { pos } => ("reset".into(), *pos),
        SfxMessage::Play { id, pos } => (format!("play:{id}"), *pos),
    }
}

fn solids(world: &mut World) -> Vec<ae::Aabb> {
    let mut query = world.query::<&ae::RoomGeometry>();
    query
        .iter(world)
        .next()
        .map(|geometry| {
            geometry
                .0
                .blocks
                .iter()
                .filter(|block| matches!(block.kind, ae::BlockKind::Solid))
                .map(|block| block.aabb)
                .collect()
        })
        .unwrap_or_default()
}

/// `(pressed against a ceiling, buried in solid)` for a body box, y-down.
fn ceiling_contact(solids: &[ae::Aabb], pos: ae::Vec2, size: ae::Vec2) -> (bool, bool) {
    let (min, max) = (pos - size * 0.5, pos + size * 0.5);
    let across = |b: &ae::Aabb| b.min.x < max.x - 1.0 && b.max.x > min.x + 1.0;
    let buried = solids.iter().any(|b| across(b) && b.min.y < max.y - 1.0 && b.max.y > min.y + 1.0);
    let pressed = solids.iter().any(|b| across(b) && (b.max.y - min.y).abs() <= 3.0);
    (pressed, buried)
}

struct Run {
    policy: Policy,
    bodies: BTreeMap<String, Body>,
    player_sounds: BTreeMap<String, usize>,
    elsewhere: BTreeMap<String, usize>,
    player_hp: (i32, i32),
    seconds: f32,
}

fn run(options: &Options, policy: Policy) -> Run {
    let harness = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(options.room.clone());
    let mut sim = Platformer2dSimHarness::new_with_options(harness)
        .unwrap_or_else(|error| panic!("the room `{}` builds headlessly: {error}", options.room));
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let player = {
        let world = sim.world_mut();
        let mut query = world.query_filtered::<Entity, PrimaryPlayerOnly>();
        query.iter(world).next().expect("the room seats a player")
    };
    let hp_of = |world: &World| world.get::<BodyHealth>(player).map_or(0, |h| h.current());
    let hp_start = hp_of(sim.world());
    let mut cursor: MessageCursor<OwnedSfxMessage> =
        sim.world().resource::<Messages<OwnedSfxMessage>>().get_cursor_current();
    let solids = solids(sim.world_mut());
    let mut bodies: BTreeMap<String, Body> = BTreeMap::new();
    let (mut player_sounds, mut elsewhere) = (BTreeMap::new(), BTreeMap::new());
    let mut driver_tick = 0u32;
    let frames = (options.seconds / DT) as usize;
    let mut obs = sim.observation();
    for frame in 0..frames {
        let t = frame as f32 * DT;
        let world = sim.world_mut();
        let player_pos = world.get::<ae::BodyKinematics>(player).map_or(ae::Vec2::ZERO, |k| k.pos);
        let mut query = world.query::<(
            Entity,
            &FeatureId,
            &ae::BodyKinematics,
            Option<&WornCharacter>,
            Option<&RidingOn>,
            Option<&BodyHealth>,
            Option<&ActorFaction>,
            Has<Dormant>,
            Option<&ActorTarget>,
        )>();
        let mut seen = Vec::new();
        let mut nearest_hostile: Option<ae::Vec2> = None;
        let rows: Vec<_> = query
            .iter(world)
            .filter(|(entity, ..)| *entity != player)
            .map(|(entity, id, kin, worn, riding, health, faction, dormant, target)| {
                let state = (dormant, target.is_some_and(|t| t.entity.is_some()));
                (entity, id.as_str().to_string(), kin.clone(), worn.map(|w| w.0.to_string()), riding.map(|r| r.mount), health.map(|h| h.alive()), faction.copied(), state)
            })
            .collect();
        for (_, id, kin, worn, mount, alive, faction, (dormant, targeting)) in rows {
            let rides = mount.and_then(|m| world.get::<FeatureId>(m)).map(|f| f.as_str().to_string());
            let body = bodies.entry(id.clone()).or_insert_with(|| Body {
                who: worn.unwrap_or_else(|| "?".into()),
                top_min: f32::MAX,
                ..Default::default()
            });
            body.rides = rides;
            body.ticks += 1;
            if alive != Some(true) {
                continue;
            }
            body.alive_ticks += 1;
            body.dormant += usize::from(dormant);
            body.targeting += usize::from(targeting);
            body.moving += usize::from(kin.vel.length() > 5.0);
            let (pressed, buried) = ceiling_contact(&solids, kin.pos, kin.size);
            body.at_ceiling += usize::from(pressed);
            body.buried += usize::from(buried);
            body.above_player.push(player_pos.y - kin.pos.y);
            body.distance.push((kin.pos - player_pos).length());
            body.x.push(kin.pos.x);
            body.top_min = body.top_min.min(kin.pos.y - kin.size.y * 0.5);
            if kin.vel.y.abs() > STILL_VY {
                let sign = kin.vel.y.signum();
                if body.vy_sign != 0.0 && sign != body.vy_sign {
                    body.vy_flips += 1;
                }
                body.vy_sign = sign;
            }
            if matches!(faction, Some(ActorFaction::Enemy)) {
                let closer = nearest_hostile.is_none_or(|h| (h - player_pos).length() > (kin.pos - player_pos).length());
                if closer {
                    nearest_hostile = Some(kin.pos);
                }
            }
            seen.push(Observed { id, pos: kin.pos });
        }
        // The sounds this tick produced, owned by the nearest body.
        let messages = world.resource::<Messages<OwnedSfxMessage>>();
        for message in cursor.read(messages) {
            let (kind, pos) = sound(&message.request);
            if (pos - player_pos).length() < ATTRIBUTE_PX * 0.5 {
                *player_sounds.entry(kind).or_default() += 1;
                continue;
            }
            let owner = seen
                .iter()
                .map(|o| (o, (o.pos - pos).length()))
                .filter(|(_, d)| *d < ATTRIBUTE_PX)
                .min_by(|a, b| a.1.total_cmp(&b.1));
            match owner {
                Some((o, _)) => {
                    let body = bodies.get_mut(&o.id).expect("observed this tick");
                    let stutter = body.last_sound.get(&kind).is_some_and(|last| t - last < CHATTER_S);
                    let row = body.sounds.entry(kind.clone()).or_default();
                    row.0 += 1;
                    row.1 += usize::from(stutter);
                    body.last_sound.insert(kind, t);
                }
                None => *elsewhere.entry(kind).or_default() += 1,
            }
        }
        driver_tick += 1;
        let action = act(policy, driver_tick, &obs, nearest_hostile);
        obs = sim.step(action);
        if obs.active_room != options.room {
            break;
        }
    }
    let hp_end = hp_of(sim.world());
    Run { policy, bodies, player_sounds, elsewhere, player_hp: (hp_start, hp_end), seconds: options.seconds }
}

fn act(policy: Policy, tick: u32, obs: &AgentObservation, aim: Option<ae::Vec2>) -> AgentAction {
    let mut action = AgentAction::default();
    if policy == Policy::Chase {
        let player = ae::Vec2::new(obs.player_pos.0, obs.player_pos.1);
        if let Some(aim) = aim {
            let d = aim - player;
            let x = if d.x.abs() > 40.0 { d.x.signum() } else { 0.0 };
            action.move_x = x;
            action.left_pressed = x < 0.0;
            action.right_pressed = x > 0.0;
            if d.y < -60.0 && obs.on_ground && tick % 30 == 0 {
                action.jump = true;
            }
            action.jump_held = !obs.on_ground && d.y < 0.0;
            if d.length() < 120.0 && tick % 10 == 0 {
                action.attack = true;
            }
        }
    }
    action.attack_held = action.attack;
    action
}

fn median(mut v: Vec<f32>) -> f32 {
    if v.is_empty() {
        return f32::NAN;
    }
    v.sort_by(f32::total_cmp);
    v[v.len() / 2]
}

fn report(options: &Options, runs: &[Run]) -> String {
    let mut md = String::new();
    let _ = writeln!(md, "# Room census: `{}`\n", options.room);
    let _ = writeln!(md, "Measured by `room_census` ({} s per policy). Sounds are attributed by position.\n", options.seconds);
    for run in runs {
        let _ = writeln!(md, "## {} — player hp {} → {}\n", run.policy.as_str(), run.player_hp.0, run.player_hp.1);
        let _ = writeln!(
            md,
            "| body | who | rides | alive | dormant | targeting | moving | distance (median px) | x (median) | at ceiling | buried | above player (median px) | highest top y | vy reversals /s | sounds (count, stutters) |"
        );
        let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
        for (id, b) in &run.bodies {
            let alive_s = b.alive_ticks as f32 * DT;
            let pct = |n: usize| if b.alive_ticks == 0 { 0.0 } else { 100.0 * n as f32 / b.alive_ticks as f32 };
            let sounds: Vec<String> = b.sounds.iter().map(|(k, (n, s))| format!("{k} {n} ({s})")).collect();
            let _ = writeln!(
                md,
                "| `{id}` | {} | {} | {:.1}s | {:.0}% | {:.0}% | {:.0}% | {:.0} | {:.0} | {:.0}% | {:.0}% | {:.0} | {:.0} | {:.2} | {} |",
                b.who,
                b.rides.as_deref().unwrap_or("—"),
                alive_s,
                pct(b.dormant),
                pct(b.targeting),
                pct(b.moving),
                median(b.distance.clone()),
                median(b.x.clone()),
                pct(b.at_ceiling),
                pct(b.buried),
                median(b.above_player.clone()),
                if b.top_min == f32::MAX { f32::NAN } else { b.top_min },
                if alive_s > 0.0 { b.vy_flips as f32 / alive_s } else { 0.0 },
                if sounds.is_empty() { "—".into() } else { sounds.join(", ") },
            );
        }
        let line = |m: &BTreeMap<String, usize>| {
            if m.is_empty() { "—".to_string() } else { m.iter().map(|(k, n)| format!("{k} {n}")).collect::<Vec<_>>().join(", ") }
        };
        let per_s = |m: &BTreeMap<String, usize>| m.values().sum::<usize>() as f32 / run.seconds;
        let body_total: usize = run.bodies.values().flat_map(|b| b.sounds.values().map(|(n, _)| n)).sum();
        let _ = writeln!(md, "\n- player's own sounds: {}", line(&run.player_sounds));
        let _ = writeln!(md, "- unattributed: {}", line(&run.elsewhere));
        let _ = writeln!(
            md,
            "- body sounds per second: {:.2}; unattributed per second: {:.2}\n",
            body_total as f32 / run.seconds,
            per_s(&run.elsewhere)
        );
    }
    md
}

fn main() {
    let options = parse_options();
    let runs: Vec<Run> = options
        .policies
        .iter()
        .map(|policy| {
            eprintln!("room_census: {} {}", options.room, policy.as_str());
            run(&options, *policy)
        })
        .collect();
    println!("{}", report(&options, &runs));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_under_a_ceiling_is_pressed_and_one_inside_it_is_buried() {
        let ceiling = [ae::Aabb::new(ae::Vec2::new(100.0, 16.0), ae::Vec2::new(100.0, 16.0))];
        let size = ae::Vec2::new(20.0, 40.0);
        assert_eq!(ceiling_contact(&ceiling, ae::Vec2::new(100.0, 52.0), size), (true, false), "top at 32");
        assert_eq!(ceiling_contact(&ceiling, ae::Vec2::new(100.0, 40.0), size), (false, true), "top at 20");
        assert_eq!(ceiling_contact(&ceiling, ae::Vec2::new(100.0, 90.0), size), (false, false), "clear of it");
    }
}
