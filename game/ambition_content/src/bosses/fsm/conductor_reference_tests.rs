//! The NATIVE Flying Spaghetti Monster conductor, kept as the reference trace
//! of its procedural module (`ambition_content_modules::fsm`). Test-only: the
//! game runs the module. `fsm_parity_tests` holds the module to it.
//!
//! The text below is the native conductor's, unchanged:
//!
//! The boss's scripted pattern decides WHICH move and WHEN (its `Special`
//! keys, telegraph then strike then rest). This decides everything that is
//! the move's own: where the god goes for it, the volume it swings, the shots
//! it throws, the row it is drawn with. The conductor owns the god's pose
//! ([`ae::PoseOwnedExternally`]) because none of the shared movement profiles
//! can do what it does — swim in beats like a jellyfish, plunge onto the floor
//! where you stood, and lie there tangled until its next move lifts it.
//!
//! A participant who drives the god owns its locomotion instead, except
//! through the dive, which moves the god by its own arc (`FsmConductor::scripts_pose`).
//! The god aims at its own [`ActorTarget`], the foe the engine selected for it.
//!
//! ⭐ THE FIGHT'S SHAPE. The god hovers so its noodles hang to just above your
//! head, and they STING: the bell is the target, the curtain under it is not
//! ([`sting`]). The bell can be reached two ways — from the arena's ledges, or
//! when it dives and lies stranded on the floor ([`Move::Dive`], the fight's
//! one long punish window). Each move pushes you between those two places:
//! the lash sweeps the ledges, the pulse throws everything near it away, the
//! grasp drags you up into the curtain, and the meatballs find you anywhere.

use ambition_platformer2d_core as ae;
use ae::Vec2;
use bevy::prelude::*;

use ambition_boss_encounter::{BossConfig, BossEncounter, BossEncounterPhase};
use ambition_characters::actor::BodyHealth;
use ambition_characters::brain::{BossAttackProfile, BossAttackState};
use ambition_combat::strike::{DamageBox, DepictedByOwner, Hitbox, HitboxAnchor, HitboxHits, HitboxKnockback, HitboxLifetime};
use ambition_platformer2d::sfx::{BodySfxWriter, SfxId, SfxMessage};
use ambition_platformer2d::sprite_sheet::character::PinnedRow;
use ambition_platformer2d::vfx::{ParticleKind, VfxMessage};
use ambition_platformer2d::vfx::VfxWriter;
use ambition_characters::control::DrivingParticipant;
use ambition_combat::components::ActorTarget;
use ambition_projectiles::{ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart};
use ambition_vfx::HitSide;

use crate::bosses::hall::{measure_hall, Hall};

pub const FSM_ID: &str = "flying_spaghetti_monster_boss";

/// The bell's half-extent: the boss's `combat_size` is (190, 88).
pub const BELL_HALF: Vec2 = Vec2::new(95.0, 44.0);
/// Where it swims: the bell's centre this far above the floor, so its noodles
/// (~140 long) hang to just over a standing player's head and its bell is out
/// of reach of a jump from the floor (MEASURED `fight_discovery`: at 230, 33
/// of a random player's 36 blows landed from a floor jump, and the ledges and
/// the dive were never needed). From a ledge, a jump reaches it.
pub const HOVER_ABOVE_FLOOR: f32 = 265.0;
/// One swim beat: squeeze, push, glide.
const BEAT_S: f32 = 1.6;
const BEAT_LIFT: f32 = 16.0;
/// Its gliding speed, and how far it keeps from the walls.
const SWIM_SPEED: f32 = 150.0;
/// It swims BESIDE you, not over you: a god keeps its distance, and a player
/// under its noodles is being stung, not fighting. Bell centre to you.
const SWIM_STANDOFF: f32 = 175.0;
const WALL_MARGIN: f32 = 150.0;

/// The curtain of noodles under the bell: as long as they are drawn (~130
/// below the bell), so a single jump under it (it rises ~83; a double ~134)
/// meets it. One sting per pass.
const STING_HALF: Vec2 = Vec2::new(88.0, 65.0);
const STING_DAMAGE: i32 = 1;
const STING_KNOCKBACK: f32 = 0.8;
const STING_REARM: f32 = 1.0;

/// The lash: a taut noodle straight out from the bell's side, a little below
/// its centre — the height of a player standing on a ledge.
const LASH_REACH: f32 = 270.0;
const LASH_HALF_H: f32 = 18.0;
const LASH_DROP: f32 = 40.0;
const LASH_DAMAGE: i32 = 2;
const LASH_KNOCKBACK: f32 = 1.3;
/// It lines up this far from you to lash, so the tip lands where you stand.
const LASH_STANDOFF: f32 = LASH_REACH * 0.72;

/// The volley: meatballs lobbed to land where you are, a little spread.
const MEATBALL_HALF: Vec2 = Vec2::new(14.0, 14.0);
const MEATBALL_GRAVITY: f32 = 900.0;
const MEATBALL_FLIGHT_S: f32 = 0.95;
const MEATBALL_DAMAGE: i32 = 2;
const MEATBALL_SPREAD: f32 = 90.0;

/// The pulse: the skirt thrown open, everything near the bell thrown away.
const PULSE_RADIUS: f32 = 205.0;
const PULSE_DAMAGE: i32 = 2;
const PULSE_KNOCKBACK: f32 = 1.6;

/// The grasp: one noodle reaching down and out for you; what it catches it
/// drags up toward the bell (into the curtain).
const GRASP_REACH: f32 = 290.0;
const GRASP_TIP_RADIUS: f32 = 34.0;
const GRASP_DAMAGE: i32 = 1;
const GRASP_PULL: f32 = 560.0;
/// The yank: TOWARD the god (a launch's `x` is mirrored away from its source,
/// and a volume riding the god has the god as its source) and up, into the
/// curtain under the bell.
const GRASP_PULL_DIR: Vec2 = Vec2::new(-0.5, -0.87);

/// The dive: up and over you on the tell, then down onto the floor.
const DIVE_RISE: f32 = 70.0;
const DIVE_TRACK_SPEED: f32 = 320.0;
const DIVE_FALL_S: f32 = 0.32;
/// Lying on the floor, its bell's centre is this far above it (the noodles
/// splayed flat under it).
const STRANDED_ABOVE_FLOOR: f32 = 50.0;
const RISE_S: f32 = 0.8;
/// The landing throws a shock each way along the floor: jump it.
const WAVE_SPEED: f32 = 430.0;
const WAVE_HALF: Vec2 = Vec2::new(24.0, 18.0);
const WAVE_DAMAGE: i32 = 1;
const WAVE_KNOCKBACK: f32 = 1.1;

const SFX_WHIP: &str = "boss.flying_spaghetti_monster.noodle_whip";
const SFX_MEATBALL: &str = "boss.flying_spaghetti_monster.meatball_impact";
const SFX_SPLASH: &str = "boss.flying_spaghetti_monster.sauce_splash";

/// The god's moves: the `Special` keys its pattern names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Lash,
    Volley,
    Pulse,
    Dive,
    Grasp,
    Appendages,
}

impl Move {
    pub const ALL: [Move; 6] = [Move::Lash, Move::Volley, Move::Pulse, Move::Dive, Move::Grasp, Move::Appendages];

    pub fn key(self) -> &'static str {
        match self {
            Move::Lash => "noodle_lash",
            Move::Volley => "meatball_volley",
            Move::Pulse => "noodly_pulse",
            Move::Dive => "noodly_dive",
            Move::Grasp => "noodly_grasp",
            Move::Appendages => "lesser_appendages",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mv| mv.key() == key)
    }

    /// The row it is drawn with, and how that row's frames split between the
    /// tell and the strike: the first `tell_frames` play over the telegraph,
    /// the rest over the strike.
    fn row(self) -> (&'static str, usize, usize, f32) {
        // (row, tell frames, total frames, seconds per frame) — the sheet's own.
        match self {
            Move::Lash => ("noodle_whip", 3, 7, 0.086),
            Move::Volley => ("meatball_volley", 3, 7, 0.088),
            Move::Pulse => ("pulse", 3, 8, 0.084),
            Move::Appendages => ("summon", 3, 8, 0.096),
            Move::Dive => ("dive", 4, 8, 0.084),
            Move::Grasp => ("grasp", 3, 8, 0.090),
        }
    }
}

/// The live part of a move: which, whether striking, and how far in.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Part {
    mv: Move,
    striking: bool,
    /// Seconds into this part.
    t: f32,
    /// This part's full length (elapsed plus what the pattern says remains).
    dur: f32,
}

/// One shock rolling along the floor.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Wave {
    hitbox: Entity,
    x: f32,
    dir: f32,
}

/// The conductor's memory, on the god.
#[derive(Component, Clone, Debug)]
pub struct FsmConductor {
    hall: Option<Hall>,
    part: Option<Part>,
    /// Last tick's time-remaining in the live part: a part that restarts (the
    /// same move twice) shows as this jumping back up.
    last_remaining: f32,
    /// The swim clock.
    clock: f32,
    ticks: u32,
    /// The side it faces and reaches toward: +1 right.
    side: f32,
    /// Where the dive will land, locked as the tell ends.
    dive_x: f32,
    /// Where the dive started falling from.
    dive_from: Vec2,
    /// Lying on the floor after a dive: seconds since it landed.
    stranded: Option<f32>,
    /// Rising from where it lay: from where, and for how long so far.
    rising: Option<(Vec2, f32)>,
    /// The strike's one-shot effect has been spent (the throw, the ring, the
    /// landing).
    fired: bool,
    /// Seconds until the curtain can sting again.
    sting_rearm: f32,
    waves: [Option<Wave>; 2],
}

impl FsmConductor {
    pub fn new(side: f32) -> Self {
        Self {
            hall: None,
            part: None,
            last_remaining: 0.0,
            clock: 0.0,
            ticks: 0,
            side,
            dive_x: 0.0,
            dive_from: Vec2::ZERO,
            stranded: None,
            rising: None,
            fired: false,
            sting_rearm: 0.0,
            waves: [None; 2],
        }
    }

    /// The dive moves the god by its own arc: up over its foe, down onto the
    /// floor, stranded there, and up again. The conductor keeps the pose
    /// through all of it, even while a participant drives the god.
    fn scripts_pose(&self, part: Option<Part>) -> bool {
        self.stranded.is_some()
            || self.rising.is_some()
            || matches!(part, Some(Part { mv: Move::Dive, .. }))
    }
}

/// The live part of the god's move.
fn live_part(attack: &BossAttackState) -> Option<(Move, bool, f32)> {
    let special = |profile: &Option<BossAttackProfile>| match profile {
        Some(BossAttackProfile::Special(key)) => Move::from_key(key),
        _ => None,
    };
    if let Some(mv) = special(&attack.active_profile) {
        return Some((mv, true, attack.active_remaining));
    }
    special(&attack.telegraph_profile).map(|mv| (mv, false, attack.telegraph_remaining))
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The swim: where the bell wants to be this tick, gliding in beats toward a
/// spot over the player (or `x_goal`), kept off the walls.
pub fn swim_goal(hall: &Hall, x_goal: f32, clock: f32) -> Vec2 {
    let beat = (clock / BEAT_S).fract();
    // A beat: a quick push up, a slow settle.
    let lift = if beat < 0.25 { ease(beat / 0.25) } else { 1.0 - ease((beat - 0.25) / 0.75) };
    let x = x_goal.clamp(hall.left + WALL_MARGIN, hall.right - WALL_MARGIN);
    Vec2::new(x, hall.floor - HOVER_ABOVE_FLOOR - BEAT_LIFT * lift)
}

/// Step from `at` toward `goal` at no more than `speed`.
fn glide(at: Vec2, goal: Vec2, speed: f32, dt: f32) -> Vec2 {
    let to = goal - at;
    let step = speed * dt;
    if to.length() <= step {
        goal
    } else {
        at + to.normalize() * step
    }
}

/// A meatball's launch so it lands on `target` after `MEATBALL_FLIGHT_S`.
pub fn lob(origin: Vec2, target: Vec2) -> (Vec2, f32) {
    let t = MEATBALL_FLIGHT_S;
    let v = (target - origin) / t - Vec2::new(0.0, 0.5 * MEATBALL_GRAVITY * t);
    (v.normalize_or_zero(), v.length())
}

fn play(sfx: &mut BodySfxWriter, body: Entity, id: &'static str, pos: Vec2) {
    sfx.write_for(body, SfxMessage::Play { id: SfxId::from_static(id), pos });
}

/// A volume riding the bell.
fn riding_hitbox(owner: Entity, offset: Vec2, half: Vec2, shape: Option<ae::VolumeShape>, damage: i32, knockback: HitboxKnockback, launch_dir: Option<Vec2>, life: f32) -> impl Bundle {
    (
        Hitbox {
            strike_sfx: None,
            owner,
            source: HitSide::Boss,
            anchor: HitboxAnchor::FollowOwner { local_offset: offset },
            half_extent: half,
            shape,
            facing: 1.0,
            damage,
            knockback,
            launch_dir,
            frame_down: Vec2::new(0.0, 1.0),
            reaction: None,
        },
        HitboxLifetime { remaining_s: life },
        HitboxHits::default(),
        // Its own art shows every volume that rides it: the noodles are the
        // sting, the rows are the lash, the pulse and the grasp.
        DepictedByOwner,
    )
}

/// Perform one tick: fly the god, perform its live move, sting, and draw it.
pub fn conduct_fsm(
    mut commands: Commands,
    time: Res<ambition_time::WorldTime>,
    // The hall is the god's own live room's (OW1 cut 7).
    world: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<ae::RoomGeometry>,
    mut gods: Query<
        (
            Entity,
            &BossAttackState,
            &mut FsmConductor,
            &mut ae::BodyKinematics,
            &mut ae::CenteredAabb,
            &BodyHealth,
            &BossEncounter,
            Option<&mut ae::SweepSample>,
            &mut PinnedRow,
            &ActorTarget,
            Has<DrivingParticipant>,
            Has<ae::PoseOwnedExternally>,
        ),
        With<BossConfig>,
    >,
    mut hitboxes: Query<&mut Hitbox>,
    mut projectiles: MessageWriter<ProjectileSpawnRequest>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut vfx: VfxWriter,
    mut sfx: BodySfxWriter,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for (god, attack, mut conductor, mut kin, mut aabb, health, encounter, mut sweep, mut row, foe, driven, pose_owned) in &mut gods {
        let conductor = &mut *conductor;
        let hall = match conductor.hall {
            Some(hall) => hall,
            None => {
                let Some(hall) = world.of(god).and_then(|geometry| measure_hall(&geometry.0, kin.pos)) else {
                    continue;
                };
                conductor.hall = Some(hall);
                hall
            }
        };
        conductor.clock += dt;
        conductor.ticks = conductor.ticks.wrapping_add(1);
        // Every effect of the fight is drawn in the god's own live room.
        let mut vfx = vfx.for_room(world.room_of(god));
        let at = kin.pos;
        let target = foe.entity.map_or(Vec2::new(hall.center_x(), hall.floor - 24.0), |_| foe.pos);

        // ── The landing's shocks roll out along the floor ──
        //
        // Before the move runs: a landing below spawns its shocks through
        // deferred commands, so this tick they are not in `hitboxes` yet and
        // would read as gone. Each shock moves first on the next tick.
        for slot in 0..conductor.waves.len() {
            let Some(mut wave) = conductor.waves[slot] else {
                continue;
            };
            wave.x += wave.dir * WAVE_SPEED * dt;
            let gone = wave.x < hall.left + WAVE_HALF.x || wave.x > hall.right - WAVE_HALF.x;
            if gone || hitboxes.get(wave.hitbox).is_err() {
                commands.entity(wave.hitbox).try_despawn();
                conductor.waves[slot] = None;
                continue;
            }
            let center = Vec2::new(wave.x, hall.floor - WAVE_HALF.y);
            if let Ok(mut hitbox) = hitboxes.get_mut(wave.hitbox) {
                hitbox.anchor = HitboxAnchor::World { center };
            }
            if conductor.ticks % 3 == 0 {
                vfx.write(VfxMessage::Burst { pos: center, count: 3, speed: 120.0, color: [0.93, 0.85, 0.62, 1.0], kind: ParticleKind::Dust });
            }
            conductor.waves[slot] = Some(wave);
        }

        // ── Dead: its shocks end with it; it sinks, limp, to the floor, and is
        // drawn by the engine. ──
        if !health.alive() {
            for wave in conductor.waves.iter_mut().filter_map(Option::take) {
                commands.entity(wave.hitbox).try_despawn();
            }
            row.clear();
            if !pose_owned {
                commands.entity(god).try_insert(ae::PoseOwnedExternally);
            }
            let rest = Vec2::new(at.x, hall.floor - STRANDED_ABOVE_FLOOR);
            let pos = glide(at, rest, 120.0, dt);
            ae::movement::constrain_body_pose(&mut kin, sweep.as_deref_mut(), pos, (pos - at) / dt);
            aabb.center = pos;
            continue;
        }

        // ── Which part of which move is live ──
        let live = live_part(attack);
        let fresh = match (conductor.part, live) {
            (Some(prev), Some((mv, striking, remaining))) => {
                prev.mv != mv || prev.striking != striking || remaining > conductor.last_remaining + 1e-3
            }
            (None, Some(_)) => true,
            _ => false,
        };
        conductor.part = match live {
            Some((mv, striking, remaining)) => {
                if fresh {
                    // A move starting anywhere but its own strike lifts a
                    // stranded god off the floor.
                    if conductor.stranded.is_some() && !(mv == Move::Dive && striking) {
                        conductor.stranded = None;
                        conductor.rising = Some((at, 0.0));
                    }
                    if !striking {
                        conductor.side = if target.x < at.x { -1.0 } else { 1.0 };
                    } else {
                        conductor.fired = false;
                        if mv == Move::Dive {
                            conductor.dive_from = at;
                            conductor.dive_x = at.x;
                        }
                    }
                    Some(Part { mv, striking, t: 0.0, dur: remaining.max(1e-3) })
                } else {
                    conductor.part.map(|p| Part { t: p.t + dt, ..p })
                }
            }
            None => None,
        };
        conductor.last_remaining = live.map_or(0.0, |(_, _, remaining)| remaining);
        let part = conductor.part;

        // ── Where the god goes ──
        // Between moves it turns to face you.
        if part.is_none() && conductor.stranded.is_none() {
            conductor.side = if target.x < at.x { -1.0 } else { 1.0 };
        }
        let swim = swim_goal(&hall, target.x - conductor.side * SWIM_STANDOFF, conductor.clock);
        let pos = if let Some(stranded) = conductor.stranded.as_mut() {
            *stranded += dt;
            Vec2::new(at.x, hall.floor - STRANDED_ABOVE_FLOOR)
        } else if let Some((from, t)) = conductor.rising.as_mut() {
            *t += dt;
            let u = ease(*t / RISE_S);
            let pos = *from + (swim - *from) * u;
            if *t >= RISE_S {
                conductor.rising = None;
            }
            pos
        } else {
            match part {
                Some(Part { mv: Move::Dive, striking: false, t, dur }) => {
                    // Up, and over where you stand.
                    let over = Vec2::new(target.x, swim.y - DIVE_RISE * ease(t / dur));
                    let over = Vec2::new(over.x.clamp(hall.left + BELL_HALF.x, hall.right - BELL_HALF.x), over.y);
                    glide(at, over, DIVE_TRACK_SPEED, dt)
                }
                Some(Part { mv: Move::Dive, striking: true, t, .. }) => {
                    let floor_y = hall.floor - STRANDED_ABOVE_FLOOR;
                    let u = (t / DIVE_FALL_S).clamp(0.0, 1.0);
                    // Falling: slow off the top, fast at the floor.
                    let y = conductor.dive_from.y + (floor_y - conductor.dive_from.y) * u * u;
                    if u >= 1.0 && !conductor.fired {
                        conductor.fired = true;
                        conductor.stranded = Some(0.0);
                        let x = conductor.dive_x;
                        land(&mut commands, god, conductor, &hall, x, &mut vfx, &mut sfx);
                    }
                    Vec2::new(conductor.dive_x, y)
                }
                Some(Part { mv: Move::Lash, striking: false, .. }) => {
                    // Line up at lash range, at your side.
                    let goal = Vec2::new(target.x - conductor.side * LASH_STANDOFF, swim.y);
                    glide(at, swim_goal(&hall, goal.x, conductor.clock), SWIM_SPEED * 1.6, dt)
                }
                // Striking, it holds where it is; otherwise it swims over you.
                Some(Part { striking: true, .. }) => at,
                _ => glide(at, swim, SWIM_SPEED, dt),
            }
        };
        // ── Who moves the god ──
        // The conductor, unless a participant drives it outside the dive.
        // `PoseOwnedExternally` tells the body integrator to leave the god's
        // locomotion alone, so it is set to agree with this answer.
        let conducts = !driven || conductor.scripts_pose(part);
        if conducts {
            ae::movement::constrain_body_pose(&mut kin, sweep.as_deref_mut(), pos, (pos - at) / dt);
            aabb.center = pos;
        } else {
            conductor.side = if kin.facing < 0.0 { -1.0 } else { 1.0 };
        }
        match (conducts, pose_owned) {
            (true, false) => {
                commands.entity(god).try_insert(ae::PoseOwnedExternally);
            }
            (false, true) => {
                commands.entity(god).try_remove::<ae::PoseOwnedExternally>();
            }
            _ => {}
        }
        // A driven god's moves start where the participant put it.
        let pos = if conducts { pos } else { kin.pos };

        // ── The strike's own onset ──
        if let Some(part) = part {
            if part.striking && !conductor.fired {
                match part.mv {
                    Move::Lash => {
                        conductor.fired = true;
                        let half = Vec2::new(LASH_REACH * 0.5, LASH_HALF_H);
                        let offset = Vec2::new(conductor.side * (BELL_HALF.x * 0.6 + LASH_REACH * 0.5), LASH_DROP);
                        commands.spawn((
                            riding_hitbox(god, offset, half, None, LASH_DAMAGE, HitboxKnockback::FeelScale(LASH_KNOCKBACK), None, part.dur),
                            Name::new("fsm_lash"),
                        ));
                        play(&mut sfx, god, SFX_WHIP, pos);
                    }
                    Move::Volley => {
                        conductor.fired = true;
                        let count = if encounter.encounter_phase() == BossEncounterPhase::Enrage { 5 } else { 3 };
                        // From the throwing meatball, on the bell's front half.
                        // A shot never touches its thrower, so it flies out of
                        // the bell.
                        let origin = pos + Vec2::new(conductor.side * BELL_HALF.x * 0.5, -10.0);
                        for k in 0..count {
                            let spread = (k as f32 - (count - 1) as f32 * 0.5) * MEATBALL_SPREAD;
                            let (dir, speed) = lob(origin, Vec2::new(target.x + spread, target.y));
                            projectiles.write(ProjectileSpawnRequest::open(
                                god,
                                ProjectileSpawn {
                                    origin,
                                    dir,
                                    speed,
                                    damage: MEATBALL_DAMAGE,
                                    max_lifetime: 3.0,
                                    half_extent: MEATBALL_HALF,
                                    gravity: MEATBALL_GRAVITY,
                                    visual_id: "meatball".to_string(),
                                    // A meatball bounces once off the floor
                                    // and rolls on: jump it, don't stand under it.
                                    bounces: 1,
                                    bounce_on_world_contact: true,
                                    splash_half_extent: 0.0,
                                    boomerang_return_s: None,
                                },
                                ProjectileStart::StepThisTick,
                            ));
                        }
                        play(&mut sfx, god, SFX_MEATBALL, origin);
                    }
                    Move::Pulse => {
                        conductor.fired = true;
                        commands.spawn((
                            riding_hitbox(
                                god,
                                Vec2::ZERO,
                                Vec2::splat(PULSE_RADIUS),
                                Some(ae::VolumeShape::Circle { radius: PULSE_RADIUS }),
                                PULSE_DAMAGE,
                                HitboxKnockback::FeelScale(PULSE_KNOCKBACK),
                                None,
                                part.dur.min(0.3),
                            ),
                            Name::new("fsm_pulse"),
                        ));
                        play(&mut sfx, god, SFX_SPLASH, pos);
                        vfx.write(VfxMessage::Burst { pos, count: 24, speed: 380.0, color: [0.95, 0.88, 0.7, 1.0], kind: ParticleKind::Spark });
                    }
                    Move::Grasp => {
                        conductor.fired = true;
                        // Reach for where you are, within reach, and never up.
                        let base = pos + Vec2::new(conductor.side * BELL_HALF.x * 0.5, BELL_HALF.y);
                        let mut to = target - base;
                        to.y = to.y.max(to.x.abs() * 0.2);
                        let reach = to.length().min(GRASP_REACH);
                        let tip = base + to.normalize_or_zero() * reach;
                        commands.spawn((
                            riding_hitbox(
                                god,
                                tip - pos,
                                Vec2::splat(GRASP_TIP_RADIUS),
                                Some(ae::VolumeShape::Circle { radius: GRASP_TIP_RADIUS }),
                                GRASP_DAMAGE,
                                HitboxKnockback::LaunchSpeed { base: GRASP_PULL, growth: Some(0.0) },
                                Some(GRASP_PULL_DIR),
                                part.dur,
                            ),
                            Name::new("fsm_grasp"),
                        ));
                        play(&mut sfx, god, SFX_WHIP, tip);
                    }
                    Move::Appendages => {
                        conductor.fired = true;
                        super::appendages_reference_tests::summon(&mut effects, god, pos, &hall, conductor.ticks);
                        play(&mut sfx, god, SFX_SPLASH, pos);
                    }
                    // The dive fires on landing, above.
                    Move::Dive => {}
                }
            }
        }

        // ── The curtain stings ──
        conductor.sting_rearm -= dt;
        let curtain_down = conductor.stranded.is_none()
            && conductor.rising.is_none()
            && !matches!(part, Some(Part { mv: Move::Dive, .. }));
        if curtain_down && conductor.sting_rearm <= 0.0 {
            conductor.sting_rearm = STING_REARM;
            commands.spawn((
                riding_hitbox(
                    god,
                    Vec2::new(0.0, BELL_HALF.y + STING_HALF.y),
                    STING_HALF,
                    None,
                    STING_DAMAGE,
                    HitboxKnockback::FeelScale(STING_KNOCKBACK),
                    None,
                    STING_REARM,
                ),
                Name::new("fsm_sting"),
            ));
        }

        // ── What it is drawn as ──
        let speed = if conducts { (pos - at).length() / dt } else { kin.vel.length() };
        match drawn_row(conductor, part, speed) {
            Some((name, elapsed, looping)) => row.pin(&[name], elapsed, looping),
            None => row.clear(),
        }
    }
}

/// Turn the god to the side its conductor chose, through the control the body
/// integrator applies. Only while the conductor holds the pose: a driven god
/// faces where its participant steers it.
///
/// The boss brain turns every boss toward its target each tick; the
/// conductor keeps the side a move chose when its tell began, so the god is
/// drawn facing where its move goes. Runs in `BossSteerSlot`, after the brain
/// and before the integration.
pub fn face_conducted_gods(
    mut gods: Query<(&FsmConductor, &mut ambition_characters::control::ActorControl), With<ae::PoseOwnedExternally>>,
) {
    for (conductor, mut control) in &mut gods {
        control.0.facing = conductor.side;
    }
}

/// The dive lands: a shock each way along the floor, and a splash.
fn land(
    commands: &mut Commands,
    god: Entity,
    conductor: &mut FsmConductor,
    hall: &Hall,
    x: f32,
    vfx: &mut ambition_platformer2d::vfx::VfxForRoom<'_, '_>,
    sfx: &mut BodySfxWriter,
) {
    let feet = Vec2::new(x, hall.floor - WAVE_HALF.y);
    for (slot, dir) in [-1.0f32, 1.0].into_iter().enumerate() {
        let wx = x + dir * BELL_HALF.x;
        let hitbox = ambition_combat::strike::spawn_damage_box(
            commands,
            god,
            HitSide::Boss,
            Vec2::new(wx, feet.y),
            DamageBox {
                half_extent: WAVE_HALF,
                shape: None,
                damage: WAVE_DAMAGE,
                knockback: WAVE_KNOCKBACK,
                lifetime_s: 4.0,
                name: Some("fsm_dive_shock"),
            },
        );
        if let Some(old) = conductor.waves[slot].take() {
            commands.entity(old.hitbox).try_despawn();
        }
        conductor.waves[slot] = Some(Wave { hitbox, x: wx, dir });
    }
    play(sfx, god, SFX_SPLASH, feet);
    vfx.write(VfxMessage::Burst { pos: feet, count: 30, speed: 340.0, color: [0.93, 0.85, 0.62, 1.0], kind: ParticleKind::Dust });
}

/// The row the god is drawn with: the live move's, played so its tell frames
/// fill the telegraph and the rest fill the strike; lying on the floor, the
/// dive's splayed last frame; otherwise swimming or idling.
fn drawn_row(conductor: &FsmConductor, part: Option<Part>, speed: f32) -> Option<(&'static str, f32, bool)> {
    if conductor.stranded.is_some() {
        let (name, _, total, fd) = Move::Dive.row();
        return Some((name, total as f32 * fd, false));
    }
    if conductor.rising.is_some() {
        return Some(("drift", conductor.clock, true));
    }
    if let Some(part) = part {
        let (name, tell, total, fd) = part.mv.row();
        let u = (part.t / part.dur).clamp(0.0, 0.999);
        let frame = if part.striking {
            tell as f32 + u * (total - tell) as f32
        } else {
            u * tell as f32
        };
        return Some((name, frame * fd, false));
    }
    if speed > 60.0 {
        Some(("drift", conductor.clock, true))
    } else {
        Some(("idle", conductor.clock, true))
    }
}
