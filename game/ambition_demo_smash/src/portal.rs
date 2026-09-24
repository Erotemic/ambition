//! The portal recovery, assembled from the portal crate's own parts.
//!
//! No portal behaviour lives here. `ambition_portal2d` owns apertures, linking
//! and transit; `PlacedPortal` is a Component, so opening one is a `spawn`, and
//! `evict_straddlers_on_portal_change` handles an aperture vanishing under a
//! body that straddles it. This module places two apertures and counts down.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::entity_catalog::smash_portal::{PortalPairParams, PORTAL_PAIR};
use ambition_platformer2d::engine_core as ae;

/// One aperture a MOVE opened, and how long it has left.
///
/// Rollback state, like `LiveBomb`'s fuse: a restore without the clock would
/// close the recovery route at a different moment on each peer.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct MovePlacedPortal {
    /// Seconds before this aperture closes on its own.
    pub remaining_s: f32,
    /// Close the pair the first time anything transits it.
    pub close_on_transit: bool,
    /// The pair's low index. Both apertures carry it, so either can find its
    /// partner. Not an `Entity`: a rewind invalidates one; the index survives.
    pub pair_index: u8,
}

/// Checksum probe: the clock is the part a peer can disagree about.
pub fn move_placed_portal_probe(portal: &MovePlacedPortal) -> u64 {
    portal.remaining_s.to_bits() as u64
}

/// Open a linked pair where a move asked for one.
///
/// The entrance is at the fighter and the exit above, with normals facing each
/// other: you fall into the low aperture (normal up) and arrive at the high
/// one. Normals pointing the same way would work in only one direction.
pub fn open_authored_portal_pairs(
    mut commands: Commands,
    mut actions: MessageReader<ActorActionMessage>,
    // The seat gives the pair an occurrence identity. `channel_index` is
    // authoring data (the same for every Alice), so two Alices recovering at
    // once would share channels, cross-link, and close each other's pairs.
    bodies: Query<(&ae::BodyKinematics, &ambition_platformer2d::actor::MatchSeat)>,
    // The aim the teleport also uses: `MovePlayback::aimed_stick`, a latched
    // stick direction that is already rollback state. A live stick would be
    // neutral anyway, because an aimed special is rooted.
    playbacks: Query<&ambition_platformer2d::combat::moveset::MovePlayback>,
    // Open move portals, so a caster's second activation can retire the
    // first; see the note at the despawn below.
    existing: Query<(
        Entity,
        &MovePlacedPortal,
        &ambition_platformer2d::portal::PlacedPortal,
    )>,
    // The running match, so what this spawns dies with it (see
    // `crate::match_scope`).
    active_match: Option<Res<ambition_platformer2d::versus_match::ActiveMatch>>,
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
        let Ok((kin, seat)) = bodies.get(message.actor) else {
            continue;
        };
        let half = ae::Vec2::new(params.half_extent.0, params.half_extent.1);
        // The tilt applies to both normals together; rotating one would change
        // where the pair sends you, not its angle.
        //
        // The player angles it: `tilt_degrees` is the base and
        // `aim_tilt_degrees` the range the stick swings it through. Only the
        // horizontal stick component counts: up and down are what the pair
        // already does, and aiming the exit downward would make a hole, not a
        // recovery. An aim range of `0.0` (the default) is not aimable.
        let aim = playbacks
            .get(message.actor)
            .ok()
            .and_then(|playback| playback.aimed_stick)
            .map(|stick| stick.x.clamp(-1.0, 1.0))
            .unwrap_or(0.0);
        let tilt = (params.tilt_degrees + aim * params.aim_tilt_degrees).to_radians();
        // Positive tilt leans toward `+x`, so holding right puts the exit to
        // the right. World space, not mirrored by facing: the stick is an aim
        // on the screen.
        let up = ae::Vec2::new(tilt.sin(), -tilt.cos());
        let down = -up;
        let entrance = kin.pos;
        let exit = kin.pos + up * params.rise;
        // The authored index is a base; the seat makes it an occurrence. A pair
        // uses two adjacent channels, so each seat gets its own window (two
        // Alices open on 8/9 and 10/11), and the expiry sweep on `pair_index`
        // closes only its own pair.
        //
        // `MatchSeat`, not `Entity`: the seat is rollback-registered
        // (`actor.match_seat`), while bevy_ggrs recreates entities.
        //
        // It saturates instead of wrapping: an overflowing seat falls back to a
        // shared channel, not onto another seat's window.
        let low = params
            .channel_index
            .saturating_add((seat.0 as u8).saturating_mul(2));
        // One live pair per caster: a second opening retires the first. A pair
        // lives 2.5s, and landing, a ledge catch or a flinching strike refresh
        // the recovery, so one seat could otherwise have two pairs on one
        // window. "You have one pair of portals" is the genre's rule and keeps
        // `pair_index` exact; a per-activation id would buy nothing a move asks
        // for.
        //
        // Roster decision #17 (Jon may overrule): recasting up-B while the shaft
        // is open closes it.
        for (entity, portal, _) in &existing {
            if portal.pair_index == low {
                commands.entity(entity).despawn();
            }
        }
        // The partner comes from `PortalChannel::partner()`, not a local
        // `low ^ 1`, so the pairing rule has one home.
        let entrance_channel = ambition_platformer2d::portal::PortalChannel::Authored(
            ambition_platformer2d::portal::PortalChannelColor::Indexed(low),
        );
        for (pos, normal, channel) in [
            (entrance, up, entrance_channel),
            (exit, down, entrance_channel.partner()),
        ] {
            let spawned = commands.spawn((
                ambition_platformer2d::portal::PlacedPortal::fixed(channel, pos, normal, half),
                MovePlacedPortal {
                    remaining_s: params.lifetime_s,
                    close_on_transit: params.close_on_transit,
                    pair_index: low,
                },
            )).id();
            // The match owns this object's end; see `crate::match_scope`.
            crate::match_scope::stamp(&mut commands, spawned, active_match.as_deref());
        }
    }
}

/// Close a move-placed pair — when its clock runs out, or when somebody used it
/// and the pair was authored to be one-shot.
///
/// Both ends close together: a pair with one aperture left swallows and never
/// returns. The sweep collects finished pairs and despawns every aperture with
/// that index.
///
/// `close_on_transit` is handled here too, so one system owns "this pair is
/// finished" and a pair never has two despawners.
///
/// A transit is matched by where the body arrived: `PortalBodyTransited` names
/// the body and exit position, not the apertures, and the exit position is
/// that aperture's centroid.
pub fn close_expired_move_portals(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut transits: MessageReader<ambition_platformer2d::portal::PortalBodyTransited>,
    // One query, not two: a second `Query<&MovePlacedPortal>` beside this
    // `&mut` is a `B0001` conflict, and clock and aperture always go together.
    mut portals: Query<(
        Entity,
        &mut MovePlacedPortal,
        &ambition_platformer2d::portal::PlacedPortal,
    )>,
) {
    let dt = time.sim_dt();
    let mut expired: Vec<u8> = Vec::new();
    for (_, mut portal, _) in &mut portals {
        portal.remaining_s -= dt;
        if portal.remaining_s <= 0.0 {
            expired.push(portal.pair_index);
        }
    }
    // Drain the reader every frame, even with no one-shot pair, so a later
    // frame does not get a stale backlog (as in `record_stock_lifecycle`).
    for transit in transits.read() {
        for (_, portal, aperture) in &portals {
            if !portal.close_on_transit {
                continue;
            }
            let offset = (transit.exit_pos - aperture.pos).abs();
            if offset.x <= aperture.half_extent.x && offset.y <= aperture.half_extent.y {
                expired.push(portal.pair_index);
            }
        }
    }
    if expired.is_empty() {
        return;
    }
    for (entity, portal, _) in &portals {
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
            aim_tilt_degrees: 0.0,
            channel_index: 8,
        }
    }

    fn app_with(lifetime_s: f32) -> (App, Entity) {
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        // The sweep reads transits; without this message its parameter
        // validation fails and it never runs.
        app.add_message::<ambition_platformer2d::portal::PortalBodyTransited>();
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
            .spawn((
                ae::BodyKinematics {
                    pos: ae::Vec2::new(100.0, 200.0),
                    ..Default::default()
                },
                // The pair's channel is derived from the seat, so a caster
                // must have one.
                ambition_platformer2d::actor::MatchSeat(0),
            ))
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
            move_instance: None,
        });
        (app, body)
    }

    /// Two Alices recovering at once open four distinct apertures, and one
    /// expiring does not close the other's pair.
    ///
    /// `channel_index` is authoring data (the same `8` for every Alice).
    /// Without the seat offset, `find_portal` returns the first match on a
    /// shared channel, and the expiry sweep closes every pair with the index.
    /// The seat is rollback-registered; an `Entity` is not stable across peers.
    #[test]
    fn two_fighters_recovering_at_once_get_pairs_that_cannot_see_each_other() {
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.add_message::<ambition_platformer2d::portal::PortalBodyTransited>();
        app.init_resource::<ambition_platformer2d::time::WorldTime>();
        {
            let mut time = app
                .world_mut()
                .resource_mut::<ambition_platformer2d::time::WorldTime>();
            time.scaled_dt = 1.0 / 60.0;
            time.raw_dt = 1.0 / 60.0;
        }
        app.add_systems(Update, open_authored_portal_pairs);
        for seat in [0usize, 1usize] {
            let body = app
                .world_mut()
                .spawn((
                    ae::BodyKinematics {
                        pos: ae::Vec2::new(100.0 * seat as f32, 0.0),
                        ..Default::default()
                    },
                    ambition_platformer2d::actor::MatchSeat(seat),
                ))
                .id();
            app.world_mut().write_message(ActorActionMessage {
                actor: body,
                request: ActionRequest::Special {
                    spec: SpecialActionSpec::Special(PORTAL_PAIR.to_string()),
                    params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
                        &params(2.5),
                    )
                    .expect("portal params serialize"),
                },
                move_instance: None,
            });
        }
        app.update();

        let channels: Vec<PortalChannel> = placed(&mut app).into_iter().map(|(c, _)| c).collect();
        assert_eq!(channels.len(), 4, "two pairs is four apertures");
        let mut unique = channels.clone();
        unique.sort_by_key(|c| format!("{c:?}"));
        unique.dedup();
        assert_eq!(
            unique.len(),
            4,
            "the two fighters share a channel ({channels:?}) — `find_portal` \
             returns the first match, so one can leave through the other's exit"
        );

        let mut indices: Vec<u8> = app
            .world_mut()
            .query::<&MovePlacedPortal>()
            .iter(app.world())
            .map(|p| p.pair_index)
            .collect();
        indices.sort_unstable();
        indices.dedup();
        assert_eq!(
            indices.len(),
            2,
            "both pairs carry one `pair_index` ({indices:?}), so the expiry \
             sweep closes BOTH when either runs out"
        );
    }

    /// One caster, one live pair: a second activation retires the first.
    ///
    /// Recovery refreshes can give one seat two live pairs on one channel
    /// window, which cross-links inside the seat and lets the older pair's
    /// expiry close the newer one. The assertion: a second cast leaves two
    /// apertures, and the survivors are the new ones.
    #[test]
    fn a_second_cast_by_one_fighter_retires_its_first_pair() {
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.add_message::<ambition_platformer2d::portal::PortalBodyTransited>();
        app.init_resource::<ambition_platformer2d::time::WorldTime>();
        {
            let mut time = app
                .world_mut()
                .resource_mut::<ambition_platformer2d::time::WorldTime>();
            time.scaled_dt = 1.0 / 60.0;
            time.raw_dt = 1.0 / 60.0;
        }
        app.add_systems(Update, open_authored_portal_pairs);
        let body = app
            .world_mut()
            .spawn((
                ae::BodyKinematics {
                    pos: ae::Vec2::ZERO,
                    ..Default::default()
                },
                ambition_platformer2d::actor::MatchSeat(0),
            ))
            .id();
        let cast = |app: &mut App, at: ae::Vec2| {
            app.world_mut().entity_mut(body).insert(ae::BodyKinematics {
                pos: at,
                ..Default::default()
            });
            app.world_mut().write_message(ActorActionMessage {
                actor: body,
                request: ActionRequest::Special {
                    spec: SpecialActionSpec::Special(PORTAL_PAIR.to_string()),
                    params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
                        &params(2.5),
                    )
                    .expect("portal params serialize"),
                },
                move_instance: None,
            });
            app.update();
        };

        cast(&mut app, ae::Vec2::ZERO);
        assert_eq!(placed(&mut app).len(), 2, "the first cast opened no pair");

        // Somewhere else, while the first pair is well inside its 2.5s life.
        let second = ae::Vec2::new(600.0, 0.0);
        cast(&mut app, second);
        let after = placed(&mut app);
        assert_eq!(
            after.len(),
            2,
            "one fighter holds {} apertures ({after:?}) — a second cast left the \
             first pair live, so his own channels cross-link and whichever \
             expires first closes both",
            after.len()
        );
        assert!(
            after.iter().any(|(_, pos)| (pos.x - second.x).abs() < 1.0),
            "the surviving pair is not the NEW one ({after:?}) — a rule that \
             merely caps the count could drop the fresh cast, which is worse than \
             the bug it replaces"
        );
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
    /// The link is the move: two apertures on unrelated channels go nowhere,
    /// and the recovery looks like it failed.
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
        // Up is negative y here, as everywhere in this codebase.
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

    /// A pair closes together even when only one end's clock has run out.
    ///
    /// Both ends normally tick in lockstep, so "close everything expired" and
    /// "close the whole pair" look the same. This builds the asymmetry
    /// directly. It matters once anything touches one end alone
    /// (`close_on_transit`, eviction, index reuse).
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
        // The sweep reads transits; without this message its parameter
        // validation fails and it never runs.
        app.add_message::<ambition_platformer2d::portal::PortalBodyTransited>();
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
    /// A pair with one end left swallows and never returns.
    #[test]
    fn an_expired_pair_closes_at_both_ends() {
        // A lifetime of 1.5 ticks: the close sweep is chained after the open,
        // so it ticks on the frame the pair appears. One tick or less would
        // close before a body could reach it.
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

    /// Build a world with one move-placed pair, both ends at the origin and at
    /// `(0, -40)`, sharing `pair_index` 3.
    fn pair_world(close_on_transit: bool) -> App {
        let mut app = App::new();
        app.init_resource::<ambition_platformer2d::time::WorldTime>();
        {
            let mut time = app
                .world_mut()
                .resource_mut::<ambition_platformer2d::time::WorldTime>();
            time.scaled_dt = 1.0 / 60.0;
            time.raw_dt = 1.0 / 60.0;
        }
        app.add_message::<ambition_platformer2d::portal::PortalBodyTransited>();
        app.add_message::<ambition_platformer2d::portal::PortalBodyTransited>();
        app.add_systems(Update, close_expired_move_portals);
        let channel = PortalChannel::Authored(
            ambition_platformer2d::portal::PortalChannelColor::Indexed(6),
        );
        for (pos, chan) in [
            (ae::Vec2::ZERO, channel),
            (ae::Vec2::new(0.0, -40.0), channel.partner()),
        ] {
            app.world_mut().spawn((
                PlacedPortal::fixed(chan, pos, ae::Vec2::new(0.0, -1.0), ae::Vec2::splat(12.0)),
                MovePlacedPortal {
                    // Long enough that nothing here can expire.
                    remaining_s: 60.0,
                    close_on_transit,
                    pair_index: 3,
                },
            ));
        }
        app
    }

    fn transit_to(app: &mut App, exit_pos: ae::Vec2) {
        let body = app.world_mut().spawn_empty().id();
        app.world_mut()
            .write_message(ambition_platformer2d::portal::PortalBodyTransited {
                body,
                enter_normal: ae::Vec2::new(0.0, -1.0),
                exit_normal: ae::Vec2::new(0.0, 1.0),
                facing_flip: false,
                input_warp: false,
                exit_pos,
            });
        app.update();
    }

    fn apertures_left(app: &mut App) -> usize {
        app.world_mut()
            .query::<&MovePlacedPortal>()
            .iter(app.world())
            .count()
    }

    /// The one-shot pair closes behind whoever went through it.
    #[test]
    fn a_one_shot_pair_closes_behind_whoever_used_it() {
        let mut app = pair_world(true);
        transit_to(&mut app, ae::Vec2::new(0.0, -40.0));
        assert_eq!(
            apertures_left(&mut app),
            0,
            "a one-shot pair stayed open after somebody used it"
        );
    }

    /// An ordinary pair survives being used: a sweep that closed every pair on
    /// every transit would pass the test above.
    #[test]
    fn an_ordinary_pair_survives_being_used() {
        let mut app = pair_world(false);
        transit_to(&mut app, ae::Vec2::new(0.0, -40.0));
        assert_eq!(apertures_left(&mut app), 2, "an ordinary pair closed itself");
    }

    /// A transit elsewhere is another portal's. The pair is identified by where
    /// the arrival landed; ignoring position would close it on any transit.
    #[test]
    fn a_transit_through_another_portal_leaves_this_pair_alone() {
        let mut app = pair_world(true);
        transit_to(&mut app, ae::Vec2::new(500.0, 500.0));
        assert_eq!(apertures_left(&mut app), 2);
    }

    /// Open one aimable pair with the given latched stick, and report the two
    /// aperture positions sorted so the exit (the higher one) comes first.
    fn aimed_pair(aim_tilt_degrees: f32, stick: Option<ae::Vec2>) -> Vec<ae::Vec2> {
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.add_message::<ambition_platformer2d::portal::PortalBodyTransited>();
        app.init_resource::<ambition_platformer2d::time::WorldTime>();
        app.add_systems(Update, open_authored_portal_pairs);
        let mut p = params(5.0);
        p.aim_tilt_degrees = aim_tilt_degrees;
        let body = app
            .world_mut()
            .spawn((
                ae::BodyKinematics {
                    pos: ae::Vec2::ZERO,
                    ..Default::default()
                },
                ambition_platformer2d::actor::MatchSeat(0),
            ))
            .id();
        // The latch, not a live stick: an aimed special is rooted, so a live
        // read is neutral. The teleport consumes the same fact.
        if let Some(stick) = stick {
            let spec = ambition_platformer2d::entity_catalog::MoveSpec {
                id: "aim_fixture".to_string(),
                display_name: None,
                clip: ambition_platformer2d::entity_catalog::ClipBinding {
                    clip: "special_up".to_string(),
                    fallbacks: Vec::new(),
                },
                duration_s: 1.0,
                windows: Vec::new(),
                events: Vec::new(),
                gates: Default::default(),
                start_impulse: None,
                smash_charge_mult: 1.0,
                smash_charge: None,
                charge_gesture: Default::default(),
                repeat: None,
                landing_lag_s: None,
                autocancel_after_s: None,
                sprite_spin_hz: None,
                equips: None,
                flow: None,
            };
            app.world_mut().entity_mut(body).insert(
                ambition_platformer2d::combat::moveset::MovePlayback::new(spec, 1.0)
                    .with_aimed_stick(Some(stick)),
            );
        }
        app.world_mut().write_message(ActorActionMessage {
            actor: body,
            request: ActionRequest::Special {
                spec: SpecialActionSpec::Special(PORTAL_PAIR.to_string()),
                params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&p)
                    .expect("portal params serialize"),
            },
            move_instance: None,
        });
        app.update();
        let mut out: Vec<ae::Vec2> = app
            .world_mut()
            .query::<&PlacedPortal>()
            .iter(app.world())
            .map(|portal| portal.pos)
            .collect();
        // Up is negative y, so the exit sorts first.
        out.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());
        out
    }

    /// The player's direction angles the shaft, and a neutral stick goes
    /// straight up (a panicking player's neutral stick must not lean it).
    #[test]
    fn the_players_direction_angles_the_shaft_and_neutral_goes_straight_up() {
        let straight = aimed_pair(32.0, None);
        assert_eq!(straight.len(), 2);
        assert!(
            straight[0].x.abs() < 0.001,
            "a neutral stick leaned the recovery to x={}",
            straight[0].x
        );

        let right = aimed_pair(32.0, Some(ae::Vec2::new(1.0, 0.0)));
        assert!(
            right[0].x > 100.0,
            "holding right moved the exit only {}px sideways",
            right[0].x
        );
        let left = aimed_pair(32.0, Some(ae::Vec2::new(-1.0, 0.0)));
        assert!(left[0].x < -100.0, "holding left leaned the wrong way");

        // Still a way up: at the authored 32° cap the exit stays far higher
        // than it is sideways.
        assert!(
            right[0].y < straight[0].y * 0.7,
            "the aimed exit rose only to y={} against a straight {}",
            right[0].y,
            straight[0].y
        );
    }

    /// A pair with no aim range ignores the stick (the default), so older
    /// pairs open where they always did.
    #[test]
    fn a_pair_with_no_aim_range_is_unmoved_by_the_stick() {
        let aimed = aimed_pair(0.0, Some(ae::Vec2::new(1.0, 0.0)));
        assert!(
            aimed[0].x.abs() < 0.001,
            "an unaimable pair leaned to x={}",
            aimed[0].x
        );
    }
}
