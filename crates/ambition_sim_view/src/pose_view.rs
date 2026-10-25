//! Per-body presentation POSE read-model for player-bodied entities (E4).
//!
//! `BodyPoseView` is the plain-data snapshot the renderer draws a
//! player-bodied entity from — the same role [`ActorAnimIndex`] plays for
//! id-keyed actor visuals, expressed as a COMPONENT because a player body's
//! sprite lives on the body entity itself (no id-keyed visual to join
//! through). Rebuilt once per tick, sim-side, LAST in the sim tail
//! (`FeatureViewSync`); presentation reads ONLY this and never queries the
//! live `Body*` clusters. Extraction is a pure function of sim state — no
//! caching across ticks — so a rollback resim rebuilds it for free.
//!
//! `ShieldRingsView` is the pooled-ring analogue for the bubble-shield
//! visual: EVERY body's raised shield (player and brain-driven alike)
//! materializes one row, so the render pool is a pure consumer.

use ambition_characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer2d_shared_tangle::lifecycle::{PlayerVisual, PosedBody};
use ambition_sprite_sheet::character::CharacterAnim;
use bevy::prelude::{Commands, Entity, Or, Query, Res, ResMut, Resource, With};

/// How long a body draws its `turnaround` row after its facing flips on the
/// ground. The row is slaved to this time, so its frames fit it whatever the
/// sheet's frame duration is. A sheet with no such row draws the body's own
/// pose.
pub const TURN_SECS: f32 = 0.22;

/// How long the head takes to go up (or down) when the stick is held up.
pub const LOOK_UP_SECS: f32 = 0.12;

/// How long a body takes to go down into its crouch, and to stand up from it.
pub const CROUCH_SECS: f32 = 0.12;

/// The row of the way down into a crouch: its first frame stands, its last
/// is the crouch. A stand up from the crouch is the same row, backward.
const CROUCH_CLIP: &str = "crouch_start";

/// The poses of a body in its crouch.
fn crouched(anim: CharacterAnim) -> bool {
    matches!(anim, CharacterAnim::Crouch | CharacterAnim::CrouchWalk | CharacterAnim::Crawl)
}

/// How long a crouched body takes to go down onto its hands when it moves,
/// and to come back up into its crouch when it stops.
pub const CRAWL_SECS: f32 = 0.15;

/// The row of the way down from the crouch into the crawl: its first frame is
/// the crouch, its last the crawl. Backward, it is the way back up.
const CRAWL_CLIP: &str = "crawl_start";

/// The poses of a body that moves in its crouch.
fn crawling(anim: CharacterAnim) -> bool {
    matches!(anim, CharacterAnim::CrouchWalk | CharacterAnim::Crawl)
}

/// How long a body that stops walking or running draws its `walk_stop` row,
/// slaved to this time.
pub const STOP_SECS: f32 = 0.2;

/// The row of a stop.
const STOP_CLIP: &str = "walk_stop";

/// The row of a look up. A body that looks up while it walks or runs has its
/// head turned as this row turns it (`BodyPoseView::head_up`).
pub const LOOK_UP_CLIP: &str = "idle_look_up";

/// The poses a body looks up in: standing (the `idle_look_up` row), walking
/// and running (its head only, over the walk: Jon, 2026-10-22, "if you press
/// up while moving it tilts the robots head up").
fn looks_up_in(anim: CharacterAnim) -> bool {
    anim == CharacterAnim::Idle || strides(anim)
}

/// The poses of a body that goes along its ground on its feet: a walk, a run
/// and a sprint.
fn strides(anim: CharacterAnim) -> bool {
    matches!(anim, CharacterAnim::Walk | CharacterAnim::Run | CharacterAnim::Sprint)
}

/// How far a body climbs in one cycle of its climb row (px).
pub const CLIMB_CYCLE_PX: f32 = 120.0;

/// The row of a climb.
const CLIMB_CLIP: &str = "climb";

/// How far a body climbs a wall in one cycle of its wall-climb row (px): two
/// pulls, one of each hand.
pub const WALL_CLIMB_CYCLE_PX: f32 = 96.0;

/// The row of a wall climb.
const WALL_CLIMB_CLIP: &str = "wall_climb";

/// How long a body that comes onto a wall draws its landing on it, before
/// the hold or the slide.
pub const WALL_LAND_SECS: f32 = 0.22;

/// The row of the landing on a wall, and of the kick off one.
const WALL_LAND_CLIP: &str = "wall_land";
const WALL_JUMP_CLIP: &str = "wall_jump";

/// The rows of a body on a wall that it came onto: a landing leads them.
fn on_a_wall(anim: CharacterAnim) -> bool {
    matches!(anim, CharacterAnim::WallGrab | CharacterAnim::WallSlide | CharacterAnim::WallLand)
}

/// The row of a turn.
const TURNAROUND_CLIP: &str = "turnaround";

/// The poses a turn is drawn from and in: on the ground, under no move.
fn turns_on_the_ground(anim: CharacterAnim) -> bool {
    anim == CharacterAnim::Idle || strides(anim)
}

/// How far up the stick is held for a body that stands to look up. Below the
/// cap of a walk (`ambition_input::control::WALK_AXIS_CAP`, 0.5), so the pose
/// is the same with the walk key held.
const LOOK_UP_AXIS: f32 = 0.4;

/// Sim-resolved presentation pose for one player-bodied entity: everything
/// the renderer needs to place, size, animate, and flash the sprite. Plain
/// data (Copy, no `Entity`/`Handle` borrows) — snapshot-safe by construction.
#[derive(bevy::prelude::Component, Clone, Debug)]
pub struct BodyPoseView {
    pub pos: ambition_platformer2d_core::Vec2,
    pub vel: ambition_platformer2d_core::Vec2,
    /// Current collision AABB size (crouch/morph compaction included).
    pub size: ambition_platformer2d_core::Vec2,
    /// Standing (base) AABB size — the denominator of the crouch stance
    /// ratio and the body-profile sprite scale. Falls back to `size` for a
    /// body without a `BodyBaseSize`.
    pub base_size: ambition_platformer2d_core::Vec2,
    pub facing: f32,
    /// Seconds since the drawn facing last flipped while the body stood,
    /// walked or ran, and `f32::INFINITY` before any. A picture fact only:
    /// the facing flips at once, and the `turnaround` row is drawn over the
    /// first [`TURN_SECS`] of it.
    pub turn_secs: f32,
    /// How far up this body looks, 0 to 1: it rises over [`LOOK_UP_SECS`]
    /// while the body stands and holds the stick up, and falls as fast when
    /// it lets go. The `idle_look_up` row is drawn at this fraction, so the
    /// head goes up and comes down through its frames.
    pub look_up: f32,
    /// How far up the head of a body that looks up while it walks or runs is
    /// turned, 0 to 1 (its `look_up` then): the head turns as the
    /// `idle_look_up` row turns it, over the row the body moves in. `0` for a
    /// body that stands, whose whole body is drawn in that row.
    pub head_up: f32,
    /// How far down into its crouch this body is, 0 to 1: it rises over
    /// [`CROUCH_SECS`] while the body crouches, and falls as fast when the
    /// body stands up. A body that stands still or crouches still between 0
    /// and 1 is drawn in the `crouch_start` row at this fraction.
    pub crouch: f32,
    /// How far down onto its hands a crouched body is, 0 to 1: it rises over
    /// [`CRAWL_SECS`] while the body moves in its crouch and falls as fast
    /// when it stops. Between 0 and 1 the body is drawn in the `crawl_start`
    /// row at this fraction. The crouch holds while this is above 0, so a
    /// body that stands up from a crawl comes up through its crouch.
    pub crawl: f32,
    /// Seconds since the body stopped walking or running on the ground, and
    /// `f32::INFINITY` before any stop. The first [`STOP_SECS`] of a stand
    /// are drawn in the `walk_stop` row.
    pub stop_secs: f32,
    /// Where in its climb row a body on a ladder is, 0 to 1. It moves with the
    /// distance the body climbs ([`CLIMB_CYCLE_PX`] a cycle), so a body that
    /// holds still on a ladder holds still in its picture.
    pub climb_phase: f32,
    /// How long the body has been on a wall (a grab or a slide), and
    /// `f32::INFINITY` when it is not on one. The first [`WALL_LAND_SECS`] of
    /// it are drawn as the landing on the wall.
    pub wall_secs: f32,
    /// Aerial/gravity roll angle (radians) — the sprite rotation.
    pub roll_angle: f32,
    /// `size.y / base_size.y`, clamped (0.1, 1.0] — the trimmed-sheet
    /// stance compaction the animator applies.
    pub stance_ratio_y: f32,
    /// Gravity direction used for the facing flip (the global field read the
    /// player path has always used).
    pub gravity_dir: ambition_platformer2d_core::Vec2,
    /// The picked animation row for this tick (the player picker over the
    /// body's real clusters).
    pub anim: CharacterAnim,
    /// What this body's ACTIVE MOVE asks to be drawn as, when one is
    /// playing — see `ActorAnimFrame::clip`, which is the same request on the
    /// actor road. `None` means *draw the semantic pose*.
    pub clip: Option<crate::ClipRequest>,
    /// Seconds remaining on the damage flash (`BodyCombat::hit_flash`).
    pub hit_flash_secs: f32,
    /// Seconds left on this body's PARRY CATCH — a parry that actually caught a
    /// strike, not a parry window standing open.
    ///
    /// `0.0` almost always; positive for a short beat starting on the tick a
    /// perfect shield turned a strike away, whether the strike was a swing or a
    /// shot. Armed by `BodyShieldState::catch_parry` at the two seams that
    /// resolve a parry, so one fact covers both routes.
    ///
    /// ⛔ never `BodyShieldState::parrying()`, which answers whether the WINDOW
    /// is open and is therefore true of every raised guard for a few ticks —
    /// a cue driven off that one fires on every shield raise.
    pub parry_flash_secs: f32,
    /// HOW HARD the hit currently freezing this body was, `0..=1`, and `0.0`
    /// when no hitlag is running.
    ///
    /// Resolved by `ambition_platformer2d_core::hit_response::hit_strength_fraction`
    /// from the hitlag the hit already set — the same quantity camera shake
    /// reads. Presentation never re-derives weight from damage, knockback or a
    /// move name, and hit resolution is untouched by anything that consumes it.
    pub hit_strength: f32,
    /// This body CANNOT BE STRUCK right now — the presentation half of
    /// `ambition_combat::util::body_vulnerable`, resolved here so no renderer
    /// re-derives hit eligibility from a pose or a move name.
    ///
    /// Covers every body-generic grant at once because the damage rule does:
    /// dodge / spot dodge / air dodge, tech and getup, the ledge grab's earned
    /// intangibility, the timed untouchable a respawn hands out, and the
    /// i-frames a hit leaves behind.
    pub unhittable: bool,
    /// WHY the canonical damage gate is closed, preserved as semantic
    /// presentation vocabulary. Shared route policy decides which causes opt
    /// into generic cues; character-owned effects may independently consume
    /// their own gameplay state and therefore compose with them.
    pub defense_cues: crate::DefenseCueCauses,
    pub hp_current: i32,
    pub hp_max: i32,
    /// The body is in morph-ball mode (draws the procedural sphere instead
    /// of the character sheet).
    pub morph_ball: bool,
    /// The body is UNDER the stage and must not be drawn at all.
    ///
    /// ⛔⛔ A PRESENTATION FACT, DERIVED — not a second authority. The truth is
    /// `BodyMode::Submerged` on the simulation body; this is that fact carried
    /// to the renderer the same way `morph_ball` is, because presentation reads
    /// the view and never the sim's components.
    pub submerged: bool,
    /// The body was knocked out past the blast envelope and is not drawn until
    /// it is placed again. Carried from the simulation's
    /// `KnockedOutOfTheWorld`, the same way `submerged` is.
    pub out_of_the_world: bool,
    /// WHERE THE WIRE THIS BODY IS HANGING FROM COMES DOWN FROM, in world
    /// space, or `None` for a body not on one.
    ///
    /// ⭐ THE SAME FIELD `FeatureView` CARRIES, and the pair is the point: this
    /// road is the session's exploration player, that one is every ACTOR, and a
    /// presentation rule stated on only one of them is not stated. Two
    /// read-models, one sentence — the way `submerged` above is.
    pub wire_anchor: Option<ambition_platformer2d_core::Vec2>,
    /// WHERE A LIVE GRAB IS REACHING TO, in world space — the far edge of the
    /// authored capture box along the body's facing, or `None` when no capture
    /// window is under the move clock.
    ///
    /// ⭐ A POINT, NOT A FLAG, for the reason `wire_anchor` beside it gives: a
    /// renderer that had to re-derive the reach would need the authored params,
    /// the facing and the acceleration frame, and would be a second authority on
    /// where the grab goes.
    pub grab_reach: Option<ambition_platformer2d_core::Vec2>,
    /// A POINT THIS BODY IS LINED TO, in world space — a ledge tether's latched
    /// anchor today, and `None` for a body lined to nothing.
    ///
    /// ⭐ THE THIRD LINE FACT, and deliberately its own. `wire_anchor` is where a
    /// body HANGS FROM; `grab_reach` is where a live capture box reaches TO,
    /// derived from the move clock. A reel is neither — the body is pulled TOWARD
    /// a point it latched — and folding it into either would make one field mean
    /// two mechanics with no way for a renderer to ask which.
    ///
    /// ⛔ Projected from `BodyLineAnchor`, an ENGINE component a ruleset inserts
    /// and removes (a module, through `ambition.feedback.body_line`), so this
    /// read model never learns what a tether is.
    pub line_anchor: Option<ambition_platformer2d_core::Vec2>,
    /// Fireball charge tier while the fire button is held (`None` when not
    /// charging): 0 / 1 / 2+ pick the charge-indicator size/alpha.
    pub charge_tier: Option<u8>,
    /// Where the charge hangs while it is held, from the body's centre (world
    /// units, world axes): the centre of the shot it will fire, so the charge
    /// is drawn where the shot appears (`ambition_combat::shot_birth`).
    pub charge_offset: Option<ambition_platformer2d_core::Vec2>,
    /// This body's SMASH CHARGE, `None` when it is not charging.
    ///
    /// Normalized `0..=1`: it appears when the hold latches, rises to `1.0` at
    /// maximum, and goes back to `None` the instant the move releases — so
    /// latched / building / loaded / released are all readable from this one
    /// value. Resolved by simulation (`MovePlayback::smash_charge_fraction`).
    ///
    /// ⛔ presentation must not re-derive it from move names or Startup
    /// progress: a tapped smash and a fully held one share both.
    pub move_charge: Option<f32>,
    /// The sprite quad this body's SHEET authored, when its geometry is
    /// sheet-authored (`SpritePosedBody`); `None` when the render must size the
    /// quad itself.
    ///
    /// Carried here rather than read as a component because the render layer
    /// does not depend on actor machinery — and because it is the same KIND of
    /// fact as `size` and `base_size` beside it: a sim-resolved geometry the
    /// renderer draws rather than re-derives.
    ///
    /// Its presence changes what the quad MEANS. Without it, `standing_render`
    /// is a guess about the art scaled by how far the collision box has drifted
    /// from its baseline — the ratio that drew Mary-O's tall form ~60% larger
    /// than the body it belonged to. With it, the box and the quad are two
    /// readings of ONE authored scale, so neither is derived from the other and
    /// there is no ratio to be wrong.
    pub authored_render: Option<ambition_platformer2d_core::Vec2>,
    /// Where to draw that quad, relative to the body centre — the companion
    /// to `authored_render` and gated on the same `SpritePosedBody`.
    pub authored_offset: Option<ambition_platformer2d_core::Vec2>,
    /// Whether `size`, `base_size` and the authored quad belong to the
    /// identity the body WEARS. See [`PoseGeometry`].
    pub geometry: PoseGeometry,
}

/// Whether a pose's geometry is the worn identity's, or the one before it.
///
/// ⛔ A pose can name one character and carry another's body. A form swap is
/// decided late in the tick (`sync_grown_form`, `FeatureInteraction`) and the
/// prepared body for the new identity is granted at the head of the NEXT one
/// (`CharacterProjection`), so the pose published in between reads
/// `mary_o_tall` with the small form's standing box and offset. A presentation
/// that finalizes from that snapshot keeps the wrong offset for the life of the
/// binding, because its one-time basis is keyed on the identity and the
/// identity does not change again when the geometry lands.
///
/// Settled is read off the body, not inferred: the kernel stamps
/// `ProjectedCharacterKit` with the id and cast generation of the prepared
/// body it granted, so the geometry is the worn identity's exactly when that
/// stamp names it — or when the worn identity has no prepared body, and the
/// body as constructed is the whole answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PoseGeometry {
    /// The geometry is the worn identity's.
    #[default]
    Settled,
    /// The body wears an identity whose prepared body has not been granted
    /// yet; the geometry here is the previous identity's. A presentation may
    /// keep drawing what it has, or draw provisionally, but must not finalize.
    Pending,
}

impl PoseGeometry {
    pub fn of(
        worn: Option<&ambition_characters::actor::WornCharacter>,
        granted: Option<&ambition_platformer2d_actor_spawn::ProjectedCharacterKit>,
        registry: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    ) -> Self {
        let Some(worn) = worn else {
            return Self::Settled;
        };
        let prepared = registry.map(|r| (r.get(worn.id()).is_some(), r.generation()));
        let settled = match (granted, prepared) {
            (Some(kit), Some((has, generation))) => {
                has && kit.id == worn.id() && kit.generation == generation
            }
            (Some(kit), None) => kit.id == worn.id(),
            // Nothing granted: settled only if there is nothing to grant.
            (None, Some((has, _))) => !has,
            (None, None) => true,
        };
        if settled {
            Self::Settled
        } else {
            Self::Pending
        }
    }

    pub fn is_settled(self) -> bool {
        self == Self::Settled
    }
}

impl Default for BodyPoseView {
    fn default() -> Self {
        Self {
            pos: ambition_platformer2d_core::Vec2::ZERO,
            vel: ambition_platformer2d_core::Vec2::ZERO,
            size: ambition_platformer2d_core::Vec2::ONE,
            base_size: ambition_platformer2d_core::Vec2::ONE,
            facing: 1.0,
            turn_secs: f32::INFINITY,
            look_up: 0.0,
            head_up: 0.0,
            crouch: 0.0,
            crawl: 0.0,
            stop_secs: f32::INFINITY,
            climb_phase: 0.0,
            wall_secs: f32::INFINITY,
            // A body with no pose yet is playing no move.
            clip: None,
            roll_angle: 0.0,
            stance_ratio_y: 1.0,
            gravity_dir: ambition_platformer2d_core::Vec2::Y,
            anim: CharacterAnim::Idle,
            hit_flash_secs: 0.0,
            parry_flash_secs: 0.0,
            hit_strength: 0.0,
            unhittable: false,
            defense_cues: crate::DefenseCueCauses::NONE,
            hp_current: 0,
            hp_max: 0,
            morph_ball: false,
            submerged: false,
            out_of_the_world: false,
            wire_anchor: None,
            grab_reach: None,
            line_anchor: None,
            charge_tier: None,
            charge_offset: None,
            move_charge: None,
            authored_render: None,
            authored_offset: None,
            geometry: PoseGeometry::Settled,
        }
    }
}

/// Only `BodyKinematics` is REQUIRED: a partial body (a test fixture that
/// spawns `PlayerVisual` + kinematics alone) still gets its transform facts;
/// the anim pick needs the full movement/ability cluster set (the same set
/// `animate_player` demanded) and holds `Idle` when any piece is absent —
/// exactly the frames the old live-query path would have skipped.
#[allow(clippy::type_complexity)]
pub fn rebuild_body_pose_views(
    mut commands: Commands,
    // The reference the hitlag law scales from, so the published strength is a
    // fraction rather than a raw freeze presentation would have to interpret.
    feel: Option<Res<ambition_combat::feel::Platformer2dFeelTuningMonolith>>,
    // The cast a granted body is compared against. See `PoseGeometry`.
    // The running session's frozen cast, or the published one with no
    // session (`SessionCast`).
    registry: ambition_characters::prepared::SessionCast,
    // The clock of the turn (`BodyPoseView::turn_secs`).
    time: Option<Res<bevy::prelude::Time>>,
    // Where a charging body's shot is born, so its charge is drawn there.
    landmarks: ambition_combat::body_landmarks::BodyLandmarks,
    mut bodies: Query<
        (
            (
                Entity,
                &ambition_platformer2d_core::BodyKinematics,
                Option<&ambition_platformer2d_core::BodyGroundState>,
                Option<&ambition_platformer2d_core::BodyMotionFacts>,
                Option<&ambition_platformer2d_core::BodyFlightState>,
                Option<&BodyCombat>,
                Option<&ambition_characters::actor::BodyAnimFacts>,
                Option<&ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState>,
                // This body's own resolved basis: the locomotion metric is
                // measured along ITS run axis and the facing flip follows ITS
                // gravity. Not `GravityField`, which mirrors the PRIMARY body.
                Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
                // The wall a wall climb holds: the row faces it.
                Option<&ambition_platformer2d_core::BodyWallState>,
            ),
            (
                Option<&ambition_platformer2d_core::BodyModeState>,
                Option<&ambition_platformer2d_core::BodyEnvironmentContact>,
                Option<&ambition_platformer2d_core::BodyAbilities>,
                Option<&ambition_platformer2d_core::BodyShieldState>,
                // With the playback, it gives the swing (`melee_swing_of`).
                Option<&ambition_combat::moveset::ActorMoveset>,
                Option<&ambition_platformer2d_core::BodyBaseSize>,
                Option<&BodyHealth>,
                Option<&ambition_platformer2d_shared_tangle::orientation::ActorRoll>,
                Option<&ambition_projectiles::PlayerProjectileState>,
                // The movement policy, for the body's own top speed (the
                // sprint row).
                Option<&ambition_platformer2d_core::MotionModel>,
            ),
            // ⛔ A THIRD GROUP BECAUSE BEVY'S QUERY TUPLE LIMIT IS FIFTEEN, not
            // sixteen — the group above reached sixteen and stopped being a
            // `QueryData` at all. The split is drawn where the SUBJECT changes:
            // everything below is the ART this body draws with, plus the row
            // this pass writes.
            (
                // Content-driven pose PIN — the SAME component the actor path
                // honours in `rebuild_actor_anim_index`. See the read below.
                Option<
                    &ambition_sprite_sheet::character::ActorAnimOverride,
                >,
                // The sheet-authored sprite quad, when this body's geometry is its art.
                //
                //  gated on `SpritePosedBody`, NOT on the render size alone.
                // `ActorRenderSize` is the SHARED sprite-quad component — several
                // spawn paths insert it from sheet metadata for bodies whose
                // collision box is still hand-authored. Reading it bare here
                // would tell the renderer "the sheet authored this geometry"
                // about every one of them, and hand it a quad that was never
                // derived from their boxes.
                Option<&ambition_combat::components::ActorRenderSize>,
                // The quad's PLACEMENT, published beside its size by the same
                // pass. Gated on `SpritePosedBody` below for the same reason the
                // size is: several spawn paths insert these from sheet metadata
                // for bodies whose collision box is still hand-authored.
                Option<&ambition_combat::components::ActorSpriteOffset>,
                bevy::prelude::Has<
                    ambition_sprite_sheet::character::SpritePosedBody,
                >,
                // Match/ruleset respawn protection is a semantic presentation
                // cause independent of the current Empowered implementation.
                bevy::prelude::Has<ambition_combat::stocks::RespawnGrace>,
                // The move this body is playing, so the drawn row can be the
                // one the move NAMES — the same request the actor path carries
                // on `ActorAnimFrame::clip` (sprite redirect P0).
                Option<&ambition_combat::moveset::MovePlayback>,
                // A ledge tether's latched anchor, published by whichever ruleset
                // owns the mechanic. See `BodyPoseView::line_anchor`.
                Option<&ambition_platformer2d_core::BodyLineAnchor>,
                // WHO the body is and whose body it was GRANTED — see `PoseGeometry`.
                Option<&ambition_characters::actor::WornCharacter>,
                Option<&ambition_platformer2d_actor_spawn::ProjectedCharacterKit>,
                bevy::prelude::Has<ambition_combat::stocks::KnockedOutOfTheWorld>,
                // The stick, for the look-up pose of a body that stands.
                Option<&ambition_characters::control::ActorControl>,
                Option<&mut BodyPoseView>,
            ),
        ),
        // ⛔⛔ WIDENED 2026-08-29: `With<PlayerVisual>` ALONE MEANT NO MATCH
        // FIGHTER EVER GOT A POSE VIEW. That marker is granted in exactly one
        // production place — the exploration player's avatar — so a seated body
        // was invisible to this read model, and the moveset inspector
        // reconstructed pose/clip/frame in JavaScript because the engine never
        // published it for the bodies it was looking at.
        //
        // ⭐ `PosedBody` IS THE OPT-IN, and it is separate from `PlayerVisual`
        // on purpose: that marker means "the player's own drawn avatar" and
        // other presentation keys on it, so granting it to every seat would turn
        // those on too.
        Or<(With<PlayerVisual>, With<PosedBody>)>,
    >,
) {
    // No feel tuning means no hitlag law to measure against: every body reports
    // no strength rather than a number derived from a reference nobody set.
    let hitlag_reference = feel.as_deref().map_or(0.0, |feel| feel.hitlag_time);
    for (
        (
            entity,
            kinematics,
            ground,
            motion_facts,
            flight,
            combat,
            anim_facts,
            blink_cam,
            body_frame,
            wall,
        ),
        (
            body_mode,
            env_contact,
            abilities,
            shield,
            moveset,
            base_size,
            health,
            roll,
            projectile_state,
            model,
        ),
        (
            anim_override,
            authored_render,
            authored_offset,
            sheet_authored_body,
            respawn_grace,
            playback,
            line_anchor,
            worn,
            granted,
            out_of_the_world,
            control,
            pose,
        ),
    ) in &mut bodies
    {
        let gravity_dir = body_down(body_frame);
        let base = base_size.map_or(kinematics.size, |b| b.base_size);
        let stance_ratio_y = base_size
            .map(|b| (kinematics.size.y / b.base_size.y.max(1.0)).clamp(0.1, 1.0))
            .unwrap_or(1.0);
        // The anim pick runs only over the FULL cluster set `animate_player`
        // used to require — a partial body keeps `Idle` (it never animated
        // before either) while its transform facts stay live.
        let pick = |swing| match (
            (ground, motion_facts, flight),
            (combat, anim_facts, blink_cam),
            (body_mode, env_contact, abilities, shield),
        ) {
            (
                (Some(ground), Some(motion_facts), Some(flight)),
                (Some(combat), Some(anim_facts), Some(blink_cam)),
                (Some(body_mode), Some(env_contact), Some(abilities), Some(shield)),
            ) => ambition_character_sprites::pick_player_anim(
                anim_facts,
                combat,
                blink_cam,
                swing,
                kinematics,
                ground,
                motion_facts,
                flight,
                body_mode,
                env_contact,
                abilities,
                shield,
                body_frame.map_or_else(
                    || {
                        ambition_platformer2d_core::AccelerationFrame::new(
                            ambition_platformer2d_core::DEFAULT_GRAVITY_DIR,
                        )
                    },
                    |f| f.basis(),
                ),
                model.map_or(0.0, ambition_platformer2d_core::MotionModel::commanded_top_speed),
            ),
            _ => CharacterAnim::Idle,
        };
        let swing = ambition_combat::moveset::melee_swing_of(playback, moveset);
        let anim = pick(swing.as_ref());
        // The move's row is the picture of the upper body: the legs are
        // drawn by what the body does under the move. The same ladder with
        // no swing says what that is. A body that stands has no such pose:
        // the row draws it whole.
        //
        // A slow fall is such a pose too: the legs row carries the boot jets
        // (`is_lower_body_track`), so a body that swings while it glides keeps
        // them. Without it the jets went out for the length of each swing.
        let legs = playback
            .filter(|playback| playback.spec.upper_body_at(playback.t))
            .map(|_| pick(None))
            .filter(|legs| {
                matches!(
                    legs,
                    CharacterAnim::Walk
                        | CharacterAnim::Run
                        | CharacterAnim::Sprint
                        | CharacterAnim::Jump
                        | CharacterAnim::Fall
                        | CharacterAnim::Fly
                        | CharacterAnim::FloatGlide
                )
            });
        // A body that stands, walks or runs and holds the stick up looks up.
        // It is a picture only: the control frame is read, and nothing is
        // written.
        let looking_up = looks_up_in(anim)
            && anim_override.is_none()
            && control.is_some_and(|control| control.0.locomotion.y < -LOOK_UP_AXIS);
        // A content pose PIN wins over the picked pose — the SAME rule `rebuild_actor_anim_index`
        // applies to every brain-driven actor, and it belongs here for the identical reason: a pose
        // the disposition-agnostic picker cannot infer (a shelled enemy's withdraw, a body
        // mid-power-up) is stated by the content that owns the state machine.
        let anim = anim_override.map(|o| o.0).unwrap_or(anim);
        // The first moments on a wall are the landing on it.
        let wall_secs = match pose.as_deref() {
            _ if !on_a_wall(anim) => f32::INFINITY,
            Some(before) if on_a_wall(before.anim) && before.wall_secs.is_finite() => {
                before.wall_secs + time.as_deref().map_or(0.0, bevy::prelude::Time::delta_secs)
            }
            _ => 0.0,
        };
        let anim = if anim_override.is_none() && wall_secs < WALL_LAND_SECS { CharacterAnim::WallLand } else { anim };
        let charge = playback
            .as_deref()
            .and_then(ambition_combat::moveset::MovePlayback::smash_charge_fraction);
        // A move may draw its owner MIRRORED — the crude spin read. Applied to
        // the published pose and nowhere else: the body's own `facing` is
        // untouched, so every rule that reads which way it is looking is
        // unaffected. See `MoveSpec::sprite_spin_hz`.
        let drawn_facing = if playback
            .as_deref()
            .is_some_and(ambition_combat::moveset::MovePlayback::sprite_mirrored_now)
        {
            -kinematics.facing
        } else if let Some(wall) = wall.filter(|wall| anim == CharacterAnim::WallClimb && wall.on_wall) {
            // A wall climb faces the wall it holds. A body in the air does not
            // turn, so one that came to the wall backward still faces away.
            -wall.wall_normal_x.signum()
        } else {
            kinematics.facing
        };
        // The turn: the drawn facing flipped between two poses on the ground.
        let free = playback.is_none_or(|playback| playback.settled()) && anim_override.is_none();
        let turn_secs = match pose.as_deref() {
            Some(before)
                if free
                    && before.facing != drawn_facing
                    && turns_on_the_ground(before.anim)
                    && turns_on_the_ground(anim) =>
            {
                0.0
            }
            Some(before) => before.turn_secs + time.as_deref().map_or(0.0, bevy::prelude::Time::delta_secs),
            None => f32::INFINITY,
        };
        let turning = free && turns_on_the_ground(anim) && turn_secs < TURN_SECS;
        let dt = time.as_deref().map_or(0.0, bevy::prelude::Time::delta_secs);
        // The look up rises while it is asked for and falls when it is not, as
        // the body stands, walks or runs; a body that does anything else has
        // its head level at once.
        let look_up = if free && looks_up_in(anim) {
            let before = pose.as_deref().map_or(0.0, |before| before.look_up);
            let step = dt / LOOK_UP_SECS;
            if looking_up { (before + step).min(1.0) } else { (before - step).max(0.0) }
        } else {
            0.0
        };
        // The crawl goes down onto the hands while the crouched body moves,
        // and comes up into the crouch when it stops or stands.
        let crawl = if crawling(anim) {
            let before = pose.as_deref().map_or(0.0, |before| before.crawl);
            if free { (before + dt / CRAWL_SECS).min(1.0) } else { 1.0 }
        } else if free && matches!(anim, CharacterAnim::Crouch | CharacterAnim::Idle) {
            let before = pose.as_deref().map_or(0.0, |before| before.crawl);
            (before - dt / CRAWL_SECS).max(0.0)
        } else {
            0.0
        };
        // The crouch goes down while the body crouches and comes up when it
        // stands, after the crawl is up. A move that crouches is in its crouch
        // at once; any other pose is out of it at once.
        let crouch = if crouched(anim) {
            let before = pose.as_deref().map_or(0.0, |before| before.crouch);
            if free { (before + dt / CROUCH_SECS).min(1.0) } else { 1.0 }
        } else if free && anim == CharacterAnim::Idle {
            let before = pose.as_deref().map_or(0.0, |before| before.crouch);
            if crawl > 0.0 { before } else { (before - dt / CROUCH_SECS).max(0.0) }
        } else {
            0.0
        };
        // The stop: the body stood after it walked or ran.
        let stop_secs = match pose.as_deref() {
            Some(before)
                if free
                    && anim == CharacterAnim::Idle
                    && strides(before.anim) =>
            {
                0.0
            }
            Some(before) => before.stop_secs + dt,
            None => f32::INFINITY,
        };
        // The climb row moves with the distance climbed.
        let climb_phase = {
            let before = pose.as_deref().map_or(0.0, |before| before.climb_phase);
            if anim == CharacterAnim::LadderClimb {
                (before + kinematics.vel.length() * dt / CLIMB_CYCLE_PX).fract()
            } else if anim == CharacterAnim::WallClimb {
                // A climb down plays the row backward, hand under hand.
                let up = -kinematics.vel.dot(gravity_dir);
                (before + up * dt / WALL_CLIMB_CYCLE_PX).rem_euclid(1.0)
            } else {
                before
            }
        };
        let charge_tier = projectile_state
            .and_then(|s| s.charging.map(|hold| s.charge_tuning.tier_for_hold(hold)));
        // The charge hangs where its shot will be born: the fireball of this
        // tier, aimed as the stick aims now.
        let charge_offset = charge_tier.map(|tier| {
            use ambition_projectiles::ProjectileKind::Fireball;
            let half_extent = Fireball
                .charged_spec(Fireball.spec(ambition_platformer2d_core::Vec2::ZERO, ambition_platformer2d_core::Vec2::X, 1.0), tier)
                .half_extent;
            let facing = if kinematics.facing.abs() < f32::EPSILON { 1.0 } else { kinematics.facing.signum() };
            let aim = control.map_or(ambition_platformer2d_core::Vec2::ZERO, |control| control.0.aim.vec());
            let basis = body_frame.map_or_else(
                || ambition_platformer2d_core::AccelerationFrame::new(ambition_platformer2d_core::DEFAULT_GRAVITY_DIR),
                |f| f.basis(),
            );
            ambition_combat::shot_birth::charge_shot_birth(
                &landmarks,
                entity,
                kinematics,
                |local| basis.to_world(local),
                gravity_dir,
                ambition_combat::shot_birth::local_fire_dir(aim, facing),
                half_extent,
            ) - kinematics.pos
        });
        let next = BodyPoseView {
            pos: kinematics.pos,
            vel: kinematics.vel,
            size: kinematics.size,
            base_size: base,
            facing: drawn_facing,
            turn_secs,
            look_up,
            // Not through a turn: the turn's row goes round through the front.
            head_up: if free && !turning && strides(anim) { look_up } else { 0.0 },
            crouch,
            crawl,
            stop_secs,
            climb_phase,
            wall_secs,
            roll_angle: roll.map_or(0.0, |r| r.angle),
            stance_ratio_y,
            gravity_dir,
            anim,
            //  a MOVE names its row; failing that, a fighter STATE does — the
            // same two-step the actor road takes, so the two never disagree.
            clip: playback
                .filter(|playback| !playback.settled())
                .map(|playback| crate::ClipRequest::for_move(&playback.spec.clip, playback.phase()).over_legs(legs))
                .or_else(|| {
                    // A row slaved to a fraction of the body's own: the turn,
                    // the crawl, the crouch, the stop, the look up, the climb. A sheet with none of these rows
                    // draws the body's own pose.
                    let slaved = |clip: &str, phase: f32| crate::ClipRequest {
                        clip: clip.to_string(),
                        fallbacks: Vec::new(),
                        phase: Some(phase),
                        legs: None,
                    };
                    if turning {
                        Some(slaved(TURNAROUND_CLIP, turn_secs / TURN_SECS))
                    } else if free
                        && 0.0 < crawl
                        && crawl < 1.0
                        && (crouched(anim) || anim == CharacterAnim::Idle)
                    {
                        Some(slaved(CRAWL_CLIP, crawl))
                    } else if free
                        && 0.0 < crouch
                        && crouch < 1.0
                        && matches!(anim, CharacterAnim::Crouch | CharacterAnim::Idle)
                    {
                        Some(slaved(CROUCH_CLIP, crouch))
                    } else if free && anim == CharacterAnim::Idle && stop_secs < STOP_SECS && look_up == 0.0 {
                        Some(slaved(STOP_CLIP, stop_secs / STOP_SECS))
                    } else if look_up > 0.0 && anim == CharacterAnim::Idle {
                        Some(slaved(LOOK_UP_CLIP, look_up))
                    } else if free && anim == CharacterAnim::LadderClimb {
                        Some(slaved(CLIMB_CLIP, climb_phase))
                    } else if free && anim == CharacterAnim::WallClimb {
                        Some(slaved(WALL_CLIMB_CLIP, climb_phase))
                    } else if free && anim == CharacterAnim::WallLand {
                        Some(slaved(WALL_LAND_CLIP, (wall_secs / WALL_LAND_SECS).min(1.0)))
                    } else if free && anim == CharacterAnim::WallJump {
                        // The kick off the wall, over the hold of its pose.
                        let left = anim_facts.map_or(0.0, |f| f.wall_jump_anim_timer);
                        let hold = ambition_characters::actor::WALL_JUMP_ANIM_HOLD_SECS;
                        Some(slaved(WALL_JUMP_CLIP, (1.0 - left / hold).clamp(0.0, 1.0)))
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    crate::ClipRequest::from_chain(ambition_character_sprites::body_state_clip(
                        motion_facts?,
                        ambition_character_sprites::FighterClipFacts {
                            held: anim_facts.is_some_and(|f| f.held),
                            holding: anim_facts.is_some_and(|f| f.holding),
                            guard_break: shield
                                .and_then(|s| s.break_phase())
                                .map(ambition_character_sprites::GuardBreakBeat::from_phase),
                            parrying: shield.is_some_and(|s| s.parrying()),
                            guard_stunned: shield.is_some_and(|s| s.stun_timer > 0.0),
                            ..Default::default()
                        }
                        .with_gestures(anim_facts),
                    )?)
                })
                // A HELD CHARGE outranks the move's own row, and only while it
                // is held: the whole point of the beat is that a fighter
                // winding up looks different from one swinging. It goes AHEAD
                // of the move's chain rather than replacing it, so a sheet that
                // authors no charge row draws exactly what it drew before.
                .map(|chain| match charge {
                    Some(_) => chain.ahead_of(crate::SMASH_CHARGE_CLIP),
                    None => chain,
                })
                .or_else(|| charge.map(|_| crate::ClipRequest::only(crate::SMASH_CHARGE_CLIP))),
            hit_flash_secs: combat.map_or(0.0, |c| c.hit_flash),
            parry_flash_secs: shield.map_or(0.0, |s| s.parry_caught_timer),
            hit_strength: ambition_platformer2d_core::hit_response::hit_strength_fraction(
                combat.map_or(0.0, |c| c.hitstop_timer),
                hitlag_reference,
            ),
            // THE DAMAGE RULE ITSELF, inverted — not a second reading of it. A
            // body missing one of these clusters cannot be protected by it, so
            // the default stands in.
            unhittable: !ambition_combat::util::body_vulnerable(
                health.map_or_else(ambition_characters::actor::Invulnerability::none, |h| {
                    h.health.invulnerable
                }),
                motion_facts.is_some_and(|m| m.evading()),
                &shield.copied().unwrap_or_default(),
                &combat.copied().unwrap_or_default(),
            ),
            defense_cues: crate::defense_cue_causes(
                health.map_or_else(ambition_characters::actor::Invulnerability::none, |h| {
                    h.health.invulnerable
                }),
                motion_facts,
                &shield.copied().unwrap_or_default(),
                &combat.copied().unwrap_or_default(),
                respawn_grace,
            ),
            hp_current: health.map_or(0, |h| h.current()),
            hp_max: health.map_or(0, |h| h.max()),
            morph_ball: body_mode
                .is_some_and(|m| m.body_mode == ambition_platformer2d_core::BodyMode::MorphBall),
            // The SAME predicate the actor road asks — see
            // `BodyMode::hides_the_body`. Two read-models, one sentence.
            submerged: body_mode.is_some_and(|m| m.body_mode.hides_the_body()),
            out_of_the_world,
            // The SAME projection the actor road reads — see
            // `BodyMotionFacts::wire_anchor`.
            wire_anchor: motion_facts.and_then(|m| m.wire_anchor),
            // ⛔ RESOLVED HERE because this is where the facing and the frame
            // are both in hand — the same reason the move timeline resolves its
            // own event offsets rather than publishing body-local ones.
            line_anchor: line_anchor.map(|anchor| anchor.0),
            grab_reach: playback.and_then(|pb| {
                pb.live_capture_reach().map(|reach| {
                    // ⛔ THE MOVE'S OWN FACING, not the drawn one. A move
                    // LOCKS its facing at start (the Smash convention — a swing
                    // does not re-aim mid-animation) and its volumes are placed
                    // with `MovePlayback::facing`. A line drawn from the
                    // presented facing would part company with the box it is
                    // meant to be showing the moment a fighter turns.
                    let side = if pb.facing >= 0.0 { 1.0 } else { -1.0 };
                    kinematics.pos
                        + ambition_platformer2d_core::Vec2::new(
                            reach.x * side,
                            reach.y,
                        )
                })
            }),
            charge_tier,
            charge_offset,
            move_charge: charge,
            authored_render: sheet_authored_body
                .then(|| authored_render.map(|r| r.0))
                .flatten(),
            authored_offset: sheet_authored_body
                .then(|| authored_offset.map(|o| o.0))
                .flatten(),
            geometry: PoseGeometry::of(worn, granted, registry.get()),
        };
        match pose {
            Some(mut pose) => *pose = next,
            None => {
                commands.entity(entity).insert(next);
            }
        }
    }
}

/// One raised bubble shield, resolved sim-side. The renderer positions one
/// pooled bubble sprite per row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShieldRingFact {
    pub pos: ambition_platformer2d_core::Vec2,
    pub size: ambition_platformer2d_core::Vec2,
    pub parrying: bool,
    /// `1.0` whole, `0.0` about to break — and `1.0` for a body whose guard is
    /// not a resource, so the bubble reads the same for every body that has one.
    pub integrity: f32,
    /// Seconds of SHIELDSTUN still owed — positive for exactly the beat after
    /// this guard absorbed a hit.
    ///
    /// Raw seconds, like `BodyPoseView::hit_flash_secs`: the timer is the
    /// resolved fact, and how long a hit should stay visible is a presentation
    /// constant. Publishing a normalized fraction instead would put a
    /// presentation decision in the simulation and need `ShieldTuning` here to
    /// make it.
    pub stun_secs: f32,
    /// How far through the beat of a blocked blow this guard is, `0.0` to
    /// `1.0` (`BodyShieldState::blow_phase`). `None` when no beat runs.
    ///
    /// Not [`Self::stun_secs`]: only a shield that is a resource owes
    /// shieldstun, and each guard that blocks a blow shows it.
    pub blow_phase: Option<f32>,
    /// Where that blow landed, from [`Self::pos`] (the middle of the bubble,
    /// not of the body), in world units.
    pub blow_from: ambition_platformer2d_core::Vec2,
    /// This body's OWN toward-feet direction — not the global field's, and not
    /// screen `+Y`.
    ///
    /// The bubble is an ellipse (wider than it is tall) and the ellipse belongs
    /// to the BODY, so under rotated gravity it has to rotate with it. A
    /// presentation that assumed screen axes would draw a wall-walker's guard
    /// lying on its side.
    pub gravity_dir: ambition_platformer2d_core::Vec2,
    /// The live room of the thing this row draws (`LiveRooms::of`). The
    /// visual is placed by that room's geometry and stamped with it.
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

/// Every body (player AND brain-driven actor) whose shield is currently
/// raised, in query order — the read-model behind the pooled bubble-shield
/// rings.
#[derive(Resource, Default, Clone, Debug)]
pub struct ShieldRingsView(pub Vec<ShieldRingFact>);

/// Both shield read-models in one pass over the shielded bodies: the raised
/// guards ([`ShieldRingsView`]) and the shattered ones ([`GuardBreaksView`]).
/// One observation cut, one traversal, each view in query order.
pub fn rebuild_shield_views(
    mut rings: ResMut<ShieldRingsView>,
    mut breaks: ResMut<GuardBreaksView>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    bodies: Query<(
        Entity,
        &ambition_platformer2d_core::BodyKinematics,
        &ambition_platformer2d_core::BodyShieldState,
        // Publish the presented pose because pooled shield rendering has no
        // owner entity left with which to resolve presentation offsets.
        Option<&crate::presented_pose::PresentedPose>,
        // The body's own shield tuning, so the ring can SHOW the resource
        // draining. A body with no motion model draws a whole ring.
        Option<&ambition_platformer2d_core::MotionModel>,
        // The body's OWN basis, so guard presentation is oriented to the
        // surface this body stands on rather than to the screen.
        Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    )>,
) {
    rings.0.clear();
    breaks.0.clear();
    for (entity, kin, shield, presented, model, frame) in &bodies {
        if shield.active {
            rings.0.push(shield_ring_fact(&live, entity, kin, shield, presented, model, frame));
        }
        if let Some(phase) = shield.break_phase() {
            breaks.0.push(GuardBreakFact {
                pos: presented.map_or(kin.pos, |p| p.presented()),
                size: kin.size,
                phase,
                gravity_dir: body_down(frame),
                room: live.of(entity),
            });
        }
    }
}

/// The raised guard of one body.
fn shield_ring_fact(
    live: &ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    entity: Entity,
    kin: &ambition_platformer2d_core::BodyKinematics,
    shield: &ambition_platformer2d_core::BodyShieldState,
    presented: Option<&crate::presented_pose::PresentedPose>,
    model: Option<&ambition_platformer2d_core::MotionModel>,
    frame: Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
) -> ShieldRingFact {
    let down = body_down(frame);
    // From the middle of the body to the middle of the bubble.
    let lean = down * (shield_half_height(kin.size, down) * shield.shield_tilt);
    ShieldRingFact {
        pos: presented.map_or(kin.pos, |p| p.presented()) + lean,
        size: kin.size,
        parrying: shield.parrying(),
        integrity: model.map_or(1.0, |m| shield.integrity_fraction(m.shield_tuning())),
        stun_secs: shield.stun_timer,
        blow_phase: shield.blow_phase(),
        blow_from: shield.blow_from - lean,
        gravity_dir: down,
        room: live.of(entity),
    }
}

/// The body's half-extent ALONG its own gravity, which is what
/// `BodyShieldState::shield_tilt` is a fraction of.
///
/// ⛔ THE SAME MEASURE THE HIT TEST USES
/// (`ambition_combat::util::guard_covers_hit`). A bubble drawn against any
/// other half-height would lean by a different amount than the guard that
/// actually blocks, and the picture would be lying about where the shield is.
fn shield_half_height(
    size: ambition_platformer2d_core::Vec2,
    down: ambition_platformer2d_core::Vec2,
) -> f32 {
    ambition_platformer2d_core::AccelerationFrame::new(down)
        .to_world_half(size * 0.5)
        .dot(down)
        .abs()
}

/// This body's toward-feet direction, falling back to the engine default for a
/// body whose frame has not been resolved.
fn body_down(
    frame: Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
) -> ambition_platformer2d_core::Vec2 {
    frame.map_or(ambition_platformer2d_core::DEFAULT_GRAVITY_DIR, |frame| {
        frame.down()
    })
}

/// One body whose guard has SHATTERED and is still paying for it.
///
/// Its own pooled view rather than a row on [`ShieldRingsView`], and the reason
/// is what a row there MEANS to the consumer that already exists:
/// `sync_bubble_shield_visual` assigns pooled bubble sprites by index over that
/// vector, so `len()` is "how many bubbles do I need". A broken guard is not
/// raised — `break_shield` drops `active` — so a row for one would either draw
/// a bubble on a shattered shield or force that loop to filter, at which point
/// the length stops answering the question it is asked. Two views, each meaning
/// one thing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuardBreakFact {
    pub pos: ambition_platformer2d_core::Vec2,
    pub size: ambition_platformer2d_core::Vec2,
    /// How far through the break this body is: `0.0` the instant the guard
    /// shatters, approaching `1.0` as it recovers
    /// (`BodyShieldState::break_phase`).
    pub phase: f32,
    /// This body's OWN toward-feet direction. Dizzy stars orbit the body's up,
    /// which is the opposite of this — never screen `-Y`.
    pub gravity_dir: ambition_platformer2d_core::Vec2,
    /// The live room of the body (`LiveRooms::of`). The stars are drawn in
    /// that room.
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

/// Every body currently in a guard break, in query order.
#[derive(Resource, Default, Clone, Debug)]
pub struct GuardBreaksView(pub Vec<GuardBreakFact>);

/// One body in INVOLUNTARY flight, resolved sim-side.
///
/// A row exists only while the body is launched — tumbling from a hit, or
/// still inside the hitstun that hit gave it. Presentation reads the row's
/// speed to decide how hard the launch reads; it never has to ask why a body
/// is moving fast, which is the question velocity alone answers wrongly for a
/// run, a fast fall or a recovery.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaunchedBodyFact {
    pub pos: ambition_platformer2d_core::Vec2,
    pub vel: ambition_platformer2d_core::Vec2,
    /// The body's current collision AABB — presentation offsets the trail
    /// behind the body by a fraction of it.
    pub size: ambition_platformer2d_core::Vec2,
    /// Seconds left in the HARD control lock at the FRONT of this launch:
    /// `BodyCombat::recoil_lock_timer`, the window in which the body has been
    /// thrown and has no authority at all over where it goes. `0.0` for a body
    /// still in involuntary flight that can answer for itself again.
    ///
    /// This is the ONE thing separating a body launched this instant from one
    /// that has been tumbling for a second: both are in this view, both are
    /// fast, and every other field reads the same for the two. Presentation
    /// gives the front of a launch its own beat off this and nothing else.
    ///
    /// A METEOR is the same window, longer — the Smash demo declares
    /// `meteor_lock_time: 0.30` against an ordinary `0.12` — so a spike reads
    /// as a longer, harder beat with no second fact.
    pub launch_beat_secs: f32,
    /// THIS BODY'S OWN launch threshold, in px/s — the speed at which the
    /// kernel calls a launch a tumble
    /// (`AxisSweptParams::abilities.tumble_speed`, the value
    /// `launch_into_tumble` gates on).
    ///
    /// ⭐ published so an onset can be read against the body it belongs to. It
    /// is authored PER BODY, so a heavyweight and a featherweight are in
    /// trouble at different speeds, and a cue fitted to one stage-wide
    /// percentile says "this one is fast" where the body's own threshold says
    /// "this one is in trouble" — which is the difference between a readable
    /// tell and a speedometer.
    ///
    /// `0.0` for a body whose policy has no tumble at all, which is every body
    /// outside a match: such a body never tumbles, so a threshold of zero is
    /// the honest answer rather than a sentinel.
    pub tumble_speed: f32,
    /// The live room of the body (`LiveRooms::of`). The trail is drawn in
    /// that room.
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

/// Every body (player AND brain-driven fighter) currently in an involuntary
/// flight, in query order — the read-model behind the hard-launch trail.
///
/// Pooled rather than a per-entity component for the reason
/// [`ShieldRingsView`] is: the effect is world particles with no owner entity
/// to hang a view on, and a seated fighter's sprite is a SEPARATE entity from
/// its body, joined by feature id. A pooled row is the one shape both roads
/// share.
#[derive(Resource, Default, Clone, Debug)]
pub struct LaunchedBodiesView(pub Vec<LaunchedBodyFact>);

pub fn rebuild_launched_bodies_view(
    mut view: ResMut<LaunchedBodiesView>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    bodies: Query<
        (
            Entity,
            &ambition_platformer2d_core::BodyKinematics,
            Option<&ambition_platformer2d_core::BodyMotionFacts>,
            Option<&BodyCombat>,
            // The presented position, so the plume leaves the body where the
            // sprite is drawn rather than at the tick position it is interpolating
            // away from.
            Option<&crate::presented_pose::PresentedPose>,
            // The body's own motion policy, which is where its launch threshold is
            // authored. Read for that ONE value; the maneuver state behind it is
            // model-private (ADR 0024) and stays that way.
            Option<&ambition_platformer2d_core::movement::MotionModel>,
        ),
        // ⛔⛔ A BODY OUT OF PLAY IS NOT IN A FLIGHT, AND THIS IS THE KO COMPOSITION
        // POLICY RATHER THAN AN AMPLITUDE TWEAK. The launch trail's own header says
        // its blast and plume are *"a LAYER over the hit spark and camera shake"*,
        // and the knockout beat then layers on top of that — three modules each
        // locally correct, and nothing anywhere saying *"the flight has RESOLVED;
        // the cues whose job was to predict danger are done"*. So an elimination
        // drew the trail that was still predicting the launch underneath the beat
        // that answered it.
        //
        // ⭐ THE SEAM WAS ALREADY HERE. This view is the sim's resolved "this motion
        // is INVOLUNTARY" fact, and a body the world has its hands off is not moving
        // involuntarily — it is not moving at all. Retiring the row is the whole
        // policy: the predictive cue stops at the instant the thing it predicted
        // happens, and the knockout owns the beat alone.
        bevy::prelude::Without<ambition_combat::death_rules::OutOfPlay>,
    >,
) {
    view.0.clear();
    view.0.extend(
        bodies
            .iter()
            .filter_map(|(entity, kin, motion, combat, presented, model)| {
                // Two published sim facts, one resolved answer: the tumble is the
                // helpless half of a launch and the hitstun is the rest of it. A
                // consumer reading only the tumble would drop the row the instant a
                // launched body stopped tumbling, mid-flight.
                let launched = motion.is_some_and(|f| f.tumbling)
                    || combat.is_some_and(|c| c.hitstun_timer > 0.0);
                launched.then(|| LaunchedBodyFact {
                    pos: presented.map_or(kin.pos, |p| p.presented()),
                    vel: kin.vel,
                    size: kin.size,
                    launch_beat_secs: combat.map_or(0.0, |c| c.recoil_lock_timer),
                    tumble_speed: match model {
                        Some(ambition_platformer2d_core::MotionModel::AxisSwept(axis)) => {
                            axis.params.abilities.tumble_speed
                        }
                        // A policy with no floor game has no launch threshold.
                        _ => 0.0,
                    },
                    room: live.of(entity),
                })
            }),
    );
}

#[cfg(test)]
mod pose_view_tests {
    use super::*;

    /// Two live rooms, one body broken and launched in each, and a stamp
    /// telling each body's room. Returns (guard-break rooms, launched rooms).
    #[allow(clippy::type_complexity)]
    fn rooms_of_two_rooms_of_bodies() -> (
        Vec<Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>>,
        Vec<Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>>,
    ) {
        use ambition_platformer2d_core as ae;
        use ambition_platformer2d_shared_tangle::lifecycle::{
            InRoomInstance, LiveRoomInstance, RoomInstanceRoot,
        };
        use bevy::prelude::{App, Update};
        let mut app = App::new();
        app.init_resource::<ShieldRingsView>();
        app.init_resource::<GuardBreaksView>();
        app.init_resource::<LaunchedBodiesView>();
        let live = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
        for instance in live {
            app.world_mut().spawn((RoomInstanceRoot, instance));
        }
        for instance in live {
            let mut facts = ae::BodyMotionFacts::default();
            facts.tumbling = true;
            let mut shield = ae::BodyShieldState::default();
            shield.break_timer = 0.5;
            shield.break_total = 1.0;
            app.world_mut().spawn((
                ae::BodyKinematics {
                    pos: ae::Vec2::new(100.0, 200.0),
                    vel: ae::Vec2::new(900.0, 0.0),
                    size: ae::Vec2::new(16.0, 32.0),
                    facing: 1.0,
                },
                facts,
                shield,
                InRoomInstance(instance),
            ));
        }
        app.add_systems(Update, (rebuild_shield_views, rebuild_launched_bodies_view));
        app.update();
        let sorted = |mut rooms: Vec<Option<LiveRoomInstance>>| {
            rooms.sort_by_key(|room| room.map(|room| room.ordinal()));
            rooms
        };
        (
            sorted(app.world().resource::<GuardBreaksView>().0.iter().map(|f| f.room).collect()),
            sorted(app.world().resource::<LaunchedBodiesView>().0.iter().map(|f| f.room).collect()),
        )
    }

    /// V2o: while two rooms are live, a guard-break row and a launch row each
    /// name their body's own room, so the stars and the trail are drawn
    /// there. A row with no room is drawn in no room.
    #[test]
    fn a_broken_or_launched_body_names_its_own_live_room() {
        use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
        let both = vec![Some(LiveRoomInstance::ACTIVATION), Some(LiveRoomInstance::ACTIVATION.next())];
        let (breaks, launched) = rooms_of_two_rooms_of_bodies();
        assert_eq!(breaks, both, "the room of each guard-break row");
        assert_eq!(launched, both, "the room of each launch row");
    }

    #[test]
    fn shield_rings_view_defaults_empty() {
        let view = ShieldRingsView::default();
        assert!(view.0.is_empty());
    }

    #[test]
    fn body_pose_view_default_is_inert() {
        let pose = BodyPoseView::default();
        assert_eq!(pose.stance_ratio_y, 1.0);
        assert_eq!(pose.hit_flash_secs, 0.0);
        assert!(pose.charge_tier.is_none());
        assert!(!pose.morph_ball);
    }

    /// ⭐⭐ A SPINNING MOVE FLIPS WHAT IS DRAWN AND NOT WHICH WAY THE BODY FACES.
    ///
    /// The crude spin read Jon asked for on Pointed's Up-B (*"fake the spin by
    /// repeatedly flipping the sprite horizontally"*), and the reason it is safe
    /// to fake: the mirror lives on the published POSE. The body's own `facing`
    /// is the fact hitboxes, launch directions and the brain all read, and a
    /// presentation trick that moved it would turn a drawing choice into a
    /// gameplay one six times a second.
    ///
    /// ⛔ SO THE ASSERTION IS A PAIR OF READINGS OF THE SAME FRAME — the pose
    /// disagrees with the body, and the body is unchanged.
    #[test]
    fn a_posed_body_publishes_its_own_gravity_not_the_primary_bodys() {
        use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
        let mut app = bevy::prelude::App::new();
        // The primary body's mirror: straight down.
        app.insert_resource(ambition_platformer2d_shared_tangle::gravity::GravityField::default());
        app.add_systems(bevy::prelude::Update, rebuild_body_pose_views);
        let sideways = ambition_platformer2d_core::Vec2::new(1.0, 0.0);
        let mut frame = ResolvedMotionFrame::default();
        frame.publish_resolved_frame(ambition_platformer2d_core::MotionFrame::from_direction(
            sideways, 0.0,
        ));
        let kin = ambition_platformer2d_core::BodyKinematics {
            pos: ambition_platformer2d_core::Vec2::ZERO,
            vel: ambition_platformer2d_core::Vec2::ZERO,
            size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
            facing: 1.0,
        };
        let seat = app.world_mut().spawn((PosedBody, kin, frame)).id();
        app.update();
        assert_eq!(
            app.world().get::<BodyPoseView>(seat).map(|p| p.gravity_dir),
            Some(sideways),
            "a second fighter under its own zone was drawn against the primary body's gravity"
        );
    }

    #[test]
    fn a_spinning_move_mirrors_the_drawn_pose_and_leaves_the_body_facing_alone() {
        use ambition_combat::moveset::MovePlayback;

        let mut app = bevy::prelude::App::new();
        app.add_systems(bevy::prelude::Update, rebuild_body_pose_views);

        let kin = ambition_platformer2d_core::BodyKinematics {
            pos: ambition_platformer2d_core::Vec2::ZERO,
            vel: ambition_platformer2d_core::Vec2::ZERO,
            size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
            facing: 1.0,
        };

        let spec = ambition_entity_catalog::MoveSpec {
            display_name: None,
            id: "spin".to_string(),
            clip: ambition_entity_catalog::ClipBinding {
                clip: "spin".to_string(),
                fallbacks: vec![],
            },
            duration_s: 1.0,
            windows: vec![],
            events: vec![],
            gates: Default::default(),
            start_impulse: None,
            charge_mult: 1.0,
            move_charge: None,
            charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
            repeat: None,
            landing_lag_s: None,
            autocancel_after_s: None,
            sprite_spin_hz: Some(10.0),
            equips: None,
            flow: None,
        };

        // Parked inside a mirrored half-period.
        let mut playback = MovePlayback::new(spec, 1.0);
        playback.t = 0.07;
        assert!(
            playback.sprite_mirrored_now(),
            "the fixture is not on a mirrored frame, so what follows measures \
             nothing"
        );

        let body = app.world_mut().spawn((PlayerVisual, kin, playback)).id();
        app.update();

        assert_eq!(
            app.world().get::<BodyPoseView>(body).map(|p| p.facing),
            Some(-1.0),
            "the drawn pose was not mirrored, so the spin is invisible"
        );
        assert_eq!(
            app.world()
                .get::<ambition_platformer2d_core::BodyKinematics>(body)
                .map(|k| k.facing),
            Some(1.0),
            "the presentation mirror reached the BODY's facing — every hitbox \
             and launch direction this fighter has now flips six times a second"
        );
    }

    /// A sheet-authored quad is reported only by a body that HAS one.
    ///
    /// `ActorRenderSize` alone cannot support that claim: it is the shared sprite-quad
    /// component, and several spawn paths set it from sheet metadata for bodies whose collision
    /// box is still hand-authored. Reading it bare would silently retarget the render for every
    /// such body, which is a change nothing would report.
    ///
    /// The negative case is the whole test — the positive one only proves the
    /// field is wired at all.
    #[test]
    fn only_a_sheet_authored_body_reports_an_authored_quad() {
        use ambition_combat::components::ActorRenderSize;
        use ambition_sprite_sheet::character::SpritePosedBody;

        let mut app = bevy::prelude::App::new();
        app.add_systems(bevy::prelude::Update, rebuild_body_pose_views);

        let kin = ambition_platformer2d_core::BodyKinematics {
            pos: ambition_platformer2d_core::Vec2::ZERO,
            vel: ambition_platformer2d_core::Vec2::ZERO,
            size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
            facing: 1.0,
        };
        let quad = ambition_platformer2d_core::Vec2::new(61.0, 73.0);

        // A quad, but the body's box is its own business.
        let hand_authored = app
            .world_mut()
            .spawn((PlayerVisual, kin, ActorRenderSize(quad)))
            .id();
        // A quad that came from the same scale as the box.
        let sheet_authored = app
            .world_mut()
            .spawn((
                PlayerVisual,
                kin,
                ActorRenderSize(quad),
                SpritePosedBody::new("robot", 2.0),
            ))
            .id();

        app.update();

        assert_eq!(
            app.world()
                .get::<BodyPoseView>(hand_authored)
                .and_then(|p| p.authored_render),
            None,
            "a render size without a sheet-authored body is not a claim about \
             that body's geometry, and must not be reported as one"
        );
        assert_eq!(
            app.world()
                .get::<BodyPoseView>(sheet_authored)
                .and_then(|p| p.authored_render),
            Some(quad),
            "and a body whose geometry IS its art reports the quad that came \
             from the same scale as its box"
        );
    }

    /// The FRONT of a launch is published, and it is the only thing that tells
    /// a body thrown this instant from one that has been tumbling for a second.
    ///
    /// Both bodies here are launched, both are travelling at the same speed,
    /// and every other field of their rows is identical — which is exactly the
    /// state presentation could not read before this field existed. The
    /// zero-beat row is the whole test: drop the field's source and the two
    /// rows become indistinguishable again, silently.
    #[test]
    fn the_front_of_a_launch_is_published_apart_from_the_tumble() {
        use ambition_characters::actor::BodyCombat;
        use ambition_platformer2d_core::BodyKinematics;
        use ambition_platformer2d_core::{BodyMotionFacts, Vec2};

        let mut app = bevy::prelude::App::new();
        app.init_resource::<LaunchedBodiesView>();
        app.add_systems(bevy::prelude::Update, rebuild_launched_bodies_view);

        let kin = |x: f32| BodyKinematics {
            pos: Vec2::new(x, 0.0),
            vel: Vec2::new(900.0, 0.0),
            size: Vec2::new(30.0, 48.0),
            facing: 1.0,
        };
        let tumbling = BodyMotionFacts {
            tumbling: true,
            ..Default::default()
        };

        // Thrown THIS INSTANT: still inside the hard control lock.
        app.world_mut().spawn((
            kin(0.0),
            tumbling,
            BodyCombat {
                hitstun_timer: 0.4,
                recoil_lock_timer: 0.09,
                ..Default::default()
            },
        ));
        // Still flying, still helpless, but it can steer again.
        app.world_mut().spawn((
            kin(1.0),
            tumbling,
            BodyCombat {
                hitstun_timer: 0.4,
                recoil_lock_timer: 0.0,
                ..Default::default()
            },
        ));
        // Under its own power at the same speed: not a launch at all.
        app.world_mut().spawn(kin(2.0));

        app.update();

        let view = app.world().resource::<LaunchedBodiesView>();
        assert_eq!(
            view.0.len(),
            2,
            "only the launched bodies are rows: {view:?}"
        );
        let beat_at = |x: f32| {
            view.0
                .iter()
                .find(|row| row.pos.x == x)
                .unwrap_or_else(|| panic!("no row at {x}: {view:?}"))
                .launch_beat_secs
        };
        assert_eq!(beat_at(0.0), 0.09, "the lock the sim wrote, unaltered");
        assert_eq!(
            beat_at(1.0),
            0.0,
            "a sustained tumble is NOT a launch beat, and reporting one here \
             would make every trailing body flash forever"
        );
    }

    /// A content pose pin reaches the PLAYER's view, not only an actor's.
    ///
    /// `ActorAnimOverride` is how content states a pose the locomotion picker cannot infer — a
    /// shell withdrawing, a body mid-transformation.
    ///
    /// The pin is the whole assertion: an unpinned body here picks `Idle` (the
    /// partial-cluster fallback), so a view that answers `Idle` under a pin is
    /// exactly the silent-drop failure, and one that answers `Idle` without a
    /// pin proves the test is reading the field the pin would have changed.
    #[test]
    fn a_content_pose_pin_reaches_the_players_pose_view() {
        use ambition_sprite_sheet::character::ActorAnimOverride;

        let mut app = bevy::prelude::App::new();
        app.add_systems(bevy::prelude::Update, rebuild_body_pose_views);

        let kin = ambition_platformer2d_core::BodyKinematics {
            pos: ambition_platformer2d_core::Vec2::ZERO,
            vel: ambition_platformer2d_core::Vec2::ZERO,
            size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
            facing: 1.0,
        };
        let unpinned = app.world_mut().spawn((PlayerVisual, kin)).id();
        let pinned = app
            .world_mut()
            .spawn((PlayerVisual, kin, ActorAnimOverride(CharacterAnim::Grow)))
            .id();

        app.update();

        assert_eq!(
            app.world().get::<BodyPoseView>(unpinned).map(|p| p.anim),
            Some(CharacterAnim::Idle),
            "an unpinned body picks its pose; `Idle` is the partial-cluster read"
        );
        assert_eq!(
            app.world().get::<BodyPoseView>(pinned).map(|p| p.anim),
            Some(CharacterAnim::Grow),
            "and a pinned one shows what the content pinned"
        );
    }
}

#[cfg(test)]
mod posed_body_gate_tests {
    use super::*;
    use bevy::prelude::*;

    /// ⭐⭐ A BODY THAT IS NOT THE PLAYER'S AVATAR STILL PUBLISHES ITS POSE.
    ///
    /// ⛔⛔ THE GATE WAS `With<PlayerVisual>` ALONE, AND A MATCH FIGHTER NEVER
    /// RECEIVES ONE — that marker is granted in exactly one production place, the
    /// exploration player's avatar. So `BodyPoseView` was not built for a seated
    /// body at all: a headless diagnostic could say where a fighter was and what
    /// move it played, and not which POSE and CLIP the game intended to draw, and
    /// the moveset inspector reconstructed the frame cursor in JavaScript.
    ///
    /// ⚠ THE SECOND ARM IS WHAT MAKES THE FIRST MEAN SOMETHING. A body with
    /// NEITHER marker must still get nothing — a gate widened to "every body"
    /// would pass the first arm and publish a read model for every crate, pot and
    /// projectile in the room.
    #[test]
    fn a_posed_body_gets_a_view_and_an_unmarked_body_does_not() {
        let mut app = App::new();
        app.add_systems(Update, rebuild_body_pose_views);
        let kin = ambition_platformer2d_core::BodyKinematics {
            pos: ambition_platformer2d_core::Vec2::new(10.0, 20.0),
            vel: ambition_platformer2d_core::Vec2::ZERO,
            size: ambition_platformer2d_core::Vec2::new(24.0, 40.0),
            facing: 1.0,
        };
        // A seated fighter: `PosedBody`, and deliberately NO `PlayerVisual`.
        let seat = app.world_mut().spawn((PosedBody, kin)).id();
        // Anything else in the room.
        let scenery = app.world_mut().spawn(kin).id();

        app.update();

        assert!(
            app.world().get::<BodyPoseView>(seat).is_some(),
            "a body carrying PosedBody must publish its pose read model without \
             ever being the player's drawn avatar"
        );
        assert!(
            app.world().get::<BodyPoseView>(scenery).is_none(),
            "a body carrying neither marker must publish nothing — a gate widened \
             to every body would build a read model for every crate and pot"
        );
    }
}
