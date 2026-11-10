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
                    pieces.extend(geometry.body_pieces(domain, owner, room)?)
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

    /// Earliest contact of a moving box with the sampled source regions.
    pub fn swept_box_hit(
        &self,
        body: Option<&ae::BodyKinematics>,
        start: ae::Vec2,
        half: ae::Vec2,
        delta: ae::Vec2,
    ) -> Result<Option<f32>, ae::frame::thread::Refusal> {
        use ae::frame::material_cast::{InitialOverlap, MaterialCast};
        use ae::frame::thread::{MaterialPrimitive, Refusal, RefusalReason};
        if self.domains().is_empty() {
            return Ok(None);
        }
        let map = self.material(body)?;
        let numeric = |fault| Refusal {
            path: Vec::new(),
            link: None,
            reason: RefusalReason::Numeric(fault),
        };
        let root = map.point(ae::Vec2::ZERO).map_err(numeric)?;
        let end = start + delta;
        let obstacle = [
            end - half,
            end + ae::Vec2::new(half.x, -half.y),
            end + half,
            end + ae::Vec2::new(-half.x, half.y),
        ];
        let mut earliest: Option<f64> = None;
        for domain in self.domains() {
            let cast = MaterialCast::new(
                MaterialPrimitive {
                    domain,
                    to_world: map,
                },
                root,
                ae::Vec2::ZERO,
                0.0..1.0,
            )
            .with_initial_overlap(InitialOverlap::Report);
            let cells = cast.cells(None)?;
            for part in cells.parts() {
                if let Some(hit) = cells.against(part, &obstacle, delta)? {
                    earliest = Some(earliest.map_or(hit.toi, |before| before.min(hit.toi)));
                }
            }
        }
        Ok(earliest.map(|time| time as f32))
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
                let time = hurt
                    .swept_box_hit(Some(&body), start, half, delta)
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
            hurt.swept_box_hit(
                None,
                ae::Vec2::new(8.0, 8.0),
                ae::Vec2::splat(0.25),
                ae::Vec2::ZERO
            )
            .unwrap()
            .is_none()
        );
        assert!(
            hurt.swept_box_hit(
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
}
