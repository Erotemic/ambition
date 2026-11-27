//! String follows a shared load, then falls and fades after release.

use super::actors::rigged::{ask_for_parts, forget_parts, PartSetDraws, PartSetRequest};
use super::posed_hand::VisualRoots;
use ambition_platformer2d_core::{
    config::{world_to_bevy, WORLD_Z_PLAYER},
    RoomGeometry,
};
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRoomInstance, LiveRoomOf, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::{HeldItemView, HostileWieldedItemsView};
use bevy::prelude::*;
use std::collections::BTreeSet;

const FADE_SECONDS: f32 = 0.4;
/// A string is drawn just under the foot (or body) it hangs from.
const UNDER_HOLDER: f32 = 0.01;
const FOOT_READER: &str = "supported_carry:feet";
const FOOT_TRACKS: [&str; 6] = [
    "near_foot",
    "front_foot",
    "right_foot",
    "far_foot",
    "back_foot",
    "left_foot",
];

#[derive(Component)]
#[require(super::AnimatedWorldSpriteTint)]
pub(super) struct SlingString {
    holder: Entity,
    room: LiveRoomInstance,
    arm: usize,
    released: f32,
    #[cfg(feature = "card_view")]
    pub(super) span: SlingSpan,
}

#[cfg(feature = "card_view")]
#[derive(Clone, Copy)]
pub(super) struct SlingSpan {
    pub(super) root: Entity,
    pub(super) foot_part: Option<usize>,
    pub(super) load: Option<(Entity, f32)>,
}

pub(super) fn sync_strings(
    mut commands: Commands,
    held: Res<HeldItemView>,
    wielded: Res<HostileWieldedItemsView>,
    rooms: LiveRoomOf<RoomGeometry>,
    session: Option<Res<ActiveSessionScope>>,
    time: ambition_time::PresentationTime,
    roots: VisualRoots,
    posed: Query<&PartSetDraws>,
    requests: Query<(Entity, &PartSetRequest)>,
    mut strings: Query<(Entity, &mut SlingString, &mut Transform, &mut Sprite)>,
    #[cfg(feature = "card_view")] loads: Query<
        (
            &ambition_platformer2d_shared_tangle::lifecycle::PresentationOf,
            &ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance,
            &super::posed_hand::HeldArtOf,
            &Transform,
        ),
        Without<SlingString>,
    >,
) {
    let Some(scope) = SessionSpawnScope::for_optional_active_session(session.as_deref()) else {
        for (entity, ..) in &strings {
            commands.entity(entity).despawn();
        }
        return;
    };
    let supports: Vec<_> = held
        .0
        .iter()
        .filter_map(|row| Some((row.holder.body, row.room?, row.support.as_ref()?)))
        .chain(
            wielded
                .0
                .iter()
                .filter_map(|row| Some((row.holder.body, row.room?, row.support.as_ref()?))),
        )
        .collect();
    let wanted_roots: BTreeSet<_> = supports
        .iter()
        .flat_map(|(_, _, support)| support.holders.iter())
        .map(|holder| roots.of(holder.body, holder.feature.as_deref()))
        .collect();
    for &root in &wanted_roots {
        if requests
            .get(root)
            .is_ok_and(|(_, request)| request.tracks_of(FOOT_READER).is_some())
        {
            continue;
        }
        ask_for_parts(&mut commands, root, FOOT_READER, &FOOT_TRACKS);
    }
    for (root, request) in &requests {
        if request.tracks_of(FOOT_READER).is_some() && !wanted_roots.contains(&root) {
            forget_parts(&mut commands, root, FOOT_READER);
        }
    }
    #[cfg(feature = "card_view")]
    let load_draws = if supports.is_empty() {
        std::collections::HashMap::new()
    } else {
        loads.iter().map(|(owner, room, art, transform)| {
            ((owner.0, room.0), (art.0, transform.translation.z))
        }).collect::<std::collections::HashMap<_, _>>()
    };
    let wanted: Vec<_> = supports
        .into_iter()
        .flat_map(|(holder, room, support)| {
            support.holders.iter().enumerate().map(move |(arm, body)| {
                (
                    holder,
                    room,
                    arm,
                    body,
                    support.geometry.attachments[arm],
                    support.geometry.anchor,
                )
            })
        })
        .filter_map(|(holder, room, arm, body, end, load)| {
            let geometry = rooms.in_room(room)?;
            let root = roots.of(body.body, body.feature.as_deref());
            // The holder's own depth: a seated holder's string is in its seat.
            let holder_z = roots.drawn_z(body.body, body.feature.as_deref()).unwrap_or(WORLD_Z_PLAYER);
            let foot = posed
                .get(root)
                .ok()
                .and_then(|draws| Some((draws.place, draws.parts(FOOT_READER).last()?)));
            let a = foot.map_or_else(
                || world_to_bevy(&geometry.0, end, holder_z),
                |(place, foot)| (place.to_matrix() * foot.local.to_matrix()).transform_point3(Vec3::ZERO),
            );
            let b = world_to_bevy(&geometry.0, load, a.z);
            let delta = b - a;
            let mut centre = (a + b) * 0.5;
            centre.z = a.z - UNDER_HOLDER;
            Some((
                holder,
                room,
                arm,
                Transform::from_translation(centre)
                    .with_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x))),
                Vec2::new(delta.truncate().length(), 1.5),
                #[cfg(feature = "card_view")]
                SlingSpan {
                    root,
                    foot_part: foot.map(|(_, foot)| foot.paint_index),
                    load: load_draws.get(&(holder, room)).copied(),
                },
                #[cfg(not(feature = "card_view"))]
                (),
            ))
        })
        .collect();
    for (entity, mut string, mut transform, mut sprite) in &mut strings {
        if let Some((_, _, _, at, size, _span)) = wanted.iter().find(|(holder, room, arm, ..)| {
            *holder == string.holder && *room == string.room && *arm == string.arm
        }) {
            if string.released > 0.0 {
                commands.entity(entity).insert(
                    ambition_platformer2d_shared_tangle::lifecycle::PresentationOf(string.holder),
                );
            }
            string.released = 0.0;
            #[cfg(feature = "card_view")]
            { string.span = *_span; }
            *transform = *at;
            sprite.custom_size = Some(*size);
            sprite.color.set_alpha(1.0);
        } else {
            string.released += time.scaled_dt();
            if string.released >= FADE_SECONDS {
                commands.entity(entity).despawn();
            } else {
                transform.translation.y -= time.scaled_dt() * (80.0 + string.released * 500.0);
                sprite.color.set_alpha(1.0 - string.released / FADE_SECONDS);
                commands
                    .entity(entity)
                    .remove::<ambition_platformer2d_shared_tangle::lifecycle::PresentationOf>();
            }
        }
    }
    for (holder, room, arm, at, size, _span) in wanted {
        if strings.iter().any(|(_, string, ..)| {
            string.holder == holder && string.room == room && string.arm == arm
        }) {
            continue;
        }
        commands.spawn_session_scoped(
            scope.in_room(Some(room)),
            (
                SlingString {
                    holder,
                    room,
                    arm,
                    released: 0.0,
                    #[cfg(feature = "card_view")]
                    span: _span,
                },
                ambition_platformer2d_shared_tangle::lifecycle::PresentationOf(holder),
                Sprite::from_color(Color::srgb(0.9, 0.83, 0.65), size),
                at,
            ),
        );
    }
}
