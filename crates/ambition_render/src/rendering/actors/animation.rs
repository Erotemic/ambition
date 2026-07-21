//! Per-frame sprite animation systems (player, characters, props).
//!
//! Split out of the former 883-line `actors/mod.rs` (2026-06-15).

use bevy::prelude::*;

use crate::rendering::primitives::{FeatureVisual, PlayerVisual, PropVisual};
use ambition_platformer_primitives::feature_kind::FeatureVisualKind;
use ambition_sprite_sheet::character::CharacterAnimator;

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
    dt: f32,
    facing: f32,
    gravity_dir: ambition_engine_core::Vec2,
    color: Color,
    // Body-mode stance compaction (crouch/crawl/slide/morph shrinks the AABB and
    // slides `pos` down to keep feet planted). `current AABB height / base height`,
    // clamped (0, 1]; `1.0` for a body at full standing height. Applied to the
    // TRIMMED per-frame height so trimmed sheets match the untrimmed stance-scale in
    // `sync_visuals` instead of restoring the standing height at the lowered pos.
    stance_ratio_y: f32,
) {
    // The stance squash is a PLACEHOLDER for sheets that lack a row for the
    // compact pose (the fallback then shows standing art at a shrunken AABB).
    // A sheet that natively owns the requested row drew the pose at world
    // scale inside the fixed logical frame — squashing it again would flatten
    // authored crouch/ball art, so the ratio collapses to 1.0.
    let stance_ratio_y = if animator.spec.maps(anim) {
        1.0
    } else {
        stance_ratio_y
    };
    animator.request(anim);
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
    let flip = ambition_platformer_primitives::gravity::gravity_aware_flip_x(facing, gravity_dir);
    sprite.flip_x = flip;
    sprite.color = color;
    // Self-capture the trim basis from the spawn-built sprite the first time we
    // see this animator: its `custom_size` + feet anchor ARE the full-logical
    // render basis, so no spawn site has to thread it in (a forgotten call would
    // silently misalign a trimmed sheet). No-op once set / when untrimmed.
    if let (Some(size), Some(a)) = (sprite.custom_size, anchor.as_deref()) {
        animator.ensure_render_basis(size, a.0);
    }
    // Alpha-trimmed (atlas-packed) sheets: each frame is stored at its own
    // trimmed size + offset, so re-derive the sprite size + anchor per frame to
    // keep the logical frame fixed. `current_render` returns `None` for
    // untrimmed sheets, so those keep their fixed spawn-time size/anchor and are
    // byte-identical. The anchor x mirrors with the facing flip so an
    // off-centre trim stays consistent left/right.
    if let (Some((mut size, mut anchor_v)), Some(anchor)) = (animator.current_render(), anchor) {
        // Crouch/crawl/slide/morph: scale the trimmed height by the collision-shrink
        // ratio so the feet stay planted (the normalized anchor preserves foot
        // alignment). Without this a trimmed sheet renders standing height at the
        // lowered crouch pos and sinks through the floor.
        size.y *= stance_ratio_y;
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
        // Presentation time uses this rendered frame's delta while applying the
        // authoritative world-clock and proper-time scales. This keeps the
        // authored cadence independent of fixed / rollback tick duration.
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
            dt,
            pose.facing,
            pose.gravity_dir,
            Color::WHITE,
            pose.stance_ratio_y,
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
    gravity: ambition_platformer_primitives::gravity::GravityCtx,
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
            dt,
            frame.facing,
            gravity.dir_at(frame.pos),
            Color::WHITE,
            // Enemies/NPCs don't drive the crouch stance-scale seam (their compaction,
            // if any, is authored per-anim); full standing height.
            1.0,
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
            dt,
            1.0,
            ambition_engine_core::Vec2::Y,
            Color::WHITE,
            1.0,
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
        // Route through the SAME frame-apply chokepoint as actors so a trimmed
        // prop sheet gets the self-captured trim basis too (props used to skip
        // it and rendered a trimmed cell at full-frame size — misaligned).
        // Props don't face or tint: facing = 1.0 is unflipped under normal
        // gravity (`Vec2::Y` is +y/down here), tint stays WHITE.
        apply_character_frame(
            &mut sprite,
            &mut animator,
            anchor.map(|a| a.into_inner()),
            ambition_sprite_sheet::character::CharacterAnim::Idle,
            dt,
            1.0,
            ambition_engine_core::Vec2::Y,
            Color::WHITE,
            // Props don't crouch — full standing height.
            1.0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{generic_feature_anim_owns, FeatureVisualKind};

    #[test]
    fn actor_animators_are_not_owned_by_the_generic_feature_idle_loop() {
        assert!(!generic_feature_anim_owns(FeatureVisualKind::Actor));
        assert!(generic_feature_anim_owns(FeatureVisualKind::Pickup));
        assert!(generic_feature_anim_owns(FeatureVisualKind::Hazard));
    }
}
