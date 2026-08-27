//! Lower fighter perception and body capabilities into the shared movement-kernel
//! recovery probe.
//!
//! The probe searches the buttons-only baseline and each authored recovery route,
//! returning whether this steering policy found a route within its bounded horizon.
//! A negative result is not proof that the body cannot recover; movement external to
//! the kernel is outside this model. Terrain is limited to perceived geometry and the
//! stage envelope uses zero blast margins, so the probe does not plan with information
//! the fighter cannot observe.
use ambition_platformer2d_core as ae;

use ambition_characters::perception::{SolidKind, WorldView};

/// How long the probe watches.
pub const RECOVERY_PROBE_SECONDS: f32 = 2.0;

/// Body capabilities passed directly to the movement recovery kernel.
/// Routes are kept separate: the kit states what the body can do, while a route
/// states one authored action to probe from the current position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyKit {
    pub abilities: ae::AbilitySet,
    pub movement: ae::MovementTuning,
}

/// One displacement a body's repertoire can command, in the terms a
/// [`RecoveryProbe`](ae::movement::recovery::RecoveryProbe) needs.
///
/// Derived from a move's frame data by
/// [`lifting_candidates`](ambition_characters::brain::fighter::options::lifting_candidates); nothing here
/// names a move, a verb or a character.
///
/// a route is a PROPOSAL, never a claim. Holding one says the body can
/// throw this displacement, and says nothing about whether throwing it helps —
/// that question belongs to [`RecoveryLens::best_route`], which asks the kernel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecoveryLift {
    /// Engine units per second, against gravity.
    pub speed: f32,
    /// Engine units per second along the body's FACING, the other half of
    /// the same commanded velocity (`MoveFrameData::lift_side`). Negative for a
    /// move that hauls its owner backwards — the recoil of firing forwards is a
    /// real authored shape.
    ///
    /// zero for a straight-up recovery, which is why every seat that had one is unaffected. A
    /// `Set` impulse overrides the drift outright, so a probe that dropped this half searched a
    /// move the body cannot throw.
    pub side: f32,
    /// Proper-time seconds from the press to the burst — the windup the body has
    /// to survive first. A probe that ignored it would certify recoveries the
    /// real move is too slow to make.
    pub after_s: f32,
}

/// Where a rolled line left the ground, in the terms the kernel needs to take it
/// from there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecoveryQuery {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    /// Mid-air jumps this line has NOT spent. A line that burned its jump
    /// getting into trouble is asked about the body it left behind, not the one
    /// it started as.
    pub air_jumps_left: u8,
}

/// How many authored routes one lens will spend kernel time on.
///
/// a bound, not a taste. Each route is a whole `probe_recovery` — three
/// efforts of up to [`RECOVERY_PROBE_SECONDS`] — and the lens is queried once
/// per rolled movement line, so the cost is linear in this number. Three plus
/// the buttons-only baseline is four searches per query, which is what a body
/// with a real repertoire needs (a recovery, a stall, and one more) and far
/// short of the dozen a full kit would offer.
///
/// the routes are ordered by [`ambition_characters::brain::fighter::options::lifting_candidates`] and the
/// cut is a prefix of that order, so which three get probed never depends on
/// iteration luck (ADR 0023).
pub const MAX_PROBED_ROUTES: usize = 3;

/// Which route got the body home, and what the search saw.
///
/// `route` is the whole product: `None` with a positive outlook means *"you
/// are getting back without throwing anything"*, `Some(i)` names the route that
/// worked, and `None` with a negative means no search this lens ran found
/// anything. Those are three different instructions to a caller and the old
/// boolean could express one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RouteVerdict {
    pub outlook: ae::movement::recovery::RecoveryOutlook,
    /// Index into the routes this lens was built with.
    pub route: Option<usize>,
}

impl RouteVerdict {
    pub fn regained(self) -> bool {
        self.outlook.regained()
    }
}

/// A perceived stage plus a body's kit and the routes it could press, ready to
/// answer recovery questions.
///
/// Built ONCE per decision and queried per rolled line: the world lowering is
/// the expensive half (it allocates one [`ae::Block`] per perceived solid) and
/// it does not change between the lines of a single decision.
#[derive(Clone, Debug)]
pub struct RecoveryLens {
    world: ae::World,
    /// The stage box's own origin, subtracted from everything. `ae::World` is a
    /// `0..size` box while a [`ambition_characters::perception::StageView`] is an AABB
    /// anywhere, so the lowering TRANSLATES rather than assuming the two agree.
    /// Translation changes no distance, no velocity and no gravity direction, so
    /// the verdict is unchanged by it.
    origin: ae::Vec2,
    kit: BodyKit,
    frame: ae::MotionFrame,
    body_size: ae::Vec2,
    /// The buttons-only search. Every armed search is this one plus a burst, so
    /// the horizon and timestep can never differ between routes.
    probe: ae::movement::recovery::RecoveryProbe,
    /// the displacements this body could throw, in probe order. Capped at
    /// [`MAX_PROBED_ROUTES`].
    routes: Vec<RecoveryLift>,
}

impl RecoveryLens {
    /// Lower a view, a kit and a set of routes into something
    /// [`ae::movement::recovery::probe_recovery`] can be run against.
    ///
    /// `None` when the view names no stage (there is no envelope to fall out of),
    /// when the stage is degenerate, or when the body's gravity is zero — a
    /// free-mover has no fall to recover from and the frame cannot be built.
    ///
    /// `routes` may be empty, and for every seat that is not a platform
    /// fighter it is. An empty set leaves the lens exactly as it was before any
    /// of this existed: one buttons-only search, bounded by
    /// [`ae::movement::recovery::RecoveryPolicy::DRIFT_AND_JUMP`].
    pub fn from_view(
        view: &WorldView,
        kit: BodyKit,
        routes: &[RecoveryLift],
        dt: f32,
    ) -> Option<Self> {
        let stage = view.stage;
        if !stage.is_known() {
            return None;
        }
        let size = stage.bounds.max - stage.bounds.min;
        if !(size.x > 0.0 && size.y > 0.0) {
            return None;
        }
        let down = view.self_view.gravity_down.normalize_or_zero();
        let frame = ae::MotionFrame::from_acceleration(down * kit.movement.gravity)?;
        let origin = stage.bounds.min;
        // Ordered exactly as the view lists them, which is the order the room's
        // own `world.blocks` were in (ADR 0023: no iteration luck).
        let blocks = view
            .terrain
            .iter()
            .map(|solid| {
                let min = solid.aabb.min - origin;
                let extent = solid.aabb.max - solid.aabb.min;
                match solid.kind {
                    // A blink wall is full collision to a body that cannot blink
                    // through it, and whether it CAN is the kernel's call, not
                    // this lowering's — perception has already distilled the tier
                    // away, so the conservative solid is what is left.
                    SolidKind::Solid | SolidKind::BlinkWall => {
                        ae::Block::solid("perceived", min, extent)
                    }
                    SolidKind::OneWay => ae::Block::one_way("perceived", min, extent),
                    SolidKind::Hazard => ae::Block::hazard("perceived", min, extent),
                }
            })
            .collect();
        Some(Self {
            world: ae::World::new("perceived stage", size, ae::Vec2::ZERO, blocks)
                // the envelope IS the death line — see the module header. One
                // model of dying, shared with `StageView::offstage`.
                .with_fall_out_margin(0.0)
                .with_side_out_margin(0.0)
                .with_rise_out_margin(0.0),
            origin,
            kit,
            frame,
            body_size: view.self_view.half_extent * 2.0,
            probe: ae::movement::recovery::RecoveryProbe::seconds(RECOVERY_PROBE_SECONDS, dt),
            routes: routes.iter().take(MAX_PROBED_ROUTES).copied().collect(),
        })
    }

    /// The body this lens probes, as this line left it.
    fn scratch(&self, at: RecoveryQuery) -> ae::BodyClusterScratch {
        // the velocity is said at CONSTRUCTION, not patched on after: the whole
        // point of this scratch is "the body as this line left it", and its speed
        // is part of that state rather than a change to it.
        let mut body =
            ae::BodyClusterScratch::new_with_abilities(at.pos - self.origin, self.kit.abilities)
                .with_velocity(at.vel);
        // The body's OWN movement law, not the engine default a scratch body is
        // born with. A character that authors its own gravity, air accel or jump
        // is probed as itself.
        body.model = ae::MotionModel::axis_swept(self.kit.movement.axis_swept_params());
        body.kinematics.size = self.body_size;
        body.base_size.base_size = self.body_size;
        body.ground.on_ground = false;
        // NOT a refresh. `new_with_abilities` hands out a full air-jump budget
        // and the question is about the body as this line left it — a line that
        // already spent its jump must be probed without it.
        body.jump.air_jumps_available = at.air_jumps_left;
        body
    }

    /// This lens's search, armed with one route's displacement.
    fn armed(&self, route: RecoveryLift) -> ae::movement::recovery::RecoveryProbe {
        self.probe.with_policy(
            ae::movement::recovery::RecoveryPolicy::drift_jump_and_burst(
                ae::movement::recovery::RecoveryBurst {
                    // Body-local: `+y` is toward the feet, so a lift is negative,
                    // and `+x` is toward the facing the effort is steering.
                    local: ae::Vec2::new(route.side, -route.speed),
                    // The windup, in kernel steps of THIS probe.
                    at_step: (route.after_s / self.probe.dt.max(f32::EPSILON))
                        .round()
                        .max(0.0) as usize,
                },
            ),
        )
    }

    /// ASK THE KERNEL WHICH OF THIS BODY'S ACTIONS IS USEFUL FROM HERE.
    ///
    /// The buttons-only search first, then each route in order, stopping at the
    /// first one that regains support.
    ///
    /// That makes a scalar into a recovery ontology: any move with a positive lift is *the way
    /// home*, a tiny rising aerial outranks a grapple that trades its energy for distance, and the
    /// route that would actually have worked is never explored. Usefulness is a question about the
    /// CURRENT STATE and the only authority on it is the movement kernel, so the kernel is asked.
    ///
    /// the buttons-only baseline goes first on purpose. A body that is
    /// already getting home does not need to throw anything, and a caller told
    /// so can save the move — which is a real fighting-game fact (spending your
    /// recovery early is how you die to an edgeguard) that fell out of the
    /// ordering rather than being encoded.
    ///
    /// the negative belongs to the LAST search run. When nothing regains,
    /// the outlook returned is the widest search's, and its
    /// `NoSupportFoundBy { search, .. }` names that policy — so a consumer
    /// reading `bounded_by()` learns what the final attempt spent, not what
    /// every attempt spent. Still a bounded claim, and now bounded by a strictly
    /// wider search than before.
    pub fn best_route(&self, at: RecoveryQuery) -> RouteVerdict {
        let body = self.scratch(at);
        let mut last =
            ae::movement::recovery::probe_recovery(&self.world, &body, self.frame, self.probe);
        if last.regained() {
            return RouteVerdict {
                outlook: last,
                route: None,
            };
        }
        for (index, route) in self.routes.iter().enumerate() {
            last = ae::movement::recovery::probe_recovery(
                &self.world,
                &body,
                self.frame,
                self.armed(*route),
            );
            if last.regained() {
                return RouteVerdict {
                    outlook: last,
                    route: Some(index),
                };
            }
        }
        RouteVerdict {
            outlook: last,
            route: None,
        }
    }

    /// Drive the real kernel from `at` and report what it found, over every
    /// route this body owns.
    pub fn outlook(&self, at: RecoveryQuery) -> ae::movement::recovery::RecoveryOutlook {
        self.best_route(at).outlook
    }

    /// [`Self::outlook`], reduced to the one bit the veto spends.
    pub fn regains_support(&self, at: RecoveryQuery) -> bool {
        self.best_route(at).regained()
    }
}

#[cfg(test)]
mod tests;
