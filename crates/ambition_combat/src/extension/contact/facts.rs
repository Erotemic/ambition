//! Shared physical contact samples for bound body rules.

use super::*;
use ambition_characters::control::DrivingParticipant;
use ambition_combat_port::{ContactParticipant, ContactRefusal, ContactSurface};
use ambition_extension_host::InBodyContact;
use ambition_platformer2d_shared_tangle::{
    body::geometry::BodyGeometry,
    lifecycle::{LiveRoomInstance, RoomFreeze, SessionScopeId, SessionScopedEntity},
};
use ambition_platformer2d_world::governing_rules::RulesOf;
use bevy::ecs::{query::QueryData, system::{SystemParam, SystemState}};
use std::collections::{BTreeMap, BTreeSet};

/// A game can bound the bodies and surface points in each contact sample.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContactFactsLimits {
    pub max_touchers: u32,
    pub max_surfaces: u32,
    pub max_world_contacts: u32,
}
impl Default for ContactFactsLimits {
    fn default() -> Self {
        Self {
            max_touchers: 32,
            max_surfaces: 8,
            max_world_contacts: 32,
        }
    }
}
impl ContactFactsLimits {
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.max_touchers == 0 {
            problems.push("body_contact_events.max_touchers must be positive".into());
        }
        if self.max_surfaces == 0 {
            problems.push("body_contact_events.max_surfaces must be positive".into());
        }
        if self.max_world_contacts == 0 {
            problems.push("body_contact_events.max_world_contacts must be positive".into());
        }
        problems
    }
}

type Context = SystemState<ContactContext<'static, 'static>>;

#[derive(SystemParam)]
pub struct ContactContext<'w, 's> {
    rooms: LiveRooms<'w, 's>,
    freeze: RoomFreeze<'w, 's>,
    rules: RulesOf<'w, 's, ContactFactsLimits>,
    bodies: Query<'w, 's, ContactBodySource>,
}

#[derive(QueryData)]
struct ContactBodySource {
    entity: Entity,
    id: &'static SimId,
    kinematics: &'static ae::BodyKinematics,
    config: Option<&'static crate::actor_tuning::ActorConfig>,
    statuses: Option<&'static super::super::status::BodyStatuses>,
    health: Option<&'static BodyHealth>,
    out_of_play: Has<crate::death_rules::OutOfPlay>,
    plane: Option<&'static ae::DepthPlane>,
    session: Option<&'static SessionScopedEntity>,
    driven: Has<DrivingParticipant>,
}

impl ContactBodySourceItem<'_, '_> {
    fn selectors(&self) -> impl Iterator<Item = &str> + '_ {
        let brain = self.config.and_then(|config| match &config.brain {
            ambition_entity_catalog::placements::CharacterBrain::Custom(key) => Some(key.as_str()),
            _ => None,
        });
        self.statuses.into_iter().flat_map(|statuses| statuses.0.iter().map(String::as_str)).chain(brain)
    }

    fn scope(
        &self,
        rooms: &LiveRooms,
        freeze: &RoomFreeze,
        rules: &RulesOf<ContactFactsLimits>,
    ) -> Option<(PartitionKey, ContactFactsLimits)> {
        if crate::util::body_is_untouchable(self.health, self.out_of_play, self.plane) {
            return None;
        }
        let room = rooms.of(self.entity);
        if (room.is_none() && rooms.count() > 0) || freeze.scale_of(self.entity) <= 0.0 {
            return None;
        }
        let limits = rules.of(self.entity).copied().unwrap_or_default();
        limits.problems().is_empty().then_some(((self.session.map(|session| session.0), room), limits))
    }
}
struct Sample {
    entity: Entity,
    id: String,
    kinematics: ae::BodyKinematics,
    session: Option<SessionScopeId>,
    room: Option<LiveRoomInstance>,
    driven: bool,
    keys: Vec<String>,
    limits: ContactFactsLimits,
    pieces: Vec<PhysicalPiece>,
    refusal: Option<String>,
}

struct PhysicalPiece {
    chart: ae::frame::thread::VolumePiece,
    volume: ae::CombatVolume,
}

struct Envelope { sample: usize, bounds: ae::Aabb }

type PartitionKey = (Option<SessionScopeId>, Option<LiveRoomInstance>);
/// A partition's material pieces and, for a large one, the range grid over
/// their bounds: no overlay, since a partition is built once and only read.
struct ContactPartition {
    material: Vec<Envelope>,
    grid: Option<ae::spatial::RangeGrid>,
    refused: Vec<usize>,
}

/// The material count above which a partition's grid pays for its build.
/// Measured (K3): a partition is built every tick and asked once per
/// recipient, and its recipients are its own samples, so the work is about
/// pieces². Built and asked, the grid beats the direct scan only near 256 x
/// 256 (141 µs against 151 µs); below that the scan is cheaper, by 7-25x at
/// tens of pieces.
const PIECES_WORTH_A_GRID: usize = 256;

fn contact_partitions(samples: &[Sample]) -> BTreeMap<PartitionKey, ContactPartition> {
    let mut partitions = BTreeMap::<PartitionKey, (Vec<Envelope>, Vec<usize>)>::new();
    for (index, sample) in samples.iter().enumerate() {
        let (material, refused) = partitions.entry((sample.session, sample.room)).or_default();
        if sample.refusal.is_some() { refused.push(index); }
        for piece in &sample.pieces {
            material.push(Envelope {sample: index, bounds: piece.volume.bounds()});
        }
    }
    partitions.into_iter().map(|(key, (material, refused))| {
        let grid = (material.len() > PIECES_WORTH_A_GRID)
            .then(|| ae::spatial::RangeGrid::build(material.iter().map(|envelope| envelope.bounds)));
        (key, ContactPartition {material, grid, refused})
    }).collect()
}

impl ContactPartition {
    fn near(&self, recipient: &Sample) -> Vec<usize> {
        // Unknown material must still report an incomplete read in this partition.
        let mut near = self.refused.clone();
        for piece in &recipient.pieces {
            let region = piece.volume.bounds();
            match &self.grid {
                Some(grid) => near.extend(grid.near(region).map(|i| &self.material[i as usize])
                    .filter(|envelope| ae::spatial::touches(envelope.bounds, region)).map(|envelope| envelope.sample)),
                None => near.extend(self.material.iter()
                    .filter(|envelope| ae::spatial::touches(envelope.bounds, region)).map(|envelope| envelope.sample)),
            }
        }
        // Grid cells and multiple portal pieces must not change identity order.
        near.sort_unstable();
        near.dedup();
        near
    }
}

pub fn queue_body_contacts(world: &mut World, mut context: Local<Option<Context>>) {
    world.resource_mut::<ContactTouchers>().0.clear();
    if world.resource::<ambition_time::WorldTime>().sim_dt() <= 0.0 {
        return;
    }
    let context = context.get_or_insert_with(|| SystemState::new(world));
    let bound: BTreeSet<&str> = world
        .resource::<AdmittedExtensions>()
        .0
        .entries
        .iter()
        .filter(|entry| entry.descriptor.trigger.port == BodyContactPort::KEY)
        .map(|entry| entry.descriptor.trigger.selector.as_ref())
        .collect();
    if bound.is_empty() {
        return;
    }
    let mut samples: Vec<Sample> = {
        let ContactContext { rooms, freeze, rules, bodies } = context.get(world).expect("contact read context");
        let mut observed = BTreeSet::new();
        let mut recipients = BTreeMap::new();
        for body in &bodies {
            let mut keys: Vec<_> = body.selectors().filter(|key| bound.contains(key)).collect();
            if keys.is_empty() { continue; }
            let Some((scope, _)) = body.scope(&rooms, &freeze, &rules) else { continue; };
            keys.sort_unstable();
            keys.dedup();
            observed.insert(scope);
            recipients.insert(body.entity, keys.into_iter().map(str::to_owned).collect());
        }
        if observed.is_empty() {
            return;
        }
        bodies.iter().filter_map(|body| {
            let ((session, room), limits) = body.scope(&rooms, &freeze, &rules)?;
            if !observed.contains(&(session, room)) { return None; }
            Some(Sample {
                entity: body.entity,
                id: body.id.as_str().into(),
                kinematics: *body.kinematics,
                session,
                room,
                driven: body.driven,
                keys: recipients.remove(&body.entity).unwrap_or_default(),
                limits,
                pieces: Vec::new(),
                refusal: None,
            })
        }).collect()
    };
    let empty_geometry = BodyGeometry::default();
    let geometry = world
        .get_resource::<BodyGeometry>()
        .unwrap_or(&empty_geometry);
    for sample in &mut samples {
        match geometry.physical_body_pieces(sample.entity, &sample.kinematics, sample.room) {
            Ok(pieces) => {
                for chart in pieces {
                    match volume(&chart) {
                        Ok(Some(volume)) => sample.pieces.push(PhysicalPiece {chart, volume}),
                        Ok(None) => {},
                        Err(reason) => { sample.refusal = Some(reason); break; },
                    }
                }
            },
            Err(refusal) => sample.refusal = Some(format!("{:?}", refusal.reason)),
        }
    }
    samples.sort_by(|a, b| {
        a.session
            .cmp(&b.session)
            .then(a.room.cmp(&b.room))
            .then(a.id.cmp(&b.id))
    });
    let partitions = contact_partitions(&samples);
    let tick = world.resource::<ambition_time::SimTick>().0;
    for recipient in samples
        .iter()
        .filter(|body| !body.keys.is_empty() && body.refusal.is_none())
    {
        let mut touchers = Vec::new();
        let mut entities = Vec::new();
        let mut truncated = false;
        let mut refusals = Vec::new();
        let mut incomplete = false;
        let mut refusals_truncated = false;
        let partition = &partitions[&(recipient.session, recipient.room)];
        for index in partition.near(recipient) {
            let toucher = &samples[index];
            if toucher.entity == recipient.entity { continue; }
            let read = toucher.refusal.as_ref().map_or_else(
                || surfaces(recipient, toucher),
                |reason| Err(reason.clone()),
            );
            let (surfaces, surfaces_truncated) = match read {
                Ok(read) => read,
                Err(reason) => {
                    incomplete = true;
                    if refusals.len() < recipient.limits.max_touchers as usize {
                        refusals.push(ContactRefusal {
                            id: toucher.id.clone(),
                            reason,
                        });
                    } else {
                        refusals_truncated = true;
                    }
                    continue;
                }
            };
            if surfaces.is_empty() {
                continue;
            }
            if touchers.len() == recipient.limits.max_touchers as usize {
                truncated = true;
                break;
            }
            touchers.push(ContactParticipant {
                id: toucher.id.clone(),
                body: contact_body(&toucher.kinematics),
                driven: toucher.driven,
                relation: crate::targeting::observed_relation(
                    world,
                    recipient.entity,
                    toucher.entity,
                ),
                surfaces,
                surfaces_truncated,
            });
            entities.push(toucher.entity);
        }
        let step = world.get::<ae::BodyFrameOutput>(recipient.entity)
            .filter(|step| step.stamp == Some(ae::BodyStepStamp {
                tick, room: recipient.room.map(|room| room.ordinal()),
                session: recipient.session.map(|session| session.0),
            }));
        let contact = BodyContact {
            observed_at: tick,
            room: recipient.room.map(|room| room.ordinal()),
            body_id: recipient.id.clone(),
            driven: recipient.driven,
            body: contact_body(&recipient.kinematics),
            down: world
                .get::<ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>(
                    recipient.entity,
                )
                .map(|frame| frame.down().to_array()),
            touchers,
            truncated,
            incomplete,
            refusals,
            refusals_truncated,
            world: step.map_or_else(Vec::new, |step| step.events.contacts.iter()
                .take(recipient.limits.max_world_contacts as usize)
                .map(super::geometry::world_contact).collect()),
            world_available: step.is_some(),
            world_truncated: step.is_some_and(|step| step.events.contacts.len()
                > recipient.limits.max_world_contacts as usize),
            world_refusal: step.and_then(|step| step.events.motion_refusal.as_ref())
                .map(|refusal| format!("{refusal:?}")),
        };
        for key in &recipient.keys {
            world
                .resource_mut::<ExtensionInvocations>()
                .trigger::<BodyContactPort>(
                    &BODY_CONTACT,
                    key.clone(),
                    recipient.entity,
                    None,
                    false,
                    contact.clone(),
                );
        }
        if !entities.is_empty() {
            world.resource_mut::<ContactTouchers>().0.push((
                recipient.entity,
                recipient.kinematics.pos,
                entities,
            ));
        }
    }
}

fn surfaces(body: &Sample, toucher: &Sample) -> Result<(Vec<ContactSurface>, bool), String> {
    let mut surfaces = Vec::new();
    for first in &body.pieces {
        for second in &toucher.pieces {
            for point in first.volume.surface_contacts(&second.volume, 0.0) {
                let position = first.chart
                    .uncarry(point.point1)
                    .map_err(|fault| format!("{fault:?}"))?;
                let other = first.chart
                    .uncarry(point.point2)
                    .map_err(|fault| format!("{fault:?}"))?;
                let normal = first.chart
                    .uncarry_dir(point.normal1)
                    .map_err(|fault| format!("{fault:?}"))?
                    .try_normalize()
                    .ok_or_else(|| "degenerate_contact_normal".to_string())?;
                let distance = (other - position).dot(normal);
                if !distance.is_finite() {
                    return Err("non_finite_contact_distance".into());
                }
                surfaces.push(ContactSurface {
                    point: position.to_array(),
                    normal: normal.to_array(),
                    distance,
                });
            }
        }
    }
    surfaces.sort_by(|a, b| {
        a.point[0]
            .total_cmp(&b.point[0])
            .then(a.point[1].total_cmp(&b.point[1]))
            .then(a.normal[0].total_cmp(&b.normal[0]))
            .then(a.normal[1].total_cmp(&b.normal[1]))
            .then(a.distance.total_cmp(&b.distance))
    });
    surfaces.dedup();
    let truncated = surfaces.len() > body.limits.max_surfaces as usize;
    surfaces.truncate(body.limits.max_surfaces as usize);
    Ok((surfaces, truncated))
}

fn volume(piece: &ae::frame::thread::VolumePiece) -> Result<Option<ae::CombatVolume>, String> {
    if piece.source_bounds().is_empty() {
        return Ok(Some(piece.world_volume().clone()));
    }
    let Some(region) = piece.source_region_wide() else {
        return Ok(None);
    };
    let region = region
        .into_iter()
        .map(|point| piece.transform().point_from_wide(point))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|fault| format!("{fault:?}"))?;
    Ok((region.len() >= 3).then(|| ae::CombatVolume::convex(region)))
}

pub fn install_status_requests(app: &mut App) {
    app.install_extension_submitted_request::<ambition_combat_port::HoldStatusPort, _>(
        BODY_CONTACT,
        "ambition_combat",
        super::super::status::lower_held_statuses::<InBodyContact>,
    );
}
