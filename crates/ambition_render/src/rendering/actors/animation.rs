//! Per-frame sprite animation systems (player, characters, props).
//!
//! Split out of the former 883-line `actors/mod.rs` (2026-06-15).

use super::*;

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
    anim: ambition_gameplay_core::character_sprites::CharacterAnim,
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
    world_time: Res<ambition_time::WorldTime>,
    mut query: Query<
        (
            &mut Sprite,
            &mut CharacterAnimator,
            &ambition_gameplay_core::features::BodyPoseView,
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
        // ADR 0011 — `entity_dt` collapses to `sim_dt` when no ProperTimeScale is
        // set (SP default), so bullet-time / hitstop / pause still slow the
        // animation in lockstep.
        let dt = world_time.entity_dt(ambition_time::ProperTimeScale::or_default(scale));
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
    world_time: Res<ambition_time::WorldTime>,
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
            Without<ambition_gameplay_core::rooms::PortalSprite>,
            Without<PropVisual>,
        ),
    >,
    // Materialized per-actor pose read-model (built by `rebuild_actor_anim_index`
    // in the render presentation chain just before this system) — the renderer
    // animates from a snapshot, no longer borrowing the live actor clusters.
    anim_index: Res<ambition_gameplay_core::features::ActorAnimIndex>,
    // Localized gravity, so an enemy/NPC wall-walking or on a flipped-gravity
    // ceiling flips the right way (the same gravity-aware facing the player got).
    gravity: ambition_platformer_primitives::gravity::GravityCtx,
) {
    // ADR 0011 — per-entity proper time. SP today: no entity carries
    // ProperTimeScale, so `entity_dt` collapses to `sim_dt` and
    // every actor ticks at the world rate. The seam matters once a
    // boss freezes the world but leaves the player un-frozen, or
    // future MP boosts one player's proper time.
    for (visual, mut sprite, mut animator, scale, anchor) in &mut query {
        let dt = world_time.entity_dt(ambition_time::ProperTimeScale::or_default(scale));
        // ONE actor path — enemy and NPC alike resolve through the SAME picker the
        // player uses, built from the actor's real `Body*` clusters. An actor
        // attacks when its `BodyMelee` is active, whatever its disposition.
        let Some(frame) = anim_index.get(&visual.id) else {
            continue;
        };
        // Hit feedback is drawn by the white-silhouette overlay in
        // `presentation::rendering::hit_flash`. Keep the warm attack tint on the
        // multiplicative `sprite.color` channel — it's a separate signal
        // (telegraphing the actor's outgoing swing, not its own damage).
        let color = if frame.attacking {
            Color::srgba(1.0, 0.85, 0.55, 1.0)
        } else {
            Color::WHITE
        };
        apply_character_frame(
            &mut sprite,
            &mut animator,
            anchor.map(|a| a.into_inner()),
            frame.anim,
            dt,
            frame.facing,
            gravity.dir_at(frame.pos),
            color,
            // Enemies/NPCs don't drive the crouch stance-scale seam (their compaction,
            // if any, is authored per-anim); full standing height.
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
/// Filtered with `Without<ambition_gameplay_core::rooms::PortalSprite>` so the gate
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
    world_time: Res<ambition_time::WorldTime>,
    mut query: Query<
        (
            &mut Sprite,
            &mut CharacterAnimator,
            &PropVisual,
            Option<&ambition_time::ProperTimeScale>,
            Option<&mut bevy::sprite::Anchor>,
        ),
        Without<ambition_gameplay_core::rooms::PortalSprite>,
    >,
) {
    // ADR 0011 — per-entity proper time. Props that need to keep
    // ticking when the world freezes (a clock prop in a frozen
    // boss arena, say) get a non-1.0 ProperTimeScale.
    for (mut sprite, mut animator, prop, scale, anchor) in &mut query {
        // Static-until-moving props hold frame 0 (dt = 0, so `tick` doesn't
        // advance); everything else ticks at its proper time.
        let dt = if PROP_KINDS_STATIC_UNTIL_MOVING.contains(&prop.kind.as_str()) {
            0.0
        } else {
            world_time.entity_dt(ambition_time::ProperTimeScale::or_default(scale))
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
            ambition_gameplay_core::character_sprites::CharacterAnim::Idle,
            dt,
            1.0,
            ambition_engine_core::Vec2::Y,
            Color::WHITE,
            // Props don't crouch — full standing height.
            1.0,
        );
    }
}
