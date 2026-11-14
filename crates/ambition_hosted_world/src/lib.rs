//! A hosted world's replication: the host's bodies reach a guest App by
//! `SimId` (NET-AUTHORITY-EXPERIMENT, `engine/hosted-world-networking-experiment.md`).
//!
//! Both Apps build a room's bodies themselves, from the same definition, so a
//! body has the same `SimId` on both. The host marks each body for replication
//! with a [`Signature`] of its `SimId`, and the guest puts the same signature on
//! the body it built: replication continues into that body instead of making a
//! second one. What the host sends is the body row (NA-2): where the body is,
//! how it moves and which way it faces; and its life row (NA-2b): its health
//! and its death's beat, which is what draws a defeat.
//!
//! A body the guest did not build (one the host spawned, or one of a room the
//! guest has not built) arrives with its spawn row ([`SpawnRow`]): its
//! `SimId`, feature id, name, character and size. The guest builds it through
//! the engine's staged-actor road into the replicated entity, so its rows land
//! on it like on any other body. Its brain does not run there: the guest
//! simulates nothing.
//!
//! A guest is sent the bodies of its player's room only (NA-2b's interest):
//! each body and each guest's client entity carry the live room they are in
//! ([`RoomOf`], [`GuestView`]), and replicon's visibility filter hides a body
//! from a client in another room, except the client's own player's body. Leaving a guest's view removes the body there, which is
//! the general rule the measurement used.
//!
//! A guest follows its own player's crossing on the host's word: each body
//! carries the definition of the room it is in ([`RoomNamed`]), and when the
//! guest's own body is named into a room it does not stand in, the guest
//! records that crossing in its own room lifecycle, which builds the room.
//! It then tells the host which room it holds ([`GuestRoom`]), and only then
//! does the host move the guest's view there: the room's bodies arrive after
//! the guest built its own copies, so replication continues into them
//! instead of making a second of each.
//!
//! The guest's input goes the other way: each frame it sends the raw control
//! frame of its own seat ([`LocalSeat`]) as a [`GuestInput`] message, on the
//! reliable ordered channel so no press is lost, and the host drives the seat
//! the sender's [`GuestSeat`] names with it, through the same seam a local
//! device or a test driver uses (`drive_slot_frame`).
//!
//! The link is the caller's: `bevy_replicon` has no I/O of its own.

use ambition_characters::actor::{BodyHealth, Corpse};
use ambition_combat::components::DeathBeat;
use ambition_platformer2d_core::BodyKinematics;
use ambition_characters::actor::{ActorFaction, WornCharacter};
use ambition_characters::control::{DrivingParticipant, PlayerSlot};
use ambition_combat::components::FeatureId;
use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

/// Where a body is, how it moves and which way it faces: what a guest needs to
/// draw it where the host has it (NA-2's 5 floats).
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyRow {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub facing: f32,
}

impl BodyRow {
    pub fn of(body: &BodyKinematics) -> Self {
        Self { pos: body.pos.to_array(), vel: body.vel.to_array(), facing: body.facing }
    }

    pub fn write_into(&self, body: &mut BodyKinematics) {
        body.pos = Vec2::from_array(self.pos);
        body.vel = Vec2::from_array(self.vel);
        body.facing = self.facing;
    }
}

/// A body's life: its health, and its death's beat (seconds since it died,
/// whether it lived, what it leaves). The guest simulates no death, so it
/// draws one from this (NA-2b: without the beat, never).
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LifeRow {
    pub current: i32,
    pub since_s: Option<f32>,
    pub lived: bool,
    pub corpse: u8,
}

impl LifeRow {
    pub fn of(health: &BodyHealth, beat: Option<&DeathBeat>) -> Self {
        Self {
            current: health.health.current,
            since_s: beat.and_then(|beat| beat.since_s),
            lived: beat.is_some_and(|beat| beat.lived),
            corpse: beat.map_or(0, |beat| corpse_byte(beat.corpse)),
        }
    }

    pub fn write_into(&self, health: &mut BodyHealth, beat: Option<&mut DeathBeat>) {
        health.health.current = self.current;
        if let Some(beat) = beat {
            beat.since_s = self.since_s;
            beat.lived = self.lived;
            beat.corpse = corpse_of(self.corpse);
        }
    }
}

fn corpse_byte(corpse: Corpse) -> u8 {
    match corpse {
        Corpse::Stay => 0,
        Corpse::Vanish => 1,
        Corpse::Gone => 2,
    }
}

fn corpse_of(byte: u8) -> Corpse {
    match byte {
        0 => Corpse::Stay,
        2 => Corpse::Gone,
        _ => Corpse::Vanish,
    }
}

/// What a guest needs to build a body it does not hold: its identity, the
/// character it wears and its size. Written once, when the body is marked; a
/// player's body is not built from one (the guest's own seats build theirs).
#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpawnRow {
    pub sim_id: String,
    pub feature: String,
    pub name: String,
    pub character: String,
    pub half_size: [f32; 2],
    pub npc: bool,
}

/// On the guest: a replicated entity its spawn row has been built into (or
/// refused), so it is not built twice.
#[derive(Component, Clone, Copy, Debug)]
pub struct BuiltFromSpawnRow;

/// The live room (its ordinal) a body is in, and the seat that drives it if
/// any. Immutable, as replicon's filters require: a move re-inserts it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[component(immutable)]
pub struct RoomOf {
    pub room: u32,
    pub seat: Option<PlayerSlot>,
}

/// On the host, a guest's view: the room it is sent, and its own seat.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[component(immutable)]
pub struct GuestView {
    pub room: u32,
    pub seat: PlayerSlot,
}

impl VisibilityFilter for RoomOf {
    type ClientComponent = GuestView;
    type Scope = Entity;

    /// A body is visible to a client in its room, to the client whose seat
    /// drives it (a guest never loses its own player, even while its view
    /// waits for the room it crossed into), and to a client with no view yet.
    fn is_visible(&self, _client: Entity, view: Option<&GuestView>) -> bool {
        view.is_none_or(|view| view.room == self.room || self.seat == Some(view.seat))
    }
}

/// On the host, the seat a connected guest drives: put on the guest's client
/// entity by whoever accepted the connection. The guest's room is this
/// seat's body's room.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuestSeat(pub PlayerSlot);

/// The definition of the room a body is in, as the guest's player's body
/// carries it to the guest.
#[derive(Component, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomNamed(pub String);

/// From a guest: the definition of the room its own player's body stands in
/// there. The host aims the guest's view at a room only once it holds it.
#[derive(Message, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuestRoom(pub String);

/// On the host: the room a guest last said it holds.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct GuestHolds(pub String);

/// On the guest: the seat this App's player drives, whose input it sends.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalSeat(pub PlayerSlot);

/// One frame of a guest's input for its seat.
#[derive(Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GuestInput(pub ambition_platformer2d_core::ControlFrame);

/// Which end of a hosted world this App is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostedRole {
    /// Owns the world: simulates it and sends its bodies.
    Host,
    /// Shows the host's world: applies the bodies it is sent.
    Guest,
}

/// The replication of a hosted world's bodies. Add it with `RepliconPlugins`
/// (and a link) to both Apps.
pub struct HostedWorldPlugin {
    pub role: HostedRole,
}

impl Plugin for HostedWorldPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<BodyRow>()
            .replicate::<LifeRow>()
            .replicate::<SpawnRow>()
            .replicate::<RoomNamed>()
            .add_client_message::<GuestInput>(Channel::Ordered)
            .add_client_message::<GuestRoom>(Channel::Ordered);
        match self.role {
            HostedRole::Host => {
                app.add_visibility_filter::<RoomOf>();
                app.add_systems(PreUpdate, (apply_guest_inputs, note_guest_rooms).after(ServerSystems::Receive));
                app.add_systems(Update, (mark_hosted_bodies, write_spawn_rows));
                app.add_systems(
                    PostUpdate,
                    (write_body_rows, write_life_rows, place_bodies, name_body_rooms, place_guests).before(ServerSystems::Send),
                );
            }
            HostedRole::Guest => {
                // Signed before each receive as well: a body the guest's own
                // room construction made late in a frame must carry its
                // signature before the host's copy of it arrives.
                app.add_systems(Update, sign_own_bodies);
                app.add_systems(PreUpdate, sign_own_bodies.before(ClientSystems::Receive));
                app.add_systems(PostUpdate, (send_local_input, report_own_room).before(ClientSystems::Send));
                app.add_systems(PreUpdate, follow_own_crossing.after(ClientSystems::Receive));
                app.add_systems(
                    PreUpdate,
                    (build_spawned_bodies, (apply_body_rows, apply_life_rows))
                        .chain()
                        .after(ClientSystems::Receive),
                );
            }
        }
    }
}

/// A body is replicated under the signature of its `SimId`.
fn signature() -> Signature {
    Signature::of::<SimId>()
}

/// Mark each new body of the host's world for replication.
fn mark_hosted_bodies(
    mut commands: Commands,
    bodies: Query<Entity, (With<SimId>, With<BodyKinematics>, Without<Replicated>)>,
) {
    for body in &bodies {
        commands.entity(body).insert((Replicated, signature()));
    }
}

/// Write the spawn row of each replicated body that is not a player's.
fn write_spawn_rows(
    mut commands: Commands,
    bodies: Query<
        (Entity, &SimId, &FeatureId, Option<&Name>, &WornCharacter, &BodyKinematics, Option<&ActorFaction>),
        (With<Replicated>, Without<SpawnRow>, Without<DrivingParticipant>),
    >,
) {
    for (body, id, feature, name, worn, kinematics, faction) in &bodies {
        let half = kinematics.canonical_size * 0.5;
        commands.entity(body).insert(SpawnRow {
            sim_id: id.as_str().to_owned(),
            feature: feature.0.clone(),
            name: name.map_or_else(|| feature.0.clone(), |name| name.as_str().to_owned()),
            character: worn.0.as_str().to_owned(),
            half_size: [half.x, half.y],
            npc: faction == Some(&ActorFaction::Npc),
        });
    }
}

/// Build each replicated body the guest does not hold from its spawn row,
/// into the replicated entity, through the engine's staged-actor road.
#[allow(clippy::too_many_arguments)]
fn build_spawned_bodies(
    mut commands: Commands,
    rows: Query<(Entity, &SpawnRow, Option<&BodyRow>), (With<Remote>, Without<BodyKinematics>, Without<BuiltFromSpawnRow>)>,
    sprite_content: Res<ambition_sprite_sheet::SpriteContent>,
    character_catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    authored_sheets: Res<ambition_sprite_sheet::character::sheets::AuthoredSheets>,
    prepared: ambition_characters::prepared::SessionCast,
    boss_catalog: Res<ambition_boss_encounter::BossCatalog>,
    active_session: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
) {
    let Some(session) =
        ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    let empty_cast = ambition_characters::prepared::PreparedCharacterRegistry::default();
    for (root, row, at) in &rows {
        let request = ambition_platformer2d_actor_spawn::SpawnActorRequest {
            id: row.feature.clone(),
            name: row.name.clone(),
            pos: at.map_or(Vec2::ZERO, |at| Vec2::from(at.pos)),
            half_size: Vec2::from(row.half_size),
            faction: if row.npc { ActorFaction::Npc } else { ActorFaction::Enemy },
            grudge_against: None,
            kind: ambition_platformer2d_actor_spawn::SpawnActorKind::Enemy {
                brain: ambition_entity_catalog::placements::CharacterBrain::Passive,
                character: ambition_entity_catalog::CharacterId::from(row.character.as_str()),
            },
        };
        ambition_platformer2d_actor_spawn::spawn_staged_actor_into(
            &sprite_content,
            &mut ambition_platformer2d_shared_tangle::construction::RootScope::new(&mut commands, session, root),
            &character_catalog,
            &authored_sheets,
            prepared.get().unwrap_or(&empty_cast),
            &boss_catalog,
            &request,
            ambition_platformer2d_actor_spawn::RecordedFate::AsAuthored,
        );
        commands
            .entity(root)
            .insert((SimId::from_snapshot(row.sim_id.clone()), BuiltFromSpawnRow));
    }
}

/// The guest signs the bodies it built, so the host's replication continues
/// into them.
fn sign_own_bodies(
    mut commands: Commands,
    bodies: Query<Entity, (With<SimId>, With<BodyKinematics>, Without<Signature>, Without<Remote>)>,
) {
    for body in &bodies {
        commands.entity(body).insert(signature());
    }
}

/// Write each replicated body's row from its kinematics, before the send.
fn write_body_rows(
    mut commands: Commands,
    mut bodies: Query<(Entity, &BodyKinematics, Option<&mut BodyRow>), With<Replicated>>,
) {
    for (entity, body, row) in &mut bodies {
        let now = BodyRow::of(body);
        match row {
            Some(mut row) => {
                row.set_if_neq(now);
            }
            None => {
                commands.entity(entity).insert(now);
            }
        }
    }
}

/// Name each replicated body's room definition on it, when it changes.
fn name_body_rooms(
    mut commands: Commands,
    bodies: Query<(Entity, Option<&RoomNamed>), (With<Replicated>, With<InRoomInstance>)>,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
) {
    for (body, named) in &bodies {
        let Some(spec) = rooms.spec_of(body) else {
            continue;
        };
        if named.is_none_or(|named| named.0 != spec.id) {
            commands.entity(body).insert(RoomNamed(spec.id.clone()));
        }
    }
}

/// Keep the room each guest last said it holds.
fn note_guest_rooms(mut commands: Commands, mut said: MessageReader<FromClient<GuestRoom>>) {
    for room in said.read() {
        if let Some(client) = room.client_id.entity() {
            commands.entity(client).insert(GuestHolds(room.message.0.clone()));
        }
    }
}

/// Tell the host the room this guest's own body stands in, when it changes,
/// once every identified body the guest built there carries its signature
/// (so the host's copies continue into them rather than arriving beside
/// them).
fn report_own_room(
    seat: Option<Res<LocalSeat>>,
    bodies: Query<(Entity, &DrivingParticipant, &InRoomInstance)>,
    unsigned: Query<&InRoomInstance, (With<SimId>, With<BodyKinematics>, Without<Signature>, Without<Remote>)>,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    mut said: Local<Option<String>>,
    mut reports: MessageWriter<GuestRoom>,
) {
    let Some(seat) = seat else {
        return;
    };
    let Some((body, room)) = bodies.iter().find(|(_, driver, _)| driver.0 == seat.0).map(|(body, _, room)| (body, room.0)) else {
        return;
    };
    let Some(here) = rooms.spec_of(body).map(|spec| spec.id.clone()) else {
        return;
    };
    if said.as_ref() == Some(&here) || unsigned.iter().any(|at| at.0 == room) {
        return;
    }
    reports.write(GuestRoom(here.clone()));
    *said = Some(here);
}

/// When the host names this guest's own body into a room it does not stand
/// in here, record that crossing in the guest's room lifecycle, at the
/// body's row: the guest builds the room as a door would have. Recorded once
/// for each room named.
fn follow_own_crossing(world: &mut World, mut asked: Local<Option<String>>) {
    use ambition_platformer2d_session::lifecycle_commit::{
        BodyTransferRequirement, LifecycleIntent, PendingLifecycleCommit, RoomArrival, RoomBodyTransfer, RoomTransitionIntent,
    };
    let Some(seat) = world.get_resource::<LocalSeat>().copied() else {
        return;
    };
    let Some((body, named, at)) = world
        .query::<(Entity, &DrivingParticipant, &RoomNamed, &BodyRow)>()
        .iter(world)
        .find(|(_, driver, ..)| driver.0 == seat.0)
        .map(|(body, _, named, row)| (body, named.0.clone(), Vec2::from(row.pos)))
    else {
        return;
    };
    let here = {
        let mut state = bevy::ecs::system::SystemState::<ambition_platformer2d_world::rooms::LiveRoomSpecs>::new(world);
        let Ok(rooms) = state.get(world) else {
            return;
        };
        rooms.spec_of(body).map(|spec| spec.id.clone())
    };
    if here.as_ref() == Some(&named) || asked.as_ref() == Some(&named) {
        return;
    }
    let Some(subject) = ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::of_entity(world, body) else {
        return;
    };
    let frame = world
        .get_resource::<ambition_platformer2d_core::ConfirmedFrameBoundary>()
        .map_or(0, |boundary| boundary.current);
    let Some(mut pending) = world.get_resource_mut::<PendingLifecycleCommit>() else {
        return;
    };
    let admitted = pending
        .record(
            frame,
            LifecycleIntent::Transition(RoomTransitionIntent {
                disposition: Default::default(),
                source_zone: None,
                initiator: RoomBodyTransfer {
                    requirement: BodyTransferRequirement::InPlay,
                    subject,
                    arrival: RoomArrival::At(at),
                    participant: Some(seat.0),
                },
                companions: Vec::new(),
                target_room: named.clone().into(),
                edge_exit: false,
                zone_sfx: None,
            }),
        )
        .admitted();
    if admitted {
        *asked = Some(named);
    }
}

/// Drive each guest's seat with the input it sent, in the order sent.
fn apply_guest_inputs(world: &mut World) {
    let received: Vec<FromClient<GuestInput>> = world.resource_mut::<Messages<FromClient<GuestInput>>>().drain().collect();
    for input in received {
        let Some(seat) = input.client_id.entity().and_then(|client| world.get::<GuestSeat>(client)).copied() else {
            continue;
        };
        ambition_platformer2d_runtime::input_drive::drive_slot_frame(world, seat.0, input.message.0);
    }
}

/// Send this guest's seat's raw frame to the host.
fn send_local_input(
    seat: Option<Res<LocalSeat>>,
    raw: Option<Res<ambition_characters::control::SeatRawFrames>>,
    mut inputs: MessageWriter<GuestInput>,
) {
    if let (Some(seat), Some(raw)) = (seat, raw) {
        inputs.write(GuestInput(raw.get(seat.0)));
    }
}

/// Put each replicated body's room on it, when it moves.
fn place_bodies(
    mut commands: Commands,
    bodies: Query<(Entity, &InRoomInstance, Option<&DrivingParticipant>, Option<&RoomOf>), With<Replicated>>,
) {
    for (body, room, driver, placed) in &bodies {
        let now = RoomOf { room: room.0.ordinal(), seat: driver.map(|driver| driver.0) };
        if placed != Some(&now) {
            commands.entity(body).insert(now);
        }
    }
}

/// Put each guest's room on its client entity: its seat's body's room, once
/// the guest says it holds a room of that definition (a guest that has said
/// nothing yet is taken at the room it booted in). Until then the view stays
/// where it was.
fn place_guests(
    mut commands: Commands,
    guests: Query<(Entity, &GuestSeat, Option<&GuestView>, Option<&GuestHolds>)>,
    drivers: Query<(Entity, &DrivingParticipant, &InRoomInstance)>,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
) {
    for (client, seat, placed, holds) in &guests {
        let Some((body, _, room)) = drivers.iter().find(|(_, driver, _)| driver.0 == seat.0) else {
            continue;
        };
        let held = holds.is_none_or(|holds| rooms.spec_of(body).is_some_and(|spec| spec.id == holds.0));
        if !held {
            continue;
        }
        let now = GuestView { room: room.0.ordinal(), seat: seat.0 };
        if placed != Some(&now) {
            commands.entity(client).insert(now);
        }
    }
}

/// Write each replicated body's life row, before the send.
fn write_life_rows(
    mut commands: Commands,
    mut bodies: Query<(Entity, &BodyHealth, Option<&DeathBeat>, Option<&mut LifeRow>), With<Replicated>>,
) {
    for (entity, health, beat, row) in &mut bodies {
        let now = LifeRow::of(health, beat);
        match row {
            Some(mut row) => {
                row.set_if_neq(now);
            }
            None => {
                commands.entity(entity).insert(now);
            }
        }
    }
}

/// Put each received life row on the guest's body.
fn apply_life_rows(
    mut bodies: Query<(&LifeRow, &mut BodyHealth, Option<&mut DeathBeat>), Or<(Changed<LifeRow>, Added<BodyHealth>)>>,
) {
    for (row, mut health, beat) in &mut bodies {
        row.write_into(&mut health, beat.map(|beat| beat.into_inner()));
    }
}

/// Put each received row on the guest's body.
fn apply_body_rows(mut bodies: Query<(&BodyRow, &mut BodyKinematics), Or<(Changed<BodyRow>, Added<BodyKinematics>)>>) {
    for (row, mut body) in &mut bodies {
        row.write_into(&mut body);
    }
}

/// `bevy_replicon` itself, so a caller names the version this crate replicates with.
pub use bevy_replicon;

/// A real link: renet's netcode over UDP. Add `RepliconRenetPlugins` beside
/// `RepliconPlugins`, finish the App (replicon makes its channels in
/// `Plugin::finish`), then open the host's socket or join it.
#[cfg(feature = "renet")]
pub mod udp {
    use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
    use std::time::SystemTime;

    use bevy::prelude::World;
    use bevy_replicon::prelude::RepliconChannels;
    use bevy_replicon_renet::netcode::{
        ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication, ServerConfig,
    };
    use bevy_replicon_renet::renet::ConnectionConfig;
    use bevy_replicon_renet::RenetChannelsExt as _;

    pub use bevy_replicon_renet::{RenetClient, RenetServer, RepliconRenetPlugins};

    /// Both ends must name the same protocol, or netcode refuses the join.
    pub const PROTOCOL_ID: u64 = u64::from_le_bytes(*b"ambhost1");

    fn connection(world: &World) -> ConnectionConfig {
        let channels = world.resource::<RepliconChannels>();
        ConnectionConfig {
            server_channels_config: channels.server_configs(),
            client_channels_config: channels.client_configs(),
            ..Default::default()
        }
    }

    fn now() -> std::io::Result<std::time::Duration> {
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_err(std::io::Error::other)
    }

    /// Open the host's socket on loopback `port` (0: any free port) for up to
    /// `guests` guests, unauthenticated. Returns the address guests join.
    pub fn host(world: &mut World, port: u16, guests: usize) -> std::io::Result<SocketAddr> {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, port))?;
        let address = socket.local_addr()?;
        let transport = NetcodeServerTransport::new(
            ServerConfig {
                current_time: now()?,
                max_clients: guests,
                protocol_id: PROTOCOL_ID,
                authentication: ServerAuthentication::Unsecure,
                public_addresses: vec![address],
            },
            socket,
        )?;
        let server = RenetServer::new(connection(world));
        world.insert_resource(server);
        world.insert_resource(transport);
        Ok(address)
    }

    /// Join the host at `server` as guest `client_id`, unauthenticated.
    pub fn join(world: &mut World, server: SocketAddr, client_id: u64) -> std::io::Result<()> {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))?;
        let transport = NetcodeClientTransport::new(
            now()?,
            ClientAuthentication::Unsecure { client_id, protocol_id: PROTOCOL_ID, server_addr: server, user_data: None },
            socket,
        )
        .map_err(std::io::Error::other)?;
        let client = RenetClient::new(connection(world));
        world.insert_resource(client);
        world.insert_resource(transport);
        Ok(())
    }
}
