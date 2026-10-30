//! Content-free entity markers shared by reusable mechanics and presentation.

use bevy::prelude::*;

/// Marks a body in the player population; zero or many may exist.
///
/// Generic simulation should use body capabilities/control authority instead.
/// Use [`PrimaryPlayer`] for home-avatar policy and [`ControlledSubject`] for the
/// body currently driven by primary local control.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerEntity;

/// Entity currently driven by the primary local control authority.
///
/// `None` is valid while no primary controlled subject is resolved.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlledSubject(pub Option<Entity>);

/// Ordered bodies to frame when no local authority drives a subject.
///
/// The session/match publishes this list; the camera does not infer which bodies
/// belong in the frame. Empty means no cast has been declared.
#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct FramedCast(pub Vec<Entity>);

/// HOW THE CAMERA FRAMES THE CAST: the box it asks for each tick, and how
/// fast the view follows that box. The engine's values frame an exploration
/// party; a platform fighter declares its own (`MatchRules::cast_framing`),
/// and the match resets it when it ends.
///
/// The model is Melee's (`cm/camera.c`): each member asks for a box around
/// itself, padded more when fewer members share the screen; the union of
/// those boxes, stopped at the camera box, is the target; and the view's
/// centre (pan) and size (zoom) follow the target separately, with zooming
/// out and zooming in at their own rates.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct CastFramingTuning {
    /// How far the view may close in on the cast, as a fraction of the base
    /// view. The cast never pushes it below this.
    pub min_view_scale: f32,
    /// Room kept around each member's body, in world units, on every side.
    pub margin: f32,
    /// The factor on `margin` for 1, 2, 3 and 4 or more members. A duel
    /// pads each fighter more than a four-way fight, whose spread already
    /// opens the view (Melee: 1.5, 1.32, 1.16, 1.0).
    pub margin_by_count: [f32; 4],
    /// The share of the view's height kept below the cast's box, so the
    /// ground stays in view and the fighters sit above the centre. A share,
    /// not a length: a wide view keeps more floor (Melee lowers the bottom
    /// with the camera's distance).
    pub floor_share: f32,
    /// The rate the view's centre follows the target's, in Hz.
    pub pan_hz: f32,
    /// The rate the view opens toward a larger target, in Hz. `None` opens
    /// at once, so a body is never cut off while exploring.
    pub zoom_out_hz: Option<f32>,
    /// The rate the view closes toward a smaller target, in Hz.
    pub zoom_in_hz: f32,
    /// How far past the room the view may look. `None` stops at the room.
    pub bounds: Option<CastCameraBounds>,
    /// A member wholly outside the view is shown in a bubble at the view's
    /// edge. `None` shows nothing: the member is simply off the screen.
    pub bubble: Option<CastBubble>,
}

/// THE OFF-SCREEN BUBBLE of a cast member: a ring at the view's edge, on the
/// line from the view's centre to the member, with a live view of the member
/// inside it. A platform fighter's camera stops short of the blast line, so a
/// launched fighter leaves the frame while still in play; the bubble keeps
/// that fighter readable until it dies or comes back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CastBubble {
    /// The bubble's width on screen, in logical pixels.
    pub diameter_px: f32,
    /// The world units across the view inside the bubble.
    pub span: f32,
    /// The ring's colour per cast member, in cast order (RGBA, sRGB).
    pub rings: [[f32; 4]; 4],
    /// The colour behind the member inside the bubble (RGBA, sRGB).
    pub backdrop: [f32; 4],
}

/// THE CAMERA BOX of a cast: how far past the room's edges the view may
/// reach, in world units. A platform fighter kills a body well past the room
/// (`World::edges`); its camera stops short of that line, so a launched body
/// visibly leaves the frame instead of the camera showing the void.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CastCameraBounds {
    pub side: f32,
    pub rise: f32,
    pub fall: f32,
}

impl CastFramingTuning {
    /// The margin for a cast of `members`.
    pub fn margin_for(&self, members: usize) -> f32 {
        self.margin * self.margin_by_count[members.clamp(1, 4) - 1]
    }
}

impl Default for CastFramingTuning {
    fn default() -> Self {
        Self {
            min_view_scale: 0.75,
            margin: 48.0,
            margin_by_count: [1.0; 4],
            floor_share: 0.0,
            // The follow camera's own target rate, so a party frames as a
            // single body is followed.
            pan_hz: 8.0,
            zoom_out_hz: None,
            zoom_in_hz: 10.0,
            bounds: None,
            bubble: None,
        }
    }
}

/// Marks the exploration home body used by primary-player save/respawn policy.
///
/// Control may move elsewhere; use [`ControlledSubject`] for current control
/// authority. This is also distinct from [`crate::body::PrimaryBody`], which
/// selects live room-gravity resolution.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrimaryPlayer;

/// Query filter for a body carrying both [`PlayerEntity`] and [`PrimaryPlayer`].
pub type PrimaryPlayerOnly = (With<PlayerEntity>, With<PrimaryPlayer>);

/// Query filter for a body that collects a touched pickup: the player
/// population, or any body currently driven through possession.
///
/// ⭐ IT LIVES HERE, NOT IN THE ACTOR KERNEL, so a consumer BELOW the kernel can
/// name it. The filter is composed of nothing but [`PlayerEntity`] and
/// [`ControlClaims`](crate::temporary_control::ControlClaims), both of
/// which are already in this crate — while it sat in
/// `actor_monolith::body::pickups` it was the only reason the
/// world-item collect pass could not leave the kernel with it. Stating the rule
/// ONCE, here, is also what keeps it from being restated per system.
///
/// ⚠ `PlayerEntity` remains sufficient on its own: a player body whose brain is
/// temporarily absent still collects.
pub type TouchCollectorFilter = bevy::prelude::Or<(
    With<PlayerEntity>,
    With<crate::temporary_control::ControlClaims>,
)>;

/// The VALUE half of [`TouchCollectorFilter`], for a pass that has already
/// fetched the two facts and cannot re-ask the query.
///
/// ⛔ THE TWO MUST AGREE, which is why they are adjacent: the filter decides who
/// a query RETURNS and this decides whether a returned body actually collects,
/// and a pass that mixes one rule with the other's population silently changes
/// who picks things up. `PlayerEntity` (here, `in_player_population`) is
/// sufficient on its own — a player body whose brain is temporarily absent still
/// collects.
pub fn body_collects_on_touch(
    in_player_population: bool,
    claims: Option<&crate::temporary_control::ControlClaims>,
) -> bool {
    in_player_population
        || claims.is_some_and(|claims| {
            claims.holds(crate::temporary_control::ControlClaimant::Possession)
        })
}
