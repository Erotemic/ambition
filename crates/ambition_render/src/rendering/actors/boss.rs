//! Boss sprite binding and per-frame boss animation.

use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::rendering::primitives::{FeatureVisual, PlayerVisual};
use ambition_sprite_sheet::boss::{self as sprites, BossAnimState, BossAnimator};
use ambition_sprite_sheet::character::CharacterAnimator;
use ambition_sprite_sheet::game_assets::GameAssets;

/// Replace the static `boss_core.png` look on boss feature entities with the
/// animated boss spritesheet once the asset is available. Like
/// `upgrade_actor_sprites`, but uses `BossAnimator` instead of
/// `CharacterAnimator` because the boss generator emits its own row set.
pub fn upgrade_boss_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    // Readiness, not residency; see `super::texture_is_ready`.
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    // The boss's render `size` is on its `FeatureView`; its static identity
    // (name and behavior id, for the sheet lookup) is on `BossRenderIndex`.
    // Reading both by id binds a boss sheet without the live boss clusters.
    feature_views: Res<ambition_sim_view::FeatureViewIndex>,
    boss_render: Res<ambition_sim_view::BossRenderIndex>,
    new_bosses: Query<
        (Entity, &FeatureVisual),
        (Without<CharacterAnimator>, Without<BossAnimator>),
    >,
    // Boss keys already warned about (no dedicated sheet), so each warns
    // once.
    mut warned_generic_bosses: Local<std::collections::HashSet<String>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual) in &new_bosses {
        // The read model is the gate: an id with no boss identity (not a boss,
        // or not materialized yet) is skipped.
        let (Some(view), Some(boss_ident)) =
            (feature_views.get(&visual.id), boss_render.get(&visual.id))
        else {
            continue;
        };
        // Pick the per-boss sheet by behavior id. Unrecognized bosses use the
        // gradient-sentinel sheet. With no asset, skip; `sync_visuals` draws
        // the coloured rectangle.
        let boss_name = boss_ident.name.as_str();
        let boss_behavior_id = boss_ident.behavior_id.as_str();
        let _ = boss_name;
        let boss_key = boss_behavior_id.to_ascii_lowercase().replace('-', "_");
        // Dedicated sheets are keyed by `boss_key` in the asset registry: one
        // lookup plus the generic fallback.
        let dedicated = assets.boss_sprite(&boss_key);
        // Warn once for a boss without its own sheet (it uses the generic
        // gradient-sentinel body), so missing art is not shipped silently.
        if dedicated.is_none() && warned_generic_bosses.insert(boss_key.clone()) {
            bevy::log::warn!(
                target: "ambition_platformer2d::sprites",
                "boss '{boss_key}' has no dedicated spritesheet wired — rendering with the \
                 generic boss body. If it should have its own sprite, wire a BossSheetSpec + \
                 a boss_sprites entry (keyed by boss_key) + its loader (see \
                 flying_spaghetti_monster_boss).",
            );
        }
        let Some(boss_asset) = dedicated.or(assets.boss.as_ref()) else {
            continue;
        };
        if !super::texture_is_ready(&asset_server, &images, &boss_asset.pages[0].texture) {
            continue;
        }
        let collision = BVec2::new(view.size.x, view.size.y);
        let render_size = boss_asset.spec.render_size(collision);
        let anchor = boss_asset.spec.drawn_anchor(&boss_asset.record, collision);
        let mut sprite = Sprite::from_atlas_image(
            boss_asset.texture(),
            bevy::image::TextureAtlas {
                layout: boss_asset.layout(),
                index: boss_asset.flat_index(sprites::BossAnim::Rest, 0),
            },
        );
        sprite.custom_size = Some(render_size);
        // `with_render_basis` lets a trimmed (alpha-packed) boss sheet compute
        // per-frame size and anchor in `animate_bosses`; untrimmed sheets ignore
        // it. `try_insert`: a boss can die on the frame its sheet finishes
        // loading, and `despawn_dead_dynamic_feature_visuals` then removes the
        // `FeatureVisual`. Covered by
        // `boss_pass::the_boss_sprite_upgrade_survives_its_target_being_retired`.
        commands.entity(entity).try_insert((
            sprite,
            anchor,
            BossAnimator::new(boss_asset).with_render_basis(render_size, anchor.0),
            sprites::BossDrawnCell::default(),
        ));
    }
}

/// The set [`animate_bosses`] runs in.
///
/// Camera shake and follow read the pose it resolves, so they run after it
/// (this frame's snapshot, not last frame's).
///
/// One member only. `manage_gradient_lane_visual` runs right after to read
/// the `BossAttackState` this system produces; including it would make the
/// camera wait on an unrelated hazard visual.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BossAnimation;

/// Per-frame state-driven animation for boss entities.
pub fn animate_bosses(
    // The sim owns the frame (`drive_boss_animators`); this system mirrors
    // the published cursor into the draw-only animator and renders it.
    boss_frames: Res<ambition_sim_view::BossFrameIndex>,
    // The boss's render size, which a conductor may change mid-fight
    // (`ambition.boss.scale`).
    feature_views: Res<ambition_sim_view::FeatureViewIndex>,
    // Read the cursor from `boss_frames` (by id), not a `BossAnimFrame`
    // component: this presentation entity is a `FeatureVisual` mirror, and
    // `BossAnimFrame` is on the separate sim boss entity.
    mut query: Query<
        (
            &FeatureVisual,
            &mut Sprite,
            // The animator addresses the texture, and holds the render basis
            // the boss is drawn at; the cursor is in `boss_frames`.
            &mut BossAnimator,
            Option<&mut bevy::sprite::Anchor>,
            // The cell drawn, published for the companion-layer overlays.
            Option<&mut sprites::BossDrawnCell>,
        ),
        Without<PlayerVisual>,
    >,
) {
    // ADR 0011: per-entity proper time. A boss with `ProperTimeScale > 1.0`
    // keeps animating while its `SimClock` request freezes the world
    // (ADR 0010, narrative authority).
    for (visual, mut sprite, mut animator, anchor, drawn) in &mut query {
        let Some(view) = boss_frames.get(&visual.id) else {
            continue;
        };
        // A boss drawn at another size than its sprite was bound at (a
        // conductor grew it): draw it at the size the sim builds it at.
        if let Some(feature) = feature_views.get(&visual.id) {
            let wanted = animator.spec.render_size(BVec2::new(feature.size.x, feature.size.y));
            match animator.render_basis.as_mut() {
                Some(basis) if (basis.render_size - wanted).length_squared() > 1e-4 => basis.render_size = wanted,
                None => sprite.custom_size = Some(wanted),
                _ => {}
            }
        }
        let animator = animator.into_inner();
        let state: BossAnimState = view.anim;
        let (row, frame) = drawn_cell(animator, view);
        if let Some(mut drawn) = drawn {
            // A layer content pins over the body's row, on its own clock.
            let layer = view
                .pinned
                .as_ref()
                .and_then(|pin| pin.layer.as_ref())
                .and_then(|layer| animator.pinned_cell(layer.rows.iter().map(String::as_str), layer.elapsed, layer.looping));
            *drawn = sprites::BossDrawnCell { row, frame, hurt_face: hurt_face(&animator.spec, view.hit_flash_secs), layer };
        }
        let index = animator.flat_index_at(row, frame);
        // Split sheets: select the page for the active frame before setting
        // the page-local index. Single-page bosses skip this.
        if animator.is_paged() {
            let page = animator.page_at(row, frame);
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
        // Default art faces +x. A left-drawn sheet (the mockingbird) sets
        // `authored_faces_left`, which inverts the flip. The gravity term
        // matches the player and enemy path: zero under normal gravity, and
        // under a flip it cancels the `ActorRoll` 180° mirror.
        let flip = ambition_sprite_sheet::art_is_mirrored(
            animator.spec.authored_faces_left,
            state.facing,
            // The boss's own resolved down, so a boss under flipped or
            // sideways gravity flips like the player and enemies. It
            // self-rights with `ActorRoll`, so its facing must be
            // gravity-aware or the 180° roll mirrors it backwards.
            view.gravity_dir,
        );
        sprite.flip_x = flip;
        // `render_of` is `None` for untrimmed sheets, which keep their spawn
        // size and anchor. The anchor x mirrors with the sprite flip.
        if let (Some((size, mut anchor_v)), Some(mut anchor)) =
            (animator.render_at(row, frame), anchor)
        {
            sprite.custom_size = Some(size);
            if flip {
                anchor_v.x = -anchor_v.x;
            }
            anchor.0 = anchor_v;
        }
        // Same split as `animate_characters`: hit feedback is the
        // `hit_flash` overlay; the warm attack tint stays on `sprite.color`
        // so the player can read the boss's swing telegraph. The tint is a
        // placeholder tell: a sheet that draws its tells opts out.
        sprite.color = if animator.spec.attack_tint && (state.attack_active || state.attack_windup) {
            Color::srgba(1.0, 0.85, 0.55, 1.0)
        } else {
            Color::WHITE
        };
    }
}

/// The record `(row, frame)` a boss draws this frame.
///
/// The render only addresses the atlas cell the sim's cursor names, so the
/// sprite and the strike geometry share one sim frame. In order:
///
/// 1. A hit reaction, for a BAKED boss. It is presentation: the `Hit` row is
///    drawn while the flash runs, and the cursor (and its geometry) continues
///    underneath, so the attack resumes at the strike's frame. A boss that is
///    hit through posed parts ([`ambition_sim_view::BossFrameView::posed_by_rig`])
///    does not draw it: its hurt parts stay in the pose of the sim's row, and
///    parts drawn in the `Hit` row would not be where a strike lands on them.
///    Its hit feedback is the flash overlay, in the pose it has. A boss drawn
///    from parts does not draw it either: it keeps its pose and wears the
///    `Hit` row's face ([`hurt_face`]; Jon, 2026-10-24: the jump to the hurt
///    row "makes a disjointed movement").
/// 2. A PINNED row (content's `PinnedRow`: a scholar's tumble), when this
///    sheet has one of its names.
/// 3. The cursor's slot row.
pub fn drawn_cell(animator: &BossAnimator, view: &ambition_sim_view::BossFrameView) -> (usize, usize) {
    match hit_reaction(&animator.spec, view.posed_by_rig, view.hit_flash_secs) {
        Some((anim, frame)) => (animator.spec.record_row(anim), frame),
        None => view
            .pinned
            .as_ref()
            .and_then(|pin| animator.pinned_cell(pin.rows.iter().map(String::as_str), pin.elapsed, pin.looping))
            .unwrap_or((animator.spec.record_row(view.cursor_anim), view.cursor_frame)),
    }
}

/// The presentation-only `Hit` cell a boss draws while its flash runs: none
/// for a boss that is hit through posed parts. See [`drawn_cell`].
fn hit_reaction(
    spec: &sprites::BossSheetSpec,
    posed_by_rig: bool,
    hit_flash_secs: f32,
) -> Option<(sprites::BossAnim, usize)> {
    if posed_by_rig || spec.parts.is_some() {
        return None;
    }
    spec.hit_reaction_frame(hit_flash_secs)
}

/// The frame of its `Hit` row whose face a boss drawn from parts wears while
/// its flash runs, over the pose it has. `None` for a baked boss (it draws
/// the row: [`hit_reaction`]) and for one that is not hit.
fn hurt_face(spec: &sprites::BossSheetSpec, hit_flash_secs: f32) -> Option<usize> {
    spec.parts.as_ref()?;
    spec.hit_reaction_frame(hit_flash_secs).map(|(_, frame)| frame)
}

/// A boss sheet drawn from a character's parts ([`sprites::BossSheetSpec::parts`])
/// asks for that character's sheet: nothing else in a boss room names it.
///
/// Asked once each time the sheet is only declared (never realized, or retired
/// by a quality change); the materializer owns what happens next.
pub fn demand_boss_part_sheets(
    assets: Option<Res<GameAssets>>,
    bosses: Query<&BossAnimator>,
    demand: Option<ResMut<ambition_characters::load_demand::CharacterLoadDemand>>,
    mut asked: Local<std::collections::HashSet<String>>,
) {
    let (Some(assets), Some(mut demand)) = (assets, demand) else {
        return;
    };
    for boss in &bosses {
        let Some(id) = boss.spec.parts.as_deref() else {
            continue;
        };
        match assets.characters.sheet_state(id) {
            ambition_sprite_sheet::character::CharacterSheetState::Declared { .. } => {
                if asked.insert(id.to_owned()) {
                    demand.request(id);
                }
            }
            _ => {
                asked.remove(id);
            }
        }
    }
}

/// Give a boss whose sheet is drawn from a character's parts that character's
/// [`CharacterAnimator`], which is what the rigged driver follows, once its
/// sheet is realized; and a new one when the sheet is realized again at
/// another tier.
///
/// The animator is never ticked: [`pose_boss_part_animators`] shows the cell
/// [`animate_bosses`] drew. Its render basis is the boss's own, so a part
/// lands where the baked boss frame puts its pixel.
pub fn bind_boss_part_animators(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    bosses: Query<(Entity, &BossAnimator, Option<&super::BoundSpriteQuality>), With<FeatureVisual>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, boss, bound) in &bosses {
        let (Some(id), Some(basis)) = (boss.spec.parts.as_deref(), boss.render_basis) else {
            continue;
        };
        let Some(sheet) = assets.characters.sheet(id) else {
            continue;
        };
        if bound.is_some_and(|bound| *bound == super::BoundSpriteQuality::of(sheet)) {
            continue;
        }
        let mut animator = CharacterAnimator::new(sheet);
        animator.ensure_render_basis(basis.render_size, basis.feet_anchor);
        commands.entity(entity).try_insert((
            animator,
            super::BoundSpriteQuality::of(sheet),
        ));
    }
}

/// Show, on a part-drawn boss's [`CharacterAnimator`], the cell
/// [`animate_bosses`] drew this frame: the row of the same NAME, the same
/// frame, at the size the boss is drawn at. After `animate_bosses` and before
/// the rigged driver, which then draws the boss from its parts over the root's
/// baked frame.
///
/// ⛔ The size follows the boss's: a conductor that grows a boss
/// (`ambition.boss.scale`) grows the root's frame, and a parts basis left at
/// the old size reads the taller root as a stance squash and stretches the
/// parts in height alone (Wolfenstein's serum, 2026-10-19).
pub fn pose_boss_part_animators(
    mut bosses: Query<(&BossAnimator, &sprites::BossDrawnCell, &mut CharacterAnimator, &Sprite), With<FeatureVisual>>,
) {
    for (boss, drawn, mut animator, sprite) in &mut bosses {
        if let Some(basis) = boss.render_basis {
            if animator.render_basis.is_some_and(|parts| parts.render_size != basis.render_size) {
                animator.render_basis = Some(basis);
            }
        }
        let Some(name) = boss.record.rows.get(drawn.row).map(|row| row.animation.as_str()) else {
            continue;
        };
        if let Some(slot) = animator.spec.clip_slot([name]) {
            animator.show_cell(slot, drawn.frame);
        }
        // Hit, it wears its `Hit` row's face in the pose it has.
        let face = drawn.hurt_face.and_then(|frame| {
            let row = boss.record.rows.get(boss.spec.record_row(sprites::BossAnim::Hit))?;
            Some((row.animation.as_str(), frame))
        });
        let worn = animator.expression.as_ref().map(|(row, frame)| (row.as_str(), *frame));
        if worn != face {
            animator.expression = face.map(|(row, frame)| (row.to_owned(), frame));
        }
        // Its layer (a ward), over whatever row it is drawn in.
        let layer = drawn.layer.and_then(|(row, frame)| Some((boss.record.rows.get(row)?.animation.as_str(), frame, sprite.flip_x)));
        let worn = animator.layer.as_ref().map(|(row, frame, mirrored)| (row.as_str(), *frame, *mirrored));
        if worn != layer {
            animator.layer = layer.map(|(row, frame, mirrored)| (row.to_owned(), frame, mirrored));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_sprite_sheet::boss::{AnimRow, BossAnim, BossSheetSpec};

    /// A BAKED BOSS FLINCHES; A BOSS HIT THROUGH POSED PARTS DOES NOT.
    ///
    /// The `Hit` row is presentation: the sim's cursor does not enter it. A
    /// baked boss is hit through geometry that does not follow its drawn row,
    /// so the flinch costs nothing. A boss with a body rig is hit through
    /// parts posed from the sim's row: drawn in the `Hit` row, its visible
    /// head would be in one place and the part a strike lands on in another.
    #[test]
    fn a_boss_posed_by_its_rig_draws_no_presentation_only_hit_row() {
        let spec = BossSheetSpec {
            rows: vec![
                (BossAnim::Rest, AnimRow { frame_count: 4, duration_secs: 0.12 }),
                (BossAnim::Hit, AnimRow { frame_count: 5, duration_secs: 0.09 }),
            ],
            ..BossSheetSpec::unauthored()
        };
        let flash = 0.09 * 5.0;
        assert_eq!(
            hit_reaction(&spec, false, flash),
            Some((BossAnim::Hit, 0)),
            "control: a baked boss with a `Hit` row and a flash draws the flinch"
        );
        assert_eq!(
            hit_reaction(&spec, true, flash),
            None,
            "a boss posed by its rig drew the `Hit` row over the pose its hurt parts have"
        );
        assert_eq!(hit_reaction(&spec, false, 0.0), None, "no flash, no flinch");
        assert_eq!(hurt_face(&spec, flash), None, "control: a baked boss draws the row, not a face");
        // A boss drawn from parts keeps its pose and wears the row's face.
        let parts = BossSheetSpec { parts: Some("toy".into()), ..spec };
        assert_eq!(hit_reaction(&parts, false, flash), None, "a part-drawn boss jumped to its `Hit` row");
        assert_eq!(hurt_face(&parts, flash), Some(0), "a part-drawn boss hit wears the `Hit` row's face");
        assert_eq!(hurt_face(&parts, 0.0), None, "no flash, no hurt face");
    }
}
