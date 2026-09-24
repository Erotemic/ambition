//! Visual marker components, scene entity registry, color/z helpers,
//! and the small `spawn_world_label` utility.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_platformer2d_core::config::{
    world_to_bevy, WORLD_Z_BLOCK, WORLD_Z_DUMMY, WORLD_Z_PLAYER,
};
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_platformer2d_shared_tangle::lifecycle::{SessionSpawnScope, SpawnSessionScopedExt};

// Runtime-owned room lifecycle markers, re-exported so existing
// `presentation::rendering::RoomVisual` call sites still resolve. The
// canonical home is `ambition_platformer2d_shared_tangle::lifecycle`
// (content-free, so sim systems can tag visuals without importing
// presentation).
pub use ambition_platformer2d_shared_tangle::lifecycle::{
    LoadingZoneVisual, PlayerVisual, RoomScopedEntity, RoomVisual,
};

/// Standing-stance render size of the textured player sprite, recorded when
/// the sprite is built. The crouch squash in `sync_visuals` uses it to scale
/// the sprite vertically to the current `body_mode` height with the feet
/// planted. The anchor is normalized, so a proportional squash keeps the feet
/// aligned without re-anchoring.
///
/// HACK(crouch-sprite-row): the robot sheet has no Crouching row yet, so the
/// standing pose is squashed as a placeholder. When the generator emits real
/// Crouch, Crawl, and MorphBall rows, remove this baseline and the squash
/// branch in `sync_visuals`.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlayerSpriteBaseline {
    pub standing_render: Vec2,
    pub standing_collision: Vec2,
}

#[derive(Component)]
pub struct HudText;

/// Marker for the quest-log panel. Separate from `HudText`, so the quest list
/// has its own UI surface (top-right). Updated by `update_quest_panel`.
#[derive(Component)]
pub struct QuestPanelText;

/// Marker for an encounter-driven lock-wall block visual. The encounter
/// system inserts `Block::solid` entries named `lockwall:<encounter_id>` into
/// `world.blocks` while the encounter runs. `sync_lock_wall_visuals` keeps one
/// entity per matching block (spawn on first sight, despawn on removal).
/// `block_name` is the full name, so concurrent encounters do not collide.
#[derive(Component, Clone, Debug)]
pub struct LockWallVisual {
    pub block_name: String,
}

/// Marker with an authored block visual's name, so a mid-run removal of that
/// block (the collision overlay's `removed_block_names`, for example a broken
/// brick) can find and despawn its sprite. `spawn_block` tags every block
/// visual; `sync_removed_block_visuals` reconciles them against the overlay.
/// `block_name` is the authored [`Block::name`](ambition_platformer2d_core::Block),
/// the same key `removed_block_names` carries.
#[derive(Component, Clone, Debug)]
pub struct BlockVisual {
    pub block_name: String,
    /// Durable geometry identity. `block_name` is the label the removal
    /// reconciler matches; this is what a contact names. Both are kept, as
    /// in `ae::Block`.
    pub geo_id: ambition_platformer2d_core::GeoId,
}

/// This block's art, chosen by the game instead of by its `BlockKind`.
///
/// `spawn_block` resolves art from `BlockKind` alone
/// (`block_tile_sprite(Solid) -> SolidTile`), so a bonus block, a used bonus
/// block, and a wall would share one texture. `art_color` can only say "no art
/// yet" (a flat quad). The identity can also change during play (a `?` block
/// becomes a used block), which a spawn-time field on `ae::Block` cannot
/// express.
///
/// Presentation only. Collision never reads it, like `art_color`.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct BlockArt(pub ambition_sprite_sheet::game_assets::EntitySprite);

#[derive(Component)]
pub struct FeatureVisual {
    pub id: String,
}

/// Marker for sprites spawned from `RoomSpec.props`. Prop animation (idle row
/// tick) runs on `With<PropVisual>`, so the engine needs no feature entity for
/// the prop. `Without<PortalSprite>` leaves the gate ring and gate portal to
/// the portal systems.
#[derive(Component, Clone, Debug)]
pub struct PropVisual {
    /// LDtk iid — for debug overlay + future save-key joins.
    #[allow(dead_code)]
    pub id: String,
    /// Registry key the sprite was looked up under.
    pub kind: String,
    pub name: String,
    pub size: Vec2,
    /// The authored [`PropDraw`], kept like `size`: the sprite is rebuilt on a
    /// `GameAssets` change, and a rebuild without it would revert a world-built
    /// prop to character sizing.
    pub draw: ambition_platformer2d_world::rooms::PropDraw,
    /// The authored vertical mirror, carried for the same reason.
    pub flip_y: bool,
}

/// Tag on the portal and gate-ring visuals so `animate_characters` and
/// `animate_props` skip them. Otherwise the generic animator sets `Idle`
/// every frame and overrides the row the gate-portal systems request from
/// `GatePortalPhase`. Those systems own these entities' animator request,
/// frame tick, and atlas index.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalSprite;

#[derive(Component)]
pub struct HealthOverlayVisual;

pub fn block_color(kind: ae::BlockKind) -> Color {
    match kind {
        ae::BlockKind::Solid => Color::srgba(0.25, 0.28, 0.36, 1.0),
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Soft,
        } => Color::srgba(0.32, 0.20, 0.72, 0.88),
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Hard,
        } => Color::srgba(0.52, 0.14, 0.80, 0.96),
        ae::BlockKind::OneWay => Color::srgba(0.36, 0.43, 0.62, 0.92),
        // Fully transparent: this is the in-game fill, and a tinted hidden
        // block would reveal its secret. A game that wants it seen once found
        // changes its art (Mary-O swaps to the spent tile).
        ae::BlockKind::BonkOnly => Color::srgba(0.0, 0.0, 0.0, 0.0),
        ae::BlockKind::Hazard => Color::srgba(0.96, 0.18, 0.26, 0.92),
        ae::BlockKind::PogoOrb => Color::srgba(0.30, 0.95, 0.64, 0.95),
        ae::BlockKind::Rebound { .. } => Color::srgba(1.0, 0.60, 0.20, 0.95),
    }
}

/// Switch on-colour (green: encounter cleared). Used instead of
/// `feature_color` when `FeatureView::switch_on` is true.
pub fn switch_on_color() -> Color {
    Color::srgba(0.20, 0.90, 0.30, 1.0)
}

pub(super) fn feature_z(kind: FeatureVisualKind) -> f32 {
    match kind {
        FeatureVisualKind::Hazard => WORLD_Z_BLOCK + 8.0,
        FeatureVisualKind::Breakable => WORLD_Z_BLOCK + 5.0,
        FeatureVisualKind::Pickup => WORLD_Z_DUMMY + 4.0,
        FeatureVisualKind::Chest => WORLD_Z_DUMMY + 3.0,
        FeatureVisualKind::Switch => WORLD_Z_DUMMY + 2.0,
        // One z for every actor. If actor draw order ever matters, it must come
        // from a real signal, not the visual kind.
        FeatureVisualKind::Actor => WORLD_Z_DUMMY + 1.0,
    }
}

/// Placeholder rectangle colour for a feature with no bound sprite. For an
/// actor the tint changes with the fighting state (`fighting` = engaged): a
/// fighting actor reads warmer. `fighting` is ignored for other kinds.
pub(super) fn feature_color(kind: FeatureVisualKind, fighting: bool, flash: bool) -> Color {
    if flash {
        return Color::srgba(1.0, 1.0, 1.0, 1.0);
    }
    match kind {
        FeatureVisualKind::Hazard => Color::srgba(0.98, 0.12, 0.22, 0.94),
        FeatureVisualKind::Actor => {
            if fighting {
                Color::srgba(0.93, 0.34, 0.28, 0.96)
            } else {
                Color::srgba(0.42, 0.78, 1.0, 0.96)
            }
        }
        FeatureVisualKind::Breakable => Color::srgba(0.62, 0.42, 0.24, 0.96),
        FeatureVisualKind::Chest => Color::srgba(1.0, 0.74, 0.22, 0.96),
        FeatureVisualKind::Pickup => Color::srgba(0.42, 1.0, 0.74, 0.96),
        // Default off-state colour for switches (red: encounter armed).
        // `sync_visuals` applies the on-state colour from
        // `FeatureView::switch_on`.
        FeatureVisualKind::Switch => Color::srgba(0.95, 0.18, 0.18, 1.0),
    }
}

/// Colour of a static world label at full opacity. The placement pass
/// ([`super::label_layout`]) is the only writer of the rendered `TextColor`
/// and needs the unfaded value to fade from.
pub(super) const WORLD_LABEL_COLOR: Color = Color::srgba(0.86, 0.94, 1.0, 0.94);

/// Spawn one static world-space label.
///
/// `owner_id` must be unique across every label family: the placement pass
/// keys its layout by it. Callers prefix static labels (`signage:`,
/// `fixture:`) so they never collide with a nameplate's bare feature or zone
/// id.
pub(super) fn spawn_world_label(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    owner_id: impl Into<String>,
    family: super::label_layout::WorldLabelFamily,
    pos: ae::Vec2,
    text: &str,
    font_size: f32,
) {
    let anchor = world_to_bevy(world, pos, WORLD_Z_PLAYER + 8.0);
    commands.spawn_session_scoped(
        session_scope,
        (
            Text2d::new(text.to_string()),
            TextFont {
                font_size: FontSize::Px(font_size),
                ..default()
            },
            TextColor(WORLD_LABEL_COLOR),
            Transform::from_translation(anchor),
            Name::new(format!("World label: {text}")),
            RoomVisual,
            super::label_layout::WorldLabel::new(owner_id, family, anchor)
                .with_colors(WORLD_LABEL_COLOR, None),
            // Room load has no view in scope, so this label is spawned unkeyed and
            // `mirror_static_world_labels_per_view` gives it to the lowest-id view
            // and copies it to the rest. In a one-view game this is the only
            // entity.
            super::label_layout::StaticWorldLabel,
        ),
    );
}


#[cfg(test)]
mod actor_band_tests {
    use super::*;

    /// Every feature kind stays inside the z band its offset names.
    ///
    /// `feature_z` adds an offset to a datum: `WORLD_Z_BLOCK + 8.0` for a hazard,
    /// `WORLD_Z_DUMMY + 4.0` for a pickup. The tightest margin is `Hazard` (8.0
    /// against a 10.0 gap to `WORLD_Z_DUMMY`), so a kind at `WORLD_Z_BLOCK + 11.0`
    /// would draw above the whole actor band.
    ///
    /// This also pins `WORLD_Z_DUMMY < actor`. `ambition_portal2d_presentation`
    /// pins `portal band <= WORLD_Z_DUMMY` in its own default build, so together
    /// they give `portal band <= WORLD_Z_DUMMY < actor` without an optional
    /// feature (`ambition_render` has `default = []`).
    ///
    /// The match is exhaustive on purpose: a new `FeatureVisualKind` fails to
    /// compile here (E0004) until someone places it in a band.
    #[test]
    fn every_feature_kind_stays_inside_the_band_its_offset_names() {
        let block = ambition_platformer2d_core::config::WORLD_Z_BLOCK;
        let dummy = ambition_platformer2d_core::config::WORLD_Z_DUMMY;
        let player = ambition_platformer2d_core::config::WORLD_Z_PLAYER;

        for kind in [
            FeatureVisualKind::Actor,
            FeatureVisualKind::Hazard,
            FeatureVisualKind::Breakable,
            FeatureVisualKind::Chest,
            FeatureVisualKind::Pickup,
            FeatureVisualKind::Switch,
        ] {
            // No `_` arm: a new kind is a compile error until it is placed.
            let (band_name, floor, ceiling) = match kind {
                FeatureVisualKind::Hazard | FeatureVisualKind::Breakable => {
                    ("the BLOCK band", block, dummy)
                }
                FeatureVisualKind::Actor
                | FeatureVisualKind::Chest
                | FeatureVisualKind::Pickup
                | FeatureVisualKind::Switch => ("the DUMMY band", dummy, player),
            };
            let z = feature_z(kind);
            assert!(
                z >= floor && z < ceiling,
                "{kind:?} draws at {z}, outside {band_name} [{floor}, {ceiling}). \
                 An offset that overruns its band puts the kind in the NEXT one, \
                 where it draws over things it should sit behind — and the line \
                 that does it looks as reasonable as every other arm of `feature_z`."
            );
        }
    }

    #[test]
    fn every_actor_draws_strictly_above_the_shared_world_datum() {
        let datum = ambition_platformer2d_core::config::WORLD_Z_DUMMY;
        let actor = feature_z(FeatureVisualKind::Actor);
        assert!(
            actor > datum,
            "the actor draw z ({actor}) is not above WORLD_Z_DUMMY ({datum}). The \
             portal band is pinned at or below that datum in \
             `ambition_portal2d_presentation`, and the two halves together are \
             what keep a portal window from being raised over the cast."
        );
    }
}
