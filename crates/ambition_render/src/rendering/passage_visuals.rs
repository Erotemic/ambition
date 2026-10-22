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

use ambition_sprite_sheet::game_assets::GameAssets;

use super::primitives::{EntityArt, FeatureVisual, LoadingZoneVisual, PlayerVisual};
use super::world::BoundEntitySprite;

#[cfg(test)]
mod tests;

/// The frames of a door's opening strip.
pub const DOOR_OPENING_FRAMES: u32 = 6;

/// This body's drawing was scaled by its passage, and is put back when the
/// passage ends.
#[derive(Component)]
pub struct PassageScaled;

/// Draw each body in a passage at its passage's size, opacity and shade.
///
/// Both roads: a player body reads the look beside its pose, and a body drawn
/// as a [`FeatureVisual`] (a possessed body, an NPC) reads it in its actor
/// frame. After the animators, which set the frame and a white tint, and
/// before the part driver, which draws a part-drawn body from its root's
/// transform and tint.
#[allow(clippy::type_complexity)]
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
    mut actors: Query<(Entity, &FeatureVisual, &mut Transform, &mut Sprite, Has<PassageScaled>), Without<PlayerVisual>>,
    frames: Option<Res<ambition_sim_view::ActorAnimIndex>>,
    features: Option<Res<ambition_sim_view::FeatureViewIndex>>,
) {
    for (entity, mut transform, mut sprite, pose, look, scaled) in &mut bodies {
        draw_passage_look(&mut commands, entity, &mut transform, &mut sprite, look.copied(), pose.size.y, scaled);
    }
    for (entity, visual, mut transform, mut sprite, scaled) in &mut actors {
        let look = frames.as_deref().and_then(|frames| frames.get(&visual.id)).and_then(|frame| frame.passage_look);
        if look.is_none() && !scaled {
            continue;
        }
        let height = features.as_deref().and_then(|views| views.get(&visual.id)).map_or(0.0, |view| view.size.y);
        draw_passage_look(&mut commands, entity, &mut transform, &mut sprite, look, height, scaled);
    }
}

/// Draw one body at `look`, about its feet (`height` is its body's height),
/// or put it back when its passage is over.
fn draw_passage_look(
    commands: &mut Commands,
    entity: Entity,
    transform: &mut Transform,
    sprite: &mut Sprite,
    look: Option<ambition_sim_view::PassageLookView>,
    height: f32,
    scaled: bool,
) {
    match look {
        Some(look) => {
            let scale = look.scale.max(0.01);
            transform.scale = Vec3::new(scale, scale, 1.0);
            // About the feet: the translation is the middle of the body
            // (`sync_visuals`), and Bevy's y is up.
            transform.translation.y -= (1.0 - scale) * height * 0.5;
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

/// Declare each body that its passage fades as read as one image
/// (`ComposedBodyDemand`).
///
/// A fading body is one picture fading. A part-drawn body drawn directly
/// takes its root's opacity into each part, so each part shows the part
/// under it through it. Declared from the passage's look, the fact that
/// fades it, before the rigged driver runs.
pub fn declare_passage_fade_demand(
    players: Query<(Entity, &ambition_sim_view::PassageLookView), With<PlayerVisual>>,
    actors: Query<(Entity, &FeatureVisual), Without<PlayerVisual>>,
    frames: Option<Res<ambition_sim_view::ActorAnimIndex>>,
    mut demand: ResMut<ambition_sprite_sheet::character::rigged::ComposedBodyDemand>,
) {
    let fades = |look: &ambition_sim_view::PassageLookView| look.alpha < 1.0;
    for (body, look) in &players {
        if fades(look) {
            demand.declare(body);
        }
    }
    let Some(frames) = frames else {
        return;
    };
    for (body, visual) in &actors {
        if frames.get(&visual.id).and_then(|frame| frame.passage_look.as_ref()).is_some_and(fades) {
            demand.declare(body);
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

/// The closed door image is the current art, not the last animation frame.
/// A room look binds `EntityArt`; an ordinary door binds `BoundEntitySprite`.
/// A themed door has neither after it is dressed, so it keeps the sprite
/// chosen by its theme. Only an active opening strip is not a closed image.
fn closed_door_image(
    sprite: &Sprite,
    opening: Option<&DoorOpening>,
    art: Option<&EntityArt>,
    bound: Option<&BoundEntitySprite>,
    catalog: Option<&GameAssets>,
) -> Handle<Image> {
    if let Some(EntityArt(key)) = art {
        if let Some(image) = catalog.and_then(|catalog| catalog.entities.get(*key)) {
            return image.clone();
        }
    } else if let Some(bound) = bound {
        if let Some(image) = catalog.and_then(|catalog| catalog.entities.get(bound.key)) {
            return image.clone();
        }
    }
    if let Some(opening) = opening {
        if sprite.image == opening.strip {
            return opening.closed.clone();
        }
    }
    sprite.image.clone()
}

/// An opening strip is published beside the closed image, with the same
/// asset source and name followed by `_opening.png`.
fn opening_strip(assets: &AssetServer, closed: &Handle<Image>) -> Option<Handle<Image>> {
    let path = assets.get_path(closed)?;
    let file = path.path().to_string_lossy();
    let stem = file.strip_suffix(".png")?;
    let strip = bevy::asset::AssetPath::from(format!("{stem}_opening.png"))
        .with_source(path.source().clone_owned())
        .into_owned();
    Some(assets.load::<Image>(strip))
}

/// Draw each door a passage goes through at its opening frame. Follow any
/// change to the door's closed art while the passage is open, and restore
/// the current art when the passage ends.
pub fn sync_door_openings(
    mut commands: Commands,
    passages: Option<Res<ambition_sim_view::PassageViews>>,
    assets: Option<Res<AssetServer>>,
    images: Option<Res<Assets<Image>>>,
    layouts: Option<ResMut<Assets<TextureAtlasLayout>>>,
    catalog: Option<Res<GameAssets>>,
    mut doors: Query<(
        Entity,
        &LoadingZoneVisual,
        &mut Sprite,
        Option<&EntityArt>,
        Option<&BoundEntitySprite>,
        Option<&mut DoorOpening>,
    )>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    let (Some(passages), Some(assets), Some(images), Some(mut layouts)) = (passages, assets, images, layouts) else {
        return;
    };
    for (entity, visual, mut sprite, art, bound, opening) in &mut doors {
        let open = passages.door_open(rooms.of(entity), &visual.id).filter(|open| *open > 0.0);
        match (open, opening) {
            (Some(open), Some(mut opening)) => {
                let closed = closed_door_image(&sprite, Some(&opening), art, bound, catalog.as_deref());
                if closed != opening.closed {
                    let Some(strip) = opening_strip(&assets, &closed) else {
                        // The new art has no strip. Draw its closed image,
                        // not an opening of the previous kind of door.
                        sprite.image = closed;
                        sprite.texture_atlas = None;
                        commands.entity(entity).try_remove::<DoorOpening>();
                        continue;
                    };
                    opening.closed = closed;
                    opening.strip = strip;
                    opening.layout = None;
                }
                if opening.layout.is_none() {
                    let Some(image) = images.get(&opening.strip) else {
                        // Keep the new closed art while its strip loads.
                        sprite.image = opening.closed.clone();
                        sprite.texture_atlas = None;
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
                let closed = closed_door_image(&sprite, None, art, bound, catalog.as_deref());
                if sprite.image != closed {
                    sprite.image = closed.clone();
                }
                let Some(strip) = opening_strip(&assets, &closed) else {
                    continue;
                };
                commands.entity(entity).try_insert(DoorOpening { closed, strip, layout: None });
            }
            (None, Some(opening)) => {
                sprite.image = closed_door_image(&sprite, Some(&opening), art, bound, catalog.as_deref());
                sprite.texture_atlas = None;
                commands.entity(entity).try_remove::<DoorOpening>();
            }
            (None, None) => {}
        }
    }
}
