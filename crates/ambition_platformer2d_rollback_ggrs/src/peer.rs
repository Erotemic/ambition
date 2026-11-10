//! A P2P session over any GGRS socket, and an in-memory socket pair.
//!
//! Netcode N2 installs a real transport through this road. The in-memory pair
//! is the transport of two Apps in one process: it is how the peer questions
//! that a sync test cannot ask (S7's unchecksummed rows, the remote rollback
//! window) are measured without signaling or deployment.

use std::collections::{BTreeMap, VecDeque};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::prelude::{Res, Resource, World};
use ambition_platformer2d_runtime::{
    ActiveRollbackAuthority, ContentFingerprint, ContentFingerprintSchemaVersion,
    PreparedContentIdentity, RollbackTimelineContract, SnapshotSchemaFingerprint,
};
use bevy_ggrs::ggrs::{self, DesyncDetection, Message, NonBlockingSocket, PlayerType, SessionBuilder};
use bevy_ggrs::Session;

use crate::session::{
    install_rebased_session, AmbitionGgrsConfig, AmbitionGgrsSession, FrameZeroEligibility,
    StartRollbackSessionError,
};

/// Prepared-content fields used for agreement between independent peers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerContentContract {
    pub fingerprint_schema: ContentFingerprintSchemaVersion,
    pub fingerprint: ContentFingerprint,
    pub snapshot_schema: SnapshotSchemaFingerprint,
}

/// The immutable content and schema proposal for one peer timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerTimelineContract {
    pub content: Option<PeerContentContract>,
    pub schema: SnapshotSchemaFingerprint,
    pub input_wire_identity: u32,
}

impl From<&RollbackTimelineContract> for PeerTimelineContract {
    fn from(local: &RollbackTimelineContract) -> Self {
        let RollbackTimelineContract { content, schema } = local;
        Self {
            content: content.map(|PreparedContentIdentity {
                fingerprint_schema, fingerprint, snapshot_schema, epoch: _,
            }| PeerContentContract { fingerprint_schema, fingerprint, snapshot_schema }),
            schema: *schema,
            input_wire_identity: ambition_platformer2d_core::CONTROL_FRAME_WIRE_IDENTITY,
        }
    }
}

/// The players of a P2P session as one peer sees them.
#[derive(Clone, Debug)]
pub struct PeerSessionSettings {
    pub players: usize,
    /// The handles (participant slots) this peer drives.
    pub local: Vec<usize>,
    /// The handles the other peers drive, at their addresses.
    pub remote: Vec<(usize, SocketAddr)>,
    pub max_prediction_window: usize,
    pub input_delay: usize,
    /// The peers compare checksums every this many confirmed frames. Must be
    /// more than zero: a peer session always compares.
    pub desync_interval: u32,
    /// How long a peer can be silent before GGRS drops it.
    pub disconnect_timeout: Duration,
}

/// Construct a P2P session WITHOUT touching the world.
pub fn build_peer_session(
    settings: &PeerSessionSettings,
    socket: impl NonBlockingSocket<SocketAddr> + 'static,
) -> Result<AmbitionGgrsSession, ggrs::GgrsError> {
    let mut builder = SessionBuilder::<AmbitionGgrsConfig>::new()
        .with_num_players(settings.players)?
        .with_fps(ambition_platformer2d_runtime::SIM_TICK_HZ as usize)?
        .with_max_prediction_window(settings.max_prediction_window)
        .with_input_delay(settings.input_delay)
        .with_desync_detection_mode(DesyncDetection::On {
            interval: settings.desync_interval,
        })
        .with_disconnect_timeout(settings.disconnect_timeout)
        .with_disconnect_notify_delay(settings.disconnect_timeout / 2);
    for &handle in &settings.local {
        builder = builder.add_player(PlayerType::Local, handle)?;
    }
    for &(handle, addr) in &settings.remote {
        builder = builder.add_player(PlayerType::Remote(addr), handle)?;
    }
    Ok(Session::P2P(builder.start_p2p_session(socket)?))
}

/// The transport of one peer: a socket for each session generation, on one
/// link to the other peers.
///
/// ⛔ THE RULE EACH TRANSPORT OWES: a socket receives only the parcels of its
/// own generation. A parcel of an older generation is dropped, and a parcel
/// of a later one waits for its socket. A new GGRS endpoint accepts each
/// message until its handshake is complete, so without this rule an input of
/// the old session is read as an input of the new timeline. See
/// [`LoopbackTransport`], which is the in-memory transport.
pub trait PeerTransport: Send + Sync + 'static {
    /// The socket of this peer's session `generation`. It makes no change to
    /// the link until the session polls it, so a session that is built and
    /// not installed costs nothing.
    fn socket_for(&self, generation: u32) -> Box<dyn NonBlockingSocket<SocketAddr>>;

    /// Announce an immutable proposal for this generation. A repeated equal
    /// proposal is permitted. A different proposal for the same generation is refused.
    fn offer_contract(&self, generation: u32, contract: &PeerTimelineContract) -> Result<(), String>;

    /// Read a remote peer's proposal, or wait while it is unavailable.
    /// Exchange must progress before GGRS sockets are polled. Return only
    /// the named peer and generation's immutable proposal.
    fn receive_contract(&self, generation: u32, from: SocketAddr) -> Result<Option<PeerTimelineContract>, String>;
}

struct BoxedSocket(Box<dyn NonBlockingSocket<SocketAddr>>);

impl NonBlockingSocket<SocketAddr> for BoxedSocket {
    fn send_to(&mut self, message: &Message, addr: &SocketAddr) {
        self.0.send_to(message, addr);
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        self.0.receive_all_messages()
    }
}

/// What a peer needs to start its next timeline: the settings of its session
/// and the transport, with the generation of the live session.
///
/// A peer session ends at each lifecycle commit, and the next one starts at
/// frame zero on the committed world (`lifecycle_commit`). The peers count
/// generations the same way: the first session of each peer, at one agreed
/// world, is generation 0, and each commit, which each peer runs, adds one.
/// The count moves only when a session is installed.
#[derive(Resource)]
pub struct PeerLineage {
    settings: PeerSessionSettings,
    transport: Box<dyn PeerTransport>,
    generation: u32,
    proposal: PeerTimelineContract,
    agreed: bool,
}

impl PeerLineage {
    pub(crate) fn input_delay(&self) -> usize {
        self.settings.input_delay
    }

    /// The generation of the live session.
    pub fn generation(&self) -> u32 {
        self.generation
    }

    fn agreement(&self, current: &RollbackTimelineContract) -> Result<bool, String> {
        if PeerTimelineContract::from(current) != self.proposal {
            return Err(format!("peer generation {} changed its proposed contract", self.generation));
        }
        if self.agreed {
            return Ok(true);
        }
        let mut complete = true;
        for &(_, remote) in &self.settings.remote {
            match self.transport.receive_contract(self.generation, remote)? {
                None => complete = false,
                Some(observed) if observed == self.proposal => {}
                Some(observed) => return Err(format!(
                    "peer contract mismatch at generation {} from {}: expected {:?}, observed {:?}",
                    self.generation, remote, self.proposal, observed,
                )),
            }
        }
        Ok(complete)
    }

    /// Construct the session of the next generation WITHOUT touching the
    /// world, the link or this lineage.
    pub(crate) fn build_next(&self) -> Result<AmbitionGgrsSession, ggrs::GgrsError> {
        build_peer_session(
            &self.settings,
            BoxedSocket(self.transport.socket_for(self.generation + 1)),
        )
    }
}

/// Start a peer timeline over the prepared live world.
///
/// Each peer must construct the same initial world. The transport compares
/// content and schema contracts before GGRS can advance generation zero.
/// A settings, carrier or proposal refusal leaves the live world unchanged.
pub fn start_peer_session(
    world: &mut World,
    settings: &PeerSessionSettings,
    transport: impl PeerTransport,
) -> Result<(), StartRollbackSessionError> {
    let session = build_peer_session(settings, BoxedSocket(transport.socket_for(0)))?;
    let eligibility = FrameZeroEligibility::check(world)?;
    let proposal = PeerTimelineContract::from(&crate::session::timeline_contract(world));
    transport.offer_contract(0, &proposal).map_err(StartRollbackSessionError::PeerContract)?;
    install_rebased_session(world, session, eligibility);
    world.insert_resource(PeerLineage {
        settings: settings.clone(),
        transport: Box::new(transport),
        generation: 0,
        proposal,
        agreed: false,
    });
    Ok(())
}

/// Install the session [`PeerLineage::build_next`] built, as frame zero of
/// the next generation. The lifecycle commit calls this after it changed the
/// world, on each peer.
pub(crate) fn install_next_peer_session(
    world: &mut World,
    session: AmbitionGgrsSession,
    eligibility: FrameZeroEligibility,
) {
    let proposal = PeerTimelineContract::from(&crate::session::timeline_contract(world));
    let offered = {
        let lineage = world.resource::<PeerLineage>();
        lineage.transport.offer_contract(lineage.generation + 1, &proposal)
    };
    if let Err(reason) = offered {
        crate::session::invalidate_session(world, reason);
        return;
    }
    install_rebased_session(world, session, eligibility);
    let mut lineage = world.resource_mut::<PeerLineage>();
    lineage.generation += 1;
    lineage.proposal = proposal;
    lineage.agreed = false;
}

/// Validate local stability and collect peer proposals before GGRS advances.
pub(crate) fn enforce_peer_contract(world: &mut World) {
    let Some(lineage) = world.get_resource::<PeerLineage>() else {
        return;
    };
    let became_ready = world.get_resource::<ActiveRollbackAuthority>()
        .ok_or_else(|| "peer session has no rollback authority".to_string())
        .and_then(|authority| lineage.agreement(authority.contract()))
        .map(|ready| ready && !lineage.agreed);
    match became_ready {
        Ok(true) => world.resource_mut::<PeerLineage>().agreed = true,
        Ok(false) => {}
        Err(reason) => crate::session::invalidate_session(world, reason),
    }
}

pub(crate) fn peer_agreement_is_ready(lineage: Option<Res<PeerLineage>>) -> bool {
    lineage.is_none_or(|lineage| lineage.agreed)
}

/// One peer's end of an in-memory link, for each session it starts.
///
/// A peer session ends at a lifecycle commit, and a new one starts at frame
/// zero. GGRS gives a session its own socket. A new endpoint accepts each
/// message until its handshake is complete: its `remote_magic` is zero until
/// then, and the filter on it is off. So an Input parcel of the old session
/// that is still on the link would be read as an input of the new timeline.
///
/// The transport prevents that. Each socket has the generation of its
/// session, each parcel has the generation of the socket that sent it, and a
/// socket receives only its own generation. A parcel of an older generation
/// is dropped, because its session ended. A parcel of a later generation
/// stays on the link for its socket: the other peer started its next session
/// first.
///
/// The caller gives the generation, and the peers count it the same way: the
/// first session of each peer, at one agreed world, is generation 0, and each
/// commit that each peer runs starts the next one. The transport does not
/// count, so a session that was built and not installed uses no number.
///
/// A clone is the same end of the same link.
#[derive(Clone)]
pub struct LoopbackTransport {
    address: SocketAddr,
    link: Arc<Mutex<Link>>,
}

impl LoopbackTransport {
    /// The socket of this peer's session `generation`.
    pub fn socket(&self, generation: u32) -> LoopbackSocket {
        LoopbackSocket {
            address: self.address,
            generation,
            link: self.link.clone(),
        }
    }

    /// The parcels the link dropped because their session had ended, at each
    /// end. More than zero shows that a new session started with parcels of
    /// the old one in flight.
    pub fn dropped_old_parcels(&self) -> u64 {
        self.link.lock().expect("no holder of the link panics").dropped_old
    }

    /// What this end has sent so far: messages, and their bytes as GGRS's UDP
    /// socket would put them in datagrams (bincode, as `UdpNonBlockingSocket`
    /// encodes a `Message`). The link traffic, as opposed to the width of one
    /// input: GGRS delta-encodes and run-length-compresses inputs before they
    /// are sent.
    pub fn sent(&self) -> LinkTraffic {
        self.link.lock().expect("no holder of the link panics")
            .sent.get(&self.address).copied().unwrap_or_default()
    }
}

/// Messages and datagram bytes one end of a link has sent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinkTraffic {
    pub messages: u64,
    pub bytes: u64,
}

/// One end of an in-memory link between peers in one process, for one session.
///
/// A message is delivered after it has waited `delay` receives at its
/// destination. GGRS receives once per poll, and a host polls once per
/// update, so the delay is a latency in updates: with a delay the remote
/// input arrives late, the peer predicts, and a wrong prediction rolls back.
pub struct LoopbackSocket {
    address: SocketAddr,
    /// See [`LoopbackTransport`].
    generation: u32,
    link: Arc<Mutex<Link>>,
}

struct Link {
    delay: u32,
    // Ordered by address, so two runs deliver in one order.
    waiting: BTreeMap<SocketAddr, VecDeque<Parcel>>,
    dropped_old: u64,
    contracts: BTreeMap<(u32, SocketAddr), PeerTimelineContract>,
    sent: BTreeMap<SocketAddr, LinkTraffic>,
}

struct Parcel {
    from: SocketAddr,
    generation: u32,
    message: Message,
    waited: u32,
}

/// The two ends of one link, at `a` and `b`.
/// Use a new link for each gameplay session.
pub fn loopback_transports(
    a: SocketAddr,
    b: SocketAddr,
    delay: u32,
) -> (LoopbackTransport, LoopbackTransport) {
    let link = Arc::new(Mutex::new(Link {
        delay,
        waiting: BTreeMap::new(),
        dropped_old: 0,
        contracts: BTreeMap::new(),
        sent: BTreeMap::new(),
    }));
    let end = |address| LoopbackTransport {
        address,
        link: link.clone(),
    };
    (end(a), end(b))
}

impl PeerTransport for LoopbackTransport {
    fn socket_for(&self, generation: u32) -> Box<dyn NonBlockingSocket<SocketAddr>> {
        Box::new(self.socket(generation))
    }

    fn offer_contract(&self, generation: u32, contract: &PeerTimelineContract) -> Result<(), String> {
        let mut link = self.link.lock().expect("no holder of the link panics");
        match link.contracts.entry((generation, self.address)) {
            std::collections::btree_map::Entry::Vacant(entry) => { entry.insert(*contract); }
            std::collections::btree_map::Entry::Occupied(entry) if entry.get() == contract => {}
            _ => return Err(format!("peer {} already proposed another contract for generation {}", self.address, generation)),
        }
        Ok(())
    }

    fn receive_contract(&self, generation: u32, from: SocketAddr) -> Result<Option<PeerTimelineContract>, String> {
        Ok(self.link.lock().expect("no holder of the link panics")
            .contracts.get(&(generation, from)).copied())
    }
}

impl NonBlockingSocket<SocketAddr> for LoopbackSocket {
    fn send_to(&mut self, message: &Message, addr: &SocketAddr) {
        let mut link = self.link.lock().expect("no holder of the link panics");
        let traffic = link.sent.entry(self.address).or_default();
        traffic.messages += 1;
        traffic.bytes += bincode::serialized_size(message).unwrap_or(0);
        link.waiting.entry(*addr).or_default().push_back(Parcel {
            from: self.address,
            generation: self.generation,
            message: message.clone(),
            waited: 0,
        });
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        let mut link = self.link.lock().expect("no holder of the link panics");
        let Link {
            delay,
            waiting,
            dropped_old,
            ..
        } = &mut *link;
        let queue = waiting.entry(self.address).or_default();
        // The session that could read a parcel of an older generation ended.
        // ⛔ Not delivered: this socket's endpoint accepts each message until
        // its handshake is complete.
        let before = queue.len();
        queue.retain(|parcel| parcel.generation >= self.generation);
        *dropped_old += (before - queue.len()) as u64;
        for parcel in queue.iter_mut() {
            parcel.waited = parcel.waited.saturating_add(1);
        }
        // In send order: each parcel has waited at least as long as each
        // parcel behind it, so none overtakes one sent before it. A parcel of
        // a later generation stays for its socket.
        let mut arrived = Vec::new();
        let mut stay = VecDeque::with_capacity(queue.len());
        for parcel in queue.drain(..) {
            if parcel.generation == self.generation && parcel.waited > *delay {
                arrived.push((parcel.from, parcel.message));
            } else {
                stay.push_back(parcel);
            }
        }
        *queue = stay;
        arrived
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addresses() -> (SocketAddr, SocketAddr) {
        ("127.0.0.1:7001".parse().unwrap(), "127.0.0.1:7002".parse().unwrap())
    }

    fn settings(local: usize, remote: (usize, SocketAddr)) -> PeerSessionSettings {
        PeerSessionSettings {
            players: 2,
            local: vec![local],
            remote: vec![remote],
            max_prediction_window: 8,
            input_delay: 0,
            desync_interval: 1,
            disconnect_timeout: Duration::from_secs(60),
        }
    }

    #[test]
    fn another_proposal_for_one_generation_refuses_start_without_changing_the_world() {
        let (a, b) = addresses();
        let (alice, _) = loopback_transports(a, b, 0);
        let mut world = World::new();
        let mut previous = PeerTimelineContract::from(&crate::session::timeline_contract(&world));
        previous.schema = ambition_platformer2d_runtime::SnapshotSchemaFingerprint::from_bytes([7; 32]);
        alice.offer_contract(0, &previous).unwrap();
        let error = start_peer_session(&mut world, &settings(0, (1, b)), alice).unwrap_err();
        assert!(matches!(error, StartRollbackSessionError::PeerContract(_)));
        assert!(!crate::session::session_is_active(&world));
        assert!(!world.contains_resource::<ActiveRollbackAuthority>());
        assert!(!world.contains_resource::<PeerLineage>());
    }

    #[test]
    fn another_input_format_cannot_agree() {
        let (a, b) = addresses();
        let (alice, bob) = loopback_transports(a, b, 0);
        let current = crate::session::timeline_contract(&World::new());
        let proposal = PeerTimelineContract::from(&current);
        let mut incompatible = proposal;
        incompatible.input_wire_identity ^= 1;
        bob.offer_contract(0, &incompatible).unwrap();
        let lineage = PeerLineage {
            settings: settings(0, (1, b)), transport: Box::new(alice),
            generation: 0, proposal, agreed: false,
        };
        let reason = lineage.agreement(&current).unwrap_err();
        assert!(reason.contains("peer contract mismatch"), "{reason}");
    }

    fn running(session: &AmbitionGgrsSession) -> bool {
        matches!(session, Session::P2P(p2p) if p2p.current_state() == ggrs::SessionState::Running)
    }

    fn p2p(session: &mut AmbitionGgrsSession) -> &mut ggrs::P2PSession<AmbitionGgrsConfig> {
        match session {
            Session::P2P(p2p) => p2p,
            _ => panic!("the fixture builds P2P sessions only"),
        }
    }

    /// Poll the two sessions until each is running.
    fn synchronize(alice: &mut AmbitionGgrsSession, bob: &mut AmbitionGgrsSession) -> u32 {
        let mut polls = 0;
        while !(running(alice) && running(bob)) {
            polls += 1;
            assert!(polls < 200, "the peers did not synchronize in 200 polls");
            p2p(alice).poll_remote_clients();
            p2p(bob).poll_remote_clients();
        }
        polls
    }

    /// The parcels on the link for `to`, as (generation, the kind of message).
    fn waiting_for(transport: &LoopbackTransport, to: SocketAddr) -> Vec<(u32, &'static str)> {
        let link = transport.link.lock().unwrap();
        link.waiting
            .get(&to)
            .into_iter()
            .flatten()
            .map(|parcel| {
                let text = format!("{:?}", parcel.message);
                let kind = ["SyncRequest", "SyncReply", "InputAck", "Input", "QualityReport", "QualityReply", "ChecksumReport", "KeepAlive"]
                    .into_iter()
                    .find(|kind| text.contains(&format!("body: {kind}")))
                    .unwrap_or("a message kind this fixture does not name");
                (parcel.generation, kind)
            })
            .collect()
    }

    /// Two sessions over one loopback link synchronize. Each handshake round
    /// trip waits the link's delay, so it takes more polls than the delay.
    #[test]
    fn two_peer_sessions_synchronize_over_a_loopback_link() {
        let (a, b) = addresses();
        let (alices, bobs) = loopback_transports(a, b, 3);
        let mut alice =
            build_peer_session(&settings(0, (1, b)), alices.socket(0)).expect("alice's session");
        let mut bob =
            build_peer_session(&settings(1, (0, a)), bobs.socket(0)).expect("bob's session");
        let polls = synchronize(&mut alice, &mut bob);
        assert!(polls > 3, "the link delivered before its delay: synchronized after {polls} polls");
    }

    /// Two running sessions, with one Input parcel of Alice's on the link
    /// for Bob. Returns the transports and Bob's running session.
    fn an_input_parcel_in_flight() -> (LoopbackTransport, LoopbackTransport, AmbitionGgrsSession, AmbitionGgrsSession) {
        let (a, b) = addresses();
        let (alices, bobs) = loopback_transports(a, b, DELAY);
        let mut alice =
            build_peer_session(&settings(0, (1, b)), alices.socket(0)).expect("alice's session");
        let mut bob =
            build_peer_session(&settings(1, (0, a)), bobs.socket(0)).expect("bob's session");
        synchronize(&mut alice, &mut bob);
        // Empty the link of the handshake's last parcels, so that the parcel
        // the arms read is the input.
        for _ in 0..=DELAY {
            p2p(&mut bob).poll_remote_clients();
        }
        p2p(&mut alice).add_local_input(0, Default::default()).expect("alice has handle 0");
        // No game state is behind these sessions: the requests are not run.
        let _ = p2p(&mut alice).advance_frame().expect("alice advances frame 0");
        assert!(
            waiting_for(&alices, b).contains(&(0, "Input")),
            "premise: an Input parcel of generation 0 is on the link for Bob; it holds {:?}",
            waiting_for(&alices, b)
        );
        (alices, bobs, alice, bob)
    }

    const DELAY: u32 = 3;

    /// THE CASE THE GENERATION IS FOR. Bob starts his next session while an
    /// Input parcel of Alice's old session is on the link. Bob's new endpoint
    /// has no `remote_magic` yet, so GGRS would read that parcel. The link
    /// does not deliver it.
    #[test]
    fn a_parcel_of_an_older_session_is_not_delivered_to_a_new_endpoint() {
        let (alices, bobs, _alice, bob) = an_input_parcel_in_flight();
        let (_, b) = addresses();
        drop(bob);
        let mut next = bobs.socket(1);
        let mut delivered = Vec::new();
        // More receives than the delay: the parcel would have arrived.
        for _ in 0..=DELAY + 1 {
            delivered.extend(next.receive_all_messages());
        }
        assert_eq!(
            delivered.iter().map(|(from, message)| (*from, format!("{message:?}"))).collect::<Vec<_>>(),
            Vec::<(SocketAddr, String)>::new(),
            "the socket of Bob's next session received a parcel of the old session"
        );
        assert_eq!(waiting_for(&alices, b), vec![], "the parcels still on the link for Bob");
        assert!(bobs.dropped_old_parcels() >= 1, "the link dropped nothing, so the parcel went elsewhere");
    }

    /// The control: the socket of the SAME session receives that parcel. So
    /// the arm above shows the generation, and not a parcel that was never
    /// going to arrive.
    #[test]
    fn the_same_parcel_is_delivered_to_the_session_it_was_sent_to() {
        let (alices, _bobs, _alice, mut bob) = an_input_parcel_in_flight();
        let (_, b) = addresses();
        for _ in 0..=DELAY + 1 {
            p2p(&mut bob).poll_remote_clients();
        }
        assert!(
            !waiting_for(&alices, b).contains(&(0, "Input")),
            "Bob's session polled past the delay and the Input parcel is still on the link"
        );
        assert_eq!(alices.dropped_old_parcels(), 0, "the link dropped a parcel of a live session");
    }

    /// One peer starts its next session first. Its handshake parcels stay on
    /// the link while the other peer still polls its old session: the old
    /// session does not read them, and the new one gets them.
    #[test]
    fn a_parcel_of_the_next_session_waits_for_its_socket() {
        let (alices, bobs, alice, mut bob) = an_input_parcel_in_flight();
        let (a, b) = addresses();
        drop(alice);
        let mut alice =
            build_peer_session(&settings(0, (1, b)), alices.socket(1)).expect("alice's next session");
        p2p(&mut alice).poll_remote_clients();
        assert!(
            waiting_for(&alices, b).contains(&(1, "SyncRequest")),
            "premise: Alice's next session sent a handshake parcel; the link holds {:?}",
            waiting_for(&alices, b)
        );
        // Bob's old session polls past the delay.
        for _ in 0..=DELAY + 1 {
            p2p(&mut bob).poll_remote_clients();
        }
        assert!(
            waiting_for(&alices, b).contains(&(1, "SyncRequest")),
            "Bob's old session took a parcel of the next generation; the link holds {:?}",
            waiting_for(&alices, b)
        );
        // Bob starts his next session, and the two synchronize.
        drop(bob);
        let mut bob =
            build_peer_session(&settings(1, (0, a)), bobs.socket(1)).expect("bob's next session");
        synchronize(&mut alice, &mut bob);
    }
}
