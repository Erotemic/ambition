//! Attached loops: a full 360° loop a runner enters from a floor and leaves
//! onto the same floor, as one authored fact.
//!
//! The shape is the classic 2.5D one. A ramp rises off the floor to the loop's
//! bottom, the loop makes a complete revolution back to the same screen point,
//! a flat deck carries the rider across the loop mouth, and a runout descends
//! to the floor again. The repeated mouth point is a crossover, not an
//! intersection: per-segment depth lanes put the approach behind the rider and
//! the loop's lower shoulders and the runout in front, and a local junction
//! joins the two visits. Two cross-chain junctions join the route to the floor
//! at the ramp's foot and the runout's end, so holding toward the ramp takes
//! it and running straight keeps the floor.
//!
//! This used to be ~120 lines of one demo's Rust (`graft_loop_route`), because
//! nothing in the authoring vocabulary could say "a loop attached to that
//! floor". [`World::attach_loop`] is the one builder; the LDtk `SurfaceLoop`
//! noun reaches it through its `attach_to` field.

use crate::world::{SurfaceChain, SurfaceJunction, SurfacePort, World};
use crate::Vec2;

/// How finely an attached loop is sampled. The default is the resolution the
/// Sanic speedway's loop was tuned at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoopResolution {
    /// Segments on the rising ramp.
    pub ramp: usize,
    /// Segments on the full revolution.
    pub revolution: usize,
    /// Segments on the flat deck across the loop mouth.
    pub overpass: usize,
    /// Segments on the descent back to the floor.
    pub descent: usize,
    /// Revolution segments on each side of the mouth drawn IN FRONT of the
    /// rider (the loop's lower shoulders).
    pub front_per_side: usize,
}

impl LoopResolution {
    /// The resolution every attached loop is built at today.
    pub const STANDARD: Self = Self {
        ramp: 32,
        revolution: 128,
        overpass: 12,
        descent: 20,
        front_per_side: 22,
    };
}

impl Default for LoopResolution {
    fn default() -> Self {
        Self::STANDARD
    }
}

/// One attached loop, in world coordinates.
///
/// `ramp_start` and `runout_end` are points ON the floor the loop attaches to;
/// the loop's bottom sits `rise` above `ramp_start`, and its center is
/// `radius` above that, at `center_x`. The runout leaves the loop on a flat
/// deck at the loop-bottom height until `overpass_end_x`, clear of the loop's
/// right shoulder, and then descends to `runout_end`.
#[derive(Clone, Debug, PartialEq)]
pub struct AttachedLoop {
    pub name: String,
    pub ramp_start: Vec2,
    pub center_x: f32,
    pub radius: f32,
    pub rise: f32,
    pub overpass_end_x: f32,
    pub runout_end: Vec2,
    pub resolution: LoopResolution,
}

/// The route an [`AttachedLoop`] builds, before it is attached.
#[derive(Clone, Debug)]
pub struct LoopRoute {
    /// The open chain: ramp → revolution → overpass → descent, with depth
    /// lanes and the mouth junction already declared.
    pub chain: SurfaceChain,
    /// The loop's center.
    pub center: Vec2,
    /// Vertex where the ramp meets the revolution (first mouth visit).
    pub entry_vertex: usize,
    /// Vertex where the revolution returns to the mouth (second visit).
    pub closure_vertex: usize,
    /// Last vertex (the runout's end on the floor).
    pub exit_vertex: usize,
}

fn cubic_bezier(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let u = 1.0 - t;
    p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t)
}

impl AttachedLoop {
    /// The loop's center.
    pub fn center(&self) -> Vec2 {
        Vec2::new(self.center_x, self.ramp_start.y - self.rise - self.radius)
    }

    /// Refuse a shape the route cannot build without folding over itself.
    pub fn validate(&self) -> Result<(), String> {
        let name = &self.name;
        if self.radius <= 0.0 {
            return Err(format!("loop `{name}`: radius must be positive"));
        }
        if self.rise < 0.0 {
            return Err(format!(
                "loop `{name}`: its bottom is {:.0}px below the floor at its ramp (x={}); \
                 raise the circle onto the floor",
                -self.rise, self.ramp_start.x
            ));
        }
        if self.ramp_start.x >= self.center_x - self.radius * 0.5 {
            return Err(format!(
                "loop `{name}`: the ramp must start well left of the loop \
                 (ramp at x={}, loop center x={})",
                self.ramp_start.x, self.center_x
            ));
        }
        if self.overpass_end_x < self.center_x + self.radius {
            return Err(format!(
                "loop `{name}`: the deck must clear the loop's right shoulder \
                 (deck ends x={}, shoulder x={})",
                self.overpass_end_x,
                self.center_x + self.radius
            ));
        }
        if self.runout_end.x <= self.overpass_end_x {
            return Err(format!(
                "loop `{name}`: the runout must end right of the deck \
                 (runout x={}, deck ends x={})",
                self.runout_end.x, self.overpass_end_x
            ));
        }
        Ok(())
    }

    /// Build the route chain.
    pub fn route(&self) -> Result<LoopRoute, String> {
        self.validate()?;
        let r = self.resolution;
        let center = self.center();
        let start_angle = std::f32::consts::FRAC_PI_2;
        let loop_start = center + Vec2::new(start_angle.cos(), start_angle.sin()) * self.radius;

        let mut points = Vec::with_capacity(1 + r.ramp + r.revolution + r.overpass + r.descent);
        points.push(self.ramp_start);
        // Rise off the floor with a horizontal tangent at both ends, so the
        // floor→ramp seam and the ramp→loop seam are both smooth.
        let ramp_c1 = self.ramp_start + Vec2::new(150.0, 0.0);
        let ramp_c2 = loop_start - Vec2::new(170.0, 0.0);
        for step in 1..=r.ramp {
            let t = step as f32 / r.ramp as f32;
            points.push(cubic_bezier(self.ramp_start, ramp_c1, ramp_c2, loop_start, t));
        }
        let entry_vertex = points.len() - 1;
        // Decreasing angle gives the inward normals a rideable interior needs.
        // The last sample equals `loop_start`; it is not adjacent to the entry
        // sample, so no degenerate segment appears.
        for step in 1..=r.revolution {
            let t = step as f32 / r.revolution as f32;
            let theta = start_angle - std::f32::consts::TAU * t;
            points.push(center + Vec2::new(theta.cos(), theta.sin()) * self.radius);
        }
        let closure_vertex = points.len() - 1;
        // Cross the mouth on a flat deck. A fast rider may launch where a track
        // starts descending; doing that at the coincident mouth would let the
        // airborne circle re-hit the back rail.
        let overpass_end = Vec2::new(self.overpass_end_x, loop_start.y);
        for step in 1..=r.overpass {
            let t = step as f32 / r.overpass as f32;
            points.push(loop_start.lerp(overpass_end, t));
        }
        // Descend only after clearing the loop, with horizontal end tangents.
        let run_c1 = overpass_end + Vec2::new(120.0, 0.0);
        let run_c2 = self.runout_end - Vec2::new(160.0, 0.0);
        for step in 1..=r.descent {
            let t = step as f32 / r.descent as f32;
            points.push(cubic_bezier(overpass_end, run_c1, run_c2, self.runout_end, t));
        }
        let exit_vertex = points.len() - 1;

        let mut depths = vec![0_i8; points.len() - 1];
        depths[..r.ramp].fill(-1);
        let front = r.front_per_side.min(r.revolution / 2);
        depths[entry_vertex..entry_vertex + front].fill(1);
        depths[closure_vertex - front..].fill(1);

        let chain = SurfaceChain::open(self.name.clone(), points)
            .with_segment_depths(depths)
            .with_junctions(vec![SurfaceJunction::new(vec![entry_vertex, closure_vertex])]);
        Ok(LoopRoute {
            chain,
            center,
            entry_vertex,
            closure_vertex,
            exit_vertex,
        })
    }
}

impl World {
    /// Index of the surface chain named `name`.
    pub fn chain_named(&self, name: &str) -> Option<usize> {
        self.chains.iter().position(|chain| chain.name == name)
    }

    /// The point on open chain `chain` at world `x`, and the vertex there,
    /// splitting the segment that spans `x` when no vertex is within half a
    /// pixel. Every junction port naming a later vertex of that chain, and its
    /// depth lanes, are kept aligned.
    pub fn floor_vertex_at(&mut self, chain: usize, x: f32) -> Result<(usize, Vec2), String> {
        let points = &self.chains[chain].points;
        let name = self.chains[chain].name.clone();
        if let Some(index) = points.iter().position(|p| (p.x - x).abs() < 0.5) {
            return Ok((index, points[index]));
        }
        let segment = points
            .windows(2)
            .position(|pair| pair[0].x < x && x < pair[1].x)
            .ok_or_else(|| format!("chain `{name}` has no left→right segment spanning x={x}"))?;
        let (a, b) = (points[segment], points[segment + 1]);
        let point = a.lerp(b, (x - a.x) / (b.x - a.x));
        let inserted = segment + 1;
        self.chains[chain].points.insert(inserted, point);
        let lanes = &mut self.chains[chain].depth_lanes;
        if !lanes.is_empty() {
            let lane = lanes[segment];
            lanes.insert(segment, lane);
        }
        for (owner, other) in self.chains.iter_mut().enumerate() {
            for junction in &mut other.junctions {
                for port in &mut junction.ports {
                    match port {
                        SurfacePort::Local(vertex) if owner == chain && *vertex >= inserted => {
                            *vertex += 1
                        }
                        SurfacePort::Chain { chain: c, vertex } if *c == chain && *vertex >= inserted => {
                            *vertex += 1
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok((inserted, point))
    }

    /// Build an [`AttachedLoop`] on the floor chain named `floor` and join it
    /// to that floor at the ramp's foot and the runout's end.
    ///
    /// The loop is its circle (`center`, `radius`), stated where it is: an
    /// LDtk `SurfaceLoop`'s box. The floor's own height at `ramp_start_x` and
    /// `runout_end_x` places the two ends, and the ramp rises from the floor
    /// to the circle's bottom. A circle whose bottom is below the floor at the
    /// ramp is refused. Returns the loop's center.
    #[allow(clippy::too_many_arguments)]
    pub fn attach_loop(
        &mut self,
        name: &str,
        floor: &str,
        ramp_start_x: f32,
        center: Vec2,
        radius: f32,
        overpass_end_x: f32,
        runout_end_x: f32,
    ) -> Result<Vec2, String> {
        let floor_index = self
            .chain_named(floor)
            .ok_or_else(|| format!("loop `{name}` attaches to `{floor}`, which is not a chain"))?;
        let (_, ramp_start) = self.floor_vertex_at(floor_index, ramp_start_x)?;
        let (_, runout_end) = self.floor_vertex_at(floor_index, runout_end_x)?;
        // Found again after both splits: the second may have moved the first.
        let (ramp_vertex, _) = self.floor_vertex_at(floor_index, ramp_start_x)?;
        let (runout_vertex, _) = self.floor_vertex_at(floor_index, runout_end_x)?;
        let spec = AttachedLoop {
            name: name.to_string(),
            ramp_start,
            center_x: center.x,
            radius,
            // The ramp climbs from the floor to the circle's bottom.
            rise: ramp_start.y - (center.y + radius),
            overpass_end_x,
            runout_end,
            resolution: LoopResolution::default(),
        };
        let route = spec.route()?;
        let exit = route.exit_vertex;
        let mut chain = route.chain;
        chain.junctions.push(SurfaceJunction::across(vec![
            SurfacePort::local(0),
            SurfacePort::chain(floor_index, ramp_vertex),
        ]));
        chain.junctions.push(SurfaceJunction::across(vec![
            SurfacePort::local(exit),
            SurfacePort::chain(floor_index, runout_vertex),
        ]));
        let problems = chain.validate();
        if !problems.is_empty() {
            return Err(format!("loop `{name}`: {}", problems.join("; ")));
        }
        self.chains.push(chain);
        let problems = self.validate_surface_junctions();
        if !problems.is_empty() {
            return Err(format!("loop `{name}`: {}", problems.join("; ")));
        }
        Ok(route.center)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor_world(points: Vec<Vec2>) -> World {
        World::new("t", Vec2::new(4000.0, 1000.0), Vec2::ZERO, Vec::new())
            .with_chains(vec![SurfaceChain::open("floor", points)])
    }

    #[test]
    fn an_attached_loop_joins_the_floor_at_both_ends_without_hand_placed_vertices() {
        let mut world = floor_world(vec![Vec2::new(0.0, 600.0), Vec2::new(3000.0, 600.0)]);
        let center = world
            .attach_loop("loop", "floor", 500.0, Vec2::new(960.0, 336.0), 180.0, 1240.0, 1680.0)
            .expect("the loop attaches");
        assert_eq!(center, Vec2::new(960.0, 600.0 - 84.0 - 180.0));
        let floor = &world.chains[0];
        assert_eq!(
            floor.points,
            vec![
                Vec2::new(0.0, 600.0),
                Vec2::new(500.0, 600.0),
                Vec2::new(1680.0, 600.0),
                Vec2::new(3000.0, 600.0)
            ],
            "the floor was split at the ramp's foot and the runout's end"
        );
        let route = &world.chains[1];
        let ports: Vec<_> = route.junctions.iter().flat_map(|j| j.ports.clone()).collect();
        assert!(ports.contains(&SurfacePort::chain(0, 1)), "ramp foot joins floor vertex 1: {ports:?}");
        assert!(ports.contains(&SurfacePort::chain(0, 2)), "runout joins floor vertex 2: {ports:?}");
        assert!(world.validate_surface_junctions().is_empty());
    }

    #[test]
    fn a_second_loop_keeps_the_first_loops_ports_on_the_right_floor_vertices() {
        let mut world = floor_world(vec![Vec2::new(0.0, 600.0), Vec2::new(4000.0, 600.0)]);
        world
            .attach_loop("late", "floor", 2500.0, Vec2::new(2960.0, 336.0), 180.0, 3240.0, 3680.0)
            .unwrap();
        // Splitting the floor LEFT of the first loop shifts every later vertex.
        world
            .attach_loop("early", "floor", 300.0, Vec2::new(760.0, 336.0), 180.0, 1040.0, 1480.0)
            .unwrap();
        let late = &world.chains[world.chain_named("late").unwrap()];
        for port in late.junctions.iter().flat_map(|j| j.ports.iter()) {
            if let SurfacePort::Chain { chain, vertex } = port {
                let at = world.chains[*chain].points[*vertex];
                assert!(
                    at.x == 2500.0 || at.x == 3680.0,
                    "a port of the first loop now names floor vertex at {at:?}"
                );
            }
        }
        assert!(world.validate_surface_junctions().is_empty());
    }

    #[test]
    fn a_loop_that_cannot_clear_its_own_shoulder_is_refused() {
        let mut world = floor_world(vec![Vec2::new(0.0, 600.0), Vec2::new(3000.0, 600.0)]);
        let refused = world.attach_loop("loop", "floor", 500.0, Vec2::new(960.0, 336.0), 180.0, 1000.0, 1680.0);
        assert!(refused.is_err(), "the deck ends inside the loop: {refused:?}");
        assert_eq!(world.chains.len(), 1, "nothing was attached");
    }

    #[test]
    fn a_loop_is_its_circle_and_one_sunk_into_its_floor_is_refused() {
        let mut world = floor_world(vec![Vec2::new(0.0, 600.0), Vec2::new(3000.0, 600.0)]);
        let center = world
            .attach_loop("loop", "floor", 500.0, Vec2::new(960.0, 400.0), 180.0, 1240.0, 1680.0)
            .expect("a circle standing above its floor attaches");
        assert_eq!(center, Vec2::new(960.0, 400.0), "the loop is built where its circle is");
        let mut world = floor_world(vec![Vec2::new(0.0, 600.0), Vec2::new(3000.0, 600.0)]);
        let sunk = world.attach_loop("loop", "floor", 500.0, Vec2::new(960.0, 500.0), 180.0, 1240.0, 1680.0);
        assert!(sunk.unwrap_err().contains("below the floor"), "a circle whose bottom is under the floor");
    }
}
