use ae::navigation::{follow_flight, FlightFacts, FLIGHT_ARRIVE};
use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::brain::{
    fighter::data::{FighterState, SustainedRecovery},
    BrainSnapshot,
};
use ambition_characters::perception::{SolidKind, WorldView};
use ambition_entity_catalog::AuthoredRecoveryRoute;
use ambition_platformer2d_core::{self as ae, Aabb, AabbExt, Vec2};

/// Current movement authority for a fighter's sustained recovery.
#[derive(Clone, Copy, Debug, Default)]
pub enum RecoveryControl {
    #[default]
    Absent,
    Reserved,
    Flight(ControlledFlight),
}

/// Live facts for the body that a fighter controls in flight.
#[derive(Clone, Copy, Debug)]
pub struct ControlledFlight {
    pub flight: FlightFacts,
    pub bounds: Aabb,
    pub rider_bounds: Aabb,
    pub command_speed: f32,
    pub steering_enabled: bool,
}

pub(super) fn recovery_input(
    state: &mut FighterState,
    snapshot: &BrainSnapshot,
    control: RecoveryControl,
) -> Option<ActorControlFrame> {
    use SustainedRecovery::{Acquiring, Guiding, Released};

    let accepted = snapshot
        .own_move
        .as_ref()
        .filter(|own| {
            matches!(
                own.spec.gates.recovery_route,
                Some(AuthoredRecoveryRoute::SustainedAuthority { .. })
            )
        })
        .map(|own| own.instance);
    let instance = match state.sustained_recovery {
        Some(Guiding(instance)) => {
            if let RecoveryControl::Flight(flight) = control {
                return Some(flight.recovery_input(
                    snapshot.actor_facing,
                    state.perception.perceive().as_deref(),
                ));
            }
            state.sustained_recovery = (accepted == Some(instance)).then_some(Released(instance));
            return None;
        }
        Some(Released(instance)) if accepted == Some(instance) => return None,
        Some(Acquiring(instance)) if !matches!(control, RecoveryControl::Absent) => Some(instance),
        _ => accepted,
    };
    state.sustained_recovery = instance.map(Acquiring);
    let instance = instance?;
    if let RecoveryControl::Flight(flight) = control {
        state.sustained_recovery = Some(Guiding(instance));
        return Some(flight.recovery_input(
            snapshot.actor_facing,
            state.perception.perceive().as_deref(),
        ));
    }
    let mut input = ActorControlFrame::neutral();
    input.facing = snapshot.actor_facing;
    Some(input)
}

impl ControlledFlight {
    fn recovery_input(self, facing: f32, view: Option<&WorldView>) -> ActorControlFrame {
        let mut input = ActorControlFrame::neutral();
        input.facing = facing;
        if let Some(view) = view.filter(|_| self.steering_enabled) {
            if let Some(route) = self.route_to_support(view, facing) {
                let (axes, _) = follow_flight(&route.points[..route.len], 1, &self.flight);
                input.velocity_target = ae::WorldVec2(
                    (self.flight.side * axes.axis + self.flight.down * axes.axis_down)
                        * self.command_speed,
                );
            }
        }
        // The lease releases the rider. A jump or burst can release it early.
        input
    }

    fn route_to_support(self, view: &WorldView, facing: f32) -> Option<Route> {
        let FlightFacts {
            center, side, down, ..
        } = self.flight;
        let rider = self.rider_bounds;
        let clearance = 2.0 * FLIGHT_ARRIVE;
        let offset = rider.center() - center;
        let death_line = view.death_line();
        let clear_leg = |from: Vec2, to: Vec2| {
            let shift = from - center;
            let delta = to - from;
            let inside = |point: Vec2| {
                point.cmpgt(death_line.min).all() && point.cmplt(death_line.max).all()
            };
            inside(to)
                && inside(to + offset)
                && view.terrain.iter().all(|solid| {
                    solid.kind == SolidKind::OneWay
                        || (self
                            .bounds
                            .translated(shift)
                            .sweep_hit(delta, solid.aabb)
                            .is_none()
                            && rider
                                .translated(shift)
                                .sweep_hit(delta, solid.aabb)
                                .is_none())
                })
        };

        view.terrain
            .iter()
            .filter(|solid| !solid.open_for_self && solid.kind != SolidKind::Hazard)
            .filter_map(|solid| {
                let floor = solid.aabb;
                let inset = rider.gravity_half(side) + clearance;
                let left = floor.head_coord(side) + inset;
                let right = floor.feet_coord(side) - inset;
                if left > right {
                    return None;
                }
                let across = rider.center().dot(side).clamp(left, right) - offset.dot(side);
                let pair_feet = rider.feet_coord(down).max(self.bounds.feet_coord(down));
                let height = floor.head_coord(down) - (pair_feet - center.dot(down)) - clearance;
                let goal = side * across + down * height;
                let rise = side * center.dot(side) + down * height;

                // Check the full path from the live pose of each body.
                let pair_low = rider.head_coord(side).min(self.bounds.head_coord(side));
                let pair_high = rider.feet_coord(side).max(self.bounds.feet_coord(side));
                let detour = |across: f32| {
                    Route::around(
                        center,
                        side * across + down * center.dot(down),
                        side * across + down * height,
                        goal,
                    )
                };
                let outside_left =
                    floor.head_coord(side) - (pair_high - center.dot(side)) - clearance;
                let outside_right =
                    floor.feet_coord(side) - (pair_low - center.dot(side)) + clearance;

                // A rider released at this point can fall onto the support.
                let released = rider.translated(goal - center);
                let fall = down * (floor.head_coord(down) - released.feet_coord(down));
                if view.terrain.iter().any(|solid| {
                    solid.kind == SolidKind::Hazard
                        && released.sweep_hit(fall, solid.aabb).is_some()
                }) {
                    return None;
                }

                [
                    Route::direct(center, goal),
                    Route::via(center, rise, goal),
                    detour(outside_left),
                    detour(outside_right),
                ]
                .into_iter()
                .filter(|route| {
                    route.points[..route.len]
                        .windows(2)
                        .all(|leg| clear_leg(leg[0], leg[1]))
                })
                .min_by(|a, b| a.compare(b, side * facing))
            })
            .min_by(|a, b| a.compare(b, side * facing))
    }
}

struct Route {
    points: [Vec2; 4],
    len: usize,
    distance: f32,
}

impl Route {
    fn direct(start: Vec2, goal: Vec2) -> Self {
        Self {
            points: [start, goal, goal, goal],
            len: 2,
            distance: start.distance(goal),
        }
    }

    fn via(start: Vec2, rise: Vec2, goal: Vec2) -> Self {
        Self {
            points: [start, rise, goal, goal],
            len: 3,
            distance: start.distance(rise) + rise.distance(goal),
        }
    }

    fn around(start: Vec2, side: Vec2, rise: Vec2, goal: Vec2) -> Self {
        Self {
            points: [start, side, rise, goal],
            len: 4,
            distance: start.distance(side) + side.distance(rise) + rise.distance(goal),
        }
    }

    fn compare(&self, other: &Self, forward: Vec2) -> std::cmp::Ordering {
        self.distance.total_cmp(&other.distance).then_with(|| {
            other.points[1]
                .dot(forward)
                .total_cmp(&self.points[1].dot(forward))
        })
    }
}
