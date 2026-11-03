//! Participant-owned UI regions resolved from the live view layout.

use ambition_platformer2d_shared_tangle::gameplay_presentation::ResolvedGameplayPresentation;
use ambition_sim_view::{
    LocalView, LocalViewId, SharedViewHudFacts, ViewHudSeat, ViewParticipant, ViewPlacement,
    ViewSubject, ResolvedViewSubject,
};
use bevy::prelude::*;

/// Resolve a UI surface from its participant and the current view layout.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ParticipantSurface<'w, 's> {
    views: Query<
        'w,
        's,
        (
            Entity,
            &'static LocalViewId,
            Option<&'static ViewParticipant>,
            Option<&'static ViewPlacement>,
            Option<&'static SharedViewHudFacts>,
            Option<&'static ViewHudSeat>,
            Option<&'static ViewSubject>,
            Option<&'static ResolvedViewSubject>,
        ),
        With<LocalView>,
    >,
    presentation: Option<Res<'w, ResolvedGameplayPresentation>>,
    windows: Query<'w, 's, &'static Window, With<bevy::window::PrimaryWindow>>,
}

impl ParticipantSurface<'_, '_> {
    pub fn display_rect(&self) -> Rect {
        self.presentation.as_deref()
            .map(|p| Rect::from_corners(p.display_rect.min, p.display_rect.max))
            .unwrap_or_else(|| Rect::from_corners(Vec2::ZERO,
                self.windows.single().map_or(Vec2::new(1280.0, 720.0),
                    |window| Vec2::new(window.width(), window.height()))))
    }

    /// The local view that presents this participant.
    pub fn view(&self, owner: ambition_input::InputOwner) -> Option<Entity> {
        let shared = self.views.iter().count() == 1;
        self.views.iter().filter(|(_, _, participant, _, seats, seat, subject, _)| {
            participant.is_some_and(|participant| owner.accepts(ambition_input::ParticipantId(participant.0.0)))
                || seat.and_then(|seat| seat.0).is_some_and(|seat| owner.accepts(ambition_input::ParticipantId(seat.0)))
                || seats.is_some_and(|seats| seats.0.iter().any(|(seat, _)| owner.accepts(ambition_input::ParticipantId(seat.0))))
                || (shared && participant.is_none() && subject.is_none() && seat.is_none())
        }).min_by_key(|(_, id, ..)| **id).map(|(entity, ..)| entity)
    }

    pub fn view_for_subject(&self, body: Entity) -> Option<Entity> {
        self.views.iter().filter(|(_, _, _, _, _, _, subject, resolved)|
            resolved.and_then(|resolved| resolved.0).or_else(|| subject.map(|subject| subject.0)) == Some(body))
            .min_by_key(|(_, id, ..)| **id).map(|(entity, ..)| entity)
    }

    fn gameplay_display(&self) -> Rect {
        self.presentation.as_deref().map_or_else(|| self.display_rect(),
            |p| Rect::from_corners(p.gameplay_rect.min, p.gameplay_rect.max))
    }

    /// The participant's part of the display, including gameplay margins.
    pub fn display_rect_for(&self, owner: Option<ambition_input::InputOwner>) -> Option<Rect> {
        let display = self.display_rect();
        if owner == Some(ambition_input::InputOwner::AllParticipants) || self.views.is_empty() {
            return Some(display);
        }
        self.rect_for_view(self.view(owner?)?, display)
    }

    fn rect_for_view(&self, view: Entity, display: Rect) -> Option<Rect> {
        let (_, _, _, placement, _, _, _, _) = self.views.get(view).ok()?;
        let (origin, size) = placement.copied().unwrap_or_default().carve(display.min, display.size());
        Some(Rect::from_corners(origin, origin + size))
    }

    /// The whole gameplay viewport, including space behind its HUD.
    pub fn gameplay_rect_for_view(&self, view: Entity) -> Option<Rect> {
        self.rect_for_view(view, self.gameplay_display())
    }

    pub fn gameplay_rect(&self, owner: Option<ambition_input::InputOwner>) -> Option<Rect> {
        if owner == Some(ambition_input::InputOwner::AllParticipants) || self.views.is_empty() {
            return Some(self.gameplay_display());
        }
        self.gameplay_rect_for_view(self.view(owner?)?)
    }

    pub fn rect(&self, owner: Option<ambition_input::InputOwner>) -> Option<Rect> {
        let reading = self.presentation.as_deref().map_or_else(|| self.display_rect(), |p| {
            let rect = p.reading_rect();
            Rect::from_corners(rect.min, rect.max)
        });
        if owner == Some(ambition_input::InputOwner::AllParticipants) || self.views.is_empty() {
            return Some(reading);
        }
        let gameplay = self.gameplay_rect(owner)?;
        let min = gameplay.min.max(reading.min);
        let max = gameplay.max.min(reading.max);
        max.cmpgt(min).all().then(|| Rect::from_corners(min, max))
    }

    pub fn place(&self, node: &mut Node, rect: Rect) {
        let display_origin = self
            .presentation
            .as_deref()
            .map_or(Vec2::ZERO, |p| p.display_rect.min);
        node.position_type = PositionType::Absolute;
        node.left = Val::Px(rect.min.x - display_origin.x);
        node.top = Val::Px(rect.min.y - display_origin.y);
        node.width = Val::Px(rect.width());
        node.height = Val::Px(rect.height());
        node.right = Val::Auto;
        node.bottom = Val::Auto;
        node.overflow = Overflow::clip();
    }
}
