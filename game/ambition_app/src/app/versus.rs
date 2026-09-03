//! Nothing called it. C4's complaint was never that the fight did not work; it was that the fight
//! existed only where a test could see it, and "a stranger can run it and watch" is the whole
//! difference between an engine demo and an engine.
//!
//! So this is a route. It appears in the launcher beside Ambition, Sanic and
//! Mary-O, it prepares a flat arena, and it seats two CPU fighters facing each
//! other. Nobody has to read a test to know whether two characters can fight.
//!
//! ## Why it lives in the app and not in a provider crate
//!
//! The fighters are `mary_o` and `sanic`, registered by two DIFFERENT provider
//! plugins. The only composition where both casts exist is the multi-game shell
//! host, so the app is the only honest home for a match between them — and a
//! crossover is exactly what the multi-provider character work has been building
//! toward. A provider that wanted its own single-cast versus mode would author
//! the same thing inside itself.
//!
//! ## What it deliberately does not do

use bevy::prelude::*;

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition_platformer2d::runtime::demo_fixture::{
    ActiveRoomMetadata, RoomSet, StartingCharacter,
};
use ambition_platformer2d::runtime::PreparedPlatformerSource;
use ambition_platformer2d::versus_match::{
    ControllerBinding, MatchParticipant, MatchParticipantRoster, RosterSeating, StagesCharacters,
};
use ambition_platformer2d::world::rooms::RoomSpec;

pub const VERSUS_EXPERIENCE: &str = "ambition_versus";
pub const VERSUS_GAMEPLAY_ROUTE: &str = "versus_gameplay";
pub const VERSUS_ROOM_ID: &str = "versus_arena";

/// The arena fighters; their character definitions share art ids with the demos.
const FIGHTERS: [&str; 2] = ["arena_duelist_long", "arena_duelist_close"];

/// Open arena with recovery platforms and authored blast margins. Crossing a
/// blast margin produces `ResetCause::LeftTheWorld`, which the versus rules score
/// as a fighter death. No ceiling blast zone is currently authored.
fn versus_arena() -> RoomSpec {
    let size = ae::Vec2::new(960.0, 540.0);
    let platform_top = 400.0;
    // Seating spreads fighters up to ±144px about the centre, so a 480-wide
    // main platform holds a full four-seat 2v2 with room either side.
    let main_half_width = 240.0;
    let world = ae::World::new(
        "Versus Arena",
        size,
        // Spawn at the centre of the main platform: seating places fighters
        // symmetrically about this point, so it IS the middle of the stage.
        ae::Vec2::new(size.x * 0.5, platform_top - 24.0),
        vec![
            ae::Block::solid(
                "arena_platform_main",
                ae::Vec2::new(size.x * 0.5 - main_half_width, platform_top),
                ae::Vec2::new(main_half_width * 2.0, 40.0),
            ),
            // Recovery ledges. Solid rather than one-way: a fighter thrown out
            // sideways needs something to catch, and a platform they can fall
            // straight back through is not a second chance.
            ae::Block::solid(
                "arena_platform_left",
                ae::Vec2::new(96.0, platform_top - 96.0),
                ae::Vec2::new(128.0, 20.0),
            ),
            ae::Block::solid(
                "arena_platform_right",
                ae::Vec2::new(size.x - 224.0, platform_top - 96.0),
                ae::Vec2::new(128.0, 20.0),
            ),
        ],
    )
    .with_fall_out_margin(96.0)
    // The SIDE blast zone, which is where a platform fighter actually loses
    // most of its stocks. Without it a fighter thrown off the left edge only
    // dies once its arc happens to carry it below the stage, which reads as the
    // throw not having worked. 160px is deliberately looser than the 96px
    // floor: you are given further to recover from a horizontal launch than
    // from a straight drop, because a horizontal launch is the one you can act
    // on.
    .with_side_out_margin(160.0);
    let mut room = RoomSpec::new(VERSUS_ROOM_ID, world);
    room.metadata.mode = Some(VERSUS_EXPERIENCE.to_owned());
    room
}

fn versus_prepared_session_world() -> PreparedPlatformerSource {
    let room = versus_arena();
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    // `for_match`: no home body. The comment this replaces described the
    // old contract exactly — *"the body a human seat takes over"* — and that
    // takeover is the fork that has been deleted. Every fighter is built by the
    // match now; `FIGHTERS[0]` remains as this experience's catalog default.
    PreparedPlatformerSource::for_match(
        VERSUS_EXPERIENCE,
        RoomSet::from_parts(VERSUS_ROOM_ID, vec![room], Vec::new()),
        geometry,
        metadata,
        StartingCharacter::new(FIGHTERS[0]),
    )
}

/// The roster seated when the stage begins.
///
/// Seat 0 adopts the session's existing human body. Additional connected
/// controllers receive human seats; remaining seats are CPUs. The roster is fixed
/// at stage entry so device changes cannot mutate a running match.
///
/// `seat_for` alternates arena sides, and teams follow the same parity. `MatchTeam`
/// is authoritative for team damage because human fighters otherwise share the
/// same player faction. Up to four seats are supported.
///
/// The versus experience owns and registers the CPU archetype requested here.
pub const VERSUS_CPU_BRAIN: &str = "versus_duelist";

// its body half went nowhere because nothing read it. `max_health`,
// `run_speed`, `melee`, `move_style` and `respawn` stopped being read the day a
// seat was built from its CHARACTER (P1.11) — see the `fighter_abilities` note
// below, which is the record of that: the row's authored `melee` "reached the
// body regardless of what the match said the body could do", and removing it is
// what exposed this stage's missing `attack` verb.

/// The roster the route PROPOSES on entry, built from live device discovery
/// before any session has decided its seating.
pub fn versus_roster(local_players: usize) -> MatchParticipantRoster {
    versus_roster_from(local_players, RosterSeating::Proposed)
}

/// The same, in a stated seating lifecycle.
pub fn versus_roster_from(local_players: usize, seating: RosterSeating) -> MatchParticipantRoster {
    // Two seats minimum (there is always an opponent, human or not) and four
    // maximum (the arena is one screen wide, and `SlotControls` holds four).
    let seats = local_players.clamp(2, MAX_VERSUS_SEATS);
    let participants = (0..seats)
        .map(|seat| {
            let controller = if seat < local_players {
                ControllerBinding::Human {
                    // PAD `seat`, and this route is entitled to say so. The
                    // versus roster is built from live device discovery — seat
                    // `n` IS the `n`-th pad, with no lobby in between to renumber
                    // anything — which is exactly the case where the source and
                    // the channel coincide. A screen that lets people CHOOSE a
                    // controller (Smash's) is the case where they do not.
                    source: ambition_platformer2d::actor::LocalInputSource::Pad(seat as u8),
                }
            } else {
                ControllerBinding::Cpu {
                    brain_profile: Some(VERSUS_CPU_BRAIN.into()),
                }
            };
            MatchParticipant::new(FIGHTERS[seat % FIGHTERS.len()])
                .driven_by(controller)
                .on_team(if seat % 2 == 0 { "blue" } else { "red" })
        })
        .collect();
    MatchParticipantRoster {
        participants,
        rules: ambition_platformer2d::versus_match::MatchRules {
            // No items: these rounds are two duelists and a health bar, and an
            // item needs somewhere authored to land.
            item_spawns: None,
            // The stage ceremony releases this suspension when the round goes live.
            opens_suspended: true,
            // The stage owns its countdown; the engine-level countdown is disabled.
            opening_countdown_ticks: 0,
            // Rounds end on health rather than a match clock.
            time_limit_ticks: 0,
            // The duel declares one symmetric capability ceiling. `at_most`
            // preserves character-authored abilities below that ceiling instead
            // of granting a common floor, so excluded verbs stay excluded.
            abilities: Some(ae::MatchAbilities::at_most(
                super::versus_fighters::VERSUS_FIGHTER_KIT,
            )),
            // This duel does not impose platform-fighter body defaults.
            body: None,
            // Health rounds, not stocks. Opting into stocks also changes fighter
            // death policy.
            stocks: None,
            // Preserve the duelists' authored health pools. A host seating
            // foreign characters must provide its own normalization.
            health_pool: None,
            ..Default::default()
        },
        seating,
        // WHOSE MATCH THIS IS. The exit rule below removes the roster it
        // finds; with a second stage in the same host publishing one from its
        // own route, "the roster" stopped meaning "mine".
        published_by: Some(VERSUS_EXPERIENCE.to_owned()),
    }
}

/// One screen, four fighters, four `SlotControls` slots.
pub const MAX_VERSUS_SEATS: usize = 4;

/// Keep the versus roster route-scoped and reconcile an unactivated roster with
/// the seat topology once rollback freezes it. Route entry precedes rollback
/// startup, so live device discovery may change before topology becomes canonical.
///
/// `ActiveMatch` absence does not imply that no seats have been materialized;
/// seating may span ticks before the activation latch closes.
fn reconcile_roster_with_frozen_topology(
    mut commands: Commands,
    topology: Option<Res<ambition_platformer2d::input::LocalSeatTopology>>,
    roster: Option<ResMut<MatchParticipantRoster>>,
    active_match: Option<ResMut<ambition_platformer2d::versus_match::ActiveMatch>>,
    mut demand: ResMut<ambition_platformer2d::characters::load_demand::CharacterLoadDemand>,
    // Bodies that are ALREADY seated, latch or no latch. This is the fact the
    // `ActiveMatch` check was standing in for, and the two are not the same fact.
    seated: Query<&ambition_platformer2d::versus_match::MatchSeat>,
    // OPTIONAL because a composition legitimately publishes none (an engine App with no content),
    // and `engine.character-authority-is-app-local` means "not part of this composition" is a real
    // answer rather than a fault.
    //
    // a `Res<CharacterRoster>` stood beside this and was passed FIRST. An
    // enemy archetype table cannot answer a controller question — it stopped
    // being able to when `seat_brain_profile`'s archetype arm went (P2.18) — and
    // a validator that consults an authority the runtime does not is a validator
    // that approves what seating then refuses.
    profiles: Option<
        Res<ambition_platformer2d::characters::actor::character_catalog::BrainProfileRegistry>,
    >,
) {
    let (Some(topology), Some(mut roster)) = (topology, roster) else {
        return;
    };
    // MINE, not "a roster exists" — the same rule `maintain_versus_stage`
    // learned and this function did not.
    //
    // `versus_roster_from` stamps `published_by: ambition_versus`, so rebuilding somebody
    // else's roster here does not just resize it: it TRANSFERS OWNERSHIP.
    //
    // it only bites where a topology is actually FROZEN — and that is EVERY
    // build now, not a `dev_tools` one.
    //
    // So the reconciler below runs in a shipped build, which is what makes the two-writer problem
    // reachable rather than theoretical.
    //
    // The headless host test still passes for the reason it always did:
    // `MinimalPlugins` installs no host, so nothing freezes and this returns
    // early. That test proves the composition no player runs.
    if !roster.is_published_by(VERSUS_EXPERIENCE) {
        return;
    }
    // NOTHING FROZE, so nothing has an opinion — activate as-is.
    //
    // A roster nobody can disagree with is not a roster awaiting confirmation, and leaving it
    // `Proposed` forever would turn a lifecycle into a deadlock.
    if !topology.is_frozen() {
        if !roster.seating.may_seat() {
            // validate as PART of activating. `status.md`'s activation row
            // asks for exactly this, and the validation already existed — its
            // only caller was `seat_match_participants`, which runs one step
            // AFTER the roster is live. So a route could activate a match its
            // own composition cannot fill, seating would refuse it, and the
            // stage would sit on a roster that never seats.
            // ONE ARM NOW, and an absent registry is a real answer rather
            // than a reason to skip the check. This branched on whether an
            // archetype table existed and activated UNVALIDATED when it did not
            // — defensible while a seat's policy had two possible homes, because
            // "no archetype table" did not mean "no answer". It does now: a CPU
            // seat naming a policy nobody published is refused by seating
            // whatever this does, so validating an empty registry reports the
            // truth one tick earlier instead of stranding anybody.
            let validated = roster.activate_if_seatable(profiles.as_deref(), None);
            if let Err(problems) = validated {
                bevy::log::warn_once!(
                    "the versus roster is not activated because this composition \
                     cannot seat it: {}. The stage stays on its proposal rather \
                     than publishing a match that would be refused a tick later.",
                    problems
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join("; "),
                );
            }
        }
        return;
    }
    if roster.seat_topology() == Some(topology.generation()) {
        return;
    }
    // What the frozen topology WOULD seat. Before activation this replaces the
    // roster; after it, it is the thing the live roster is compared against.
    let rebuilt = versus_roster_from(
        topology.players(),
        RosterSeating::activated_at(topology.generation()),
    );
    if let Some(mut active) = active_match {
        // The match is LIVE and these are its bodies. Reseating them underneath
        // the round is worse than the disagreement — so the question is whether
        // there IS one.
        //
        // and the obvious comparison is wrong. A one-player topology seats
        // TWO participants (there is always an opponent, human or CPU), so
        // comparing `participants.len()` against `topology.players()` reports a
        // disagreement in the ordinary single-player case. Compare the FIGHTERS —
        // the roster the frozen topology would build against the one that is
        // live — because that is what "the same match" actually means.
        if rebuilt.participants == roster.participants {
            // AGREEMENT ABOUT WHO, disagreement only about the paperwork.
            //
            // Ordinary, not contrived: the route builds its roster from live
            // device discovery on entry, and the rollback session freezes its
            // topology afterwards, so a roster stamped `None` meets a frozen
            // generation the moment the session starts. Nothing about the stage
            // is wrong — the same fighters, driven by the same devices — and the
            // only stale thing is the record of which decision produced them.
            //
            // Correcting a record is a repair. This is the half of Y′9 that was
            // left as "reported rather than repaired", and it turns the warning
            // below into a signal instead of startup noise.
            // Already seated: these bodies exist, so this is correcting a record
            // rather than deciding a match, and re-validating what is already on
            // the stage would be asking a question whose answer cannot change
            // anything.
            roster.activate(Some(topology.generation()));
            active.adopt_seat_topology(topology.generation());
            return;
        }
        bevy::log::warn_once!(
            "the versus roster seats {} fighter(s) from seat topology {:?}, but the session \
             froze topology {} with {} player(s), which would seat {} different fighter(s) \
             across {} body/bodies already on the stage. The match is already seated, so it \
             is left alone — reseating bodies mid-match would be the worse bug.",
            roster.participants.len(),
            roster.seat_topology(),
            topology.generation(),
            topology.players(),
            rebuilt.participants.len(),
            active.seats(),
        );
        return;
    }
    // NOT activated — but possibly half seated. When the cast agrees, replacing
    // the roster is inert with respect to the bodies (same characters, same seat
    // indices) and all it does is stamp the generation, so it is safe either way.
    //
    // The warning survives for the case it was written for: a roster that was ALREADY activated
    // (nothing froze when the route entered, so it was activated as-is) and then met a session
    // that froze a topology seating somebody else.
    if rebuilt.participants != roster.participants && !seated.is_empty() {
        bevy::log::warn_once!(
            "the versus roster seats {} fighter(s) from seat topology {:?} and the session \
             froze topology {} with {} player(s), which would seat {} DIFFERENT fighter(s) — \
             but {} body/bodies are already on the stage from the current roster and the \
             match has not activated yet. The roster is left alone: swapping it now would \
             leave those bodies seated under definitions they were not built from, which \
             is a match assembled from two rosters and is worse than a seat count that \
             disagrees with the handle count.",
            roster.participants.len(),
            roster.seat_topology(),
            topology.generation(),
            topology.players(),
            rebuilt.participants.len(),
            seated.iter().count(),
        );
        return;
    }
    rebuilt.project_demand(&mut demand);
    commands.insert_resource(rebuilt);
}

/// What a `VersusMatch` IS, for a rollback checksum.
///
/// the phase is not a tag here — it carries the numbers. `Starting` holds
/// the countdown, `Ko` and `Won` hold the dwell clock, and a rewind that agreed
/// about "we are in Ko" while disagreeing about how much of it is left produces
/// two timelines that resume play on different frames.
///
/// the per-team wins are folded order-independently. `rounds_won` is a
/// `BTreeMap` so its order is already deterministic, but a projection that
/// depended on that would silently start reporting desyncs if the container ever
/// changed — the same reasoning as Mary-O's spent-block set, which IS a
/// `HashSet`.
fn versus_match_checksum(state: &super::versus_rules::VersusMatch) -> u64 {
    use super::versus_rules::MatchPhase;
    use std::hash::{Hash, Hasher};
    let phase = match &state.phase {
        MatchPhase::Starting { ticks_remaining } => 1 ^ ((*ticks_remaining as u64) << 8),
        MatchPhase::Fighting => 2,
        MatchPhase::Ko {
            winner,
            remaining_s,
        } => {
            3 ^ ((remaining_s.to_bits() as u64) << 8)
                ^ winner
                    .as_deref()
                    .map(hash_team)
                    .unwrap_or(0)
                    .rotate_left(21)
        }
        MatchPhase::Won {
            winner,
            remaining_s,
        } => 4 ^ ((remaining_s.to_bits() as u64) << 8) ^ hash_team(winner).rotate_left(37),
    };
    let wins = state.rounds_won.iter().fold(0u64, |acc, (team, won)| {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        team.hash(&mut hasher);
        won.hash(&mut hasher);
        acc ^ hasher.finish()
    });
    ((state.round as u64) << 48) ^ phase ^ wins.rotate_left(11)
}

fn hash_team(team: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    team.hash(&mut hasher);
    hasher.finish()
}

/// The fighting stage's directional-influence budget, in radians.
///
/// and `Platformer2dFeelTuningMonolith::di_max_angle`'s own doc has named this mode as the
/// caller since the field was written: *"a fighter mode (Super Smash Siblings)
/// authors a smash-like ≈ 0.31 (18°) to turn it on."*
///
/// 0.31 rad ≈ 18°: the victim's held stick may rotate its own launch by that
/// much, which is what makes a knock-off a read instead of a coin flip.
const VERSUS_DI_MAX_ANGLE: f32 = 0.31;

/// What the fighting stage overwrote, so leaving can put it back.
///
/// Is the versus stage the one being played right now?
///
/// A composition can INSTALL this stage without being on it — the shipped host
/// installs three — and its rules act on fighters by shape rather than by
/// ownership, so they must not run otherwise. `ShellRouter` is optional because
/// a composition with no shell has no route, and there is no honest way for a
/// route-scoped rule to claim a world that has no routes.
fn versus_stage_is_active(
    router: Option<Res<ambition_platformer2d::game_shell::ShellRouter>>,
) -> bool {
    router.is_some_and(|router| {
        router
            .active
            .as_ref()
            .is_some_and(|active| active.route_id.as_str() == VERSUS_GAMEPLAY_ROUTE)
    })
}

fn track_versus_roster(
    mut commands: Commands,
    router: Res<ambition_platformer2d::game_shell::ShellRouter>,
    devices: Res<ambition_platformer2d::input::LocalDeviceOrder>,
    // The seating a rollback session froze, when one is running.
    topology: Option<Res<ambition_platformer2d::input::LocalSeatTopology>>,
    roster: Option<Res<MatchParticipantRoster>>,
    mut demand: ResMut<ambition_platformer2d::characters::load_demand::CharacterLoadDemand>,

    mut match_state: ResMut<super::versus_rules::VersusMatch>,
) {
    let on_versus = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == VERSUS_GAMEPLAY_ROUTE);
    // MINE, not "a roster exists".
    let mine = roster
        .as_ref()
        .is_some_and(|roster| roster.is_published_by(VERSUS_EXPERIENCE));
    match (on_versus, mine) {
        (true, false) => {
            let frozen = topology.as_ref().filter(|topology| topology.is_frozen());
            let roster = versus_roster_from(
                frozen
                    .map(|topology| topology.players())
                    .unwrap_or_else(|| devices.devices().len().max(1)),
                // frozen means ACTIVATED, unfrozen means PROPOSED. The
                // seat count above already comes from the frozen topology when
                // one exists, so when it does, the session has already agreed
                // and there is nothing left to reconcile. When it does not, this
                // is a guess from live devices — which is precisely the roster
                // that must not seat until somebody confirms it.
                frozen
                    .map(|topology| RosterSeating::activated_at(topology.generation()))
                    .unwrap_or(RosterSeating::Proposed),
            );
            // Demand the art before the bodies exist: a fighter seated with no
            // decoded sheet draws a placeholder, and the whole point of a visible
            // slice is that it looks like the two characters it says it is.
            roster.project_demand(&mut demand);
            // THE SEAT COUNT THIS MATCH DECIDED, published with the roster
            // and under this experience's name. The local session maintainer
            // freezes its topology from connected DEVICES otherwise — and
            // devices are not participants: a keyboard seat has no controller
            // entity, a spare pad may not be playing, a CPU seat has none at
            // all.
            //
            // the claim is OWNED, and that is what makes it releasable: this
            // route's scope hands it back on the way out, so the next
            // experience's session is not sized by a match that has ended.
            commands.insert_resource(ambition_platformer2d::input::SessionSeatingSource::decided(
                VERSUS_EXPERIENCE,
                // CHANNELS, not participants — the versus stage published
                // the same conflation Smash did. See
                // `ControllerBinding::local_source`.
                roster.local_channel_plan(),
            ));
            commands.insert_resource(roster);
            // A NEW roster is not yet a match. Activation is seating's to
            // publish, once every participant has a body.
            commands.remove_resource::<ambition_platformer2d::versus_match::ActiveMatch>();
            // The stage DID switch on global `FriendlyFire` for a day, because
            // `effective_faction` maps any player-brained body to Player and two
            // humans are therefore always the same faction. That works for a
            // free-for-all and is wrong the moment a 2v2 exists: it makes
            // teammates hittable too, and it is a world-wide rule change made by
            // one stage. Teams say the same thing locally and correctly.
            //
            // DECLARE, don't borrow (AE6). The stage says what IT plays under
            // and writes nothing global, so there is no capture to get wrong on
            // re-entry, no restore to skip on a crash, and no window in which
            // another writer can win. `project_combat_rules` folds this over the
            // world's baseline every tick; removing it below IS the exit.
            //
            // DI ON. A launched fighter can steer its own trajectory, which is
            // the difference between a knock-off that is a read and one that is a
            // coin flip. Inert everywhere else: Ambition's PvE keeps 0.0.
            commands.insert_resource(ambition_platformer2d::combat::rules::DeclaredCombatRules {
                // The versus route says nothing about barks: every hit speaks,
                // which is what it did before the rate existed.
                bark_chance: None,
                // The versus route drops a trumped body where it hung.
                ledge_trump_pop: None,
                // The versus route says nothing: its edges trump, which is what
                // they always did.
                ledge_occupancy: None,
                // The versus route's air jumps run their full arc.
                double_jump_cancel: None,
                // The versus route says nothing about the edge cancel either:
                // its landing lag runs out wherever the body is, which is what
                // it did before the rule existed.
                edge_cancel_recovery: None,
                // The versus route's specials come out the way the body faces.
                special_turn: None,
                special_turn_reverses_drift: None,
                // Third time this repo has learned it: the participant roster, the prepared
                // match, now the rules.
                declared_by: VERSUS_EXPERIENCE.to_string(),
                di_max_angle: VERSUS_DI_MAX_ANGLE,
                // no meteor rule here for the same reason the knockback stays
                // flat: rounds end on health, not on a blast zone, so a spike
                // has nowhere to send you and a window you cannot recover in
                // would be a stun with no payoff.
                meteor_lock_time: 0.0,
                // and no rage, for the third time the same reason: health
                // rounds have no percent mechanic to mirror.
                rage_per_damage: 0.0,
                rage_max_scale: 1.0,
                // and no staling: rounds this short do not repeat a move
                // enough for a queue to mean anything.
                stale_step: 0.0,
                stale_floor: 1.0,
                // and no crouch cancel: this stage has no crouch verb to
                // reward, and a rule nothing can reach is a rule that lies about
                // what the stage does.
                crouch_cancel_scale: 1.0,
                // and no clanking, for the reason the meteor rule is absent:
                // these duelists author four moves between them and none of the
                // reads a trade opens — bait a clank, win the recoil — exists on
                // this stage. A rule nothing can reach lies about what the stage
                // does.
                clank_damage_window: 0.0,
                clank_rebound_speed: 0.0,
                sudden_death_damage: None,
                // and the engine's own post-hit window stands. This stage's
                // moves author no separated multi-hit window, so there is
                // nothing here for a shorter window to make reachable, and
                // shortening one to match another stage would be tuning this
                // round against a game it is not.
                hit_repeat_window_scale: 1.0,
                // a versus round ends on health, and its grabs are the
                // engine's flat hold rather than a percent mechanic.
                grab_hold_base_seconds:
                    ambition_platformer2d::combat::rules::FLAT_GRAB_HOLD_SECONDS,
                grab_hold_per_damage: 0.0,
                grab_hold_max_seconds: ambition_platformer2d::combat::rules::FLAT_GRAB_HOLD_SECONDS,
                grab_mash_seconds: ambition_platformer2d::combat::rules::FLAT_GRAB_MASH_SECONDS,
                // the generic versus stage stays FLAT for now: its rounds end
                // on health rather than on a blast zone, so a launch that grows
                // without bound is a different game's mechanic. Smash declares
                // its own — see `SMASH_KNOCKBACK_GROWTH`.
                knockback_growth: 0.0,
                // the generic versus stage keeps the POGO reading, deliberately:
                // its rounds end on health rather than on a blast zone, so a
                // spike has nothing to kill you off and the rebound is the more
                // useful of the two. Smash declares otherwise — see
                // `SMASH_KNOCKBACK_GROWTH`'s neighbour.
                downward_hit: ambition_platformer2d::combat::rules::DownwardHitStyle::Pogo,
                friendly_fire: false,
                // the versus route says NOTHING here. Its
                // seats are the shipped cast, every one of which authors its own
                // kit; a floor is what a stage needs when a body does not, and
                // this stage does not have that body. `None` leaves the engine's
                // exploration default standing, which nothing here reaches.
            });
            *match_state = super::versus_rules::VersusMatch::opening();
        }
        // THE EXIT IS A DECLARATION, not a branch. Everything this route published leaves with
        // the experience — see the scope in [`compose_versus_experience`].
        _ => {}
    }
}

/// Register the versus experience: a launcher entry, a route, and the stage.
pub fn compose_versus_experience(app: &mut App) {
    // The ROW (who they are) and the DEFINITION (what they do), in that order.
    // Preparation refuses an experience whose starting character has no catalog
    // row, and the hand-authored moveset only exists on the definition.
    {
        use ambition_platformer2d::characters::actor::character_catalog::{
            CharacterCatalogAppExt, CharacterCatalogFragment,
        };
        app.register_character_catalog_fragment(
            CharacterCatalogFragment::from_ron(
                VERSUS_EXPERIENCE,
                Some(FIGHTERS[0]),
                super::versus_fighters::VERSUS_CATALOG_RON,
            )
            .expect("the versus fighter catalog is valid"),
        );
    }
    use ambition_platformer2d::actors::character_runtime::CharacterDefinitionAppExt;
    for fighter in super::versus_fighters::duelists() {
        app.register_character(fighter);
    }
    // no archetype fragment. A CPU seat's `brain_profile` resolves against
    // this experience's published `autonomous_profiles` (see the note where
    // `VERSUS_ROSTER_RON` used to be), which is the authority a controller
    // question belongs to.

    PlatformerExperienceAuthoring::new(
        VERSUS_EXPERIENCE,
        VERSUS_GAMEPLAY_ROUTE,
        "Versus",
        "Two characters from two providers, fighting",
        "Prepare Versus",
        AuthoredCatalogFragments::new(FIGHTERS[0], VERSUS_EXPERIENCE),
    )
    // Declare one health gauge per possible seat. `publish_versus_hud` owns
    // round-specific text; the engine does not model rounds.
    .with_hud({
        let mut hud = ambition_platformer2d::presentation::HudDeclaration::new();
        for (seat, slot) in super::versus_rules::HEALTH_HUD_SLOTS.iter().enumerate() {
            hud = hud.slot(
                ambition_platformer2d::presentation::HudSlotSpec::new(*slot)
                    .with_region(ambition_platformer2d::presentation::SurroundRegion::Top)
                    .with_font_size(22.0)
                    // The gauge's full extent is the slot's declared minimum
                    // width, so a stage sizes its bar by saying how much room it
                    // wants rather than by knowing anything about the renderer.
                    .with_min_px(ambition_platformer2d::engine_core::Vec2::new(220.0, 30.0))
                    // Coloured by SIDE, matching the roster's seat-parity teams,
                    // so a partner's bar reads as a partner's at a glance.
                    .with_color(if seat % 2 == 0 {
                        [0.55, 0.85, 1.0, 1.0]
                    } else {
                        [1.0, 0.6, 0.55, 1.0]
                    }),
            );
        }
        hud.slot(
            ambition_platformer2d::presentation::HudSlotSpec::new(
                super::versus_rules::ROUNDS_HUD_SLOT,
            )
            .with_region(ambition_platformer2d::presentation::SurroundRegion::Top)
            .with_font_size(16.0)
            .with_color([0.75, 0.8, 0.95, 1.0]),
        )
        // The announce line: the round-start COUNTDOWN ("ROUND 2", then "FIGHT")
        // and the KO / match card. One slot for both because they are the same
        // thing — the words the stage says between exchanges — and a second slot
        // would let them overlap.
        //
        // Without that the word FIGHT would sit over the whole round.
        .slot(
            ambition_platformer2d::presentation::HudSlotSpec::new(
                super::versus_rules::ANNOUNCE_HUD_SLOT,
            )
            .centered()
            .with_font_size(34.0)
            .with_color([1.0, 0.85, 0.3, 1.0]),
        )
    })
    // it cannot become a standalone binary, and that is a fact about what it
    // IS rather than an omission. Its fighters are `mary_o` and `sanic`,
    // registered by two DIFFERENT provider plugins, so the multi-game shell host
    // is the only composition where both casts exist — which is exactly why this
    // module lives in the app and says so at the top. Dropping the composition
    // would delete the only proof in the workspace that two providers' characters
    // can fight, at the moment the Smash lane is reporting the same boundary from
    // the other side.
    //
    //  so the ROW goes and the STAGE stays. `versus_stage.rs` drives the real
    // shell and activates `VERSUS_GAMEPLAY_ROUTE` by id; nothing it asserts is
    // about the launcher listing it.
    .unlisted()
    .with_defense_presentation(
        ambition_platformer2d::presentation::DefensePresentationPolicy::shared_iframe_blink(),
    )
    .install(app, versus_prepared_session_world);

    app.init_resource::<super::versus_rules::VersusMatch>();
    // The scoreboard is SIMULATION state: a rewind that restores the fighters
    // and not the score leaves the two disagreeing about what round it is.
    {
        use ambition_platformer2d::rollback::AmbitionRollbackApp;
        app.rollback_resource_clone_checksum::<super::versus_rules::VersusMatch>(
            VERSUS_EXPERIENCE,
            "resource.versus_match",
            "bevy_ggrs clone snapshot + a checksum over the round, the phase and the per-team wins",
            versus_match_checksum,
        );
    }

    // The RULES on the sim schedule, after damage has resolved — they read this
    // tick's health outcome and write body position. `Settle` is the phase for
    // exactly that: "everything that reads this tick's damage outcome rather
    // than producing it".
    //
    // ALL of them. There was a second system in `Update` holding the KO card on
    // the render clock, and it mutated `VersusMatch` — which is rollback state,
    // so the restored score depended on presentation-frame history that
    // resimulation does not replay. Calling a system "the presentation half"
    // does not make the resource it writes presentational.
    {
        use ambition_platformer2d::platformer::schedule::{CombatSet, SimScheduleExt};
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            super::versus_rules::settle_versus_round
                .in_set(CombatSet::Settle)
                // ONLY WHILE THIS STAGE IS THE ONE BEING PLAYED.
                //
                // It ran unconditionally, and `VersusMatch` defaults to
                // `MatchPhase::Starting` — whose arm calls `take_the_controls`
                // on EVERY fighter, every tick, idempotently. So in any
                // composition that installs this stage and is not on its route,
                // every seated fighter carried `ScriptedControl` forever and
                // `blank_scripted_control_frames` zeroed its control frame each
                // tick. The input arrived correctly the whole way — GGRS
                // published it, the brain read it — and the body never moved.
                .run_if(versus_stage_is_active),
        );
    }

    // Round lifetime is installed by match composition, not the engine. Advancing
    // the round retires everything scoped to the prior round without enumerating
    // transient entity families.
    app.add_plugins(ambition_platformer2d::platformer::lifecycle::RoundScopePlugin);

    // Publishing the scoreboard IS presentation: it reads `VersusMatch` and
    // writes HUD text, and it writes nothing the simulation reads back.
    app.add_systems(Update, super::versus_rules::publish_versus_hud);

    // DELIBERATE SILENCE, declared. Preparation refuses an experience whose
    // provider registered no explicit audio fragment — a good refusal, and one
    // the external-consumer fixture hit first. The stage authors no cues of its
    // own: the fighters bring theirs, attributed to THEIR providers, which is the
    // whole point of a crossover.
    {
        use ambition_platformer2d::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
        app.register_audio_catalog_fragment(
            AudioCatalogFragment::new(VERSUS_EXPERIENCE, None, None)
                .expect("the silent versus audio fragment is valid"),
        );
    }

    // The stage READS the device order, so the stage guarantees it exists.
    // `ambition_platformer2d_host` owns the systems that maintain it, and a composition
    // without the host input plugin (the lifecycle fixtures) still routes
    // through here — an `Option<Res>` would turn "this host has no device
    // layer" into "zero controllers", which is exactly the answer that decides
    // player two is a CPU. `init_resource` is a no-op when the host already
    // installed it, so there is still one writer.
    app.init_resource::<ambition_platformer2d::input::LocalDeviceOrder>();

    // Run on `Update`, outside `GameplaySimulationRoot`, so leaving gameplay does
    // not disable the teardown responsible for cleaning up that transition.
    app.add_systems(
        Update,
        (
            track_versus_roster,
            // AFTER, and in the same tick: the roster this built may have been
            // decided before the session froze its seating.
            reconcile_roster_with_frozen_topology,
        )
            .chain()
            // BEFORE THE SESSION IS SIZED, and stated rather than hoped.
            // The maintainer freezes a topology from connected DEVICES when
            // nothing has claimed roster-driven seating, and the session is
            // never resized afterwards — so a maintainer that ran first on the
            // frame this route opens sized the whole match from what was
            // plugged in. Same schedule, so this is a real edge; a cross-
            // schedule `.after` would be silently vacuous.
            .before(ambition_platformer2d::rollback::local_session::LocalSessionSet::Maintain),
    );

    declare_versus_experience_scope(app);
}

/// WHAT THIS EXPERIENCE OWNS, AND WHAT LEAVES WITH IT.
///
/// A match's roster, its activation, its combat declaration and the seat
/// count it decided are all global resources with the lifetime of one route
/// visit. Declaring them here means the exit is one list rather than a
/// teardown arm inside the system that also builds them — and a teardown that
/// lives inside the thing it tears down can only ever run while it is not
/// needed.
///
/// A named function so the stage-rule tests compose THIS declaration — not a
/// re-typed copy that would stay green after the real one lost a line.
fn declare_versus_experience_scope(app: &mut App) {
    {
        use ambition_platformer2d::game_shell::ShellExperienceScopeAppExt;
        app.experience_owns(VERSUS_EXPERIENCE)
            // Shared with every other experience that stages a cast, so it is
            // released by OWNER: removing it by type would be one game deleting
            // another's match.
            .releasing_owned::<MatchParticipantRoster>(|roster, owner| {
                roster.is_published_by(owner.as_str())
            })
            // The match ends WITH its route. An activation that outlives its
            // match is the next game inheriting somebody else's fighters.
            //
            // `ActiveMatch` is rollback state that deliberately carries no identity, so the plan it
            // was activated from answers for it.
            .releasing_witnessed::<
                ambition_platformer2d::versus_match::ActiveMatch,
                ambition_platformer2d::versus_match::PreparedMatch,
            >(|plan, owner| plan.is_published_by(owner.as_str()))
            // AND THE PLAN it activated from. A `PreparedMatch` that outlives
            // its experience is the next game's stage quietly building THIS
            // game's fighters — the same lesson as the roster above, one
            // resource later.
            //
            // declared AFTER the activation above, which reads it: releases
            // run in declaration order and the witness has to still be here.
            .releasing_owned::<ambition_platformer2d::versus_match::PreparedMatch>(
                |plan, owner| plan.is_published_by(owner.as_str()),
            )
            // DROP THE DECLARATION. A match rule that outlives its match is a
            // rule the next game silently inherits, and "your allies can now
            // shoot you" is a bad surprise to bring into a co-op level.
            //
            // there is nothing to put back, and that is the point: writing the
            // engine defaults reads as a restore and is not one.
            .releasing_owned::<ambition_platformer2d::combat::rules::DeclaredCombatRules>(
                |rules, owner| rules.is_declared_by(owner.as_str()),
            )
            .releasing_with("SessionSeatingSource", |world, owner| {
                if let Some(mut seating) = world.get_resource_mut::<
                    ambition_platformer2d::input::SessionSeatingSource,
                >() {
                    seating.release(owner.as_str());
                }
            });
    }
}

#[cfg(test)]
mod stage_rule_tests {
    use super::*;
    use ambition_platformer2d::combat::feel::Platformer2dFeelTuningMonolith;

    fn stage_rule_app() -> App {
        let mut app = App::new();
        app.init_resource::<Platformer2dFeelTuningMonolith>();
        app.init_resource::<ambition_platformer2d::combat::targeting::FriendlyFire>();
        app.init_resource::<super::super::versus_rules::VersusMatch>();
        app.init_resource::<ambition_platformer2d::characters::load_demand::CharacterLoadDemand>();
        app.init_resource::<ambition_platformer2d::input::LocalDeviceOrder>();
        app.insert_resource(ambition_platformer2d::game_shell::ShellRouter::default());
        // The projection normally runs in `Platformer2dSimulationPhaseMonolith::WorldPrep`; here it
        // runs straight after the declarer, which is the same ORDER and all these tests need.
        // Asserting on the resolved value rather than on a global is the whole point of AE6 — a
        // test that read the baseline would be asserting the borrow it replaced. The REAL exit
        // mechanism, not a fixture copy: the versus scope declaration (the same call
        // `compose_versus_experience` makes) plus the shell's release system.
        super::declare_versus_experience_scope(&mut app);
        app.add_systems(
            Update,
            (
                ambition_platformer2d::game_shell::release_departed_experience_state,
                track_versus_roster,
                ambition_platformer2d::actors::features::combat_rules::project_combat_rules,
            )
                .chain(),
        );
        app.update();
        app
    }

    fn resolved(app: &App) -> ambition_platformer2d::combat::rules::ResolvedCombatTuning {
        *app.world()
            .resource::<ambition_platformer2d::combat::rules::ResolvedCombatTuning>()
    }

    fn enter_versus(app: &mut App) {
        app.world_mut()
            .resource_mut::<ambition_platformer2d::game_shell::ShellRouter>()
            .active = Some(ambition_platformer2d::game_shell::ActiveShellExperience {
            activation_id: ambition_platformer2d::game_shell::ShellActivationId(1),
            route_id: ambition_platformer2d::game_shell::ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE),
            experience_id: ambition_platformer2d::game_shell::ShellExperienceId::new(
                VERSUS_EXPERIENCE,
            ),
            parameters: Default::default(),
            load_authorization: None,
            prepared_session: None,
        });
        app.update();
    }

    fn leave_versus(app: &mut App) {
        app.world_mut()
            .resource_mut::<ambition_platformer2d::game_shell::ShellRouter>()
            .active = None;
        app.update();
    }

    /// A match leaves with its route — and takes only its OWN.
    ///
    /// both halves, because either alone is green for the wrong reason.
    /// Deleting the release entirely passes "a stranger's survives"; leaving it
    /// unconditional passes "mine is gone". Only the pair pins the fix.
    #[test]
    fn a_match_leaves_with_its_route() {
        use ambition_platformer2d::versus_match::{ActiveMatch, PreparedMatch};

        let mut app = stage_rule_app();
        enter_versus(&mut app);

        app.world_mut()
            .insert_resource(PreparedMatch::for_test_published_by(Some(
                VERSUS_EXPERIENCE,
            )));
        app.world_mut()
            .insert_resource(ActiveMatch::for_test(2, None));

        leave_versus(&mut app);

        assert!(
            app.world().get_resource::<ActiveMatch>().is_none(),
            "versus's own activation outlived its route — the next experience \
             inherits somebody else's fighters"
        );
        assert!(
            app.world().get_resource::<PreparedMatch>().is_none(),
            "versus's own plan outlived its route — the next experience's \
             seating quietly builds THIS game's cast"
        );
    }

    /// ...and never another game's. The other half of the pair above.
    ///
    /// The plan is the WITNESS: `ActiveMatch` is rollback state that carries no
    /// identity by design, so the question "whose match is this" is answered by
    /// the plan it was activated from. A stranger's plan must protect the
    /// stranger's activation too, which is the property a release keyed only on
    /// the plan could get wrong in one direction.
    #[test]
    fn another_experiences_match_is_left_standing() {
        use ambition_platformer2d::versus_match::{ActiveMatch, PreparedMatch};

        let mut app = stage_rule_app();
        enter_versus(&mut app);

        app.world_mut()
            .insert_resource(PreparedMatch::for_test_published_by(Some("smash")));
        app.world_mut()
            .insert_resource(ActiveMatch::for_test(4, None));

        leave_versus(&mut app);

        assert!(
            app.world().get_resource::<PreparedMatch>().is_some(),
            "versus deleted another game's plan on its way out"
        );
        let survivor = app.world().get_resource::<ActiveMatch>();
        assert_eq!(
            survivor.map(ActiveMatch::seats),
            Some(4),
            "versus deleted another game's LIVE MATCH on its way out — the \
             roster's lesson, one resource later"
        );
    }

    /// DI is a rule of the fighting stage, and it leaves with the stage.
    ///
    /// wants Smash physics in the flagship, without naming a number. So the
    /// budget is switched on by the versus route and gone again on the way out.
    ///
    /// asserted on the RESOLVED tuning, and on the baseline NOT MOVING (AE6).
    #[test]
    fn di_switches_on_with_the_versus_route_and_off_again_when_it_ends() {
        let mut app = stage_rule_app();

        assert_eq!(
            resolved(&app).di_max_angle,
            0.0,
            "Ambition's PvE default is no DI; this is the case the flagship \
             actually returns to"
        );

        enter_versus(&mut app);
        assert_eq!(
            resolved(&app).di_max_angle,
            VERSUS_DI_MAX_ANGLE,
            "the fighting stage must author its DI budget"
        );
        assert_eq!(
            app.world()
                .resource::<Platformer2dFeelTuningMonolith>()
                .di_max_angle,
            0.0,
            "the stage reached its DI budget by WRITING the world's tuning \
             again — the borrow AE6 removed"
        );

        leave_versus(&mut app);
        assert_eq!(
            resolved(&app).di_max_angle,
            0.0,
            "DI outlived its match, so every knockback in the game it returns to \
             now steers and nothing says why"
        );
    }

    /// The half the default-start test could not see, and what replaced it.
    ///
    /// Starting from values NOBODY ships made the difference visible: an experience that authored
    /// its own DI budget and its own friendly-fire rule had to get them back, having lost them
    /// merely because the player visited the versus route.
    ///
    /// the restore is gone, because the borrow is (AE6). The stage declares its rules and
    /// the projection folds them over the baseline, so the baseline is never written and there
    /// is nothing to hand back. A relapse to writing globals fails on the middle assertion,
    /// which is the one the old shape could not make at all.
    #[test]
    fn a_match_never_touches_the_world_tuning_it_plays_over() {
        let mut app = stage_rule_app();

        // Some other experience's authored tuning. Neither value is a default.
        const PRIOR_DI: f32 = 0.12;
        app.world_mut()
            .resource_mut::<Platformer2dFeelTuningMonolith>()
            .di_max_angle = PRIOR_DI;
        app.world_mut()
            .resource_mut::<ambition_platformer2d::combat::targeting::FriendlyFire>()
            .enabled = true;
        assert_ne!(
            PRIOR_DI,
            Platformer2dFeelTuningMonolith::default().di_max_angle,
            "the premise: this test is only meaningful while the prior value \
             differs from the value a reset would write"
        );
        assert!(!ambition_platformer2d::combat::targeting::FriendlyFire::default().enabled);

        let authored_is_intact = |app: &App, when: &str| {
            assert_eq!(
                app.world()
                    .resource::<Platformer2dFeelTuningMonolith>()
                    .di_max_angle,
                PRIOR_DI,
                "{when}: the versus route wrote another experience's authored DI \
                 budget. It must DECLARE its rules, never borrow the world's."
            );
            assert!(
                app.world()
                    .resource::<ambition_platformer2d::combat::targeting::FriendlyFire>()
                    .enabled,
                "{when}: same for friendly fire — the baseline is not the stage's \
                 to write, in either direction"
            );
        };

        app.update();
        authored_is_intact(&app, "before");
        assert_eq!(resolved(&app).di_max_angle, PRIOR_DI);
        assert!(resolved(&app).friendly_fire);

        enter_versus(&mut app);
        // DURING.
        authored_is_intact(&app, "during the match");
        assert_eq!(
            resolved(&app).di_max_angle,
            VERSUS_DI_MAX_ANGLE,
            "the match is not playing under its own declared DI"
        );
        assert!(
            !resolved(&app).friendly_fire,
            "the stage runs on teams, not on global free-for-all"
        );

        leave_versus(&mut app);
        authored_is_intact(&app, "after");
        assert_eq!(
            resolved(&app).di_max_angle,
            PRIOR_DI,
            "the match's DI outlived the match"
        );
        assert!(
            resolved(&app).friendly_fire,
            "the match's friendly-fire rule outlived the match"
        );
        assert!(
            !app.world()
                .contains_resource::<ambition_platformer2d::combat::rules::DeclaredCombatRules>(),
            "the declaration must not outlive the match — dropping it IS the exit"
        );
    }
}

#[cfg(test)]
mod roster_topology_tests {
    use super::*;
    use ambition_platformer2d::characters::load_demand::CharacterLoadDemand;
    use ambition_platformer2d::input::{LocalDeviceOrder, LocalSeatTopology};
    use ambition_platformer2d::versus_match::ActiveMatch;

    fn topology_of(pads: usize) -> LocalSeatTopology {
        let mut topology = LocalSeatTopology::default();
        topology.capture(&LocalDeviceOrder::from_devices(
            (0..pads as u32).filter_map(Entity::from_raw_u32).collect(),
        ));
        topology
    }

    fn app_with(roster: MatchParticipantRoster, pads: usize, active: bool) -> App {
        let mut app = App::new();
        app.insert_resource(roster);
        app.insert_resource(topology_of(pads));
        if active {
            // An ACTIVE match: seating published it, so its bodies are on the
            // stage and rebuilding the roster would reseat underneath them.
            app.insert_resource(ActiveMatch::for_test(0, None));
        }
        app.init_resource::<CharacterLoadDemand>();
        app.add_systems(Update, reconcile_roster_with_frozen_topology);
        app
    }

    /// Activation VALIDATES, and a roster this composition cannot fill is not
    /// activated.
    ///
    /// `status.md`'s activation row asks for *"validate every participant, activate the roster
    /// atomically"*. The validation existed — `unsatisfiable_seats` — and its caller was
    /// `seat_match_participants`, which runs one step AFTER the roster is live. So a route could
    /// activate a match naming a brain profile its own `CharacterRoster` has never heard of,
    /// seating would refuse it every tick, and the stage would sit forever on a match that
    /// publishes a refusal instead of fighters.
    #[test]
    fn a_roster_this_composition_cannot_seat_is_not_activated() {
        let mut app = App::new();
        // ONE local player, and that is not a detail. `versus_roster_from`
        // makes seat `n` human only while `n < local_players`, so a two-player
        // roster is two humans and has no brain profile to validate at all —
        // this test passed for that reason first time out.
        app.insert_resource(versus_roster_from(1, RosterSeating::Proposed));
        // Not frozen: this is the entry-time arm, where the route activates its
        // own proposal because no session has an opinion.
        app.insert_resource(LocalSeatTopology::default());
        app.init_resource::<CharacterLoadDemand>();
        app.add_systems(Update, reconcile_roster_with_frozen_topology);
        app.update();

        let roster = app.world().resource::<MatchParticipantRoster>();
        assert!(
            !roster.seating.may_seat(),
            "a roster naming brain profile `{VERSUS_CPU_BRAIN}`, which this \
             composition's BrainProfileRegistry does not publish, was activated \
             anyway. Seating will refuse it every tick and the stage will never \
             open"
        );

        // and the SAME roster activates once the composition can answer for it,
        // or the refusal above would pass against a route that never activates.
        // assembled from the stage's OWN catalog RON, not a hand-built table:
        // that file is what a shipped composition registers, so this half also
        // asserts `VERSUS_CPU_BRAIN` is actually in it — the exact thing that was
        // false when the stage named `medium_striker`.
        //
        // Same claim, one authority over: a CPU seat's controller question is answered by published
        // `autonomous_profiles`, so that is what the composition must carry.
        {
            use ambition_platformer2d::characters::actor::character_catalog::{
                CharacterCatalogAppExt, CharacterCatalogFragment,
            };
            // through the App seam the composition itself uses, so what is
            // published here is what a shipped versus route publishes — the
            // registration is what turns an authored profile into a resolvable
            // `versus::versus_duelist` (cite-ok: an authored key, not a path).
            app.register_character_catalog_fragment(
                CharacterCatalogFragment::from_ron(
                    VERSUS_EXPERIENCE,
                    Some(FIGHTERS[0]),
                    crate::app::versus_fighters::VERSUS_CATALOG_RON,
                )
                .expect("the versus fighter catalog is valid"),
            );
        }
        app.update();
        assert!(
            app.world()
                .resource::<MatchParticipantRoster>()
                .seating
                .may_seat(),
            "a roster this composition CAN seat was still refused"
        );
    }

    /// Another game's roster is not this one's to rebuild.
    ///
    /// `versus_roster_from` stamps `published_by: ambition_versus`, so rebuilding
    /// a roster somebody else published does not resize it — it TRANSFERS
    /// OWNERSHIP, and `maintain_versus_stage` then deletes it, correctly, as its
    /// own on a route that is not versus. Smash's select screen published two
    /// fighters, this took them, versus threw them away, and the stage opened
    /// with one fighter.
    ///
    /// the fixture has to FREEZE a topology, because that is the only
    /// condition under which the reconciler runs at all — and it is exactly why
    /// the headless host test never saw this: `MinimalPlugins` freezes none, so
    /// the function returned early and Smash's roster survived in a composition
    /// no player runs.
    #[test]
    fn a_roster_published_by_another_experience_is_left_alone() {
        let mut foreign = versus_roster_from(2, RosterSeating::Proposed);
        foreign.published_by = Some("ambition_smash".to_owned());
        let before = foreign.clone();

        let mut app = app_with(foreign, 1, false);
        app.update();

        let after = app.world().resource::<MatchParticipantRoster>();
        assert_eq!(
            after.published_by.as_deref(),
            Some("ambition_smash"),
            "versus took ownership of another experience's roster"
        );
        assert_eq!(
            after.participants, before.participants,
            "versus reseated another experience's fighters"
        );
        assert_eq!(
            after.seating, before.seating,
            "versus restamped another experience's roster with its own topology"
        );
    }

    /// The reconciler still does its job for a roster that IS versus's.
    #[test]
    fn its_own_roster_is_still_reconciled() {
        let mut mine = versus_roster_from(2, RosterSeating::Proposed);
        mine.published_by = Some(VERSUS_EXPERIENCE.to_owned());
        let mut app = app_with(mine, 1, false);
        app.update();
        let after = app.world().resource::<MatchParticipantRoster>();
        assert!(
            after.seat_topology().is_some(),
            "a versus roster meeting a frozen topology should be stamped with it"
        );
    }

    /// A roster decided before the session froze its seating is rebuilt.
    ///
    /// The real sequence, not a contrived one: a route is entered — and its
    /// roster built from live device discovery — before its rollback session
    /// exists. A controller connecting in that gap left the roster seating two
    /// fighters into a session with three handles, both citing "the connected
    /// controllers". Nothing has a body yet at that point, so the roster is only
    /// an intention and rebuilding it costs nothing.
    #[test]
    fn a_roster_built_before_the_freeze_is_rebuilt_against_it() {
        let mut app = app_with(versus_roster_from(2, RosterSeating::Proposed), 3, false);
        app.update();
        let roster = app.world().resource::<MatchParticipantRoster>();
        assert_eq!(
            roster.participants.len(),
            3,
            "the roster kept the seat count it guessed from live devices while \
             the session had already frozen a different one"
        );
        assert_eq!(
            roster.seat_topology(),
            Some(app.world().resource::<LocalSeatTopology>().generation()),
            "a rebuilt roster must record WHICH topology it agreed with, or the \
             next tick rebuilds it again forever"
        );
    }

    /// A HALF-SEATED match is not handed a different roster.
    ///
    /// The test above is the safe case and was read as the general one: "no `ActiveMatch`" was
    /// taken to mean "nothing has a body yet". It does not. Seating retries across ticks and the
    /// latch closes only when every seat is filled, so a match with one fighter standing and one
    /// still pending sits in exactly this state — and swapping the roster there is not a rebuild,
    /// it is a match assembled from two rosters.
    #[test]
    fn a_half_seated_match_is_not_handed_a_different_roster() {
        let mut app = app_with(versus_roster_from(2, RosterSeating::Proposed), 3, false);
        // One fighter is already standing on the stage; the other has not seated
        // yet, so `ActiveMatch` is absent — the exact window.
        app.world_mut()
            .spawn(ambition_platformer2d::versus_match::MatchSeat(0));
        app.update();
        let roster = app.world().resource::<MatchParticipantRoster>();
        assert_eq!(
            roster.participants.len(),
            2,
            "the roster was replaced under a body that is already seated, so seat \
             0 keeps a fighter the new roster never asked for and seating will \
             skip it as occupied"
        );
        assert_eq!(
            roster.seat_topology(),
            None,
            "and the stamp must NOT be corrected either — a roster that records \
             agreement with a topology it does not implement is the disagreement \
             made invisible"
        );
    }

    /// The window is only dangerous when the CAST differs. A body seated from a
    /// roster the frozen topology would rebuild identically is not in conflict
    /// with anything, and refusing there would strand the stamp forever.
    #[test]
    fn a_half_seated_match_whose_cast_agrees_still_gets_its_stamp() {
        let topology = topology_of(2);
        let mut app = app_with(versus_roster_from(2, RosterSeating::Proposed), 2, false);
        app.world_mut()
            .spawn(ambition_platformer2d::versus_match::MatchSeat(0));
        app.update();
        let roster = app.world().resource::<MatchParticipantRoster>();
        assert_eq!(
            roster.participants,
            versus_roster_from(2, RosterSeating::Proposed).participants
        );
        assert_eq!(
            roster.seat_topology(),
            Some(topology.generation()),
            "same fighters, same seats — replacing the roster here changes \
             nothing about the bodies and is how the reconciler stops re-running"
        );
    }

    /// A roster already built from this topology is left completely alone —
    /// otherwise the reconciler rebuilds it every tick and the demand set grows
    /// without end.
    #[test]
    fn an_agreeing_roster_is_not_touched() {
        let topology = topology_of(2);
        let mut app = app_with(
            versus_roster_from(2, RosterSeating::activated_at(topology.generation())),
            2,
            false,
        );
        let before = app.world().resource::<MatchParticipantRoster>().clone();
        app.update();
        assert_eq!(*app.world().resource::<MatchParticipantRoster>(), before);
    }

    /// A topology that has decided nothing does not get to overrule live
    /// discovery.
    ///
    /// `generation == 0` means never captured — the resource exists, no session has frozen its
    /// seating. Treating that as an answer would rebuild every roster down to the two-seat floor.
    #[test]
    fn an_unfrozen_topology_does_not_overrule_the_roster() {
        let mut app = App::new();
        app.insert_resource(versus_roster_from(4, RosterSeating::Proposed));
        app.insert_resource(LocalSeatTopology::default());
        app.init_resource::<CharacterLoadDemand>();
        app.add_systems(Update, reconcile_roster_with_frozen_topology);
        app.update();
        assert_eq!(
            app.world()
                .resource::<MatchParticipantRoster>()
                .participants
                .len(),
            4,
            "a topology that has never been captured overruled a four-player \
             roster and cut it to the two-seat floor"
        );
    }

    /// The stamp is repaired when the match and the session agree about WHO.
    ///
    /// The ordinary sequence, and the one that made this worth building: the
    /// route builds its roster from live device discovery on entry, stamped
    /// `None` because no session has frozen anything yet; the fighters seat; the
    /// rollback session starts and freezes its topology. Now the roster
    /// disagrees with the session about which decision produced it while
    /// agreeing completely about who is fighting.
    #[test]
    fn a_live_match_that_agrees_with_its_session_adopts_its_topology() {
        let mut app = app_with(versus_roster_from(1, RosterSeating::Proposed), 1, true);
        let generation = app.world().resource::<LocalSeatTopology>().generation();
        let fighters = app
            .world()
            .resource::<MatchParticipantRoster>()
            .participants
            .clone();

        app.update();

        let roster = app.world().resource::<MatchParticipantRoster>();
        assert_eq!(
            roster.participants, fighters,
            "the repair changed WHO is fighting; it may only correct the record \
             of which topology decided them"
        );
        assert_eq!(
            roster.seat_topology(),
            Some(generation),
            "the roster and the session agree about the fighters and the roster \
             still records no topology, so the reconciler warns about a stage \
             where nothing is wrong — every tick, forever"
        );
        assert_eq!(
            app.world().resource::<ActiveMatch>().seat_topology(),
            Some(generation),
            "the ACTIVATION kept the stale stamp, so the next comparison asks \
             the same question again and the repair has to be redone"
        );
    }

    /// After seating it is bodies on a stage, and it is left alone.
    #[test]
    fn a_seated_match_is_never_reseated_underneath_itself() {
        let mut app = app_with(versus_roster_from(2, RosterSeating::Proposed), 3, true);
        app.update();
        assert_eq!(
            app.world()
                .resource::<MatchParticipantRoster>()
                .participants
                .len(),
            2,
            "a seated match was rebuilt underneath its own bodies"
        );
    }
}
