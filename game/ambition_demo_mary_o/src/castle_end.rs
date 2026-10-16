//! The end of a castle: a switch, the fall of the bridge, a gate, and who waits past it.
//!
//! A castle has no pole. Its goal is a switch (the block [`GOAL_SWITCH`]) past
//! the bridge of its boss, and a wall with a gate in it (the block
//! [`GOAL_GATE`]) is past the switch. A body takes the switch when it LANDS on
//! it. Then the end of the castle plays, one beat after the other
//! ([`CastleBeat`]), and each beat has its time, so the player sees each thing
//! (Jon, 2026-10-12: "There should be a sequence of beats to this so it
//! doesn't happen in a blink of an eye"): the switch goes down, charges go off
//! on the bridge, the planks break one by one, the lava takes the boss, the
//! gate opens, she walks to the one she saved, he thanks her, and the level
//! waits before it goes.
//!
//! The beat is a phase of the sequence of the level
//! ([`FlagPhase::Castle`]), so it rewinds with the sequence, and each reader
//! of "the level is at its end" (the clock, the hold on her controls, the
//! tally) reads it as it reads a pole. The level gives the bridge one view
//! that does not move (`Level.arena`), so each beat is in it.

use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability, WornCharacter};
use ambition_platformer2d::characters::control::{claim_control_hold, ControlHold, ControlHolds};
use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::cutscene::{CutsceneBeat, CutsceneLibrary, CutsceneScript, CutsceneTriggerQueue};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::lifecycle::{LiveRoomOf, LiveRooms};
use ambition_platformer2d::platformer::markers::ControlledSubject;
use ambition_platformer2d::vfx::{FireworksRequest, FxRequest, ParticleKind, VfxMessage, VfxWriter};
use ambition_platformer2d::world::rooms::{LiveRoomSpecs, RoomSpec};
use bevy::prelude::*;

use crate::flag::{FlagDrive, FlagPhase, FlagSequence};

/// The name of the block that is the switch of a castle.
pub const GOAL_SWITCH: &str = "goal_switch";
/// The name of the prop that draws the switch.
pub const SWITCH_ART: &str = "goal_switch_art";
/// The row of the sheet of the switch when it is down.
const PRESSED_ROW: &str = "pressed";
/// The side of the square the switch is drawn in, in world px. The base of the
/// square is on the floor, and the top of the button is the top of the block.
pub const SWITCH_ART_PX: f32 = 48.0;
/// How far the button goes down, in world px: the difference of its two rows.
const PRESS_DROP: f32 = 15.0;

/// The name of the block that is the gate of a castle.
pub const GOAL_GATE: &str = "goal_gate";
/// The name of the prop that draws the gate.
pub const GATE_ART: &str = "goal_gate_art";
/// The rows of the sheet of the gate, from down to up in the wall. The gate
/// that is down is the `idle` row: the engine draws no prop without one.
const GATE_ROWS: [&str; 6] = ["idle", "rise_1", "rise_2", "rise_3", "rise_4", "open"];

/// The cutscene in which the one she saved thanks her: a script of the pack's
/// `data/cutscenes/castle.ron`.
pub const RESCUE_CUTSCENE: &str = "mary_o_castle_rescue";
/// The character of the catalog that waits past the gate.
pub const RESCUED_CHARACTER_ID: &str = "mary_o_robot";


/// The beats of the end of a castle, in their sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CastleBeat {
    /// She is on the switch, and it goes down under her.
    Pressed,
    /// Charges go off along the bridge. Each plank is whole.
    Charges,
    /// The planks break one by one, from the switch.
    BridgeFalls,
    /// The boss falls, and the lava takes it.
    BossFalls,
    /// The boss is dead. Nothing moves.
    Aftermath,
    /// The gate goes up in the wall.
    GateOpens,
    /// She walks from the switch to the one she saved.
    Walks,
    /// He thanks her.
    Thanks,
    /// The level waits, so the player has the time of the victory.
    Victory,
}

impl CastleBeat {
    /// The seconds of a beat that ends on time. A beat with `None` ends on a
    /// fact of the world ([`CastleFacts`]).
    fn seconds(self) -> Option<f32> {
        let castle = crate::rules::mary_o_rules().castle_end;
        let beats = castle.beats;
        match self {
            Self::Pressed => Some(beats.pressed),
            Self::Charges => Some(beats.charges),
            Self::BridgeFalls | Self::BossFalls | Self::Walks => None,
            Self::Aftermath => Some(beats.aftermath),
            Self::GateOpens => Some(beats.gate_opens),
            Self::Thanks => Some(rescue_line_seconds() + beats.thanks_after_line),
            Self::Victory => Some(beats.victory),
        }
    }

    fn next(self) -> Option<Self> {
        Some(match self {
            Self::Pressed => Self::Charges,
            Self::Charges => Self::BridgeFalls,
            Self::BridgeFalls => Self::BossFalls,
            Self::BossFalls => Self::Aftermath,
            Self::Aftermath => Self::GateOpens,
            Self::GateOpens => Self::Walks,
            Self::Walks => Self::Thanks,
            Self::Thanks => Self::Victory,
            Self::Victory => return None,
        })
    }

    /// Which way she looks: at the bridge while it falls, and then at him.
    /// On the switch she looks the way she came to it.
    fn she_looks(self) -> Option<f32> {
        match self {
            Self::Pressed => None,
            beat if beat <= Self::Aftermath => Some(-1.0),
            _ => Some(1.0),
        }
    }
}


/// The switch of a castle, and where the end of the castle takes her.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CastleGoal {
    /// The box of the switch. Its top is the top of the button, up.
    pub switch: ae::Aabb,
    /// The x at which she stops, before the one she saved.
    pub stand_x: f32,
}

/// What a body does at the switch this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stomper {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub half: ae::Vec2,
    /// The body stands on a thing.
    pub on_ground: bool,
}

impl Stomper {
    /// Did this body land on `switch`? Its feet are on the top of the switch,
    /// and it does not go up. A body that is over the switch, or at its side,
    /// did not press it (Jon, 2026-10-12: "the button triggers if you are
    /// above it, it doesn't work as a button that you stomp on").
    pub fn landed_on(&self, switch: &ae::Aabb) -> bool {
        let feet = self.pos.y + self.half.y;
        self.on_ground
            && self.vel.y >= 0.0
            && (feet - switch.min.y).abs() <= 2.0
            && self.pos.x + self.half.x > switch.min.x
            && self.pos.x - self.half.x < switch.max.x
    }
}

/// The facts of the world that end a beat.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CastleFacts {
    /// A plank of the bridge is whole.
    pub planks_left: bool,
    /// A boss of the castle is alive.
    pub boss_left: bool,
}

/// The end of a castle, as a pure function of `(state, goal, body, facts, dt)`.
///
/// Returns where the body is this tick, as [`crate::flag::step_flag_sequence`]
/// does for a pole. `None` in `Idle`: the player still plays.
pub fn step_castle_ending(
    seq: &mut FlagSequence,
    goal: &CastleGoal,
    body: Stomper,
    facts: CastleFacts,
    dt: f32,
) -> Option<FlagDrive> {
    // The body the sequence drives is the one IT last put down.
    let at = seq.driven.unwrap_or(body.pos);
    let out = step_beat(seq, goal, Stomper { pos: at, ..body }, facts, dt);
    seq.driven = out.map(|drive| drive.pos);
    out
}

fn step_beat(
    seq: &mut FlagSequence,
    goal: &CastleGoal,
    body: Stomper,
    facts: CastleFacts,
    dt: f32,
) -> Option<FlagDrive> {
    let switch = goal.switch;
    let still = |pos: ae::Vec2| {
        Some(FlagDrive {
            pos,
            vel: ae::Vec2::ZERO,
            on_pole: false,
        })
    };
    // Her centre when her feet are on the button, `down` px under its top.
    let on_button = |down: f32| switch.min.y + down - body.half.y;
    let (score, beat, since) = match seq.phase {
        FlagPhase::Idle => {
            if !body.landed_on(&switch) {
                return None;
            }
            seq.phase = FlagPhase::Castle {
                score: crate::rules::mary_o_rules().castle_end.switch_score,
                beat: CastleBeat::Pressed,
                since: 0.0,
            };
            // On the button, and not on its edge: it goes down under her.
            let x = body.pos.x.clamp(switch.min.x + body.half.x * 0.5, switch.max.x - body.half.x * 0.5);
            return still(ae::Vec2::new(x, on_button(0.0)));
        }
        FlagPhase::Castle { score, beat, since } => (score, beat, since + dt),
        // A pole is not the goal of a castle, and after the tally she stays.
        _ => return still(body.pos),
    };

    let mut drive = FlagDrive {
        pos: body.pos,
        vel: ae::Vec2::ZERO,
        on_pole: false,
    };
    let done = match beat {
        CastleBeat::Pressed => {
            drive.pos.y = on_button((since * crate::rules::mary_o_rules().castle_end.press_speed).min(PRESS_DROP));
            false
        }
        CastleBeat::BridgeFalls => !facts.planks_left,
        CastleBeat::BossFalls => !facts.boss_left,
        CastleBeat::Walks => {
            let floor = switch.max.y - body.half.y;
            let x = (body.pos.x + crate::rules::mary_o_rules().flag.walk_off_speed * dt).min(goal.stand_x.max(body.pos.x));
            // Off the button when her middle is past its edge.
            let y = if x < switch.max.x {
                body.pos.y
            } else {
                (body.pos.y + crate::rules::mary_o_rules().castle_end.step_down_speed * dt).min(floor)
            };
            let arrived = x >= goal.stand_x && y >= floor;
            drive.pos = ae::Vec2::new(x, y);
            if !arrived {
                drive.vel = ae::Vec2::new(crate::rules::mary_o_rules().flag.walk_off_speed, 0.0);
            }
            arrived
        }
        _ => false,
    };
    let done = done || beat.seconds().is_some_and(|seconds| since >= seconds);
    seq.phase = match (done, beat.next()) {
        (false, _) => FlagPhase::Castle { score, beat, since },
        (true, Some(next)) => FlagPhase::Castle {
            score,
            beat: next,
            since: 0.0,
        },
        (true, None) => FlagPhase::Tallied { score },
    };
    Some(drive)
}

/// The box of the switch of a room, if the room ends on one.
pub fn room_switch(room: &RoomSpec) -> Option<ae::Aabb> {
    block_named(room, GOAL_SWITCH)
}

fn block_named(room: &RoomSpec, name: &str) -> Option<ae::Aabb> {
    room.world
        .blocks
        .iter()
        .find(|block| block.name == name)
        .map(|block| block.aabb)
}

/// The pack's cutscenes (`data/cutscenes/*.ron`).
pub fn cutscenes() -> &'static [CutsceneScript] {
    ambition_platformer2d::cutscene::content_schema::lowered_cutscenes(crate::pack::PACK.prepared())
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// The script of the rescue, as the pack states it.
pub fn rescue_script() -> &'static CutsceneScript {
    cutscenes()
        .iter()
        .find(|script| script.id == RESCUE_CUTSCENE)
        .unwrap_or_else(|| panic!("the pack's cutscenes have no `{RESCUE_CUTSCENE}`"))
}

/// What he says: the banner of the rescue.
pub fn rescue_line() -> &'static str {
    rescue_script()
        .beats
        .iter()
        .find_map(|beat| match beat {
            CutsceneBeat::Banner { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("`{RESCUE_CUTSCENE}` shows no banner, so he says nothing"))
}

/// How long his line is on the screen: the banners of the rescue.
fn rescue_line_seconds() -> f32 {
    rescue_script()
        .beats
        .iter()
        .map(|beat| match beat {
            CutsceneBeat::Banner { seconds, .. } => *seconds,
            _ => 0.0,
        })
        .sum()
}

/// Put the pack's cutscenes in the library of the app, one time, when the app
/// is built: a provider gives the library the rows of its own ids.
pub fn install(app: &mut App) {
    app.init_resource::<CutsceneLibrary>();
    let mut library = app.world_mut().resource_mut::<CutsceneLibrary>();
    for script in cutscenes() {
        library.insert(script.clone());
    }
    drop(library);
    // A build that reads the pack off disk plays an edit to a cutscene from
    // the next session.
    app.add_plugins(ambition_platformer2d::content::live_reload::EmbeddedPackReloadPlugin::cutscenes(&crate::pack::PACK));
    app.add_message::<FxRequest>();
    app.add_message::<FireworksRequest>();
}

/// The switch of the room the controlled body is in, with that body.
fn switch_of_her_room(
    rooms: Option<&LiveRoomSpecs>,
    subject: Option<&ControlledSubject>,
) -> Option<(Entity, ae::Aabb)> {
    let body = subject?.0?;
    let switch = room_switch(rooms?.spec_of(body)?)?;
    Some((body, switch))
}

/// The beat of the end of the castle of her room: her body, the switch, the
/// beat and the seconds in it.
fn beat_of_her_room(
    sequences: &Query<&FlagSequence>,
    rooms: Option<&LiveRoomSpecs>,
    subject: Option<&ControlledSubject>,
) -> Option<(Entity, ae::Aabb, CastleBeat, f32)> {
    let (beat, since) = sequences.single().ok()?.castle_beat()?;
    let (body, switch) = switch_of_her_room(rooms, subject)?;
    Some((body, switch, beat, since))
}

/// Drive the end of a castle, and her body with it.
///
/// It is to a switch what `flag::run_flag_sequence` is to a pole. From the
/// switch to the tally she is where the beat puts her, nothing hurts her, and
/// each boss of her room stops where it is: it throws nothing more, and it
/// falls with its plank.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn run_castle_ending(
    time: Res<ambition_platformer2d::time::WorldTime>,
    subject: Option<Res<ControlledSubject>>,
    rooms: Option<LiveRoomSpecs>,
    live: LiveRooms,
    broken: Option<Res<crate::bricks::BrokenBricks>>,
    geometry: LiveRoomOf<ae::RoomGeometry>,
    mut commands: Commands,
    mut sequences: Query<&mut FlagSequence>,
    mut bodies: Query<(
        Entity,
        &mut ae::BodyKinematics,
        Option<&mut ae::SweepSample>,
        Option<&ae::BodyGroundState>,
        Option<&mut ae::BodyModeState>,
        Option<&mut BodyHealth>,
        Option<&ActorConfig>,
        Option<&WornCharacter>,
    )>,
    mut holds: Query<&mut ControlHolds>,
) {
    let Some(her) = subject.as_deref().and_then(|subject| subject.0) else {
        return;
    };
    let Ok(mut sequence) = sequences.single_mut() else {
        return;
    };
    let Some((_, switch)) = switch_of_her_room(rooms.as_ref(), subject.as_deref()) else {
        return;
    };
    let room = live.of(her);

    // The others of her room: each boss, and the one she saved.
    let mut boss_left = false;
    let mut he_is_at = None;
    for (entity, kin, _, _, _, health, config, worn) in &bodies {
        if entity == her || live.of(entity) != room {
            continue;
        }
        if worn.is_some_and(|worn| worn.id() == RESCUED_CHARACTER_ID) {
            he_is_at = Some(kin.pos.x);
        }
        let is_boss = config.is_some_and(|config| crate::void_lizard::is_void_lizard_brain(&config.brain));
        if !is_boss || !health.is_some_and(|health| health.alive()) {
            continue;
        }
        boss_left = true;
        // It is dead before the sequence ends, so nothing releases this.
        if sequence.active() {
            claim_control_hold(&mut commands, entity, ControlHold::Sequence);
        }
    }
    let planks_left = match (broken.as_deref(), geometry.room_of(her), geometry.of(her)) {
        (Some(broken), Some(room), Some(room_geometry)) => {
            crate::bricks::planks_left(broken, room, room_geometry).next().is_some()
        }
        _ => false,
    };

    crate::flag::hold_her_while_the_sequence_runs(
        &mut commands,
        her,
        &sequence,
        holds.get_mut(her).ok().as_deref_mut(),
    );
    let Ok((_, mut kin, mut sweep, ground, mode, health, ..)) = bodies.get_mut(her) else {
        return;
    };
    let goal = CastleGoal {
        switch,
        // With nobody past the gate, she walks off as she does from a pole.
        stand_x: he_is_at.map_or(switch.max.x + crate::rules::mary_o_rules().flag.walk_off_px, |x| x - crate::rules::mary_o_rules().castle_end.stand_before_him),
    };
    let stomper = Stomper {
        pos: kin.pos,
        vel: kin.vel,
        half: kin.size * 0.5,
        on_ground: ground.is_some_and(|ground| ground.on_ground),
    };
    let facts = CastleFacts { planks_left, boss_left };
    let in_the_beats = sequence.castle_beat().is_some();
    let drive = step_castle_ending(&mut sequence, &goal, stomper, facts, time.sim_dt());
    // Nothing hurts her from the switch to the tally. The claim is taken on
    // the tick the beats start and released on the tick they end: another
    // owner of the same reason keeps its claim in each other tick.
    let now = sequence.castle_beat().is_some();
    if let Some(mut health) = health.filter(|_| now != in_the_beats) {
        health.health.invulnerable.set(Invulnerability::SCRIPTED, now);
    }
    let Some(drive) = drive else {
        return;
    };
    crate::flag::drive_her(&drive, &mut kin, sweep.as_deref_mut(), mode);
    let looks = sequence.castle_beat().and_then(|(beat, _)| beat.she_looks());
    if let Some(side) = looks.filter(|side| drive.vel.x == 0.0 && kin.facing != *side) {
        kin.facing = side;
    }
}

/// Charges go off along the bridge before it breaks.
///
/// On the first tick of [`CastleBeat::Charges`], one show of explosions is
/// asked for, along the planks of her room and a small distance over them.
/// The presentation spreads them over the beat.
pub fn charges_go_off_on_the_bridge(
    sequences: Query<&FlagSequence>,
    subject: Option<Res<ControlledSubject>>,
    rooms: Option<LiveRoomSpecs>,
    broken: Option<Res<crate::bricks::BrokenBricks>>,
    geometry: LiveRoomOf<ae::RoomGeometry>,
    mut fireworks: MessageWriter<FireworksRequest>,
) {
    let Some((body, _, CastleBeat::Charges, since)) =
        beat_of_her_room(&sequences, rooms.as_ref(), subject.as_deref())
    else {
        return;
    };
    if since != 0.0 {
        return;
    }
    let (Some(broken), Some(room), Some(room_geometry)) = (broken.as_deref(), geometry.room_of(body), geometry.of(body))
    else {
        return;
    };
    // The bridge: from its first plank to its last, and the top of its deck.
    let Some((left, right, deck)) = crate::bricks::planks_left(broken, room, room_geometry)
        .map(|plank| (plank.aabb.min.x, plank.aabb.max.x, plank.aabb.min.y))
        .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1), a.2.min(b.2)))
    else {
        return;
    };
    let mut show = FireworksRequest::around(Some(room), ae::Vec2::new((left + right) * 0.5, deck));
    show.count = 9;
    show.spread = ae::Vec2::new((right - left) * 0.85, 34.0);
    show.duration = CastleBeat::Charges.seconds().unwrap_or(1.0);
    fireworks.write(show);
}

/// The lava takes the boss.
///
/// After the switch is taken, a Void Lizard of her room that is in the lava
/// goes down at a fixed speed, with a splash each few ticks, and dies when it
/// is under: the lava is drawn over a body, so it is seen to go in, and its
/// death is one large splash. With no lava under it, it dies a few tiles
/// under the floor. One that did not fall dies `castle_end.boss_fall_limit_seconds` after its
/// bridge is gone. So the switch is a sure end of the boss, also when a post
/// held it up.
#[allow(clippy::type_complexity)]
pub fn the_lava_takes_the_boss(
    sequences: Query<&FlagSequence>,
    subject: Option<Res<ControlledSubject>>,
    rooms: Option<LiveRoomSpecs>,
    live: LiveRooms,
    tick: Option<Res<ambition_platformer2d::time::SimTick>>,
    mut vfx: VfxWriter,
    mut explosions: MessageWriter<FxRequest>,
    mut bosses: Query<(Entity, &ActorConfig, &mut BodyHealth, &mut ae::BodyKinematics)>,
) {
    let Some((body, switch, beat, since)) = beat_of_her_room(&sequences, rooms.as_ref(), subject.as_deref()) else {
        return;
    };
    let pools = rooms
        .as_ref()
        .and_then(|rooms| rooms.spec_of(body))
        .map(crate::scenery::lava_pools_of)
        .unwrap_or_default();
    let out_of_time = beat > CastleBeat::BossFalls || (beat == CastleBeat::BossFalls && since > crate::rules::mary_o_rules().castle_end.boss_fall_limit_seconds);
    let room = live.of(body);
    // The base of the switch is the line of the floor.
    let floor = switch.max.y;
    for (entity, config, mut health, mut kin) in &mut bosses {
        if !crate::void_lizard::is_void_lizard_brain(&config.brain) || !health.alive() {
            continue;
        }
        if live.of(entity) != room {
            continue;
        }
        let (top, feet) = (kin.pos.y - kin.size.y * 0.5, kin.pos.y + kin.size.y * 0.5);
        let lava = pools
            .iter()
            .map(|(_, pool)| *pool)
            .find(|pool| kin.pos.x >= pool.min.x && kin.pos.x <= pool.max.x);
        let dies_at = match lava {
            _ if out_of_time => Some(kin.pos),
            Some(pool) if feet >= pool.min.y => {
                // In the lava: down at a fixed speed, and no way to the side.
                // The lava holds the speed; the step moves the body.
                let at = kin.pos;
                ae::movement::constrain_body_pose(&mut kin, None, at, ae::Vec2::new(0.0, crate::rules::mary_o_rules().castle_end.sink_speed));
                if tick.as_ref().is_some_and(|tick| tick.get() % crate::rules::mary_o_rules().castle_end.splash_ticks == 0) {
                    vfx.for_room(room).write(VfxMessage::Burst {
                        pos: ae::Vec2::new(kin.pos.x, pool.min.y),
                        count: 10,
                        speed: 190.0,
                        color: [1.0, 0.52, 0.14, 1.0],
                        kind: ParticleKind::Shard,
                    });
                }
                (top > pool.min.y + crate::rules::mary_o_rules().castle_end.sunk_under).then_some(ae::Vec2::new(kin.pos.x, pool.min.y))
            }
            None if top > floor + crate::rules::mary_o_rules().castle_end.fallen_under => Some(kin.pos),
            _ => None,
        };
        let Some(at) = dies_at else {
            continue;
        };
        health.health.current = 0;
        // Its death is seen: one explosion where it went, and the lava it throws.
        explosions.write(
            FxRequest::classic(room, at)
                .with_scale(1.6)
                .from_source(crate::provider::MARY_O_EXPERIENCE.into()),
        );
        vfx.for_room(room).write(VfxMessage::Burst {
            pos: at,
            count: 30,
            speed: 300.0,
            color: [1.0, 0.6, 0.18, 1.0],
            kind: ParticleKind::Shard,
        });
    }
}

/// The gate opens when the boss is dead.
///
/// The art of the gate goes up through its rows in [`CastleBeat::GateOpens`]
/// (`dress_the_goal`). When it is up, its block is marked broken: the same
/// mark takes a broken brick out of the room, and it is whole again in the
/// next attempt. So the gate is a wall until the end of the castle opens it.
pub fn the_gate_opens(
    sequences: Query<&FlagSequence>,
    subject: Option<Res<ControlledSubject>>,
    rooms: Option<LiveRoomSpecs>,
    geometry: LiveRoomOf<ae::RoomGeometry>,
    mut broken: ResMut<crate::bricks::BrokenBricks>,
    mut vfx: VfxWriter,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
) {
    let Ok(sequence) = sequences.single() else {
        return;
    };
    let Some((body, _)) = switch_of_her_room(rooms.as_ref(), subject.as_deref()) else {
        return;
    };
    let Some(gate) = rooms
        .as_ref()
        .and_then(|rooms| rooms.spec_of(body))
        .and_then(|room| block_named(room, GOAL_GATE))
    else {
        return;
    };
    let Some(room) = geometry.room_of(body) else {
        return;
    };
    let base = ae::Vec2::new((gate.min.x + gate.max.x) * 0.5, gate.max.y);
    let mut clank = |at: ae::Vec2| {
        sfx.write_from(
            crate::provider::MARY_O_EXPERIENCE,
            ambition_platformer2d::sfx::SfxMessage::Hit { pos: at },
        );
    };
    if sequence.castle_beat() == Some((CastleBeat::GateOpens, 0.0)) {
        // It comes off the floor.
        clank(base);
        vfx.for_room(Some(room)).write(VfxMessage::Dust { pos: base, facing: -1.0 });
        vfx.for_room(Some(room)).write(VfxMessage::Dust { pos: base, facing: 1.0 });
    }
    if gate_row(sequence) == GATE_ROWS[GATE_ROWS.len() - 1] && broken.mark(room, GOAL_GATE) {
        clank(ae::Vec2::new(base.x, gate.min.y));
    }
}

/// The row of the sheet of the gate for the state of the sequence.
fn gate_row(sequence: &FlagSequence) -> &'static str {
    let last = GATE_ROWS.len() - 1;
    let row = match sequence.phase {
        FlagPhase::Castle { beat, .. } if beat < CastleBeat::GateOpens => 0,
        FlagPhase::Castle {
            beat: CastleBeat::GateOpens,
            since,
        .. } => (1 + (since / crate::rules::mary_o_rules().castle_end.gate_step_seconds) as usize).min(last),
        FlagPhase::Castle { .. } | FlagPhase::Tallied { .. } => last,
        _ => 0,
    };
    GATE_ROWS[row]
}

/// The one she saved thanks her.
///
/// On the first tick of [`CastleBeat::Thanks`], his cutscene is asked for. The
/// beat is as long as his line, and the level does not wait for a button.
pub fn the_robot_thanks_her(
    sequences: Query<&FlagSequence>,
    subject: Option<Res<ControlledSubject>>,
    rooms: Option<LiveRoomSpecs>,
    queue: Option<ResMut<CutsceneTriggerQueue>>,
) {
    // A world with no cutscenes (a test of one rule) ends its castle with no line.
    let Some(mut queue) = queue else {
        return;
    };
    if let Some((_, _, CastleBeat::Thanks, since)) = beat_of_her_room(&sequences, rooms.as_ref(), subject.as_deref()) {
        if since == 0.0 {
            queue.request(RESCUE_CUTSCENE);
        }
    }
}

/// The one she saved looks at her.
///
/// He stands and does not turn of his own will, so his side is written here,
/// each tick, from where she is.
pub fn he_looks_at_her(
    subject: Option<Res<ControlledSubject>>,
    live: LiveRooms,
    mut bodies: Query<(Entity, &mut ae::BodyKinematics, Option<&WornCharacter>)>,
) {
    let Some(her) = subject.as_deref().and_then(|subject| subject.0) else {
        return;
    };
    let Ok((_, her_kin, _)) = bodies.get(her) else {
        return;
    };
    let (her_x, room) = (her_kin.pos.x, live.of(her));
    for (entity, mut kin, worn) in &mut bodies {
        if !worn.is_some_and(|worn| worn.id() == RESCUED_CHARACTER_ID) || live.of(entity) != room {
            continue;
        }
        let side = if her_x < kin.pos.x { -1.0 } else { 1.0 };
        if kin.facing != side {
            kin.facing = side;
        }
    }
}

/// The switch shows that it is down, and the gate shows how far it is up.
///
/// Derived from the sequence each frame, so a rewind to before the switch puts
/// the button up and the gate down again.
pub fn dress_the_goal(
    mut commands: Commands,
    sequences: Query<&FlagSequence>,
    props: Query<(
        Entity,
        &ambition_platformer2d::render::rendering::PropVisual,
        Option<&ambition_platformer2d::render::rendering::PropClip>,
    )>,
) {
    use ambition_platformer2d::render::rendering::PropClip;
    let sequence = sequences.single().ok();
    for (entity, prop, clip) in &props {
        let row = match prop.name.as_str() {
            SWITCH_ART => sequence.is_some_and(|sequence| sequence.active()).then_some(PRESSED_ROW),
            GATE_ART => Some(sequence.map_or(GATE_ROWS[0], gate_row)),
            _ => continue,
        };
        let wanted = row.map(|row| {
            PropClip(ambition_platformer2d::sim_view::ClipRequest::from_chain(&[row]).expect("one row"))
        });
        match wanted {
            Some(wanted) if clip != Some(&wanted) => {
                commands.entity(entity).insert(wanted);
            }
            None if clip.is_some() => {
                commands.entity(entity).remove::<PropClip>();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;
    const HALF: ae::Vec2 = ae::Vec2::new(12.0, 16.0);

    fn goal() -> CastleGoal {
        CastleGoal {
            switch: ae::Aabb::new(ae::Vec2::new(118.0, 378.0), ae::Vec2::new(18.0, 22.0)),
            stand_x: 260.0,
        }
    }

    fn on_the_switch(goal: &CastleGoal) -> Stomper {
        Stomper {
            pos: ae::Vec2::new(118.0, goal.switch.min.y - HALF.y),
            vel: ae::Vec2::ZERO,
            half: HALF,
            on_ground: true,
        }
    }

    /// Jon, 2026-10-12: "the button triggers if you are above it, it doesn't
    /// work as a button that you stomp on, which it should."
    #[test]
    fn only_a_body_that_lands_on_the_switch_presses_it() {
        let goal = goal();
        let landed = on_the_switch(&goal);
        let facts = CastleFacts::default();
        for (what, body) in [
            ("over it, in the air", Stomper { pos: landed.pos - ae::Vec2::new(0.0, 60.0), on_ground: false, ..landed }),
            ("on a thing high over it", Stomper { pos: landed.pos - ae::Vec2::new(0.0, 96.0), ..landed }),
            ("on the floor at its side", Stomper { pos: ae::Vec2::new(80.0, goal.switch.max.y - HALF.y), ..landed }),
            ("up through its top", Stomper { vel: ae::Vec2::new(0.0, -200.0), ..landed }),
        ] {
            let mut seq = FlagSequence::default();
            assert_eq!(step_castle_ending(&mut seq, &goal, body, facts, DT), None, "a body {what} pressed the switch");
            assert_eq!(seq.phase, FlagPhase::Idle, "a body {what} pressed the switch");
        }
        let mut seq = FlagSequence::default();
        assert!(step_castle_ending(&mut seq, &goal, landed, facts, DT).is_some());
        assert_eq!(seq.castle_beat(), Some((CastleBeat::Pressed, 0.0)));
    }

    /// Jon, 2026-10-12: "There should be a sequence of beats to this so it
    /// doesn't happen in a blink of an eye." Each beat comes in its sequence,
    /// the beats that wait for the world wait for it, and she ends before him,
    /// on the floor.
    #[test]
    fn the_beats_come_in_their_sequence_and_each_takes_its_time() {
        let goal = goal();
        let body = on_the_switch(&goal);
        let mut seq = FlagSequence::default();
        let mut seen = vec![];
        let mut ticks_in = std::collections::BTreeMap::<CastleBeat, u32>::new();
        let mut at = body.pos;
        for tick in 0..3000 {
            // The bridge takes one second to fall, and the boss one more.
            let facts = CastleFacts { planks_left: tick < 200, boss_left: tick < 320 };
            let drive = step_castle_ending(&mut seq, &goal, body, facts, DT).expect("the sequence drives her");
            at = drive.pos;
            let Some((beat, _)) = seq.castle_beat() else {
                break;
            };
            if seen.last() != Some(&beat) {
                seen.push(beat);
            }
            *ticks_in.entry(beat).or_default() += 1;
            if beat <= CastleBeat::BridgeFalls {
                assert!(facts.planks_left, "{beat:?} is the beat and the bridge is gone");
            }
            if beat >= CastleBeat::Aftermath {
                assert!(!facts.boss_left, "{beat:?} is the beat and the boss is alive");
            }
        }
        use CastleBeat::*;
        assert_eq!(seen, [Pressed, Charges, BridgeFalls, BossFalls, Aftermath, GateOpens, Walks, Thanks, Victory]);
        assert!(matches!(seq.phase, FlagPhase::Tallied { .. }), "the end of the castle has no end: {:?}", seq.phase);
        for (beat, ticks) in &ticks_in {
            assert!(*ticks >= 30, "{beat:?} was {ticks} ticks: the player does not see it");
        }
        assert_eq!(at, ae::Vec2::new(goal.stand_x, goal.switch.max.y - HALF.y), "she is not before him, on the floor");
    }

    /// The gate is a wall until the boss is dead, and it does not close again.
    #[test]
    fn the_gate_is_down_until_its_beat_and_up_after_it() {
        let phase = |beat, since| FlagSequence {
            phase: FlagPhase::Castle { score: 0, beat, since },
            driven: None,
        };
        assert_eq!(gate_row(&FlagSequence::default()), "idle");
        assert_eq!(gate_row(&phase(CastleBeat::Aftermath, 0.9)), "idle");
        assert_eq!(gate_row(&phase(CastleBeat::GateOpens, 0.0)), "rise_1");
        assert_eq!(gate_row(&phase(CastleBeat::GateOpens, 0.9)), "open");
        assert_eq!(gate_row(&phase(CastleBeat::Walks, 0.0)), "open");
        let tallied = FlagSequence { phase: FlagPhase::Tallied { score: 0 }, driven: None };
        assert_eq!(gate_row(&tallied), "open");
    }
}
