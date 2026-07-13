//! Ambition portal → audio adapter.
//!
//! Per the ownership boundary the portal crate owns neither audio nor the sfx
//! vocabulary, so it does NOT write `SfxMessage`. Instead it emits portal-owned
//! audio SIGNALS — `PortalShotFired` (fire), `PortalBodyEntered` (aperture
//! entry), and `PortalBodyTransited` (the crossing, carrying the exit position) —
//! and this Ambition adapter maps each to the concrete portal sfx cues. The shot
//! placement / close / fizzle cues stay in `shot_adapter` (they already lived in
//! the Ambition world-seam adapter, which owns `RoomGeometry`).
//!
//! Moving these here (Stage 19 Phase 5a) removes the last `ambition_sfx`
//! reference from portal core (`portal_transit` / `portal_fire_system`).

use bevy::prelude::*;

use ambition_portal::{PortalBodyEntered, PortalBodyTransited, PortalShotFired};

/// Play the portal audio cues from the portal-owned signals:
///
/// - [`PortalShotFired`] → the punchy fire blast + the airy travel whizz (at the
///   shot's origin), exactly what `portal_fire_system` used to emit inline.
/// - [`PortalBodyEntered`] → the ENTER cue (at the entry portal), exactly what
///   `portal_transit`'s `Begin` branch used to emit.
/// - [`PortalBodyTransited`] → the EXIT cue (at the exit-side centroid
///   `exit_pos`), exactly what `portal_transit`'s `Transfer` branch used to emit.
///
/// Runs after the portal fire + transit systems so every signal emitted this
/// frame is played the same frame, byte-identical to the old in-core writes.
pub fn play_portal_sfx(
    mut fired: MessageReader<PortalShotFired>,
    mut entered: MessageReader<PortalBodyEntered>,
    mut transited: MessageReader<PortalBodyTransited>,
    mut sfx: ambition_sfx::SfxWriter,
) {
    for ev in fired.read() {
        sfx.write(ambition_sfx::SfxMessage::Play {
            id: ambition_sfx::ids::PORTAL_FIRE,
            pos: ev.origin,
        });
        sfx.write(ambition_sfx::SfxMessage::Play {
            id: ambition_sfx::ids::PORTAL_TRAVEL,
            pos: ev.origin,
        });
    }
    for ev in entered.read() {
        sfx.write(ambition_sfx::SfxMessage::Play {
            id: ambition_sfx::ids::PORTAL_ENTER,
            pos: ev.pos,
        });
    }
    for ev in transited.read() {
        sfx.write(ambition_sfx::SfxMessage::Play {
            id: ambition_sfx::ids::PORTAL_EXIT,
            pos: ev.exit_pos,
        });
    }
}
