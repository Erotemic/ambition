//! Backend-neutral authored input delivery for simulation drivers.
//!
//! A fixed-tick/headless driver should not need to know whether an input latch
//! exists. These helpers deliver directly to the generic control surfaces. A
//! rollback backend may provide a stronger wrapper that also understands its
//! pending-input resources while delegating to the same public SDK seam.

use bevy::prelude::World;

use ambition_platformer2d_core::ControlFrame;

/// Drive the primary participant's next control frame on a non-rollback host.
///
/// If a device bridge installed a latch, accumulate into it so sub-tick edges
/// survive until the simulation drains them. Otherwise write the authoritative
/// frame directly.
pub fn drive_control_frame(world: &mut World, frame: ControlFrame) {
    drive_one_seat(
        world,
        ambition_characters::brain::PlayerSlot::PRIMARY,
        frame,
    );
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
    drive_one_seat(world, slot, frame);
}

/// **ONE seat's input, delivered to whichever surface this composition has.**
///
/// ⭐ this was TWO functions with the same shape, differing only in which
/// resource each arm named — and seat zero's latch has since become row zero of
/// the same table every other seat uses. What is left of the fork is the last
/// arm, where seat zero's input IS the global `ControlFrame` because the shapers
/// only it has read that resource (D175).
fn drive_one_seat(
    world: &mut World,
    slot: ambition_characters::brain::PlayerSlot,
    frame: ControlFrame,
) {
    if let Some(mut latches) =
        world.get_resource_mut::<ambition_characters::brain::SlotControlLatches>()
    {
        latches.accumulate(slot, frame);
        return;
    }
    if slot.0 == 0 {
        if let Some(mut control) = world.get_resource_mut::<ControlFrame>() {
            *control = frame;
        }
    } else if let Some(mut slots) =
        world.get_resource_mut::<ambition_characters::brain::SlotControls>()
    {
        slots.set(slot, frame);
    }
}
