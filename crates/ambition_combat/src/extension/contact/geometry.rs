//! Project native movement facts without another collision query.

use ambition_platformer2d_core::{self as ae, collision_semantics::*};
use ambition_combat_port::WorldContact;
use ambition_combat_port::world_geometry::*;

pub(super) fn world_contact(contact: &Contact) -> WorldContact {
    let (block, chain) = match &contact.source {
        ContactSource::Block { id, kind } => (Some(BlockContact {
            geometry: geometry_id(id), material: block_material(*kind),
            blink_tier: match kind {
                ae::BlockKind::BlinkWall { tier } => Some(match tier {
                    ae::BlinkWallTier::Soft => BlinkTier::Soft,
                    ae::BlinkWallTier::Hard => BlinkTier::Hard,
                }),
                _ => None,
            },
            rebound_impulse: match kind {
                ae::BlockKind::Rebound { impulse } => Some(impulse.to_array()),
                _ => None,
            },
        }), None),
        ContactSource::Chain { chain, segment } => (None, Some(ChainContact {
            chain: *chain, segment: *segment,
        })),
    };
    WorldContact {
        role: match contact.kind {
            ContactKind::Support => ContactRole::Support,
            ContactKind::Head => ContactRole::Head,
            ContactKind::Side => ContactRole::Side,
            ContactKind::Attachment => ContactRole::Attachment,
        },
        point: contact.point.to_array(), normal: contact.normal.to_array(),
        toi: contact.toi, surface_velocity: contact.surface_velocity.to_array(),
        impact_speed: contact.impact_speed, involuntary: contact.involuntary,
        block, chain,
        met: contact.met.map(|met| MetContact {
            hops: met.hops, point: met.point.to_array(), normal: met.normal.to_array(),
            surface_velocity: met.surface_velocity.to_array(),
        }),
    }
}

fn geometry_id(id: &ae::GeoId) -> GeometryId {
    use ae::geo_id::GeoSource;
    let (origin, key, operation) = match &id.source {
        GeoSource::Placement(id) => (GeometryOrigin::Placement, Some(id.as_str().into()), None),
        GeoSource::TileLayer { layer } => (GeometryOrigin::TileLayer, Some(layer.clone()), None),
        GeoSource::Generator(id) => (GeometryOrigin::Generator, Some(id.as_str().into()), None),
        GeoSource::Delta { op_index } => (GeometryOrigin::Delta, None, Some(*op_index)),
        GeoSource::Anon => (GeometryOrigin::Anonymous, None, None),
    };
    GeometryId { origin, key, operation, index: id.index.into() }
}

fn block_material(kind: ae::BlockKind) -> BlockMaterial {
    match kind {
        ae::BlockKind::Solid => BlockMaterial::Solid,
        ae::BlockKind::OneWay => BlockMaterial::OneWay,
        ae::BlockKind::BonkOnly => BlockMaterial::BonkOnly,
        ae::BlockKind::BlinkWall { .. } => BlockMaterial::BlinkWall,
        ae::BlockKind::Barrier => BlockMaterial::Barrier,
        ae::BlockKind::Hazard => BlockMaterial::Hazard,
        ae::BlockKind::PogoOrb => BlockMaterial::PogoOrb,
        ae::BlockKind::Rebound { .. } => BlockMaterial::Rebound,
    }
}
