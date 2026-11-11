//! Loose-item motion and contacts are one admitted transaction.

use bevy::prelude::*;
use bevy::math::DVec2;
use ambition_combat::components::DamageableVolumes;
use ambition_platformer2d_core::{self as ae, AabbExt, BodyKinematics, Vec2};
use ae::frame::{NumericFault, Sim2, material_cast::MaterialCast,
    thread::{MaterialPrimitive, MotionAdmission, Refusal, RefusalReason}};
use ambition_platformer2d_shared_tangle::{body::geometry::BodyGeometry,
    lifecycle::LiveRoomInstance, sim_id::SimId};
use super::{GroundItem, ItemBody, ItemCustody, ItemStruckBody, ItemStruckSurface,
    SettledItem, SolidsByRoom, GROUND_ITEM_GRAVITY};

fn numeric(reason: NumericFault) -> Refusal {
    Refusal { path: vec![], link: None, reason: RefusalReason::Numeric(reason) }
}

fn speed(velocity: DVec2) -> Result<f32, Refusal> {
    Sim2::IDENTITY.vector_from_wide(DVec2::new(velocity.length(), 0.0))
        .map(|value| value.x).map_err(numeric)
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct ItemGeometry<'w, 's> {
    #[cfg(feature = "portal")]
    placed: Query<'w, 's, (Entity, &'static ambition_portal2d::PlacedPortal)>,
    #[cfg(feature = "portal")]
    rooms: ambition_portal2d::PortalRooms<'w, 's>,
    #[cfg(feature = "portal")]
    tuning: Option<Res<'w, ambition_portal2d::PortalTuning>>,
    #[cfg(feature = "portal")]
    backside: Query<'w, 's, &'static ambition_portal2d::PortalBackside>,
    geometry: Option<Res<'w, BodyGeometry>>,
    #[cfg(not(feature = "portal"))]
    marker: Local<'s, ()>,
}

type Items<'w, 's> = Query<'w, 's,
    (Entity, &'static mut BodyKinematics, &'static mut ae::SweepSample,
        &'static ItemCustody, Option<&'static ItemBody>, Has<SettledItem>), With<GroundItem>>;
type Recipients<'w, 's> = Query<'w, 's,
    (Entity, &'static ae::CenteredAabb, &'static ambition_characters::actor::BodyHealth,
        Has<ambition_combat::death_rules::OutOfPlay>, Option<&'static ae::DepthPlane>,
        Option<&'static BodyKinematics>, Option<&'static DamageableVolumes>, Option<&'static SimId>)>;

struct Recipient {
    entity: Entity,
    room: Option<LiveRoomInstance>,
    body: Option<BodyKinematics>,
    hurt: DamageableVolumes,
    position: Vec2,
    id: Option<SimId>,
}

struct Leg {
    body: BodyKinematics,
    sample: ae::SweepSample,
    first: Option<u32>,
}

struct Flight {
    body: BodyKinematics,
    legs: Vec<Leg>,
    first: Option<u32>,
    turnable: bool,
    struck: f32,
    settles: bool,
    #[cfg(feature = "portal")]
    transfers: Vec<ambition_portal2d::PreparedPortalTransfer>,
}

/// Stage the complete flight before motion, portal events or impact facts change.
pub fn ground_item_physics(
    time: Res<ambition_time::WorldTime>,
    world: ambition_platformer2d_world::collision::CollisionWorld,
    // Each item flies at its room's pace (ISL-1b): a held room's items wait.
    freeze: ambition_platformer2d_shared_tangle::lifecycle::RoomFreeze,
    gravity: ambition_platformer2d_shared_tangle::gravity::GravityCtx,
    portals: ItemGeometry,
    mut commands: Commands,
    mut populations: ParamSet<(Items, Recipients)>,
) {
    let live = freeze.live();
    let shared_dt = time.sim_dt();
    if shared_dt <= 0.0 { return; }
    let geometry = portals.geometry.as_deref();
    #[cfg(not(feature = "portal"))]
    let _ = &portals.marker;
    #[cfg(feature = "portal")]
    let placed = ambition_portal2d::PortalsByRoom::collect(portals.placed.iter(), &portals.rooms);
    let recipients: Vec<_> = populations.p1().iter().filter(|(_, _, health, out, plane, ..)|
        !ambition_combat::util::body_is_untouchable(Some(*health), *out, *plane))
        .map(|(entity, bounds, _, _, _, body, hurt, id)| Recipient {
            entity, room: live.of(entity), body: body.copied(), position: bounds.aabb().center(),
            hurt: hurt.filter(|hurt| hurt.published()).cloned().unwrap_or_else(||
                body.map_or_else(|| DamageableVolumes::world_box(bounds.aabb()), |body|
                    DamageableVolumes::body_box(body.canonical_size * 0.5))), id: id.cloned(),
        }).collect();
    let mut items = populations.p0();
    let solids = SolidsByRoom::build(&world, items.iter().map(|(entity, ..)| live.of(entity)));
    for (entity, mut item, mut sweep, custody, response, settled) in &mut items {
        let dt = shared_dt * freeze.scale_of(entity);
        if dt <= 0.0 { continue; }
        let before = *item;
        let prior = *sweep;
        *sweep = ae::SweepSample::at_tick_end(before, Vec2::ZERO);
        commands.entity(entity).remove::<(ItemStruckBody, ItemStruckSurface)>();
        if !custody.in_world() { continue; }
        let room = live.of(entity);
        let Some(world) = solids.raw(room) else { continue; };
        let down = gravity.dir_for(room, ae::Aabb::new(before.pos, before.half_extent()));
        let admission = geometry.and_then(|geometry| geometry.motion_for(entity, room));
        let response = response.copied().unwrap_or(ItemBody {
            gravity_scale: 1.0, bounce: 0.0, slide: 0.0, rest_speed: 60.0,
        });
        let prepared = (|| -> Result<_, Refusal> {
            let mut flight = if settled {
                let sample = prior.ending_at(before.pos).filter(|sample| sample.time[0] < sample.time[1])
                    .copied().unwrap_or_else(|| ae::SweepSample::at_rest(before, down));
                let first = admission.and_then(|policy| policy.first);
                let start = BodyKinematics { pos: sample.prev, vel: sample.vel, material: sample.material, ..before };
                Flight { body: before, first, turnable: matches!(sample.travel, ae::LinearTravel::Complete),
                    legs: vec![Leg { body: start, sample, first }], struck: 0.0, settles: false,
                    #[cfg(feature = "portal")] transfers: vec![] }
            } else {
                let mut moving = before;
                ambition_platformer2d_shared_tangle::gravity::apply_world_forces(&mut moving.vel,
                    GROUND_ITEM_GRAVITY * response.gravity_scale,
                    &ambition_platformer2d_shared_tangle::gravity::GravityField { dir: down }, dt);
                prepare_flight(world, moving, down, dt, response, admission,
                    #[cfg(feature = "portal")] placed.in_room(room),
                    #[cfg(feature = "portal")] portals.backside.get(entity).ok().copied(),
                    #[cfg(feature = "portal")] portals.tuning.as_deref(),
                )?
            };
            // Righting can use the last free leg. A contact or a root rebase
            // at tick end leaves no free leg for this turn proposal.
            if flight.turnable {
                if let Some(leg) = flight.legs.last_mut().filter(|leg|
                    leg.sample.curr == flight.body.pos && leg.sample.material == flight.body.material) {
                    let duration = f64::from(dt) * (leg.sample.time[1] - leg.sample.time[0]);
                    if duration > 0.0 {
                        let policy = admission.map(|policy| MotionAdmission { first: leg.first, ..policy });
                        ae::movement::admit_body_righting(&mut flight.body, &mut leg.sample, down,
                            duration * 8.0, duration, world, policy, &[])?;
                    }
                }
            }
            let policy = admission.map(|policy| MotionAdmission { first: flight.first, ..policy });
            let pose = ae::movement::prepare_body_pose(&flight.body, None, flight.body, policy, &[])?;
            pose.commit(&mut flight.body, None);
            let record = if settled { flight.legs.last().map(|leg| leg.sample)
                .unwrap_or_else(|| ae::SweepSample::at_tick_end(flight.body, down)) }
                else { ae::SweepSample::at_tick_end(flight.body, down) };
            let mut contacts = Vec::new();
            for recipient in recipients.iter().filter(|recipient| recipient.entity != entity && recipient.room == room) {
                for leg in &flight.legs {
                    let policy = admission.map(|policy| MotionAdmission { first: leg.first, ..policy });
                    let domain = leg.body.canonical_box().map_err(numeric)?;
                    let cells = leg.sample.material_cast(&domain)?.cells(policy)?;
                    let hit = recipient.hurt.swept_body_contact(recipient.entity, recipient.body.as_ref(),
                        geometry, room, &cells)?;
                    let Some(hit) = hit else { continue; };
                    // A released item can start inside its holder. Only a new
                    // entry into each recipient produces an impact fact.
                    if hit.hit.toi == 0.0 { continue; }
                    let physical = cells.contact_velocity_frame(&hit.hit)?.to_physical(leg.sample.vel).map_err(numeric)?;
                    contacts.push((hit.hit.world_time, recipient, speed(physical.as_dvec2())?));
                    break;
                }
            }
            contacts.sort_by(|(at, a, _), (bt, b, _)| at.total_cmp(bt)
                .then(a.position.x.total_cmp(&b.position.x)).then(a.position.y.total_cmp(&b.position.y))
                .then_with(|| ambition_combat::hitbox::victim_identity_key(a.id.as_ref().map(SimId::as_str))
                    .cmp(&ambition_combat::hitbox::victim_identity_key(b.id.as_ref().map(SimId::as_str)))));
            Ok((flight, record, contacts))
        })();
        let (flight, record, contacts) = match prepared {
            Ok(prepared) => prepared,
            Err(refusal) => {
                bevy::log::debug!(target: "ambition_portal::refusal", body=?entity, ?refusal,
                    "loose item motion refused before commit");
                continue;
            }
        };
        #[cfg(feature = "portal")]
        for transfer in flight.transfers {
            ambition_portal2d::commit_body_transfer(&mut commands, entity, &mut item, None, None,
                transfer, portals.tuning.as_deref().expect("an admitted crossing has tuning").teleport_cooldown_s);
        }
        *item = flight.body;
        // Free-flight legs already produced contacts and root transfers.
        // A support carry keeps its path for the shared transit phase.
        *sweep = record;
        if !contacts.is_empty() {
            commands.entity(entity).insert(ItemStruckBody {
                impact_speed: contacts.iter().map(|(_, _, speed)| *speed).fold(0.0, f32::max),
                bodies: contacts.iter().map(|(_, recipient, _)| recipient.entity).collect(),
            });
        }
        if flight.struck > 0.0 {
            commands.entity(entity).insert(ItemStruckSurface { impact_speed: flight.struck });
        }
        if flight.settles { commands.entity(entity).insert(SettledItem { impact_speed: flight.struck }); }
    }
}

#[allow(clippy::too_many_arguments)]
fn prepare_flight(world: &ae::World, mut body: BodyKinematics, down: Vec2, dt: f32,
    response: ItemBody, admission: Option<MotionAdmission<'_>>,
    #[cfg(feature = "portal")] portals: &[ambition_portal2d::PlacedPortal],
    #[cfg(feature = "portal")] backside: Option<ambition_portal2d::PortalBackside>,
    #[cfg(feature = "portal")] tuning: Option<&ambition_portal2d::PortalTuning>,
) -> Result<Flight, Refusal> {
    if [dt, response.gravity_scale, response.bounce, response.slide, response.rest_speed]
        .iter().any(|value| !value.is_finite()) || !body.vel.is_finite() || !down.is_finite() {
        return Err(numeric(NumericFault::NonFinite));
    }
    if dt <= 0.0 || response.bounce < 0.0 || response.slide < 0.0 || response.rest_speed < 0.0 {
        return Err(numeric(NumericFault::InvalidScale));
    }
    let mut result = Flight { body, legs: vec![], first: admission.and_then(|policy| policy.first), turnable: false, struck: 0.0, settles: false,
        #[cfg(feature = "portal")] transfers: vec![] };
    let mut time = 0.0;
    for _ in 0..64 {
        let policy = admission.map(|policy| MotionAdmission { first: result.first, ..policy });
        let domain = body.canonical_box().map_err(numeric)?;
        let delta = Sim2::IDENTITY.vector_from_wide(body.vel.as_dvec2() * f64::from(dt) * (1.0 - time)).map_err(numeric)?;
        let cast = MaterialCast::new(MaterialPrimitive { domain: &domain,
            to_world: body.material_map().map_err(numeric)? }, body.pos, delta, time..1.0);
        #[cfg(feature = "portal")]
        let crossing = ambition_portal2d::first_portal_crossing_through(&cast, portals,
            if result.transfers.is_empty() { backside } else { None }, 1.0)
            .map_err(|refusal| Refusal { path: vec![], link: None, reason: refusal.reason })?;
        #[cfg(feature = "portal")]
        let until = crossing.map_or(1.0, |crossing| crossing.root.toi);
        #[cfg(not(feature = "portal"))]
        let until = 1.0;
        let polygon = domain.polygon().expect("a canonical box is a polygon");
        let contact = world.first_material_sweep_prefix(cast.clone(), until, policy, |collider, cell, hit| {
            if collider.clears_material_contact(cell, &polygon, hit, 1.0 - time).map_err(numeric)? { return Ok(false); }
            use ae::world::MaterialCollider;
            let one_way = match collider {
                MaterialCollider::Block(block) => {
                    if !ae::collision_semantics::is_support_surface(block.kind) { return Ok(false); }
                    block.kind == ae::BlockKind::OneWay
                }
                MaterialCollider::Chain { chain, segment, .. } => {
                    if chain.kind == ae::world::SurfaceKind::Ground {
                        if hit.map.direction(hit.normal).dot(chain.normal_at_time(segment, hit.world_time).map_err(numeric)?) < 0.5 {
                            return Ok(false);
                        }
                        true
                    } else { false }
                }
            };
            let surface = collider.velocity_in_root(hit, dt, policy)?;
            let approaching = (body.vel.as_dvec2() - surface.as_dvec2()).dot(hit.normal.as_dvec2()) < 0.0;
            Ok(!one_way || approaching && hit.normal.dot(down) < -0.5)
        })?;
        result.turnable = contact.is_none() && until == 1.0;
        let end = contact.as_ref().map_or(until, |hit| hit.contact.toi);
        let cells = cast.prefix(end)?.cells(policy)?;
        cells.admit_payloads(body.vel, &[])?;
        let start = body;
        body = BodyKinematics { pos: Sim2::IDENTITY.point_from_wide(start.pos.as_dvec2() + delta.as_dvec2() * end).map_err(numeric)?, ..body };
        let sample = ae::SweepSample { curr: body.pos, vel: start.vel, time: [time, 1.0],
            travel: ae::LinearTravel::Prefix { delta, until: end }, ..ae::SweepSample::at_rest(start, down) };
        result.legs.push(Leg { body: start, sample, first: result.first });
        let next_time = time + (1.0 - time) * end;
        if let Some(hit) = contact {
            let surface = hit.collider.velocity_in_root(&hit.contact, dt, policy)?;
            let normal = hit.contact.normal.as_dvec2();
            let relative = body.vel.as_dvec2() - surface.as_dvec2();
            let into = relative.dot(normal);
            if into >= 0.0 { return Err(numeric(NumericFault::InvalidFrame)); }
            let impact = speed(hit.contact.map.vector_from_wide(normal * -into).map_err(numeric)?.as_dvec2())?;
            result.struck = result.struck.max(impact);
            body = BodyKinematics { vel: Sim2::IDENTITY.vector_from_wide((relative - normal * into) * f64::from(response.slide)
                - normal * into * f64::from(response.bounce) + surface.as_dvec2()).map_err(numeric)?, ..body };
            cells.admit_payloads_at(end, body.vel, &[])?;
            if hit.contact.normal.dot(down) < -0.5 && (body.vel.as_dvec2() - surface.as_dvec2()).length()
                < f64::from(body.material.at(Vec2::ZERO).map_err(numeric)?.length(response.rest_speed).map_err(numeric)?) {
                body = BodyKinematics { vel: surface, ..body };
                result.settles = true;
            }
        } else {
            #[cfg(feature = "portal")]
            if let Some(crossing) = crossing {
                let tuning = tuning.ok_or_else(|| numeric(NumericFault::InvalidFrame))?;
                let (transfer, reverse) = ambition_portal2d::prepare_body_crossing(body, None, crossing,
                    portals, policy, result.transfers.len() as u8, tuning)?;
                body = *transfer.body();
                result.transfers.push(transfer);
                result.first = Some(reverse);
            } else { result.body = body; return Ok(result); }
            #[cfg(not(feature = "portal"))]
            { result.body = body; return Ok(result); }
        }
        time = next_time;
        if time == 1.0 { result.body = body; return Ok(result); }
    }
    Err(Refusal { path: vec![], link: None, reason: RefusalReason::WorkLimit })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response() -> ItemBody {
        ItemBody { gravity_scale: 0.0, bounce: 1.0, slide: 1.0, rest_speed: 0.0 }
    }

    fn flight(world: &ae::World, body: BodyKinematics, dt: f32) -> Flight {
        prepare_flight(world, body, Vec2::Y, dt, response(), None,
            #[cfg(feature = "portal")] &[],
            #[cfg(feature = "portal")] None,
            #[cfg(feature = "portal")] None,
        ).unwrap()
    }

    #[test]
    fn a_fast_item_uses_the_time_left_after_two_thin_wall_contacts() {
        let world = ae::World::new("corridor", Vec2::splat(1000.0), Vec2::ZERO, vec![
            ae::Block::solid("left", Vec2::new(0.0, 0.0), Vec2::new(1.0, 200.0)),
            ae::Block::solid("right", Vec2::new(100.0, 0.0), Vec2::new(1.0, 200.0)),
        ]);
        let body = BodyKinematics { pos: Vec2::new(50.0, 100.0), vel: Vec2::X * 3000.0,
            canonical_size: Vec2::splat(4.0), ..Default::default() };
        let result = flight(&world, body, 0.05);
        assert_eq!(result.legs.len(), 3);
        assert!((result.body.pos.x - 10.0).abs() < 0.001, "{:?}", result.body);
        assert_eq!(result.body.vel, body.vel);
        assert_eq!(result.struck, 3000.0);
        assert!(!result.settles);
    }

    #[cfg(feature = "portal")]
    #[test]
    fn a_scaled_reflected_item_bounces_in_the_exit_chart_and_crosses_back() {
        use ambition_portal2d::{PlacedPortal, PortalChannel, PortalChannelColor, PortalTuning};
        let portals = [
            PlacedPortal::fixed(PortalChannel::Authored(PortalChannelColor::Purple),
                Vec2::new(100.0, 100.0), -Vec2::X, 40.0).unwrap(),
            PlacedPortal::fixed(PortalChannel::Authored(PortalChannelColor::Yellow),
                Vec2::new(500.0, 100.0), Vec2::X, 20.0).unwrap(),
        ];
        let mut geometry = BodyGeometry::new(ae::frame::MapConvention::Reflection);
        geometry.add_pair(None, (portals[0].aperture(), portals[1].aperture()));
        let owner = Entity::PLACEHOLDER;
        let policy = geometry.motion_for(owner, None);
        let tuning = PortalTuning { min_exit_speed: 0.0, convention: ambition_portal2d::PortalConvention::Reflection, ..Default::default() };
        let world = ae::World::new("exit wall", Vec2::splat(1000.0), Vec2::ZERO,
            vec![ae::Block::solid("wall", Vec2::new(530.0, 0.0), Vec2::new(1.0, 200.0))]);
        let body = BodyKinematics { pos: Vec2::new(80.0, 100.0), vel: Vec2::X * 2000.0,
            canonical_size: Vec2::new(4.0, 8.0), ..Default::default() };
        let run = || prepare_flight(&world, body, Vec2::Y, 0.05,
            ItemBody { bounce: 0.5, ..response() }, policy, &portals, None, Some(&tuning)).unwrap();
        let result = run();
        assert_eq!(result.transfers.len(), 1);
        assert_eq!(result.legs.len(), 3);
        assert_eq!(result.body.material.scale(), 0.5);
        assert!(result.body.material.reflected());
        assert_eq!(result.body.canonical_size, body.canonical_size);
        assert!((result.body.pos.x - 523.5).abs() < 0.001, "{:?}", result.body);
        assert_eq!(result.body.vel, -Vec2::X * 500.0);
        assert_eq!(run().body, result.body);
        let returning = BodyKinematics { vel: -Vec2::X * 2000.0, ..result.body };
        let reverse_policy = policy.map(|policy| MotionAdmission { first: Some(1), ..policy });
        let back = prepare_flight(&world, returning, Vec2::Y, 0.02, response(),
            reverse_policy, &portals, None, Some(&tuning)).unwrap();
        assert_eq!(back.transfers.len(), 1);
        assert_eq!(back.body.material, body.material);
        assert!((back.body.pos.x - 67.0).abs() < 0.001, "{:?}", back.body);
        assert_eq!(back.body.vel, -Vec2::X * 4000.0);
    }

    #[cfg(feature = "portal")]
    #[test]
    fn a_refused_recipient_prevents_item_motion_and_portal_events_until_repaired() {
        use ambition_portal2d::{PlacedPortal, PortalChannel, PortalChannelColor, PortalTuning,
            PortalBodyTransited};
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, insert_live_room_component};
        use ae::frame::thread::PieceBudget;
        let mut app = App::new();
        let room = LiveRoomInstance::ACTIVATION;
        insert_live_room_component(app.world_mut(), ae::RoomGeometry(ae::World::new(
            "item admission", Vec2::splat(1000.0), Vec2::ZERO, vec![])));
        app.insert_resource(ambition_time::WorldTime::new(0.05, 0.05));
        app.insert_resource(PortalTuning { min_exit_speed: 0.0,
            convention: ambition_portal2d::PortalConvention::Reflection, ..Default::default() });
        app.add_message::<PortalBodyTransited>();
        app.add_systems(Update, ground_item_physics);
        let portals = [
            PlacedPortal::fixed(PortalChannel::Authored(PortalChannelColor::Purple),
                Vec2::new(100.0, 100.0), -Vec2::X, 40.0).unwrap(),
            PlacedPortal::fixed(PortalChannel::Authored(PortalChannelColor::Yellow),
                Vec2::new(500.0, 100.0), Vec2::X, 20.0).unwrap(),
        ];
        let mut geometry = BodyGeometry::new(ae::frame::MapConvention::Reflection);
        geometry.add_pair(Some(room), (portals[0].aperture(), portals[1].aperture()));
        for portal in portals { app.world_mut().spawn((portal, InRoomInstance(room))); }
        let item = app.world_mut().spawn((GroundItem::released(super::super::axe_spec(),
            Vec2::new(80.0, 100.0), Vec2::X * 2000.0, Vec2::new(2.0, 4.0)),
            response(), InRoomInstance(room))).id();
        let victim_pose = BodyKinematics { pos: Vec2::new(515.0, 100.0),
            canonical_size: Vec2::splat(4.0), ..Default::default() };
        let victim = app.world_mut().spawn((victim_pose,
            ae::CenteredAabb::new(victim_pose.pos, Vec2::splat(2.0)),
            ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(100)),
            DamageableVolumes::body_box(Vec2::splat(2.0)), InRoomInstance(room))).id();
        geometry.set_body_budget(victim, PieceBudget { max_pieces: 0, ..PieceBudget::COMPOSED });
        app.insert_resource(geometry);
        let before = *app.world().get::<BodyKinematics>(item).unwrap();
        app.update();
        assert_eq!(*app.world().get::<BodyKinematics>(item).unwrap(), before);
        assert!(app.world().get::<ItemStruckBody>(item).is_none());
        assert!(app.world().resource::<Messages<PortalBodyTransited>>().is_empty());
        app.world_mut().resource_mut::<BodyGeometry>().set_body_budget(victim, PieceBudget::COMPOSED);
        app.update();
        let after = app.world().get::<BodyKinematics>(item).unwrap();
        assert_eq!(after.material.scale(), 0.5);
        assert!(after.material.reflected());
        assert_eq!(after.pos, Vec2::new(540.0, 100.0));
        assert_eq!(after.vel, Vec2::X * 1000.0);
        let sample = app.world().get::<ae::SweepSample>(item).unwrap();
        assert_eq!(sample.time, [1.0, 1.0]);
        assert!(sample.turn.is_none(), "the admitted flight is already consumed");
        assert!(after.material.orthogonal().x_axis.y.abs() > 0.1,
            "righting uses the free leg in the new root chart");
        let impact = app.world().get::<ItemStruckBody>(item).unwrap();
        assert_eq!(impact.bodies, vec![victim]);
        let spin_speed = 8.0 * (before.canonical_size * 0.5).length() * after.material.scale();
        assert!((impact.impact_speed - 1000.0).abs() <= spin_speed,
            "the physical contact speed includes the bounded righting motor: {}", impact.impact_speed);
        assert_eq!(app.world().resource::<Messages<PortalBodyTransited>>().len(), 1);
    }
}
