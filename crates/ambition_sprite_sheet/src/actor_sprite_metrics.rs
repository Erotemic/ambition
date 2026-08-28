use ambition_platformer2d_core as ae;

// ⭐⭐ MOVED OUT OF `ambition_boss_encounter` 2026-08-28 (D117). It is SPRITE
// METRICS — frame dimensions, `PixelRect`, `NamedPixelRect`, a render size, a
// per-animation table — and every one of those is this crate's. It lived in the
// boss crate because a boss was the first thing that needed it, and that is what
// made `CombatGeometry` look like boss vocabulary: the trait names this type, so
// a census read the whole seam as the boss crate's.
//
// ⛔ `ambition_characters` COULD NOT HOST IT, which is what the earlier refusal
// measured correctly: `characters` has no `sprite_sheet` dependency and adding
// one inverts the edge, because `sprite_sheet` depends on `characters`.

/// Snapshot of the sprite generator's `body_metrics` for a boss,
/// captured once at sprite-registry lookup time so per-tick
/// damage/hurtbox math doesn't re-query the SheetRegistry resource.
///
/// `body_pixel_bbox` is the single overall body bbox (legacy /
/// single-piece bosses). `body_pixel_parts` is the multi-rect
/// representation for disjointed-piece bosses (head + body + arms).
/// Either one or both may be populated; the consumer picks parts
/// when present and falls back to bbox otherwise.
///
/// `sprite_render_size` is the world-space extent of the rendered
/// sprite quad — i.e. `BossSheetSpec::render_size(boss.size)`. The
/// hurtbox / hitbox math uses this (NOT `boss.size`) as the world
/// scale so the cyan / red / yellow boxes line up with the visible
/// sprite. Without this distinction, the boss spawns at LDtk size
/// (e.g. 128×160) but renders 1.6× bigger (~256×256), and the boxes
/// end up half the size of the visible body.
#[derive(Clone, Debug, Default)]
pub struct ActorSpriteMetrics {
    pub frame_width: u32,
    pub frame_height: u32,
    pub body_pixel_bbox: Option<crate::PixelRect>,
    pub body_pixel_parts: Vec<crate::NamedPixelRect>,
    /// World-space extent of the rendered sprite quad. Equal to
    /// `BossSheetSpec::render_size(boss.size)` at derivation time.
    /// Falls back to `(boss.size, boss.size)` when the sprite spec
    /// isn't known (test fixtures); consumers treat zero as
    /// "no render size yet, use ctx.size".
    pub sprite_render_size: ae::Vec2,
    /// World-space offset from `boss.pos` to the body's bounding
    /// AABB center. Captures the fact that the body bbox inside the
    /// sprite frame isn't necessarily at the frame center —
    /// the gradient sentinel's body sits a few pixels left of center
    /// and ~17 px above frame center, which scales to ~(-6, -35) in
    /// world space at 256×256 render. Without this offset,
    /// `boss.aabb()` is centered on `boss.pos` but the visible body
    /// is centered ~41 px above, so the pogo zone / orange debug
    /// box / body-contact zone all sit "below" the visible body
    /// and pogo doesn't register where the player aims.
    pub combat_offset: ae::Vec2,
    /// Per-animation `{hurtbox, hitbox}` data keyed by animation
    /// name (matches the spritesheet rows: `"rest"`,
    /// `"floor_slam"`, `"side_sweep"`, …). The renderer fills
    /// `hurtbox` from each animation's union alpha-bbox; the
    /// adapter declares `hitbox` rects for attack animations.
    /// Consumers (`damageable_volumes`, `volumes_for_profile`)
    /// look up by current animation name to scale hurtboxes /
    /// hitboxes with the on-screen sprite pose.
    pub animations: std::collections::HashMap<String, crate::AnimationMetrics>,
}

// ⛔ THE INHERENT IMPL HAD TO COME TOO, and the ORPHAN RULE is why: only the
// crate that defines a type may write an inherent impl for it. That is the
// cheapest possible reminder that a type and its methods are one thing to move.

impl ActorSpriteMetrics {
    /// True iff this snapshot carries at least one rectangle the
    /// derivation can use.
    pub fn has_body(&self) -> bool {
        !self.body_pixel_parts.is_empty() || self.body_pixel_bbox.is_some()
    }

    /// Per-animation hurtbox lookup. Used by `damageable_volumes`
    /// to size the hurtbox to the *currently-playing* animation
    /// (so attack frames with extended arms get a wider hurtbox
    /// than the rest pose). Returns `None` if the animation has
    /// no per-animation override; the caller falls back to
    /// `body_pixel_parts` / `body_pixel_bbox`.
    pub fn hurtbox_for_animation(&self, animation: &str) -> Option<&crate::AnimationBox> {
        self.animations.get(animation)?.hurtbox.as_ref()
    }

    /// Per-animation hitbox lookup. Used by `volumes_for_profile`
    /// to read the sprite-author-declared damage geometry for an
    /// attack animation (so a side-sweep's hitbox covers both
    /// extended arms, not the generic bounding rect). Returns
    /// `None` if the animation has no authored hitbox; the
    /// caller falls back to its hardcoded volume math.
    pub fn hitbox_for_animation(&self, animation: &str) -> Option<&crate::AnimationBox> {
        self.animations.get(animation)?.hitbox.as_ref()
    }
}
