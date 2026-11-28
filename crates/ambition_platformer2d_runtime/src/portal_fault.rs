//! The visible face of a numeric portal refusal (PSIM-5).
//!
//! A refused movement keeps the previous admitted world, so the player sees
//! nothing happen. Numeric faults show IEEE 754 and the fault symbols at
//! the mouth. A scale limit shows EXPONENT BIAS.
//!
//! This reads the refusal each body already reports and keeps no state of its
//! own: every tick a refusal stands, it is written again under the same
//! episode, and the presentation draws one effect per episode.

use bevy::prelude::*;

use ambition_platformer2d_actor::body::BodyFrameOutput;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_core::frame::NumericFault;
use ambition_platformer2d_core::frame::thread::{PortalLink, RefusalReason};
use ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry;
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms};
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use ambition_vfx::{FaultGlyph, VfxMessage, VfxWriter};

/// What a refusal shows, or `None` for a refusal that is not numeric.
pub(crate) fn fault_glyph(reason: RefusalReason) -> Option<FaultGlyph> {
    Some(match reason {
        RefusalReason::Numeric(NumericFault::NonFinite) => FaultGlyph::NotANumber,
        RefusalReason::Numeric(NumericFault::Overflow) => FaultGlyph::Infinity,
        RefusalReason::Numeric(NumericFault::Underflow) => FaultGlyph::SignedZero,
        RefusalReason::Numeric(NumericFault::Precision | NumericFault::InvalidScale | NumericFault::InvalidFrame) => {
            FaultGlyph::BitPattern
        }
        RefusalReason::ScaleLimit => FaultGlyph::ExponentBias,
        RefusalReason::MissingMaterial | RefusalReason::HopLimit | RefusalReason::PieceLimit | RefusalReason::WorkLimit => {
            return None;
        }
    })
}

/// One key for a body, live room, mouth, and refusal reason.
fn episode(
    body: Entity,
    id: Option<&SimId>,
    room: Option<LiveRoomInstance>,
    link: u32,
    reason: RefusalReason,
) -> u64 {
    let reason_code = match reason {
        RefusalReason::MissingMaterial => 1,
        RefusalReason::HopLimit => 2,
        RefusalReason::PieceLimit => 3,
        RefusalReason::WorkLimit => 4,
        RefusalReason::ScaleLimit => 5,
        RefusalReason::Numeric(fault) => 16 + fault as u8,
    };
    let entity_bits = body.to_bits().to_le_bytes();
    let body_key = id.map_or(entity_bits.as_slice(), |id| id.as_str().as_bytes());
    let room_key = room.map_or(u64::MAX, |room| u64::from(room.ordinal()));
    // The key also sets the glyph positions. Use the simulation identity when it exists.
    [u8::from(id.is_some()), reason_code].into_iter()
        .chain(room_key.to_le_bytes())
        .chain(link.to_le_bytes())
        .chain(body_key.iter().copied())
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
}

/// The mouth to draw at: the link the refusal names, else the entry nearest
/// the body (a refusal found inside a curved search can name no link).
fn refusing_mouth(links: &[PortalLink], link: Option<u32>, body: Vec2) -> Option<&PortalLink> {
    match link {
        Some(key) => links.iter().find(|candidate| candidate.key == key),
        None => links.iter().min_by(|a, b| {
            a.enter.frame.origin.distance_squared(body).total_cmp(&b.enter.frame.origin.distance_squared(body))
        }),
    }
}

/// Write each body's numeric portal refusal at the mouth that refused it.
pub fn publish_portal_numeric_faults(
    bodies: Query<(Entity, Option<&SimId>, &BodyFrameOutput, &BodyKinematics)>,
    geometry: Option<Res<BodyGeometry>>,
    rooms: LiveRooms,
    mut vfx: VfxWriter,
) {
    let Some(geometry) = geometry else {
        return;
    };
    for (entity, id, output, body) in &bodies {
        let Some(refusal) = &output.events.motion_refusal else {
            continue;
        };
        let Some(glyph) = fault_glyph(refusal.reason) else {
            continue;
        };
        let room = rooms.of(entity);
        let Some(link) = refusing_mouth(geometry.links_in(room), refusal.link, body.pos) else {
            continue;
        };
        let mouth = link.enter;
        vfx.write_in(room, VfxMessage::PortalFault {
            pos: mouth.frame.origin,
            normal: mouth.frame.normal,
            half_length: mouth.half_length,
            glyph,
            episode: episode(entity, id, room, link.key, refusal.reason),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only arithmetic shows IEEE 754; a policy cap shows its own accent, and
    /// a budget shows nothing.
    #[test]
    fn each_refusal_shows_its_own_glyphs_and_a_budget_shows_none() {
        use NumericFault::*;
        let shown = |reason| fault_glyph(reason);
        assert_eq!(shown(RefusalReason::Numeric(NonFinite)), Some(FaultGlyph::NotANumber));
        assert_eq!(shown(RefusalReason::Numeric(Overflow)), Some(FaultGlyph::Infinity));
        assert_eq!(shown(RefusalReason::Numeric(Underflow)), Some(FaultGlyph::SignedZero));
        for fault in [Precision, InvalidScale, InvalidFrame] {
            assert_eq!(shown(RefusalReason::Numeric(fault)), Some(FaultGlyph::BitPattern));
        }
        assert_eq!(shown(RefusalReason::ScaleLimit), Some(FaultGlyph::ExponentBias));
        for budget in [RefusalReason::MissingMaterial, RefusalReason::HopLimit, RefusalReason::PieceLimit, RefusalReason::WorkLimit] {
            assert_eq!(shown(budget), None);
        }
    }
}
