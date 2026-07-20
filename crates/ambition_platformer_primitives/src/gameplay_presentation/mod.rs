//! Gameplay presentation policy: where the gameplay camera renders on the
//! physical display, and where important subjects should stay inside it.
//!
//! Design of record: `docs/planning/triage/gameplay-presentation-profiles.md`.
//!
//! Four INDEPENDENT axes, deliberately not collapsed into one enum:
//!
//! 1. **Viewport** — where the gameplay camera renders ([`GameplayViewportPolicy`]);
//! 2. **Framing** — where subjects should remain inside it ([`SubjectFramingPolicy`]);
//! 3. **Screen occupancy** — what controls/HUD reserve ([`ScreenOccluder`]);
//! 4. **Activation** — which profile applies ([`PresentationEnvironment`]).
//!
//! Everything here is pure: no windows, no rendering, no touch input, no game
//! content, no provider. The host resolves once per frame and every consumer
//! reads the single [`ResolvedGameplayPresentation`] — no camera, HUD, touch,
//! pointer, or transition system recalculates margins on its own.
//!
//! **The world-space actor is never constrained by presentation.** The camera
//! and UI adapt around the actor; simulation is identical on desktop, mobile,
//! full-bleed, and fixed-aspect.

use ambition_engine_core as ae;
use bevy::prelude::{Component, Resource};

mod control_regions;
mod presets;
mod resolve;

#[cfg(test)]
mod tests;

pub use control_regions::{
    ControlAnchor, ControlFootprint, ControlFootprints, ControlPlacement, ControlPlacementPolicy,
    PlacedControl, ResolvedControlRegions,
};
pub use presets::profiles;
pub use resolve::{resolve_gameplay_presentation, GameplayPresentationInput};

// ---------------------------------------------------------------------------
// Normalized screen geometry
// ---------------------------------------------------------------------------

/// Axis-aligned rectangle in screen pixels: origin at the display's top-left,
/// +Y downward. This is Bevy's window/cursor convention, and also Ambition's
/// world convention, which is why screen-space framing math needs no Y flip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenRect {
    pub min: ae::Vec2,
    pub max: ae::Vec2,
}

impl ScreenRect {
    pub fn from_min_size(min: ae::Vec2, size: ae::Vec2) -> Self {
        Self {
            min,
            max: min + size.max(ae::Vec2::ZERO),
        }
    }

    pub fn from_corners(a: ae::Vec2, b: ae::Vec2) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    pub fn size(self) -> ae::Vec2 {
        (self.max - self.min).max(ae::Vec2::ZERO)
    }

    pub fn width(self) -> f32 {
        self.size().x
    }

    pub fn height(self) -> f32 {
        self.size().y
    }

    pub fn center(self) -> ae::Vec2 {
        (self.min + self.max) * 0.5
    }

    pub fn area(self) -> f32 {
        let size = self.size();
        size.x * size.y
    }

    pub fn is_empty(self) -> bool {
        self.max.x <= self.min.x || self.max.y <= self.min.y
    }

    pub fn contains(self, point: ae::Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    pub fn overlaps(self, other: Self) -> bool {
        self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
    }

    /// Intersection, clamped so the result is never inverted.
    pub fn intersect(self, other: Self) -> Self {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max).max(min);
        Self { min, max }
    }

    /// Shrink by per-side insets.
    ///
    /// Over-consuming insets collapse the affected axis to a point rather than
    /// swapping the corners — a swapped rect would read as a large valid area
    /// and silently pass an emptiness check. Callers decide whether to fall
    /// back on the collapse.
    pub fn inset(self, insets: ScreenInsets) -> Self {
        let min = self.min + ae::Vec2::new(insets.left, insets.top);
        let max = self.max - ae::Vec2::new(insets.right, insets.bottom);
        let midpoint = (min + max) * 0.5;
        Self {
            min: min.min(midpoint),
            max: max.max(midpoint),
        }
    }

    /// Express this rect as fractions of `outer`. A zero-sized `outer` yields
    /// [`NormalizedScreenRegion::FULL`].
    pub fn normalized_within(self, outer: Self) -> NormalizedScreenRegion {
        let size = outer.size();
        if size.x <= 0.0 || size.y <= 0.0 {
            return NormalizedScreenRegion::FULL;
        }
        NormalizedScreenRegion {
            min: (self.min - outer.min) / size,
            max: (self.max - outer.min) / size,
        }
    }
}

/// Fractions of a reference rectangle, origin top-left: `(0,0)..(1,1)` is the
/// whole rectangle. Normalized so a region authored once survives every
/// display size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizedScreenRegion {
    pub min: ae::Vec2,
    pub max: ae::Vec2,
}

impl NormalizedScreenRegion {
    pub const FULL: Self = Self {
        min: ae::Vec2::ZERO,
        max: ae::Vec2::ONE,
    };

    pub fn new(min: ae::Vec2, max: ae::Vec2) -> Self {
        Self {
            min: min.min(max),
            max: min.max(max),
        }
    }

    /// A centered region inset by a fraction on each axis.
    pub fn centered_inset(x: f32, y: f32) -> Self {
        let x = x.clamp(0.0, 0.49);
        let y = y.clamp(0.0, 0.49);
        Self {
            min: ae::Vec2::new(x, y),
            max: ae::Vec2::new(1.0 - x, 1.0 - y),
        }
    }

    pub fn size(self) -> ae::Vec2 {
        (self.max - self.min).max(ae::Vec2::ZERO)
    }

    pub fn center(self) -> ae::Vec2 {
        (self.min + self.max) * 0.5
    }

    /// Project onto a concrete rectangle.
    pub fn resolve(self, outer: ScreenRect) -> ScreenRect {
        let size = outer.size();
        ScreenRect {
            min: outer.min + self.min * size,
            max: outer.min + self.max * size,
        }
    }

    /// Interpolate toward `other`. Used for framing hysteresis so controls
    /// appearing or disappearing do not step the camera.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            min: self.min + (other.min - self.min) * t,
            max: self.max + (other.max - self.max) * t,
        }
    }
}

impl Default for NormalizedScreenRegion {
    fn default() -> Self {
        Self::FULL
    }
}

/// Per-side screen insets in pixels — platform safe area (notch, cutout,
/// gesture bar) or any other reserved border.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScreenInsets {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl ScreenInsets {
    pub const ZERO: Self = Self {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        bottom: 0.0,
    };

    pub fn new(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self {
            left: left.max(0.0),
            right: right.max(0.0),
            top: top.max(0.0),
            bottom: bottom.max(0.0),
        }
    }

    pub fn is_zero(self) -> bool {
        self == Self::ZERO
    }
}

/// Platform safe-area insets for the primary display.
///
/// **Nothing writes a non-zero value yet** — no supported platform exposes
/// cutout information to this codebase today. The resource exists so the
/// policy is already inset-correct (and tested asymmetrically): when an
/// Android/iOS bridge lands it writes this resource and no presentation code
/// changes. Zero is the honest fallback, not a placeholder.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct DisplaySafeAreaInsets(pub ScreenInsets);

/// Width:height ratio for a fixed-aspect gameplay viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AspectRatio {
    pub width: f32,
    pub height: f32,
}

impl AspectRatio {
    pub const FOUR_THREE: Self = Self {
        width: 4.0,
        height: 3.0,
    };
    pub const SIXTEEN_NINE: Self = Self {
        width: 16.0,
        height: 9.0,
    };

    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width: width.max(f32::EPSILON),
            height: height.max(f32::EPSILON),
        }
    }

    pub fn ratio(self) -> f32 {
        (self.width / self.height).max(f32::EPSILON)
    }
}

// ---------------------------------------------------------------------------
// Axis 1 — viewport
// ---------------------------------------------------------------------------

/// Where the gameplay camera renders on the physical display.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GameplayViewportPolicy {
    /// Gameplay covers the whole device-safe display.
    #[default]
    FullBleed,
    /// Gameplay is fitted to a fixed aspect inside the device-safe display;
    /// the remainder becomes surround.
    FixedAspect {
        aspect: AspectRatio,
        fit: FixedAspectFit,
    },
}

/// Where a fixed-aspect rectangle sits when the safe display leaves slack.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FixedAspectFit {
    #[default]
    Center,
    /// Pin to the top edge — leaves all vertical slack below, which is where
    /// thumbs are.
    Top,
    Bottom,
}

// ---------------------------------------------------------------------------
// Axis 2 — framing
// ---------------------------------------------------------------------------

/// Where important subjects should remain inside the gameplay viewport.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SubjectFramingPolicy {
    /// Ordinary camera behavior: the existing follow/zoom/clamp policy alone.
    #[default]
    Normal,
    /// Soft-region framing against the authored region only.
    SoftSafeRegion(SoftFramingProfile),
    /// Soft-region framing, additionally reduced by active screen occupancy.
    OcclusionAware(SoftFramingProfile),
}

impl SubjectFramingPolicy {
    pub fn profile(self) -> Option<SoftFramingProfile> {
        match self {
            Self::Normal => None,
            Self::SoftSafeRegion(profile) | Self::OcclusionAware(profile) => Some(profile),
        }
    }

    pub fn consumes_occlusions(self) -> bool {
        matches!(self, Self::OcclusionAware(_))
    }
}

/// Tuning for soft subject framing.
///
/// The camera behaves normally while the subject stays inside the region; when
/// the subject's protected bounds cross an edge, only the correction needed to
/// return them is applied. That deadzone IS the softness — there is no separate
/// "correction strength" knob to fight the existing target easing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoftFramingProfile {
    /// Authored subject-safe region, normalized within the gameplay rectangle.
    pub safe_region: NormalizedScreenRegion,
    /// Seconds of subject velocity folded into the protected bounds. Gives
    /// high-speed movement room ahead of itself without an asymmetric anchor.
    pub look_ahead_seconds: f32,
    /// Exponential rate at which the resolved region follows a change in
    /// occupancy. This is the hysteresis: controls appearing or disappearing
    /// must not twitch the camera.
    pub region_ease_hz: f32,
    /// Extra padding around the subject's body box, in gameplay-viewport
    /// pixels — held items, attack anticipation, large controlled bodies.
    pub subject_padding_px: ae::Vec2,
    /// The resolved region is never carved below this fraction of the gameplay
    /// rectangle. Prevents dense occupancy from collapsing framing to a point.
    pub min_region_fraction: ae::Vec2,
}

impl Default for SoftFramingProfile {
    fn default() -> Self {
        Self::platformer()
    }
}

impl SoftFramingProfile {
    /// General platformer framing: a generous central region, mild look-ahead.
    pub fn platformer() -> Self {
        Self {
            safe_region: NormalizedScreenRegion::centered_inset(0.18, 0.14),
            look_ahead_seconds: 0.12,
            region_ease_hz: 4.0,
            subject_padding_px: ae::Vec2::splat(24.0),
            min_region_fraction: ae::Vec2::new(0.25, 0.25),
        }
    }

    /// High-speed framing: a tighter vertical band and much stronger
    /// look-ahead, so a fast runner sees where it is going instead of where it
    /// has been.
    pub fn high_speed() -> Self {
        Self {
            safe_region: NormalizedScreenRegion::centered_inset(0.26, 0.16),
            look_ahead_seconds: 0.28,
            region_ease_hz: 3.0,
            subject_padding_px: ae::Vec2::new(36.0, 24.0),
            min_region_fraction: ae::Vec2::new(0.22, 0.25),
        }
    }
}

// ---------------------------------------------------------------------------
// Axis 3 — screen occupancy
// ---------------------------------------------------------------------------

/// A display corner (or the center) that a screen region anchors to.
///
/// Occupancy is anchored rather than normalized because on-screen controls are
/// authored as fixed-size touch targets at a fixed inset from a corner — a
/// thumbstick does not grow with the display.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScreenAnchor {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

impl ScreenAnchor {
    /// Resolve a rectangle whose `offset_px` runs from the anchor toward the
    /// display interior.
    pub fn resolve_rect(
        self,
        display: ScreenRect,
        offset_px: ae::Vec2,
        size: ae::Vec2,
    ) -> ScreenRect {
        let size = size.max(ae::Vec2::ZERO);
        let min = match self {
            Self::TopLeft => display.min + offset_px,
            Self::TopRight => ae::Vec2::new(
                display.max.x - offset_px.x - size.x,
                display.min.y + offset_px.y,
            ),
            Self::BottomLeft => ae::Vec2::new(
                display.min.x + offset_px.x,
                display.max.y - offset_px.y - size.y,
            ),
            Self::BottomRight => ae::Vec2::new(
                display.max.x - offset_px.x - size.x,
                display.max.y - offset_px.y - size.y,
            ),
            Self::Center => display.center() - size * 0.5 + offset_px,
        };
        ScreenRect::from_min_size(min, size)
    }
}

/// What a reserved screen region is for.
///
/// Purpose exists so policy can distinguish "this hides the actor and matters"
/// from "this is chrome the participant can look past" WITHOUT the presentation
/// subsystem knowing which crate produced it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScreenOcclusionPurpose {
    #[default]
    VirtualMovementStick,
    VirtualActionCluster,
    ContextualAction,
    PersistentHud,
    SystemMenuControl,
    Dialogue,
}

impl ScreenOcclusionPurpose {
    /// Whether the controlled subject should be kept out of this region.
    ///
    /// Movement/action controls sit under a thumb for the whole session, so the
    /// actor must not live behind them. System menu chrome is small, cornered,
    /// and glanced at; dialogue already takes the camera's attention through
    /// its own presentation. Neither should shrink gameplay framing.
    pub fn reserves_subject_space(self) -> bool {
        match self {
            Self::VirtualMovementStick
            | Self::VirtualActionCluster
            | Self::ContextualAction
            | Self::PersistentHud => true,
            Self::SystemMenuControl | Self::Dialogue => false,
        }
    }
}

/// Where an occluder's rectangle comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum OccluderGeometry {
    /// Derive it from this entity's own computed `bevy_ui` layout.
    ///
    /// The common case, and the reason this is the default: a producer tags the
    /// `Node` it already has and restates nothing. Moving, resizing,
    /// percentage sizing, parent constraints, safe-area reflow and compact
    /// fallback layouts are all picked up automatically, because the occupancy
    /// IS the layout rather than a second description of it.
    #[default]
    ComputedUi,
    /// An explicit rectangle in logical display pixels, for sources that
    /// genuinely have no `bevy_ui` node.
    Explicit(ScreenRect),
    /// A display-corner-anchored box, for non-UI sources that must track an
    /// edge.
    Anchored {
        anchor: ScreenAnchor,
        offset_px: ae::Vec2,
        size_px: ae::Vec2,
    },
}

/// Generic screen occupancy published by whatever draws over the display.
///
/// Producers tag their existing UI entities and publish nothing else — they do
/// not own camera policy, do not compute margins, and do not know which
/// framing profile is active:
///
/// ```ignore
/// commands.spawn((node, ScreenOccluder::action_controls()));
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct ScreenOccluder {
    pub purpose: ScreenOcclusionPurpose,
    /// Breathing room added on every side when composing the safe region.
    pub padding_px: ae::Vec2,
    pub geometry: OccluderGeometry,
}

impl ScreenOccluder {
    /// Occupancy derived from this entity's computed UI layout.
    pub fn new(purpose: ScreenOcclusionPurpose) -> Self {
        Self {
            purpose,
            padding_px: ae::Vec2::ZERO,
            geometry: OccluderGeometry::ComputedUi,
        }
    }

    pub fn movement_stick() -> Self {
        Self::new(ScreenOcclusionPurpose::VirtualMovementStick)
    }

    pub fn action_controls() -> Self {
        Self::new(ScreenOcclusionPurpose::VirtualActionCluster)
    }

    pub fn contextual_action() -> Self {
        Self::new(ScreenOcclusionPurpose::ContextualAction)
    }

    pub fn hud() -> Self {
        Self::new(ScreenOcclusionPurpose::PersistentHud)
    }

    pub fn system_controls() -> Self {
        Self::new(ScreenOcclusionPurpose::SystemMenuControl)
    }

    pub fn dialogue() -> Self {
        Self::new(ScreenOcclusionPurpose::Dialogue)
    }

    /// An explicit rectangle, for a producer with no `bevy_ui` node.
    pub fn explicit(purpose: ScreenOcclusionPurpose, rect: ScreenRect) -> Self {
        Self {
            purpose,
            padding_px: ae::Vec2::ZERO,
            geometry: OccluderGeometry::Explicit(rect),
        }
    }

    /// A display-corner-anchored box, for a producer with no `bevy_ui` node.
    pub fn anchored(
        purpose: ScreenOcclusionPurpose,
        anchor: ScreenAnchor,
        offset_px: ae::Vec2,
        size_px: ae::Vec2,
    ) -> Self {
        Self {
            purpose,
            padding_px: ae::Vec2::ZERO,
            geometry: OccluderGeometry::Anchored {
                anchor,
                offset_px,
                size_px,
            },
        }
    }

    pub fn with_padding(mut self, padding_px: ae::Vec2) -> Self {
        self.padding_px = padding_px;
        self
    }

    /// Resolve the geometry this occluder owns itself. Returns `None` for
    /// [`OccluderGeometry::ComputedUi`], whose rectangle only the UI layout
    /// knows — the host supplies it via [`Self::from_rect`].
    pub fn self_resolved(self, display: ScreenRect) -> Option<ScreenOcclusion> {
        let rect = match self.geometry {
            OccluderGeometry::ComputedUi => return None,
            OccluderGeometry::Explicit(rect) => rect,
            OccluderGeometry::Anchored {
                anchor,
                offset_px,
                size_px,
            } => anchor.resolve_rect(display, offset_px, size_px),
        };
        Some(self.from_rect(rect))
    }

    /// Occupancy from a `bevy_ui` computed layout.
    ///
    /// `size_physical` is `ComputedNode::size` and `transform` is the node's
    /// `UiGlobalTransform`, both in physical pixels; `inverse_scale` is
    /// `ComputedNode::inverse_scale_factor`. Returns `None` for a node with no
    /// area, which cannot occlude anything.
    ///
    /// The ONE projection from computed UI layout into occupancy — the host
    /// collector and any producer testing its own occupancy call this same
    /// function rather than each doing the DPI arithmetic.
    ///
    /// # Why all four corners
    ///
    /// `UiGlobalTransform` is an [`Affine2`], not a translation: `UiTransform`
    /// carries `scale` and `rotation`, and `ui_layout_system` multiplies each
    /// node's local affine into its parent's, so a transformed ancestor reaches
    /// here too. Reading only the translation reported a rotated or scaled
    /// node's occupancy as its UNTRANSFORMED box centred at the right place —
    /// correct for the identity transform every current producer happens to
    /// use, and wrong the moment one animates a HUD panel in with a scale
    /// tween.
    ///
    /// Occupancy is axis-aligned by contract (the carve consumes rectangles),
    /// so the honest answer for a rotated node is the bounding box of its four
    /// transformed corners. The node's local frame is centred on the origin,
    /// spanning `-size/2 ..= size/2` — the same convention
    /// `ComputedNode::contains_point` inverts against.
    pub fn from_computed_ui(
        self,
        size_physical: ae::Vec2,
        transform: bevy::math::Affine2,
        inverse_scale: f32,
    ) -> Option<ScreenOcclusion> {
        if !inverse_scale.is_finite() || inverse_scale <= 0.0 {
            return None;
        }
        let half = size_physical * 0.5;
        if half.x <= 0.0 || half.y <= 0.0 {
            return None;
        }

        let mut min = ae::Vec2::splat(f32::INFINITY);
        let mut max = ae::Vec2::splat(f32::NEG_INFINITY);
        for corner in [
            ae::Vec2::new(-half.x, -half.y),
            ae::Vec2::new(half.x, -half.y),
            ae::Vec2::new(-half.x, half.y),
            ae::Vec2::new(half.x, half.y),
        ] {
            let point = transform.transform_point2(corner) * inverse_scale;
            min = min.min(point);
            max = max.max(point);
        }
        if !min.is_finite() || !max.is_finite() {
            return None;
        }

        let rect = ScreenRect { min, max };
        // A degenerate transform (zero scale, say) collapses the box; nothing
        // with no area occludes anything.
        if rect.is_empty() {
            return None;
        }
        Some(self.from_rect(rect))
    }

    /// Apply this occluder's purpose and padding to an already-known rect.
    pub fn from_rect(self, rect: ScreenRect) -> ScreenOcclusion {
        ScreenOcclusion {
            purpose: self.purpose,
            rect: ScreenRect {
                min: rect.min - self.padding_px,
                max: rect.max + self.padding_px,
            },
        }
    }
}

/// A resolved occupied region — the plain data the pure resolver consumes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenOcclusion {
    pub purpose: ScreenOcclusionPurpose,
    pub rect: ScreenRect,
}

// ---------------------------------------------------------------------------
// Axis 4 — activation
// ---------------------------------------------------------------------------

/// The stable presentation environment a profile is selected for.
///
/// **Stable is the point.** This must not follow the most recent input device:
/// glyphs may change the instant a gamepad is touched, but the gameplay
/// viewport and camera framing hold for the session (or until the participant
/// changes the preference explicitly). A camera that recomposes because a thumb
/// left the glass is a bug, not a feature.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresentationEnvironment {
    #[default]
    Desktop,
    /// Touch is the primary way this session is played: virtual controls
    /// occupy the display for its duration.
    TouchPrimary,
    /// Physically-attached controls on a small display (handhelds, Steam Deck).
    Handheld,
}

// ---------------------------------------------------------------------------
// Surround + HUD
// ---------------------------------------------------------------------------

/// What fills the display outside a fixed-aspect gameplay rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SurroundPolicy {
    /// There is no surround (full-bleed), or the game wants none drawn.
    #[default]
    None,
    /// Plain bars.
    Solid,
    /// The game draws its own surround presentation.
    GameAuthored,
    /// Non-mechanical world continues into the surround. The gameplay
    /// rectangle stays the authoritative view.
    DecorativeWorldExtension,
}

/// Where HUD elements prefer to live.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HudLayoutPolicy {
    #[default]
    OverGameplay,
    /// Use the surround when the viewport policy leaves any; fall back to
    /// overlaying gameplay when it does not.
    PreferSurround,
}

/// A named region of the display outside the gameplay rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SurroundRegion {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NamedScreenRect {
    pub region: SurroundRegion,
    pub rect: ScreenRect,
}

// ---------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------

/// One complete presentation policy: all four axes resolved for a single
/// environment.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GameplayPresentationProfile {
    pub viewport: GameplayViewportPolicy,
    pub framing: SubjectFramingPolicy,
    pub surround: SurroundPolicy,
    pub hud: HudLayoutPolicy,
    /// Whether on-screen controls should be kept out of the gameplay
    /// rectangle. Separate from [`Self::hud`] because a game may well want its
    /// score in the surround while its controls still overlay, or the reverse.
    pub controls: ControlPlacementPolicy,
}

impl GameplayPresentationProfile {
    pub fn full_bleed() -> Self {
        Self::default()
    }

    pub fn fixed_aspect(width: f32, height: f32) -> Self {
        Self {
            viewport: GameplayViewportPolicy::FixedAspect {
                aspect: AspectRatio::new(width, height),
                fit: FixedAspectFit::Center,
            },
            ..Self::default()
        }
    }

    pub fn with_fit(mut self, fit: FixedAspectFit) -> Self {
        if let GameplayViewportPolicy::FixedAspect { fit: slot, .. } = &mut self.viewport {
            *slot = fit;
        }
        self
    }

    pub fn with_soft_framing(mut self, profile: SoftFramingProfile) -> Self {
        self.framing = SubjectFramingPolicy::SoftSafeRegion(profile);
        self
    }

    pub fn with_occlusion_aware_framing(mut self, profile: SoftFramingProfile) -> Self {
        self.framing = SubjectFramingPolicy::OcclusionAware(profile);
        self
    }

    pub fn with_surround(mut self, surround: SurroundPolicy) -> Self {
        self.surround = surround;
        self
    }

    /// Reserve the surround for HUD and controls.
    ///
    /// "Reserve" is a PREFERENCE, not a promise: whether the controls actually
    /// fit is a property of the display, resolved into
    /// [`ResolvedControlRegions::placement`] every frame.
    pub fn with_reserved_surround(mut self) -> Self {
        self.surround = SurroundPolicy::Solid;
        self.hud = HudLayoutPolicy::PreferSurround;
        self.controls = ControlPlacementPolicy::PreferSurround;
        self
    }

    pub fn with_control_placement(mut self, controls: ControlPlacementPolicy) -> Self {
        self.controls = controls;
        self
    }
}

/// A game's declared presentation, per stable environment.
///
/// `default` always applies; the optional entries override it. A game that
/// declares nothing gets full-bleed normal framing — today's behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GameplayPresentationProfiles {
    pub default: GameplayPresentationProfile,
    pub touch_primary: Option<GameplayPresentationProfile>,
    pub handheld: Option<GameplayPresentationProfile>,
}

impl GameplayPresentationProfiles {
    pub fn uniform(profile: GameplayPresentationProfile) -> Self {
        Self {
            default: profile,
            touch_primary: None,
            handheld: None,
        }
    }

    pub fn builder() -> GameplayPresentationProfilesBuilder {
        GameplayPresentationProfilesBuilder::default()
    }

    pub fn for_environment(
        &self,
        environment: PresentationEnvironment,
    ) -> &GameplayPresentationProfile {
        match environment {
            PresentationEnvironment::Desktop => &self.default,
            PresentationEnvironment::TouchPrimary => {
                self.touch_primary.as_ref().unwrap_or(&self.default)
            }
            PresentationEnvironment::Handheld => self.handheld.as_ref().unwrap_or(&self.default),
        }
    }
}

#[derive(Default)]
pub struct GameplayPresentationProfilesBuilder {
    profiles: GameplayPresentationProfiles,
}

impl GameplayPresentationProfilesBuilder {
    pub fn default_profile(mut self, profile: GameplayPresentationProfile) -> Self {
        self.profiles.default = profile;
        self
    }

    pub fn touch_primary(mut self, profile: GameplayPresentationProfile) -> Self {
        self.profiles.touch_primary = Some(profile);
        self
    }

    pub fn handheld(mut self, profile: GameplayPresentationProfile) -> Self {
        self.profiles.handheld = Some(profile);
        self
    }

    pub fn build(self) -> GameplayPresentationProfiles {
        self.profiles
    }
}

/// App-local map from shell route id to the profiles that route declared.
///
/// Providers register into this at authoring time; exactly one selection
/// system (in the provider crate, which is the only layer that knows about
/// routes) copies the active entry into
/// [`ActiveGameplayPresentationProfiles`]. The host reads only the active
/// resource, so it never learns a route, an experience, or a game name.
#[derive(Resource, Default)]
pub struct GameplayPresentationProfileCatalog {
    by_route: std::collections::BTreeMap<String, GameplayPresentationProfiles>,
}

impl GameplayPresentationProfileCatalog {
    pub fn insert(&mut self, route_id: impl Into<String>, profiles: GameplayPresentationProfiles) {
        self.by_route.insert(route_id.into(), profiles);
    }

    pub fn get(&self, route_id: &str) -> Option<&GameplayPresentationProfiles> {
        self.by_route.get(route_id)
    }

    pub fn is_empty(&self) -> bool {
        self.by_route.is_empty()
    }

    pub fn routes(&self) -> impl Iterator<Item = &str> {
        self.by_route.keys().map(String::as_str)
    }
}

/// Ordering handle for the host's resolve-and-publish cluster.
///
/// Lives here rather than in the host because the PROVIDER layer must order
/// its profile selection before it — a selection that landed after the resolve
/// would show one frame of the previous experience's viewport on every switch
/// — and the provider crate cannot see `ambition_host`.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct GameplayPresentationSet;

/// The profiles in force right now.
///
/// Defaults to full-bleed normal framing — which is exactly today's behavior,
/// and what a route with no declaration (the launcher, a menu, a provider that
/// declared nothing) correctly gets.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct ActiveGameplayPresentationProfiles(pub GameplayPresentationProfiles);

// ---------------------------------------------------------------------------
// Resolved product
// ---------------------------------------------------------------------------

/// THE resolved presentation layout, shared by every consumer.
///
/// No camera, HUD, touch, pointer, or transition system should independently
/// recalculate margins — they read this.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ResolvedGameplayPresentation {
    /// The full physical display.
    pub display_rect: ScreenRect,
    /// The display minus platform safe-area insets.
    pub display_safe_rect: ScreenRect,
    /// Where the gameplay camera renders.
    pub gameplay_rect: ScreenRect,
    /// Where the controlled subject should preferably stay, in pixels.
    pub subject_safe_rect: ScreenRect,
    /// The same region normalized WITHIN [`Self::gameplay_rect`] — the form
    /// the camera resolver consumes.
    pub subject_safe_region: NormalizedScreenRegion,
    /// Soft-framing tuning when framing is active; `None` for normal framing.
    pub soft_framing: Option<SoftFramingProfile>,
    pub surround: SurroundPolicy,
    pub hud: HudLayoutPolicy,
    /// Display regions outside the gameplay rectangle, if any.
    pub surround_rects: Vec<NamedScreenRect>,
    /// Where the on-screen controls and HUD actually go, and which rung of the
    /// fallback ladder that placement took.
    pub controls: ResolvedControlRegions,
    /// The occupancy this layout was composed against, resolved to pixels.
    pub occlusions: Vec<ScreenOcclusion>,
}

impl Default for ResolvedGameplayPresentation {
    fn default() -> Self {
        let rect = ScreenRect::from_min_size(
            ae::Vec2::ZERO,
            ae::Vec2::new(ae::config::WINDOW_W as f32, ae::config::WINDOW_H as f32),
        );
        Self {
            display_rect: rect,
            display_safe_rect: rect,
            gameplay_rect: rect,
            subject_safe_rect: rect,
            subject_safe_region: NormalizedScreenRegion::FULL,
            soft_framing: None,
            surround: SurroundPolicy::None,
            hud: HudLayoutPolicy::OverGameplay,
            surround_rects: Vec::new(),
            controls: ResolvedControlRegions::default(),
            occlusions: Vec::new(),
        }
    }
}

impl ResolvedGameplayPresentation {
    /// Whether the gameplay rectangle is smaller than the safe display, i.e.
    /// whether any surround exists to draw or lay HUD into.
    pub fn has_surround(&self) -> bool {
        !self.surround_rects.is_empty()
    }

    pub fn surround_rect(&self, region: SurroundRegion) -> Option<ScreenRect> {
        self.surround_rects
            .iter()
            .find(|named| named.region == region)
            .map(|named| named.rect)
    }

    /// The largest surround rectangle, when HUD prefers the surround.
    /// Every part of the PHYSICAL display the gameplay camera does not draw.
    ///
    /// Distinct from [`Self::surround_rects`], which is safe-area-relative and
    /// answers "where may HUD live". This answers "what must be painted": a
    /// Bevy camera with a viewport never clears outside it, so whatever these
    /// rectangles cover is undefined until something fills it. A fixed-aspect
    /// profile therefore OWES the display a surround, and this is the region
    /// it owes.
    pub fn letterbox_rects(&self) -> Vec<NamedScreenRect> {
        let display = self.display_rect;
        let gameplay = self.gameplay_rect;
        let mut out = Vec::new();
        let mut push = |region, rect: ScreenRect| {
            if rect.width() > 0.5 && rect.height() > 0.5 {
                out.push(NamedScreenRect { region, rect });
            }
        };
        push(
            SurroundRegion::Left,
            ScreenRect::from_corners(display.min, ae::Vec2::new(gameplay.min.x, display.max.y)),
        );
        push(
            SurroundRegion::Right,
            ScreenRect::from_corners(ae::Vec2::new(gameplay.max.x, display.min.y), display.max),
        );
        push(
            SurroundRegion::Top,
            ScreenRect::from_corners(
                ae::Vec2::new(gameplay.min.x, display.min.y),
                ae::Vec2::new(gameplay.max.x, gameplay.min.y),
            ),
        );
        push(
            SurroundRegion::Bottom,
            ScreenRect::from_corners(
                ae::Vec2::new(gameplay.min.x, gameplay.max.y),
                ae::Vec2::new(gameplay.max.x, display.max.y),
            ),
        );
        out
    }

    /// The surround area left for HUD in a named region, once controls have
    /// taken theirs.
    ///
    /// The author API for a HUD surface that wants to live in the surround:
    /// ask for the region you want, get `None` when this profile and display
    /// leave none, and fall back to overlaying gameplay yourself. Deliberately
    /// not a layout engine — a HUD knows its own size and anchor, and only
    /// needs to be told what rectangle is available.
    pub fn hud_region(&self, region: SurroundRegion) -> Option<ScreenRect> {
        self.controls
            .hud
            .iter()
            .find(|named| named.region == region)
            .map(|named| named.rect)
    }

    /// Whether the active profile asked for HUD in the surround at all.
    pub fn prefers_surround_hud(&self) -> bool {
        self.hud == HudLayoutPolicy::PreferSurround
    }

    pub fn largest_surround_rect(&self) -> Option<ScreenRect> {
        self.surround_rects
            .iter()
            .map(|named| named.rect)
            .max_by(|a, b| a.area().total_cmp(&b.area()))
    }
}
