//! The passages through doors and gates, drawn (`rooms::passage`; Jon,
//! 2026-10-19: "You should open the door walk through it").
//!
//! - A body in a passage is drawn at the size, opacity and shade its passage
//!   gives ([`ambition_sim_view::PassageLookView`]), about its feet.
//! - A door that a passage goes through draws its opening row: the frames of
//!   its `<picture>_opening.png` strip, published beside each door picture
//!   (closed in the first frame, open in the last).
//!
//! A gate's ripple is drawn by the gate's own animation
//! (`gate_portal_visuals::sync_portal_sprite_animation`).

use bevy::prelude::*;

use super::primitives::{LoadingZoneVisual, PlayerVisual};

/// The frames of a door's opening strip.
pub const DOOR_OPENING_FRAMES: u32 = 6;

/// This body's drawing was scaled by its passage, and is put back when the
/// passage ends.
#[derive(Component)]
pub struct PassageScaled;

/// Draw each body in a passage at its passage's size, opacity and shade.
///
/// After the animator, which sets the frame and a white tint, and before the
/// part driver, which draws a part-drawn body from its root's transform and
/// tint.
pub fn apply_passage_looks(
    mut commands: Commands,
    mut bodies: Query<
        (
            Entity,
            &mut Transform,
            &mut Sprite,
            &ambition_sim_view::BodyPoseView,
            Option<&ambition_sim_view::PassageLookView>,
            Has<PassageScaled>,
        ),
        With<PlayerVisual>,
    >,
) {
    for (entity, mut transform, mut sprite, pose, look, scaled) in &mut bodies {
        match look {
            Some(look) => {
                let scale = look.scale.max(0.01);
                transform.scale = Vec3::new(scale, scale, 1.0);
                // About the feet: the translation is the middle of the body
                // (`sync_visuals`), and Bevy's y is up.
                transform.translation.y -= (1.0 - scale) * pose.size.y * 0.5;
                let lit = 1.0 - look.shade.clamp(0.0, 1.0);
                let tint = sprite.color.to_srgba();
                sprite.color = Color::srgba(tint.red * lit, tint.green * lit, tint.blue * lit, tint.alpha * look.alpha);
                if !scaled {
                    commands.entity(entity).try_insert(PassageScaled);
                }
            }
            None if scaled => {
                transform.scale = Vec3::ONE;
                commands.entity(entity).try_remove::<PassageScaled>();
            }
            None => {}
        }
    }
}

/// A door's closed picture, while its opening strip draws it.
#[derive(Component)]
pub struct DoorOpening {
    closed: Handle<Image>,
    strip: Handle<Image>,
    layout: Option<Handle<TextureAtlasLayout>>,
}

/// Draw each door a passage goes through at its opening frame, and give it
/// back its closed picture when the passage is gone.
pub fn sync_door_openings(
    mut commands: Commands,
    passages: Option<Res<ambition_sim_view::PassageViews>>,
    assets: Option<Res<AssetServer>>,
    images: Option<Res<Assets<Image>>>,
    layouts: Option<ResMut<Assets<TextureAtlasLayout>>>,
    mut doors: Query<(Entity, &LoadingZoneVisual, &mut Sprite, Option<&mut DoorOpening>)>,
) {
    let (Some(passages), Some(assets), Some(images), Some(mut layouts)) = (passages, assets, images, layouts) else {
        return;
    };
    for (entity, visual, mut sprite, opening) in &mut doors {
        let open = passages.door_open(&visual.id).filter(|open| *open > 0.0);
        match (open, opening) {
            (Some(open), Some(mut opening)) => {
                if opening.layout.is_none() {
                    let Some(image) = images.get(&opening.strip) else {
                        continue;
                    };
                    let size = image.size();
                    let cell = UVec2::new(size.x / DOOR_OPENING_FRAMES, size.y);
                    opening.layout = Some(layouts.add(TextureAtlasLayout::from_grid(cell, DOOR_OPENING_FRAMES, 1, None, None)));
                }
                let Some(layout) = opening.layout.clone() else {
                    continue;
                };
                let index = ((open * (DOOR_OPENING_FRAMES - 1) as f32).round() as usize).min(DOOR_OPENING_FRAMES as usize - 1);
                if sprite.image != opening.strip {
                    sprite.image = opening.strip.clone();
                }
                match sprite.texture_atlas.as_mut() {
                    Some(atlas) if atlas.layout == layout => atlas.index = index,
                    _ => sprite.texture_atlas = Some(TextureAtlas { layout, index }),
                }
            }
            (Some(_), None) => {
                // A door drawn from a sheet has no strip of its own.
                if sprite.texture_atlas.is_some() {
                    continue;
                }
                let Some(path) = assets.get_path(&sprite.image) else {
                    continue;
                };
                let file = path.path().to_string_lossy().to_string();
                let Some(stem) = file.strip_suffix(".png") else {
                    continue;
                };
                // From the source the closed picture came from.
                let strip = bevy::asset::AssetPath::from(format!("{stem}_opening.png"))
                    .with_source(path.source().clone_owned())
                    .into_owned();
                let strip = assets.load::<Image>(strip);
                commands.entity(entity).try_insert(DoorOpening { closed: sprite.image.clone(), strip, layout: None });
            }
            (None, Some(opening)) => {
                sprite.image = opening.closed.clone();
                sprite.texture_atlas = None;
                commands.entity(entity).try_remove::<DoorOpening>();
            }
            (None, None) => {}
        }
    }
}
