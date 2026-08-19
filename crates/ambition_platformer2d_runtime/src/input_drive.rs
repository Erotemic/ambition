//! Backend-neutral authored input delivery for simulation drivers.
//!
//! A fixed-tick/headless driver should not need to know whether an input latch
//! exists. These helpers deliver directly to the generic control surfaces. A
//! rollback backend may provide a stronger wrapper that also understands its
//! pending-input resources while delegating to the same public SDK seam.

use bevy::prelude::World;

use ambition_platformer2d_core::{ControlFrame, ControlFrameLatch};

/// Drive the primary participant's next control frame on a non-rollback host.
///
/// If a device bridge installed a latch, accumulate into it so sub-tick edges
/// survive until the simulation drains them. Otherwise write the authoritative
/// frame directly.
pub fn drive_control_frame(world: &mut World, frame: ControlFrame) {
    if let Some(mut latch) = world.get_resource_mut::<ControlFrameLatch>() {
        latch.accumulate(frame);
        return;
    }
    if let Some(mut control) = world.get_resource_mut::<ControlFrame>() {
        *control = frame;
    }
}

/// Drive a named secondary participant's next control frame on a non-rollback
/// host. Slot zero belongs to [`drive_control_frame`] and is refused here.
pub fn drive_seat_frame(
    world: &mut World,
    slot: ambition_characters::brain::PlayerSlot,
    frame: ControlFrame,
) {
    if slot.0 == 0 {
        return;
    }
    if let Some(mut latches) =
        world.get_resource_mut::<ambition_characters::brain::SlotControlLatches>()
    {
        latches.accumulate(slot, frame);
        return;
    }
    if let Some(mut slots) = world.get_resource_mut::<ambition_characters::brain::SlotControls>() {
        slots.set(slot, frame);
    }
}
