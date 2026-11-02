//! Participant-owned UI regions resolved from the live view layout.

use ambition_platformer2d_shared_tangle::gameplay_presentation::ResolvedGameplayPresentation;
use ambition_sim_view::{
    LocalView, LocalViewId, SharedViewHudFacts, ViewHudSeat, ViewParticipant, ViewPlacement,
    ViewSubject,
};
use bevy::prelude::*;

/// Resolve a UI surface from its participant and the current view layout.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ParticipantSurface<'w, 's> {
    views: Query<
        'w,
        's,
        (
            &'static LocalViewId,
            Option<&'static ViewParticipant>,
            Option<&'static ViewPlacement>,
            Option<&'static SharedViewHudFacts>,
            Option<&'static ViewHudSeat>,
            Has<ViewSubject>,
        ),
        With<LocalView>,
    >,
    presentation: Option<Res<'w, ResolvedGameplayPresentation>>,
    windows: Query<'w, 's, &'static Window, With<bevy::window::PrimaryWindow>>,
}

impl ParticipantSurface<'_, '_> {
    pub fn rect(&self, owner: Option<ambition_input::InputOwner>) -> Option<Rect> {
        use ambition_input::InputOwner;
        let display = self
            .presentation
            .as_deref()
            .map(|p| Rect::from_corners(p.display_rect.min, p.display_rect.max))
            .unwrap_or_else(|| {
                Rect::from_corners(
                    Vec2::ZERO,
                    self.windows
                        .single()
                        .map_or(Vec2::new(1280.0, 720.0), |window| {
                            Vec2::new(window.width(), window.height())
                        }),
                )
            });
        let reading = self.presentation.as_deref().map_or(display, |p| {
            let rect = p.reading_rect();
            Rect::from_corners(rect.min, rect.max)
        });
        if owner == Some(InputOwner::AllParticipants) || self.views.is_empty() {
            return Some(reading);
        }
        let owner = owner?;
        let shared = self.views.iter().count() == 1;
        let (_, _, placement, _, _, _) =
            self.views
                .iter()
                .filter(|(_, participant, _, seats, seat, subject)| {
                    participant.is_some_and(|participant| {
                        owner.accepts(ambition_input::ParticipantId(participant.0 .0))
                    }) || seat
                        .and_then(|seat| seat.0)
                        .is_some_and(|seat| owner.accepts(ambition_input::ParticipantId(seat.0)))
                        || seats.is_some_and(|seats| {
                            seats.0.iter().any(|(seat, _)| {
                                owner.accepts(ambition_input::ParticipantId(seat.0))
                            })
                        })
                        || (shared && participant.is_none() && !subject && seat.is_none())
                })
                .min_by_key(|(id, ..)| **id)?;
        let gameplay = self.presentation.as_deref().map_or(display, |p| {
            Rect::from_corners(p.gameplay_rect.min, p.gameplay_rect.max)
        });
        let (origin, size) = placement
            .copied()
            .unwrap_or_default()
            .carve(gameplay.min, gameplay.size());
        let min = origin.max(reading.min);
        let max = (origin + size).min(reading.max);
        (max.cmpgt(min).all()).then(|| Rect::from_corners(min, max))
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
