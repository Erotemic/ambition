//! Physical item release from custody into an admitted world chart.

use super::*;
use ae::snapshot::SnapshotState;

/// The holder's material when this item entered custody.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct HeldMaterialBasis(pub ae::frame::BodyMaterial);

impl SnapshotState for HeldMaterialBasis {
    fn encode(&self, out: &mut Vec<u8>) { self.0.encode(out); }
    fn decode(reader: &mut ae::snapshot::Reader<'_>) -> Option<Self> {
        Some(Self(ae::frame::BodyMaterial::decode(reader)?))
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct ItemReleaseWorld<'w, 's> {
    pub(super) rooms: LiveRooms<'w, 's>,
    geometry: Option<Res<'w, ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry>>,
}

impl ItemReleaseWorld<'_, '_> {
    pub fn prepare_release(&self, holder: Entity, item: Option<Entity>, kin: &BodyKinematics, stored: &BodyKinematics,
        basis: HeldMaterialBasis, pos: Vec2, vel: Vec2,
    ) -> Result<PreparedItemRelease, ae::frame::thread::Refusal> {
        use ae::frame::thread::{MaterialPrimitive, Refusal, RefusalReason, partition};
        let numeric = |fault| Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(fault) };
        let delta = basis.0.at(Vec2::ZERO).and_then(|map| map.inverse())
            .and_then(|map| map.then(kin.material.at(Vec2::ZERO)?)).map_err(numeric)?;
        let mut desired = BodyKinematics { pos, vel, material: stored.material.carried(delta).map_err(numeric)?, ..*stored };
        let room = self.rooms.of(holder);
        let admission = self.geometry.as_deref().and_then(|geometry| geometry.motion_for(holder, room)
            .map(|admission| ae::frame::thread::MotionAdmission { budget: geometry.budget_for(item, room), ..admission }));
        if let Some(admission) = admission {
            let domain = desired.canonical_box().map_err(numeric)?;
            let parts = partition(MaterialPrimitive { domain: &domain, to_world: desired.material_map().map_err(numeric)? },
                kin.pos, admission.links, admission.first, admission.budget, admission.convention)?;
            let mut root = None;
            for piece in &parts {
                if piece.source_bounds_contain(Vec2::ZERO).map_err(numeric)?
                    && root.as_ref().is_none_or(|old: &&ae::frame::thread::VolumePiece|
                        (piece.path.len(), &piece.path) < (old.path.len(), &old.path)) { root = Some(piece); }
            }
            let piece = root.ok_or_else(|| numeric(ae::frame::NumericFault::InvalidFrame))?;
            let map = piece.path_transform().map_err(numeric)?;
            desired.vel = admission.carry_velocity(&piece.path, desired.pos, desired.vel, 1.0)?;
            desired.pos = map.point(desired.pos).map_err(numeric)?;
            desired.material = desired.material.carried(map).map_err(numeric)?;
        }
        let mut result = desired;
        ae::movement::prepare_body_materialization(desired, admission.map(|admission| ae::frame::thread::MotionAdmission { first: None, ..admission }))?.commit(&mut result, None);
        Ok(PreparedItemRelease { body: result })
    }
}


/// Put an object a body holds down at `pos`, at rest: the hand let go of it
/// without throwing it (a stow, a swap, or the body leaving the session).
///
/// The same entity and `SimId` go back to the world, so nothing is minted and
/// nothing is lost.
pub fn put_down(
    commands: &mut Commands,
    item: Entity,
    body: &BodyKinematics,
    custody: &mut ItemCustody,
    basis: HeldMaterialBasis,
    holder_kin: &BodyKinematics,
    world: &ItemReleaseWorld,
    pos: Vec2,
) -> Result<(), ae::frame::thread::Refusal> {
    let ItemCustody::Held { holder } = *custody else { return Ok(()); };
    let body = world.prepare_release(holder, Some(item), holder_kin, body, basis, pos, Vec2::ZERO)?;
    body.put_down(commands, item, custody);
    Ok(())
}

/// An admitted world pose for an item that leaves custody.
#[must_use]
pub struct PreparedItemRelease {
    pub(super) body: BodyKinematics,
}

impl PreparedItemRelease {
    /// Commit while the holder and release context remain unchanged.
    pub fn put_down(self, commands: &mut Commands, item: Entity, custody: &mut ItemCustody) {
        commands.entity(item).insert((self.body, ae::SweepSample::at_tick_end(self.body, Vec2::ZERO), HeldMaterialBasis::default()));
        *custody = ItemCustody::InWorld;
        // Not a `Release`: it stamps no `ReleasedAs`, so a bomb that leaves a
        // hand this way does not arm.
        commands.entity(item).remove::<ReleasedAs>();
        // Let go at whatever height that body is at: the object is back under
        // gravity until it lands on something.
        commands.entity(item).remove::<SettledItem>();
    }
}


/// A body lets go of what it holds: the ONE `Held -> InWorld` of an object
/// that has an entity, for a press and for a body no press drives alike (a
/// bird that lets a coconut go). The hand empties; the object the body took
/// custody of is put at `pos` with `vel`, in the world again, and says how it
/// was released.
///
/// `Ok(false)` means the hand holds a quantity without a world object. The
/// caller must admit its materialization before it empties that hand.
/// A refusal changes neither the hand nor the stored item.
pub fn release_carried_item(
    commands: &mut Commands,
    body: Entity,
    repertoire: &mut RepertoireQueryItem<'_, '_>,
    carried: &mut Query<(Entity, &GroundItem, &BodyKinematics, &mut ItemCustody, &HeldMaterialBasis)>,
    holder_kin: &BodyKinematics,
    world: &ItemReleaseWorld,
    pos: Vec2,
    vel: Vec2,
    release: Release,
) -> Result<bool, ae::frame::thread::Refusal> {
    let Some((entity, _, item_body, mut custody, basis)) = carried.iter_mut().find(|(_, _, _, custody, _)| custody.held_by(body)) else {
        return Ok(false);
    };
    let released = world.prepare_release(body, Some(entity), holder_kin, item_body, *basis, pos, vel)?;
    unequip_held(commands, body, repertoire);
    released.put_down(commands, entity, &mut custody);
    // The release SAYS what it was. Every fuse downstream reads this
    // rather than re-deriving intent from the launch vector.
    commands.entity(entity).insert(ReleasedAs(release));
    // ⭐ A DROP IS THE CASE THAT NEEDS THIS, not the throw. `Release::Drop`
    // launches at ZERO velocity, so an object that kept the settled marker
    // it wore when it was picked up would hang at head height forever.
    commands.entity(entity).remove::<SettledItem>();
    Ok(true)
}

