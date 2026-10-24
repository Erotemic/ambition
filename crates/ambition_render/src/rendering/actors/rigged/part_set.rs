//! Where a named set of a body's parts is drawn, for a reader that draws
//! about a part of a body and not about the whole of it.
//!
//! A body read as one image is composited (`ComposedBodyDemand`), and that
//! image has no parts: a reader of it cannot tell a wing from an arm. A
//! reader that needs the difference (a barrier about a body and not about
//! its wings) asks for its tracks on the root under its own name
//! ([`ask_for_parts`]). Each frame the body is drawn from parts, the driver
//! writes [`PartSetDraws`] on the same root: the draws of the tracks of each
//! reader, and the place of the body's feet in the world. The reader draws
//! what it wants with them. Nothing here draws.
//!
//! ⛔ EACH READER HAS ITS OWN ASK. The request was one list of tracks with
//! one answer, and a reader took it away with both components: a second
//! reader on the same body lost its ask and its answer when the first
//! stopped. A reader now forgets only its own ask ([`forget_parts`]), and
//! the components go with the last ask.

use std::collections::BTreeMap;

use bevy::prelude::*;

use ambition_sprite_sheet::character::rigged::PartDraw;

use super::{RiggedPresentation, SLOT_DEPTH_STEP};

/// The asks of the readers of a rigged root: for each reader, by its name,
/// the draws of these tracks, each frame.
///
/// A track is named as the body's flipbook names it
/// (`RiggedSpriteAsset::tracks`): `torso`, `overlay:hair`. A name the
/// flipbook does not have matches nothing.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
#[require(PartSetDraws)]
pub struct PartSetRequest {
    asks: BTreeMap<String, Vec<String>>,
}

impl PartSetRequest {
    /// The tracks `reader` asks for, if it asks.
    pub fn tracks_of(&self, reader: &str) -> Option<&[String]> {
        self.asks.get(reader).map(Vec::as_slice)
    }

    /// The names of the readers that ask.
    pub fn readers(&self) -> impl Iterator<Item = &str> {
        self.asks.keys().map(String::as_str)
    }
}

/// Ask for the draws of `tracks` on `root`, as `reader`. The asks of other
/// readers stay. Idempotent.
pub fn ask_for_parts(commands: &mut Commands, root: Entity, reader: impl Into<String>, tracks: &[&str]) {
    let reader = reader.into();
    let tracks: Vec<String> = tracks.iter().map(|track| (*track).to_owned()).collect();
    commands.entity(root).queue_silenced(move |mut root: EntityWorldMut| {
        let mut request = root.take::<PartSetRequest>().unwrap_or_default();
        request.asks.insert(reader, tracks);
        root.insert(request);
    });
}

/// Forget the ask of `reader` on `root`, and only its ask. The components
/// go with the last ask.
pub fn forget_parts(commands: &mut Commands, root: Entity, reader: impl Into<String>) {
    let reader = reader.into();
    commands.entity(root).queue_silenced(move |mut root: EntityWorldMut| {
        let Some(mut request) = root.get_mut::<PartSetRequest>() else {
            return;
        };
        if request.asks.remove(&reader).is_none() {
            return;
        }
        if request.asks.is_empty() {
            root.remove::<(PartSetRequest, PartSetDraws)>();
        } else if let Some(mut draws) = root.get_mut::<PartSetDraws>() {
            draws.answers.remove(&reader);
        }
    });
}

/// The answer to a [`PartSetRequest`], written by the rigged-sprite driver.
///
/// Empty on a frame the body is not drawn from parts (a baked row of a
/// hybrid, a sheet with no flipbook, a tier that is still loading).
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct PartSetDraws {
    /// The body's feet in the world. Under it one unit is one sheet pixel and
    /// +y is up; it carries the body's size, its mirror and its turn. It is
    /// the same on both roads (parts in the world, and a composited cell).
    pub place: Transform,
    /// The draws of the tracks of each reader, in paint order.
    answers: BTreeMap<String, Vec<PlacedPart>>,
}

impl PartSetDraws {
    /// The draws of the tracks `reader` asks for, in paint order. Empty on a
    /// frame the body is not drawn from parts, and for a reader that does
    /// not ask.
    pub fn parts(&self, reader: &str) -> &[PlacedPart] {
        self.answers.get(reader).map_or(&[], Vec::as_slice)
    }

    /// An answer as the driver writes it, for a reader's own tests.
    pub fn from_parts(place: Transform, reader: &str, parts: Vec<PlacedPart>) -> Self {
        Self {
            place,
            answers: BTreeMap::from([(reader.to_owned(), parts)]),
        }
    }
}

/// One part of a [`PartSetDraws`].
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedPart {
    /// The page the part is on.
    pub image: Handle<Image>,
    /// The part on its page, in texels of the page.
    pub rect: Rect,
    /// The part's size, in sheet pixels (a quality tier keeps it).
    pub size: Vec2,
    /// The part's pivot as a sprite anchor: `(-0.5, -0.5)` is the bottom left.
    pub anchor: Vec2,
    /// The pivot under [`PartSetDraws::place`]: where, turned and scaled as
    /// the draw is. Its z is the depth of the draw among the body's draws.
    pub local: Transform,
}

/// Write the answer for one root: for each reader, the draws of `draws` on
/// its tracks. One pass over the draws for all readers.
pub(super) fn answer(
    request: &PartSetRequest,
    answer: &mut Mut<PartSetDraws>,
    presentation: &RiggedPresentation,
    draws: &[PartDraw],
    place: Transform,
) {
    let flipbook = &presentation.pages.flipbook;
    let mut answers: BTreeMap<String, Vec<PlacedPart>> =
        request.asks.keys().map(|reader| (reader.clone(), Vec::new())).collect();
    for (index, draw) in draws.iter().enumerate() {
        let Some(name) = draw.track.and_then(|track| flipbook.tracks.get(usize::from(track))) else {
            continue;
        };
        let readers: Vec<&String> =
            request.asks.iter().filter(|(_, tracks)| tracks.contains(name)).map(|(reader, _)| reader).collect();
        if readers.is_empty() {
            continue;
        }
        let part = flipbook.parts[usize::from(draw.part)];
        let Some(image) = presentation.pages.pages.get(usize::from(part.page)) else {
            continue;
        };
        let placed = PlacedPart {
            image: image.clone(),
            rect: Rect::new(
                part.rect.min.x as f32,
                part.rect.min.y as f32,
                part.rect.max.x as f32,
                part.rect.max.y as f32,
            ),
            size: part.size,
            anchor: part.anchor(),
            // As a slot is placed (`write_slots`).
            local: Transform::from_translation(Vec3::new(draw.at.x, -draw.at.y, index as f32 * SLOT_DEPTH_STEP))
                .with_rotation(Quat::from_rotation_z(-draw.rotation))
                .with_scale(draw.scale.extend(1.0)),
        };
        for reader in readers {
            answers.get_mut(reader).expect("each reader has its list").push(placed.clone());
        }
    }
    let next = PartSetDraws { place, answers };
    if **answer != next {
        **answer = next;
    }
}

/// Nothing of the body is drawn from parts this frame.
pub(super) fn clear(answer: &mut Mut<PartSetDraws>) {
    if answer.answers.values().any(|parts| !parts.is_empty()) {
        for parts in answer.answers.values_mut() {
            parts.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    /// ⭐ A6: two readers of one body, each with its own ask. When one
    /// forgets its ask, the other keeps its ask and its answer; the
    /// components go with the last ask.
    #[test]
    fn a_reader_forgets_its_own_ask_and_no_other() {
        let mut world = World::new();
        let body = world.spawn_empty().id();
        world
            .run_system_once(move |mut commands: Commands| {
                ask_for_parts(&mut commands, body, "barrier", &["overlay:body"]);
                ask_for_parts(&mut commands, body, "trail", &["hands", "weapon"]);
            })
            .unwrap();
        let request = world.get::<PartSetRequest>(body).expect("the asks");
        assert_eq!(request.tracks_of("barrier"), Some(&["overlay:body".to_owned()][..]));
        assert_eq!(request.tracks_of("trail"), Some(&["hands".to_owned(), "weapon".to_owned()][..]));
        // The driver has answered both.
        let part = PlacedPart {
            image: Handle::default(),
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            size: Vec2::ONE,
            anchor: Vec2::ZERO,
            local: Transform::IDENTITY,
        };
        world.get_mut::<PartSetDraws>(body).unwrap().answers =
            BTreeMap::from([("barrier".to_owned(), vec![part.clone()]), ("trail".to_owned(), vec![part.clone()])]);

        world.run_system_once(move |mut commands: Commands| forget_parts(&mut commands, body, "barrier")).unwrap();
        let request = world.get::<PartSetRequest>(body).expect("the other reader still asks");
        assert_eq!(request.tracks_of("barrier"), None);
        assert!(request.tracks_of("trail").is_some());
        let draws = world.get::<PartSetDraws>(body).expect("and has its answer");
        assert_eq!(draws.parts("trail"), &[part][..]);
        assert!(draws.parts("barrier").is_empty());

        world.run_system_once(move |mut commands: Commands| forget_parts(&mut commands, body, "trail")).unwrap();
        assert!(world.get::<PartSetRequest>(body).is_none() && world.get::<PartSetDraws>(body).is_none());
    }
}
