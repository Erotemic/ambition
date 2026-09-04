//! Per-frame sprite animation systems (player, characters, props).

use bevy::prelude::*;

use crate::rendering::primitives::{FeatureVisual, PlayerVisual, PropVisual};
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_sprite_sheet::character::CharacterAnimator;

/// How a stance compaction (crouch / crawl / slide / morph) squashes the DRAWN
/// art, for a sheet that has no row for the compact pose.
///
/// The RATIO is what the collision box did — `current AABB height / base
/// height`, clamped (0, 1]. The PIVOT is which point of the quad holds still
/// while that happens, and the two sprite-placement schemes disagree about it,
/// so it cannot be assumed:
///
/// - A FEET-ANCHORED quad puts the body's feet AT the transform, so scaling
///   about the anchor already holds them.
/// - An AUTHORED-OFFSET quad is anchored at its CENTRE — the placement is
///   carried by the translation (`sync_visuals`) — so the same scale lifts the
///   art clear of the floor. Its foot line is the quad's own +gravity edge.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StanceSquash {
    pub(crate) ratio_y: f32,
    /// Hold the quad's +gravity edge rather than its anchor.
    pub(crate) about_quad_foot: bool,
}

impl StanceSquash {
    /// A body at full standing height — nothing to squash.
    pub(crate) const NONE: Self = Self {
        ratio_y: 1.0,
        about_quad_foot: false,
    };

    /// The squashed height and the anchor that keeps the pivot where it was, in
    /// Bevy sprite-local units (`+y` up, anchor normalized to the quad).
    fn squash(self, height: f32, anchor_y: f32) -> (f32, f32) {
        let scaled = height * self.ratio_y;
        if !self.about_quad_foot || self.ratio_y >= 1.0 || scaled <= f32::EPSILON {
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

/// The shared animation TAIL every animated actor (player, enemy, NPC) runs:
/// request the chosen anim, tick the animator by the entity's dt, push the
/// resulting atlas frame onto the sprite, apply the gravity-aware facing flip,
/// and set the sprite tint. The per-actor systems differ only in how they SELECT
/// the anim + tint — pay-for-use: the player's picker reads its rich clusters
/// (crouch / slide / ladder / blink / …), the enemy/NPC picker reads its small
/// actor state. The frame-application MECHANISM is identical for every actor, so
/// it lives here once instead of being duplicated per render path.
pub(crate) fn apply_character_frame(
    sprite: &mut Sprite,
    animator: &mut CharacterAnimator,
    anchor: Option<&mut bevy::sprite::Anchor>,
    anim: ambition_sprite_sheet::character::CharacterAnim,
    // What the body's ACTIVE MOVE asks to be drawn as, when one is playing.
    //
    // sprite redirect P0: `anim` is the 56-variant semantic vocabulary and
    // the new fighter sheets carry rows it has no variant for — `smash_forward`,
    // `air_dodge`, `tumble`. A move already names its clip and its fallbacks, so
    // the exact row is drawn when this sheet has it, the author's fallbacks when
    // it does not, and `anim`'s pose ladder when it has none of them.
    clip: Option<&ambition_sim_view::ClipRequest>,
    dt: f32,
    facing: f32,
    gravity_dir: ambition_platformer2d_core::Vec2,
    color: Color,
    stance: StanceSquash,
) {
    // The stance squash is a PLACEHOLDER for sheets that lack a row for the compact pose (the
    // fallback then shows standing art at a shrunken AABB).
    let stance = if animator.spec.maps(anim) {
        StanceSquash::NONE
    } else {
        stance
    };
    match clip {
        Some(request) => animator.request_clip(request.chain(), anim),
        None => animator.request(anim),
    }
    let index = animator.tick(dt);
    // Split sheets: select the page image the active animation draws from.
    // Single-page sheets (the common case) skip this entirely, so their
    // sprite image + layout stay exactly as built. `index` is already
    // page-local, so it addresses the swapped-in page's layout.
    if animator.is_paged() {
        let page = animator.current_page();
        if let Some(pg) = animator.pages.get(page as usize) {
            sprite.image = pg.texture.clone();
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.layout = pg.layout.clone();
            }
        }
    }
    if let Some(atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = index;
    }
    // Gravity-aware facing flip: a ~180° up-gravity roll already mirrors the
    // sprite, so the flip inverts (fixes #33 "move left, face right upside down").
    //
    // XORed with the SHEET's own drawn facing, so the mirror asks "does the
    // requested facing differ from the facing this art was drawn in" rather
    // than "is facing negative". `authored_faces_left` is false for all but a
    // handful of sheets, so every +x-drawn character is byte-identical. This
    // is the same term `animate_bosses` has applied since the mockingbird —
    // the character path was simply the half that never got it, which is why
    // the Patent Clerk (an SVG rig whose paperdoll view is `Side Left`) faced
    // away from wherever he was going.
    let flip = ambition_sprite_sheet::art_is_mirrored(
        animator.spec.authored_faces_left(),
        facing,
        gravity_dir,
    );
    sprite.flip_x = flip;
    sprite.color = color;
    // Compatibility fallback for specialized/legacy sprite construction. Normal
    // character construction seeds this basis BEFORE the sprite becomes drawable,
    // which is what prevents frame zero of a packed sheet from flashing at the
    // full logical size. `ensure_render_basis` is a no-op once initialized.
    if let (Some(size), Some(a)) = (sprite.custom_size, anchor.as_deref()) {
        animator.ensure_render_basis(size, a.0);
    }
    // The anchor x mirrors with the facing flip so an off-centre trim stays consistent
    // left/right.
    if let (Some((mut size, mut anchor_v)), Some(anchor)) = (animator.current_render(), anchor) {
        // Crouch/crawl/slide/morph: scale the trimmed height by the collision-shrink
        // ratio so the feet stay planted. Without this a trimmed sheet renders
        // standing height at the lowered crouch pos and sinks through the floor.
        (size.y, anchor_v.y) = stance.squash(size.y, anchor_v.y);
        sprite.custom_size = Some(size);
        if flip {
            anchor_v.x = -anchor_v.x;
        }
        anchor.0 = anchor_v;
    }
}

/// Drive the player sprite's animation state, atlas index, and facing flip.
/// Runs every frame; no-op on color-rectangle fallbacks (no `CharacterAnimator`).
///
/// The anim pick and every cluster read moved SIM-side (E4 slices 1–3):
/// `rebuild_body_pose_views` resolves the pose in `FeatureViewSync` and this
/// system is a pure consumer of [`BodyPoseView`] — it only ticks the
/// animator by presentation dt and pushes the frame onto the sprite.
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
    // Iterate EVERY player-bodied visual, not just the primary: the human player
    // and any brain-driven player clone animate through the identical picker
    // (sim-side, in the pose rebuild). The player body is not special to
    // rendering, only the camera/HUD are.
    for (mut sprite, mut animator, pose, scale, anchor) in &mut query {
        // Presentation time uses this rendered frame's delta while applying the authoritative
        // world-clock and proper-time scales.
        let dt = presentation_time.entity_dt(ambition_time::ProperTimeScale::or_default(scale));
        // Hit feedback is drawn by the white-silhouette overlay in
        // `presentation::rendering::hit_flash` — a sibling mesh that samples this
        // atlas frame and outputs pure white modulated by the pose's flash fact.
        // The source sprite stays untinted (`WHITE`); the overlay flashes.
        apply_character_frame(
            &mut sprite,
            &mut animator,
            anchor.map(|a| a.into_inner()),
            pose.anim,
            // and the local player's move names its row too — the same
            // request the actor road carries, so a human-driven fighter and a
            // CPU one on the same character draw the same animation for the same
            // move. That is the property the whole seam exists for.
            pose.clip.as_ref(),
            dt,
            pose.facing,
            pose.gravity_dir,
            Color::WHITE,
            StanceSquash {
                ratio_y: pose.stance_ratio_y,
                // The authored placement carries the body on the TRANSLATION and
                // anchors the quad at its centre, so the squash has nothing at
                // the feet to pivot on unless it takes the quad's own edge.
                about_quad_foot: pose.authored_offset.is_some(),
            },
        );
    }
}

/// Drive enemy AND NPC sprite animation, atlas index, and facing flip.
///
/// Enemies and NPCs both render through `CharacterAnimator`; their
/// per-frame state is owned by separate runtime lists, but a feature
/// id only ever appears in one of them at a time. We try the enemy
/// lookup first (most entities in the room) and fall through to the
/// NPC lookup, so a stationary General sheet ticks its 8 idle frames
/// once the animator is attached.
///
/// One system instead of two avoids the borrow conflict on the
/// shared `(&mut Sprite, &mut CharacterAnimator)` query.
pub fn animate_characters(
    presentation_time: ambition_time::PresentationTime,
    mut query: Query<
        (
            &FeatureVisual,
            &mut Sprite,
            &mut CharacterAnimator,
            Option<&ambition_time::ProperTimeScale>,
            Option<&mut bevy::sprite::Anchor>,
        ),
        (
            Without<PlayerVisual>,
            Without<super::super::primitives::PortalSprite>,
            Without<PropVisual>,
        ),
    >,
    // Materialized per-actor pose read-model (built by `rebuild_actor_anim_index`
    // in the render presentation chain just before this system) — the renderer
    // animates from a snapshot, no longer borrowing the live actor clusters.
    anim_index: Res<ambition_sim_view::ActorAnimIndex>,
    // Localized gravity, so an enemy/NPC wall-walking or on a flipped-gravity
    // ceiling flips the right way (the same gravity-aware facing the player got).
    gravity: ambition_platformer2d_shared_tangle::gravity::GravityCtx,
) {
    // ADR 0011 — per-entity proper time on the presentation frame clock.
    // SP today: no entity carries ProperTimeScale, so every actor ticks at
    // the current world rate. The seam matters once a
    // boss freezes the world but leaves the player un-frozen, or
    // future MP boosts one player's proper time.
    for (visual, mut sprite, mut animator, scale, anchor) in &mut query {
        let dt = presentation_time.entity_dt(ambition_time::ProperTimeScale::or_default(scale));
        // ONE actor path — enemy and NPC alike resolve through the SAME picker the
        // player uses, built from the actor's real `Body*` clusters. An actor
        // attacks when its `BodyMelee` is active, whatever its disposition.
        let Some(frame) = anim_index.get(&visual.id) else {
            continue;
        };
        // Hit feedback (taking damage) is drawn by the white-silhouette overlay in
        // `presentation::rendering::hit_flash`; the source sprite stays untinted
        // (`WHITE`). Actors deliberately do NOT flash/tint on their OWN outgoing
        // attack — a flash on an attack is something a character should opt INTO,
        // not out of, and nothing wants it by default. If a game later needs
        // per-character attack presentation (a warm windup tint, a charge glow), it
        // belongs behind an explicit game-authored customization seam (a
        // per-character presentation spec), not a hardcoded default here.
        apply_character_frame(
            &mut sprite,
            &mut animator,
            anchor.map(|a| a.into_inner()),
            frame.anim,
            frame.clip.as_ref(),
            dt,
            frame.facing,
            gravity.dir_at(frame.pos),
            Color::WHITE,
            // Enemies/NPCs don't drive the crouch stance-scale seam (their compaction,
            // if any, is authored per-anim); full standing height.
            StanceSquash::NONE,
        );
    }
}

fn generic_feature_anim_owns(kind: FeatureVisualKind) -> bool {
    !matches!(kind, FeatureVisualKind::Actor)
}

/// Idle-tick the animation of every non-actor [`FeatureVisual`] that carries a
/// [`CharacterAnimator`] — an animated pickup (a spinning ring), and any future
/// animated feature (a pulsing hazard, a glowing switch). It is the feature
/// counterpart to [`animate_props`]: `sync_visuals` positions these entities by
/// id, and this advances their looping `idle` row. Players (their own picker),
/// index-driven actors ([`animate_characters`]), props ([`animate_props`]), and
/// portal sprites are excluded, so each animator is ticked by exactly one system.
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
        ),
    >,
) {
    for (visual, mut sprite, mut animator, scale, anchor) in &mut query {
        let Some(view) = feature_views.get(&visual.id) else {
            continue;
        };
        // Actors are owned by `animate_characters`, which selects their live pose
        // from `ActorAnimIndex`. Letting this generic idle-loop pass touch them as
        // well advances an Idle actor twice per frame, and continually switches a
        // moving flyer Fly -> Idle -> Fly so neither clip can leave frame zero.
        if !generic_feature_anim_owns(view.kind) {
            continue;
        }
        let dt = presentation_time.entity_dt(ambition_time::ProperTimeScale::or_default(scale));
        apply_character_frame(
            &mut sprite,
            &mut animator,
            anchor.map(|a| a.into_inner()),
            ambition_sprite_sheet::character::CharacterAnim::Idle,
            // A prop plays no moves.
            None,
            dt,
            1.0,
            ambition_platformer2d_core::Vec2::Y,
            Color::WHITE,
            // An animated feature doesn't crouch — full standing height.
            StanceSquash::NONE,
        );
    }
}

/// Prop kinds whose authored "Idle" row depicts motion (e.g. rolling
/// wheels). These props stay pinned at frame 0 in [`animate_props`]
/// until a `PropMotionState` component lands to gate their tick by
/// real motion. Add a kind here when its sprite's idle frame reads
/// as "this prop is moving" — the cart is the v1 case.
pub const PROP_KINDS_STATIC_UNTIL_MOVING: &[&str] = &["intro_cart"];

/// Tick the idle animation row for every `PropVisual` sprite that
/// owns a `CharacterAnimator`. Props have no ECS actor entity, so
/// the regular `animate_characters` lookup would skip them — without
/// this system the sprite stays pinned to frame 0 forever.
///
/// Filtered with `Without<super::super::primitives::PortalSprite>` so the gate
/// ring + gate portal stay owned by the portal-presentation systems
/// (which drive the animator from `GatePortalPhase` instead of a flat
/// Idle row tick).
///
/// Motion-gated props: a kind listed in [`PROP_KINDS_STATIC_UNTIL_MOVING`]
/// stays pinned at frame 0. The intro cart's authored "idle" row is a
/// wheel-rolling cycle that reads as "the cart is moving"; without a
/// real motion source today (no scripted push), looping it makes the
/// cart look like it's drifting in place. Until a `PropMotionState`
/// component lands, hold these kinds at rest.
pub fn animate_props(
    presentation_time: ambition_time::PresentationTime,
    mut query: Query<
        (
            &mut Sprite,
            &mut CharacterAnimator,
            &PropVisual,
            Option<&ambition_time::ProperTimeScale>,
            Option<&mut bevy::sprite::Anchor>,
        ),
        Without<super::super::primitives::PortalSprite>,
    >,
) {
    // ADR 0011 — per-entity proper time on the presentation frame clock.
    // Props that need to keep ticking when the world freezes (a clock prop in
    // a frozen boss arena, say) get a non-1.0 ProperTimeScale.
    for (mut sprite, mut animator, prop, scale, anchor) in &mut query {
        // Static-until-moving props hold frame 0 (dt = 0, so `tick` doesn't
        // advance); everything else ticks at its proper time.
        let dt = if PROP_KINDS_STATIC_UNTIL_MOVING.contains(&prop.kind.as_str()) {
            0.0
        } else {
            presentation_time.entity_dt(ambition_time::ProperTimeScale::or_default(scale))
        };
        apply_character_frame(
            &mut sprite,
            &mut animator,
            anchor.map(|a| a.into_inner()),
            ambition_sprite_sheet::character::CharacterAnim::Idle,
            // A prop plays no moves.
            None,
            dt,
            1.0,
            ambition_platformer2d_core::Vec2::Y,
            Color::WHITE,
            // Props don't crouch — full standing height.
            StanceSquash::NONE,
        );
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

    /// Which way a sheet's body POINTS ON SCREEN, as the renderer draws it.
    ///
    /// This is `apply_character_frame`'s flip decision composed with the fact
    /// it is deciding about: the art itself points `-x` when it was drawn
    /// facing left, and mirroring negates whichever way it points. A test that
    /// only checked `flip_x` would be checking a mechanism against itself —
    /// the answerable question is which way the character ends up looking.
    fn drawn_direction(authored_faces_left: bool, facing: f32) -> f32 {
        let art_points = if authored_faces_left { -1.0 } else { 1.0 };
        let flip = ambition_sprite_sheet::art_is_mirrored(authored_faces_left, facing, Vec2::NEG_Y);
        if flip {
            -art_points
        } else {
            art_points
        }
    }

    /// A left-drawn character faces the way they are going, exactly like a
    /// character whose art was drawn the other way round.
    ///
    /// facing WEST (the SVG paperdoll view is `Patent Clerk - Side Left`, and
    /// his rig declares `features.facing: "west"`), while the renderer assumed
    /// every sheet is drawn facing +x — so the one mirror it applied pointed
    /// him away from his own movement. Carl Stargan is the same paperdoll shape
    /// and was found in the same sweep.
    ///
    /// The comparison is the point: the goblin is drawn facing right, and Emmy
    /// (`noether`) is a rigged character from the SAME pipeline drawn facing
    /// east — so this is not "rigged characters are special", it is "the sheet
    /// says which way it was drawn". Given the same facing every one of these
    /// must LOOK the same way, and the right-drawn ones may not move.
    #[test]
    fn a_left_drawn_character_faces_the_way_they_are_going_like_a_right_drawn_one() {
        for left_drawn in ["patent_clerk", "carl_stargan"] {
            let sheet = record_for_sheet_key(left_drawn)
                .unwrap_or_else(|| panic!("{left_drawn}'s sheet is baked into the sheet table"));
            // The premise, pinned: this really is a LEFT-drawn sheet. Without
            // it the comparison passes for a sheet that never exercised the
            // term — which is precisely the state Carl was left in when his rig
            // declared `west` and his manifest published nothing.
            assert!(
                sheet.authored_faces_left,
                "{left_drawn}'s manifest must publish the drawn facing its rig declares \
                 (`features.facing: \"west\"`); regenerate the sheet if this is missing"
            );
            for right_drawn in ["goblin_cave_dagger", "noether"] {
                let other = record_for_sheet_key(right_drawn)
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

    /// Every baked sheet points where its body is facing — however it was
    /// drawn. The whole-population form of the rule above, so the term can
    /// never be right for the one character it was added for and wrong for the
    /// rest.
    #[test]
    fn every_baked_sheet_is_drawn_pointing_where_its_body_faces() {
        let mut left_drawn: Vec<&str> = Vec::new();
        let mut checked = 0usize;
        for key in available_sheet_keys() {
            let Some(record) = record_for_sheet_key(key) else {
                continue;
            };
            checked += 1;
            if record.authored_faces_left {
                left_drawn.push(key);
            }
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
        // THE OTHER SHEETS ARE UNMOVED, as a measurement rather than a
        // hope. `authored_faces_left` is `#[serde(default)]` and the
        // generator emits it only when true, so a sheet absent from this list
        // resolves `flip_x` to exactly `facing < 0` — byte-identical to what it
        // did before the field existed. This list is the complete set of sheets
        // whose drawing changed.
        //
        // the list is EXACT on purpose. It fails both ways: if a left-drawn
        // sheet silently stops publishing its facing (a regen against a stale
        // generator), and if some other sheet starts declaring one without
        // anybody deciding that its art was redrawn.
        left_drawn.sort_unstable();
        let expected: Vec<&str> = vec![
            // ⭐ AUTHOR AND OFFICER ARE POINTED POLYGON'S PAPERDOLLS, so they
            // inherit its west-drawn art — their sheets declare
            // `authored_faces_left: true` for the same reason its own do. Added
            // 2026-08-25 when the easter-egg fighters shipped; the guard caught
            // it, which is what a hand-kept list is for.
            "author",
            "author.0_25x",
            "author.0_5x",
            "author.potato",
            "carl_stargan",
            "carl_stargan.0_25x",
            "carl_stargan.0_5x",
            "carl_stargan.potato",
            "officer",
            "officer.0_25x",
            "officer.0_5x",
            "officer.potato",
            "patent_clerk",
            "patent_clerk.0_25x",
            "patent_clerk.0_5x",
            "patent_clerk.potato",
            // The list did not know because this test only sees sheets that are ON DISK, and the
            // polygons' art is gitignored: a checkout without it passes vacuously. that is the real
            // lesson here, not the row.
            //
            // the BRAWLER is deliberately absent: its SVG declares no facing,
            // so its sheets keep the +x default. Two polygons, one facing.
            "pointed_polygon",
            "pointed_polygon.0_25x",
            "pointed_polygon.0_5x",
            "pointed_polygon.potato",
        ];
        assert_eq!(
            left_drawn, expected,
            "exactly the west-drawn paperdoll sheets (and their quality tiers) declare a \
             left-drawn art facing; every other sheet must keep the +x default"
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

    /// The feet-anchored scheme plants the body AT the transform, so the squash
    /// must leave the anchor alone — anything else moves a quad whose placement
    /// was already correct.
    #[test]
    fn squashing_about_the_anchor_leaves_the_anchor_where_it_was() {
        let squash = StanceSquash {
            ratio_y: 0.5,
            about_quad_foot: false,
        };
        let (height, anchor) = squash.squash(100.0, -0.3);
        assert!((height - 50.0).abs() < 1e-4);
        assert!((anchor - -0.3).abs() < 1e-6, "the anchor moved to {anchor}");
    }

    /// The authored-offset scheme anchors the quad at its CENTRE and carries the
    /// placement on the translation, so the squash has to hold the art's own
    /// foot edge. Without this the crouch lifts the body clear of the floor by
    /// half of everything below the frame centre.
    #[test]
    fn squashing_about_the_quad_foot_holds_the_foot_edge() {
        for (anchor_in, ratio) in [(0.0_f32, 0.5_f32), (-0.3, 0.4), (0.2, 0.85)] {
            let squash = StanceSquash {
                ratio_y: ratio,
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

    /// A body at full standing height is untouched by either pivot — the
    /// no-crouch path must be byte-identical to having no squash at all.
    #[test]
    fn a_standing_body_is_left_exactly_as_it_was() {
        for about_quad_foot in [false, true] {
            let squash = StanceSquash {
                ratio_y: 1.0,
                about_quad_foot,
            };
            assert_eq!(squash.squash(120.0, -0.42), (120.0, -0.42));
        }
        assert_eq!(StanceSquash::NONE.squash(120.0, -0.42), (120.0, -0.42));
    }
}
