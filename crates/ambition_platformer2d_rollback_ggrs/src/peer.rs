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

use bevy::prelude::World;
use bevy_ggrs::ggrs::{self, DesyncDetection, Message, NonBlockingSocket, PlayerType, SessionBuilder};
use bevy_ggrs::Session;

use crate::session::{
    install_rebased_session, AmbitionGgrsConfig, AmbitionGgrsSession, FrameZeroEligibility,
    StartSyncTestError,
};

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

/// Start a P2P session whose peers agreed to begin at the live world: the
/// world becomes frame zero (`install_rebased_session`). Each peer calls this
/// at the same point of the same world.
///
/// The same two checks as a sync test, in the same order: GGRS refuses the
/// settings, or the world cannot declare frame zero. Either refusal leaves the
/// world unchanged.
pub fn start_peer_session(
    world: &mut World,
    settings: &PeerSessionSettings,
    socket: impl NonBlockingSocket<SocketAddr> + 'static,
) -> Result<(), StartSyncTestError> {
    let session = build_peer_session(settings, socket)?;
    let eligibility = FrameZeroEligibility::check(world)?;
    install_rebased_session(world, session, eligibility);
    Ok(())
}

/// One end of an in-memory link between peers in one process.
///
/// A message is delivered after it has waited `delay` receives at its
/// destination. GGRS receives once per poll, and a host polls once per
/// update, so the delay is a latency in updates: with a delay the remote
/// input arrives late, the peer predicts, and a wrong prediction rolls back.
pub struct LoopbackSocket {
    address: SocketAddr,
    link: Arc<Mutex<Link>>,
}

struct Link {
    delay: u32,
    // Ordered by address, so two runs deliver in one order.
    waiting: BTreeMap<SocketAddr, VecDeque<Parcel>>,
}

struct Parcel {
    from: SocketAddr,
    message: Message,
    waited: u32,
}

/// Two ends of one link, at `a` and `b`.
pub fn loopback_pair(a: SocketAddr, b: SocketAddr, delay: u32) -> (LoopbackSocket, LoopbackSocket) {
    let link = Arc::new(Mutex::new(Link {
        delay,
        waiting: BTreeMap::new(),
    }));
    (
        LoopbackSocket {
            address: a,
            link: link.clone(),
        },
        LoopbackSocket { address: b, link },
    )
}

impl NonBlockingSocket<SocketAddr> for LoopbackSocket {
    fn send_to(&mut self, message: &Message, addr: &SocketAddr) {
        let mut link = self.link.lock().expect("no holder of the link panics");
        link.waiting.entry(*addr).or_default().push_back(Parcel {
            from: self.address,
            message: message.clone(),
            waited: 0,
        });
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        let mut link = self.link.lock().expect("no holder of the link panics");
        let delay = link.delay;
        let queue = link.waiting.entry(self.address).or_default();
        for parcel in queue.iter_mut() {
            parcel.waited = parcel.waited.saturating_add(1);
        }
        let mut arrived = Vec::new();
        // In send order: a parcel never overtakes one sent before it.
        while queue.front().is_some_and(|parcel| parcel.waited > delay) {
            let parcel = queue.pop_front().expect("the front was just read");
            arrived.push((parcel.from, parcel.message));
        }
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

    fn running(session: &AmbitionGgrsSession) -> bool {
        matches!(session, Session::P2P(p2p) if p2p.current_state() == ggrs::SessionState::Running)
    }

    /// Two sessions over one loopback link synchronize. Each handshake round
    /// trip waits the link's delay, so it takes more polls than the delay.
    #[test]
    fn two_peer_sessions_synchronize_over_a_loopback_link() {
        let (a, b) = addresses();
        let (to_bob, to_alice) = loopback_pair(a, b, 3);
        let mut alice = build_peer_session(&settings(0, (1, b)), to_bob).expect("alice's session");
        let mut bob = build_peer_session(&settings(1, (0, a)), to_alice).expect("bob's session");
        let mut polls = 0;
        while !(running(&alice) && running(&bob)) {
            polls += 1;
            assert!(polls < 200, "the peers did not synchronize in 200 polls");
            for session in [&mut alice, &mut bob] {
                if let Session::P2P(p2p) = session {
                    p2p.poll_remote_clients();
                }
            }
        }
        assert!(polls > 3, "the link delivered before its delay: synchronized after {polls} polls");
    }
}
