//! Visual marker components, scene entity registry, color/z helpers,
//! and the small `spawn_world_label` utility.

use ambition_engine as ae;
use bevy::prelude::*;

use crate::config::{world_to_bevy, WORLD_Z_BLOCK, WORLD_Z_DUMMY, WORLD_Z_PLAYER};
use crate::features::FeatureVisualKind;

#[derive(Resource)]
pub struct SceneEntities {
    pub player: Entity,
    pub hud: Entity,
    pub quest_panel: Entity,
}

#[derive(Component)]
pub struct PlayerVisual;

#[derive(Component)]
pub struct HudText;

/// Marker for the dedicated quest-log panel. Separated from `HudText`
/// so the quest list lives in its own UI surface (top-right anchored)
/// instead of trailing the debug-stats dump. Updated by
/// `update_quest_panel`.
#[derive(Component)]
pub struct QuestPanelText;

#[derive(Component)]
pub struct RoomVisual;

#[derive(Component)]
pub struct FeatureVisual {
    pub id: String,
}

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

pub(super) fn object_visual_kind(kind: &ae::RoomObjectKind) -> Option<FeatureVisualKind> {
    match kind {
        ae::RoomObjectKind::DamageVolume(_) => Some(FeatureVisualKind::Hazard),
        ae::RoomObjectKind::Pickup(_) => Some(FeatureVisualKind::Pickup),
        ae::RoomObjectKind::Chest(_) => Some(FeatureVisualKind::Chest),
        ae::RoomObjectKind::Breakable(_) => Some(FeatureVisualKind::Breakable),
        ae::RoomObjectKind::Interactable(interactable)
            if matches!(interactable.kind, ae::InteractionKind::Npc { .. }) =>
        {
            Some(FeatureVisualKind::Npc)
        }
        ae::RoomObjectKind::Interactable(interactable) if matches!(&interactable.kind, ae::InteractionKind::Custom(s) if s.starts_with("switch:")) => {
            Some(FeatureVisualKind::Switch)
        }
        ae::RoomObjectKind::EnemySpawn(ae::EnemyBrain::Custom(name))
            if name.starts_with("sandbag_") =>
        {
            Some(FeatureVisualKind::Sandbag)
        }
        ae::RoomObjectKind::EnemySpawn(_) => Some(FeatureVisualKind::Enemy),
        ae::RoomObjectKind::BossSpawn(_) => Some(FeatureVisualKind::Boss),
        _ => None,
    }
}

pub(super) fn feature_z(kind: FeatureVisualKind) -> f32 {
    match kind {
        FeatureVisualKind::Hazard => WORLD_Z_BLOCK + 8.0,
        FeatureVisualKind::Breakable => WORLD_Z_BLOCK + 5.0,
        FeatureVisualKind::Pickup => WORLD_Z_DUMMY + 4.0,
        FeatureVisualKind::Chest => WORLD_Z_DUMMY + 3.0,
        FeatureVisualKind::Npc => WORLD_Z_DUMMY + 2.0,
        FeatureVisualKind::Switch => WORLD_Z_DUMMY + 2.0,
        FeatureVisualKind::Enemy => WORLD_Z_DUMMY + 1.0,
        FeatureVisualKind::Sandbag => WORLD_Z_DUMMY + 1.0,
        FeatureVisualKind::Boss => WORLD_Z_DUMMY + 1.0,
    }
}

pub(super) fn feature_color(kind: FeatureVisualKind, flash: bool) -> Color {
    if flash {
        return Color::srgba(1.0, 1.0, 1.0, 1.0);
    }
    match kind {
        FeatureVisualKind::Hazard => Color::srgba(0.98, 0.12, 0.22, 0.94),
        FeatureVisualKind::Enemy => Color::srgba(0.93, 0.34, 0.28, 0.96),
        FeatureVisualKind::Sandbag => Color::srgba(0.78, 0.62, 0.42, 0.96),
        FeatureVisualKind::Boss => Color::srgba(0.78, 0.20, 0.92, 0.96),
        FeatureVisualKind::Breakable => Color::srgba(0.62, 0.42, 0.24, 0.96),
        FeatureVisualKind::Chest => Color::srgba(1.0, 0.74, 0.22, 0.96),
        FeatureVisualKind::Pickup => Color::srgba(0.42, 1.0, 0.74, 0.96),
        FeatureVisualKind::Npc => Color::srgba(0.42, 0.78, 1.0, 0.96),
        // Default off-state color for switches (red — encounter armed).
        // The on-state override happens in `sync_visuals` via the
        // `FeatureView::switch_on` flag.
        FeatureVisualKind::Switch => Color::srgba(0.95, 0.18, 0.18, 1.0),
    }
}

pub(super) fn spawn_world_label(
    commands: &mut Commands,
    world: &ae::World,
    pos: ae::Vec2,
    text: &str,
    font_size: f32,
) {
    commands.spawn((
        Text2d::new(text.to_string()),
        TextFont {
            font_size,
            ..default()
        },
        TextColor(Color::srgba(0.86, 0.94, 1.0, 0.94)),
        Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_PLAYER + 8.0)),
        Name::new(format!("World label: {text}")),
        RoomVisual,
    ));
}
