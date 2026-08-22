//! **WHO IS DRIVING, and what they pressed.** The per-seat control vocabulary:
//! seat identity, the tables keyed by it, and the component that says which body
//! a seat drives.
//!
//! ⭐ **this lived in `brain` and did not belong there.** A `Brain` is an AI
//! backend and nothing else — its own doc says so, right after the module said
//! "control authority flows THROUGH the brain", which stopped being true when
//! `Brain::Player` was deleted. Seat tables kept landing beside each other on
//! the honest local reasoning that the neighbours were already there, and the
//! neighbours were all in the wrong module too (GPT review, 2026-08-22).
//!
//! The split is by QUESTION: this module answers *who is driving and what did
//! they press*; `brain` answers *what policy drives a body with nobody in it*.

use bevy::prelude::*;

/// Per-player slot identifier. Slot `0` is the local primary player;
/// future co-op / split-screen / network players will use slots
/// `1..=N`. Stored as a `u8` so it can fit comfortably in a HUD
/// label, a save key, or a debug overlay glyph.
///
/// `PlayerSlot` is the canonical "which player?" handle for new
/// player-bearing messages and resources. New player-domain message
/// types (heal, damage, respawn, cosmetic, …) SHOULD carry either an
/// `Entity` or a `PlayerSlot` so they don't silently assume the
/// primary player.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerSlot(pub u8);

impl PlayerSlot {
    /// Slot reserved for the local primary player in single-player
    /// builds and for player 1 in future local-multiplayer modes.
    pub const PRIMARY: PlayerSlot = PlayerSlot(0);

    pub fn index(self) -> u8 {
        self.0
    }
}

/// The canonical slot-based controller input model: `PlayerSlot -> ControlFrame`.
///
/// This is the SINGLE source of player control. The body that consumes slot
/// `S`'s frame is whichever entity carries [`DrivingParticipant`]`(S)` — the home
/// avatar, a possessed NPC, or any future controlled body. Local
/// keyboard/gamepad input populates [`PlayerSlot::PRIMARY`]; co-op /
/// split-screen / netcode will fill higher slots via their own adapters.
///
/// Control authority flows THROUGH the brain: nothing reads "the primary
/// player's input" to decide who acts. The universal-brain path looks up this
/// resource by the ticking brain's slot, so possession is just brain transfer —
/// no input-copy component, no possession-specific override.
#[derive(bevy::ecs::resource::Resource, Clone, Copy, Debug, Default)]
pub struct SlotControls {
    slots: [ambition_platformer2d_core::ControlFrame; Self::MAX_SLOTS],
}

impl SlotControls {
    /// Supported controller slots. Bumped when local multiplayer lands.
    pub const MAX_SLOTS: usize = 4;

    /// This slot's current controller frame (neutral for an unfilled slot).
    pub fn get(&self, slot: PlayerSlot) -> ambition_platformer2d_core::ControlFrame {
        self.slots.get(slot.0 as usize).copied().unwrap_or_default()
    }

    /// Publish a slot's controller frame. Out-of-range slots are ignored.
    pub fn set(&mut self, slot: PlayerSlot, frame: ambition_platformer2d_core::ControlFrame) {
        if let Some(entry) = self.slots.get_mut(slot.0 as usize) {
            *entry = frame;
        }
    }
}

/// Per-seat device frames before proposal-side shaping and input latching.
///
/// Device- or wall-clock-derived stages use this table before
/// `PrimarySlotInputCommit`. Confirmed-input stages run in the simulation schedule.
/// [`SlotControls`] is the post-latch simulation input; the two tables are distinct.
#[derive(bevy::ecs::resource::Resource, Clone, Copy, Debug, Default)]
pub struct SeatRawFrames {
    slots: [ambition_platformer2d_core::ControlFrame; SlotControls::MAX_SLOTS],
}

impl SeatRawFrames {
    /// This seat's raw frame as it currently stands (neutral for a seat nobody
    /// is driving).
    pub fn get(&self, slot: PlayerSlot) -> ambition_platformer2d_core::ControlFrame {
        self.slots.get(slot.0 as usize).copied().unwrap_or_default()
    }

    /// Replace this seat's raw frame. Out-of-range slots are ignored.
    pub fn set(&mut self, slot: PlayerSlot, frame: ambition_platformer2d_core::ControlFrame) {
        if let Some(entry) = self.slots.get_mut(slot.0 as usize) {
            *entry = frame;
        }
    }

    /// Shape this seat's frame in place — the form every shaping stage wants,
    /// since each one adjusts a field rather than replacing the frame.
    pub fn shape(
        &mut self,
        slot: PlayerSlot,
        edit: impl FnOnce(&mut ambition_platformer2d_core::ControlFrame),
    ) {
        if let Some(entry) = self.slots.get_mut(slot.0 as usize) {
            edit(entry);
        }
    }

    /// Every seat's row, in slot order — what a shaping stage that genuinely
    /// applies to all of them iterates.
    pub fn seats(
        &self,
    ) -> impl Iterator<Item = (PlayerSlot, ambition_platformer2d_core::ControlFrame)> + '_ {
        self.slots
            .iter()
            .enumerate()
            .map(|(index, frame)| (PlayerSlot(index as u8), *frame))
    }
}

// Controller-slot gesture state.

/// One controller slot's double-tap timers and interact buffer.
///
/// Controller-local state; possessed bodies read the gesture state of their driving slot.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SlotGestures {
    /// Non-zero means a down-tap is live and a second one would be a double-tap.
    pub down_tap_timer: f32,
    /// Non-zero means an up-tap is live and a second one would be a double-tap.
    pub up_tap_timer: f32,
    /// Keeps `interact` live across frames, so the button need not be held for
    /// the whole activation animation.
    pub interact_buffer_timer: f32,
    /// A double-tap-down edge, awaiting its consumer.
    pub double_tap_down_pending: bool,
    /// A double-tap-up edge, awaiting its consumer.
    pub double_tap_up_pending: bool,
    /// How long Up has been held without interruption. Crossing the hold
    /// threshold is an alternative way to interact; releasing resets it.
    pub up_hold_timer: f32,
}

impl SlotGestures {
    /// Advance timers and detect a double-tap-down edge. Returns `true` when
    /// two taps arrive within `window` seconds.
    pub fn register_down_tap(&mut self, down_pressed: bool, frame_dt: f32, window: f32) -> bool {
        self.down_tap_timer = (self.down_tap_timer - frame_dt).max(0.0);
        if !down_pressed {
            return false;
        }
        if self.down_tap_timer > 0.0 {
            self.down_tap_timer = 0.0;
            true
        } else {
            self.down_tap_timer = window;
            false
        }
    }

    /// Advance timers and detect a double-tap-up edge. Returns `true` when
    /// two taps arrive within `window` seconds.
    pub fn register_up_tap(&mut self, up_pressed: bool, frame_dt: f32, window: f32) -> bool {
        self.up_tap_timer = (self.up_tap_timer - frame_dt).max(0.0);
        if !up_pressed {
            return false;
        }
        if self.up_tap_timer > 0.0 {
            self.up_tap_timer = 0.0;
            true
        } else {
            self.up_tap_timer = window;
            false
        }
    }

    /// Advance the Up hold and return `true` on the single frame it crosses
    /// `threshold` — an EDGE, so a player who keeps holding Up interacts once
    /// rather than every frame after the second.
    pub fn held_up_interact(&mut self, up_held: bool, frame_dt: f32, threshold: f32) -> bool {
        if !up_held {
            self.up_hold_timer = 0.0;
            return false;
        }
        let before = self.up_hold_timer;
        self.up_hold_timer += frame_dt;
        before < threshold && self.up_hold_timer >= threshold
    }

    /// Update the interact buffer and return whether the buffer is live.
    pub fn buffered_interact(&mut self, pressed: bool, frame_dt: f32, window: f32) -> bool {
        self.interact_buffer_timer = (self.interact_buffer_timer - frame_dt).max(0.0);
        if pressed {
            self.interact_buffer_timer = window;
        }
        self.interact_buffer_timer > 0.0
    }

    pub fn buffered(self) -> bool {
        self.interact_buffer_timer > 0.0
    }

    pub fn clear(&mut self) {
        self.interact_buffer_timer = 0.0;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Slot-keyed gestures — the authority for "which controller wants to interact,
/// morph or double-tap". Input publishes into a slot; consumers read the slot of
/// the body they act on.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct SlotInteractionState {
    slots: [SlotGestures; SlotControls::MAX_SLOTS],
}

impl SlotInteractionState {
    /// This slot's gestures (default for an out-of-range slot).
    pub fn get(&self, slot: PlayerSlot) -> SlotGestures {
        self.slots.get(slot.0 as usize).copied().unwrap_or_default()
    }

    /// Mutable access to a slot's gestures. Invalid slots return `None` rather
    /// than redirecting the write to another participant.
    pub fn get_mut(&mut self, slot: PlayerSlot) -> Option<&mut SlotGestures> {
        self.slots.get_mut(slot.0 as usize)
    }

    /// The local primary controller's gestures — the single-player default.
    pub fn primary(&self) -> SlotGestures {
        self.get(PlayerSlot::PRIMARY)
    }

    /// Mutable primary-controller gestures. The const assertion below guarantees
    /// that the primary slot is in bounds.
    pub fn primary_mut(&mut self) -> &mut SlotGestures {
        &mut self.slots[PlayerSlot::PRIMARY.0 as usize]
    }
}

/// Compile-time guard for `primary_mut`'s unconditional index.
const _: () = assert!((PlayerSlot::PRIMARY.0 as usize) < SlotControls::MAX_SLOTS);

/// Frame-to-tick input latches keyed by participant slot.
///
/// Device samples accumulate on the frame clock and are drained on simulation ticks,
/// preserving edges that begin and end between ticks.
#[derive(bevy::ecs::resource::Resource, Clone, Copy, Debug, Default)]
pub struct SlotControlLatches {
    slots: [ambition_platformer2d_core::ControlFrameLatch; SlotControls::MAX_SLOTS],
}

impl SlotControlLatches {
    /// **Does a device feed `slot` at all**, so its latch speaks for the frame?
    ///
    /// A consumer that would OVERWRITE another writer's frame must ask first: an
    /// untouched latch means *no device is wired to this seat*, not *the device
    /// said nothing*. Sticky by design — see `ControlFrameLatch::device_seen`.
    pub fn is_device_authority(&self, slot: PlayerSlot) -> bool {
        self.slots
            .get(slot.0 as usize)
            .is_some_and(ambition_platformer2d_core::ControlFrameLatch::is_device_authority)
    }

    /// Fold one device sample for `slot`. Levels overwrite; edges stick.
    pub fn accumulate(
        &mut self,
        slot: PlayerSlot,
        sample: ambition_platformer2d_core::ControlFrame,
    ) {
        if let Some(latch) = self.slots.get_mut(slot.0 as usize) {
            latch.accumulate(sample);
        }
    }

    /// Hand `slot`'s accumulated frame to a tick, retaining levels.
    pub fn take(&mut self, slot: PlayerSlot) -> ambition_platformer2d_core::ControlFrame {
        self.slots
            .get_mut(slot.0 as usize)
            .map(ambition_platformer2d_core::ControlFrameLatch::take)
            .unwrap_or_default()
    }

    /// Clear a slot outright — a pause, a lost context, a seat going neutral.
    ///
    /// Distinct from `take`, which RETAINS levels: a seat that has stopped being
    /// driven must not keep reporting a held direction, and the post-pause
    /// re-press has to start from a clean Released state (the rule the primary
    /// seat already follows).
    pub fn reset(&mut self, slot: PlayerSlot) {
        if let Some(latch) = self.slots.get_mut(slot.0 as usize) {
            *latch = ambition_platformer2d_core::ControlFrameLatch::default();
        }
    }

    /// What `slot`'s next tick would take. Test/debug only.
    pub fn peek(&self, slot: PlayerSlot) -> ambition_platformer2d_core::ControlFrame {
        self.slots
            .get(slot.0 as usize)
            .map(ambition_platformer2d_core::ControlFrameLatch::peek)
            .unwrap_or_default()
    }
}

/// **The participant slot driving this body**, this tick.
///
/// ⭐⭐ **this WAS the `slot` inside `Brain::Player(slot)`, and it is not a
/// brain.** *"A participant drives this body"* is not an AI backend; it sat in
/// that enum because the enum was the only place to say it, which is why
/// possession used to MOVE a policy variant in order to change who is driving,
/// and why `PossessionState` needed a `restore_brain` to put the displaced
/// policy back. `Brain` is AI policy only now, and this is the driver.
///
/// ⭐ **AUTHORED at the spawn/seat site, RECONCILED by exactly one system.** A
/// body that a participant drives is spawned wearing this; the one runtime writer
/// is `control::project_driving_participant`, which moves the PRIMARY seat onto a
/// possessed body and back. Nothing else inserts or removes it, because two
/// writable answers to *who drives this body* is the defect this type exists to
/// end.
///
/// ⛔ **it is REGISTERED rollback state, not a derive.** It was declared derived
/// while it was reprojected from `Brain::Player`, which IS in the snapshot; with
/// the variant gone there is no upstream to reproject from — the seat assignment
/// lives here and nowhere else, so a rewind that did not carry it would restore a
/// body nobody drives.
///
/// The TYPE lives here because it is vocabulary — `PlayerSlot` is here, `Brain`
/// is here, and the two crates that ask *who drives this body* (the interaction
/// seam and the conversation seam) can both already see this module and neither
/// can see the other.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrivingParticipant(pub PlayerSlot);
