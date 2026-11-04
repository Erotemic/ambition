//! Bind transition covers and loading roots to their affected local view.

use bevy::prelude::*;
use crate::game_shell::{FrontendEntityOwner, FrontendOwnedEntity, FrontendPresentationKind};
use crate::input::{InputOwner, ParticipantId};
use crate::platformer::lifecycle::{LiveBodies, LiveBodyId};
use crate::render::ui_surface::ParticipantSurface;
use crate::render::rendering::room_transition_cover::RoomTransitionCover;
use crate::runtime::room_transition::{ActiveRoomTransitionLoad, RoomTransitionLoadState};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TransitionSurface {
    Display,
    Participant(ParticipantId),
    View(Entity),
    Hidden,
}

#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct TransitionSurfaces<'w, 's> {
    surface: ParticipantSurface<'w, 's>,
    bodies: LiveBodies<'w, 's>,
}

impl TransitionSurfaces<'_, '_> {
    pub(super) fn target(&self, active: &ActiveRoomTransitionLoad) -> TransitionSurface {
        if let Some(participant) = active.intent.participant() {
            return TransitionSurface::Participant(ParticipantId(participant.0));
        }
        let Some(subject) = active.intent.subject() else { return TransitionSurface::Display; };
        let arrived = active.arrived_in.map(|room| LiveBodyId::new(subject.sim_id.clone(), Some(room)));
        self.bodies.entity_of(arrived.as_ref().unwrap_or(subject))
            .and_then(|body| self.surface.view_for_subject(body))
            .map_or(TransitionSurface::Hidden, TransitionSurface::View)
    }

    fn rect(&self, target: TransitionSurface) -> Option<Rect> {
        match target {
            TransitionSurface::Display => Some(self.surface.display_rect()),
            TransitionSurface::Participant(participant) => self.surface.view(InputOwner::Participant(participant))
                .and_then(|view| self.surface.gameplay_rect_for_view(view)),
            TransitionSurface::View(view) => self.surface.gameplay_rect_for_view(view),
            TransitionSurface::Hidden => None,
        }
    }
}

pub(super) fn place_transition_surfaces(
    mut commands: Commands,
    transitions: Res<RoomTransitionLoadState>,
    targets: TransitionSurfaces,
    mut roots: Query<(Entity, &mut Node, Option<&RoomTransitionCover>,
        Option<&FrontendOwnedEntity>, Option<&mut TransitionSurface>),
        Or<(With<RoomTransitionCover>, With<FrontendOwnedEntity>)>>,
) {
    for (entity, mut node, cover, frontend, mut stored) in &mut roots {
        let transaction = cover.filter(|cover| !cover.is_lifting()).and_then(|cover| transitions.presentation(cover.sequence))
            .or_else(|| frontend.filter(|frontend| matches!(frontend.kind,
                FrontendPresentationKind::LoadingRoot | FrontendPresentationKind::LoadingActivity)).and_then(|frontend|
                transitions.presentations().find(|active| matches!(&frontend.owner,
                    FrontendEntityOwner::Load(load) if load == &active.barrier.load_id))));
        let current = transaction.map(|active| targets.target(active)).or_else(|| stored.as_deref().copied());
        let Some(target) = current else { continue; };
        if let Some(stored) = stored.as_deref_mut() {
            if *stored != target { *stored = target; }
        } else {
            commands.entity(entity).insert(target);
        }
        let mut wanted = node.clone();
        if let Some(rect) = targets.rect(target) {
            targets.surface.place(&mut wanted, rect);
            wanted.display = Display::Flex;
        } else {
            wanted.display = Display::None;
        }
        if *node != wanted { *node = wanted; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim_view::{LocalView, LocalViewId, ViewParticipant, ViewPlacement};
    use crate::render::rendering::room_transition_cover::{spawn_room_transition_cover, RoomTransitionStyle};

    #[test]
    fn a_transition_surface_follows_its_view_and_hides_when_that_view_retires() {
        let mut app = App::new();
        app.init_resource::<RoomTransitionLoadState>();
        app.add_systems(Update, place_transition_surfaces);
        let views = [0u8, 1].map(|slot| app.world_mut().spawn((LocalView, LocalViewId(slot),
            ViewParticipant(crate::characters::control::PlayerSlot(slot)),
            ViewPlacement::column(usize::from(slot), 2))).id());
        let covers = [0u8, 1].map(|slot| {
            let root = { let mut commands = app.world_mut().commands();
                let root = spawn_room_transition_cover(&mut commands, u64::from(slot), RoomTransitionStyle::cut(Color::BLACK), 0.0);
                commands.entity(root).insert(TransitionSurface::Participant(ParticipantId(slot)));
                root
            };
            app.world_mut().flush();
            root
        });
        app.update();
        let first = app.world().get::<Node>(covers[0]).unwrap().clone();
        let second = app.world().get::<Node>(covers[1]).unwrap().clone();
        assert_eq!(first.left, Val::Px(0.0));
        assert_eq!(second.left, Val::Px(640.0));
        assert_eq!(first.width, Val::Px(640.0));
        assert_eq!(second.width, Val::Px(640.0));
        app.world_mut().entity_mut(views[1]).insert(ViewPlacement::FULL);
        app.update();
        assert_eq!(app.world().get::<Node>(covers[0]).unwrap(), &first);
        assert_eq!(app.world().get::<Node>(covers[1]).unwrap().width, Val::Px(1280.0));
        app.world_mut().entity_mut(views[1]).despawn();
        app.update();
        assert_eq!(app.world().get::<Node>(covers[0]).unwrap(), &first);
        assert_eq!(app.world().get::<Node>(covers[1]).unwrap().display, Display::None);
        app.world_mut().entity_mut(views[0]).despawn();
        app.update();
        for cover in covers {
            assert_eq!(app.world().get::<Node>(cover).unwrap().display, Display::None);
        }
    }
}
