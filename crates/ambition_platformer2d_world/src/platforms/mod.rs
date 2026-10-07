//! Authored moving platforms: the spec an editor writes, the motion it
//! resolves to, and the runtime state the simulation advances.
//!
//! Moving platforms are ordinary deterministic world geometry. They contribute
//! solid blocks to the collision world each frame, carry riders and ledge
//! contacts by [`MovingPlatformState::last_delta`], and can host a portal face.
//! The authoritative state lives here in the world crate; the Bevy visual is a
//! read-model projection of it.

use crate::rooms::KinematicPathSpec;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;

/// Sweep span used when an author places a platform and states no motion.
///
/// The engine owns its defaults, so an empty field has one meaning that does
/// not depend on the adapter.
pub const DEFAULT_SWEEP_DX: f32 = 240.0;
/// Travel speed used when an author states none.
pub const DEFAULT_PLATFORM_SPEED: f32 = 130.0;

/// How an authored platform moves — exactly one motion, decided when the room
/// is authored.
///
/// Motion is one variant, not a set of optional fields with a precedence
/// rule. A precedence rule makes every wrong combination silent, and an author
/// cannot see it from inside LDtk. [`AuthoredPlatformMotion::classify`]
/// refuses ambiguous combinations and names the fields in the message.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MovingPlatformMotionSpec {
    /// Horizontal ping-pong across `dx` from the authored position. Sign is the
    /// direction the platform first travels.
    Sweep { dx: f32, speed: f32 },
    /// Follow a room-local [`KinematicPathSpec`], which owns its own speed.
    Path { path_id: String },
    /// A wrapping vertical loop — the paternoster / "infinite elevator".
    ///
    /// `dy` works like a sweep's span: magnitude is the shaft, sign is the
    /// direction of travel. World y is down-positive (the LDtk conversion keeps
    /// it), so a descending elevator has a positive `dy`. A loop never
    /// reverses, so a run of them reads as one elevator.
    ///
    /// When anchored, the authored position is a phase within a shared shaft.
    /// `None` anchors the shaft at the platform, which suits a lone lift.
    VerticalLoop {
        dy: f32,
        anchor_y: Option<f32>,
        speed: f32,
    },
    /// A wrapping HORIZONTAL loop, the sideways sibling of
    /// [`Self::VerticalLoop`]: a flock that leaves one side of the room and
    /// comes back in at the other, bobbing as it flies (the Mockingbird's
    /// sharks, fleeing across a sky that never stops).
    ///
    /// `dx` is the lane: magnitude is its length, sign the direction of
    /// travel. Anchored, the authored position is a phase within a shared
    /// lane (`anchor_x` is its left end); `None` starts the lane at the
    /// platform. `bob` is how far it rises and falls about its authored
    /// height, over [`BOB_PERIOD_S`].
    HorizontalLoop {
        dx: f32,
        anchor_x: Option<f32>,
        speed: f32,
        bob: f32,
    },
    /// A lift that waits for a rider: it rests where it was authored until a
    /// body stands on it, then carries it `dy` (negative rises) at `speed`
    /// and, once there, comes back down to rest the way it went. A ride out of
    /// a room through its ceiling: a shark that swoops down for you.
    Lift { dy: f32, speed: f32 },
}

/// How long a [`MovingPlatformMotionSpec::HorizontalLoop`] takes to rise and
/// fall once.
pub const BOB_PERIOD_S: f32 = 2.6;

/// The fastest a ONE-WAY platform may climb (px/s). The one-way landing rule
/// compares a body's previous feet with the surface's CURRENT face; a face
/// that rose further than `ONE_WAY_CROSSING_SLOP` in one tick would leave its
/// own rider below it, and the rider would fall through. At 60 Hz the slop
/// (8 px) allows 480 px/s; this keeps a margin.
pub const MAX_ONE_WAY_CLIMB: f32 = 420.0;

/// The motion fields an editor can write on one platform, before they are known
/// to describe a coherent motion.
///
/// This is the adapter-facing shape: an LDtk converter fills in whichever fields
/// the author touched and asks [`Self::classify`] what they mean. Nothing
/// downstream of `classify` can observe an ambiguous platform.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AuthoredPlatformMotion {
    pub sweep_dx: Option<f32>,
    pub speed: Option<f32>,
    pub path_id: Option<String>,
    pub loop_dy: Option<f32>,
    pub loop_anchor_y: Option<f32>,
    pub loop_dx: Option<f32>,
    pub loop_anchor_x: Option<f32>,
    pub bob: Option<f32>,
    pub lift_dy: Option<f32>,
}

impl AuthoredPlatformMotion {
    /// Decide which motion these fields describe, or say why they describe none.
    ///
    /// Stating nothing is legal and means a default sweep — an author who drops
    /// a platform into a room gets a platform that moves. Stating two motions is
    /// not legal, because there is no honest way to guess which one was meant.
    pub fn classify(self) -> Result<MovingPlatformMotionSpec, String> {
        let path_id = self.path_id.and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });

        let mut authored = Vec::new();
        if self.sweep_dx.is_some() {
            authored.push("sweep_dx");
        }
        if path_id.is_some() {
            authored.push("path_id");
        }
        if self.loop_dy.is_some() {
            authored.push("loop_dy");
        }
        if self.loop_dx.is_some() {
            authored.push("loop_dx");
        }
        if self.lift_dy.is_some() {
            authored.push("lift_dy");
        }
        if authored.len() > 1 {
            return Err(format!(
                "authors {} at once, but a platform has exactly one motion — \
                 keep the field for the motion you want and clear the others",
                authored.join(" and ")
            ));
        }

        if self.loop_anchor_y.is_some() && self.loop_dy.is_none() {
            return Err(
                "authors loop_min_y without loop_dy — the anchor names where a \
                 wrapping shaft starts, so on its own it describes no motion at \
                 all"
                .to_string(),
            );
        }

        if self.loop_anchor_x.is_some() && self.loop_dx.is_none() {
            return Err("authors loop_min_x without loop_dx — the anchor names where a \
                 wrapping lane starts, so on its own it describes no motion at all"
                .to_string());
        }
        if self.bob.is_some() && self.loop_dx.is_none() {
            return Err("authors bob without loop_dx — only a horizontal loop bobs".to_string());
        }
        if let Some(dx) = self.loop_dx {
            if dx.abs() <= f32::EPSILON {
                return Err("authors loop_dx of zero — a lane with no length never moves; \
                     give it a signed length (positive travels RIGHT) or clear it"
                    .to_string());
            }
            return Ok(MovingPlatformMotionSpec::HorizontalLoop {
                dx,
                anchor_x: self.loop_anchor_x,
                speed: self.speed.unwrap_or(DEFAULT_PLATFORM_SPEED),
                bob: self.bob.unwrap_or(0.0),
            });
        }
        if let Some(dy) = self.lift_dy {
            if dy.abs() <= f32::EPSILON {
                return Err("authors lift_dy of zero — a lift that goes nowhere; give it a \
                     signed rise (negative travels UP) or clear it"
                    .to_string());
            }
            return Ok(MovingPlatformMotionSpec::Lift {
                dy,
                speed: self.speed.unwrap_or(DEFAULT_PLATFORM_SPEED),
            });
        }

        if let Some(dy) = self.loop_dy {
            if dy.abs() <= f32::EPSILON {
                return Err(
                    "authors loop_dy of zero — a shaft with no span never moves; \
                     give it a signed height (positive travels DOWN) or clear it"
                        .to_string(),
                );
            }
            return Ok(MovingPlatformMotionSpec::VerticalLoop {
                dy,
                anchor_y: self.loop_anchor_y,
                speed: self.speed.unwrap_or(DEFAULT_PLATFORM_SPEED),
            });
        }

        if let Some(path_id) = path_id {
            // The path owns its speed, so a `speed` written here does nothing.
            if self.speed.is_some() {
                return Err(format!(
                    "follows path '{path_id}' and also authors speed, but a \
                     path carries its own speed — set it on the KinematicPath"
                ));
            }
            return Ok(MovingPlatformMotionSpec::Path { path_id });
        }

        Ok(MovingPlatformMotionSpec::Sweep {
            dx: self.sweep_dx.unwrap_or(DEFAULT_SWEEP_DX),
            speed: self.speed.unwrap_or(DEFAULT_PLATFORM_SPEED),
        })
    }
}

/// An authored moving-platform declaration before path references are resolved.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MovingPlatformSpec {
    pub id: String,
    pub name: String,
    pub start_pos: ae::Vec2,
    pub size: ae::Vec2,
    pub motion: MovingPlatformMotionSpec,
    /// The sheet it is drawn as (a registered prop sheet kind, such as
    /// `burning_flying_shark`); `None` draws the plain platform.
    #[serde(default)]
    pub visual: Option<String>,
    /// A body jumps up through it and lands on top (a shark's back), instead
    /// of striking its underside. Its climb is held to [`MAX_ONE_WAY_CLIMB`].
    #[serde(default)]
    pub one_way: bool,
}

impl MovingPlatformSpec {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        start_pos: ae::Vec2,
        size: ae::Vec2,
        motion: MovingPlatformMotionSpec,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            start_pos,
            size,
            motion,
            visual: None,
            one_way: false,
        }
    }

    /// Draw it as `visual` (a registered prop sheet kind).
    pub fn with_visual(mut self, visual: Option<String>) -> Self {
        self.visual = visual;
        self
    }

    /// Make it one-way ([`Self::one_way`]).
    pub fn with_one_way(mut self, one_way: bool) -> Self {
        self.one_way = one_way;
        self
    }

    pub fn resolve(self, paths: &[KinematicPathSpec]) -> Result<MovingPlatformState, String> {
        let (visual, one_way) = (self.visual.clone(), self.one_way);
        if one_way {
            let climb = match &self.motion {
                MovingPlatformMotionSpec::VerticalLoop { speed, .. } | MovingPlatformMotionSpec::Lift { speed, .. } => {
                    *speed
                }
                MovingPlatformMotionSpec::HorizontalLoop { bob, .. } => {
                    bob.abs() * std::f32::consts::TAU / BOB_PERIOD_S
                }
                MovingPlatformMotionSpec::Sweep { .. } => 0.0,
                // A path owns its speed; its climb is its own business.
                MovingPlatformMotionSpec::Path { .. } => 0.0,
            };
            if climb > MAX_ONE_WAY_CLIMB {
                return Err(format!(
                    "MovingPlatform '{}' is one-way but climbs at {climb} px/s; above \
                     {MAX_ONE_WAY_CLIMB} its own rider is left below its face and falls through",
                    self.name
                ));
            }
        }
        self.resolve_motion(paths).map(|mut state| {
            state.visual = visual;
            state.one_way = one_way;
            state
        })
    }

    fn resolve_motion(self, paths: &[KinematicPathSpec]) -> Result<MovingPlatformState, String> {
        match self.motion {
            MovingPlatformMotionSpec::Path { path_id } => {
                let Some(path_spec) = paths.iter().find(|path| path.matches_id(&path_id)) else {
                    let known = paths
                        .iter()
                        .flat_map(|path| path.resolution_aliases())
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(format!(
                        "MovingPlatform '{}' references unknown path_id '{}' (known: [{}])",
                        self.name, path_id, known
                    ));
                };
                Ok(MovingPlatformState::from_path(
                    self.id,
                    self.name,
                    self.size,
                    path_spec.path.clone(),
                ))
            }
            MovingPlatformMotionSpec::VerticalLoop {
                dy,
                anchor_y,
                speed,
            } => {
                let (min_y, max_y) = match anchor_y {
                    Some(base) => (base, base + dy.abs()),
                    None => {
                        let end_y = self.start_pos.y + dy;
                        (self.start_pos.y.min(end_y), self.start_pos.y.max(end_y))
                    }
                };
                Ok(MovingPlatformState::from_vertical_loop(
                    self.id,
                    self.name,
                    self.start_pos,
                    self.size,
                    min_y,
                    max_y,
                    speed,
                    // positive dy travels toward +y, which is DOWN.
                    dy > 0.0,
                ))
            }
            MovingPlatformMotionSpec::HorizontalLoop { dx, anchor_x, speed, bob } => {
                let (min_x, max_x) = match anchor_x {
                    Some(base) => (base, base + dx.abs()),
                    None => {
                        let end_x = self.start_pos.x + dx;
                        (self.start_pos.x.min(end_x), self.start_pos.x.max(end_x))
                    }
                };
                Ok(MovingPlatformState::from_horizontal_loop(
                    self.id,
                    self.name,
                    self.start_pos,
                    self.size,
                    (min_x, max_x),
                    speed,
                    dx > 0.0,
                    bob,
                ))
            }
            MovingPlatformMotionSpec::Lift { dy, speed } => Ok(MovingPlatformState::lift(
                self.id,
                self.name,
                self.start_pos,
                self.size,
                dy,
                speed,
            )),
            MovingPlatformMotionSpec::Sweep { dx, speed } => Ok(MovingPlatformState::from_sweep(
                self.id,
                self.name,
                self.start_pos,
                self.size,
                dx,
                speed,
            )),
        }
    }
}

/// Runtime state for one LDtk-authored moving platform.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MovingPlatformState {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    motion: MovingPlatformMotion,
    /// Displacement applied by the most recent [`Self::update`] advance.
    last_delta: ae::Vec2,
    /// The sheet it is drawn as (`MovingPlatformSpec::visual`).
    #[serde(default)]
    pub visual: Option<String>,
    /// One-way (`MovingPlatformSpec::one_way`).
    #[serde(default)]
    pub one_way: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
enum MovingPlatformMotion {
    Sweep {
        min_x: f32,
        max_x: f32,
        speed: f32,
        dir: f32,
    },
    Path {
        path: ambition_platformer2d_core::KinematicPath,
        segment: usize,
        dir: i32,
    },
    /// A one-way vertical loop — the paternoster / "infinite elevator".
    ///
    /// Moves continuously in one vertical direction and wraps to the opposite
    /// end instead of reversing.
    ///
    /// It wraps where the other variants reverse. A reversing platform is a
    /// lift; a wrapping one is a conveyor of lifts (step off the top, another
    /// arrives from below).
    Loop {
        min_y: f32,
        max_y: f32,
        speed: f32,
        /// `+1` travels toward +y, `-1` toward -y. Constant for the lifetime of
        /// the platform: this motion never reverses.
        ///
        /// +y is down. The LDtk conversion does not flip the axis, so `+1`
        /// descends on screen.
        dir: f32,
    },
    /// A one-way horizontal loop that bobs: it wraps where a sweep reverses.
    SideLoop {
        min_x: f32,
        max_x: f32,
        speed: f32,
        /// `+1` travels right, `-1` left.
        dir: f32,
        /// The height it bobs about, how far, and how far into the bob.
        base_y: f32,
        bob: f32,
        bob_t: f32,
    },
    /// A lift that waits for a rider (`MovingPlatformMotionSpec::Lift`).
    Lift {
        rest_y: f32,
        /// Where it carries a rider to.
        end_y: f32,
        speed: f32,
        /// A body stood on it at the last advance ([`MovingPlatformState::set_ridden`]).
        ridden: bool,
        stage: LiftStage,
    },
    /// A carrier that comes up under a falling body, lifts it to `rise_to_y`
    /// and then flies off with the flock, out past `gone_x`: the fall rescue.
    Ferry {
        rise_to_y: f32,
        rise_speed: f32,
        drift: f32,
        gone_x: f32,
    },
}

/// Where a [`MovingPlatformMotion::Lift`] is in its round trip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LiftStage {
    Resting,
    Carrying,
    Returning,
}

impl MovingPlatformState {
    /// Build from LDtk-authored AABB + sweep range. Kept as a test/helper
    /// constructor for simple horizontal platforms; runtime LDtk conversion now
    /// goes through `MovingPlatformSpec` (see same module) so optional
    /// `path_id` references can be resolved against the active area's
    /// `KinematicPathSpec` index.
    pub fn from_authored(start_pos: ae::Vec2, size: ae::Vec2, sweep_dx: f32, speed: f32) -> Self {
        Self::from_sweep(
            "moving_platform",
            "Moving Platform",
            start_pos,
            size,
            sweep_dx,
            speed,
        )
    }

    pub fn from_sweep(
        id: impl Into<String>,
        name: impl Into<String>,
        start_pos: ae::Vec2,
        size: ae::Vec2,
        sweep_dx: f32,
        speed: f32,
    ) -> Self {
        let (min_x, max_x) = if sweep_dx >= 0.0 {
            (start_pos.x, start_pos.x + sweep_dx)
        } else {
            (start_pos.x + sweep_dx, start_pos.x)
        };
        let dir = if sweep_dx >= 0.0 { 1.0 } else { -1.0 };
        Self {
            id: id.into(),
            name: name.into(),
            pos: start_pos,
            size,
            motion: MovingPlatformMotion::Sweep {
                min_x,
                max_x,
                speed: speed.max(0.0),
                dir,
            },
            last_delta: ae::Vec2::ZERO,
            visual: None,
            one_way: false,
        }
    }

    /// A wrapping vertical loop between `min_y` and `max_y`.
    ///
    /// `speed` is magnitude; `downward` picks the direction. A run of these with
    /// staggered `start_pos` values along the same span is the elevator shaft.
    ///
    /// The parameter is `downward`, not `rising`, because +y is down.
    pub fn from_vertical_loop(
        id: impl Into<String>,
        name: impl Into<String>,
        start_pos: ae::Vec2,
        size: ae::Vec2,
        min_y: f32,
        max_y: f32,
        speed: f32,
        downward: bool,
    ) -> Self {
        let (min_y, max_y) = if min_y <= max_y {
            (min_y, max_y)
        } else {
            (max_y, min_y)
        };
        Self {
            id: id.into(),
            name: name.into(),
            pos: start_pos,
            size,
            motion: MovingPlatformMotion::Loop {
                min_y,
                max_y,
                speed: speed.max(0.0),
                dir: if downward { 1.0 } else { -1.0 },
            },
            last_delta: ae::Vec2::ZERO,
            visual: None,
            one_way: false,
        }
    }

    pub fn from_path(
        id: impl Into<String>,
        name: impl Into<String>,
        size: ae::Vec2,
        path: ambition_platformer2d_core::KinematicPath,
    ) -> Self {
        let pos = path.points.first().copied().unwrap_or(ae::Vec2::ZERO);
        Self {
            id: id.into(),
            name: name.into(),
            pos,
            size,
            motion: MovingPlatformMotion::Path {
                path,
                segment: 0,
                dir: 1,
            },
            last_delta: ae::Vec2::ZERO,
            visual: None,
            one_way: false,
        }
    }

    /// A wrapping horizontal loop between `min_x` and `max_x` that bobs `bob`
    /// about its authored height. Its bob starts at a phase taken from where
    /// it starts in the lane, so a flock does not bob in step.
    #[allow(clippy::too_many_arguments)]
    pub fn from_horizontal_loop(
        id: impl Into<String>,
        name: impl Into<String>,
        start_pos: ae::Vec2,
        size: ae::Vec2,
        (min_x, max_x): (f32, f32),
        speed: f32,
        rightward: bool,
        bob: f32,
    ) -> Self {
        let (min_x, max_x) = if min_x <= max_x { (min_x, max_x) } else { (max_x, min_x) };
        let span = (max_x - min_x).max(1.0);
        Self {
            id: id.into(),
            name: name.into(),
            pos: start_pos,
            size,
            motion: MovingPlatformMotion::SideLoop {
                min_x,
                max_x,
                speed: speed.max(0.0),
                dir: if rightward { 1.0 } else { -1.0 },
                base_y: start_pos.y,
                bob,
                bob_t: ((start_pos.x - min_x) / span).rem_euclid(1.0) * BOB_PERIOD_S,
            },
            last_delta: ae::Vec2::ZERO,
            visual: None,
            one_way: false,
        }
    }

    /// A lift resting at `start_pos` that carries a rider `dy` at `speed`.
    pub fn lift(
        id: impl Into<String>,
        name: impl Into<String>,
        start_pos: ae::Vec2,
        size: ae::Vec2,
        dy: f32,
        speed: f32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            pos: start_pos,
            size,
            motion: MovingPlatformMotion::Lift {
                rest_y: start_pos.y,
                end_y: start_pos.y + dy,
                speed: speed.max(1.0),
                ridden: false,
                stage: LiftStage::Resting,
            },
            last_delta: ae::Vec2::ZERO,
            visual: None,
            one_way: false,
        }
    }

    /// A rescue carrier, spawned under a falling body at `start_pos`: it
    /// rises to `rise_to_y` at `rise_speed`, then flies at `drift` (px/s,
    /// signed) until it is past `gone_x`, where it is spent
    /// ([`Self::is_spent`]).
    #[allow(clippy::too_many_arguments)]
    pub fn ferry(
        id: impl Into<String>,
        name: impl Into<String>,
        start_pos: ae::Vec2,
        size: ae::Vec2,
        rise_to_y: f32,
        rise_speed: f32,
        drift: f32,
        gone_x: f32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            pos: start_pos,
            size,
            motion: MovingPlatformMotion::Ferry {
                rise_to_y,
                rise_speed: rise_speed.max(1.0),
                drift,
                gone_x,
            },
            last_delta: ae::Vec2::ZERO,
            visual: None,
            one_way: false,
        }
    }

    /// Draw it as `visual` (a registered prop sheet kind).
    pub fn with_visual(mut self, visual: impl Into<String>) -> Self {
        self.visual = Some(visual.into());
        self
    }

    /// Make it one-way: a body jumps up through it and lands on top.
    pub fn one_way(mut self) -> Self {
        self.one_way = true;
        self
    }

    /// Tell a lift whether a body stands on it. The advance asks before each
    /// [`Self::update`]; any other motion ignores it.
    pub fn set_ridden(&mut self, on: bool) {
        if let MovingPlatformMotion::Lift { ridden, .. } = &mut self.motion {
            *ridden = on;
        }
    }

    /// Whether this is a lift (so the advance asks whether it is ridden).
    pub fn is_lift(&self) -> bool {
        matches!(self.motion, MovingPlatformMotion::Lift { .. })
    }

    /// Where a lift is in its round trip; `None` for any other motion.
    pub fn lift_stage(&self) -> Option<LiftStage> {
        match self.motion {
            MovingPlatformMotion::Lift { stage, .. } => Some(stage),
            _ => None,
        }
    }

    /// A ferry that has flown past its end: its rescue is over and whoever
    /// launched it takes it away.
    pub fn is_spent(&self) -> bool {
        match self.motion {
            MovingPlatformMotion::Ferry { drift, gone_x, rise_to_y, .. } => {
                self.pos.y <= rise_to_y + 0.5
                    && if drift < 0.0 { self.pos.x < gone_x } else { self.pos.x > gone_x }
            }
            _ => false,
        }
    }

    /// Which way it flies sideways, when it does: `-1.0` left, `1.0` right.
    /// `None` for a motion with no sideways heading (a lift, a shaft, a path).
    pub fn heading_x(&self) -> Option<f32> {
        match &self.motion {
            MovingPlatformMotion::Sweep { dir, .. } | MovingPlatformMotion::SideLoop { dir, .. } => Some(*dir),
            MovingPlatformMotion::Ferry { drift, .. } if *drift != 0.0 => Some(drift.signum()),
            _ => None,
        }
    }

    /// Whether it is a rescue ferry.
    pub fn is_ferry(&self) -> bool {
        matches!(self.motion, MovingPlatformMotion::Ferry { .. })
    }

    /// The lane a horizontally-LOOPING platform runs in, as `(min_x, max_x)`.
    pub fn horizontal_loop_span(&self) -> Option<(f32, f32)> {
        match self.motion {
            MovingPlatformMotion::SideLoop { min_x, max_x, .. } => Some((min_x, max_x)),
            _ => None,
        }
    }

    /// Displacement applied by the most recent [`Self::update`] advance. Read by
    /// the per-entity player tick (platform-ride / ledge-carry) so the advance can
    /// run once per frame instead of being interleaved with per-body logic.
    pub fn last_delta(&self) -> ae::Vec2 {
        self.last_delta
    }

    /// Advance the platform and return its displacement this frame. Also records
    /// it as [`Self::last_delta`] for readers that run after the advance.
    pub fn update(&mut self, dt: f32) -> ae::Vec2 {
        let old = self.pos;
        // A wrap is a position change that is not movement, and `last_delta`
        // carries the rider. The wrapping arm must report its real travel, or
        // `pos - old` gives the rider the whole span in one frame, in the
        // wrong direction. Reversing arms move continuously, so their position
        // difference is their travel.
        let mut carried: Option<ae::Vec2> = None;
        match &mut self.motion {
            MovingPlatformMotion::Sweep {
                min_x,
                max_x,
                speed,
                dir,
            } => {
                self.pos.x += *speed * *dir * dt;
                if self.pos.x > *max_x {
                    self.pos.x = *max_x;
                    *dir = -1.0;
                } else if self.pos.x < *min_x {
                    self.pos.x = *min_x;
                    *dir = 1.0;
                }
            }
            MovingPlatformMotion::Path { path, segment, dir } => {
                self.pos = advance_path_position(path, segment, dir, self.pos, dt);
            }
            MovingPlatformMotion::Loop {
                min_y,
                max_y,
                speed,
                dir,
            } => {
                let step = ae::Vec2::new(0.0, *speed * *dir * dt);
                self.pos += step;
                let span = *max_y - *min_y;
                if span > 0.0 {
                    if self.pos.y > *max_y {
                        self.pos.y -= span;
                    } else if self.pos.y < *min_y {
                        self.pos.y += span;
                    }
                }
                // The TRAVEL, never the teleport.
                carried = Some(step);
            }
            MovingPlatformMotion::SideLoop {
                min_x,
                max_x,
                speed,
                dir,
                base_y,
                bob,
                bob_t,
            } => {
                *bob_t = (*bob_t + dt).rem_euclid(BOB_PERIOD_S);
                let y = *base_y + *bob * (std::f32::consts::TAU * *bob_t / BOB_PERIOD_S).sin();
                let step = ae::Vec2::new(*speed * *dir * dt, y - self.pos.y);
                self.pos += step;
                let span = *max_x - *min_x;
                if span > 0.0 {
                    if self.pos.x > *max_x {
                        self.pos.x -= span;
                    } else if self.pos.x < *min_x {
                        self.pos.x += span;
                    }
                }
                carried = Some(step);
            }
            MovingPlatformMotion::Lift {
                rest_y,
                end_y,
                speed,
                ridden,
                stage,
            } => {
                let toward = |from: f32, to: f32, max: f32| from + (to - from).clamp(-max, max);
                match stage {
                    LiftStage::Resting => {
                        self.pos.y = *rest_y;
                        if *ridden {
                            *stage = LiftStage::Carrying;
                        }
                    }
                    LiftStage::Carrying => {
                        self.pos.y = toward(self.pos.y, *end_y, *speed * dt);
                        if (self.pos.y - *end_y).abs() < 0.5 {
                            *stage = LiftStage::Returning;
                        }
                    }
                    LiftStage::Returning => {
                        // Back the way it went, a little slower: it swoops
                        // down to wait again.
                        self.pos.y = toward(self.pos.y, *rest_y, *speed * 0.8 * dt);
                        if (self.pos.y - *rest_y).abs() < 0.5 {
                            self.pos.y = *rest_y;
                            *stage = LiftStage::Resting;
                        }
                    }
                }
            }
            MovingPlatformMotion::Ferry {
                rise_to_y,
                rise_speed,
                drift,
                ..
            } => {
                if self.pos.y > *rise_to_y {
                    // Rising, easing in to its height over its last 80 px.
                    let left = self.pos.y - *rise_to_y;
                    let v = (*rise_speed * (left / 80.0).clamp(0.25, 1.0)).min(left / dt.max(1e-4));
                    self.pos.y -= v * dt;
                } else {
                    self.pos.x += *drift * dt;
                }
            }
        }
        self.last_delta = carried.unwrap_or(self.pos - old);
        self.last_delta
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }

    /// The shaft a vertically-LOOPING platform runs in, as `(min_y, max_y)`.
    ///
    /// `None` for every other motion — a sweep and a path REVERSE at their limits, which is
    /// visible on purpose.
    pub fn vertical_loop_span(&self) -> Option<(f32, f32)> {
        match self.motion {
            MovingPlatformMotion::Loop { min_y, max_y, .. } => Some((min_y, max_y)),
            _ => None,
        }
    }

    /// Direction of travel, +1 or -1. For path-driven platforms this reports
    /// the playback direction (not a local tangent sign), which is enough for
    /// trace/HUD readers that want to surface motion phase.
    pub fn direction(&self) -> f32 {
        match &self.motion {
            MovingPlatformMotion::Sweep { dir, .. } => *dir,
            MovingPlatformMotion::Path { dir, .. } => *dir as f32,
            MovingPlatformMotion::Loop { dir, .. } => *dir,
            MovingPlatformMotion::SideLoop { dir, .. } => *dir,
            MovingPlatformMotion::Lift { stage, .. } => {
                if *stage == LiftStage::Returning {
                    -1.0
                } else {
                    1.0
                }
            }
            MovingPlatformMotion::Ferry { drift, .. } => drift.signum(),
        }
    }

    /// The collision face this platform presents this frame: a two-axis `BlinkWall{Soft}`
    /// solid, or a `OneWay` surface when authored so. The one-way landing rule compares a
    /// body's previous feet with the CURRENT support face, which is frame-consistent only
    /// while the face moves less than `ONE_WAY_CROSSING_SLOP` a tick: a one-way platform's
    /// climb is held under [`MAX_ONE_WAY_CLIMB`] when it is resolved.
    pub fn as_collision_block(&self) -> ae::Block {
        ae::Block {
            // The platform's LDtk iid IS its durable identity (§3.6
            // `GeoSource::Placement`) — the CC6 portal host ref resolves
            // moving hosts through it per frame.
            id: ae::GeoId::placement(ae::PlacementId::new(self.id.clone()), 0),
            name: self.name.clone(),
            aabb: self.aabb(),
            // This frame's displacement — the collision sweep carries any body
            // resting on this platform by it, so riding is emergent + uniform.
            velocity: self.last_delta,
            // Moving platforms are ordinary solids for walking/riding because
            // `BlockKind::BlinkWall` still resolves as solid collision on both
            // axes. They are deliberately *not* hard blink blockers: if the
            // player has the soft blink-through upgrade, blink pathing may pass
            // through the moving platform just like a soft blink membrane.
            kind: if self.one_way {
                ae::BlockKind::OneWay
            } else {
                ae::BlockKind::BlinkWall {
                    tier: ae::BlinkWallTier::Soft,
                }
            },
            art_color: None,
        }
    }

    /// The platform AABB before the latest [`Self::update`] displacement.
    ///
    /// Moving platforms advance once near the beginning of the frame, before the
    /// per-body simulation phase, so a contact fact stored last tick still
    /// describes where the platform WAS. [`Self::is_supporting_body`] matches
    /// both poses for that reason.
    pub fn previous_aabb(&self) -> ae::Aabb {
        self.aabb().translated(-self.last_delta)
    }

    /// Detect whether a body is supported by this platform under the active
    /// acceleration frame.
    ///
    /// `on_ground` remains a relative term: the caller has already decided that
    /// the body's feet are supported this frame. This helper answers whether this
    /// moving platform is the support by comparing the body's feet face to the
    /// platform's anti-feet/head face in side/down coordinates.
    pub fn is_supporting_body(
        &self,
        body: ae::Aabb,
        on_ground: bool,
        gravity_dir: ae::Vec2,
    ) -> bool {
        if !on_ground {
            return false;
        }
        support_contact_matches(body, self.aabb(), gravity_dir)
            || support_contact_matches(body, self.previous_aabb(), gravity_dir)
    }

}

fn projected_half(half: ae::Vec2, axis: ae::Vec2) -> f32 {
    half.x * axis.x.abs() + half.y * axis.y.abs()
}

fn side_overlap_len(a: ae::Aabb, b: ae::Aabb, frame: ae::AccelerationFrame) -> f32 {
    let a_center = a.center().dot(frame.side);
    let b_center = b.center().dot(frame.side);
    let a_half = projected_half(a.half_size(), frame.side);
    let b_half = projected_half(b.half_size(), frame.side);
    (a_center + a_half).min(b_center + b_half) - (a_center - a_half).max(b_center - b_half)
}

fn support_contact_matches(body: ae::Aabb, support: ae::Aabb, gravity_dir: ae::Vec2) -> bool {
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let overlap = side_overlap_len(body, support, frame);
    if overlap <= 3.0 {
        return false;
    }
    let body_down = body.center().dot(frame.down);
    let support_down = support.center().dot(frame.down);
    let body_feet = body_down + projected_half(body.half_size(), frame.down);
    let support_head = support_down - projected_half(support.half_size(), frame.down);
    (body_feet - support_head).abs() <= 6.0
}

fn advance_path_position(
    path: &ambition_platformer2d_core::KinematicPath,
    segment: &mut usize,
    dir: &mut i32,
    mut pos: ae::Vec2,
    dt: f32,
) -> ae::Vec2 {
    if !path.is_valid() || dt <= 0.0 {
        return pos;
    }
    let mut remaining = path.speed * dt;
    while remaining > 0.0 {
        let target_index = path_target_index(path, *segment, *dir);
        let Some(target) = path.points.get(target_index).copied() else {
            break;
        };
        let to_target = target - pos;
        let distance = to_target.length();
        if distance <= 0.001 {
            // This branch consumes no `remaining`; if advancing leaves the cursor
            // unchanged, stop rather than spinning forever on a zero-distance
            // target.
            let before = (*segment, *dir);
            advance_path_segment(path, segment, dir);
            if (*segment, *dir) == before {
                break;
            }
            continue;
        }
        let step = remaining.min(distance);
        pos += to_target / distance * step;
        remaining -= step;
        if step >= distance - 0.001 {
            advance_path_segment(path, segment, dir);
        }
    }
    pos
}

/// The waypoint a cursor is heading for.
///
/// `Loop` closes the circuit: a path of `n` points has `n` segments, including
/// the closing leg `p[n-1] → p[0]`.
///
/// Reverse (`dir < 0`) is not wrapped: under `Loop` nothing sets a backwards
/// direction (only `PingPong` flips `dir`).
fn path_target_index(
    path: &ambition_platformer2d_core::KinematicPath,
    segment: usize,
    dir: i32,
) -> usize {
    if dir >= 0 {
        let next = segment + 1;
        if matches!(
            path.mode,
            ambition_platformer2d_core::KinematicPathMode::Loop
        ) {
            return next % path.points.len().max(1);
        }
        next
    } else {
        segment
    }
}

/// The highest segment index this mode may occupy.
///
/// `Loop` has one more than the others: the closing leg back to the first point.
fn path_last_segment(path: &ambition_platformer2d_core::KinematicPath) -> usize {
    match path.mode {
        ambition_platformer2d_core::KinematicPathMode::Loop => path.points.len().saturating_sub(1),
        _ => path.points.len().saturating_sub(2),
    }
}

fn advance_path_segment(
    path: &ambition_platformer2d_core::KinematicPath,
    segment: &mut usize,
    dir: &mut i32,
) {
    let last_segment = path_last_segment(path);
    match path.mode {
        ambition_platformer2d_core::KinematicPathMode::Once => {
            if *dir >= 0 && *segment < last_segment {
                *segment += 1;
            }
        }
        ambition_platformer2d_core::KinematicPathMode::Loop => {
            if *dir >= 0 {
                *segment = if *segment >= last_segment {
                    0
                } else {
                    *segment + 1
                };
            } else if *segment == 0 {
                *segment = last_segment;
            } else {
                *segment -= 1;
            }
        }
        ambition_platformer2d_core::KinematicPathMode::PingPong => {
            if *dir >= 0 {
                if *segment >= last_segment {
                    *dir = -1;
                } else {
                    *segment += 1;
                }
            } else if *segment == 0 {
                *dir = 1;
            } else {
                *segment -= 1;
            }
        }
    }
}

/// Return the active room's LDtk-authored moving platforms.
///
/// No compatibility platform is synthesized here: if an active area has no
/// `MovingPlatform` entities, the room has no moving platforms. That keeps LDtk
/// as the sole gameplay source of truth for platform placement.
pub fn moving_platforms_for_room(room: &crate::rooms::RoomSpec) -> Vec<MovingPlatformState> {
    room.moving_platforms.clone()
}

/// Return a temporary collision world with all current moving platforms inserted.
///
/// The inserted blocks are solid for normal collision, but blink-passable for
/// upgraded blink pathing. This keeps debug previews, blink destination
/// resolution, and actual movement collision in agreement.
pub fn world_with_moving_platforms(
    world: &ae::World,
    platforms: &[MovingPlatformState],
) -> ae::World {
    let mut collision_world = world.clone();
    collision_world.blocks.extend(
        platforms
            .iter()
            .map(MovingPlatformState::as_collision_block),
    );
    collision_world
}

#[cfg(test)]
mod tests;
