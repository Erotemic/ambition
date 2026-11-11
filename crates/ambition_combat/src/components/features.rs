//! Feature-entity components: identity, geometry, and the pickup/chest/
//! breakable/switch/pogo feature families.

use super::super::*;

/// Stable authored/runtime identity for a feature entity.
///
/// Use this for save keys, traces, and entity lookup. It intentionally mirrors
/// the IDs currently embedded in `PickupRuntime`, `ChestRuntime`,
/// `BreakableRuntime`, and `SwitchRuntime` so migration patches can move one
/// family without changing persistence vocabulary.
#[derive(Component, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FeatureId(pub String);

impl FeatureId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Human-facing authored name for debug overlays / inspectors.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct FeatureName(pub String);

/// A body must face this interactable to use it (Q63,
/// `InteractableSpec::requires_facing`). Put on the root by construction;
/// the interact road asks `ambition_interaction::faces`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RequiresFacing;

impl FeatureName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// World-space collision / interaction shape for a feature entity.
///
/// Re-export of the canonical machinery-layer center+half box
/// [`ae::CenteredAabb`]. ECS systems prefer this single component so collection,
/// interaction, damage, and overlay systems query one canonical shape — and it
/// is the same type the engine uses everywhere, so there is no per-layer box
/// conversion.
pub use ae::CenteredAabb;

/// Explicit persistence key. Kept separate from `FeatureId` so migrated features
/// can choose when authored identity and save identity differ.
#[derive(Component, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PersistKey(pub String);

impl PersistKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Published source regions with an explicit coordinate space.
#[derive(Clone, Debug, PartialEq)]
pub enum HurtSource {
    World(Vec<ae::CombatVolume>),
    Body(Vec<ae::CombatVolume>),
}

#[derive(Clone, Copy)]
enum HurtSpace {
    World,
    Body,
}

/// Damageable source regions. Absence of publication permits a body fallback.
/// An explicitly empty publication is intangible.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct DamageableVolumes {
    source: Option<HurtSource>,
}

/// One sampled hurt contact with its physical source and target-chart origin.
#[derive(Clone, Debug, PartialEq)]
pub struct BodySweepContact {
    pub hit: ae::frame::material_cast::SampledPieceHit,
    pub source_volume: ae::CombatVolume,
    pub source_in_target: ae::Vec2,
}

impl DamageableVolumes {
    pub fn published(&self) -> bool {
        self.source.is_some()
    }
    pub fn intangible(&self) -> bool {
        self.published() && self.is_empty()
    }
    pub fn source(&self) -> Option<&HurtSource> {
        self.source.as_ref()
    }
    pub fn len(&self) -> usize {
        self.domains().len()
    }
    pub fn is_empty(&self) -> bool {
        self.domains().is_empty()
    }

    fn domains(&self) -> &[ae::CombatVolume] {
        match &self.source {
            Some(HurtSource::World(domains) | HurtSource::Body(domains)) => domains,
            None => &[],
        }
    }

    fn space(&self) -> HurtSpace {
        match self.source {
            Some(HurtSource::Body(_)) => HurtSpace::Body,
            _ => HurtSpace::World,
        }
    }

    pub fn clear(&mut self) {
        self.publish_world(Vec::new());
    }

    /// Publish fixed regions in room coordinates.
    pub fn publish_world(&mut self, domains: Vec<ae::CombatVolume>) {
        self.source = Some(HurtSource::World(domains));
    }

    /// Publish regions in canonical body axes. Facing is already applied.
    pub fn publish_body(&mut self, domains: Vec<ae::CombatVolume>) {
        self.source = Some(HurtSource::Body(domains));
    }

    pub fn set_world_box(&mut self, bounds: ae::Aabb) {
        self.publish_world(vec![ae::CombatVolume::aabb(bounds)]);
    }

    pub fn world_box(bounds: ae::Aabb) -> Self {
        let mut out = Self::default();
        out.set_world_box(bounds);
        out
    }

    pub fn body_box(canonical_half: ae::Vec2) -> Self {
        let mut out = Self::default();
        out.publish_body(vec![ae::CombatVolume::aabb(ae::Aabb::new(
            ae::Vec2::ZERO,
            canonical_half,
        ))]);
        out
    }

    fn material(
        &self,
        body: Option<&ae::BodyKinematics>,
    ) -> Result<ae::frame::Sim2, ae::frame::thread::Refusal> {
        use ae::frame::thread::{Refusal, RefusalReason};
        match self.space() {
            HurtSpace::World => Ok(ae::frame::Sim2::IDENTITY),
            HurtSpace::Body => body
                .ok_or(Refusal {
                    path: Vec::new(),
                    link: None,
                    reason: RefusalReason::MissingMaterial,
                })?
                .material_map()
                .map_err(|fault| Refusal {
                    path: Vec::new(),
                    link: None,
                    reason: RefusalReason::Numeric(fault),
                }),
        }
    }

    /// Project source regions through the accepted physical material.
    pub fn world_volumes(
        &self,
        body: Option<&ae::BodyKinematics>,
    ) -> Result<Vec<ae::CombatVolume>, ae::frame::thread::Refusal> {
        if self.domains().is_empty() {
            return Ok(Vec::new());
        }
        let map = self.material(body)?;
        self.domains()
            .iter()
            .map(|domain| {
                ae::frame::thread::map_volume(domain, &map).map_err(|fault| {
                    ae::frame::thread::Refusal {
                        path: Vec::new(),
                        link: None,
                        reason: ae::frame::thread::RefusalReason::Numeric(fault),
                    }
                })
            })
            .collect()
    }

    /// Partition source material with the current room and body policy.
    pub fn pieces(
        &self,
        owner: Entity,
        body: Option<&ae::BodyKinematics>,
        geometry: Option<&ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry>,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    ) -> ae::frame::thread::Partition {
        use ae::frame::thread::{MaterialPrimitive, Refusal, RefusalReason, VolumePiece};
        use ambition_platformer2d_shared_tangle::body::geometry::PartSource;
        if self.domains().is_empty() {
            return Ok(Vec::new());
        }
        let map = self.material(body)?;
        let mut pieces = Vec::new();
        for domain in self.domains() {
            match (geometry, self.space()) {
                (Some(geometry), HurtSpace::Body) => {
                    pieces.extend(geometry.body_pieces(MaterialPrimitive { domain, to_world: map }, owner, room)?)
                }
                (Some(geometry), HurtSpace::World) => pieces.extend(geometry.pieces(
                    domain,
                    PartSource::World { owner: Some(owner) },
                    domain.center(),
                    room,
                )?),
                (None, _) => pieces.push(
                    VolumePiece::from_material(MaterialPrimitive {
                        domain,
                        to_world: map,
                    })
                    .map_err(|fault| Refusal {
                        path: Vec::new(),
                        link: None,
                        reason: RefusalReason::Numeric(fault),
                    })?,
                ),
            }
        }
        Ok(pieces)
    }

    /// Sweep the physical source body through the sampled hurt pieces.
    pub fn swept_body_contact(
        &self,
        owner: Entity,
        body: Option<&ae::BodyKinematics>,
        geometry: Option<&ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry>,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
        source: &ae::BodyKinematics,
        delta: ae::Vec2,
    ) -> Result<Option<BodySweepContact>, ae::frame::thread::Refusal> {
        use ae::frame::thread::{Refusal, RefusalReason};
        if self.domains().is_empty() { return Ok(None); }
        // Reject only bounds that cannot touch the source or enter a portal.
        // A failed bound remains a candidate and keeps its typed refusal.
        let numeric = |fault| Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(fault) };
        if !delta.is_finite() { return Err(numeric(ae::frame::NumericFault::NonFinite)); }
        let source_domain = source.canonical_box().map_err(numeric)?;
        let source_map = source.material_map().map_err(numeric)?;
        let projected_bounds = |domain: &ae::CombatVolume, map: ae::frame::Sim2| {
            let bounds = domain.bounds();
            // Primitive bounds can round inwards before a chart offset cancels.
            let bounds = ae::Aabb {
                min: ae::Vec2::new(bounds.min.x.next_down(), bounds.min.y.next_down()),
                max: ae::Vec2::new(bounds.max.x.next_up(), bounds.max.y.next_up()),
            };
            map.bounds(bounds)
        };
        let start = projected_bounds(&source_domain, source_map).map_err(numeric)?;
        let swept_min = start.min.as_dvec2().min(start.min.as_dvec2() + delta.as_dvec2());
        let swept_max = start.max.as_dvec2().max(start.max.as_dvec2() + delta.as_dvec2());
        let reaches_opening = |min: bevy::math::DVec2, max: bevy::math::DVec2,
            ap: ae::frame::PortalAperture, front: bool| {
            if !ap.frame.origin.is_finite() || !ap.frame.normal.is_finite()
                || !ap.half_length.is_finite() || ap.half_length <= 0.0 { return true; }
            let corners = [min, bevy::math::DVec2::new(max.x, min.y),
                max, bevy::math::DVec2::new(min.x, max.y)];
            let local = corners.map(|point| {
                let offset = point - ap.frame.origin.as_dvec2();
                (offset.dot(ap.frame.tangent().as_dvec2()), offset.dot(ap.frame.normal.as_dvec2()))
            });
            let half = f64::from(ap.half_length);
            !local.iter().all(|(_, y)| if front { *y < 0.0 } else { *y > 0.0 })
                && !local.iter().all(|(x, _)| *x < -half)
                && !local.iter().all(|(x, _)| *x > half)
        };
        if let Ok(map) = self.material(body) {
            let possible = self.domains().iter().any(|domain| {
                let Ok(bounds) = projected_bounds(domain, map) else { return true; };
                let (min, max) = (bounds.min.as_dvec2(), bounds.max.as_dvec2());
                if min.cmple(swept_max).all() && swept_min.cmple(max).all() { return true; }
                let Some(geometry) = geometry else { return false; };
                let carried = matches!(self.space(), HurtSpace::Body)
                    .then(|| geometry.straddled_by(owner)).flatten();
                let links = || geometry.links_in(room).iter().chain(carried.iter());
                // A carried region enters one opening and leaves a final opening.
                // Keep all link combinations, including chained paths.
                links().any(|link| reaches_opening(min, max, link.enter, false))
                    && links().any(|link| reaches_opening(swept_min, swept_max, link.exit, true))
            });
            if !possible { return Ok(None); }
        }
        let pieces = self.pieces(owner, body, geometry, room)?;
        if pieces.is_empty() { return Ok(None); }
        let domain = source_domain;
        let polygon = domain.polygon().expect("a canonical body box is a polygon");
        let map = source_map;
        let mut earliest: Option<BodySweepContact> = None;
        for piece in &pieces {
            if let Some(hit) = ae::frame::material_cast::sweep_polygon_against_piece(&polygon, map, delta, piece)? {
                if earliest.as_ref().is_none_or(|before| hit.toi < before.hit.toi) {
                    let source_volume = ae::frame::thread::map_volume(&domain, &hit.source_map).map_err(numeric)?;
                    let source_in_target = hit.target_map.inverse().and_then(|map| map.point(hit.source_map.point(ae::Vec2::ZERO)?)).map_err(numeric)?;
                    earliest = Some(BodySweepContact { hit, source_volume, source_in_target });
                }
            }
        }
        Ok(earliest)
    }

    /// Coarse bounds for affordance and presentation readers.
    pub fn bounds(&self,
        body: Option<&ae::BodyKinematics>,
    ) -> Result<Vec<ae::Aabb>, ae::frame::thread::Refusal> {
        self.world_volumes(body)
            .map(|volumes| volumes.iter().map(ae::CombatVolume::bounds).collect())
    }
}

/// The barrier after which every damage consumer may read published hurt
/// geometry, and before which no consumer may.
///
/// ⛔⛔ **IT EXISTS BECAUSE THE PUBLICATION HAPPENS TWICE AND ONLY ONE OF THEM
/// IS THIS ONE.** `refresh_body_damageable_volumes` is registered in `WorldPrep`
/// for the pogo derivation and the collision overlay, and again between
/// `Playback` and `Resolve` for damage — same rule, two consumers with different
/// timing needs. A producer that ordered itself `.before(the function)` named a
/// `SystemTypeSet` with two instances and Bevy refused the schedule. This set
/// names the SECOND registration, so an input's writer can say which sample it
/// must precede.
///
/// ⚠ MEMBERSHIP IS THE CLAIM. A system in here promises the damage road that its
/// geometry is current for this tick; a consumer outside it that derives hurt
/// geometry for itself is the defect A2a removed three instances of.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DamageFacingVolumesPublished;

/// Per-feature pogo derivation policy.
///
/// The default game rule is that things a body can damage are also valid
/// downslash/pogo refresh targets. `Disabled` is the escape hatch for puzzle
/// targets or hazardous objects that should take damage without granting a
/// bounce, while `Custom` leaves `PogoTargetVolumes` to a domain-specific
/// system.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PogoPolicy {
    #[default]
    FromDamageable,
    Custom,
    Disabled,
}

/// Entity-side geometry that may grant a pogo rebound.
///
/// Bodies publish this alongside their damageable silhouette so proximity/UI and
/// `PogoPolicy::Custom` can describe the pogoable part without losing body
/// identity. These volumes are not collision-world blocks by default. An ECS
/// feature becomes world rebound geometry only when it also carries
/// [`PogoTargetContributor`].
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct PogoTargetVolumes {
    pub volumes: Vec<ae::Aabb>,
}

/// Explicit marker for an ECS feature that contributes WORLD pogo geometry.
///
/// This is deliberately absent from combat bodies. A body is pogoed through its
/// resolved entity hit and [`PogoPolicy`]; a contributor is lowered into a
/// collision-world `PogoOrb` because it represents a surface with no body-victim
/// semantics (for example a stand-to-crumble or pogo-refresh platform).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PogoTargetContributor;

#[cfg(test)]
mod hurt_material_tests {
    use super::*;
    use ae::frame::{BodyMaterial, thread::RefusalReason};

    fn box_hit(hurt: &DamageableVolumes, body: Option<&ae::BodyKinematics>, start: ae::Vec2, half: ae::Vec2, delta: ae::Vec2)
        -> Result<Option<f64>, ae::frame::thread::Refusal> {
        let source = ae::BodyKinematics { pos: start, canonical_size: half * 2.0, ..Default::default() };
        hurt.swept_body_contact(Entity::PLACEHOLDER, body, None, None, &source, delta).map(|hit| hit.map(|hit| hit.hit.toi))
    }

    #[test]
    fn a_remote_refused_recipient_does_not_block_an_unrelated_sweep() {
        use ae::frame::thread::PieceBudget;
        use ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry;
        let owner = Entity::PLACEHOLDER;
        let body = ae::BodyKinematics { pos: ae::Vec2::new(1000.0, 400.0), ..Default::default() };
        let hurt = DamageableVolumes::body_box(ae::Vec2::splat(10.0));
        let mut geometry = BodyGeometry::default();
        geometry.set_body_budget(owner, PieceBudget { max_pieces: 0, ..PieceBudget::SINGLE_HOP });
        geometry.add_pair(None, (
            ae::frame::PortalAperture { frame: ae::frame::PortalFrame::fixed(body.pos, ae::Vec2::NEG_Y), half_length: 40.0 },
            ae::frame::PortalAperture { frame: ae::frame::PortalFrame::fixed(ae::Vec2::new(500.0, -200.0), ae::Vec2::Y), half_length: 40.0 },
        ));
        let mut source = ae::BodyKinematics { pos: body.pos - ae::Vec2::new(100.0, 200.0),
            canonical_size: ae::Vec2::splat(4.0), ..Default::default() };
        let contact = |source: &ae::BodyKinematics| hurt.swept_body_contact(owner, Some(&body),
            Some(&geometry), None, source, ae::Vec2::X * 20.0);
        assert!(contact(&source).unwrap().is_none());
        source.pos = body.pos - ae::Vec2::X * 20.0;
        assert!(contact(&source).is_err());
    }

    #[test]
    fn a_large_material_offset_does_not_shrink_the_hurt_bounds() {
        let body = ae::BodyKinematics { pos: ae::Vec2::new(-1_000_000.0, 0.0), ..Default::default() };
        let mut hurt = DamageableVolumes::default();
        hurt.publish_body(vec![ae::CombatVolume::circle(ae::Vec2::new(1_000_000.0, 0.0), 3.7)]);
        assert!(box_hit(&hurt, Some(&body), ae::Vec2::new(3.695, 0.0),
            ae::Vec2::splat(0.001), ae::Vec2::ZERO).unwrap().is_some());
    }

    #[test]
    fn native_sweeps_reach_shaped_material_between_endpoints() {
        let shapes = [
            ae::CombatVolume::aabb(ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(3.0, 5.0))),
            ae::CombatVolume::Circle {
                center: ae::Vec2::ZERO,
                radius: 5.0,
            },
            ae::CombatVolume::convex(vec![
                ae::Vec2::new(-5.0, -4.0),
                ae::Vec2::new(6.0, 0.0),
                ae::Vec2::new(-5.0, 4.0),
            ]),
        ];
        for reflected in [false, true] {
            let axes = bevy::math::Mat2::from_angle(0.41)
                * bevy::math::Mat2::from_cols(
                    if reflected {
                        ae::Vec2::NEG_X
                    } else {
                        ae::Vec2::X
                    },
                    ae::Vec2::Y,
                );
            let body = ae::BodyKinematics {
                pos: ae::Vec2::new(100.0, 200.0),
                material: BodyMaterial::new(2.0, axes).unwrap(),
                ..Default::default()
            };
            let start = body.pos + axes * ae::Vec2::new(-40.0, 0.0);
            let delta = axes * ae::Vec2::new(80.0, 0.0);
            let half = ae::Vec2::splat(0.5);
            for shape in &shapes {
                let mut hurt = DamageableVolumes::default();
                hurt.publish_body(vec![shape.clone()]);
                let world = hurt.world_volumes(Some(&body)).unwrap();
                assert!(!world[0].intersects_aabb(ae::Aabb::new(start, half)));
                assert!(!world[0].intersects_aabb(ae::Aabb::new(start + delta, half)));
                let time = box_hit(&hurt, Some(&body), start, half, delta)
                    .unwrap()
                    .expect("the path crosses the material");
                assert!(time > 0.0 && time < 1.0);
            }
        }
    }

    #[test]
    fn a_convex_regions_empty_corner_is_not_damageable() {
        let mut hurt = DamageableVolumes::default();
        hurt.publish_world(vec![ae::CombatVolume::convex(vec![
            ae::Vec2::ZERO,
            ae::Vec2::new(10.0, 0.0),
            ae::Vec2::new(0.0, 10.0),
        ])]);
        assert!(
            box_hit(&hurt,
                None,
                ae::Vec2::new(8.0, 8.0),
                ae::Vec2::splat(0.25),
                ae::Vec2::ZERO
            )
            .unwrap()
            .is_none()
        );
        assert!(
            box_hit(&hurt,
                None,
                ae::Vec2::new(2.0, 2.0),
                ae::Vec2::splat(0.25),
                ae::Vec2::ZERO
            )
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn canonical_hurt_regions_keep_source_precision_and_require_material() {
        let source = ae::CombatVolume::aabb(ae::Aabb::new(
            ae::Vec2::new(1.0 / 3.0, 2.0 / 7.0),
            ae::Vec2::new(3.7, 5.1),
        ));
        let mut hurt = DamageableVolumes::default();
        hurt.publish_body(vec![source.clone()]);
        assert!(matches!(
            hurt.world_volumes(None).unwrap_err().reason,
            RefusalReason::MissingMaterial
        ));
        let mut body = ae::BodyKinematics {
            pos: ae::Vec2::splat(1_000_000.0),
            material: BodyMaterial::new(1.3, bevy::math::Mat2::from_angle(0.37)).unwrap(),
            ..Default::default()
        };
        for reflected in [false, true] {
            if reflected {
                body.material = body
                    .material
                    .carried(
                        ae::frame::Sim2::new(
                            0.5,
                            bevy::math::Mat2::from_cols(ae::Vec2::NEG_X, ae::Vec2::Y),
                            ae::Vec2::ZERO,
                        )
                        .unwrap(),
                    )
                    .unwrap();
            }
            let pieces = hurt
                .pieces(Entity::PLACEHOLDER, Some(&body), None, None)
                .unwrap();
            assert_eq!(pieces[0].source(), &source);
            assert_eq!(
                pieces[0].world_volume(),
                &hurt.world_volumes(Some(&body)).unwrap()[0]
            );
        }
        hurt.clear();
        assert!(
            hurt.pieces(Entity::PLACEHOLDER, None, None, None)
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn sampled_sweeps_retain_the_clipped_far_side_cap_and_live_body_pose() {
        use ae::frame::{MapConvention, PortalAperture, PortalFrame, Sim2};
        use ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry;
        let owner = Entity::PLACEHOLDER;
        let mut geometry = BodyGeometry::new(MapConvention::Reflection);
        geometry.add_pair(None, (
            PortalAperture { frame: PortalFrame::fixed(ae::Vec2::ZERO, ae::Vec2::NEG_Y), half_length: 40.0 },
            PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(500.0, -200.0), ae::Vec2::Y), half_length: 80.0 },
        ));
        geometry.set_body_material(owner, Ok(Sim2::IDENTITY));
        let body = ae::BodyKinematics {
            pos: ae::Vec2::new(0.0, -5.0),
            material: BodyMaterial::new(0.5, bevy::math::Mat2::from_cols(ae::Vec2::NEG_X, ae::Vec2::Y)).unwrap(),
            ..Default::default()
        };
        let mut hurt = DamageableVolumes::default();
        hurt.publish_body(vec![ae::CombatVolume::circle(ae::Vec2::ZERO, 20.0)]);
        for (point, expected) in [(ae::Vec2::new(500.0, -195.0), true), (ae::Vec2::new(515.0, -195.0), false), (ae::Vec2::new(500.0, -215.0), false)] {
            let source = ae::BodyKinematics { pos: point, canonical_size: ae::Vec2::splat(0.5), ..Default::default() };
            let hit = hurt.swept_body_contact(owner, Some(&body), Some(&geometry), None, &source, ae::Vec2::ZERO).unwrap();
            assert_eq!(hit.is_some(), expected, "point {point:?}");
            if let Some(hit) = hit { assert!(!hit.hit.target_path.is_empty()); }
        }
        let source = ae::BodyKinematics { pos: ae::Vec2::new(500.0, -175.0), canonical_size: ae::Vec2::ONE, ..Default::default() };
        let delta = ae::Vec2::new(0.0, -30.0);
        let contact = hurt.swept_body_contact(owner, Some(&body), Some(&geometry), None, &source, delta).unwrap().unwrap();
        assert!(contact.hit.toi > 0.0 && contact.hit.toi < 1.0);
        assert!(contact.hit.normal.dot(delta) < 0.0);
        assert!(contact.hit.point.y >= -200.0 && contact.hit.point.y <= -190.0 + 1.0e-4);
        assert!(contact.source_in_target.x.abs() < 1.0);
        assert!(!contact.hit.target_path.is_empty());
    }

}
