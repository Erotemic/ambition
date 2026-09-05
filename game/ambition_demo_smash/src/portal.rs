//! The portal recovery, assembled from the portal crate's own parts.
//!
//! ⭐⭐ NOTHING HERE IS PORTAL BEHAVIOUR. `ambition_portal2d` owns apertures,
//! linking and transit; `PlacedPortal` is a Component, so opening one is a
//! `spawn`, and the crate's `evict_straddlers_on_portal_change` already handles
//! the one hard case — an aperture vanishing under a body that straddles it,
//! which it calls "the ONE sanctioned pushout". This module places two apertures
//! and counts down. That is the whole of it.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_portal::{PortalPairParams, PORTAL_PAIR};
use ambition_platformer2d::engine_core as ae;

/// One aperture a MOVE opened, and how long it has left.
///
/// ⛔ ROLLBACK STATE, for the reason `LiveBomb`'s doc gives about its fuse: the
/// countdown outlives the tick that made it, so a rewind that put the aperture
/// back without putting its clock back would give the resimulated timeline a
/// portal that closes at a different moment from the confirmed one — and a
/// recovery route that exists on one peer and not the other.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct MovePlacedPortal {
    /// Seconds before this aperture closes on its own.
    pub remaining_s: f32,
    /// Close the pair the first time anything transits it.
    pub close_on_transit: bool,
    /// The pair's low index — both apertures carry the same value, so either can
    /// find its partner without a handle. ⛔ Not an `Entity`: a rewind
    /// invalidates one and the channel index survives it.
    pub pair_index: u8,
}

/// Checksum probe: the clock is the part a peer can disagree about.
pub fn move_placed_portal_probe(portal: &MovePlacedPortal) -> u64 {
    portal.remaining_s.to_bits() as u64
}

/// Open a linked pair where a move asked for one.
///
/// ⛔ THE ENTRANCE IS AT THE FIGHTER AND THE EXIT IS ABOVE, and the normals face
/// each other: you fall INTO the low aperture (its normal points up, out of the
/// floor you are above) and arrive at the high one. A pair whose normals both
/// pointed the same way would be a route that only works in one direction, which
/// is not what "it's a portal" means.
pub fn open_authored_portal_pairs(
    mut commands: Commands,
    mut actions: MessageReader<ActorActionMessage>,
    bodies: Query<&ae::BodyKinematics>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != PORTAL_PAIR {
            continue;
        }
        let params: PortalPairParams = match params.hydrate() {
            Ok(p) => p,
            Err(err) => {
                warn!("smash portal pair params did not hydrate: {err}");
                continue;
            }
        };
        let Ok(kin) = bodies.get(message.actor) else {
            continue;
        };
        let half = ae::Vec2::new(params.half_extent.0, params.half_extent.1);
        // ⭐ THE TILT IS APPLIED TO BOTH NORMALS TOGETHER. Rotating one and not
        // the other would change where the pair sends you rather than how it is
        // angled, which is a different move and a confusing one.
        let tilt = params.tilt_degrees.to_radians();
        let up = ae::Vec2::new(-tilt.sin(), -tilt.cos());
        let down = -up;
        let entrance = kin.pos;
        let exit = kin.pos + up * params.rise;
        let low = params.channel_index;
        // ⭐ THE PARTNER COMES FROM THE CRATE, not from `low ^ 1` written again
        // here. `PortalChannel::partner()` is where that rule lives, and a second
        // copy of it is a pairing rule with two homes that drift apart the day
        // the channel space changes shape.
        let entrance_channel = ambition_platformer2d::portal::PortalChannel::Authored(
            ambition_platformer2d::portal::PortalChannelColor::Indexed(low),
        );
        for (pos, normal, channel) in [
            (entrance, up, entrance_channel),
            (exit, down, entrance_channel.partner()),
        ] {
            commands.spawn((
                ambition_platformer2d::portal::PlacedPortal::fixed(channel, pos, normal, half),
                MovePlacedPortal {
                    remaining_s: params.lifetime_s,
                    close_on_transit: params.close_on_transit,
                    pair_index: low,
                },
            ));
        }
    }
}

/// Close a move-placed pair when its clock runs out.
///
/// ⛔ BOTH ENDS TOGETHER, ALWAYS. A pair with one aperture left is a hole that
/// swallows and never returns, which is worse than either closing or staying
/// open — so the sweep collects the pairs whose clock expired and despawns every
/// aperture carrying that index.
pub fn close_expired_move_portals(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut portals: Query<(Entity, &mut MovePlacedPortal)>,
) {
    let dt = time.sim_dt();
    let mut expired: Vec<u8> = Vec::new();
    for (_, mut portal) in &mut portals {
        portal.remaining_s -= dt;
        if portal.remaining_s <= 0.0 {
            expired.push(portal.pair_index);
        }
    }
    if expired.is_empty() {
        return;
    }
    for (entity, portal) in &portals {
        if expired.contains(&portal.pair_index) {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::portal::{PlacedPortal, PortalChannel};

    fn params(lifetime_s: f32) -> PortalPairParams {
        PortalPairParams {
            rise: 320.0,
            half_extent: (26.0, 6.0),
            lifetime_s,
            close_on_transit: false,
            tilt_degrees: 0.0,
            channel_index: 8,
        }
    }

    fn app_with(lifetime_s: f32) -> (App, Entity) {
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.init_resource::<ambition_platformer2d::time::WorldTime>();
        {
            let mut time = app
                .world_mut()
                .resource_mut::<ambition_platformer2d::time::WorldTime>();
            time.scaled_dt = 1.0 / 60.0;
            time.raw_dt = 1.0 / 60.0;
        }
        app.add_systems(
            Update,
            (open_authored_portal_pairs, close_expired_move_portals).chain(),
        );
        let body = app
            .world_mut()
            .spawn(ae::BodyKinematics {
                pos: ae::Vec2::new(100.0, 200.0),
                ..Default::default()
            })
            .id();
        app.world_mut().write_message(ActorActionMessage {
            actor: body,
            request: ActionRequest::Special {
                spec: SpecialActionSpec::Special(PORTAL_PAIR.to_string()),
                params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params(
                    lifetime_s,
                ))
                .expect("portal params serialize"),
            },
        });
        (app, body)
    }

    fn placed(app: &mut App) -> Vec<(PortalChannel, ae::Vec2)> {
        app.world_mut()
            .query::<&PlacedPortal>()
            .iter(app.world())
            .map(|p| (p.channel, p.pos))
            .collect()
    }

    /// The move opens a LINKED pair, one above the other.
    ///
    /// ⛔ THE LINK IS THE MOVE. Two apertures on unrelated channels are two holes
    /// that go nowhere, and the fighter falls through the lower one onto the
    /// stage below — which reads as the recovery simply failing.
    #[test]
    fn the_move_opens_a_linked_pair_with_the_exit_above() {
        let (mut app, _body) = app_with(2.5);
        app.update();

        let mut portals = placed(&mut app);
        assert_eq!(
            portals.len(),
            2,
            "the move opened {} aperture(s) rather than a pair",
            portals.len()
        );
        portals.sort_by(|a, b| a.1.y.partial_cmp(&b.1.y).unwrap());
        let (high_channel, high) = portals[0];
        let (low_channel, low) = portals[1];
        assert_eq!(
            high_channel,
            low_channel.partner(),
            "the two apertures are not each other's partner, so falling into one \
             does not arrive at the other"
        );
        // Up is NEGATIVE y here, as everywhere in this codebase.
        assert!(
            (low.y - high.y - 320.0).abs() < 0.001,
            "the exit is {}px above the entrance, not the authored 320",
            low.y - high.y
        );
        assert!(
            (low.x - high.x).abs() < 0.001,
            "an untilted pair drifted sideways by {}px",
            low.x - high.x
        );
    }

    /// A pair closes TOGETHER even when only one end's clock has run out.
    ///
    /// ⛔⛔ THE GUARD BELOW COULD NOT SEE THIS AND I ONLY FOUND OUT BY POISONING
    /// IT. Both apertures of a move-placed pair share an authored lifetime and
    /// tick in lockstep, so "close everything expired" and "close the whole PAIR
    /// when any of it expires" are indistinguishable in the ordinary case — the
    /// poison that narrowed the sweep to `remaining_s <= 0.0` left the test
    /// green. This constructs the asymmetry directly, which is the only way the
    /// pair rule is observable.
    ///
    /// ⇒ It matters the moment anything can touch one end alone: a
    /// `close_on_transit` pair, an aperture evicted with its host face, or a
    /// second placement reusing the index. A pair with one end left is a hole
    /// that swallows and never returns.
    #[test]
    fn one_end_expiring_closes_the_other() {
        let mut app = App::new();
        app.init_resource::<ambition_platformer2d::time::WorldTime>();
        {
            let mut time = app
                .world_mut()
                .resource_mut::<ambition_platformer2d::time::WorldTime>();
            time.scaled_dt = 1.0 / 60.0;
            time.raw_dt = 1.0 / 60.0;
        }
        app.add_systems(Update, close_expired_move_portals);
        let channel = PortalChannel::Authored(
            ambition_platformer2d::portal::PortalChannelColor::Indexed(8),
        );
        // The near end is about to expire; the far end has seconds left.
        for (remaining_s, chan) in [(1.0 / 120.0, channel), (5.0, channel.partner())] {
            app.world_mut().spawn((
                PlacedPortal::fixed(chan, ae::Vec2::ZERO, ae::Vec2::new(0.0, -1.0), ae::Vec2::ONE),
                MovePlacedPortal {
                    remaining_s,
                    close_on_transit: false,
                    pair_index: 8,
                },
            ));
        }
        app.update();
        assert!(
            placed(&mut app).is_empty(),
            "one end of the pair expired and {} aperture(s) stayed open — the \
             survivor is a hole that swallows and never returns",
            placed(&mut app).len()
        );
    }

    /// When the clock runs out BOTH ends close.
    ///
    /// ⛔ A PAIR WITH ONE END LEFT IS WORSE THAN EITHER OUTCOME: a hole that
    /// swallows and never returns. Whatever falls in is gone.
    #[test]
    fn an_expired_pair_closes_at_both_ends() {
        // ⓘ A LIFETIME OF ONE AND A HALF TICKS. The close sweep is chained
        // AFTER the open, so it ticks on the frame the pair appears — an
        // authored lifetime is therefore spent from that frame, and anything at
        // or under one tick would close before a body could ever reach it.
        let (mut app, _body) = app_with(1.5 / 60.0);
        app.update();
        assert_eq!(placed(&mut app).len(), 2, "the pair did not open");
        app.update();
        assert!(
            placed(&mut app).is_empty(),
            "an expired portal pair left {} aperture(s) standing — a hole that \
             swallows and never returns",
            placed(&mut app).len()
        );
    }
}
