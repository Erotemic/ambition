//! Visual marker components, scene entity registry, color/z helpers,
//! and the small `spawn_world_label` utility.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_platformer2d_core::config::{
    world_to_bevy, WORLD_Z_BLOCK, WORLD_Z_DUMMY, WORLD_Z_PLAYER,
};
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_platformer2d_shared_tangle::lifecycle::{SessionSpawnScope, SpawnSessionScopedExt};

// Runtime-owned room lifecycle markers. Re-exported so presentation systems +
// existing `presentation::rendering::RoomVisual` call sites keep resolving; the
// canonical home is `ambition_platformer2d_shared_tangle::lifecycle` (content-free, so sim
// systems can tag visual entities without importing presentation).
pub use ambition_platformer2d_shared_tangle::lifecycle::{
    LoadingZoneVisual, PlayerVisual, RoomScopedEntity, RoomVisual,
};

/// Standing-stance render size of the textured player sprite, recorded
/// at sprite-build time. The crouch-squash hack in `sync_visuals` uses
/// it to scale the sprite vertically to the current `body_mode`'s
/// height while keeping the feet planted — the sprite anchor is in
/// normalized space, so a proportional vertical squash preserves foot
/// alignment without re-anchoring.
///
/// HACK(crouch-sprite-row): the robot sheet has no authored Crouching
/// row yet, so we visually squash the standing pose as a placeholder.
/// Once the sprite generator emits a real Crouch (and Crawl/MorphBall)
/// animation, this baseline + the squash branch in `sync_visuals` can
/// go away and the standing anchor will plant feet directly.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlayerSpriteBaseline {
    pub standing_render: Vec2,
    pub standing_collision: Vec2,
}

#[derive(Component)]
pub struct HudText;

/// Marker for the dedicated quest-log panel. Separated from `HudText`
/// so the quest list lives in its own UI surface (top-right anchored)
/// instead of trailing the debug-stats dump. Updated by
/// `update_quest_panel`.
#[derive(Component)]
pub struct QuestPanelText;

/// Marker for an encounter-driven lock-wall block visual. The
/// encounter system inserts `Block::solid` entries named
/// `lockwall:<encounter_id>` into `world.blocks` while the encounter
/// is in flight; `sync_lock_wall_visuals` reads that name and keeps
/// one Bevy entity per matching block in sync (spawn on first sight,
/// despawn when the block is removed). `block_name` is the full name
/// (`lockwall:goblin_encounter`, etc.) so the dedup is bullet-proof against
/// multiple concurrent encounters in the same world.
#[derive(Component, Clone, Debug)]
pub struct LockWallVisual {
    pub block_name: String,
}

/// Marker carrying an authored block visual's name, so a mid-run SUBTRACTION of
/// that block (the collision overlay's `removed_block_names` — a content gate
/// dropping authored geometry, e.g. a broken brick) can find and despawn its
/// sprite. `spawn_block` tags every block visual with this; `sync_removed_block_visuals`
/// reconciles them against the overlay. `block_name` is the authored
/// [`Block::name`](ambition_platformer2d_core::Block), the same key `removed_block_names`
/// carries, so the match is exact.
#[derive(Component, Clone, Debug)]
pub struct BlockVisual {
    pub block_name: String,
    /// Durable geometry identity. `block_name` is the human label the removal
    /// reconciler matches on; this is what a CONTACT names, and the two are kept
    /// side by side for the same reason `ae::Block` keeps both.
    pub geo_id: ambition_platformer2d_core::GeoId,
}

/// This block's art, said by the game rather than inherited from its
/// `BlockKind`.
///
/// every block of a kind drew the same picture, and nothing could say
/// otherwise. `spawn_block` resolves art from `BlockKind` alone —
/// `block_tile_sprite(Solid) -> SolidTile` for every code-authored solid in the
/// room — so a bonus block, a used bonus block and a wall were one texture.
/// `art_color` was the only per-block lever and it can only say "no art yet"
/// (a flat quad), which is a statement about ABSENCE, not identity.
///
/// and the identity a game needs is DYNAMIC. A `?`-block becomes a used block mid-play; a
/// spawn-time field on `ae::Block` could never express that.
///
/// Presentation only. Collision never reads it, exactly as `art_color` does not.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct BlockArt(pub ambition_sprite_sheet::game_assets::EntitySprite);

#[derive(Component)]
pub struct FeatureVisual {
    pub id: String,
}

/// Marker for sprites spawned from `RoomSpec.props`. Generic prop
/// animation (idle row tick) runs against `With<PropVisual>` so the
/// sprite stays alive without the engine ever seeing a feature
/// entity for the prop. Filtered with `Without<PortalSprite>` so
/// the gate ring + gate portal stay owned by the portal systems.
#[derive(Component, Clone, Debug)]
pub struct PropVisual {
    /// LDtk iid — for debug overlay + future save-key joins.
    #[allow(dead_code)]
    pub id: String,
    /// Registry key the sprite was looked up under.
    pub kind: String,
    pub name: String,
    pub size: Vec2,
    /// The authored [`PropDraw`], carried for the same reason as `size`: the
    /// sprite is REBUILT on a `GameAssets` change, and a rebuild that does not
    /// know a prop is built world silently reverts it to character sizing.
    pub draw: ambition_platformer2d_world::rooms::PropDraw,
    /// The authored vertical mirror, carried for the same reason.
    pub flip_y: bool,
}

/// Tag on the portal + gate-ring visual entities so the generic
/// `animate_characters` / `animate_props` systems skip them. Without this
/// filter the generic animator re-pins them to `Idle` every frame and
/// clobbers the row the gate-portal presentation systems request from
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
        // FULLY TRANSPARENT: this is the in-game fill, and a hidden block that
        // tinted itself would announce the secret it exists to keep. A game that
        // wants it seen once found dresses it (Mary-O swaps to the spent tile).
        ae::BlockKind::BonkOnly => Color::srgba(0.0, 0.0, 0.0, 0.0),
        ae::BlockKind::Hazard => Color::srgba(0.96, 0.18, 0.26, 0.92),
        ae::BlockKind::PogoOrb => Color::srgba(0.30, 0.95, 0.64, 0.95),
        ae::BlockKind::Rebound { .. } => Color::srgba(1.0, 0.60, 0.20, 0.95),
    }
}

/// Switch on-color: green = encounter cleared / armed for fresh attempt
/// disabled. Used as an override on top of `feature_color` when
/// `FeatureView::switch_on` is true.
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
        // ONE z for every actor. The former Npc-one-layer-higher nuance died with
        // the variant; if actor draw order ever matters it must come from a real
        // signal, not the visual kind.
        FeatureVisualKind::Actor => WORLD_Z_DUMMY + 1.0,
    }
}

/// Placeholder rectangle color for a feature with no bound sprite. For an actor
/// the tint modulates on the FIGHTING state (`fighting` = engaged) — information
/// about state, not type; every actor is ONE kind, a fighting one just reads
/// warmer. `fighting` is ignored for non-actor kinds.
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
        // Default off-state color for switches (red — encounter armed).
        // The on-state override happens in `sync_visuals` via the
        // `FeatureView::switch_on` flag.
        FeatureVisualKind::Switch => Color::srgba(0.95, 0.18, 0.18, 1.0),
    }
}

/// Colour of a static world label at full opacity. Named because the placement
/// pass ([`super::label_layout`]) is the single writer of the rendered
/// `TextColor` and needs the un-faded value to fade FROM.
pub(super) const WORLD_LABEL_COLOR: Color = Color::srgba(0.86, 0.94, 1.0, 0.94);

/// Spawn one static world-space label.
///
/// `owner_id` must be unique across every label family — the placement pass
/// keys its resolved layout by it. Static labels are prefixed by their caller
/// (`signage:` / `fixture:`) so they can never collide with a nameplate's
/// view identity, which is a bare feature/zone id.
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
            // Room load has no view in scope — it runs once for a room, not once
            // per observer — so this label is spawned unkeyed and
            // `mirror_static_world_labels_per_view` gives it to the lowest-id
            // view and copies it to the rest. In a one-view game that is exactly
            // this entity and nothing else.
            super::label_layout::StaticWorldLabel,
        ),
    );
}

#[cfg(all(test, feature = "portal_render"))]
mod portal_band_tests {
    use super::*;

    /// ⛔⛔ THE PORTAL BAND SITS BELOW THE ACTOR BAND, AND UNTIL NOW THAT WAS A
    /// COINCIDENCE OF LITERALS.
    ///
    /// `PORTAL_WINDOW_Z = 9.5`, `PORTAL_RIM_OVERLAY_Z = 10.0` and
    /// `WORLD_Z_DUMMY = 10.0` are defined in two different crates, and the portal
    /// constants' docs assert a RELATIONSHIP to this one — *"below actors so a
    /// near-side actor still occludes it"*, *"below actors"*. Nothing checked it.
    /// `ambition_platformer2d_core` guards its OWN band's order
    /// (`WORLD_Z_BLOCK < WORLD_Z_DUMMY < WORLD_Z_PLAYER < WORLD_Z_FX`) and cannot
    /// see the portal band; the portal crate cannot see the `+ 1.0` that puts an
    /// actor above `WORLD_Z_DUMMY`. **This crate is the only one that sees both.**
    ///
    /// ⭐⭐ AND IT IS THE GUARD AGAINST THE RULED-OUT CHEAP FIX. The obvious way to
    /// stop a far-side actor punching through a portal window is to raise
    /// `PORTAL_WINDOW_Z` above the actor band. It is two lines, it fixes the
    /// screenshot, and it inverts the bug: a near-side actor would then vanish
    /// behind an aperture it is standing in front of. This test fails on that
    /// change and says why in its message, which a comment in another crate
    /// could not.
    #[test]
    fn the_portal_band_stays_below_the_actor_band() {
        let actor_z = feature_z(FeatureVisualKind::Actor);
        for (name, z) in [
            (
                "PORTAL_EXIT_COPY_Z",
                ambition_portal2d_presentation::PORTAL_EXIT_COPY_Z,
            ),
            (
                "PORTAL_WINDOW_Z",
                ambition_portal2d_presentation::PORTAL_WINDOW_Z,
            ),
            (
                "PORTAL_RIM_OVERLAY_Z",
                ambition_portal2d_presentation::PORTAL_RIM_OVERLAY_Z,
            ),
        ] {
            assert!(
                z < actor_z,
                "{name} = {z} is not below the actor draw z ({actor_z}). If this \
                 moved to fix a far-side actor drawing over a portal window: that \
                 inverts the bug -- a NEAR-side actor would vanish behind an \
                 aperture it stands in front of. The fix is a per-pane compositing \
                 relation (`ambition_portal2d_presentation::pane_relation`), not a \
                 global z, because one body is near one pane and far of another in \
                 the same frame."
            );
        }
    }

    /// ⚠ The control: the band this test compares against must itself be ordered,
    /// or "below the actor band" is a comparison with nothing.
    #[test]
    fn the_actor_band_is_above_the_world_it_stands_on() {
        assert!(
            feature_z(FeatureVisualKind::Actor)
                > ambition_platformer2d_core::config::WORLD_Z_BLOCK
        );
    }
}
