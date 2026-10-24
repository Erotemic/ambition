//! Worn gear: the pictures of a gear, drawn on the anchor part of the body
//! that wears it (`docs/systems/worn-gear.md`).
//!
//! The render reads the gear each body wears from the observation boundary
//! (`WornGearView`) and the gear's pictures and fits from the
//! `WornGearManifest`. It asks the body's rigged root for the draws of the
//! fit's track under a reader of its own ([`READER`] and the track), and
//! draws two sprites on the part that answers: the `back` picture under the
//! whole body and the `front` picture just over the anchor part. Both are at
//! the part's pivot, turned with the part and squashed with it, moved by the
//! fit, and mirrored with the body.
//!
//! A boss that is drawn from a baked sheet has no parts to answer. Its gear
//! is drawn at the middle of its picture, moved by the fit's offset (the fit
//! of its sheet key), and mirrored with it: the place of the anchor on that
//! sheet, as near as one place for all its frames can be. Another body
//! drawn baked shows no gear, and the first time this is seen for a gear and
//! a sheet, one warning says so.

use std::collections::BTreeSet;

use bevy::prelude::*;
use bevy::sprite::Anchor;

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRoomInstance, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_shared_tangle::worn_gear::{GearFit, WornGearManifest};
use ambition_sim_view::WornGearView;
use ambition_sprite_sheet::character::CharacterAnimator;

use super::actors::rigged::{ask_for_parts, forget_parts, PartSetDraws, PartSetRequest, PlacedPart};
use super::posed_hand::VisualRoots;

/// The start of the name of each ask: a reader for each anchor track
/// (`worn_gear:head`), because an answer does not say which track a part is
/// of.
pub const READER: &str = "worn_gear:";

/// How far under the body the `back` picture is drawn: half the depth
/// between two of the body's draws, so it is under each draw of the body,
/// and under the body's one quad when its parts are drawn into one image.
const GEAR_UNDER_BODY: f32 = 0.5e-4;

/// How far over the anchor part the `front` picture is drawn: half the depth
/// between two of the body's draws, so a part drawn after the anchor (a near
/// arm) is over it.
const GEAR_OVER_ANCHOR: f32 = 0.5e-4;

/// One picture of a gear.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GearLayer {
    /// Under the body.
    Back,
    /// Over the anchor part.
    Front,
}

/// What a worn gear visual draws: one picture of this gear on this body, in
/// this live room. A visual is kept while the body wears the gear there, and
/// moved; another gear, body or room is a new visual.
#[derive(Component, Debug)]
pub struct WornGearVisual {
    body: Entity,
    gear: String,
    layer: GearLayer,
    room: LiveRoomInstance,
}

/// Where the two pictures of a gear go, in the space sprites are drawn in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GearPlacement {
    pub back: Transform,
    pub front: Transform,
}

/// Where the pictures of a gear go on `part`, a draw of the anchor track
/// placed under `place` ([`PartSetDraws::place`]), as `fit` puts them.
/// `mirror_row` says the body draws the mirror row of its sheet: it faces the
/// other way with no mirror in `place`, so the gear is mirrored.
///
/// The pictures are at the part's pivot, turned with the part and squashed
/// with it (its own scale, not the body's size: the gear's size is in world
/// units), then moved by the fit's offset and turned by its turn, in the
/// body's frame. A mirrored body has the mirror image of all of it. `None`
/// for a part with no up axis.
pub fn place_gear(part: &PlacedPart, place: Transform, fit: &GearFit, mirror_row: bool) -> Option<GearPlacement> {
    let world = place.to_matrix() * part.local.to_matrix();
    let pivot = world.transform_point3(Vec3::ZERO);
    let x = world.transform_vector3(Vec3::X).truncate();
    let y = world.transform_vector3(Vec3::Y).truncate();
    let mirrored = (x.perp_dot(y) < 0.0) != mirror_row;
    let side = if mirrored { -1.0 } else { 1.0 };
    // The part's turn in the world is the turn of its up axis: a mirror
    // reverses its x axis and not its up axis.
    let up = y.try_normalize()?;
    let part_turn = (-up.x).atan2(up.y);
    let offset = Vec2::from_angle(part_turn).rotate(Vec2::new(side * fit.offset.x, fit.offset.y));
    let squash = part.local.scale.truncate().abs();
    let at = |z: f32| {
        Transform::from_translation((pivot.truncate() + offset).extend(z))
            .with_rotation(Quat::from_rotation_z(part_turn + side * fit.turn_degrees.to_radians()))
            .with_scale(Vec3::new(side * fit.scale * squash.x, fit.scale * squash.y, 1.0))
    };
    Some(GearPlacement {
        back: at(place.translation.z - GEAR_UNDER_BODY),
        front: at(pivot.z + GEAR_OVER_ANCHOR),
    })
}

/// Where the pictures of a gear go on a boss drawn from a baked sheet: at
/// `middle` (the place of its picture), moved by the fit's offset in the
/// body's frame, turned by its turn, and mirrored when the picture is.
pub fn place_gear_baked(middle: Vec3, mirrored: bool, fit: &GearFit) -> GearPlacement {
    let side = if mirrored { -1.0 } else { 1.0 };
    let at = |z: f32| {
        Transform::from_translation((middle.truncate() + Vec2::new(side * fit.offset.x, fit.offset.y)).extend(z))
            .with_rotation(Quat::from_rotation_z(side * fit.turn_degrees.to_radians()))
            .with_scale(Vec3::new(side * fit.scale, fit.scale, 1.0))
    };
    GearPlacement { back: at(middle.z - GEAR_UNDER_BODY), front: at(middle.z + 2.0 * GEAR_OVER_ANCHOR) }
}

/// The sprite anchor of a socket: the point of a picture from 0 to 1 on each
/// axis, from its top left.
fn socket_anchor(socket: Vec2) -> Anchor {
    Anchor(Vec2::new(socket.x - 0.5, 0.5 - socket.y))
}

/// [`sync_worn_gear_visuals`]'s condition: a body wears gear, or did on the
/// last frame (that pass takes the sprites down and forgets the asks). The
/// view is rebuilt each tick, so its change tick cannot tell an empty frame
/// from a full one.
pub fn gear_is_worn_or_was(view: Option<Res<WornGearView>>, mut were: Local<bool>) -> bool {
    let worn = view.is_some_and(|view| !view.0.is_empty());
    let run = worn || *were;
    *were = worn;
    run
}

/// Why a gear is not shown, said one time for each gear and sheet.
#[derive(Default)]
pub struct GearWarnings(BTreeSet<(String, String)>);

impl GearWarnings {
    fn once(&mut self, gear: &str, sheet: &str, why: impl FnOnce() -> String) {
        if self.0.insert((gear.to_owned(), sheet.to_owned())) {
            warn!(target: "ambition_render::worn_gear", "the gear `{gear}` is not shown on `{sheet}`: {}", why());
        }
    }
}

/// Ask each wearer's rigged root for its anchor track, forget the asks of a
/// root that wears nothing there now, and draw each worn gear on the part
/// that answers. See the module docs.
#[allow(clippy::too_many_arguments)]
pub fn sync_worn_gear_visuals(
    mut commands: Commands,
    view: Res<WornGearView>,
    manifest: Option<Res<WornGearManifest>>,
    active_session: Option<Res<ActiveSessionScope>>,
    asset_server: Option<Res<AssetServer>>,
    assets: Option<Res<ambition_sprite_sheet::game_assets::GameAssets>>,
    roots: VisualRoots,
    bodies: Query<(Option<&CharacterAnimator>, Option<&PartSetDraws>)>,
    baked: Query<(&ambition_sprite_sheet::boss::BossAnimator, &GlobalTransform, &Sprite)>,
    asked: Query<(Entity, &PartSetRequest)>,
    mut visuals: Query<(Entity, &WornGearVisual, &mut Transform)>,
    mut warnings: Local<GearWarnings>,
) {
    let Some(session_scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref()) else {
        for (entity, ..) in &visuals {
            commands.entity(entity).despawn();
        }
        return;
    };
    // The asks each root needs this frame, by root and reader name.
    let mut wanted_asks: BTreeSet<(Entity, String)> = BTreeSet::new();
    // Each picture to draw, with where it goes; `true` once a visual has it.
    let mut wanted: Vec<(&ambition_sim_view::WornGearFact, GearLayer, &str, Transform, Anchor, Vec2, bool)> = Vec::new();
    for fact in &view.0 {
        let Some(gear) = manifest.as_deref().and_then(|manifest| manifest.get(&fact.gear)) else {
            continue;
        };
        let root = roots.of(fact.body, fact.feature.as_deref());
        let (animator, draws) = bodies.get(root).unwrap_or((None, None));
        let Some(animator) = animator else {
            if let Ok((boss, global, sprite)) = baked.get(root) {
                let sheet = boss.record.key.as_str();
                let Some(fit) = gear.fits.get(sheet) else {
                    warnings.once(&fact.gear, sheet, || "the gear has no fit for the sheet (`worn_gear.ron`, `fits`)".to_owned());
                    continue;
                };
                let placement = place_gear_baked(global.translation(), sprite.flip_x, fit);
                let anchor = socket_anchor(gear.socket);
                for (layer, path, transform) in [
                    (GearLayer::Back, gear.back.as_deref(), placement.back),
                    (GearLayer::Front, gear.front.as_deref(), placement.front),
                ] {
                    if let Some(path) = path {
                        wanted.push((fact, layer, path, transform, anchor, gear.size, false));
                    }
                }
                continue;
            }
            let who = fact.feature.clone().unwrap_or_else(|| format!("{:?}", fact.body));
            warnings.once(&fact.gear, &who, || "the body is drawn from a baked sheet, with no parts".to_owned());
            continue;
        };
        let sheet = animator.spec.base_sheet_key();
        let Some(fit) = gear.fits.get(sheet) else {
            warnings.once(&fact.gear, sheet, || "the gear has no fit for the sheet (`worn_gear.ron`, `fits`)".to_owned());
            continue;
        };
        let reader = format!("{READER}{}", fit.track);
        let part = draws.and_then(|draws| draws.parts(&reader).last().map(|part| (draws.place, part)));
        wanted_asks.insert((root, reader));
        let Some((place, part)) = part else {
            // No answer yet (the ask is new, or the pages load), or the
            // sheet has no part flipbook.
            let baked = assets.as_deref().is_some_and(|assets| {
                let (mut ready, mut rigged) = (false, false);
                for asset in assets.characters.ready_sheets().filter(|asset| asset.spec.base_sheet_key() == sheet) {
                    ready = true;
                    rigged |= asset.rigged.is_some();
                }
                ready && !rigged
            });
            if baked {
                warnings.once(&fact.gear, sheet, || "the sheet has no part flipbook (it is drawn baked)".to_owned());
            }
            continue;
        };
        let Some(placement) = place_gear(part, place, fit, animator.draws_mirror_row()) else {
            continue;
        };
        let anchor = socket_anchor(gear.socket);
        for (layer, path, transform) in [
            (GearLayer::Back, gear.back.as_deref(), placement.back),
            (GearLayer::Front, gear.front.as_deref(), placement.front),
        ] {
            if let Some(path) = path {
                wanted.push((fact, layer, path, transform, anchor, gear.size, false));
            }
        }
    }

    for &(root, ref reader) in &wanted_asks {
        if asked.get(root).is_ok_and(|(_, request)| request.tracks_of(reader).is_some()) {
            continue;
        }
        let track = reader.strip_prefix(READER).unwrap_or(reader);
        ask_for_parts(&mut commands, root, reader.clone(), &[track]);
    }
    for (root, request) in &asked {
        for reader in request.readers().filter(|reader| reader.starts_with(READER)) {
            if !wanted_asks.contains(&(root, reader.to_owned())) {
                forget_parts(&mut commands, root, reader.to_owned());
            }
        }
    }

    for (entity, drawn, mut transform) in &mut visuals {
        let kept = wanted.iter_mut().find(|(fact, layer, .., taken)| {
            !*taken && fact.body == drawn.body && fact.gear == drawn.gear && *layer == drawn.layer && fact.room == Some(drawn.room)
        });
        match kept {
            Some((.., wanted_transform, _, _, taken)) => {
                *taken = true;
                transform.set_if_neq(*wanted_transform);
            }
            None => commands.entity(entity).despawn(),
        }
    }
    let Some(asset_server) = asset_server else {
        return;
    };
    for (fact, layer, path, transform, anchor, size, taken) in wanted {
        let Some(room) = fact.room.filter(|_| !taken) else {
            continue;
        };
        // A picture that is not published yet loads as nothing: the asset
        // server reports the missing file, and the sprite draws nothing.
        let sprite = Sprite { image: asset_server.load(path.to_owned()), custom_size: Some(size), ..default() };
        commands.spawn_session_scoped(
            session_scope.in_room(Some(room)),
            (
                WornGearVisual { body: fact.body, gear: fact.gear.clone(), layer, room },
                sprite,
                anchor,
                transform,
                Name::new(format!("Worn gear {} ({layer:?})", fact.gear)),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fit() -> GearFit {
        GearFit { track: "head".into(), scale: 1.5, offset: Vec2::new(4.0, 6.0), turn_degrees: 10.0 }
    }

    fn part(at: Vec3, turn: f32) -> PlacedPart {
        PlacedPart {
            image: Handle::default(),
            rect: Rect::new(0.0, 0.0, 8.0, 8.0),
            size: Vec2::splat(8.0),
            anchor: Vec2::ZERO,
            local: Transform::from_translation(at).with_rotation(Quat::from_rotation_z(turn)),
        }
    }

    fn turn_of(transform: &Transform) -> f32 {
        transform.rotation.to_euler(EulerRot::XYZ).2
    }

    fn assert_near(a: Vec3, b: Vec3) {
        assert!(a.abs_diff_eq(b, 1e-4), "{a} is not {b}");
    }

    /// ⭐ A body facing right: the pictures are at the head's pivot (the
    /// place doubles the sheet's pixels into the world), moved by the fit's
    /// offset and turned by its turn, at the fit's scale and not the body's.
    /// The back is under the whole body; the front is just over the head.
    #[test]
    fn the_gear_is_on_the_pivot_of_the_anchor_part() {
        let place = Transform::from_xyz(100.0, 50.0, 5.0).with_scale(Vec3::new(2.0, 2.0, 1.0));
        let head = part(Vec3::new(10.0, 20.0, 3.0e-4), 0.0);
        let placed = place_gear(&head, place, &fit(), false).unwrap();
        assert_near(placed.front.translation, Vec3::new(124.0, 96.0, 5.0 + 3.0e-4 + GEAR_OVER_ANCHOR));
        assert_near(placed.back.translation, Vec3::new(124.0, 96.0, 5.0 - GEAR_UNDER_BODY));
        assert!((turn_of(&placed.front) - 10f32.to_radians()).abs() < 1e-5);
        assert_near(placed.front.scale, Vec3::new(1.5, 1.5, 1.0));
        assert!(placed.back.translation.z < place.translation.z, "the back is under every draw of the body");
    }

    /// A head turned by its row turns the gear with it, and the offset turns
    /// too: it is in the frame of the part.
    #[test]
    fn a_turned_head_turns_the_gear_and_its_offset() {
        let head = part(Vec3::ZERO, 90f32.to_radians());
        let placed = place_gear(&head, Transform::IDENTITY, &fit(), false).unwrap();
        assert_near(placed.front.translation.truncate().extend(0.0), Vec3::new(-6.0, 4.0, 0.0));
        assert!((turn_of(&placed.front) - 100f32.to_radians()).abs() < 1e-5);
    }

    /// ⭐ A body that faces left has the mirror image of the right-facing
    /// gear, whether its place mirrors it or it draws its sheet's mirror row:
    /// the offset is to the side it faces, the turn is reversed, and the
    /// picture is mirrored.
    #[test]
    fn a_body_that_faces_left_wears_the_mirror_image() {
        let head = part(Vec3::new(10.0, 20.0, 0.0), 30f32.to_radians());
        let right = place_gear(&head, Transform::from_xyz(100.0, 50.0, 0.0), &fit(), false).unwrap();
        let mirrored = place_gear(&head, Transform::from_xyz(100.0, 50.0, 0.0).with_scale(Vec3::new(-1.0, 1.0, 1.0)), &fit(), false).unwrap();
        // The mirror of x about the place's origin, with the same height.
        assert_near(
            mirrored.front.translation,
            Vec3::new(200.0 - right.front.translation.x, right.front.translation.y, right.front.translation.z),
        );
        assert!((turn_of(&mirrored.front) + turn_of(&right.front)).abs() < 1e-5, "the turn is reversed");
        assert!(mirrored.front.scale.x < 0.0 && right.front.scale.x > 0.0, "the picture is mirrored");

        // The mirror row: the art already faces left, at its own pivot and
        // turn. The gear is mirrored there, and its offset goes left.
        let left_row = part(Vec3::new(-10.0, 20.0, 0.0), -30f32.to_radians());
        let row = place_gear(&left_row, Transform::from_xyz(100.0, 50.0, 0.0), &fit(), true).unwrap();
        assert_near(row.front.translation, mirrored.front.translation);
        assert!((turn_of(&row.front) - turn_of(&mirrored.front)).abs() < 1e-5);
        assert!(row.front.scale.x < 0.0);
    }

    /// The socket is the point of the picture on the pivot: its top left is
    /// the anchor's top left.
    #[test]
    fn a_socket_from_the_top_left_is_a_sprite_anchor() {
        assert_eq!(socket_anchor(Vec2::new(0.5, 0.5)).0, Vec2::ZERO);
        assert_eq!(socket_anchor(Vec2::new(0.0, 0.0)).0, Vec2::new(-0.5, 0.5));
        assert_eq!(socket_anchor(Vec2::new(0.5, 0.75)).0, Vec2::new(0.0, -0.25));
    }
}
