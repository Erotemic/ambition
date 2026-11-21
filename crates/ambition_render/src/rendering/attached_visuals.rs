//! Reconcile body attachments by stable presentation identity.

use super::presented_bodies::{PresentedBodies, PresentedBody, PresentedBodyKey};
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, PresentationOf, SessionSpawnScope, SpawnSessionScopedExt,
};
use bevy::prelude::*;
use std::{collections::BTreeMap, marker::PhantomData};

/// One effect family attached to one body representation.
#[derive(Component)]
pub struct BodyAttachedVisual<F: Send + Sync + 'static> {
    pub body: Entity,
    key: PresentedBodyKey,
    family: PhantomData<F>,
}

/// Update surviving attachments, retire absent owners, then create new attachments.
/// Art and placement remain with the effect family.
pub fn reconcile_presented_visuals<F: Send + Sync + 'static, T>(
    commands: &mut Commands,
    bodies: &PresentedBodies,
    active_session: Option<&ActiveSessionScope>,
    sprite: Option<&Handle<Image>>,
    visuals: &mut Query<(
        Entity,
        &mut BodyAttachedVisual<F>,
        &mut Transform,
        &mut Sprite,
    )>,
    name: &'static str,
    desired: impl Fn(&PresentedBody) -> Option<T>,
    place: impl Fn(&ambition_platformer2d_core::World, &mut Transform, &mut Sprite, &T),
) {
    let mut wanted: BTreeMap<_, _> = bodies
        .iter()
        .filter_map(|body| {
            let facts = desired(&body)?;
            Some((body.key(), (body, facts)))
        })
        .collect();
    for (entity, mut owner, mut transform, mut art) in visuals.iter_mut() {
        let Some((body, facts)) = wanted.remove(&owner.key) else {
            commands.entity(entity).despawn();
            continue;
        };
        if owner.body != body.visual {
            owner.body = body.visual;
            commands.entity(entity).insert(PresentationOf(body.visual));
        }
        if let Some(room) = body.geometry {
            place(room, &mut transform, &mut art, &facts);
        }
    }
    let Some(sprite) = sprite else {
        return;
    };
    let Some(scope) = SessionSpawnScope::for_optional_active_session(active_session) else {
        return;
    };
    for (key, (body, facts)) in wanted {
        let Some(room) = body.geometry else {
            continue;
        };
        let mut transform = Transform::default();
        let mut art = Sprite::from_image(sprite.clone());
        place(room, &mut transform, &mut art, &facts);
        commands.spawn_session_scoped(
            scope.in_room(body.room),
            (
                art,
                transform,
                Visibility::Visible,
                BodyAttachedVisual::<F> {
                    body: body.visual,
                    key,
                    family: PhantomData,
                },
                PresentationOf(body.visual),
                Name::new(name),
            ),
        );
    }
}
