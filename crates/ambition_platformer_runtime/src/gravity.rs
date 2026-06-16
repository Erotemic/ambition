//! Shared world physics applied to every actor body.
//!
//! [`GravityField`] is the world's gravity state (a redirectable down). The
//! goal of this module is that adding a new *global* force later — wind, a
//! tractor field, a gravity well — is a one-place change that reaches **every**
//! actor automatically:
//!
//! - **Free bodies** (thrown / ground items, projectiles) integrate through
//!   [`apply_world_forces`] — the single per-frame "apply global forces to a
//!   velocity" call. Add a force to `GravityField` + one line here and they all
//!   pick it up.
//! - **Collision-bound controllers** (the player, enemies) own bespoke swept-AABB
//!   integrators whose ground / jump logic is axis-based, so they consume
//!   [`GravityField::vertical_sign`] (which way is "down" along Y). They read the
//!   same `GravityField`, so a gravity flip moves them too.
//!
//! We deliberately keep this lightweight rather than reaching for a full
//! rigid-body engine: the platformer controllers are custom (parry2d swept
//! AABBs) for feel. **Avian2D remains available (ADR 0007)** for the day real
//! rigid-body physics — debris, ragdoll, stacked/complex collisions — is
//! genuinely needed; that's the escape hatch, not this seam.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// The world's gravity direction (unit vector, in the y-DOWN world frame) —
/// default straight down. Change `dir` and every actor reorients + falls the new
/// way: it's the gravity-room / gravity-effect hook. Set by e.g. the
/// `GravityFlipSwitch`; consumed by the player / enemy / item / projectile
/// integrators and by the portal orient-to-gravity roll.
#[derive(Resource, Clone, Copy, Debug)]
pub struct GravityField {
    pub dir: Vec2,
}

impl Default for GravityField {
    fn default() -> Self {
        // +Y is down in the world frame, so default gravity points +Y.
        Self {
            dir: Vec2::new(0.0, 1.0),
        }
    }
}

impl GravityField {
    /// Gravity acceleration vector for a body whose gravity magnitude is
    /// `magnitude` (px/s²). Used by free bodies that can fall in any direction.
    pub fn gravity_accel(&self, magnitude: f32) -> Vec2 {
        self.dir.normalize_or_zero() * magnitude
    }

    /// Sign of gravity along Y: `+1` = down (normal), `-1` = up (flipped). Used
    /// by the axis-based collision controllers (player / enemies).
    pub fn vertical_sign(&self) -> f32 {
        if self.dir.y >= 0.0 {
            1.0
        } else {
            -1.0
        }
    }
}

/// The room's **ambient** gravity — the default an actor falls under when it's
/// not inside any [`GravityZone`]. Flipped by the `GravityFlipSwitch` and
/// (later) authored per room. [`resolve_active_gravity`] copies this (or an
/// overlapping zone's direction) into the live [`GravityField`] each frame, so
/// the switch sets the ambient while zones override locally.
#[derive(Resource, Clone, Copy, Debug)]
pub struct BaseGravity {
    pub dir: Vec2,
}

impl Default for BaseGravity {
    fn default() -> Self {
        Self {
            dir: Vec2::new(0.0, 1.0),
        }
    }
}

impl BaseGravity {
    /// Step the ambient gravity to the next cardinal direction, cycling
    /// down → left → up → right → down. Shared by the `\` dev hotkey and the
    /// developer menu's Gravity row so both stay in lock-step. Engine y grows
    /// downward, so `+y` is screen-DOWN.
    pub fn cycle(&mut self) {
        self.dir = match (self.dir.x.round() as i32, self.dir.y.round() as i32) {
            (0, 1) => Vec2::new(-1.0, 0.0),  // down  -> left
            (-1, 0) => Vec2::new(0.0, -1.0), // left  -> up
            (0, -1) => Vec2::new(1.0, 0.0),  // up    -> right
            _ => Vec2::new(0.0, 1.0),        // right (or any) -> down
        };
    }

    /// Human-readable cardinal label for the current ambient direction, for the
    /// developer menu's Gravity row value.
    pub fn direction_label(&self) -> &'static str {
        match (self.dir.x.round() as i32, self.dir.y.round() as i32) {
            (0, 1) => "Down",
            (-1, 0) => "Left",
            (0, -1) => "Up",
            (1, 0) => "Right",
            _ => "Custom",
        }
    }
}

/// An authored region with its own gravity direction — the building block of a
/// "gravity room". Gravity is resolved **per body, by position**: any actor whose
/// center is inside the zone's `aabb` feels gravity along `dir` (and reorients via
/// the shared `ActorRoll`); outside every zone it falls under [`BaseGravity`]. So
/// an NPC standing in a gravity column feels the column even when the player is
/// elsewhere — see [`gravity_dir_at`].
#[derive(Component, Clone, Copy, Debug)]
pub struct GravityZone {
    /// World-space region (engine coords) the zone covers.
    pub aabb: ambition_engine_core::Aabb,
    /// Gravity direction inside the zone (e.g. `(0,-1)` = up).
    pub dir: Vec2,
}

/// Per-frame snapshot of every [`GravityZone`] in the world, so the many actor
/// integrators (enemies, NPCs, projectiles, items, the orient-to-gravity roll)
/// can resolve their **own** local gravity by position cheaply — reading one
/// resource instead of each taking a `Query<&GravityZone>`. Rebuilt by
/// [`collect_gravity_zones`].
#[derive(Resource, Default, Clone, Debug)]
pub struct GravityZones {
    /// `(region, gravity direction)` for each zone.
    pub zones: Vec<(ambition_engine_core::Aabb, Vec2)>,
}

/// Rebuild the [`GravityZones`] snapshot from the live zone components. Runs
/// before the actor integrators each frame.
pub fn collect_gravity_zones(mut snapshot: ResMut<GravityZones>, zones: Query<&GravityZone>) {
    snapshot.zones.clear();
    snapshot.zones.extend(zones.iter().map(|z| (z.aabb, z.dir)));
}

/// A [`GravityZone`] that slides horizontally — a "gravity column riding a moving
/// platform" (Jon's gravity TODO). Its region oscillates each frame, and because
/// the snapshot is rebuilt every frame, a body riding the column is carried with
/// it (localized gravity + motion).
#[derive(Component, Clone, Copy, Debug)]
pub struct OscillatingZone {
    /// Center the column oscillates around.
    pub base_center: Vec2,
    /// Half-extent of the column region (kept as the zone slides).
    pub half: Vec2,
    /// Horizontal sweep amplitude (px).
    pub amplitude_x: f32,
    /// Angular frequency (radians / second).
    pub freq: f32,
    /// Accumulated phase (radians); advanced by scaled dt so pause / bullet-time
    /// freeze the column too.
    pub phase: f32,
}

/// A [`GravityZone`] with a lifetime — spawned by a gravity grenade and despawned
/// when its timer runs out. Lets thrown grenades create short-lived gravity wells.
#[derive(Component, Clone, Copy, Debug)]
pub struct TemporaryZone {
    /// Seconds of life remaining; the zone despawns at zero.
    pub remaining: f32,
}

/// Tick down temporary gravity zones and despawn the expired ones. Uses scaled dt
/// so pause / bullet-time hold the well open.
pub fn tick_temporary_zones(
    time: Res<crate::time::SimDt>,
    mut commands: Commands,
    mut zones: Query<(Entity, &mut TemporaryZone)>,
) {
    let dt = time.get();
    for (entity, mut zone) in &mut zones {
        zone.remaining -= dt;
        if zone.remaining <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Slide each oscillating gravity zone before [`collect_gravity_zones`] snapshots
/// it, so the moved region is what the actor integrators read this frame.
pub fn oscillate_gravity_zones(
    time: Res<crate::time::SimDt>,
    mut zones: Query<(&mut GravityZone, &mut OscillatingZone)>,
) {
    for (mut zone, mut osc) in &mut zones {
        osc.phase += time.get() * osc.freq;
        let cx = osc.base_center.x + osc.phase.sin() * osc.amplitude_x;
        zone.aabb = ambition_engine_core::Aabb::new(Vec2::new(cx, osc.base_center.y), osc.half);
    }
}

/// The **localized** gravity direction for a body whose center is at `pos`: the
/// first [`GravityZone`] containing `pos`, else `base_dir` (the room ambient).
///
/// This is the heart of "gravity is local in space" — every non-player actor
/// resolves gravity from its **own** position through this, so a body inside a
/// gravity column feels the column independently of where the player is. (The
/// player resolves the same way via [`resolve_active_gravity`] into its
/// [`GravityField`].)
pub fn gravity_dir_at(pos: Vec2, zones: &GravityZones, base_dir: Vec2) -> Vec2 {
    for (aabb, dir) in &zones.zones {
        if pos.x >= aabb.min.x && pos.x <= aabb.max.x && pos.y >= aabb.min.y && pos.y <= aabb.max.y
        {
            return dir.normalize_or_zero();
        }
    }
    base_dir.normalize_or_zero()
}

/// Sign of the localized gravity along Y at `pos` (`+1` down / `-1` up) — the
/// per-body analogue of [`GravityField::vertical_sign`] for the axis-based
/// collision controllers (enemies, NPCs).
pub fn local_gravity_sign(pos: Vec2, zones: &GravityZones, base_dir: Vec2) -> f32 {
    if gravity_dir_at(pos, zones, base_dir).y >= 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// Snap a gravity direction to the nearest cardinal unit vector — down `(0,1)`,
/// up `(0,-1)`, right `(1,0)`, left `(-1,0)`. The player's wall-walking model is
/// cardinal so the AABB collision stays axis-aligned; a diagonal zone direction
/// resolves to whichever axis dominates (ties favour the vertical axis).
pub fn snap_cardinal(dir: Vec2) -> Vec2 {
    if dir == Vec2::ZERO {
        return Vec2::new(0.0, 1.0);
    }
    if dir.x.abs() > dir.y.abs() {
        Vec2::new(dir.x.signum(), 0.0)
    } else {
        Vec2::new(0.0, dir.y.signum())
    }
}

/// Sync a freshly-built [`MovementTuning`]'s gravity direction (and the legacy
/// `gravity_sign` scalar) from the live world gravity. EVERY system that builds
/// a tuning via `as_engine()` and then applies a gravity-relative impulse
/// (jump, pogo, wall-kick, knockback) MUST call this — otherwise the tuning
/// keeps its default `(0,1)` "down" and the impulse launches the wrong way
/// under a gravity flip. This is the single seam that keeps those mechanics
/// flipping together; `pass `gravity_field.map(|g| g.dir)`.
pub fn apply_gravity_dir(tuning: &mut ambition_engine_core::MovementTuning, gravity_dir: Vec2) {
    tuning.gravity_dir = snap_cardinal(gravity_dir);
    tuning.gravity_sign = if tuning.gravity_dir.y != 0.0 {
        tuning.gravity_dir.y.signum()
    } else {
        1.0
    };
}

/// One bundled system param for the world's gravity, so the many actor
/// integrators read gravity through a single argument (Bevy caps systems at 16
/// params) and resolve it **by position** — `sign_at`/`dir_at` give a body its
/// own localized gravity. All three resources are `Option` so headless/test apps
/// that don't insert them still get a sensible default (down).
#[derive(SystemParam)]
pub struct GravityCtx<'w> {
    /// The primary player's resolved gravity (used as the fallback when there
    /// are no zones, e.g. in tests).
    pub field: Option<Res<'w, GravityField>>,
    /// Snapshot of all gravity zones, for per-position resolution.
    pub zones: Option<Res<'w, GravityZones>>,
    /// Room ambient gravity (flipped by the global switch).
    pub base: Option<Res<'w, BaseGravity>>,
}

impl GravityCtx<'_> {
    fn base_dir(&self) -> Vec2 {
        self.base.as_deref().map_or(Vec2::new(0.0, 1.0), |b| b.dir)
    }

    /// The player's gravity direction (fallback when a body has no position).
    pub fn field_dir(&self) -> Vec2 {
        self.field.as_deref().map_or(Vec2::new(0.0, 1.0), |g| g.dir)
    }

    /// Localized gravity direction at `pos` (zone-or-ambient); falls back to the
    /// player's field if no zone snapshot is present.
    pub fn dir_at(&self, pos: Vec2) -> Vec2 {
        match self.zones.as_deref() {
            Some(zones) => gravity_dir_at(pos, zones, self.base_dir()),
            None => self.field_dir(),
        }
    }

    /// Localized gravity sign at `pos` (`+1` down / `-1` up).
    pub fn sign_at(&self, pos: Vec2) -> f32 {
        match self.zones.as_deref() {
            Some(zones) => local_gravity_sign(pos, zones, self.base_dir()),
            None => self.field.as_deref().map_or(1.0, |g| g.vertical_sign()),
        }
    }
}

/// Resolve the live [`GravityField`] each frame: it points along the first
/// [`GravityZone`] the player overlaps, else the room's [`BaseGravity`]. This is
/// the one writer of `GravityField.dir`, so zones and the ambient switch compose
/// cleanly (zone overrides ambient while inside). Reorientation is handled for
/// free by the shared `update_actor_roll`, which eases every body toward the new
/// gravity.
pub fn resolve_active_gravity(
    base: Option<Res<BaseGravity>>,
    zones: Query<&GravityZone>,
    bodies: Query<&crate::body::BodyKinematics, With<crate::body::PrimaryBody>>,
    mut gravity: ResMut<GravityField>,
) {
    use ambition_engine_core::AabbExt;
    let base_dir = base.map_or(Vec2::new(0.0, 1.0), |b| b.dir);
    let target = bodies
        .single()
        .ok()
        .and_then(|kin| {
            let body = ambition_engine_core::Aabb::new(kin.pos, kin.size * 0.5);
            zones
                .iter()
                .find(|z| body.strict_intersects(z.aabb))
                .map(|z| z.dir)
        })
        .unwrap_or(base_dir);
    gravity.dir = target.normalize_or_zero();
}

/// Apply the world's per-frame global forces to a free body's velocity. This is
/// the ONE place new global forces get added, so every caller (items,
/// projectiles, …) inherits them. Today it's just gravity; future forces go
/// right here.
pub fn apply_world_forces(vel: &mut Vec2, gravity_magnitude: f32, gravity: &GravityField, dt: f32) {
    *vel += gravity.gravity_accel(gravity_magnitude) * dt;
    // ── add new global forces here (wind, drag fields, gravity wells) ──
}

/// Render-space z-rotation that stands a body upright under `gravity_dir`: it
/// points the sprite's local +Y ("up") along world-up (`-gravity`), accounting
/// for the y-down→y-up render flip. Default gravity → angle 0. (Lives here with
/// the gravity state; consumed by the portal orient-to-gravity roll.)
pub fn gravity_upright_angle(gravity_dir: Vec2) -> f32 {
    let g = gravity_dir.normalize_or_zero();
    if g == Vec2::ZERO {
        return 0.0;
    }
    // World up = -g; the render frame flips y.
    let render_up = Vec2::new(-g.x, g.y);
    render_up.y.atan2(render_up.x) - std::f32::consts::FRAC_PI_2
}

/// Horizontal `flip_x` for a sprite that the gravity roll
/// ([`gravity_upright_angle`]) may rotate. The roll re-aims the rolled sprite
/// along the move-axis; whether that still matches `facing` inverts with the
/// gravity direction. UP gravity inverts (the ~180° roll mirrors horizontally) and
/// RIGHT gravity inverts (the 90° roll points the sprite opposite the down=right
/// move-axis) — without this the body "moves left but faces right" (the #33 bug,
/// first seen upside-down, then under sideways gravity). DOWN and LEFT keep the
/// normal `facing < 0` flip.
///
/// UPDATE (run now follows the control-frame `side`): the rolled sprite's natural
/// (unflipped) facing in screen space is `(g.y, -g.x)` — which is EXACTLY the
/// player-frame `side` axis the run moves along (`AccelerationFrame::side`). So
/// under any gravity where the `Hybrid` control frame keeps the player frame
/// (down / left / right, `g.y >= 0`), the rolled sprite already faces the run
/// direction and needs no inversion. Only UP gravity (`g.y < 0`), where `Hybrid`
/// screen-aligns the run past 90°, makes the move-axis oppose the rolled facing —
/// there `facing` inverts. The old `g.x > 0` term was an artifact of the previous
/// screen-down `move_axis` and is gone now. (Tracks `InputFrameMode::Hybrid`.)
pub fn gravity_aware_flip_x(facing: f32, gravity_dir: Vec2) -> bool {
    (facing < 0.0) ^ (gravity_dir.y < 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_aware_flip_inverts_only_upside_down() {
        let down = Vec2::new(0.0, 1.0);
        let up = Vec2::new(0.0, -1.0);
        let right = Vec2::new(1.0, 0.0);
        // Down: normal facing flip.
        assert!(
            gravity_aware_flip_x(-1.0, down),
            "facing left flips under down gravity"
        );
        assert!(!gravity_aware_flip_x(1.0, down));
        // Up: inverted (the #33 bug — moving left must not face right upside down).
        assert!(
            !gravity_aware_flip_x(-1.0, up),
            "facing left must NOT flip upside down"
        );
        assert!(gravity_aware_flip_x(1.0, up));
        // Sideways: now that the run follows the control-frame `side`, the rolled
        // sprite's natural facing IS the move direction, so BOTH walls keep the
        // normal `facing < 0` flip (no inversion). RIGHT gravity used to invert to
        // match the old screen-down move-axis; that artifact is gone.
        let left = Vec2::new(-1.0, 0.0);
        assert!(
            gravity_aware_flip_x(-1.0, right),
            "facing left under RIGHT gravity flips (normal) — run follows the rolled facing now"
        );
        assert!(!gravity_aware_flip_x(1.0, right));
        assert!(
            gravity_aware_flip_x(-1.0, left),
            "facing left under LEFT gravity flips (normal)"
        );
        assert!(!gravity_aware_flip_x(1.0, left));
    }

    #[test]
    fn vertical_sign_and_accel_track_the_direction() {
        let down = GravityField::default();
        assert_eq!(down.vertical_sign(), 1.0);
        assert_eq!(down.gravity_accel(1000.0), Vec2::new(0.0, 1000.0));
        let up = GravityField {
            dir: Vec2::new(0.0, -1.0),
        };
        assert_eq!(up.vertical_sign(), -1.0);
        assert_eq!(up.gravity_accel(1000.0), Vec2::new(0.0, -1000.0));
    }

    #[test]
    fn gravity_zone_overrides_ambient_while_inside_then_reverts() {
        let mut app = App::new();
        app.init_resource::<GravityField>();
        app.init_resource::<BaseGravity>();
        app.add_systems(Update, resolve_active_gravity);
        let player = app
            .world_mut()
            .spawn((
                crate::body::PrimaryBody,
                crate::body::BodyKinematics {
                    pos: Vec2::new(0.0, 0.0),
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
            ))
            .id();
        app.world_mut().spawn(GravityZone {
            aabb: ambition_engine_core::Aabb::new(Vec2::new(200.0, 0.0), Vec2::new(60.0, 60.0)),
            dir: Vec2::new(0.0, -1.0), // up
        });

        // Outside the zone → ambient (down).
        app.update();
        assert!(
            app.world().resource::<GravityField>().dir.y > 0.0,
            "starts ambient down"
        );

        // Inside the zone → gravity points up.
        app.world_mut()
            .get_mut::<crate::body::BodyKinematics>(player)
            .unwrap()
            .pos = Vec2::new(200.0, 0.0);
        app.update();
        assert!(
            app.world().resource::<GravityField>().dir.y < 0.0,
            "inside the gravity-up zone, gravity points up"
        );

        // Leave the zone → reverts to ambient down.
        app.world_mut()
            .get_mut::<crate::body::BodyKinematics>(player)
            .unwrap()
            .pos = Vec2::new(0.0, 0.0);
        app.update();
        assert!(
            app.world().resource::<GravityField>().dir.y > 0.0,
            "exiting the zone reverts to ambient gravity"
        );
    }

    #[test]
    fn apply_world_forces_adds_gravity_over_dt() {
        let g = GravityField::default();
        let mut vel = Vec2::ZERO;
        apply_world_forces(&mut vel, 1200.0, &g, 0.5);
        assert_eq!(vel, Vec2::new(0.0, 600.0));
    }

    fn zones_with(up_at: Vec2, half: Vec2) -> GravityZones {
        GravityZones {
            zones: vec![(
                ambition_engine_core::Aabb::new(up_at, half),
                Vec2::new(0.0, -1.0), // up
            )],
        }
    }

    #[test]
    fn gravity_is_local_two_bodies_feel_different_gravity() {
        // One up-gravity column at x=300; ambient is down.
        let zones = zones_with(Vec2::new(300.0, 0.0), Vec2::new(60.0, 240.0));
        let base = Vec2::new(0.0, 1.0); // down

        // A body INSIDE the column feels up — independent of any other body.
        let inside = Vec2::new(300.0, 50.0);
        assert!(
            gravity_dir_at(inside, &zones, base).y < 0.0,
            "inside the column → up"
        );
        assert_eq!(local_gravity_sign(inside, &zones, base), -1.0);

        // A body OUTSIDE the column (e.g. the player elsewhere) still feels the
        // ambient down. This is the bug fix: the column body's gravity does NOT
        // depend on where the player is.
        let outside = Vec2::new(-200.0, 50.0);
        assert!(
            gravity_dir_at(outside, &zones, base).y > 0.0,
            "outside → ambient down"
        );
        assert_eq!(local_gravity_sign(outside, &zones, base), 1.0);
    }

    #[test]
    fn gravity_dir_at_falls_back_to_ambient_with_no_zones() {
        let empty = GravityZones::default();
        assert_eq!(
            gravity_dir_at(Vec2::new(10.0, 10.0), &empty, Vec2::new(0.0, 1.0)),
            Vec2::new(0.0, 1.0),
        );
        // Flipped ambient (the global switch) reaches a zone-less body.
        assert_eq!(
            gravity_dir_at(Vec2::new(10.0, 10.0), &empty, Vec2::new(0.0, -1.0)),
            Vec2::new(0.0, -1.0),
        );
    }

    #[test]
    fn collect_gravity_zones_snapshots_the_components() {
        let mut app = App::new();
        app.init_resource::<GravityZones>();
        app.add_systems(Update, collect_gravity_zones);
        app.world_mut().spawn(GravityZone {
            aabb: ambition_engine_core::Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0)),
            dir: Vec2::new(0.0, -1.0),
        });
        app.update();
        assert_eq!(app.world().resource::<GravityZones>().zones.len(), 1);
    }

    #[test]
    fn oscillating_zone_slides_horizontally_over_time() {
        let mut app = App::new();
        app.insert_resource(crate::time::SimDt { dt: 0.1 });
        app.add_systems(Update, oscillate_gravity_zones);
        let base = Vec2::new(100.0, 50.0);
        let e = app
            .world_mut()
            .spawn((
                GravityZone {
                    aabb: ambition_engine_core::Aabb::new(base, Vec2::new(20.0, 20.0)),
                    dir: Vec2::new(0.0, -1.0),
                },
                OscillatingZone {
                    base_center: base,
                    half: Vec2::new(20.0, 20.0),
                    amplitude_x: 80.0,
                    freq: 2.0,
                    phase: 0.0,
                },
            ))
            .id();
        app.update(); // phase -> 0.2, sin(0.2) > 0 -> slides right of base
        let aabb = app.world().get::<GravityZone>(e).unwrap().aabb;
        let c = (aabb.min + aabb.max) * 0.5;
        assert!(
            c.x > base.x,
            "the column slid right of its base (x={})",
            c.x
        );
        assert!((c.y - base.y).abs() < 1e-3, "vertical position unchanged");
    }
}
