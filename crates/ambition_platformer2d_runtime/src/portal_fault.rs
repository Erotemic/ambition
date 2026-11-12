//! The visible face of a numeric portal refusal (PSIM-5).
//!
//! A refused movement keeps the previous admitted world, so the player sees
//! nothing happen. When the reason is numeric, the mouth that refused says so
//! with the letters IEEE 754 and the fault's symbols. Budget limits are policy,
//! not arithmetic, and show nothing here.
//!
//! This reads the refusal each body already reports and keeps no state of its
//! own: every tick a refusal stands, it is written again under the same
//! episode, and the presentation draws one effect per episode.

use bevy::prelude::*;

use ambition_platformer2d_actor_monolith::body::BodyFrameOutput;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_core::frame::NumericFault;
use ambition_platformer2d_core::frame::thread::{PortalLink, RefusalReason};
use ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
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

/// One refused operation: the body, the mouth it refused at and the reason.
/// Equal for every tick the same refusal stands, and for a replay of that
/// tick; the presentation tells a refusal that ended and came back by the gap.
fn episode(body: Entity, link: u32, reason: RefusalReason) -> u64 {
    // FNV-1a over fixed bytes: the same refusal gives the same key in every
    // process, and the key seeds where each glyph is drawn.
    let reason_code: u64 = match reason {
        RefusalReason::MissingMaterial => 1,
        RefusalReason::HopLimit => 2,
        RefusalReason::PieceLimit => 3,
        RefusalReason::WorkLimit => 4,
        RefusalReason::ScaleLimit => 5,
        RefusalReason::Numeric(fault) => 16 + fault as u64,
    };
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for word in [body.to_bits(), u64::from(link), reason_code] {
        for byte in word.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
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
    bodies: Query<(Entity, &BodyFrameOutput, &BodyKinematics)>,
    geometry: Option<Res<BodyGeometry>>,
    rooms: LiveRooms,
    mut vfx: VfxWriter,
) {
    let Some(geometry) = geometry else {
        return;
    };
    for (entity, output, body) in &bodies {
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
            episode: episode(entity, link.key, refusal.reason),
        });
    }
}
