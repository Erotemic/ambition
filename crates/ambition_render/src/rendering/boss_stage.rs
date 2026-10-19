//! What a boss stages around it (`ambition.boss.stage`): the light of its
//! room, and its name card.
//!
//! The light is two veils over the camera's view while the camera is in the
//! boss's room: black for its dark, white for its flash. They lie over the
//! bodies and under the effects (`BOSS_STAGE_Z`), so sparks, lightning and
//! glowing eyes show through the dark. Compositing, not drawing: the same
//! with screen shaders off.
//!
//! The name card is the boss's name, large across the screen, with its title
//! under it, at the alpha its conductor asks for.

use bevy::prelude::*;

use crate::ui_fonts::{UiFontWeight, UiFonts};
use ambition_platformer2d_core::config::WORLD_Z_FX;
use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRoomOf, PrimaryLiveRoom, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::BossRenderIndex;

/// Over the bodies, under the effects.
pub const BOSS_STAGE_Z: f32 = WORLD_Z_FX - 0.5;
/// The veils are this wide and high, centred on the camera: more than any view.
const VEIL_SIZE: f32 = 40_000.0;
/// The dark at its darkest: the room's shapes are only just there.
const DARKEST: f32 = 0.93;
/// The white of a full flash.
const BRIGHTEST: f32 = 0.75;

/// A veil of a boss's stage: its dark (`false`) or its flash (`true`).
#[derive(Component, Clone, Copy, Debug)]
pub struct BossStageVeil {
    flash: bool,
}

/// The one name card's root node.
#[derive(Component, Clone, Copy, Debug)]
pub struct BossNameCard;

/// The stage of the boss in the primary live room: the lowest light and the
/// brightest flash any boss there asks for.
fn stage_here(
    index: &BossRenderIndex,
    sprites: &Query<(Entity, &super::primitives::FeatureVisual)>,
    rooms: &LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    primary: &PrimaryLiveRoom,
) -> Option<ambition_sim_view::BossStage> {
    let here = primary.get()?;
    sprites
        .iter()
        .filter(|(entity, _)| rooms.room_of(*entity) == Some(here))
        .filter_map(|(_, sprite)| index.get(&sprite.id))
        .map(|view| view.stage)
        .reduce(|a, b| ambition_sim_view::BossStage {
            lit: a.lit.min(b.lit),
            flash: a.flash.max(b.flash),
            card: a.card.max(b.card),
        })
}

/// Put the veils over the camera's view at the light the boss asks for.
pub fn light_the_boss_stage(
    mut commands: Commands,
    // `Option`: a composition with no sim read model has no boss to stage.
    index: Option<Res<BossRenderIndex>>,
    sprites: Query<(Entity, &super::primitives::FeatureVisual)>,
    rooms: LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    primary: PrimaryLiveRoom,
    cameras: Query<&Transform, (With<MainCamera>, Without<BossStageVeil>)>,
    mut veils: Query<(&BossStageVeil, &mut Transform, &mut Sprite)>,
) {
    let stage = index
        .as_deref()
        .and_then(|index| stage_here(index, &sprites, &rooms, &primary))
        .unwrap_or_default();
    let dark = Color::srgba(0.0, 0.0, 0.0, (1.0 - stage.lit).clamp(0.0, 1.0) * DARKEST);
    let flash = Color::srgba(0.86, 0.92, 1.0, stage.flash.clamp(0.0, 1.0) * BRIGHTEST);
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let at = camera.translation.truncate();
    if veils.is_empty() {
        for (is_flash, color) in [(false, dark), (true, flash)] {
            commands.spawn((
                BossStageVeil { flash: is_flash },
                Sprite { color, custom_size: Some(Vec2::splat(VEIL_SIZE)), ..Default::default() },
                Transform::from_translation(at.extend(BOSS_STAGE_Z + if is_flash { 0.05 } else { 0.0 })),
                Name::new(if is_flash { "Boss stage: flash" } else { "Boss stage: dark" }),
            ));
        }
        return;
    }
    for (veil, mut transform, mut sprite) in &mut veils {
        transform.translation.x = at.x;
        transform.translation.y = at.y;
        sprite.color = if veil.flash { flash } else { dark };
    }
}

/// Show the name card of the boss in the primary live room while its
/// conductor asks for it.
#[allow(clippy::too_many_arguments)]
pub fn show_boss_name_card(
    mut commands: Commands,
    cards: Query<Entity, With<BossNameCard>>,
    index: Res<BossRenderIndex>,
    sprites: Query<(Entity, &super::primitives::FeatureVisual)>,
    rooms: LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    primary: PrimaryLiveRoom,
    ui_fonts: Option<Res<UiFonts>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    for entity in &cards {
        commands.entity(entity).despawn();
    }
    let Some(session_scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref()) else {
        return;
    };
    let Some(here) = primary.get() else {
        return;
    };
    let Some((view, card)) = sprites
        .iter()
        .filter(|(entity, _)| rooms.room_of(*entity) == Some(here))
        .filter_map(|(_, sprite)| index.get(&sprite.id))
        .map(|view| (view, view.stage.card.clamp(0.0, 1.0)))
        .filter(|(_, card)| *card > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1))
    else {
        return;
    };
    let font = |size: f32, weight: UiFontWeight| {
        ui_fonts.as_deref().map(|fonts| fonts.text_font(size, weight)).unwrap_or(TextFont {
            font_size: FontSize::Px(size),
            ..default()
        })
    };
    // It settles into place as it comes in.
    let rise = (1.0 - card) * 18.0;
    commands
        .spawn_session_scoped(
            session_scope,
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Percent(30.0),
                    margin: UiRect::top(Val::Px(rise)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                ZIndex(36),
                Name::new("Boss name card"),
                BossNameCard,
            ),
        )
        .with_children(|root| {
            root.spawn((
                Text::new(view.name.to_uppercase()),
                font(72.0, UiFontWeight::Semibold),
                TextColor(Color::srgba(0.97, 0.93, 0.86, card)),
                TextShadow { offset: Vec2::new(0.0, 3.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85 * card) },
            ));
            // A rule under the name, as wide as the card has come in.
            root.spawn((
                Node { width: Val::Px(420.0 * card), height: Val::Px(2.0), ..default() },
                BackgroundColor(Color::srgba(0.62, 0.86, 1.0, 0.8 * card)),
            ));
            if let Some(title) = &view.title {
                root.spawn((
                    Text::new(title.clone()),
                    font(28.0, UiFontWeight::Regular),
                    TextColor(Color::srgba(0.70, 0.88, 1.0, card)),
                    TextShadow { offset: Vec2::new(0.0, 2.0), color: Color::srgba(0.0, 0.0, 0.0, 0.8 * card) },
                ));
            }
        });
}
