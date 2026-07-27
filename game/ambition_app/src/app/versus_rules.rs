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
#[derive(Resource, Debug)]
pub struct VersusMatch {
    pub rounds_won: [u8; 2],
    pub phase: MatchPhase,
}

impl Default for VersusMatch {
    fn default() -> Self {
        Self {
            rounds_won: [0, 0],
            phase: MatchPhase::Fighting,
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

/// Watch for a KO, count the round, and start the next one.
///
/// One system for the whole rule because the three states are one state machine
/// and splitting it across systems would put its transitions at the mercy of
/// system order — the failure mode being a round that both ends and resets in
/// the same tick, or neither.
#[allow(clippy::type_complexity)]
pub fn run_versus_rules(
    time: Res<Time>,
    roster: Option<Res<ambition::actors::character_runtime::MatchParticipantRoster>>,
    geometry: Option<ambition::platformer::lifecycle::SessionWorldRef<ae::RoomGeometry>>,
    mut state: ResMut<VersusMatch>,
    mut clock: MessageWriter<ambition::actors::time::time_control::ClockScaleRequest>,
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
    let dt = time.delta_secs();

    // FREEZE the fight for the duration of a KO or match card.
    //
    // Without this the surviving fighter keeps taking input, the CPU keeps
    // steering, attacks keep resolving and projectiles keep flying across the
    // round boundary — the card is shown over a fight that never stopped (GPT
    // 5.6, 2026-07-27). `ClockScaleRequest` at scale 0 is the engine's own
    // primitive for exactly this (its docstring names cutscene pause and boss
    // freeze), so a KO hold stops the whole simulation rather than this module
    // trying to name and silence every system that could still act.
    //
    // The hold itself is counted on the RENDER clock on purpose: it is a
    // presentation beat, and counting it on a clock it has just set to zero
    // would hold forever. What is authoritative — a fighter reaching zero
    // health, the round being awarded — is a simulation fact, and that is
    // decided below before any freeze is requested.
    if !matches!(state.phase, MatchPhase::Fighting) {
        clock.write(ambition::actors::time::time_control::ClockScaleRequest {
            domain: ambition::time::ClockDomain::SimClock,
            scale: 0.0,
            requester: ambition::actors::time::time_control::ClockRequester::Scripted,
            reason: "versus_ko_hold",
        });
    }

    match state.phase {
        MatchPhase::Fighting => {
            // A fighter at zero health loses the round. Read health rather than
            // any "is dead" marker: health is the fact, and a marker is a
            // downstream opinion about it that a versus stage does not install.
            let mut knocked_out = None;
            for (seat, health, _, _) in &fighters {
                if health.current() <= 0 {
                    knocked_out = Some(seat.0);
                    break;
                }
            }
            let Some(loser) = knocked_out else {
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
        MatchPhase::Ko { seat, remaining_s } => {
            let remaining = remaining_s - dt;
            if remaining > 0.0 {
                state.phase = MatchPhase::Ko {
                    seat,
                    remaining_s: remaining,
                };
                return;
            }
            reset_fighters(&geometry, &mut fighters);
            state.phase = MatchPhase::Fighting;
            release_freeze(&mut clock);
        }
        MatchPhase::Won { seat, remaining_s } => {
            let remaining = remaining_s - dt;
            if remaining > 0.0 {
                state.phase = MatchPhase::Won {
                    seat,
                    remaining_s: remaining,
                };
                return;
            }
            reset_fighters(&geometry, &mut fighters);
            *state = VersusMatch::default();
            release_freeze(&mut clock);
        }
    }
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
