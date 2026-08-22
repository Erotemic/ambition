//! Backend-neutral authored input delivery for simulation drivers.
//!
//! These helpers deliver directly to the generic control surfaces. A rollback backend may
//! provide a stronger wrapper that also understands its pending-input resources while
//! delegating to the same public SDK seam.

use bevy::prelude::World;

use ambition_platformer2d_core::ControlFrame;

/// Drive the primary participant's next control frame on a non-rollback host.
///
/// If a device bridge installed a latch, accumulate into it so sub-tick edges
/// survive until the simulation drains them. Otherwise write the authoritative
/// frame directly.
pub fn drive_control_frame(world: &mut World, frame: ControlFrame) {
    drive_slot_frame(
        world,
        ambition_characters::control::PlayerSlot::PRIMARY,
        frame,
    );
}

/// ANY seat's input, delivered to whichever surface this composition has.
///
/// this was TWO functions with the same shape, differing only in which resource each arm named
/// — and seat zero's latch has since become row zero of the same table every other seat uses.
///
/// [`drive_control_frame`] remains as the name for the primary seat, but it is a convenience
/// over this, not a second road with different rules.
pub fn drive_slot_frame(
    world: &mut World,
    slot: ambition_characters::control::PlayerSlot,
    frame: ControlFrame,
) {
    if let Some(mut latches) =
        world.get_resource_mut::<ambition_characters::control::SlotControlLatches>()
    {
        latches.accumulate(slot, frame);
        return;
    }
    // It is that seat's output mirror now, so writing it would deliver a press to nobody — the
    // silent no-op this whole seam exists to prevent.
    //
    // BOTH surfaces, and that is the helper's whole contract. A driver
    // says *this seat is holding this frame* and must not have to know how the
    // composition was assembled. The RAW row is what a shaping stage reads — a
    // scripted reset has to reach the reset stage, a scripted stick the portal
    // warp — and a composition that installs the shaping stages will overwrite
    // the slot below with the shaped result anyway. A composition that installs
    // NONE of them (the smallest headless fixture) has no commit either, so
    // without the second write its press would sit in a table nothing drains.
    if let Some(mut raw) = world.get_resource_mut::<ambition_characters::control::SeatRawFrames>() {
        raw.set(slot, frame);
    }
    if let Some(mut slots) = world.get_resource_mut::<ambition_characters::control::SlotControls>() {
        slots.set(slot, frame);
    }
}
