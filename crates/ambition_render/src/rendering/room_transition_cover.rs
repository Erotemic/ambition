//! How a room change looks: the cover that comes over the old room and lifts
//! off the new one.
//!
//! The engine owns the picture and the timing; a game owns the choice. A game
//! sets [`RoomTransitionStyles`] (its default, and optionally a resolver that
//! picks a style for one room change, by where it is from and where it goes).
//! The default is a short fade to black. Built in: [`CoverStroke::Cut`], a
//! [`CoverStroke::Fade`] in any colour, and a [`CoverStroke::Wipe`] from any
//! side.
//!
//! The contract with whoever runs the transaction (Ambition's
//! `room_transition_presentation`, a demo's host):
//!
//! 1. [`spawn_room_transition_cover`] when the room change begins. The cover
//!    comes in over [`RoomTransitionStyle::cover`].
//! 2. Commit the room change only once [`RoomTransitionCover::is_opaque`]
//!    has held across a rendered frame: the change happens under a whole
//!    cover, whatever the style.
//! 3. [`RoomTransitionCover::lift`] when the new room is presentable, instead
//!    of despawning it. The cover goes over [`RoomTransitionStyle::reveal`]
//!    and despawns itself; gameplay may resume under it.
//!
//! [`PresentationCover`] (what is drawn under it is not in front of the
//! player) is on the cover only while it is opaque.

use std::sync::Arc;
use std::time::Duration;

use bevy::prelude::*;

use super::presentation_settling::PresentationCover;

/// The z of the cover: over the game and its HUD, under a loading screen's
/// foreground.
pub const ROOM_TRANSITION_COVER_Z: i32 = 900;

/// How a cover comes in or goes out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CoverStroke {
    /// At once.
    Cut,
    /// Its opacity, over `duration`.
    Fade { duration: Duration, ease: CoverEase },
    /// It slides in from `from` (and out toward the far side), over `duration`.
    Wipe { duration: Duration, from: WipeSide, ease: CoverEase },
}

impl CoverStroke {
    pub const fn fade(millis: u64) -> Self {
        Self::Fade { duration: Duration::from_millis(millis), ease: CoverEase::Smooth }
    }

    pub const fn wipe(millis: u64, from: WipeSide) -> Self {
        Self::Wipe { duration: Duration::from_millis(millis), from, ease: CoverEase::Smooth }
    }

    pub fn duration(&self) -> Duration {
        match *self {
            Self::Cut => Duration::ZERO,
            Self::Fade { duration, .. } | Self::Wipe { duration, .. } => duration,
        }
    }

    fn ease(&self) -> CoverEase {
        match *self {
            Self::Cut => CoverEase::Linear,
            Self::Fade { ease, .. } | Self::Wipe { ease, .. } => ease,
        }
    }
}

/// The side a wipe comes in from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WipeSide {
    Left,
    Right,
    Top,
    Bottom,
}

/// How a stroke's progress goes over its time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CoverEase {
    Linear,
    /// Slow at both ends.
    #[default]
    Smooth,
    /// Slow at the start.
    In,
    /// Slow at the end.
    Out,
}

impl CoverEase {
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::Smooth => t * t * (3.0 - 2.0 * t),
            Self::In => t * t,
            Self::Out => 1.0 - (1.0 - t) * (1.0 - t),
        }
    }
}

/// How one room change looks: the cover's colour, how it comes over the old
/// room, and how it lifts off the new one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoomTransitionStyle {
    pub colour: Color,
    pub cover: CoverStroke,
    pub reveal: CoverStroke,
}

impl RoomTransitionStyle {
    /// The default: a quick fade to black and a slightly slower fade back.
    pub const fn fade_to_black() -> Self {
        Self::fade(Color::BLACK, 180, 260)
    }

    /// A fade to `colour` over `out_ms`, and back over `in_ms`.
    pub const fn fade(colour: Color, out_ms: u64, in_ms: u64) -> Self {
        Self { colour, cover: CoverStroke::fade(out_ms), reveal: CoverStroke::fade(in_ms) }
    }

    /// No animation: the cover is there at once and gone at once.
    pub const fn cut(colour: Color) -> Self {
        Self { colour, cover: CoverStroke::Cut, reveal: CoverStroke::Cut }
    }

    /// A wipe of `colour` in from `from` and off toward the far side.
    pub const fn wipe(colour: Color, from: WipeSide, millis: u64) -> Self {
        Self { colour, cover: CoverStroke::wipe(millis, from), reveal: CoverStroke::wipe(millis, from) }
    }
}

impl Default for RoomTransitionStyle {
    fn default() -> Self {
        Self::fade_to_black()
    }
}

/// One room change, as a style resolver sees it.
#[derive(Clone, Copy, Debug)]
pub struct RoomTransitionStyleQuery<'a> {
    pub source_room: &'a str,
    pub target_room: &'a str,
}

/// Picks the style of one room change, or `None` for the default.
pub type RoomTransitionStyleResolver = Arc<dyn Fn(RoomTransitionStyleQuery<'_>) -> Option<RoomTransitionStyle> + Send + Sync>;

/// A game's room-change look: its default, and optionally a resolver for one
/// room change (a pipe that irises, a dream that fades to white).
#[derive(Resource, Clone, Default)]
pub struct RoomTransitionStyles {
    pub default: RoomTransitionStyle,
    pub resolver: Option<RoomTransitionStyleResolver>,
}

impl RoomTransitionStyles {
    pub fn new(default: RoomTransitionStyle) -> Self {
        Self { default, resolver: None }
    }

    pub fn with_resolver(mut self, resolver: impl Fn(RoomTransitionStyleQuery<'_>) -> Option<RoomTransitionStyle> + Send + Sync + 'static) -> Self {
        self.resolver = Some(Arc::new(resolver));
        self
    }

    /// The style of the room change from `source_room` to `target_room`.
    pub fn resolve(&self, source_room: &str, target_room: &str) -> RoomTransitionStyle {
        self.resolver
            .as_ref()
            .and_then(|resolve| resolve(RoomTransitionStyleQuery { source_room, target_room }))
            .unwrap_or(self.default)
    }
}

impl std::fmt::Debug for RoomTransitionStyles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoomTransitionStyles")
            .field("default", &self.default)
            .field("resolver", &self.resolver.is_some())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum CoverPhase {
    /// Coming in: how far (0..1).
    Covering(f32),
    Covered,
    /// Going out: how far (0..1).
    Revealing(f32),
}

/// A room change's cover. Its owner tags it with the transaction's
/// `sequence`.
#[derive(Component, Clone, Copy, Debug)]
pub struct RoomTransitionCover {
    pub sequence: u64,
    style: RoomTransitionStyle,
    phase: CoverPhase,
}

impl RoomTransitionCover {
    /// Whether the cover hides the whole frame: the room may change under it.
    pub fn is_opaque(&self) -> bool {
        self.phase == CoverPhase::Covered
    }

    /// Whether it is going out (its transaction is over).
    pub fn is_lifting(&self) -> bool {
        matches!(self.phase, CoverPhase::Revealing(_))
    }

    /// Start going out; it despawns itself when it is gone.
    pub fn lift(&mut self) {
        if !self.is_lifting() {
            self.phase = if self.style.reveal == CoverStroke::Cut { CoverPhase::Revealing(1.0) } else { CoverPhase::Revealing(0.0) };
        }
    }

    /// How much of the frame it hides, 0..1 (a fade's opacity, a wipe's reach).
    pub fn coverage(&self) -> f32 {
        match self.phase {
            CoverPhase::Covering(t) => self.style.cover.ease().apply(t),
            CoverPhase::Covered => 1.0,
            CoverPhase::Revealing(t) => 1.0 - self.style.reveal.ease().apply(t),
        }
    }
}

/// Spawn the cover of room change `sequence` in `style`. `from_coverage` is
/// how much of the frame is already hidden (a cover that was still going out
/// when the next room change began), so it does not pop.
pub fn spawn_room_transition_cover(commands: &mut Commands, sequence: u64, style: RoomTransitionStyle, from_coverage: f32) -> Entity {
    let phase = match style.cover {
        CoverStroke::Cut => CoverPhase::Covered,
        _ if from_coverage >= 1.0 => CoverPhase::Covered,
        _ => CoverPhase::Covering(inverse_ease(style.cover.ease(), from_coverage.clamp(0.0, 1.0))),
    };
    spawn_cover(commands, RoomTransitionCover { sequence, style, phase })
}

/// A whole cover that is going out now, in `style`'s reveal: for a frame
/// hidden some other way (a first room's cover) to lift in the game's look.
/// It belongs to no room change.
pub fn spawn_lifting_cover(commands: &mut Commands, style: RoomTransitionStyle) -> Entity {
    let mut cover = RoomTransitionCover { sequence: u64::MAX, style, phase: CoverPhase::Covered };
    cover.lift();
    spawn_cover(commands, cover)
}

fn spawn_cover(commands: &mut Commands, cover: RoomTransitionCover) -> Entity {
    let sequence = cover.sequence;
    let mut entity = commands.spawn((
        cover,
        cover_node(&cover),
        BackgroundColor(cover_colour(&cover)),
        GlobalZIndex(ROOM_TRANSITION_COVER_Z),
        Name::new(format!("room transition cover {sequence}")),
    ));
    if cover.is_opaque() {
        entity.insert(PresentationCover);
    }
    entity.id()
}

/// The coverage left, put back as a stroke's time: the `t` whose eased value
/// is `coverage` (bisection; the eases are monotonic).
fn inverse_ease(ease: CoverEase, coverage: f32) -> f32 {
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..20 {
        let mid = 0.5 * (lo + hi);
        if ease.apply(mid) < coverage {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

fn cover_colour(cover: &RoomTransitionCover) -> Color {
    let stroke = match cover.phase {
        CoverPhase::Revealing(_) => cover.style.reveal,
        _ => cover.style.cover,
    };
    match stroke {
        CoverStroke::Fade { .. } => cover.style.colour.with_alpha(cover.style.colour.alpha() * cover.coverage()),
        _ => cover.style.colour,
    }
}

fn cover_node(cover: &RoomTransitionCover) -> Node {
    let full = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    let (stroke, going_out) = match cover.phase {
        CoverPhase::Revealing(_) => (cover.style.reveal, true),
        _ => (cover.style.cover, false),
    };
    let CoverStroke::Wipe { from, .. } = stroke else {
        return full;
    };
    // Coming in, the cover grows from `from`; going out, it leaves toward
    // the far side, so it is anchored there.
    let reach = Val::Percent(100.0 * cover.coverage());
    let anchored_far = going_out;
    match (from, anchored_far) {
        (WipeSide::Left, false) | (WipeSide::Right, true) => Node { left: Val::Px(0.0), width: reach, ..full },
        (WipeSide::Right, false) | (WipeSide::Left, true) => Node { right: Val::Px(0.0), width: reach, ..full },
        (WipeSide::Top, false) | (WipeSide::Bottom, true) => Node { top: Val::Px(0.0), height: reach, ..full },
        (WipeSide::Bottom, false) | (WipeSide::Top, true) => Node { bottom: Val::Px(0.0), height: reach, ..full },
    }
}

/// Advance every cover by real time: coming in, held, going out and gone.
pub fn animate_room_transition_covers(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut covers: Query<(Entity, &mut RoomTransitionCover, &mut Node, &mut BackgroundColor, Has<PresentationCover>)>,
) {
    let dt = time.delta_secs();
    for (entity, mut cover, mut node, mut colour, marked) in &mut covers {
        let step = |stroke: CoverStroke, t: f32| {
            let secs = stroke.duration().as_secs_f32();
            if secs <= 0.0 { 1.0 } else { (t + dt / secs).min(1.0) }
        };
        cover.phase = match cover.phase {
            CoverPhase::Covering(t) => match step(cover.style.cover, t) {
                t if t >= 1.0 => CoverPhase::Covered,
                t => CoverPhase::Covering(t),
            },
            CoverPhase::Covered => CoverPhase::Covered,
            CoverPhase::Revealing(t) if t >= 1.0 => {
                commands.entity(entity).despawn();
                continue;
            }
            CoverPhase::Revealing(t) => CoverPhase::Revealing(step(cover.style.reveal, t)),
        };
        if cover.is_opaque() != marked {
            if cover.is_opaque() {
                commands.entity(entity).insert(PresentationCover);
            } else {
                commands.entity(entity).remove::<PresentationCover>();
            }
        }
        *node = cover_node(&cover);
        colour.0 = cover_colour(&cover);
    }
}

/// Where the covers are advanced. A cover's owner reads them after it.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RoomTransitionCoverAnimationSet;

/// The room-change look: the default style and the cover's animation. A
/// game that wants another look inserts its own [`RoomTransitionStyles`]
/// (before or after this plugin).
pub struct RoomTransitionCoverPlugin;

impl Plugin for RoomTransitionCoverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoomTransitionStyles>()
            .add_systems(Update, animate_room_transition_covers.in_set(RoomTransitionCoverAnimationSet));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cover(style: RoomTransitionStyle) -> RoomTransitionCover {
        RoomTransitionCover { sequence: 1, style, phase: CoverPhase::Covering(0.0) }
    }

    #[test]
    fn a_fade_hides_the_frame_only_at_its_end() {
        let mut c = cover(RoomTransitionStyle::fade_to_black());
        assert!(!c.is_opaque() && c.coverage() == 0.0);
        c.phase = CoverPhase::Covering(0.5);
        assert!(!c.is_opaque() && c.coverage() > 0.0 && c.coverage() < 1.0);
        c.phase = CoverPhase::Covered;
        assert!(c.is_opaque());
        c.lift();
        assert!(c.is_lifting() && !c.is_opaque() && c.coverage() == 1.0);
    }

    #[test]
    fn a_cut_lifts_at_once() {
        let mut c = cover(RoomTransitionStyle::cut(Color::BLACK));
        c.phase = CoverPhase::Covered;
        c.lift();
        assert_eq!(c.coverage(), 0.0);
    }

    #[test]
    fn a_resolver_picks_one_room_change_and_the_default_the_rest() {
        let white = RoomTransitionStyle::fade(Color::WHITE, 400, 600);
        let styles = RoomTransitionStyles::default().with_resolver(move |q| (q.target_room == "dream").then_some(white));
        assert_eq!(styles.resolve("hub", "dream"), white);
        assert_eq!(styles.resolve("hub", "cellar"), RoomTransitionStyle::fade_to_black());
    }

    #[test]
    fn a_cover_that_starts_part_way_does_not_pop() {
        for ease in [CoverEase::Linear, CoverEase::Smooth, CoverEase::In, CoverEase::Out] {
            let t = inverse_ease(ease, 0.4);
            assert!((ease.apply(t) - 0.4).abs() < 1e-3, "{ease:?}");
        }
    }
}
