//! **Which participant is acting THROUGH this body — and what did they press?**
//!
//! The interaction systems each used to answer this twice and differently: they
//! took reach geometry from the controlled subject (right) and the buffered
//! press from slot 0 (wrong the moment a second seat exists), then applied the
//! resulting animation to whichever entity carried `PrimaryPlayer` (wrong under
//! possession, where the acting body is not the home avatar at all). The visible
//! version of that bug is **a possessed body opening a chest while the vacated
//! home avatar plays the reach-and-open pose.**
//!
//! ⭐ **it reads [`DrivingParticipant`], which is the fact by itself.** A seat
//! that possessed an actor and walked it up to a chest holds that body's
//! authority, which is why this needs to know nothing about possession to get
//! possession right — the same reason `DialogueDispatch::driving_slot` gives for
//! attributing a conversation.
//!
//! ⚠ **it used to read `Brain::Player(slot)` directly, and the answer has not
//! changed.** Possession MOVES that variant (one system asserts exactly one body
//! carries `Brain::Player(PRIMARY)`), so the brain and the derive agree today by
//! construction. What moved is the DEPENDENCY: this asks who is driving, and it
//! no longer has to know that the answer is currently spelled inside an AI-policy
//! enum. When possession stops swapping brains, nothing here changes.
//!
//! ⛔ **this is not a new participant model.** It is the one place the two
//! interaction systems agree on the question, so the primary-seat fallback below
//! is stated ONCE instead of being re-decided at four call sites.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::authority::DrivingParticipant;
use super::components::{SlotGestures, SlotInteractionState};
use ambition_characters::brain::PlayerSlot;

/// The controller gestures that answer for a given body.
#[derive(SystemParam)]
pub struct ActingParticipant<'w, 's> {
    gestures: ResMut<'w, SlotInteractionState>,
    drivers: Query<'w, 's, &'static DrivingParticipant>,
}

impl ActingParticipant<'_, '_> {
    /// **Which controller slot drives this body**, or `None` for a body no
    /// participant is driving (a CPU actor, a prop, a body whose brain has not
    /// been built yet).
    pub fn driving_slot(&self, body: Entity) -> Option<PlayerSlot> {
        self.drivers.get(body).ok().map(|driver| driver.0)
    }

    /// The slot whose gestures answer for this body.
    ///
    /// ⚠ **the fallback is the STARTUP frame, and it is stated here so the call
    /// sites do not each invent one.** The controlled subject resolves from the
    /// brain, so a subject without one is a world that has not finished being
    /// built; answering `PRIMARY` there preserves the behaviour every existing
    /// single-player fixture depends on. It is NOT a claim that a body with no
    /// participant may consume the primary seat's input during play — a body
    /// nobody drives never becomes the controlled subject in the first place.
    pub fn acting_slot(&self, body: Entity) -> PlayerSlot {
        self.driving_slot(body).unwrap_or(PlayerSlot::PRIMARY)
    }

    /// This body's driver's gesture state.
    pub fn gestures(&self, body: Entity) -> SlotGestures {
        self.gestures.get(self.acting_slot(body))
    }

    /// Is this body's driver holding a live buffered interact?
    pub fn buffered_interact(&self, body: Entity) -> bool {
        self.gestures(body).buffered()
    }

    /// **Spend this body's driver's buffered interact.**
    ///
    /// ⭐ an interaction consumes the press of the seat that MADE it, so a
    /// second participant standing at the same door still has theirs.
    pub fn consume_interact(&mut self, body: Entity) {
        let slot = self.acting_slot(body);
        if let Some(gestures) = self.gestures.get_mut(slot) {
            gestures.clear();
        }
    }
}
