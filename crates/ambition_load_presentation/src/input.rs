//! Read loading controls from the participants that own the foreground.

use ambition_input::{InputOwner, LOADING_CONTEXT, MenuControlFrame, MenuInputFrame,
    ParticipantId, SeatInputContexts, SeatMenuFrames};
use bevy::prelude::*;

#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct LoadInput<'w> {
    global: Option<Res<'w, MenuControlFrame>>,
    seats: Option<Res<'w, SeatMenuFrames>>,
    contexts: Option<Res<'w, SeatInputContexts>>,
}

impl LoadInput<'_> {
    pub(super) fn read(&self, owner: Option<InputOwner>) -> MenuInputFrame {
        let mut out = MenuInputFrame::default();
        let mut add = |slot, frame: &MenuControlFrame| {
            if !owner.is_some_and(|owner| owner.accepts(ParticipantId(slot)))
                || self.contexts.as_deref().is_some_and(|contexts| !contexts.for_seat(slot).allows(LOADING_CONTEXT)) {
                return;
            }
            out.up |= frame.up;
            out.down |= frame.down;
            out.left |= frame.left;
            out.right |= frame.right;
            out.select |= frame.select;
            out.back |= frame.back;
            out.start |= frame.start;
        };
        if let Some(frame) = self.global.as_deref() { add(0, frame); }
        if let Some(frames) = self.seats.as_deref() {
            for (slot, frame) in frames.seats() { add(slot, &frame); }
        }
        out
    }
}
