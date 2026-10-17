//! Line effects: lightning, lasers, any look drawn along a line or round a
//! body. One road for all of them.
//!
//! Two kinds of source, one visual:
//!
//! * a RAY STRIKE (`CombatGeometryView`'s strikes with a `ray`): an attack,
//!   drawn as its named look from the ray's own published geometry. The
//!   attack does not draw itself and the look does not hurt; a new kind of
//!   ray attack is a ray with a look.
//! * a DECORATION (`VfxMessage::Lightning`, `VfxMessage::LightningAura`): a
//!   look with no attack under it (a neck bolt's crackle, a powered-up aura),
//!   LEASED by its key (`ambition_vfx::lightning`).
//!
//! Each frame each visual lays its strands out as triangles in the world,
//! unfolded, and hands them to a [`FoldedMesh`]: the folded-mesh writer is
//! what draws them, through the room's portals. A line that goes through a
//! portal is drawn out of the other end the same way the ray's volume was
//! cut, so a look cannot be added that ignores portals.
//!
//! Quality is presentation policy. With the screen-shader budget a line has
//! its halo, its forks and its light where it lands; with none it is its
//! white-hot core alone and an aura is not drawn. A ray's core is drawn on
//! every device: it is the tell.

pub mod strands;

use bevy::math::Vec2;
use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::config::WORLD_Z_FX;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRoomOf, LiveRooms, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::presented_pose::PresentedPose;
use ambition_sim_view::CombatGeometryView;
use ambition_vfx::vfx::{VfxInRoom, VfxMessage};
use ambition_vfx::{LightningEnd, LightningKey, LightningLook};

use crate::folded_mesh::{Composite, FoldedMesh, FoldedMeshSync, Tris};
use crate::quality::ResolvedVisualQuality;
use strands::{Rng, Strand};

/// Over bodies and the other effects.
const Z: f32 = WORLD_Z_FX + 3.0;

/// Seconds a line takes to fade once its source is gone.
const FADE_S: f32 = 0.12;

/// A ray strike's line is held this long past the last frame it was seen:
/// a frame or two of the view's tick.
const RAY_HOLD_S: f32 = 0.05;

/// A laser: one straight beam, its width breathing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaserLook {
    pub glow: [f32; 4],
    pub core: [f32; 4],
    /// The core's width over the ray's.
    pub core_of_width: f32,
    /// The halo's width over the core's.
    pub glow_scale: f32,
    /// Breaths a second.
    pub pulse_hz: f32,
    /// The light where it lands.
    pub contact_glow: f32,
}

impl LaserLook {
    pub const RED: LaserLook = LaserLook {
        glow: [1.0, 0.15, 0.1, 0.55],
        core: [1.0, 0.92, 0.86, 1.0],
        core_of_width: 0.3,
        glow_scale: 3.5,
        pulse_hz: 14.0,
        contact_glow: 22.0,
    };
}

/// How a line is drawn. A ray names one ([`LineLook::named`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineLook {
    Lightning(LightningLook),
    Laser(LaserLook),
}

impl LineLook {
    /// The look a ray strike names, or `None`.
    pub fn named(name: &str) -> Option<LineLook> {
        match name {
            "laser" => Some(LineLook::Laser(LaserLook::RED)),
            other => LightningLook::named(other).map(LineLook::Lightning),
        }
    }

    fn flicker_s(&self) -> f32 {
        match self {
            LineLook::Lightning(look) => look.flicker_s,
            LineLook::Laser(_) => f32::INFINITY,
        }
    }
}

/// What this device draws of a line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Detail {
    /// The halo, the light where a line lands, and auras.
    pub rich: bool,
    /// Of a look's forks (and an aura's arcs), how many.
    pub forks: f32,
}

impl Detail {
    pub fn of(quality: Option<&ResolvedVisualQuality>) -> Detail {
        match quality {
            None => Detail { rich: true, forks: 1.0 },
            Some(q) if !q.budget.shaders.draws_screen_shaders() => Detail { rich: false, forks: 0.0 },
            Some(q) => Detail { rich: true, forks: q.budget.particles.spawn_rate_scale.clamp(0.0, 1.0) },
        }
    }

    fn forks(&self, look: &LightningLook) -> usize {
        (look.forks as f32 * self.forks).round() as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    Ray(Entity),
    Lease(LightningKey),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Shape {
    /// From one end to the other. `band`: a ray's width, which a jagged look
    /// keeps within, so the drawn line is where the ray hurts.
    Line { from: LightningEnd, to: LightningEnd, band: Option<f32> },
    Aura { owner: Entity, center: ae::Vec2, half: ae::Vec2 },
}

/// One line effect.
#[derive(Component)]
pub struct LineVisual {
    source: Source,
    shape: Shape,
    look: LineLook,
    /// Seconds left before it fades: renewed while its source is there.
    hold: f32,
    age: f32,
    /// 1 while held; down to 0 once the hold runs out.
    fade: f32,
    since_path: f32,
    rng: Rng,
    strands: Vec<Strand>,
    intensity: f32,
    /// Where the body ends were last seen, for a body that has gone.
    last_seen: [Option<ae::Vec2>; 2],
}

impl LineVisual {
    fn new(source: Source, seed: u64, shape: Shape, look: LineLook, hold: f32) -> Self {
        Self {
            source,
            shape,
            look,
            hold,
            age: 0.0,
            fade: 1.0,
            since_path: f32::INFINITY,
            rng: Rng::new(seed),
            strands: Vec::new(),
            intensity: 1.0,
            last_seen: [None, None],
        }
    }

    fn renew(&mut self, shape: Shape, look: LineLook, hold: f32) {
        self.shape = shape;
        self.look = look;
        self.hold = self.hold.max(hold);
        self.fade = 1.0;
    }
}

/// Installs line effects. A host with no renderer installs nothing.
pub struct LinesPlugin;

impl Plugin for LinesPlugin {
    fn build(&self, app: &mut App) {
        if app.get_sub_app(bevy::render::RenderApp).is_none() {
            return;
        }
        use ambition_platformer2d_shared_tangle::lifecycle::session_world_exists;
        use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase;
        app.add_systems(
            Update,
            (receive_decorations, follow_ray_strikes, lay_out_lines)
                .chain()
                .after(Platformer2dSimulationPhase::CoreSimulation)
                // A body end is where its body is drawn this frame.
                .after(crate::rendering::actors::sync_visuals)
                .before(FoldedMeshSync)
                .run_if(session_world_exists),
        );
    }
}

fn spawn_line(commands: &mut Commands, scope: SessionSpawnScope, visual: LineVisual) {
    commands.spawn_session_scoped(
        scope,
        (Name::new("VFX line"), Transform::default(), Visibility::default(), FoldedMesh::default(), visual),
    );
}

/// Renew the lease of each decoration row's key, or start one.
fn receive_decorations(
    mut commands: Commands,
    mut messages: MessageReader<VfxInRoom>,
    rooms: LiveRooms,
    active_session: Option<Res<ActiveSessionScope>>,
    mut live: Query<&mut LineVisual>,
) {
    let session = SessionSpawnScope::for_optional_active_session(active_session.as_deref());
    let mut started: Vec<LightningKey> = Vec::new();
    for message in messages.read() {
        let (key, shape, look, hold) = match message.vfx {
            VfxMessage::Lightning { key, from, to, look, hold } => (key, Shape::Line { from, to, band: None }, look, hold),
            VfxMessage::LightningAura { key, owner, center, half, look, hold } => {
                (key, Shape::Aura { owner, center, half }, look, hold)
            }
            _ => continue,
        };
        let look = LineLook::Lightning(look);
        if let Some(mut visual) = live.iter_mut().find(|v| v.source == Source::Lease(key)) {
            visual.renew(shape, look, hold);
            continue;
        }
        // A second row for a lease started this frame: the first holds it.
        if started.contains(&key) {
            continue;
        }
        let (Some(session), Some(room)) = (session, message.room.or_else(|| rooms.sole())) else {
            continue;
        };
        started.push(key);
        spawn_line(&mut commands, session.in_room(Some(room)), LineVisual::new(Source::Lease(key), key.0, shape, look, hold));
    }
}

/// Keep a visual on each ray strike the combat view publishes, along its
/// line, in its look.
fn follow_ray_strikes(
    mut commands: Commands,
    view: Option<Res<CombatGeometryView>>,
    rooms: LiveRooms,
    active_session: Option<Res<ActiveSessionScope>>,
    mut live: Query<&mut LineVisual>,
) {
    let Some(view) = view else {
        return;
    };
    let session = SessionSpawnScope::for_optional_active_session(active_session.as_deref());
    for strike in &view.strikes {
        let Some(ray) = &strike.ray else {
            continue;
        };
        let Some(look) = LineLook::named(&ray.look) else {
            bevy::log::warn_once!("a ray strike looks {:?}, which no line look is named; it is not drawn", ray.look);
            continue;
        };
        let shape = Shape::Line {
            from: LightningEnd::World(ray.origin),
            to: LightningEnd::World(ray.origin + ray.dir * ray.length),
            band: Some(ray.width),
        };
        if let Some(mut visual) = live.iter_mut().find(|v| v.source == Source::Ray(strike.strike)) {
            visual.renew(shape, look, RAY_HOLD_S);
            continue;
        }
        let (Some(session), Some(room)) = (session, rooms.of(strike.owner).or_else(|| rooms.sole())) else {
            continue;
        };
        let visual = LineVisual::new(Source::Ray(strike.strike), strike.strike.to_bits(), shape, look, RAY_HOLD_S);
        spawn_line(&mut commands, session.in_room(Some(room)), visual);
    }
}

/// Where `end` is now, remembering a body end in `seen`.
fn resolve(end: LightningEnd, owners: &Query<&PresentedPose>, seen: &mut Option<ae::Vec2>) -> Option<ae::Vec2> {
    match end {
        LightningEnd::World(at) => Some(at),
        LightningEnd::Body { owner, offset } => {
            if let Ok(pose) = owners.get(owner) {
                *seen = Some(pose.presented() + offset);
            }
            *seen
        }
    }
}

/// The body a line is carried with: the owner of its start, or of its aura.
fn carrier_of(shape: &Shape) -> Option<Entity> {
    match *shape {
        Shape::Line { from: LightningEnd::Body { owner, .. }, .. } => Some(owner),
        Shape::Line { .. } => None,
        Shape::Aura { owner, .. } => Some(owner),
    }
}

fn linear(c: [f32; 4], alpha: f32) -> [f32; 4] {
    let l = Color::srgba(c[0], c[1], c[2], c[3]).to_linear();
    [l.red, l.green, l.blue, (l.alpha * alpha).clamp(0.0, 1.0)]
}

/// Age, fade and re-path every line, and lay its triangles out in the world
/// for its folded mesh.
fn lay_out_lines(
    mut commands: Commands,
    time: Res<Time>,
    quality: Option<Res<ResolvedVisualQuality>>,
    world: LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    owners: Query<&PresentedPose>,
    mut visuals: Query<(Entity, &mut LineVisual, &mut FoldedMesh)>,
) {
    let dt = time.delta_secs();
    let detail = Detail::of(quality.as_deref());
    for (entity, mut v, mut mesh) in &mut visuals {
        v.age += dt;
        v.hold -= dt;
        v.since_path += dt;
        if v.hold <= 0.0 {
            v.fade -= dt / FADE_S;
            if v.fade <= 0.0 {
                commands.entity(entity).despawn();
                continue;
            }
        }
        if world.of(entity).is_none() {
            continue;
        }
        let v = &mut *v;
        // The frame: where its (0, 0) is in the world, and its two axes.
        let frame = match v.shape {
            Shape::Line { from, to, .. } => {
                let [seen_from, seen_to] = &mut v.last_seen;
                match (resolve(from, &owners, seen_from), resolve(to, &owners, seen_to)) {
                    (Some(a), Some(b)) => Some((a, b - a, (b - a).perp())),
                    _ => None,
                }
            }
            Shape::Aura { owner, center, half } => {
                resolve(LightningEnd::Body { owner, offset: center }, &owners, &mut v.last_seen[0])
                    .map(|c| (c, ae::Vec2::new(half.x, 0.0), ae::Vec2::new(0.0, half.y)))
            }
        };
        let Some((origin, ax, ay)) = frame else {
            continue;
        };
        if v.since_path >= v.look.flicker_s() || v.strands.is_empty() {
            v.since_path = 0.0;
            v.strands = match (v.look, v.shape) {
                (LineLook::Laser(_), _) => strands::straight(ax.length()),
                (LineLook::Lightning(look), Shape::Line { band, .. }) => {
                    let length = ax.length().max(1.0);
                    // A ray's bolt strays no farther than its band: the
                    // line drawn is where it hurts (the halving levels sum to
                    // under 2.25 times the first).
                    let jag = band.map_or(look.jag, |band| look.jag.min(band * 0.5 / (2.25 * length)));
                    strands::bolt(&mut v.rng, length, jag, detail.forks(&look))
                }
                (LineLook::Lightning(look), Shape::Aura { .. }) if detail.rich => {
                    strands::aura(&mut v.rng, look.jag, detail.forks(&look).max(1))
                }
                (LineLook::Lightning(_), Shape::Aura { .. }) => Vec::new(),
            };
            v.intensity = v.rng.range(0.7, 1.0);
        }
        let place = |p: Vec2| origin + ax * p.x + ay * p.y;
        let k = v.intensity * v.fade.clamp(0.0, 1.0);
        let mut tris = Tris::default();
        match v.look {
            LineLook::Lightning(look) => {
                let head = if look.grow_s > 0.0 { (v.age / look.grow_s).min(1.0) } else { 1.0 };
                for strand in &v.strands {
                    if !strand.trunk && strand.appears_at > head {
                        continue;
                    }
                    let points = if strand.trunk { strands::grown(&strand.points, head) } else { strand.points.clone() };
                    let points: Vec<Vec2> = points.into_iter().map(place).collect();
                    let widths: Vec<f32> = strand.widths.iter().take(points.len()).map(|w| w * look.width).collect();
                    if detail.rich {
                        let halo: Vec<f32> = widths.iter().map(|w| w * look.glow_scale).collect();
                        strands::ribbon(&mut tris, &points, &halo, linear(look.glow, k), linear(look.glow, 0.0));
                    }
                    strands::ribbon(&mut tris, &points, &widths, linear(look.core, k), linear(look.core, 0.3 * k));
                }
                if detail.rich && look.contact_glow > 0.0 {
                    let radius = look.contact_glow * (0.75 + 0.35 * v.intensity);
                    let (center, edge) = (linear(look.glow, 0.8 * k), linear(look.glow, 0.0));
                    match v.shape {
                        Shape::Line { .. } => {
                            strands::glow(&mut tris, origin, radius * 0.45, center, edge);
                            if head >= 1.0 {
                                strands::glow(&mut tris, place(Vec2::X), radius, center, edge);
                            }
                        }
                        Shape::Aura { .. } => strands::glow(&mut tris, origin, radius, center, edge),
                    }
                }
            }
            LineLook::Laser(look) => {
                let band = match v.shape {
                    Shape::Line { band, .. } => band.unwrap_or(8.0),
                    Shape::Aura { half, .. } => half.x,
                };
                let breath = 1.0 + 0.12 * (v.age * look.pulse_hz * std::f32::consts::TAU).sin();
                let core = band * look.core_of_width * breath;
                for strand in &v.strands {
                    let points: Vec<Vec2> = strand.points.iter().copied().map(place).collect();
                    let widths: Vec<f32> = strand.widths.iter().map(|w| w * core).collect();
                    if detail.rich {
                        let halo: Vec<f32> = widths.iter().map(|w| w * look.glow_scale).collect();
                        strands::ribbon(&mut tris, &points, &halo, linear(look.glow, k), linear(look.glow, 0.0));
                    }
                    strands::ribbon(&mut tris, &points, &widths, linear(look.core, k), linear(look.core, 0.4 * k));
                }
                if detail.rich {
                    let (center, edge) = (linear(look.glow, 0.8 * k), linear(look.glow, 0.0));
                    strands::glow(&mut tris, place(Vec2::X), look.contact_glow * breath, center, edge);
                }
            }
        }
        *mesh = FoldedMesh { tris, root: origin, carrier: carrier_of(&v.shape), z: Z, composite: Composite::Add };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_persistence::settings::video::{VisualQualityBudget, VisualQualityProfile};

    fn at(profile: VisualQualityProfile) -> Detail {
        Detail::of(Some(&ResolvedVisualQuality { profile, budget: VisualQualityBudget::for_profile(profile) }))
    }

    /// The core is the tell, so every device draws it; the rest is on top.
    #[test]
    fn a_device_with_no_shader_budget_draws_the_core_alone() {
        let potato = at(VisualQualityProfile::Potato);
        assert!(!potato.rich);
        assert_eq!(potato.forks(&LightningLook::STRIKE), 0);
        let high = at(VisualQualityProfile::High);
        assert!(high.rich);
        assert_eq!(high.forks(&LightningLook::STRIKE), LightningLook::STRIKE.forks as usize);
        assert!(at(VisualQualityProfile::Low).forks(&LightningLook::FORCE) < high.forks(&LightningLook::FORCE));
    }

    #[test]
    fn a_ray_names_a_lightning_or_a_laser_look() {
        assert!(matches!(LineLook::named("force"), Some(LineLook::Lightning(_))));
        assert!(matches!(LineLook::named("laser"), Some(LineLook::Laser(_))));
        assert_eq!(LineLook::named("plasma"), None);
    }
}
