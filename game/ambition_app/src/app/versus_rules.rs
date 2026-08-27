//! Versus rounds, KOs, and scoreboard rules.
//!
//! A team loses a round when all of its members are down; simultaneous defeat
//! is a draw. The match is best of three with no timer, ring-out, stocks, or
//! stage-hazard rules. These rules remain app-owned because rounds and victory
//! conditions are game policy built from engine facts rather than simulation
//! primitives.

use std::collections::BTreeMap;

use bevy::prelude::*;

use ambition_platformer2d::actors::character_runtime::{seat_placement, MatchSeat};
use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer2d::combat::targeting::MatchTeam;
use ambition_platformer2d::engine_core as ae;

/// Round wins needed to take the match. Best of three.
pub const ROUNDS_TO_WIN: u8 = 2;

/// How long the KO is held before the next round starts, in seconds. Long
/// enough to see what happened and short enough that nobody reaches for the
/// controller thinking it froze.
const KO_HOLD_S: f32 = 2.0;

/// How long the match-over card stays up before the match resets.
const MATCH_HOLD_S: f32 = 4.0;

/// How long a round-start countdown runs, in SIMULATION TICKS.
///
/// Ticks, not seconds. A countdown is the one beat where "did both machines start the round on
/// the same tick" is the whole question, so it counts the thing it actually cares about. At the
/// normal 60Hz rate this is a second and a half.
const COUNTDOWN_TICKS: u32 = 90;

/// The last stretch of the countdown reads FIGHT instead of ROUND n.
///
/// The split is what makes a countdown a countdown rather than a card: the first
/// beat tells you WHICH round, the second tells you it is about to be yours.
const FIGHT_CALL_TICKS: u32 = 30;

#[derive(Clone, Debug, PartialEq)]
pub enum MatchPhase {
    /// Fighters are reset, visible, and control-suspended during the countdown.
    /// Simulation time continues so animation, HUD, and countdown presentation
    /// advance normally while no fighter can affect the round.
    Starting { ticks_remaining: u32 },
    /// The round is live.
    Fighting,
    /// A round ended. `winner` is the team that took it, or `None` for a double
    /// KO.
    Ko {
        winner: Option<String>,
        remaining_s: f32,
    },
    /// `winner` took the match.
    Won { winner: String, remaining_s: f32 },
}

/// The match scoreboard, keyed by TEAM.
///
/// A `BTreeMap` rather than an array: teams are declared by the roster as names,
/// the number of them is whatever the roster says, and iteration order has to be
/// the same on every machine — this is rollback state.
#[derive(Resource, Debug, Clone)]
pub struct VersusMatch {
    pub rounds_won: BTreeMap<String, u8>,
    /// Which round is being fought, from 1.
    ///
    /// NOT derived from the win totals. Rounds played and rounds won are different facts and
    /// the scoreboard needs both.
    pub round: u32,
    pub phase: MatchPhase,
}

impl Default for VersusMatch {
    fn default() -> Self {
        Self::opening()
    }
}

impl VersusMatch {
    /// A fresh match, counting into round one.
    ///
    /// This IS `Default`. It was a separate constructor, and route entry reset
    /// the match with `default()` — which announced nothing — so the documented
    /// "ROUND 1 — FIGHT" card never appeared at the start of a match, only after
    /// a later round reset. One constructor, so a caller
    /// cannot pick the silent one by accident.
    pub fn opening() -> Self {
        Self {
            rounds_won: BTreeMap::new(),
            round: 1,
            phase: MatchPhase::Starting {
                ticks_remaining: COUNTDOWN_TICKS,
            },
        }
    }

    pub fn wins(&self, team: &str) -> u8 {
        self.rounds_won.get(team).copied().unwrap_or(0)
    }

    /// The team that has taken the match, if any.
    pub fn winner(&self) -> Option<&str> {
        self.rounds_won
            .iter()
            .find(|(_, wins)| **wins >= ROUNDS_TO_WIN)
            .map(|(team, _)| team.as_str())
    }
}

/// The team a body fights for.
///
/// A seated fighter always has a `MatchTeam` — the roster declares one — so the
/// fallback is for a body seated without one, and it makes that body its own
/// team. That is the honest reading of "no declared ally": a free-for-all, which
/// scores correctly, rather than everyone silently sharing a side.
fn team_of(seat: usize, team: Option<&MatchTeam>) -> String {
    team.map(|team| team.0.clone())
        .unwrap_or_else(|| format!("seat {}", seat + 1))
}

/// Who, if anyone, took the round.
///
/// the predicate itself lives in the engine now
/// (`ambition_platformer2d::combat::stocks::last_side_standing`, S4), because a stocks match
/// asks the identical question with a different liveness input: a round asks
/// "is this fighter's health above zero", a stocks match asks "does this
/// fighter have a stock left", and everything after that question is the same.
/// Two copies of "a side is out when every member is out" is exactly the kind of
/// near-duplicate that drifts on the three-side case.
///
/// What stays here is the ADAPTER: rounds are settled on health.
use ambition_platformer2d::combat::stocks::{last_side_standing, SidesOutcome as RoundResult};

fn round_result<'a>(
    rows: impl Iterator<Item = (usize, Option<&'a MatchTeam>, i32)>,
) -> Option<RoundResult> {
    // `team_of` stays the naming authority — it numbers a teamless seat from ONE,
    // and the stage's scoreboard reads the same function.
    last_side_standing(rows.map(|(seat, team, health)| (team_of(seat, team), health > 0)))
}

type FighterQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static MatchSeat,
        Option<&'static MatchTeam>,
        &'static mut BodyHealth,
        ae::BodyClusterQueryData,
        &'static mut ambition_platformer2d::actor::MotionModel,
        // The swing a fighter was mid-way through when the round ended. Carried
        // by VALUE because cancelling a move means despawning the strike boxes
        // it derived, and only the playback knows which entities those are.
        Option<&'static mut ambition_platformer2d::combat::moveset::MovePlayback>,
    ),
>;

/// Run the versus ruleset entirely on the simulation schedule.
/// Round-hold timing uses unscaled `WorldTime::wall_dt` because the hold itself
/// may freeze scaled simulation time.
#[allow(clippy::too_many_arguments)]
/// Whether the starting countdown may tick down.
///
/// The count runs only while a match is ACTIVE — every participant has a body.
fn countdown_may_advance(active_match: bool) -> bool {
    active_match
}

pub fn settle_versus_round(
    time: Res<ambition_platformer2d::time::WorldTime>,
    roster: Option<Res<ambition_platformer2d::actors::character_runtime::MatchParticipantRoster>>,
    geometry: Option<
        ambition_platformer2d::platformer::lifecycle::SessionWorldRef<ae::RoomGeometry>,
    >,
    // Whether every participant on the roster actually HAS a body yet.
    // Seating retries until they all do; the countdown must not run ahead of it.
    active_match: Option<Res<ambition_platformer2d::actors::character_runtime::ActiveMatch>>,
    mut state: ResMut<VersusMatch>,
    mut commands: Commands,
    // A round boundary must not enumerate the transient entity families that might exist.
    mut rounds: ResMut<ambition_platformer2d::platformer::lifecycle::ActiveRoundScope>,
    mut fighters: FighterQuery,
    mut reactions: Query<&mut BodyCombat, With<MatchSeat>>,
    mut holds: Query<&mut ambition_platformer2d::characters::control::ControlHolds>,
    mut firing: Query<
        &mut ambition_platformer2d::projectiles::PlayerProjectileState,
        With<MatchSeat>,
    >,
    mut scale: MessageWriter<ambition_platformer2d::time::time_control::ClockScaleRequest>,
    mut snap: MessageWriter<ambition_platformer2d::time::time_control::ClockResetRequest>,
) {
    // No roster means no match. The stage's own teardown removes it, so this
    // needs no second opinion about whether versus is running.
    let (Some(_roster), Some(geometry)) = (roster, geometry) else {
        return;
    };
    let dt = time.wall_dt();

    let (winner, remaining_s) = match state.phase.clone() {
        MatchPhase::Starting { ticks_remaining } => {
            // THE COUNTDOWN WAITS FOR THE ROSTER.
            //
            // This arm decremented as soon as the roster resource and the room
            // geometry existed, and neither of those means a fighter has a
            // body. `seat_match_participants` retries until every participant
            // is seated and only THEN sets `MatchSeated` — so the count could
            // run, and the round could go live, while a seat was still empty.
            // A participant arriving afterwards joined a round already in
            // progress, having never passed the countdown or its reset
            // boundary, which is the one place a round equalises its fighters
            // .
            //
            // Holding at the initial value rather than pausing mid-count: a
            // countdown that starts, stalls and resumes reads as a stutter, and
            // the fighters are held anyway by the marker below.
            let everybody_is_here = countdown_may_advance(active_match.is_some());

            // Hold the controls EVERY tick, not once on entry.
            //
            // `try_insert` is idempotent, and asking every tick is what makes
            // this correct for the two ways a round can begin: after a KO the
            // marker is already on (the KO arm put it there and `begin_round` no
            // longer takes it off), while at match start the fighters were
            // seated by the stage and have never had it. One arm, both cases,
            // and no dependence on which path arrived here.
            take_the_controls(
                &mut commands,
                fighters.iter().map(|(entity, ..)| entity),
                ambition_platformer2d::characters::control::ControlHold::Opening,
            );

            if !everybody_is_here {
                // Held, not paused: the phase keeps its full count so the round
                // begins with the whole card once the last fighter lands.
                return;
            }

            let left = ticks_remaining.saturating_sub(1);
            if left > 0 {
                state.phase = MatchPhase::Starting {
                    ticks_remaining: left,
                };
                return;
            }

            // Open the round lifetime when the round actually goes live. Minting
            // scope only during transitions would leave the initial round unscoped.
            rounds.begin();
            state.phase = MatchPhase::Fighting;
            for (entity, ..) in fighters.iter() {
                hand_back_the_controls(&mut commands, entity, &mut holds);
            }
            // Whatever was mashed during the countdown does NOT carry into the
            // round.
            //
            // Suspending control stops a body ACTING on input; it does not stop
            // the buffers that exist to make inputs forgiving from filling up
            // while it is suspended. A player holding fire through "3, 2, 1"
            // would open the round with a charged shot they did not time, and a
            // quarter-circle rolled during the count would still be inside the
            // 0.45s motion window when the count ended. The countdown's whole
            // promise is that both fighters start the round equal, and a stale
            // edge is the cheapest way to break it.
            for mut projectile_state in &mut firing {
                projectile_state.motion_buffer.clear();
                projectile_state.charging = None;
            }
            return;
        }
        MatchPhase::Fighting => {
            // Read health rather than any "is dead" marker: health is the fact,
            // and a marker is a downstream opinion about it that a versus stage
            // does not install.
            let Some(result) = round_result(
                fighters
                    .iter()
                    .map(|(_, seat, team, health, ..)| (seat.0, team, health.current())),
            ) else {
                return;
            };
            let winner = match result {
                RoundResult::Winner(team) => {
                    *state.rounds_won.entry(team.clone()).or_insert(0) += 1;
                    Some(team)
                }
                RoundResult::Draw => None,
            };
            state.phase = match state.winner() {
                Some(champion) => MatchPhase::Won {
                    winner: champion.to_string(),
                    remaining_s: MATCH_HOLD_S,
                },
                None => MatchPhase::Ko {
                    winner,
                    remaining_s: KO_HOLD_S,
                },
            };
            // FREEZE ON THIS TICK, not the next one.
            //
            // A fighter could take a hit after losing.
            request_freeze(&mut scale);
            // AND TAKE THE CONTROLS AWAY, for the same tick and the same reason.
            //
            // The freeze is a TARGET: the clock smoother ramps the live scale
            // toward zero rather than snapping it, deliberately, because a KO
            // decelerating into the card is the genre's own beat. Nothing else
            // reads `MatchPhase` — not input, not the brains, not move
            // triggering, not damage — so for the length of that ramp the
            // fighters went on accepting control, walking, starting moves and
            // spawning strikes, after the score had already been incremented and
            // the winner named. Slow motion is a feel
            // decision; deciding a round and then letting it keep being fought
            // is not.
            //
            // `ScriptedControl` is the engine's existing word for "a sequence is
            // driving this body, it does not answer input", and a KO card is
            // exactly that sequence. It gates the DECISION and leaves everything
            // physical alone, so a body already in the air still arcs, a move
            // already playing still finishes, and both do it decelerating.
            take_the_controls(
                &mut commands,
                fighters.iter().map(|(entity, ..)| entity),
                ambition_platformer2d::characters::control::ControlHold::Interlude,
            );
            return;
        }
        MatchPhase::Ko {
            winner,
            remaining_s,
        } => (winner, remaining_s),
        MatchPhase::Won {
            winner,
            remaining_s,
        } => (Some(winner), remaining_s),
    };

    let match_over = matches!(state.phase, MatchPhase::Won { .. });
    let remaining = remaining_s - dt;
    if remaining > 0.0 {
        // FREEZE the fight for the duration of the card.
        //
        // Without this the surviving fighter keeps taking input, the CPU keeps
        // steering, attacks keep resolving and projectiles keep flying across
        // the round boundary — the card is shown over a fight that never
        // stopped. `ClockScaleRequest` at scale 0 is the engine's own primitive
        // for exactly this (its docstring names cutscene pause and boss freeze),
        // so a hold stops the whole simulation rather than this module trying to
        // name and silence every system that could still act.
        //
        // Re-asked EVERY tick because a frame with no request leaves the target
        // untouched: absence of an opinion is not an opinion, and the systems
        // that ask for full pace every frame would otherwise thaw the hold.
        request_freeze(&mut scale);
        state.phase = if match_over {
            MatchPhase::Won {
                winner: winner.expect("a won match has a winner"),
                remaining_s: remaining,
            }
        } else {
            MatchPhase::Ko {
                winner,
                remaining_s: remaining,
            }
        };
        return;
    }

    // The hold is over. A RESET rather than a scale request: reset/respawn
    // semantics snap the clock instead of ramping it, which is what the end of a
    // KO hold wants — the next round starts at full speed, not sliding up to it
    // over the following second. A scale request cannot do this job at all,
    // because `min` keeps the strongest slow and the freeze is still in force.
    snap.write(
        ambition_platformer2d::time::time_control::ClockResetRequest::sim_clock(
            ambition_platformer2d::time::time_control::ClockRequester::Scripted,
            "versus_round_start",
        ),
    );
    begin_round(
        &geometry,
        &mut fighters,
        &mut reactions,
        &mut commands,
        &mut rounds,
    );
    *state = if match_over {
        VersusMatch::opening()
    } else {
        VersusMatch {
            rounds_won: std::mem::take(&mut state.rounds_won),
            // The round ADVANCES whether or not anybody scored. A draw scores
            // for no team, so a counter derived from win totals repeats itself.
            round: state.round + 1,
            // Into the COUNTDOWN, not straight into the fight. Every round gets
            // the same opening, including the first — a countdown that only the
            // later rounds have is a countdown that tells you the match already
            // started without you.
            phase: MatchPhase::Starting {
                ticks_remaining: COUNTDOWN_TICKS,
            },
        }
    };
}

/// Ask the engine to stop the fight.
///
/// The clock SMOOTHER ramps the live scale toward this target rather than snapping it, and that
/// is left alone deliberately: a KO decelerating into the card is the genre's own beat, it is
/// the same primitive hitstop uses, and an instant stop is a feel decision this stage has not
/// made.
///
/// Paired with the removal in [`begin_round`], and nowhere else: a marker that
/// suspends control is only as good as the one place that takes it off.
fn take_the_controls(
    commands: &mut Commands,
    fighters: impl Iterator<Item = Entity>,
    hold: ambition_platformer2d::characters::control::ControlHold,
) {
    for fighter in fighters {
        // which hold, said at the call site. The countdown and the KO card
        // are two different authorities that happen to want the same thing, and
        // a capture can be holding one of these same fighters besides. Whoever
        // releases first must not free a body another still holds, and that is
        // arithmetic now rather than an argument about which features overlap.
        ambition_platformer2d::characters::control::claim_control_hold(commands, fighter, hold);
    }
}

/// GO: every ceremony hold this stage is responsible for ends here.
///
/// BOTH bits, and leaving one out froze the stage solid. Two ceremonies
/// converge on this instant: the countdown this stage runs, and the OPENING the
/// ruleset declared with `opens_suspended`. The engine's own
/// `release_the_opening_hold` deliberately declines the second — *"a ceremony
/// this ruleset did not declare is not this system's to end"*, because a
/// composition with `opening_countdown_ticks == 0` has named this stage's GO as
/// the owner. So this is where it is owed. While this released only its own
/// countdown, the ruleset's opening hold survived GO forever and the round went
/// live with two fighters who could not move.
fn hand_back_the_controls(
    commands: &mut Commands,
    fighter: Entity,
    holds: &mut Query<&mut ambition_platformer2d::characters::control::ControlHolds>,
) {
    use ambition_platformer2d::characters::control::{release_control_hold, ControlHold};
    let mut held = holds.get_mut(fighter).ok();
    for hold in [ControlHold::Opening, ControlHold::Interlude] {
        release_control_hold(commands, fighter, held.as_deref_mut(), hold);
    }
}

fn request_freeze(
    scale: &mut MessageWriter<ambition_platformer2d::time::time_control::ClockScaleRequest>,
) {
    scale.write(
        ambition_platformer2d::time::time_control::ClockScaleRequest {
            domain: ambition_platformer2d::time::ClockDomain::SimClock,
            scale: 0.0,
            requester: ambition_platformer2d::time::time_control::ClockRequester::Scripted,
            reason: "versus_ko_hold",
        },
    );
}

/// Put the fighters back, and leave the last round behind with them.
///
/// The other half is that a KO pause FREEZES the world rather than emptying it: the smash that
/// was mid-swing, the box it had spawned, the shot crossing the stage and the hitstun on the
/// fighter who ate it are all still there, and they resume into round two — which can kill a
/// fighter who has not yet been given a chance to move.
///
/// What this function can do ITSELF stops at the engine's own clusters. The
/// fighters are authored by providers, and a provider's transient state is
/// reached through [`ae::BodyRestarted`] rather than guessed at from here — see
/// the trigger below.
///
/// Removing `MovePlayback` is what retires the strike volumes: their existence
/// is DERIVED from `(owner's playback t, window)` and
/// `retire_orphaned_strike_volumes` enforces that derivation against the world
/// every frame, so an owner with no playback has no boxes. This deliberately
/// does not despawn them itself — a second authority over a volume's lifetime is
/// how the two come to disagree.
fn begin_round(
    geometry: &ae::RoomGeometry,
    fighters: &mut FighterQuery,
    reactions: &mut Query<&mut BodyCombat, With<MatchSeat>>,
    commands: &mut Commands,
    rounds: &mut ambition_platformer2d::platformer::lifecycle::ActiveRoundScope,
) {
    // NOTHING IN FLIGHT CROSSES THE ROUND BOUNDARY, and this function no
    // longer knows what "in flight" is made of.
    //
    // Every family added after it needed another query HERE, and forgetting one fails silently:
    // an entity from the previous round is simply still there, in a round that never asked for
    // it. Minting the next round is now the whole statement; `despawn_departed_round_entities`
    // culls whatever belonged to the last one.
    rounds.begin();
    let centre = geometry.0.spawn;
    for (entity, seat, _, mut health, item, mut model, playback) in fighters.iter_mut() {
        health.health.current = health.health.max;
        let (at, facing) = seat_placement(seat.0, centre);
        let mut item = item;
        let mut clusters = item.as_clusters_mut();
        // The RESET authority, not the transit one.
        //
        // `transit_body` is right for a teleport and wrong for a round boundary:
        // it documents that "axis maneuver state (coyote, buffers, dash timers)
        // is deliberately KEPT — those are time facts, not place facts". True of
        // a blink; false of a new round. A fighter opened round two holding a
        // buffered maneuver, a live dash timer, or a shield/dodge state left over
        // from the frame it was knocked out on.
        //
        // `reset_body_clusters` is the verb that means "this body starts again",
        // the same one the sandbox reset uses, and it clears the whole
        // movement/ability cluster set rather than the place-facts subset.
        ae::reset_body_clusters(
            &mut model,
            &mut clusters,
            at,
            // The versus stage does not override the air game; a stage that did
            // would pass its own number, which is why this is asked rather than
            // assumed.
            ae::DEFAULT_TUNING.air_jumps,
        );
        clusters.kinematics.facing = facing;
        //  through the ONE teardown path. Stripping the component alone
        // leaves the swing's strike boxes standing until the next tick's orphan
        // sweep, so an entity from the previous round outlives the round -- the
        // exact thing the `rounds.begin()` above exists to stop.
        if let Some(mut playback) = playback {
            ambition_platformer2d::combat::moveset::cancel_move_playback(
                commands,
                entity,
                &mut playback,
                // ⭐ A ROUND RESET IS THE BODY LEAVING PLAY, so a storing charge
                // does not survive it into the next round. See `MoveEnd`.
                ambition_platformer2d::combat::moveset::MoveEnd::LeftPlay,
            );
        }
        // The controls deliberately do NOT come back here.
        //
        // A round now opens on a COUNTDOWN, and the fighters are reset and visible through all
        // of it without being able to act — so the suspension has to outlive this reset and end
        // at the one place the round actually goes live, which is the `Starting` arm reaching
        // zero. The half of the reset this module cannot perform itself — a ball-dash charge, a
        // rolling form, a spark cadence — is announced by the ENGINE, not from here.
        // `reset_body_clusters` raises the pending flag and `announce_body_restarts` triggers
        // `ae::BodyRestarted` at the front of the next tick, so every provider hears about this
        // round boundary the same way it hears about a death respawn.
        //
        // This module used to trigger it by hand, which worked and was exactly
        // the wrong shape: seven other production resets did NOT, so ordinary
        // play still leaked provider state and only the versus stage did not
        // .
    }
    // Hitstun, recoil lock, i-frames and the damage blink are all round-scoped
    // reactions to a fight that is over.
    for mut combat in reactions.iter_mut() {
        combat.reset();
    }
}

/// Slot ids for the versus readouts — one health slot per seat.
pub const HEALTH_HUD_SLOTS: [&str; 4] = [
    "versus_health_seat_1",
    "versus_health_seat_2",
    "versus_health_seat_3",
    "versus_health_seat_4",
];
pub const ROUNDS_HUD_SLOT: &str = "versus_rounds";
pub const ANNOUNCE_HUD_SLOT: &str = "versus_announce";

/// Publish the scoreboard.
pub fn publish_versus_hud(
    state: Option<Res<VersusMatch>>,
    roster: Option<Res<ambition_platformer2d::actors::character_runtime::MatchParticipantRoster>>,
    fighters: Query<(&MatchSeat, Option<&MatchTeam>, &BodyHealth, &Name)>,
    mut readouts: ResMut<ambition_platformer2d::presentation::HudReadouts>,
) {
    let (Some(state), Some(_roster)) = (state, roster) else {
        for slot in HEALTH_HUD_SLOTS {
            readouts.clear_slot(slot);
        }
        readouts.clear_slot(ROUNDS_HUD_SLOT);
        readouts.clear_slot(ANNOUNCE_HUD_SLOT);
        return;
    };

    // Sorted by seat, so the left name is always the left fighter. Query order
    // is not an order, and a scoreboard whose sides swap is worse than none.
    let mut rows: Vec<(usize, String, String, i32, i32)> = fighters
        .iter()
        .map(|(seat, team, health, name)| {
            (
                seat.0,
                team_of(seat.0, team),
                name.as_str().to_string(),
                health.current(),
                health.health.max,
            )
        })
        .collect();
    rows.sort_by_key(|(seat, ..)| *seat);

    // One GAUGE per fighter, plus the number. A bar is what a player reads at a glance
    // mid-fight — "am I nearly dead" — and the number is what they read when they want to know
    // exactly.
    let mut written = [false; HEALTH_HUD_SLOTS.len()];
    for (seat, _, name, hp, max) in &rows {
        let Some(slot) = HEALTH_HUD_SLOTS.get(*seat) else {
            continue;
        };
        written[*seat] = true;
        readouts.set(
            *slot,
            ambition_platformer2d::presentation::HudReadout::gauge(
                name.clone(),
                format!("{hp}/{max}"),
                if *max > 0 {
                    *hp as f32 / *max as f32
                } else {
                    0.0
                },
            ),
        );
    }
    // A 1v1 declares four slots and fills two. An unwritten slot must be
    // CLEARED, not left holding the previous match's fourth fighter.
    for (index, slot) in HEALTH_HUD_SLOTS.iter().enumerate() {
        if !written[index] {
            readouts.clear_slot(*slot);
        }
    }

    // The scoreboard names TEAMS, in the order the seats introduce them, so the
    // left-hand team is the one the left-hand fighter belongs to.
    let mut teams: Vec<&String> = Vec::new();
    for (_, team, ..) in &rows {
        if !teams.contains(&team) {
            teams.push(team);
        }
    }
    let score = teams
        .iter()
        .map(|team| format!("{team} {}", state.wins(team)))
        .collect::<Vec<_>>()
        .join("  -  ");
    readouts.set(
        ROUNDS_HUD_SLOT,
        ambition_platformer2d::presentation::HudReadout::bare(if score.is_empty() {
            String::new()
        } else {
            format!("ROUNDS  {score}   (first to {ROUNDS_TO_WIN})")
        }),
    );

    match &state.phase {
        // The countdown reads in two beats: WHICH round, then GO. Both are
        // derived from the same tick counter the simulation is already using, so
        // the card cannot disagree with the phase — a presentation timer running
        // beside a simulation timer is two clocks for one fact.
        MatchPhase::Starting { ticks_remaining } => readouts.set(
            ANNOUNCE_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::bare(
                if *ticks_remaining > FIGHT_CALL_TICKS {
                    format!("ROUND {}", state.round)
                } else {
                    "FIGHT".to_string()
                },
            ),
        ),
        MatchPhase::Fighting => readouts.clear_slot(ANNOUNCE_HUD_SLOT),
        MatchPhase::Ko { winner, .. } => readouts.set(
            ANNOUNCE_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::bare(match winner {
                Some(team) => format!("K.O.  {team} wins the round"),
                None => "DOUBLE K.O.  —  the round is a draw".to_string(),
            }),
        ),
        MatchPhase::Won { winner, .. } => readouts.set(
            ANNOUNCE_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::bare(format!(
                "{winner} WINS THE MATCH"
            )),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The countdown waits for the roster.
    ///
    /// It decremented as soon as the roster resource and the room geometry
    /// existed, and neither means a fighter has a BODY. Seating retries until
    /// every participant is seated and only then sets `MatchSeated`, so the
    /// count could run — and the round go live — with a seat still empty. A
    /// participant landing afterwards joins a round in progress, having never
    /// passed the countdown or its reset boundary, which is the one place a
    /// round equalises its fighters.
    ///
    /// Tested as a PREDICATE, not through the app, and the reason is worth keeping: an
    /// app-level version cannot express the scenario. Reaching the real case needs a
    /// participant that CANNOT seat.
    ///
    /// ✔ that case IS producible, and is driven end-to-end by
    /// `a_roster_with_an_unseatable_participant_never_latches` in
    /// `ambition_platformer2d_actor_monolith`: a roster naming a character nothing registered never
    /// completes, so the match never activates.
    #[test]
    fn the_countdown_holds_until_every_seat_is_filled() {
        assert!(
            !countdown_may_advance(false),
            "no ACTIVE match means no fighters have bodies yet, and a countdown \
             that runs there opens a round a participant never entered"
        );
        assert!(
            countdown_may_advance(true),
            "and it releases the moment the match is active — every participant \
             seated, published in one insert"
        );
    }

    fn team(name: &str) -> MatchTeam {
        MatchTeam::new(name)
    }

    /// A fighter's defeat must never score for its own side.
    ///
    /// The old rule was `1 - loser.min(1)`: "the other index", which is only the
    /// other side when there are exactly two bodies. Seat 2 is on blue, so blue
    /// going down mapped to `loser.min(1) == 1` and awarded the round to index
    /// 0 — blue again.
    #[test]
    fn a_seat_above_the_first_two_scores_for_the_other_team() {
        let blue = team("blue");
        let red = team("red");
        let result = round_result(
            [
                (0, Some(&blue), 0),
                (1, Some(&red), 40),
                (2, Some(&blue), 0),
                (3, Some(&red), 12),
            ]
            .into_iter(),
        );
        assert!(
            matches!(&result, Some(RoundResult::Winner(team)) if team == "red"),
            "blue was wiped out, so red takes the round"
        );
    }

    /// A team is out only when EVERY member is down — one partner standing is a
    /// round that continues.
    #[test]
    fn a_team_survives_while_one_partner_stands() {
        let blue = team("blue");
        let red = team("red");
        assert!(round_result(
            [
                (0, Some(&blue), 0),
                (1, Some(&red), 40),
                (2, Some(&blue), 7),
                (3, Some(&red), 12),
            ]
            .into_iter()
        )
        .is_none());
    }

    #[test]
    fn both_sides_falling_together_is_a_draw() {
        let blue = team("blue");
        let red = team("red");
        assert!(matches!(
            round_result([(0, Some(&blue), 0), (1, Some(&red), 0)].into_iter()),
            Some(RoundResult::Draw)
        ));
    }

    /// One team is not a match. Without the guard the sole team is "wiped out"
    /// the moment it falls and the stage scores a round against nobody.
    #[test]
    fn a_single_team_never_wins_a_round() {
        let blue = team("blue");
        assert!(round_result([(0, Some(&blue), 0), (1, Some(&blue), 0)].into_iter()).is_none());
    }

    /// A body seated with no declared team is its own team — a free-for-all,
    /// which scores, rather than everyone silently sharing a side, which cannot.
    #[test]
    fn teamless_seats_are_their_own_sides() {
        assert!(matches!(
            round_result([(0, None, 0), (1, None, 55)].into_iter()),
            Some(RoundResult::Winner(team)) if team == "seat 2"
        ));
    }

    #[test]
    fn the_match_is_won_at_two_rounds() {
        let mut state = VersusMatch::default();
        assert_eq!(state.winner(), None);
        state.rounds_won.insert("red".into(), 1);
        assert_eq!(state.winner(), None);
        state.rounds_won.insert("red".into(), ROUNDS_TO_WIN);
        assert_eq!(state.winner(), Some("red"));
    }
}
