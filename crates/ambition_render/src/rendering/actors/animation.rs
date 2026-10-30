//! Per-frame sprite animation systems (player, characters, props).

use bevy::prelude::*;

use crate::rendering::primitives::{FeatureVisual, PlayerVisual, PropVisual};
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_sprite_sheet::character::CharacterAnimator;

/// How the drawn art's height stands in for a row the sheet does not have.
///
/// Two factors multiply:
///
/// - `ratio_y`, the stance compaction (crouch, crawl, slide, morph) for a sheet
///   with no row for the compact pose: the collision-box ratio
///   `current AABB height / base height`, clamped to (0, 1].
/// - `motion_y`, the body's motion squash and stretch (its air stretch times
///   its landing squash, `BodyPoseView::air_stretch` and `squash.y`), for a
///   sheet that has no air row and so draws an air pose or a landing from a
///   ground row ([`draws_motion_from_ground_row`]). Above 1 in a fast fall.
///
/// The pivot is the point of the quad that holds still. The two
/// sprite-placement schemes use different pivots:
///
/// - A feet-anchored quad puts the feet at the transform, so scaling about
///   the anchor holds them.
/// - An authored-offset quad is anchored at its centre and placed by the
///   translation (`sync_visuals`). Its foot line is the quad's +gravity edge.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StanceSquash {
    pub(crate) ratio_y: f32,
    pub(crate) motion_y: f32,
    /// Hold the quad's +gravity edge rather than its anchor.
    pub(crate) about_quad_foot: bool,
}

/// True when the sheet has neither a `jump` nor a `fall` row and no row of its
/// own for `anim`, so it draws `anim` from a ground row. The body's motion
/// squash and stretch then stands in for the air drawing it does not have. A
/// sheet with an air row is drawn as authored.
pub(crate) fn draws_motion_from_ground_row(
    spec: &ambition_sprite_sheet::character::CharacterSheetSpec,
    anim: ambition_sprite_sheet::character::CharacterAnim,
) -> bool {
    use ambition_sprite_sheet::character::CharacterAnim;
    !spec.maps(CharacterAnim::Jump) && !spec.maps(CharacterAnim::Fall) && !spec.maps(anim)
}

impl StanceSquash {
    /// A body at full standing height — nothing to squash.
    pub(crate) const NONE: Self = Self {
        ratio_y: 1.0,
        motion_y: 1.0,
        about_quad_foot: false,
    };

    /// The squash `animator` draws `anim` with: each factor only where the
    /// sheet lacks the row it stands in for, and no motion factor while an
    /// authored clip shows. Call after the frame's request, so the clip that
    /// shows is this frame's.
    fn drawn_by(self, animator: &CharacterAnimator, anim: ambition_sprite_sheet::character::CharacterAnim) -> Self {
        let compact = if animator.spec.maps(anim) { 1.0 } else { self.ratio_y };
        let motion = if animator.showing_clip().is_none() && draws_motion_from_ground_row(&animator.spec, anim) {
            self.motion_y
        } else {
            1.0
        };
        Self {
            ratio_y: compact * motion,
            motion_y: 1.0,
            about_quad_foot: self.about_quad_foot,
        }
    }

    /// The squashed height and the anchor that keeps the pivot where it was, in
    /// Bevy sprite-local units (`+y` up, anchor normalized to the quad). The
    /// motion factor is not applied here: [`Self::drawn_by`] folds it in.
    fn squash(self, height: f32, anchor_y: f32) -> (f32, f32) {
        let scaled = height * self.ratio_y;
        if !self.about_quad_foot || self.ratio_y == 1.0 || scaled <= f32::EPSILON {
            return (scaled, anchor_y);
        }
        // Scale the quad's extents about its foot edge, then re-express the
        // result as an anchor: the translation is fixed by the placement above
        // and only the anchor can move the quad relative to it.
        let foot = -(anchor_y + 0.5) * height;
        let top = foot + ((0.5 - anchor_y) * height - foot) * self.ratio_y;
        (scaled, 0.5 - top / scaled)
    }
}

/// The shared animation tail for every animated actor (player, enemy, NPC):
/// request the anim, tick the animator by the entity dt, set the atlas frame,
/// apply the gravity-aware facing flip, and set the tint. The per-actor
/// systems differ only in how they select the anim and tint.
pub(crate) fn apply_character_frame(
    sprite: &mut Sprite,
    animator: &mut CharacterAnimator,
    anchor: Option<&mut bevy::sprite::Anchor>,
    anim: ambition_sprite_sheet::character::CharacterAnim,
    // The clip that the body's active move requests, if any. `anim` has no
    // variant for some sheet rows (`smash_forward`, `air_dodge`, `tumble`). The
    // move names its clip and fallbacks; `anim`'s pose ladder is the last resort.
    clip: Option<&ambition_sim_view::ClipRequest>,
    conversation_held: bool,
    barking: bool,
    dt: f32,
    facing: f32,
    gravity_dir: ambition_platformer2d_core::Vec2,
    color: Color,
    stance: StanceSquash,
) -> FrameWrites {
    animator.note_idle(anim == ambition_sprite_sheet::character::CharacterAnim::Idle && clip.is_none(), dt);
    animator.request_actor_pose(
        anim,
        clip.into_iter().flat_map(|request| request.chain()),
        clip.is_some(),
        conversation_held,
        barking,
    );
    animator.slave_clip_to(clip.and_then(|request| request.phase));
    // A move whose row is the upper body only: the legs keep their own pose.
    animator.under_clip(clip.and_then(|request| request.legs));
    // The squash is a placeholder for rows the sheet lacks.
    let stance = stance.drawn_by(animator, anim);
    // Gravity-aware facing flip: a ~180° up-gravity roll already mirrors the
    // sprite, so the flip inverts (#33). The flip is XORed with the sheet's
    // authored facing (`authored_faces_left`), the same term `animate_bosses`
    // uses. This makes left-drawn rigs such as the Patent Clerk face the way
    // they move.
    let flip = ambition_sprite_sheet::art_is_mirrored(
        animator.spec.authored_faces_left(),
        facing,
        gravity_dir,
    );
    // A sheet drawn from both sides answers the flip with its mirror rows, so
    // the texture is flipped only for a character that is left-right symmetric.
    let flip = animator.face(flip);
    let mut writes = draw_animator_frame(sprite, animator, anchor, dt, flip, stance);
    if sprite.color != color {
        sprite.color = color;
        writes.sprite = true;
    }
    writes
}

/// Which of the two components a frame draw wrote. The draw compares before it
/// writes, so a system that hands it the components with change detection
/// bypassed marks each one changed only when it was written: an animated
/// sprite whose frame did not advance is not marked changed, and its bounds
/// and its readers are not done again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub(crate) struct FrameWrites {
    pub sprite: bool,
    pub anchor: bool,
}

impl FrameWrites {
    /// Mark what was written changed: the draw was given both with change
    /// detection bypassed.
    pub(crate) fn mark(self, sprite: &mut Mut<Sprite>, anchor: Option<&mut Mut<bevy::sprite::Anchor>>) {
        if self.sprite {
            sprite.set_changed();
        }
        if let (true, Some(anchor)) = (self.anchor, anchor) {
            anchor.set_changed();
        }
    }
}

/// Tick `animator` by `dt` and draw its current frame on `sprite`: the page,
/// the atlas index, the facing flip, and the frame's trimmed size and anchor.
///
/// Every system that owns an animator draws through this. A packed sheet trims
/// each frame to its own rect, so a system that sets only the atlas index
/// draws every later frame inside the quad of the frame it was spawned on (a
/// gate membrane that opens from a 1 px point stays 1 px).
pub(crate) fn draw_animator_frame(
    sprite: &mut Sprite,
    animator: &mut CharacterAnimator,
    anchor: Option<&mut bevy::sprite::Anchor>,
    dt: f32,
    flip: bool,
    stance: StanceSquash,
) -> FrameWrites {
    let mut writes = FrameWrites::default();
    let index = animator.tick(dt);
    // Split sheets: select the page image the active animation draws from.
    // Single-page sheets (the common case) skip this entirely, so their
    // sprite image + layout stay exactly as built. `index` is already
    // page-local, so it addresses the swapped-in page's layout.
    if animator.is_paged() {
        let page = animator.current_page();
        if let Some(pg) = animator.pages.get(page as usize) {
            if sprite.image != pg.texture {
                sprite.image = pg.texture.clone();
                writes.sprite = true;
            }
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                if atlas.layout != pg.layout {
                    atlas.layout = pg.layout.clone();
                    writes.sprite = true;
                }
            }
        }
    }
    if let Some(atlas) = sprite.texture_atlas.as_mut() {
        if atlas.index != index {
            atlas.index = index;
            writes.sprite = true;
        }
    }
    // A piece that touches the next piece samples inside its frame, and each
    // frame has its own size.
    if animator.samples_inside_frame {
        let rect = animator.sample_rect();
        if sprite.rect != rect {
            sprite.rect = rect;
            writes.sprite = true;
        }
    }
    if sprite.flip_x != flip {
        sprite.flip_x = flip;
        writes.sprite = true;
    }
    // Compatibility fallback for legacy sprite construction. Normal construction
    // seeds this basis before the sprite is drawable, so frame zero of a packed
    // sheet does not flash at full logical size. No-op once initialized.
    if let (Some(size), Some(a)) = (sprite.custom_size, anchor.as_deref()) {
        animator.ensure_render_basis(size, a.0);
    }
    // A column that tiles fills the box its owner gave it.
    if animator.keeps_its_quad {
        return writes;
    }
    // The anchor x mirrors with the facing flip so an off-centre trim stays consistent
    // left/right.
    if let (Some((mut size, mut anchor_v)), Some(anchor)) = (animator.current_render(), anchor) {
        // Crouch/crawl/slide/morph: scale the trimmed height by the collision
        // ratio so the feet stay on the floor.
        (size.y, anchor_v.y) = stance.squash(size.y, anchor_v.y);
        if sprite.custom_size != Some(size) {
            sprite.custom_size = Some(size);
            writes.sprite = true;
        }
        if flip {
            anchor_v.x = -anchor_v.x;
        }
        if anchor.0 != anchor_v {
            anchor.0 = anchor_v;
            writes.anchor = true;
        }
    }
    writes
}

/// Draw `animator`'s current frame on `sprite` without advancing it: the
/// page, atlas index, facing flip and trimmed size and anchor, exactly as the
/// game draws a frame. For a harness that pins a row and frame and compares
/// realizations (`examples/rigged_sprite_parity.rs`).
pub fn draw_held_frame(
    sprite: &mut Sprite,
    animator: &mut CharacterAnimator,
    anchor: &mut bevy::sprite::Anchor,
    flip: bool,
) {
    // Plain references: the caller owns any change marking.
    let _ = draw_animator_frame(sprite, animator, Some(anchor), 0.0, flip, StanceSquash::NONE);
}

/// Drive the player sprite's animation state, atlas index, and facing flip.
/// Runs every frame; no-op on color-rectangle fallbacks (no `CharacterAnimator`).
///
/// The anim pick is sim-side: `rebuild_body_pose_views` resolves the pose in
/// `FeatureViewSync`. This system only consumes [`BodyPoseView`], ticks the
/// animator by presentation dt, and sets the frame.
pub fn animate_player(
    presentation_time: ambition_time::PresentationTime,
    mut query: Query<
        (
            &mut Sprite,
            &mut CharacterAnimator,
            &ambition_sim_view::BodyPoseView,
            Option<&ambition_time::ProperTimeScale>,
            Option<&mut bevy::sprite::Anchor>,
        ),
        With<PlayerVisual>,
    >,
) {
    // Iterate every player-bodied visual, not only the primary. A brain-driven
    // player clone uses the same sim-side picker.
    for (mut sprite, mut animator, pose, scale, anchor) in &mut query {
        // Presentation dt, scaled by the world clock and proper time.
        let dt = presentation_time.entity_dt(ambition_time::ProperTimeScale::or_default(scale));
        // A body that looks up while it moves: its head turns up over the row
        // it walks or runs in.
        animator.head_turn = (pose.head_up > 0.0).then_some((ambition_sim_view::LOOK_UP_CLIP, pose.head_up));
        // Hit feedback is the white-silhouette overlay in `rendering::hit_flash`.
        // The source sprite stays `WHITE`.
        let mut anchor = anchor;
        let writes = apply_character_frame(
            sprite.bypass_change_detection(),
            &mut animator,
            anchor.as_mut().map(|anchor| anchor.bypass_change_detection()),
            pose.anim,
            // A human and a CPU fighter on the same character draw the same row for
            // the same move.
            pose.clip.as_ref(),
            false,
            false,
            dt,
            pose.facing,
            pose.gravity_dir,
            Color::WHITE,
            StanceSquash {
                ratio_y: pose.stance_ratio_y,
                motion_y: pose.air_stretch * pose.squash.y,
                // The authored placement uses the translation and a centre anchor, so
                // the squash must pivot on the quad's own foot edge.
                about_quad_foot: pose.authored_offset.is_some(),
            },
        );
        writes.mark(&mut sprite, anchor.as_mut());
    }
}

/// The seconds left in a body's short bark pose. Presentation state on the
/// body's visual: the simulation emits the bark as a
/// [`ambition_vfx::VfxMessage::BarkGesture`] and keeps no timer of its own.
#[derive(bevy::prelude::Component, Clone, Copy, Debug)]
pub struct BarkPose(pub f32);

/// Start the bark pose on the visual whose feature id the bark names.
pub fn start_bark_poses(
    mut commands: Commands,
    mut messages: MessageReader<ambition_vfx::VfxInRoom>,
    visuals: Query<(Entity, &FeatureVisual)>,
) {
    for message in messages.read().map(|m| &m.vfx) {
        let ambition_vfx::VfxMessage::BarkGesture { feature_id, seconds } = message else {
            continue;
        };
        for (entity, visual) in &visuals {
            if visual.id == *feature_id {
                commands.entity(entity).try_insert(BarkPose(*seconds));
            }
        }
    }
}

/// Drive enemy and NPC sprite animation, atlas index, and facing flip.
///
/// A feature id is in only one of the enemy or NPC runtime lists. One system
/// for both avoids a borrow conflict on the shared
/// `(&mut Sprite, &mut CharacterAnimator)` query.
pub fn animate_characters(
    presentation_time: ambition_time::PresentationTime,
    mut query: Query<
        (
            &FeatureVisual,
            &mut Sprite,
            &mut CharacterAnimator,
            Option<&ambition_time::ProperTimeScale>,
            Option<&mut bevy::sprite::Anchor>,
            Option<&mut BarkPose>,
        ),
        (
            Without<PlayerVisual>,
            Without<super::super::primitives::PortalSprite>,
            Without<PropVisual>,
            // A part-drawn boss carries an animator for the rigged driver;
            // `animate_bosses` draws it, from the sim's cursor.
            Without<ambition_sprite_sheet::boss::BossAnimator>,
        ),
    >,
    // Per-actor pose read-model, built by `rebuild_actor_anim_index` just before
    // this system. The renderer does not borrow live actor clusters.
    anim_index: Res<ambition_sim_view::ActorAnimIndex>,
) {
    // ADR 0011: per-entity proper time on the presentation clock. No SP entity
    // carries `ProperTimeScale` yet, so every actor ticks at the world rate.
    for (visual, mut sprite, mut animator, scale, anchor, bark) in &mut query {
        let dt = presentation_time.entity_dt(ambition_time::ProperTimeScale::or_default(scale));
        let barking = bark.is_some_and(|mut pose| {
            pose.0 -= dt;
            pose.0 > 0.0
        });
        // Enemies and NPCs resolve through the same picker as the player, from
        // their `Body*` clusters.
        let Some(frame) = anim_index.get(&visual.id) else {
            continue;
        };
        // Hit feedback is the white-silhouette overlay in `rendering::hit_flash`;
        // the source sprite stays `WHITE`. Actors do not tint on their own attack.
        // Per-character attack presentation belongs in a game-authored spec, not a
        // default here.
        let mut anchor = anchor;
        // A centre basis anchor is an authored-offset quad
        // (`character_render_basis`): its feet are the quad's foot edge.
        let about_quad_foot = animator.render_basis.is_some_and(|basis| basis.feet_anchor == Vec2::ZERO);
        let writes = apply_character_frame(
            sprite.bypass_change_detection(),
            &mut animator,
            anchor.as_mut().map(|anchor| anchor.bypass_change_detection()),
            frame.anim,
            frame.clip.as_ref(),
            frame.conversation_held,
            barking,
            dt,
            frame.facing,
            // The body's own resolved down, so a wall-walking or ceiling
            // actor flips correctly.
            frame.gravity_dir,
            Color::WHITE,
            // Enemies and NPCs do not use the stance compaction.
            StanceSquash {
                ratio_y: 1.0,
                motion_y: frame.air_stretch * frame.squash.y,
                about_quad_foot,
            },
        );
        writes.mark(&mut sprite, anchor.as_mut());
    }
}

fn generic_feature_anim_owns(kind: FeatureVisualKind) -> bool {
    !matches!(kind, FeatureVisualKind::Actor)
}

/// Idle-tick the animation of every non-actor [`FeatureVisual`] that has a
/// [`CharacterAnimator`] (for example a spinning ring pickup). This is the
/// feature counterpart to [`animate_props`]. Players, actors
/// ([`animate_characters`]), props ([`animate_props`]), and portal sprites
/// are excluded, so each animator is ticked by one system only.
pub fn animate_feature_sprites(
    presentation_time: ambition_time::PresentationTime,
    feature_views: Res<ambition_sim_view::FeatureViewIndex>,
    mut query: Query<
        (
            &FeatureVisual,
            &mut Sprite,
            &mut CharacterAnimator,
            Option<&ambition_time::ProperTimeScale>,
            Option<&mut bevy::sprite::Anchor>,
        ),
        (
            Without<PropVisual>,
            Without<PlayerVisual>,
            Without<super::super::primitives::PortalSprite>,
            // See `animate_characters`: `animate_bosses` draws a boss.
            Without<ambition_sprite_sheet::boss::BossAnimator>,
        ),
    >,
) {
    for (visual, mut sprite, mut animator, scale, anchor) in &mut query {
        let Some(view) = feature_views.get(&visual.id) else {
            continue;
        };
        // `animate_characters` owns actors. Ticking them here too advances Idle
        // twice per frame and makes a flyer switch Fly -> Idle -> Fly every frame.
        if !generic_feature_anim_owns(view.kind) {
            continue;
        }
        let dt = presentation_time.entity_dt(ambition_time::ProperTimeScale::or_default(scale));
        let mut anchor = anchor;
        let writes = apply_character_frame(
            sprite.bypass_change_detection(),
            &mut animator,
            anchor.as_mut().map(|anchor| anchor.bypass_change_detection()),
            ambition_sprite_sheet::character::CharacterAnim::Idle,
            // A prop plays no moves.
            None,
            false,
            false,
            dt,
            1.0,
            ambition_platformer2d_core::Vec2::Y,
            Color::WHITE,
            // An animated feature doesn't crouch — full standing height.
            StanceSquash::NONE,
        );
        writes.mark(&mut sprite, anchor.as_mut());
    }
}

/// Prop kinds whose authored Idle row shows motion (for example rolling
/// wheels). [`animate_props`] holds them at frame 0 until a `PropMotionState`
/// component can gate their tick by real motion.
pub const PROP_KINDS_STATIC_UNTIL_MOVING: &[&str] = &["intro_cart"];

/// Tick the idle row for every `PropVisual` sprite that has a
/// `CharacterAnimator`. Props have no actor entity, so `animate_characters`
/// skips them.
///
/// `Without<PortalSprite>` leaves the gate ring and portal to the portal
/// systems, which drive the animator from `GatePortalPhase`.
///
/// Kinds in [`PROP_KINDS_STATIC_UNTIL_MOVING`] stay at frame 0.
pub fn animate_props(
    presentation_time: ambition_time::PresentationTime,
    mut query: Query<
        (
            &mut Sprite,
            &mut CharacterAnimator,
            &PropVisual,
            Option<&ambition_time::ProperTimeScale>,
            Option<&mut bevy::sprite::Anchor>,
            Option<&super::super::primitives::PropClip>,
            Option<&super::super::primitives::PropTint>,
        ),
        Without<super::super::primitives::PortalSprite>,
    >,
) {
    // ADR 0011: per-entity proper time. A prop that must tick while the world
    // is frozen gets a non-1.0 `ProperTimeScale`.
    for (mut sprite, mut animator, prop, scale, anchor, clip, tint) in &mut query {
        // Static-until-moving props use dt = 0, so `tick` does not advance.
        let dt = if PROP_KINDS_STATIC_UNTIL_MOVING.contains(&prop.kind.as_str()) {
            0.0
        } else {
            presentation_time.entity_dt(ambition_time::ProperTimeScale::or_default(scale))
        };
        let mut anchor = anchor;
        let writes = apply_character_frame(
            sprite.bypass_change_detection(),
            &mut animator,
            anchor.as_mut().map(|anchor| anchor.bypass_change_detection()),
            ambition_sprite_sheet::character::CharacterAnim::Idle,
            // A prop plays no moves; the row its game names, if any.
            clip.map(|clip| &clip.0),
            false,
            false,
            dt,
            1.0,
            ambition_platformer2d_core::Vec2::Y,
            tint.map_or(Color::WHITE, |tint| tint.0),
            // Props don't crouch — full standing height.
            StanceSquash::NONE,
        );
        writes.mark(&mut sprite, anchor.as_mut());
    }
}

#[cfg(test)]
mod tests {
    use super::{generic_feature_anim_owns, FeatureVisualKind};
    use ambition_platformer2d_core::Vec2;
    use ambition_sprite_sheet::character::sheets::{available_sheet_keys, record_for_sheet_key};

    #[test]
    fn actor_animators_are_not_owned_by_the_generic_feature_idle_loop() {
        assert!(!generic_feature_anim_owns(FeatureVisualKind::Actor));
        assert!(generic_feature_anim_owns(FeatureVisualKind::Pickup));
        assert!(generic_feature_anim_owns(FeatureVisualKind::Hazard));
    }

    /// The screen direction that a sheet's body points, as drawn.
    ///
    /// This combines the flip decision with the art's authored direction. A test
    /// of `flip_x` alone would test the mechanism against itself.
    fn drawn_direction(authored_faces_left: bool, facing: f32) -> f32 {
        let art_points = if authored_faces_left { -1.0 } else { 1.0 };
        let flip = ambition_sprite_sheet::art_is_mirrored(authored_faces_left, facing, Vec2::NEG_Y);
        if flip {
            -art_points
        } else {
            art_points
        }
    }

    /// A left-drawn character faces the way it moves, like a right-drawn one.
    ///
    /// The Patent Clerk and Carl Stargan are west-drawn paperdolls
    /// (`features.facing: "west"`). The goblin and Emmy (`noether`) are drawn
    /// facing east; Emmy comes from the same rig pipeline. For the same facing,
    /// all of them must look the same way.
    #[test]
    fn a_left_drawn_character_faces_the_way_they_are_going_like_a_right_drawn_one() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
        for left_drawn in ["patent_clerk", "carl_stargan"] {
            let sheet = record_for_sheet_key(&sprite_content, left_drawn)
                .unwrap_or_else(|| panic!("{left_drawn}'s sheet is baked into the sheet table"));
            // Premise: this sheet is left-drawn. Without this check the comparison
            // passes for a sheet that never uses the term.
            assert!(
                sheet.authored_faces_left,
                "{left_drawn}'s manifest must publish the drawn facing its rig declares \
                 (`features.facing: \"west\"`); regenerate the sheet if this is missing"
            );
            for right_drawn in ["goblin_cave_dagger", "noether"] {
                let other = record_for_sheet_key(&sprite_content, right_drawn)
                    .unwrap_or_else(|| panic!("{right_drawn} is baked into the sheet table"));
                assert!(
                    !other.authored_faces_left,
                    "{right_drawn} is drawn facing +x and must not have acquired a mirror"
                );
                for facing in [-1.0_f32, 1.0] {
                    assert_eq!(
                        drawn_direction(sheet.authored_faces_left, facing),
                        drawn_direction(other.authored_faces_left, facing),
                        "at facing {facing} {left_drawn} and {right_drawn} must look the same way"
                    );
                }
            }
        }
    }

    /// Every baked sheet points where its body faces, however it was drawn.
    #[test]
    fn every_baked_sheet_is_drawn_pointing_where_its_body_faces() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
        let mut checked = 0usize;
        for key in available_sheet_keys(&sprite_content) {
            let Some(record) = record_for_sheet_key(&sprite_content, key) else {
                continue;
            };
            checked += 1;
            for facing in [-1.0_f32, 1.0] {
                assert_eq!(
                    drawn_direction(record.authored_faces_left, facing),
                    facing,
                    "{key} draws its body pointing away from facing {facing}"
                );
            }
        }
        assert!(
            checked > 100,
            "expected the baked sheet table to hold the whole cast, saw {checked}"
        );
    }
}

#[cfg(test)]
mod stance_squash_tests {
    use super::StanceSquash;

    /// Sprite-local extents (`+y` up) of a quad of `height` anchored at `anchor_y`.
    fn extents(height: f32, anchor_y: f32) -> (f32, f32) {
        (-(anchor_y + 0.5) * height, (0.5 - anchor_y) * height)
    }

    /// The feet-anchored scheme puts the body at the transform, so the squash
    /// must not move the anchor.
    #[test]
    fn squashing_about_the_anchor_leaves_the_anchor_where_it_was() {
        let squash = StanceSquash {
            ratio_y: 0.5,
            motion_y: 1.0,
            about_quad_foot: false,
        };
        let (height, anchor) = squash.squash(100.0, -0.3);
        assert!((height - 50.0).abs() < 1e-4);
        assert!((anchor - -0.3).abs() < 1e-6, "the anchor moved to {anchor}");
    }

    /// The authored-offset scheme anchors the quad at its centre, so the squash
    /// must hold the art's foot edge. Otherwise the crouch lifts the body off
    /// the floor.
    #[test]
    fn squashing_about_the_quad_foot_holds_the_foot_edge() {
        // A ratio above 1 is the air stretch of a falling body: its foot edge
        // must not go down into the floor.
        for (anchor_in, ratio) in [(0.0_f32, 0.5_f32), (-0.3, 0.4), (0.2, 0.85), (0.0, 1.12), (-0.3, 1.05)] {
            let squash = StanceSquash {
                ratio_y: ratio,
                motion_y: 1.0,
                about_quad_foot: true,
            };
            let (foot_before, top_before) = extents(100.0, anchor_in);
            let (height, anchor) = squash.squash(100.0, anchor_in);
            let (foot_after, top_after) = extents(height, anchor);
            assert!(
                (foot_after - foot_before).abs() < 1e-3,
                "ratio {ratio} anchor {anchor_in}: the foot edge moved {} \
                 (before {foot_before}, after {foot_after})",
                foot_after - foot_before
            );
            assert!(
                ((top_after - foot_after) - (top_before - foot_before) * ratio).abs() < 1e-3,
                "ratio {ratio} anchor {anchor_in}: the quad is not {ratio} as tall"
            );
        }
    }

    /// A body at full height is unchanged by either pivot.
    #[test]
    fn a_standing_body_is_left_exactly_as_it_was() {
        for about_quad_foot in [false, true] {
            let squash = StanceSquash {
                ratio_y: 1.0,
                motion_y: 1.0,
                about_quad_foot,
            };
            assert_eq!(squash.squash(120.0, -0.42), (120.0, -0.42));
        }
        assert_eq!(StanceSquash::NONE.squash(120.0, -0.42), (120.0, -0.42));
    }
}

#[cfg(test)]
mod motion_squash_tests {
    use super::{apply_character_frame, StanceSquash};
    use ambition_sprite_sheet::character::sheets::{available_sheet_keys, try_load_spec_for_character_id};
    use ambition_sprite_sheet::character::{CharacterAnim, CharacterAnimator, CharacterSpriteAsset};
    use bevy::prelude::*;
    use bevy::sprite::Anchor;

    /// The full stretch of a fast fall.
    const STRETCH: f32 = 1.12;

    /// An animator on the first baked trimmed sheet that has an air row
    /// (`jump` or `fall`) or has none.
    fn animator(has_air_row: bool) -> CharacterAnimator {
        let content = ambition_sprite_sheet::test_support::shared_sprites();
        let spec = available_sheet_keys(&content)
            .into_iter()
            .filter_map(|key| try_load_spec_for_character_id(&content, key))
            .find(|spec| {
                spec.is_trimmed()
                    && (spec.maps(CharacterAnim::Jump) || spec.maps(CharacterAnim::Fall)) == has_air_row
            })
            .unwrap_or_else(|| panic!("the cast holds a trimmed sheet with has_air_row = {has_air_row}"));
        let mut animator = CharacterAnimator::new(&CharacterSpriteAsset {
            texture: Handle::default(),
            layout: Handle::default(),
            spec,
            pages: Vec::new(),
            requested_tier: ambition_persistence::settings::TextureResolutionScale::Full,
            resolved_tier: ambition_persistence::settings::TextureResolutionScale::Full,
            rigged: None,
        });
        animator.ensure_render_basis(Vec2::new(40.0, 80.0), Vec2::new(0.0, -0.45));
        animator
    }

    /// The quad size and anchor `animator` draws `anim` with at `motion_y`.
    fn drawn(animator: &mut CharacterAnimator, anim: CharacterAnim, motion_y: f32) -> (Vec2, Vec2) {
        let mut sprite = Sprite::default();
        let mut anchor = Anchor::default();
        let _ = apply_character_frame(
            &mut sprite,
            animator,
            Some(&mut anchor),
            anim,
            None,
            false,
            false,
            0.0,
            1.0,
            ambition_platformer2d_core::DEFAULT_GRAVITY_DIR,
            Color::WHITE,
            StanceSquash {
                ratio_y: 1.0,
                motion_y,
                about_quad_foot: false,
            },
        );
        (sprite.custom_size.expect("a trimmed sheet sizes its quad"), anchor.0)
    }

    /// A sheet with no air row draws a fall from its idle row: the fast fall is
    /// drawn taller about the feet, and only taller.
    #[test]
    fn a_sheet_with_no_air_row_draws_a_fast_fall_taller_about_its_feet() {
        let mut animator = animator(false);
        let (rest, rest_anchor) = drawn(&mut animator, CharacterAnim::Fall, 1.0);
        let (fall, fall_anchor) = drawn(&mut animator, CharacterAnim::Fall, STRETCH);
        assert!((fall.y - rest.y * STRETCH).abs() < 1e-3, "{fall:?} is not {STRETCH} times {rest:?}");
        assert_eq!(fall.x, rest.x);
        assert_eq!(fall_anchor, rest_anchor, "the feet anchor moved");
    }

    /// A sheet with an air row draws its fall as authored.
    #[test]
    fn a_sheet_with_an_air_row_draws_its_fall_as_authored() {
        let mut animator = animator(true);
        let anim = if animator.spec.maps(CharacterAnim::Fall) { CharacterAnim::Fall } else { CharacterAnim::Jump };
        assert_eq!(drawn(&mut animator, anim, STRETCH), drawn(&mut animator, anim, 1.0));
    }

    /// A row the sheet has is never squashed by the motion: a body that stands
    /// is drawn at its own height whatever its fall speed was.
    #[test]
    fn a_row_the_sheet_has_is_not_squashed() {
        let mut animator = animator(false);
        assert_eq!(
            drawn(&mut animator, CharacterAnim::Idle, STRETCH),
            drawn(&mut animator, CharacterAnim::Idle, 1.0)
        );
    }
}

#[cfg(test)]
mod bark_pose_tests {
    use super::{start_bark_poses, BarkPose};
    use crate::rendering::primitives::FeatureVisual;
    use bevy::prelude::*;

    /// A bark names its body, and only that body's visual takes the pose.
    #[test]
    fn a_bark_poses_the_visual_it_names() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ambition_vfx::VfxInRoom>();
        app.add_systems(Update, start_bark_poses);
        let dog = app
            .world_mut()
            .spawn(FeatureVisual { id: "dog".into() })
            .id();
        let parrot = app
            .world_mut()
            .spawn(FeatureVisual { id: "parrot".into() })
            .id();
        app.world_mut()
            .write_message(ambition_vfx::VfxInRoom { room: None, vfx: ambition_vfx::VfxMessage::BarkGesture {
                feature_id: "dog".into(),
                seconds: 0.48,
            } });
        app.update();
        assert_eq!(app.world().get::<BarkPose>(dog).map(|pose| pose.0), Some(0.48));
        assert!(app.world().get::<BarkPose>(parrot).is_none());
    }
}
