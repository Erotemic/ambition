//! **Rounds, KOs, and a scoreboard.** (queue L8)
//!
//! Slices 1–6 built a stage and deliberately gave it no rules: a stage has to be
//! right before rules can be written, and inventing a ruleset over a stage where
//! the CPU could not move would have been ruling on a fight that was not
//! happening. It happens now — two people, two controllers, two fighters who can
//! damage each other — so the missing piece is the only one that makes it a
//! GAME rather than a sandbox: a way to win.
//!
//! ## The smallest honest ruleset
//!
//! KO at zero health, best of three, and a scoreboard. No timer, no ring-out, no
//! stocks, no stage hazards. Each of those is a real design decision with a real
//! feel cost, and shipping four of them at once means none of them was chosen —
//! it means a genre was copied. What is here is the minimum under which the
//! sentence "I won" is true.
//!
//! ## Why the rules live in the app and not the engine
//!
//! A round is not a simulation primitive. Health, damage, bodies and seats are;
//! "best of three" is a statement about a particular game, and an engine that
//! knew about it would be a fighting-game engine rather than an engine a fighting
//! game can be built on. Everything below reads engine facts (`MatchSeat`,
//! `BodyHealth`) and writes engine verbs (`transit_body`), and no engine crate
//! knows it exists.

use bevy::prelude::*;

use ambition::actors::character_runtime::{seat_placement, MatchSeat};
use ambition::characters::actor::BodyHealth;
use ambition::engine_core as ae;

/// Round wins needed to take the match. Best of three.
pub const ROUNDS_TO_WIN: u8 = 2;

/// How long the KO is held before the next round starts, in seconds of game
/// time. Long enough to see what happened and short enough that nobody reaches
/// for the controller thinking it froze.
const KO_HOLD_S: f32 = 2.0;

/// How long the match-over card stays up before the match resets.
const MATCH_HOLD_S: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MatchPhase {
    /// The round is live.
    Fighting,
    /// Somebody was knocked out; `seat` won the round.
    Ko { seat: usize, remaining_s: f32 },
    /// `seat` took the match.
    Won { seat: usize, remaining_s: f32 },
}

/// The match scoreboard.
///
/// Rounds are indexed by SEAT, not by player, because a seat is what the roster
/// declares and what a body carries — "player two" is a thing about controllers
/// and stops being well-defined the moment seat 1 is a CPU.
#[derive(Resource, Debug, Clone)]
pub struct VersusMatch {
    pub rounds_won: [u8; 2],
    pub phase: MatchPhase,
    /// The presentation half has finished holding the card and the AUTHORITATIVE
    /// half should start the next round on its next tick.
    ///
    /// The one bit that crosses between the two systems, and it crosses in the
    /// safe direction: the render clock decides WHEN a beat is over, the sim
    /// clock decides what happens because of it.
    pub reset_pending: bool,
}

impl Default for VersusMatch {
    fn default() -> Self {
        Self {
            rounds_won: [0, 0],
            phase: MatchPhase::Fighting,
            reset_pending: false,
        }
    }
}

impl VersusMatch {
    /// The seat that has taken the match, if any.
    pub fn winner(&self) -> Option<usize> {
        self.rounds_won
            .iter()
            .position(|wins| *wins >= ROUNDS_TO_WIN)
    }
}

/// **The authoritative half: who lost, who scored, and putting them back.**
///
/// Runs on the SIM schedule, because every fact it reads and every value it
/// writes is simulation state — a fighter's health, a body's position and
/// velocity, the score. The earlier version ran in `Update` under a comment
/// claiming "nothing here needs the sim schedule's ordering", which was simply
/// false: it teleported bodies between simulation boundaries and counted rounds
/// off the render clock (GPT 5.6, 2026-07-27).
///
/// The KO HOLD is not here. Holding a card is a presentation beat, and it is
/// counted by [`advance_versus_hold`] on the render clock — deliberately, since
/// the hold zeroes the sim clock and a hold counted on that clock would never
/// end. The two meet at one bit: `reset_pending`.
pub fn settle_versus_round(
    roster: Option<Res<ambition::actors::character_runtime::MatchParticipantRoster>>,
    geometry: Option<ambition::platformer::lifecycle::SessionWorldRef<ae::RoomGeometry>>,
    mut state: ResMut<VersusMatch>,
    mut fighters: Query<(
        &MatchSeat,
        &mut BodyHealth,
        ae::BodyClusterQueryData,
        &mut ambition::actors::features::MotionModel,
    )>,
) {
    // No roster means no match. The stage's own teardown removes it, so this
    // needs no second opinion about whether versus is running.
    let (Some(_roster), Some(geometry)) = (roster, geometry) else {
        return;
    };

    if state.reset_pending {
        state.reset_pending = false;
        reset_fighters(&geometry, &mut fighters);
        state.phase = if state.winner().is_some() {
            // The match was won and its card has been shown: start over.
            *state = VersusMatch::default();
            return;
        } else {
            MatchPhase::Fighting
        };
        return;
    }
    if !matches!(state.phase, MatchPhase::Fighting) {
        return;
    }

    // A fighter at zero health loses the round. Read health rather than any "is
    // dead" marker: health is the fact, and a marker is a downstream opinion
    // about it that a versus stage does not install.
    let Some(loser) = fighters
        .iter()
        .find(|(_, health, _, _)| health.current() <= 0)
        .map(|(seat, ..)| seat.0)
    else {
        return;
    };
    let winner = 1 - loser.min(1);
    if let Some(wins) = state.rounds_won.get_mut(winner) {
        *wins += 1;
    }
    state.phase = if state.winner().is_some() {
        MatchPhase::Won {
            seat: winner,
            remaining_s: MATCH_HOLD_S,
        }
    } else {
        MatchPhase::Ko {
            seat: winner,
            remaining_s: KO_HOLD_S,
        }
    };
}

/// **The presentation half: hold the card, and freeze the fight while it is up.**
///
/// Render clock on purpose — see [`settle_versus_round`]. This writes no
/// simulation state: it counts a timer, asks the engine to scale the sim clock,
/// and raises the one bit that tells the authoritative half a beat has ended.
pub fn advance_versus_hold(
    time: Res<Time>,
    roster: Option<Res<ambition::actors::character_runtime::MatchParticipantRoster>>,
    mut state: ResMut<VersusMatch>,
    mut clock: MessageWriter<ambition::actors::time::time_control::ClockScaleRequest>,
) {
    if roster.is_none() {
        return;
    }
    let dt = time.delta_secs();
    let (seat, remaining) = match state.phase {
        MatchPhase::Fighting => return,
        MatchPhase::Ko { seat, remaining_s } | MatchPhase::Won { seat, remaining_s } => {
            (seat, remaining_s - dt)
        }
    };

    // FREEZE the fight for the duration of the card.
    //
    // Without this the surviving fighter keeps taking input, the CPU keeps
    // steering, attacks keep resolving and projectiles keep flying across the
    // round boundary — the card is shown over a fight that never stopped.
    // `ClockScaleRequest` at scale 0 is the engine's own primitive for exactly
    // this (its docstring names cutscene pause and boss freeze), so a hold stops
    // the whole simulation rather than this module trying to name and silence
    // every system that could still act.
    clock.write(ambition::actors::time::time_control::ClockScaleRequest {
        domain: ambition::time::ClockDomain::SimClock,
        scale: 0.0,
        requester: ambition::actors::time::time_control::ClockRequester::Scripted,
        reason: "versus_ko_hold",
    });

    if remaining > 0.0 {
        state.phase = match state.phase {
            MatchPhase::Won { .. } => MatchPhase::Won {
                seat,
                remaining_s: remaining,
            },
            _ => MatchPhase::Ko {
                seat,
                remaining_s: remaining,
            },
        };
        return;
    }
    state.reset_pending = true;
    release_freeze(&mut clock);
}

/// Hand the clock back at full pace.
///
/// A reset rather than a scale request: reset/respawn semantics SNAP the clock
/// instead of ramping it, which is what the end of a KO hold wants — the next
/// round starts at full speed, not sliding up to it over the following second.
fn release_freeze(
    clock: &mut MessageWriter<ambition::actors::time::time_control::ClockScaleRequest>,
) {
    clock.write(ambition::actors::time::time_control::ClockScaleRequest {
        domain: ambition::time::ClockDomain::SimClock,
        scale: 1.0,
        requester: ambition::actors::time::time_control::ClockRequester::Scripted,
        reason: "versus_round_start",
    });
}

/// Put both fighters back on full health at their seats.
///
/// Through `transit_body`, the ONE transit authority (ADR 0024) — a bare
/// position write is a pose the kernel never sees, so the fighter would arrive
/// believing it is still standing where it was knocked down.
fn reset_fighters(
    geometry: &ae::RoomGeometry,
    fighters: &mut Query<(
        &MatchSeat,
        &mut BodyHealth,
        ae::BodyClusterQueryData,
        &mut ambition::actors::features::MotionModel,
    )>,
) {
    let centre = geometry.0.spawn;
    for (seat, mut health, item, mut model) in fighters.iter_mut() {
        health.health.current = health.health.max;
        let (at, facing) = seat_placement(seat.0, centre);
        let mut item = item;
        let mut clusters = item.as_clusters_mut();
        ae::movement::transit_body(
            &mut model,
            &mut clusters,
            at,
            ae::movement::TransitVelocity::Zero,
        );
        clusters.kinematics.facing = facing;
    }
}

/// Slot ids for the versus readouts.
pub const HEALTH_HUD_SLOT: &str = "versus_health";
pub const ROUNDS_HUD_SLOT: &str = "versus_rounds";
pub const ANNOUNCE_HUD_SLOT: &str = "versus_announce";

/// Publish the scoreboard.
///
/// Text, not bars. A health BAR is the right presentation and it is a renderer
/// feature the declared HUD seam does not have; shipping numbers now means the
/// rules are visible today, and the bar is a presentation change that will not
/// touch a line of the rules when it lands.
pub fn publish_versus_hud(
    state: Option<Res<VersusMatch>>,
    roster: Option<Res<ambition::actors::character_runtime::MatchParticipantRoster>>,
    fighters: Query<(&MatchSeat, &BodyHealth, &Name)>,
    mut readouts: ResMut<ambition::presentation::HudReadouts>,
) {
    let (Some(state), Some(_roster)) = (state, roster) else {
        readouts.clear_slot(HEALTH_HUD_SLOT);
        readouts.clear_slot(ROUNDS_HUD_SLOT);
        readouts.clear_slot(ANNOUNCE_HUD_SLOT);
        return;
    };

    // Sorted by seat, so the left name is always the left fighter. Query order
    // is not an order, and a scoreboard whose sides swap is worse than none.
    let mut rows: Vec<(usize, String, i32, i32)> = fighters
        .iter()
        .map(|(seat, health, name)| {
            (
                seat.0,
                name.as_str().to_string(),
                health.current(),
                health.health.max,
            )
        })
        .collect();
    rows.sort_by_key(|(seat, ..)| *seat);

    readouts.set(
        HEALTH_HUD_SLOT,
        ambition::presentation::HudReadout::bare(
            rows.iter()
                .map(|(_, name, hp, max)| format!("{name}  {hp}/{max}"))
                .collect::<Vec<_>>()
                .join("        "),
        ),
    );
    readouts.set(
        ROUNDS_HUD_SLOT,
        ambition::presentation::HudReadout::bare(format!(
            "ROUNDS  {} - {}   (first to {ROUNDS_TO_WIN})",
            state.rounds_won[0], state.rounds_won[1]
        )),
    );

    let seat_name = |seat: usize| -> String {
        rows.iter()
            .find(|(s, ..)| *s == seat)
            .map(|(_, name, ..)| name.clone())
            .unwrap_or_else(|| format!("SEAT {}", seat + 1))
    };
    match state.phase {
        MatchPhase::Fighting => readouts.clear_slot(ANNOUNCE_HUD_SLOT),
        MatchPhase::Ko { seat, .. } => readouts.set(
            ANNOUNCE_HUD_SLOT,
            ambition::presentation::HudReadout::bare(format!(
                "K.O.  {} wins the round",
                seat_name(seat)
            )),
        ),
        MatchPhase::Won { seat, .. } => readouts.set(
            ANNOUNCE_HUD_SLOT,
            ambition::presentation::HudReadout::bare(format!("{} WINS THE MATCH", seat_name(seat))),
        ),
    }
}
