//! A PASSAGE through a door or a dimension gate: the short scene a body plays
//! going in by one, and coming out of the one it arrives at (Jon, 2026-10-19:
//! "a short sequence for entering and exiting doors. You should open the door
//! walk through it"; at a gate, "the player faces the wormhole and jumps into
//! it", with "a ripple in the gate showing they moved through it").
//!
//! - A DOOR opens, and the body turns its back to you and walks into it,
//!   smaller and darker as it goes. On the far side it walks out of the open
//!   door, and the door shuts behind it.
//! - A GATE: the body turns its back to you, crouches and leaps into the
//!   middle of the gate, smaller as it goes into depth, and the gate swallows
//!   it with a ripple. On the far side the gate swells and throws it out
//!   toward you, and it lands in front of it.
//!
//! The scene is a [`ZonePassage`] on the body, rollback state. It holds the
//! body's control and pose while it plays, and places the body each tick
//! through the engine's transit authority. The crossing is recorded when the
//! going-in half ends; the body keeps its passage across the room change, and
//! its arrival in the far room turns it into the coming-out half. What the
//! body, the door and the gate look like at each moment is a pure function of
//! the passage ([`ZonePassage::look`], [`ZonePassage::door_open`],
//! [`ZonePassage::gate_ripple`]), which presentation reads.
//!
//! A game turns passages on for its own rooms by declaring
//! [`ZonePassageRules`] (`declare_rules`); in a room without them a door is
//! the instant crossing it always was.

use bevy::prelude::*;

use ambition_characters::control::{claim_control_hold, release_control_hold, ControlHold, ControlHolds};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};

use crate::session::lifecycle_commit::{LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent};

/// The rule that a room's doors and gates play their passage. A game declares
/// it for its own rooms' scope, so another game's rooms in the same host keep
/// the instant crossing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ZonePassageRules;

/// What a body passes through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassageKind {
    Door,
    Gate,
}

/// Which half of the passage is playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassageStage {
    /// Going in, in the room left. Its end records the crossing.
    Entering,
    /// Coming out, in the room arrived at.
    Exiting,
}

/// The zone a passage comes out of, in the room it arrives at.
#[derive(Clone, Debug, PartialEq)]
pub struct PassageArrival {
    /// What it comes out of: a gate's passage can come out of a door. `None`:
    /// a zone that is neither, which it comes out of with no passage.
    pub kind: Option<PassageKind>,
    pub zone: String,
    pub foot: ae::Vec2,
    pub heart: ae::Vec2,
}

// ---- Timing (s) -----------------------------------------------------------

/// The door swings open while the body steps to the middle of the doorway.
pub const DOOR_OPEN_S: f32 = 0.24;
/// It walks into the doorway, out of sight.
pub const DOOR_IN_S: f32 = 0.46;
/// On the far side, it walks out of the open door...
pub const DOOR_OUT_S: f32 = 0.42;
/// ...and the door shuts behind it, while the body is already free.
pub const DOOR_SHUT_S: f32 = 0.26;
/// The body turns its back to the gate...
pub const GATE_TURN_S: f32 = 0.12;
/// ...crouches...
pub const GATE_CROUCH_S: f32 = 0.14;
/// ...and leaps into its middle.
pub const GATE_LEAP_S: f32 = 0.42;
/// The surface closes over it before the crossing.
pub const GATE_SINK_S: f32 = 0.24;
/// The gate's ripple rows (`swallow`, `emit`): 12 frames of 55 ms.
pub const GATE_RIPPLE_S: f32 = 12.0 * 0.055;
/// The swallow starts when the body meets the surface: this far through the leap.
const GATE_CONTACT: f32 = 0.7;
/// The far gate swells before the body comes out of it.
pub const GATE_EMERGE_DELAY_S: f32 = 0.06;
/// The arc out of the gate and down to the floor in front of it.
pub const GATE_ARC_S: f32 = 0.44;
/// A body that was let through and has not arrived anywhere in this long is
/// given back to its player where it stands: the crossing did not happen.
pub const CROSSING_GIVE_UP_S: f32 = 4.0;

/// How far out of the doorway the body walks, and how far in front of a gate
/// it lands (px).
const DOOR_STEP_OUT: f32 = 28.0;
const GATE_LANDING: f32 = 34.0;
/// How high the leap and the arc rise over their straight line (px).
const GATE_LEAP_RISE: f32 = 30.0;
const GATE_ARC_RISE: f32 = 26.0;

/// A body passing through a door or a gate. See the module.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ZonePassage {
    pub kind: PassageKind,
    pub stage: PassageStage,
    /// Seconds into this half.
    pub elapsed: f32,
    /// Where the body stood when this half began (its centre).
    pub from: ae::Vec2,
    /// The zone it passes in this half.
    pub zone: String,
    /// The floor under the middle of the zone.
    pub foot: ae::Vec2,
    /// The middle of the zone: of a gate, its surface.
    pub heart: ae::Vec2,
    /// Half the body's height, to stand it on the floor.
    pub half_h: f32,
    /// The way it comes out (+1 right, -1 left).
    pub facing: f32,
    /// Going in: the crossing it records when the going-in ends.
    pub crossing: Option<RoomTransitionIntent>,
    /// Going in: the crossing was recorded, and the body waits for its room.
    pub recorded: bool,
    /// Going in: the seconds into this half at which the crossing was
    /// admitted. The wait for the crossing ([`CROSSING_GIVE_UP_S`]) starts
    /// here. A wait for the lifecycle slot, while another operation holds it,
    /// is not part of it.
    pub recorded_at: f32,
    /// Going in: the zone it comes out of.
    pub arrival: Option<PassageArrival>,
    /// The live room this half plays in. `None` until its first tick.
    pub room: Option<LiveRoomInstance>,
}

/// What the body looks like this moment of its passage. Presentation draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PassageLook {
    pub pose: PassagePose,
    /// Its drawn size, about its feet (1 = as it is).
    pub scale: f32,
    /// How much of it shows (0 = none).
    pub alpha: f32,
    /// How far into shadow it is (0 = lit, 1 = black).
    pub shade: f32,
}

/// The pose a passage draws the body in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassagePose {
    /// Turning its back to you (`turn_away`, 4 frames of 60 ms).
    TurnAway,
    /// Its back to you, walking away (into a doorway).
    WalkAway,
    /// Its back to you, standing, crouching or leaping (into a gate).
    BackTo,
    Walk,
    Jump,
    Fall,
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl ZonePassage {
    /// A value-complete desync projection of the passage: its scene, its
    /// clock, the crossing it records and where it comes out.
    ///
    /// ⛔ EXHAUSTIVE BY CONSTRUCTION, as `LifecycleIntent::checksum` is: every
    /// field is destructured by name, so a new one stops this compiling
    /// instead of falling outside the hash. Two peers whose passages record
    /// different crossings, or come out of different zones, must disagree.
    pub fn checksum(&self) -> u64 {
        use ae::snapshot::{checksum_bytes, put_bool, put_f32, put_str, put_u32, put_u64, put_u8, put_vec2};
        let Self {
            kind,
            stage,
            elapsed,
            from,
            zone,
            foot,
            heart,
            half_h,
            facing,
            crossing,
            recorded,
            recorded_at,
            arrival,
            room,
        } = self;
        let kind_code = |kind: PassageKind| match kind {
            PassageKind::Door => 1u8,
            PassageKind::Gate => 2,
        };
        let mut bytes = Vec::new();
        put_u8(&mut bytes, kind_code(*kind));
        put_u8(&mut bytes, match stage {
            PassageStage::Entering => 0,
            PassageStage::Exiting => 1,
        });
        put_f32(&mut bytes, *elapsed);
        put_vec2(&mut bytes, *from);
        put_str(&mut bytes, zone);
        put_vec2(&mut bytes, *foot);
        put_vec2(&mut bytes, *heart);
        put_f32(&mut bytes, *half_h);
        put_f32(&mut bytes, *facing);
        put_bool(&mut bytes, crossing.is_some());
        put_u64(
            &mut bytes,
            crossing.as_ref().map_or(0, |crossing| LifecycleIntent::Transition(crossing.clone()).checksum()),
        );
        put_bool(&mut bytes, *recorded);
        put_f32(&mut bytes, *recorded_at);
        put_bool(&mut bytes, arrival.is_some());
        if let Some(PassageArrival { kind, zone, foot, heart }) = arrival {
            put_u8(&mut bytes, kind.map_or(0, kind_code));
            put_str(&mut bytes, zone);
            put_vec2(&mut bytes, *foot);
            put_vec2(&mut bytes, *heart);
        }
        put_bool(&mut bytes, room.is_some());
        put_u32(&mut bytes, room.map_or(0, |room| room.ordinal()));
        checksum_bytes(&bytes)
    }

    /// The going-in half, at `zone` (its box `aabb`), for a body at `at` of
    /// height `2 * half_h`.
    #[allow(clippy::too_many_arguments)]
    pub fn enter(
        kind: PassageKind,
        zone: &str,
        aabb: ae::Aabb,
        at: ae::Vec2,
        half_h: f32,
        facing: f32,
        crossing: RoomTransitionIntent,
        arrival: PassageArrival,
    ) -> Self {
        use ae::AabbExt as _;
        Self {
            kind,
            stage: PassageStage::Entering,
            elapsed: 0.0,
            from: at,
            zone: zone.to_string(),
            foot: aabb.center() + ae::Vec2::new(0.0, aabb.half_size().y),
            heart: aabb.center(),
            half_h,
            facing: if facing < 0.0 { -1.0 } else { 1.0 },
            crossing: Some(crossing),
            recorded: false,
            recorded_at: 0.0,
            arrival: Some(arrival),
            room: None,
        }
    }

    /// The coming-out half, at the zone it arrived at, for a body facing
    /// `facing`.
    fn exiting(&self, arrival: &PassageArrival, kind: PassageKind, facing: f32, room: Option<LiveRoomInstance>) -> Self {
        Self {
            kind,
            stage: PassageStage::Exiting,
            elapsed: 0.0,
            from: ae::Vec2::new(arrival.foot.x, arrival.foot.y - self.half_h),
            zone: arrival.zone.clone(),
            foot: arrival.foot,
            heart: arrival.heart,
            half_h: self.half_h,
            facing: if facing < 0.0 { -1.0 } else { 1.0 },
            crossing: None,
            recorded: false,
            recorded_at: 0.0,
            arrival: None,
            room,
        }
    }

    /// The body arrived in the room its crossing went to (the room
    /// transition's commit says so): the coming-out half begins, at the zone
    /// it comes out of. A zone that is neither a door nor a gate ends the
    /// passage on its next tick.
    ///
    /// `placed` is where the commit put the body. The body stands there, not
    /// on the far zone's foot: a door at a room's edge reaches into the wall,
    /// and its foot is inside it, where the commit's arrival point is the one
    /// it resolved clear of the geometry.
    pub fn arrived(&mut self, facing: f32, placed: Option<ae::Vec2>) {
        let Some(mut arrival) = self.arrival.clone().filter(|_| self.stage == PassageStage::Entering && self.recorded) else {
            return;
        };
        if let Some(placed) = placed {
            arrival.foot = placed + ae::Vec2::new(0.0, self.half_h);
        }
        match arrival.kind {
            Some(kind) => *self = self.exiting(&arrival, kind, facing, None),
            None => {
                *self = self.exiting(&arrival, self.kind, facing, None);
                self.elapsed = Self::exiting_s(self.kind);
            }
        }
    }

    /// How long the going-in half plays before it records the crossing.
    pub fn entering_s(kind: PassageKind) -> f32 {
        match kind {
            PassageKind::Door => DOOR_OPEN_S + DOOR_IN_S,
            PassageKind::Gate => GATE_TURN_S + GATE_CROUCH_S + GATE_LEAP_S + GATE_SINK_S,
        }
    }

    /// How long the coming-out half holds the body.
    pub fn exiting_body_s(kind: PassageKind) -> f32 {
        match kind {
            PassageKind::Door => DOOR_OUT_S,
            PassageKind::Gate => GATE_EMERGE_DELAY_S + GATE_ARC_S,
        }
    }

    /// How long the coming-out half plays, the door or the gate after the
    /// body is free included.
    pub fn exiting_s(kind: PassageKind) -> f32 {
        match kind {
            PassageKind::Door => DOOR_OUT_S + DOOR_SHUT_S,
            PassageKind::Gate => GATE_RIPPLE_S.max(GATE_EMERGE_DELAY_S + GATE_ARC_S),
        }
    }

    /// The body still belongs to the passage: its control and pose are held.
    pub fn holds_body(&self) -> bool {
        match self.stage {
            PassageStage::Entering => true,
            PassageStage::Exiting => self.elapsed < Self::exiting_body_s(self.kind),
        }
    }

    fn standing(&self) -> ae::Vec2 {
        ae::Vec2::new(self.foot.x, self.foot.y - self.half_h)
    }

    /// Where the body is this tick, or `None` once it is free.
    pub fn body_at(&self) -> Option<ae::Vec2> {
        if !self.holds_body() {
            return None;
        }
        let t = self.elapsed;
        let stand = self.standing();
        Some(match (self.kind, self.stage) {
            (PassageKind::Door, PassageStage::Entering) => {
                // To the middle of the doorway while the door opens, then
                // there: the walk in is into depth. The whole position moves:
                // a body that came in off the stand line (on a slope, or in a
                // gravity whose run is along y) is not snapped onto it.
                self.from.lerp(stand, smooth(t / DOOR_OPEN_S))
            }
            (PassageKind::Door, PassageStage::Exiting) => {
                let k = smooth(t / DOOR_OUT_S);
                ae::Vec2::new(stand.x + self.facing * DOOR_STEP_OUT * k, stand.y)
            }
            (PassageKind::Gate, PassageStage::Entering) => {
                let set = GATE_TURN_S + GATE_CROUCH_S;
                if t < set {
                    self.from.lerp(stand, smooth(t / set))
                } else {
                    let k = ((t - set) / GATE_LEAP_S).clamp(0.0, 1.0);
                    let into = self.heart + ae::Vec2::new(0.0, 10.0);
                    let e = smooth(k);
                    stand.lerp(into, e) - ae::Vec2::new(0.0, GATE_LEAP_RISE * (k * std::f32::consts::PI).sin())
                }
            }
            (PassageKind::Gate, PassageStage::Exiting) => {
                let out = self.heart + ae::Vec2::new(0.0, 8.0);
                if t < GATE_EMERGE_DELAY_S {
                    out
                } else {
                    let k = ((t - GATE_EMERGE_DELAY_S) / GATE_ARC_S).clamp(0.0, 1.0);
                    let land = ae::Vec2::new(stand.x + self.facing * GATE_LANDING, stand.y);
                    ae::Vec2::new(
                        out.x + (land.x - out.x) * smooth(k),
                        out.y + (land.y - out.y) * k * k - GATE_ARC_RISE * (k * std::f32::consts::PI).sin(),
                    )
                }
            }
        })
    }

    /// What the body looks like this moment, or `None` once it is free.
    pub fn look(&self) -> Option<PassageLook> {
        if !self.holds_body() {
            return None;
        }
        let t = self.elapsed;
        let look = |pose, scale, alpha, shade| PassageLook { pose, scale, alpha, shade };
        Some(match (self.kind, self.stage) {
            (_, PassageStage::Entering) if self.recorded => look(PassagePose::BackTo, 0.2, 0.0, 1.0),
            (PassageKind::Door, PassageStage::Entering) => {
                if t < DOOR_OPEN_S {
                    look(PassagePose::TurnAway, 1.0, 1.0, 0.0)
                } else {
                    // Into the dark of the doorway: smaller, darker, gone.
                    let k = ((t - DOOR_OPEN_S) / DOOR_IN_S).clamp(0.0, 1.0);
                    let gone = ((k - 0.55) / 0.45).clamp(0.0, 1.0);
                    look(PassagePose::WalkAway, 1.0 - 0.2 * smooth(k), 1.0 - smooth(gone), 0.85 * smooth(k))
                }
            }
            (PassageKind::Door, PassageStage::Exiting) => {
                // Out of the dark of the doorway: it grows, lightens, shows.
                let k = (t / DOOR_OUT_S).clamp(0.0, 1.0);
                look(PassagePose::Walk, 0.8 + 0.2 * smooth(k), smooth(k / 0.45), 0.85 * (1.0 - smooth(k)))
            }
            (PassageKind::Gate, PassageStage::Entering) => {
                let set = GATE_TURN_S + GATE_CROUCH_S;
                if t < set {
                    let pose = if t < 0.24 { PassagePose::TurnAway } else { PassagePose::BackTo };
                    look(pose, 1.0, 1.0, 0.0)
                } else {
                    // Smaller with depth; through the surface, gone.
                    let k = ((t - set) / GATE_LEAP_S).clamp(0.0, 1.0);
                    let gone = ((k - 0.75) / 0.25).clamp(0.0, 1.0);
                    look(PassagePose::BackTo, 1.0 - 0.85 * smooth(k).powf(1.4), 1.0 - gone, 0.0)
                }
            }
            (PassageKind::Gate, PassageStage::Exiting) => {
                if t < GATE_EMERGE_DELAY_S {
                    look(PassagePose::Jump, 0.2, 0.0, 0.0)
                } else {
                    let k = ((t - GATE_EMERGE_DELAY_S) / GATE_ARC_S).clamp(0.0, 1.0);
                    let pose = if k < 0.4 { PassagePose::Jump } else { PassagePose::Fall };
                    look(pose, 0.2 + 0.8 * smooth(k * 1.3), (0.3 + k * 3.0).min(1.0), 0.0)
                }
            }
        })
    }

    /// How open the door of this passage is (0 shut, 1 open), and its zone.
    pub fn door_open(&self) -> Option<(&str, f32)> {
        if self.kind != PassageKind::Door {
            return None;
        }
        let open = match self.stage {
            PassageStage::Entering if self.recorded => 1.0,
            PassageStage::Entering => smooth(self.elapsed / DOOR_OPEN_S),
            PassageStage::Exiting => 1.0 - smooth((self.elapsed - DOOR_OUT_S) / DOOR_SHUT_S),
        };
        Some((self.zone.as_str(), open))
    }

    /// The ripple row this passage's gate plays (`swallow` or `emit`) and how
    /// far through it, with its zone.
    pub fn gate_ripple(&self) -> Option<(&str, &'static str, f32)> {
        if self.kind != PassageKind::Gate {
            return None;
        }
        let (row, start) = match self.stage {
            PassageStage::Entering => ("swallow", GATE_TURN_S + GATE_CROUCH_S + GATE_CONTACT * GATE_LEAP_S),
            PassageStage::Exiting => ("emit", 0.0),
        };
        let k = (self.elapsed - start) / GATE_RIPPLE_S;
        (0.0..1.0).contains(&k).then_some((self.zone.as_str(), row, k))
    }
}

/// Play each passage: place its body, hold it, record the crossing when the
/// going-in ends, turn it into the coming-out when its body arrives in another
/// room, and end it when it is over.
#[allow(clippy::too_many_arguments)]
pub fn run_zone_passages(
    mut commands: Commands,
    world_time: Res<ambition_time::WorldTime>,
    boundary: Option<Res<ae::ConfirmedFrameBoundary>>,
    mut pending_lifecycle: ResMut<PendingLifecycleCommit>,
    mut sfx: ambition_sfx::SfxWriter,
    mut bodies: Query<(
        Entity,
        ae::BodyClusterQueryData,
        &mut ae::MotionModel,
        &mut ZonePassage,
        Option<&mut ControlHolds>,
        Option<&ae::PoseOwnedExternally>,
        Option<&InRoomInstance>,
    )>,
) {
    let dt = world_time.sim_dt();
    let now = boundary.as_ref().map_or(0, |boundary| boundary.current);
    for (entity, clusters, mut model, mut passage, mut control_holds, pose_holds, stamp) in &mut bodies {
        let here = stamp.map(|stamp| stamp.0);
        if passage.room.is_none() {
            passage.room = here;
        }
        let mut item = clusters;
        let mut clusters = item.as_clusters_mut();
        if passage.room != here {
            // Moved to another room by something else: the crossing's own
            // arrival is the commit's (`ZonePassage::arrived`), which starts
            // the half anew in its room.
            end_passage(&mut commands, entity, control_holds.as_deref_mut());
            continue;
        }
        passage.elapsed += dt;
        let stage_s = match passage.stage {
            PassageStage::Entering => ZonePassage::entering_s(passage.kind),
            PassageStage::Exiting => ZonePassage::exiting_s(passage.kind),
        };
        if passage.stage == PassageStage::Entering && passage.elapsed >= stage_s && !passage.recorded {
            if let Some(crossing) = passage.crossing.clone() {
                let admission = pending_lifecycle
                    .record(now, LifecycleIntent::Transition(crossing));
                // Refused: another lifecycle operation owns this frame; ask
                // again on the next.
                passage.recorded = admission.admitted();
                passage.recorded_at = passage.elapsed;
            }
        }
        if passage.stage == PassageStage::Entering
            && passage.recorded
            && passage.elapsed - passage.recorded_at >= CROSSING_GIVE_UP_S
        {
            bevy::log::warn!("passage through `{}`: the crossing never arrived; the body is given back", passage.zone);
            // The crossing is withdrawn with the passage. A commit that is
            // only late would otherwise move the body after its player got
            // it back. The withdrawal is recorded on this frame, so the host
            // ends the load it opened for the crossing when the frame is
            // confirmed.
            if let Some(crossing) = passage.crossing.clone() {
                let _ = pending_lifecycle.retract(&LifecycleIntent::Transition(crossing), now);
            }
            end_passage(&mut commands, entity, control_holds.as_deref_mut());
            continue;
        }
        if passage.stage == PassageStage::Exiting
            && passage.kind == PassageKind::Door
            && passage.elapsed >= DOOR_OUT_S
            && passage.elapsed - dt < DOOR_OUT_S
        {
            sfx.write(ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_DOOR_CLOSE,
                pos: passage.foot,
            });
        }
        if passage.stage == PassageStage::Exiting && passage.elapsed >= stage_s {
            end_passage(&mut commands, entity, control_holds.as_deref_mut());
            continue;
        }
        match passage.body_at() {
            Some(pos) => {
                let pos = if passage.recorded { clusters.kinematics.pos } else { pos };
                ae::movement::transit_body(&mut model, &mut clusters, pos, ae::movement::TransitVelocity::Zero);
                if passage.stage == PassageStage::Exiting {
                    clusters.kinematics.facing = passage.facing;
                }
                // Claimed again on any tick that does not hold them, so a
                // passage a restore brings back holds its body.
                if !control_holds.as_deref().is_some_and(|holds| holds.holds(ControlHold::Transit)) {
                    claim_control_hold(&mut commands, entity, ControlHold::Transit);
                }
                if !pose_holds.is_some_and(|holds| holds.holds(ae::PoseHold::Transit)) {
                    ae::claim_pose_hold(&mut commands, entity, ae::PoseHold::Transit);
                }
            }
            None => {
                // The body is free; the door or the gate plays on.
                release_control_hold(&mut commands, entity, control_holds.as_deref_mut(), ControlHold::Transit);
                if pose_holds.is_some_and(|holds| holds.holds(ae::PoseHold::Transit)) {
                    ae::release_pose_hold(&mut commands, entity, ae::PoseHold::Transit);
                }
            }
        }
    }
}

fn end_passage(commands: &mut Commands, entity: Entity, control_holds: Option<&mut ControlHolds>) {
    commands.entity(entity).try_remove::<ZonePassage>();
    release_control_hold(commands, entity, control_holds, ControlHold::Transit);
    ae::release_pose_hold(commands, entity, ae::PoseHold::Transit);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passage(kind: PassageKind) -> ZonePassage {
        ZonePassage {
            kind,
            stage: PassageStage::Entering,
            elapsed: 0.0,
            from: ae::Vec2::new(90.0, 177.0),
            zone: "z".into(),
            foot: ae::Vec2::new(100.0, 200.0),
            heart: ae::Vec2::new(100.0, 104.0),
            half_h: 23.0,
            facing: 1.0,
            crossing: None,
            recorded: false,
            recorded_at: 0.0,
            arrival: None,
            room: None,
        }
    }

    /// A body in a door passage, with its crossing to `elsewhere` not yet
    /// recorded, in an app that runs only the passage system at 60 Hz.
    fn passage_app() -> (App, Entity, RoomTransitionIntent) {
        let mut app = App::new();
        let mut time = ambition_time::WorldTime::default();
        time.set_sim_dt(1.0 / 60.0);
        app.insert_resource(time);
        app.init_resource::<PendingLifecycleCommit>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_systems(Update, run_zone_passages);
        let crossing = RoomTransitionIntent {
            subject: ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement("hero"),
                None,
            ),
            target_room: "elsewhere".into(),
            arrival: ae::Vec2::new(1.0, 2.0),
            edge_exit: false,
            zone_sfx: None,
            participant: None,
        };
        let body = app
            .world_mut()
            .spawn((
                (
                    ae::BodyAbilities::default(),
                    ae::BodyKinematics::default(),
                    ae::BodyBaseSize::default(),
                    ae::body_clusters::BodyGroundState::default(),
                    ae::body_clusters::BodyWallState::default(),
                    ae::body_clusters::BodyJumpState::default(),
                    ae::body_clusters::BodyDashState::default(),
                    ae::body_clusters::BodyFlightState::default(),
                ),
                (
                    ae::body_clusters::BodyBlinkState::default(),
                    ae::body_clusters::BodyLedgeState::default(),
                    ae::body_clusters::BodyDodgeState::default(),
                    ae::body_clusters::BodyShieldState::default(),
                    ae::body_clusters::BodyModeState::default(),
                    ae::body_clusters::BodyActionBuffer::default(),
                    ae::body_clusters::BodyRestartLatch::default(),
                    ae::MotionModel::default(),
                ),
                ZonePassage { crossing: Some(crossing.clone()), ..passage(PassageKind::Door) },
            ))
            .id();
        (app, body, crossing)
    }

    /// Review finding #4 (2026-10-19). The crossing is recorded and its
    /// commit is late (the target room is still preparing): the passage gives
    /// up and gives the body back, and the crossing is withdrawn with it, so a
    /// later commit cannot move the body. Measured before: the passage ended
    /// and the crossing stayed pending. Poison: do not retract at the give-up.
    #[test]
    fn a_passage_that_gives_up_withdraws_its_crossing() {
        let (mut app, body, crossing) = passage_app();
        let frames = |seconds: f32| (seconds * 60.0).ceil() as usize + 1;
        for _ in 0..frames(ZonePassage::entering_s(PassageKind::Door)) {
            app.update();
        }
        assert_eq!(
            app.world().resource::<PendingLifecycleCommit>().peek().map(|pending| &pending.kind),
            Some(&LifecycleIntent::Transition(crossing.clone())),
            "premise: the going-in ended and recorded the crossing"
        );
        assert!(app.world().get::<ZonePassage>(body).is_some(), "premise: the passage waits for the crossing");

        for _ in 0..frames(CROSSING_GIVE_UP_S) {
            app.update();
        }
        assert!(app.world().get::<ZonePassage>(body).is_none(), "premise: the passage gave up");
        assert_eq!(
            app.world().resource::<PendingLifecycleCommit>().peek(),
            None,
            "the passage gave the body back and its crossing is still pending"
        );
    }

    /// Review b2c36ae4, finding 3. Another operation holds the lifecycle slot
    /// for longer than the give-up time, so the crossing waits in the queue.
    /// When the slot comes free the crossing is admitted, and the passage
    /// waits the full give-up time from then. Measured before: the wait was
    /// measured from the entry, so the passage gave up on the tick its
    /// crossing was admitted. Poison: measure from the entry again.
    #[test]
    fn a_crossing_admitted_after_a_long_queue_wait_stays_active() {
        let (mut app, body, crossing) = passage_app();
        let frames = |seconds: f32| (seconds * 60.0).ceil() as usize + 1;
        // A second body's crossing, admitted first.
        let other = LifecycleIntent::Transition(RoomTransitionIntent {
            subject: ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement("partner"),
                None,
            ),
            target_room: "another room".into(),
            ..crossing.clone()
        });
        let _ = app.world_mut().resource_mut::<PendingLifecycleCommit>().record(1, other.clone());
        for _ in 0..frames(ZonePassage::entering_s(PassageKind::Door) + CROSSING_GIVE_UP_S + 1.0) {
            app.update();
        }
        let waiting = app.world().get::<ZonePassage>(body).expect("premise: the passage waits for the slot");
        assert!(!waiting.recorded, "premise: the slot is another operation's");
        assert_eq!(app.world_mut().resource_mut::<PendingLifecycleCommit>().take().map(|p| p.kind), Some(other));

        app.update();
        assert_eq!(
            app.world().resource::<PendingLifecycleCommit>().withdrawn.as_ref().map(|withdrawn| &withdrawn.kind),
            None,
            "the crossing was withdrawn on the tick the free slot admitted it"
        );
        assert_eq!(
            app.world().resource::<PendingLifecycleCommit>().peek().map(|pending| &pending.kind),
            Some(&LifecycleIntent::Transition(crossing.clone())),
            "premise: the free slot admits the crossing"
        );
        for _ in 0..frames(CROSSING_GIVE_UP_S - 1.0) {
            app.update();
        }
        assert!(
            app.world().get::<ZonePassage>(body).is_some_and(|passage| passage.recorded),
            "the passage gave up a crossing it had waited for for less than {CROSSING_GIVE_UP_S} s"
        );
        assert_eq!(
            app.world().resource::<PendingLifecycleCommit>().peek().map(|pending| &pending.kind),
            Some(&LifecycleIntent::Transition(crossing)),
            "the admitted crossing was withdrawn"
        );
        // Control: the give-up still comes, a give-up time after the admission.
        for _ in 0..frames(1.0) {
            app.update();
        }
        assert!(app.world().get::<ZonePassage>(body).is_none(), "the passage never gave up");
    }

    /// Maintainability finding of review b2c36ae4. The desync probe saw only
    /// the clock, the stage, `recorded` and `from`, so two peers whose
    /// passages recorded different crossings or came out of different zones
    /// agreed. Each field of the passage now moves it. Poison: leave a field
    /// out of the hash.
    #[test]
    fn each_field_of_a_passage_moves_its_checksum() {
        let base = ZonePassage {
            crossing: Some(RoomTransitionIntent {
                subject: ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(
                    ambition_platformer2d_shared_tangle::sim_id::SimId::placement("hero"),
                    None,
                ),
                target_room: "elsewhere".into(),
                arrival: ae::Vec2::new(1.0, 2.0),
                edge_exit: false,
                zone_sfx: None,
                participant: None,
            }),
            arrival: Some(PassageArrival {
                kind: Some(PassageKind::Door),
                zone: "far door".into(),
                foot: ae::Vec2::new(5.0, 6.0),
                heart: ae::Vec2::new(5.0, 3.0),
            }),
            ..passage(PassageKind::Door)
        };
        let crossing = base.crossing.clone().unwrap();
        let arrival = base.arrival.clone().unwrap();
        let changed: Vec<(&str, ZonePassage)> = vec![
            ("kind", ZonePassage { kind: PassageKind::Gate, ..base.clone() }),
            ("stage", ZonePassage { stage: PassageStage::Exiting, ..base.clone() }),
            ("elapsed", ZonePassage { elapsed: 0.5, ..base.clone() }),
            ("from", ZonePassage { from: ae::Vec2::new(91.0, 177.0), ..base.clone() }),
            ("zone", ZonePassage { zone: "y".into(), ..base.clone() }),
            ("foot", ZonePassage { foot: ae::Vec2::new(100.0, 201.0), ..base.clone() }),
            ("heart", ZonePassage { heart: ae::Vec2::new(101.0, 104.0), ..base.clone() }),
            ("half_h", ZonePassage { half_h: 24.0, ..base.clone() }),
            ("facing", ZonePassage { facing: -1.0, ..base.clone() }),
            ("crossing", ZonePassage { crossing: None, ..base.clone() }),
            (
                "crossing target",
                ZonePassage { crossing: Some(RoomTransitionIntent { target_room: "another".into(), ..crossing.clone() }), ..base.clone() },
            ),
            ("recorded", ZonePassage { recorded: true, ..base.clone() }),
            ("recorded_at", ZonePassage { recorded_at: 0.25, ..base.clone() }),
            ("arrival", ZonePassage { arrival: None, ..base.clone() }),
            ("arrival kind", ZonePassage { arrival: Some(PassageArrival { kind: None, ..arrival.clone() }), ..base.clone() }),
            ("arrival zone", ZonePassage { arrival: Some(PassageArrival { zone: "near door".into(), ..arrival.clone() }), ..base.clone() }),
            ("arrival foot", ZonePassage { arrival: Some(PassageArrival { foot: ae::Vec2::ZERO, ..arrival.clone() }), ..base.clone() }),
            ("arrival heart", ZonePassage { arrival: Some(PassageArrival { heart: ae::Vec2::ZERO, ..arrival.clone() }), ..base.clone() }),
            ("room", ZonePassage { room: Some(LiveRoomInstance::ACTIVATION), ..base.clone() }),
            ("room instance", ZonePassage { room: Some(LiveRoomInstance::ACTIVATION.next()), ..base.clone() }),
        ];
        for (field, passage) in &changed {
            let against = if *field == "room instance" {
                ZonePassage { room: Some(LiveRoomInstance::ACTIVATION), ..base.clone() }
            } else {
                base.clone()
            };
            assert_ne!(passage.checksum(), against.checksum(), "the passage's `{field}` does not move its checksum");
        }
        assert_eq!(base.checksum(), base.clone().checksum(), "control: one passage, one checksum");
    }

    /// Control: the give-up withdraws only its own crossing. Another
    /// operation that holds the slot stays.
    #[test]
    fn a_passage_that_gives_up_leaves_another_operation_pending() {
        let (mut app, body, crossing) = passage_app();
        for _ in 0..(ZonePassage::entering_s(PassageKind::Door) * 60.0).ceil() as usize + 1 {
            app.update();
        }
        let other = LifecycleIntent::Transition(RoomTransitionIntent { target_room: "another room".into(), ..crossing });
        app.world_mut().resource_mut::<PendingLifecycleCommit>().pending =
            Some(crate::session::lifecycle_commit::PendingIntent { frame: 7, kind: other.clone() });
        for _ in 0..(CROSSING_GIVE_UP_S * 60.0).ceil() as usize + 1 {
            app.update();
        }
        assert!(app.world().get::<ZonePassage>(body).is_none(), "premise: the passage gave up");
        assert_eq!(
            app.world().resource::<PendingLifecycleCommit>().peek().map(|pending| &pending.kind),
            Some(&other),
            "the give-up withdrew an operation that was not its own"
        );
    }

    /// Review finding #5 (2026-10-19). A body that comes in off the stand
    /// line is moved there, not put there. The body meets the zone 30 px
    /// along y from the stand point, as a body in sideways gravity does (its
    /// run is along y). On the first tick it is where it was, and no tick of
    /// the walk to the doorway or the crouch before a leap moves it further
    /// than a smooth step. Measured before: y was the stand line's from the
    /// first tick, a 30 px jump. Poison: snap y again.
    ///
    /// ⚠ The stand point is still the zone's bottom edge in world y: the
    /// passage places a body upright whatever its gravity.
    #[test]
    fn a_body_that_comes_in_off_the_stand_line_is_moved_onto_it() {
        for kind in [PassageKind::Door, PassageKind::Gate] {
            let mut p = passage(kind);
            p.from = ae::Vec2::new(p.foot.x - 10.0, p.foot.y - p.half_h - 30.0);
            assert_eq!(p.body_at(), Some(p.from), "{kind:?}: on the first tick the body is where it was");
            let settle = match kind {
                PassageKind::Door => DOOR_OPEN_S,
                PassageKind::Gate => GATE_TURN_S + GATE_CROUCH_S,
            };
            let dt = 1.0 / 60.0;
            let mut last = p.from;
            while p.elapsed + dt < settle {
                p.elapsed += dt;
                let at = p.body_at().expect("the body is held");
                assert!(at.distance(last) < 4.0, "{kind:?} at {:.3} s: a step of {} px", p.elapsed, at.distance(last));
                last = at;
            }
            p.elapsed = settle;
            let stand = ae::Vec2::new(p.foot.x, p.foot.y - p.half_h);
            assert!(p.body_at().expect("held").distance(stand) < 1e-3, "{kind:?}: then it stands in the doorway");
        }
    }

    /// A door opens before the body goes in, stays open while it is through,
    /// and shuts after it has come out.
    #[test]
    fn a_door_opens_for_the_body_and_shuts_behind_it() {
        let mut p = passage(PassageKind::Door);
        assert_eq!(p.door_open().unwrap().1, 0.0);
        p.elapsed = DOOR_OPEN_S;
        assert_eq!(p.door_open().unwrap().1, 1.0);
        p.stage = PassageStage::Exiting;
        p.elapsed = DOOR_OUT_S * 0.5;
        assert_eq!(p.door_open().unwrap().1, 1.0);
        assert!(p.holds_body());
        p.elapsed = DOOR_OUT_S + DOOR_SHUT_S;
        assert_eq!(p.door_open().unwrap().1, 0.0);
        assert!(!p.holds_body());
    }

    /// The body ends the going-in out of sight, at a gate in its middle, and
    /// the gate's swallow starts as the body reaches it.
    #[test]
    fn a_body_goes_out_of_sight_into_the_middle_of_a_gate() {
        let mut p = passage(PassageKind::Gate);
        p.elapsed = GATE_TURN_S + GATE_CROUCH_S + GATE_LEAP_S;
        let look = p.look().unwrap();
        assert!(look.alpha < 0.01, "{look:?}");
        assert!(look.scale < 0.3, "{look:?}");
        assert!(p.body_at().unwrap().distance(p.heart) < 12.0);
        assert_eq!(p.gate_ripple().unwrap().1, "swallow");
        p.elapsed = 0.0;
        assert!(p.gate_ripple().is_none());
    }

    /// Out of a gate, the body lands on the floor in front of it, at full size.
    #[test]
    fn a_body_thrown_out_of_a_gate_lands_in_front_of_it() {
        let mut p = passage(PassageKind::Gate);
        p.stage = PassageStage::Exiting;
        p.elapsed = GATE_EMERGE_DELAY_S + GATE_ARC_S - 1e-4;
        let at = p.body_at().unwrap();
        assert!((at.y - 177.0).abs() < 1.0 && at.x > 120.0, "{at:?}");
        assert!(p.look().unwrap().scale > 0.99);
    }
}
