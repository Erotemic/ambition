//! The encounter as a first-class ENTITY.
//!
//! E1 makes the live encounter a Bevy entity rather than a value in a
//! resource-owned map: [`Encounter`] is its stable identity and the entity
//! carries the live [`EncounterState`](crate::EncounterState) component.
//!
//! [`EncounterView`] is the one cross-crate PRESENTATION read-model (§6): the
//! host publishes it each tick from the live encounter entities so presentation
//! adapters in other crates (the camera) read a stable resource instead of
//! reaching into the entity representation.

use bevy::prelude::*;

/// Stable identity of a live encounter entity — matches the authored id (the
/// LDtk `EncounterTrigger.id` for waves; the boss placement id for a boss
/// fight).
#[derive(Component, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Encounter {
    pub id: String,
}

impl Encounter {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

/// The one encounter PRESENTATION read-model (§6, started minimal at E1).
///
/// Cross-crate presentation adapters (the camera today) must not query the
/// encounter entities directly — the host publishes the derived presentation
/// intent here each tick, so those adapters stay decoupled from the encounter
/// state representation. Grows (music already has its own stream) as later
/// slices route HUD/camera/lock intent through the read model.
///
/// ⭐ THE ZOOM IS KEPT PER LIVE ROOM (customer 2). An encounter zooms the
/// views that frame its own room: a wave in Bob's room does not zoom Alice's
/// view of the hub.
#[derive(Resource, Clone, Debug, Default)]
pub struct EncounterView {
    /// Camera zoom the active encounters of each live room want this frame,
    /// for the rooms that want one (`None`: a composition with no live room).
    zooms: Vec<(Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>, f32)>,
}

impl EncounterView {
    /// The camera zoom the encounters of `room` want (`1.0` = no zoom).
    pub fn camera_zoom_in(
        &self,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    ) -> f32 {
        self.zooms
            .iter()
            .find(|(zoomed, _)| *zoomed == room)
            .map_or(1.0, |(_, zoom)| *zoom)
    }

    /// Publish this frame's zoom of every live room: the max authored zoom
    /// of each room's in-flight encounters. `max`, so the result does not
    /// depend on query order.
    pub fn set_camera_zooms(
        &mut self,
        states: impl IntoIterator<
            Item = (
                Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
                crate::EncounterPhase,
                f32,
            ),
        >,
    ) {
        let mut by_room: std::collections::BTreeMap<_, Vec<(crate::EncounterPhase, f32)>> =
            std::collections::BTreeMap::new();
        for (room, phase, zoom) in states {
            by_room.entry(room).or_default().push((phase, zoom));
        }
        self.zooms = by_room
            .into_iter()
            .map(|(room, states)| (room, crate::active_encounter_camera_zoom(states)))
            .filter(|(_, zoom)| *zoom != 1.0)
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;

    /// An encounter zooms only the views of its own live room. The first
    /// room has an active encounter that wants 1.2; the second has one that
    /// is not in flight (1.5). The first room zooms (the control), the
    /// second does not, and a room with no encounter does not. Poison: one
    /// zoom for every room, and the second room zooms 1.2.
    #[test]
    fn an_encounter_zooms_only_the_views_of_its_own_live_room() {
        let first = Some(LiveRoomInstance::ACTIVATION.next());
        let second = first.map(LiveRoomInstance::next);
        let mut view = EncounterView::default();
        view.set_camera_zooms([
            (first, crate::EncounterPhase::Active, 1.2),
            (second, crate::EncounterPhase::Inactive, 1.5),
        ]);
        assert_eq!(
            (view.camera_zoom_in(first), view.camera_zoom_in(second), view.camera_zoom_in(None)),
            (1.2, 1.0, 1.0),
            "(the room with an active encounter, the room with none in flight, no room)"
        );
    }
}
