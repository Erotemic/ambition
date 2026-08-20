//! **A versus stage you can pick from the launcher.** (C4 slice 2)
//!
//! Slice 1 gave the engine the verb — `seat_match_participants` turns a
//! [`MatchParticipantRoster`] into bodies. Nothing called it. C4's complaint was
//! never that the fight did not work; it was that the fight existed only where a
//! test could see it, and "a stranger can run it and watch" is the whole
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
//!
//! No win condition, no rounds, no HUD. Those are match RULES, and a stage that
//! has to be right before rules can be written is the thing that was missing;
//! inventing a ruleset here would make the slice bigger and the seam no more
//! proven. Slice 3 (player vs CPU) needs the human seat, not rules.

use bevy::prelude::*;

use ambition_platformer2d::actors::character_runtime::{
    ControllerBinding, MatchParticipant, MatchParticipantRoster, RosterSeating, StagesCharacters,
};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition_platformer2d::runtime::demo_fixture::{
    ActiveRoomMetadata, RoomSet, StartingCharacter,
};
use ambition_platformer2d::runtime::PreparedPlatformerSource;
use ambition_platformer2d::world::rooms::RoomSpec;

pub const VERSUS_EXPERIENCE: &str = "ambition_versus";
pub const VERSUS_GAMEPLAY_ROUTE: &str = "versus_gameplay";
pub const VERSUS_ROOM_ID: &str = "versus_arena";

/// The two fighters.
///
/// They used to be `mary_o` and `sanic`, chosen because a match between two
/// demo providers' casts is the crossover the character seam was built for.
/// That was the right cast for proving the seam and the wrong one for a fight:
/// neither authors a move list, so couch versus was two people walking into
/// each other, and giving either one attacks would have been authoring against
/// their design to make a different mode work (Sanic's row says "no combat
/// moveset" in as many words; Mary-O has to play like SMB1).
///
/// The arena has its own fighters now — see [`super::versus_fighters`]. The
/// crossover is not lost: they SHARE the demos' art by id, so the stage still
/// draws one cast against the other, and it is still the only composition where
/// both sheets exist.
const FIGHTERS: [&str; 2] = ["arena_duelist_long", "arena_duelist_close"];

/// A stage with EDGES. The first version was a closed box — a floor and two
/// walls — which is a room, not a stage: a fighter could be pushed to the wall
/// and no further, so the only way to lose was to run out of health, and the
/// verb the whole genre is built on (put them somewhere there is no floor) had
/// nowhere to happen.
///
/// So: a main platform floating in open space, two side platforms to recover
/// onto, no walls, and a blast margin the stage authors itself. Past that
/// margin the engine's out-of-bounds gate reports
/// `ResetCause::LeftTheWorld` — a lethal hit against exactly that fighter,
/// which zeroes their health, which is the condition `round_result` already
/// scores. A knock-off ends a round through the rule that was already there.
///
/// The margins are deliberately generous relative to the drop: a fighter has the
/// whole 140px below the platform plus 96px past the world before they are
/// gone, which is long enough to see the mistake and long enough to jump back
/// from. A tight blast zone turns every trade near the edge into a coin flip.
/// No CEILING zone: nothing in the current moveset launches hard enough upward
/// for one to be anything but a surprise.
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
    .with_blast_margin(96.0)
    // The SIDE blast zone, which is where a platform fighter actually loses
    // most of its stocks. Without it a fighter thrown off the left edge only
    // dies once its arc happens to carry it below the stage, which reads as the
    // throw not having worked. 160px is deliberately looser than the 96px
    // floor: you are given further to recover from a horizontal launch than
    // from a straight drop, because a horizontal launch is the one you can act
    // on.
    .with_side_blast_margin(160.0);
    let mut room = RoomSpec::new(VERSUS_ROOM_ID, world);
    room.metadata.mode = Some(VERSUS_EXPERIENCE.to_owned());
    room
}

fn versus_prepared_session_world() -> PreparedPlatformerSource {
    let room = versus_arena();
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    // ⛔ **`for_match`: no home body.** The comment this replaces described the
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

/// The roster this stage seats: the player, and either a friend or a CPU.
///
/// Seat 0 is HUMAN and is the body the session already spawned wearing
/// `FIGHTERS[0]` — seating adopts it rather than spawning beside it. That is not
/// a detail: the first version of this stage made both seats CPU while the
/// session had already spawned a player wearing the same character, and the
/// arena held two Mary-Os. The test passed because it asserted both fighters were
/// present, which is the assertion you write when you have not looked at the
/// screen.
///
/// Seat 1 is a HUMAN when a second controller is plugged in, and a CPU
/// otherwise. That is the whole of Jon's ordering (cpu vs cpu, then player vs
/// cpu, then local couch, and only then netcode) reaching its third step, and it
/// needs no menu: the presence of a second controller is an unambiguous
/// statement that a second person is here, and a stage that asked the question
/// in a lobby screen would be a lobby screen standing between a stranger and the
/// thing they came to see.
///
/// Deliberately decided at STAGE ENTRY, not per frame. A pad unplugged mid-match
/// must not silently hand player two's fighter to the AI, and a pad plugged in
/// mid-match must not spawn a third body into a running fight; both are
/// mid-match roster edits, which is a rule change, and this stage has no rules.
/// Up to four seats: 1v1 with two controllers, 2v2 with four.
///
/// The seat count follows the controllers because that is the least ceremonious
/// honest signal — four pads is four people, and a lobby screen asking how many
/// players there are would stand between a stranger and the thing they came to
/// see.
///
/// TEAMS FOLLOW SIDES. `seat_for` alternates left/right by index, so evens stand
/// together and odds stand together; pairing the teams the same way puts
/// partners on the same side of the arena rather than opposite each other.
///
/// It is also the case teams were built for. With four HUMAN fighters,
/// `effective_faction` maps every one of them to `ActorFaction::Player` — so
/// faction distinguishes nobody and `MatchTeam` is the only thing that decides
/// who may hit whom. A 2v2 is not a bigger 1v1; it is the first arrangement
/// where the relation is load-bearing.
/// **The archetype a versus CPU seat asks for**, and this experience registers.
///
/// ⚠ it used to be `"medium_striker"` — a row in `ambition_content`'s archetype
/// table, which the versus experience does not compose. So the lookup missed,
/// and before seating learned to REFUSE an unresolvable profile (2026-07-31) the
/// seat silently became a `stand_still` body: a versus match against an opponent
/// that has never once moved. The refusal turned that into a panic in
/// `versus_through_the_sdk`, which is the guard doing exactly its job.
///
/// The name is this experience's own, because the experience owns the row. A
/// demo naming another provider's archetype is the same borrowed-authority bug
/// one namespace over.
pub const VERSUS_CPU_BRAIN: &str = "versus_duelist";

// ⛔⛔ **`VERSUS_ROSTER_RON` WAS HERE AND IS DELETED (2026-08-13, campaign
// P2.18)** — one `ArchetypeSpec` row, registered as a `CharacterRosterFragment`,
// existing for exactly one lookup: a CPU seat naming `versus_duelist`, answered
// through an ENEMY ARCHETYPE TABLE. Its controller half is published as an
// `autonomous_profiles` entry in `versus_fighters::VERSUS_CATALOG_RON` now,
// which is what a controller policy IS.
//
// ⚠ **its body half went nowhere because nothing read it.** `max_health`,
// `run_speed`, `melee`, `move_style` and `respawn` stopped being read the day a
// seat was built from its CHARACTER (P1.11) — see the `fighter_abilities` note
// below, which is the record of that: the row's authored `melee` "reached the
// body regardless of what the match said the body could do", and removing it is
// what exposed this stage's missing `attack` verb.
//
// ⇒ the same migration ledger D87 made for the Smash stage's six rows, and it
// leaves `seat_brain_profile`'s archetype arm with no production caller.

/// **The roster the route PROPOSES on entry**, built from live device discovery
/// before any session has decided its seating.
///
/// ⭐ **proposed, not activated** (2026-08-06). Nothing seats from this until
/// [`activate_versus_roster`] agrees, because the route builds it from devices
/// and the rollback session freezes its topology afterwards — so a roster built
/// here can describe a different match than the session will run. Seating used
/// to happen from it anyway and the disagreement was reported after the fact,
/// which is what `status.md` means by *"MECHANISMS DONE, ACTIVATION OPEN"*.
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
                    // ⚠ **PAD `seat`, and this route is entitled to say so.** The
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
        // A round opens on a COUNTDOWN, so nobody acts until it ends. Declared on
        // the roster rather than applied after seating: a fighter that seats on
        // the tick the countdown begins would otherwise get one simulation step
        // first — a CPU decision, or a held direction — before the suspension
        // insert lands (GPT 5.6, 2026-07-29). The `Starting` arm reaching zero is
        // what takes it off, which is already the one place a round goes live.
        opens_suspended: true,
        // **THIS STAGE OWNS ITS OWN CEREMONY**, so the engine's countdown stays
        // out of it: the `Starting` arm reaching zero is what takes the hold
        // off, and it has been that way since before a ruleset could declare a
        // count. `0` says "not mine to end" rather than "no countdown" — the
        // versus round very much has one.
        opening_countdown_ticks: 0,
        // **A FAIR FIGHT.** Seat 0 is the ADOPTED primary player and arrives
        // carrying whatever the session granted it — in the shipped host, the
        // sandbox dev kit (blink, fly, shield). Every other seat is spawned with
        // the basic run-and-jump floor. So player one could teleport and fly and
        // the opponent could not, and the control legend on screen said so.
        //
        // Stated by the match rather than assumed by seating: a stage that wants
        // asymmetric fighters says something else here.
        // ⛔ **`basic()` HAS NO `attack`, and a duel where nobody may swing is
        // incoherent.** It stood because the swing did not come from the fighter
        // at all: a seat was built out of the CPU archetype, and
        // `versus_duelist`'s authored `melee` reached the body regardless of
        // what the match said the body could do. Building a seat from its
        // CHARACTER instead (campaign P1.11) took that away, and this is the
        // fact that was hiding behind it — the mask never granted the verb its
        // own fighters needed.
        //
        // ⚠ deliberately the SAME shape as the Smash stage's floor minus its
        // platform-fighter extras: this stage is a duel on one screen, and its
        // opponent brain does not use a dodge or a ledge.
        //
        // ⭐ **`at_most`, which is exactly what this has always been** — a
        // CEILING and no floor. The smash stage next door levels instead
        // (`MatchAbilities::levelled`); saying which of the two a stage means is
        // the whole point of the type, and this one means *"a character keeps
        // what it authored, minus what this duel forbids"*. A floor here would
        // hand the robot lineage the `reset` its definition deliberately refuses
        // — `basic()` grants one — which is the trap that stopped this being a
        // grant in the first place.
        // ⭐ **the SAME set the duelists now author** (D151), named once in
        // `versus_fighters` so a ceiling and the kit under it cannot drift into
        // disagreeing. Restating it here is how the stage came to be the only
        // thing dressing its own cast.
        fighter_abilities: Some(ae::MatchAbilities::at_most(
            super::versus_fighters::VERSUS_FIGHTER_KIT,
        )),
        // **NONE, and it is a decision.** This stage's cast is
        // `versus_fighters::duelists()`, two characters authored FOR it, so the
        // body they play with is already the one they were tuned with — there is
        // no fighter here whose feel came from somebody else's game. The smash
        // stage next door supplies one because it seats a crossover cast that
        // never agreed to be a platform fighter (see
        // `MatchParticipantRoster::fighter_body`), and this duel wants none of
        // its extras: no jump squat, no air dodge, no floor game.
        fighter_body: None,
        // S4: NOT a stocks match yet, and the `None` is a decision rather than a
        // gap. The shipped stage settles ROUNDS off health, and switching it to
        // stocks changes what a versus match IS — a product call, not a
        // refactor. The loop it would switch to exists and is proven
        // (`ambition_combat::stocks`, plus the app-level fixture); this line is
        // where a stage opts in, and flipping it to `Some(n)` also flips every
        // fighter to `DeathPolicy::Unbounded`, which is the pair that has to
        // travel together.
        fighter_stocks: None,
        // **NONE, and it is a decision** (queue D131). This stage settles ROUNDS
        // off health, so the pool is literal hit points rather than the scale a
        // percent is read against — and its cast is `versus_fighters::duelists()`,
        // two characters authored FOR this stage whose light/heavy split is
        // partly the pool itself (`with_health`). Declaring one number here
        // would flatten a difference the stage exists to have.
        //
        // ⚠ **the day this stage seats somebody else's character, this becomes
        // `Some(_)`.** That is the whole of D131: an authored `max_health` is a
        // statement made under the AUTHORING game's rules, and a host that seats
        // a foreign cast owes its own. Smash seats fourteen and declares one.
        fighter_health_pool: None,
        seating,
        // **WHOSE MATCH THIS IS.** The exit rule below removes the roster it
        // finds; with a second stage in the same host publishing one from its
        // own route, "the roster" stopped meaning "mine".
        published_by: Some(VERSUS_EXPERIENCE.to_owned()),
    }
}

/// One screen, four fighters, four `SlotControls` slots.
pub const MAX_VERSUS_SEATS: usize = 4;

/// Install the roster while the versus route is active, and take it away when it
/// is not.
///
/// Scoped rather than global on purpose: `MatchParticipantRoster` is what seating
/// reads, so leaving it installed would seat two fighters into Mary-O's level the
/// next time somebody played it. Removing it on exit also resets the seating
/// latch, so returning to the stage seats a fresh match rather than an empty one.
/// **The roster and the session must agree about who is playing, and now they
/// can be asked.** (queue Y′9)
///
/// A route is entered before its rollback session starts, so the roster is built
/// from LIVE device discovery and the topology is frozen later. A controller
/// connecting in that gap leaves the roster seating N fighters into a session
/// with M handles, with both citing "the connected controllers".
///
/// Before seating, a roster is only an INTENTION — nothing has a body yet — so
/// the honest response is to rebuild it against the topology that won. After
/// seating it is bodies on a stage, and silently reseating mid-match would be a
/// worse bug than the disagreement; that case is reported instead.
///
/// Match activation now performs the full atomic publication step: a prepared
/// match builds the seated bodies and publishes `ActiveMatch` in one flush, and
/// `ActiveMatch` is optional canonical rollback state. This reconciler therefore
/// owns only the earlier question: whether a still-unactivated roster must be
/// rebuilt against the topology that actually froze. The migration history is
/// archived in `docs/archive/planning-superseded/2026-08-13/character-preparation-finalization-plan.md`.
///
/// ⛔ **"no `ActiveMatch`" does NOT mean "no bodies yet", and reading it that way
/// was a real authority split** (GPT 5.6, 2026-07-30). Seating retries across
/// ticks and the latch closes only when EVERY seat has a body, so between the
/// first seated fighter and the last there is a window in which participants
/// exist and the latch does not. Replacing the roster inside that window left the
/// already-seated bodies alone — seating skips a seat index it finds occupied —
/// so the match could activate with bodies built from the OLD roster and
/// definitions from the NEW one: wrong character, wrong team, wrong
/// human-versus-CPU assignment, and a warning afterwards that could not repair
/// any of it. The window is now checked, and a disagreement inside it is
/// reported rather than half-applied.
fn reconcile_roster_with_frozen_topology(
    mut commands: Commands,
    topology: Option<Res<ambition_platformer2d::input::LocalSeatTopology>>,
    roster: Option<ResMut<MatchParticipantRoster>>,
    active_match: Option<ResMut<ambition_platformer2d::actors::character_runtime::ActiveMatch>>,
    mut demand: ResMut<ambition_platformer2d::actors::character_runtime::CharacterLoadDemand>,
    // Bodies that are ALREADY seated, latch or no latch. This is the fact the
    // `ActiveMatch` check was standing in for, and the two are not the same fact.
    seated: Query<&ambition_platformer2d::actors::character_runtime::MatchSeat>,
    // **The published controller policies** — the authority activation validates
    // against, and since 2026-08-13 the only one. OPTIONAL because a composition
    // legitimately publishes none (an engine App with no content), and
    // `engine.character-authority-is-app-local` means "not part of this
    // composition" is a real answer rather than a fault.
    //
    // ⛔ a `Res<CharacterRoster>` stood beside this and was passed FIRST. An
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
    // ⛔ **MINE, not "a roster exists" — the same rule `maintain_versus_stage`
    // learned and this function did not.**
    //
    // `versus_roster_from` stamps `published_by: ambition_versus`, so rebuilding
    // somebody else's roster here does not just resize it: it TRANSFERS
    // OWNERSHIP. Smash's select screen publishes a two-fighter roster, this
    // replaced it with a versus-stamped one built from the frozen topology, and
    // `maintain_versus_stage` then correctly deleted it as its own on a route
    // that was not versus. Smash never seated anybody and the stage opened with
    // one fighter — Jon, 2026-08-01: *"even when we add a CPU player in smash
    // there is only ever one player that shows up in game."*
    //
    // ⚠ it only bites where a topology is actually FROZEN — and that is EVERY
    // build now, not a `dev_tools` one.
    //
    // ⛔ **this comment used to say the rollback observatory was the only thing
    // that froze a topology (queue S35), and that stopped being true when
    // `freeze_local_seating_for_the_decided_match` shipped**: it is registered
    // by `PlatformerHostPlugins`, which every host adds, and the ggrs session
    // maintainer captures one of its own besides. So the reconciler below runs
    // in a shipped build, which is what makes the two-writer problem reachable
    // rather than theoretical (queue G1 PICK 17).
    //
    // The headless host test still passes for the reason it always did:
    // `MinimalPlugins` installs no host, so nothing freezes and this returns
    // early. That test proves the composition no player runs.
    if !roster.is_published_by(VERSUS_EXPERIENCE) {
        return;
    }
    // ⭐ **NOTHING FROZE, so nothing has an opinion — activate as-is.**
    //
    // This arm used to `return`, and with seating refusing a `Proposed` roster
    // that would have stranded every composition where no rollback session ever
    // starts: the headless host, the shell's non-rollback routes, a
    // `MinimalPlugins` test. A roster nobody can disagree with is not a roster
    // awaiting confirmation, and leaving it `Proposed` forever would turn a
    // lifecycle into a deadlock.
    if !topology.is_frozen() {
        if !roster.seating.may_seat() {
            // ⭐ **validate as PART of activating.** `status.md`'s activation row
            // asks for exactly this, and the validation already existed — its
            // only caller was `seat_match_participants`, which runs one step
            // AFTER the roster is live. So a route could activate a match its
            // own composition cannot fill, seating would refuse it, and the
            // stage would sit on a roster that never seats.
            // ⭐ **ONE ARM NOW, and an absent registry is a real answer rather
            // than a reason to skip the check.** This branched on whether an
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
        // ⚠ **and the obvious comparison is wrong.** A one-player topology seats
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
    // ⭐ **and a PROPOSED roster is never half seated, which is the point of the
    // lifecycle.** Seating refuses one, so a disagreement discovered here has no
    // bodies to be inconsistent with and falls through to the replacement below
    // — repaired instead of reported. The warning survives for the case it was
    // written for: a roster that was ALREADY activated (nothing froze when the
    // route entered, so it was activated as-is) and then met a session that
    // froze a topology seating somebody else.
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

/// **What a `VersusMatch` IS, for a rollback checksum.**
///
/// ⚠ **the phase is not a tag here — it carries the numbers.** `Starting` holds
/// the countdown, `Ko` and `Won` hold the dwell clock, and a rewind that agreed
/// about "we are in Ko" while disagreeing about how much of it is left produces
/// two timelines that resume play on different frames.
///
/// ⭐ **the per-team wins are folded order-independently.** `rounds_won` is a
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

/// **The fighting stage's directional-influence budget, in radians.**
///
/// Jon, asked whether DI was worth turning on: *"In smash DI is critical!"* —
/// and `Platformer2dFeelTuningMonolith::di_max_angle`'s own doc has named this mode as the
/// caller since the field was written: *"a fighter mode (Super Smash Siblings)
/// authors a smash-like ≈ 0.31 (18°) to turn it on."*
///
/// ⚠ **a MATCH RULE, set on route entry and restored on exit** — the same shape
/// `FriendlyFire` already uses below, and for the same reason: a rule that
/// outlives its match is a rule the next game silently inherits. Ambition's PvE
/// knockback keeps the `0.0` default, because Jon said he *"probably"* wants
/// Smash physics in the flagship too and did not name a number; changing how the
/// whole game feels is his call, not an inference from this one.
///
/// 0.31 rad ≈ 18°: the victim's held stick may rotate its own launch by that
/// much, which is what makes a knock-off a read instead of a coin flip.
const VERSUS_DI_MAX_ANGLE: f32 = 0.31;

/// **What the fighting stage overwrote, so leaving can put it back.**
///
/// **Is the versus stage the one being played right now?**
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
    mut demand: ResMut<ambition_platformer2d::actors::character_runtime::CharacterLoadDemand>,

    mut match_state: ResMut<super::versus_rules::VersusMatch>,
) {
    let on_versus = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == VERSUS_GAMEPLAY_ROUTE);
    // MINE, not "a roster exists". Smash's character select publishes one from
    // a different route, and this system ran every frame: not on the versus
    // route, a roster present, remove it — deleting another game's match before
    // it could open. Neither stage was wrong on its own; the resource is global
    // and had no owner until it did.
    let mine = roster
        .as_ref()
        .is_some_and(|roster| roster.is_published_by(VERSUS_EXPERIENCE));
    match (on_versus, mine) {
        (true, false) => {
            // One local player per connected controller, capped by the stage's
            // two seats. Zero controllers is one seat: the keyboard is player
            // one, which is how every other route in the shell already plays.
            // THE SESSION'S FROZEN SEATING when one exists, the live order
            // otherwise. A rollback session decides its handle count once, at
            // session start; a roster that re-sampled the live device order
            // could seat a fighter the session has no handle for, and both
            // would be citing "the connected controllers" (GPT 5.6,
            // 2026-07-28). Without a session — the shell's non-rollback
            // routes — the live order IS the answer.
            let frozen = topology.as_ref().filter(|topology| topology.is_frozen());
            let roster = versus_roster_from(
                frozen
                    .map(|topology| topology.players())
                    .unwrap_or_else(|| devices.devices().len().max(1)),
                // ⭐ **frozen means ACTIVATED, unfrozen means PROPOSED.** The
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
            // ⭐ **THE SEAT COUNT THIS MATCH DECIDED, published with the roster
            // and under this experience's name.** The local session maintainer
            // freezes its topology from connected DEVICES otherwise — and
            // devices are not participants: a keyboard seat has no controller
            // entity, a spare pad may not be playing, a CPU seat has none at
            // all.
            //
            // ⚠ the claim is OWNED, and that is what makes it releasable: this
            // route's scope hands it back on the way out, so the next
            // experience's session is not sized by a match that has ended.
            commands.insert_resource(
                ambition_platformer2d::rollback::local_session::SessionSeatingSource::decided(
                    VERSUS_EXPERIENCE,
                    // CHANNELS, not participants — the versus stage published
                    // the same conflation Smash did. See
                    // `ControllerBinding::local_source`.
                    roster.local_channel_plan(),
                ),
            );
            commands.insert_resource(roster);
            // A NEW roster is not yet a match. Activation is seating's to
            // publish, once every participant has a body.
            commands
                .remove_resource::<ambition_platformer2d::actors::character_runtime::ActiveMatch>();
            // NO global free-for-all. The fighters are on declared TEAMS
            // (`blue` / `red`), and `MatchTeam` outranks faction for "may this
            // land" — which is what the roster's teams were for since §7.8 and
            // what nothing read until 2026-07-27.
            //
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
                // ⛔ BY OWNER, because this stage is no longer the only one that
                // declares rules — the smash demo does too, and a giveback that
                // removed the resource by TYPE would delete whichever stage was
                // still playing. Third time this repo has learned it: the
                // participant roster, the prepared match, now the rules.
                declared_by: VERSUS_EXPERIENCE.to_string(),
                di_max_angle: VERSUS_DI_MAX_ANGLE,
                // ⚠ no meteor rule here for the same reason the knockback stays
                // flat: rounds end on health, not on a blast zone, so a spike
                // has nowhere to send you and a window you cannot recover in
                // would be a stun with no payoff.
                meteor_lock_time: 0.0,
                // ⚠ and no rage, for the third time the same reason: health
                // rounds have no percent mechanic to mirror.
                rage_per_damage: 0.0,
                rage_max_scale: 1.0,
                // ⚠ and no staling: rounds this short do not repeat a move
                // enough for a queue to mean anything.
                stale_step: 0.0,
                stale_floor: 1.0,
                // ⚠ the generic versus stage stays FLAT for now: its rounds end
                // on health rather than on a blast zone, so a launch that grows
                // without bound is a different game's mechanic. Smash declares
                // its own — see `SMASH_KNOCKBACK_GROWTH`.
                knockback_growth: 0.0,
                // ⚠ the generic versus stage keeps the POGO reading, deliberately:
                // its rounds end on health rather than on a blast zone, so a
                // spike has nothing to kill you off and the rebound is the more
                // useful of the two. Smash declares otherwise — see
                // `SMASH_KNOCKBACK_GROWTH`'s neighbour.
                downward_hit: ambition_platformer2d::combat::rules::DownwardHitStyle::Pogo,
                friendly_fire: false,
                // ⚠ **the versus route says NOTHING here** (2026-08-12). Its
                // seats are the shipped cast, every one of which authors its own
                // kit; a floor is what a stage needs when a body does not, and
                // this stage does not have that body. `None` leaves the engine's
                // exploration default standing, which nothing here reaches.
                unarmed_melee: None,
            });
            // A FRESH match. `VersusMatch` is a long-lived resource, so without
            // this, leaving mid-round and coming back resumes the old score —
            // and a KO or match-over countdown resumes with it, which reads as
            // the stage opening onto somebody else's game (GPT 5.6,
            // 2026-07-27). Reset on ENTRY rather than exit so a crash or a
            // route change that skips the teardown still starts clean.
            *match_state = super::versus_rules::VersusMatch::opening();
        }
        // ⭐ **THE EXIT IS A DECLARATION, not a branch.** Everything this route
        // published leaves with the experience — see the scope in
        // [`compose_versus_experience`]. It used to be a `(false, true)` arm
        // here, which could only fire while `track_versus_roster` was still
        // installed and still running, and which named "the roster exists" as
        // its trigger rather than "my experience ended".
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
    // ⚠ **no archetype fragment.** A CPU seat's `brain_profile` resolves against
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
    // The scoreboard. One health gauge PER SEAT plus two text readouts; the
    // engine never learns what a ROUND is — `publish_versus_hud` writes the
    // words.
    //
    // Four gauges are declared and a 1v1 fills two, because a HUD declaration is
    // a statement about the STAGE and the stage seats four. Declaring two and
    // making the third fighter share one is what the first version did.
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
        // ⚠ this said "published only while a round is over, so it needs no hide
        // path". True until the countdown landed (2026-07-28) and false after:
        // the count publishes every tick of a LIVE-adjacent phase, so
        // `publish_versus_hud` clears the slot explicitly on `Fighting`. Without
        // that the word FIGHT would sit over the whole round.
        .slot(
            ambition_platformer2d::presentation::HudSlotSpec::new(
                super::versus_rules::ANNOUNCE_HUD_SLOT,
            )
            .centered()
            .with_font_size(34.0)
            .with_color([1.0, 0.85, 0.3, 1.0]),
        )
    })
    // ⭐⭐ **COMPOSED AND ROUTED, BUT NOT OFFERED** (Jon, 2026-08-15: the
    // game-selection shell should list games, and these can be standalone for
    // tests).
    //
    // ⛔ **it cannot become a standalone binary, and that is a fact about what it
    // IS rather than an omission.** Its fighters are `mary_o` and `sanic`,
    // registered by two DIFFERENT provider plugins, so the multi-game shell host
    // is the only composition where both casts exist — which is exactly why this
    // module lives in the app and says so at the top. Dropping the composition
    // would delete the only proof in the workspace that two providers' characters
    // can fight, at the moment the Smash lane is reporting the same boundary from
    // the other side.
    //
    // ⇒ so the ROW goes and the STAGE stays. `versus_stage.rs` drives the real
    // shell and activates `VERSUS_GAMEPLAY_ROUTE` by id; nothing it asserts is
    // about the launcher listing it.
    .unlisted()
    .install(app, versus_prepared_session_world);

    app.init_resource::<super::versus_rules::VersusMatch>();
    // The scoreboard is SIMULATION state: a rewind that restores the fighters
    // and not the score leaves the two disagreeing about what round it is.
    {
        use ambition_platformer2d::rollback::AmbitionRollbackApp;
        // ⭐ **a value checksum over the match, not merely its presence**
        // (2026-08-06). `VersusMatch` decides the round, the phase and who has
        // won — two timelines that disagree about the phase disagree about
        // whether input is even accepted, and a presence probe saw none of it.
        // Eighteen of these surfaced at once when K2b edit 2 made the rollback
        // harness compose the shipped host; see `ambition_demo_mary_o`'s
        // `rollback_probes` for the family.
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
                // ⛔ **ONLY WHILE THIS STAGE IS THE ONE BEING PLAYED.**
                //
                // It ran unconditionally, and `VersusMatch` defaults to
                // `MatchPhase::Starting` — whose arm calls `take_the_controls`
                // on EVERY fighter, every tick, idempotently. So in any
                // composition that installs this stage and is not on its route,
                // every seated fighter carried `ScriptedControl` forever and
                // `blank_scripted_control_frames` zeroed its control frame each
                // tick. The input arrived correctly the whole way — GGRS
                // published it, the brain read it — and the body never moved.
                //
                // ⭐ **the same rule `track_versus_roster` learned about the
                // roster**: not "a fighter exists", MINE. A global resource with
                // no owner deleted another game's match on 2026-08-01; a global
                // countdown with no owner froze another game's fighters, and the
                // two are the same mistake one campaign apart.
                //
                // Invisible until K2b edit 2 (2026-08-06) made the rollback
                // harness compose the shipped shell host, which installs this
                // stage. `two_local_seats_drive_independently_under_a_rollback_host`
                // is what caught it.
                .run_if(versus_stage_is_active),
        );
    }

    // The ROUND lifetime (Campaign 3A). Installed by the thing that composes a
    // MATCH rather than by the engine at large — a single-player platformer has
    // no rounds and should carry no round culler. `settle_versus_round` mints the
    // next round; this despawns whatever belonged to the last one, so the rules
    // never enumerate the transient families that might exist.
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

    // `Update`, NOT the sim schedule.
    //
    // The first draft put this in `Platformer2dSimulationPhaseMonolith::PlayerInput`, and its own test
    // caught the consequence: every `Platformer2dSimulationPhaseMonolith` is nested inside
    // `GameplaySimulationRoot`, which carries the session gate, so leaving
    // gameplay switches OFF the system whose job is to clean up after leaving
    // gameplay. The roster survived, and the next game the player picked would
    // have had two fighters seated into it — silently, because the bodies look
    // like they belong to the level.
    //
    // A teardown that lives inside the thing it tears down can only ever run
    // while it is not needed. This reads the shell router and writes resources;
    // it is shell lifecycle, and the shell clock is where it belongs.
    app.add_systems(
        Update,
        (
            track_versus_roster,
            // AFTER, and in the same tick: the roster this built may have been
            // decided before the session froze its seating.
            reconcile_roster_with_frozen_topology,
        )
            .chain()
            // ⛔ **BEFORE THE SESSION IS SIZED, and stated rather than hoped.**
            // The maintainer freezes a topology from connected DEVICES when
            // nothing has claimed roster-driven seating, and the session is
            // never resized afterwards — so a maintainer that ran first on the
            // frame this route opens sized the whole match from what was
            // plugged in. Same schedule, so this is a real edge; a cross-
            // schedule `.after` would be silently vacuous.
            .before(
                ambition_platformer2d::rollback::local_session::LocalSessionSet::Maintain,
            ),
    );

    declare_versus_experience_scope(app);
}

/// **WHAT THIS EXPERIENCE OWNS, AND WHAT LEAVES WITH IT.**
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
            // ⛔ **by OWNER, and the owner is on the PLAN.** This removed the
            // activation by TYPE until 2026-08-07, as did smash's scope three
            // files away — so whichever experience left first deleted the
            // other's live match, and each declaration read as correct on its
            // own. `ActiveMatch` is rollback state that deliberately carries no
            // identity, so the plan it was activated from answers for it.
            .releasing_witnessed::<
                ambition_platformer2d::actors::character_runtime::ActiveMatch,
                ambition_platformer2d::actors::character_runtime::PreparedMatch,
            >(|plan, owner| plan.is_published_by(owner.as_str()))
            // AND THE PLAN it activated from. A `PreparedMatch` that outlives
            // its experience is the next game's stage quietly building THIS
            // game's fighters — the same lesson as the roster above, one
            // resource later.
            //
            // ⚠ declared AFTER the activation above, which reads it: releases
            // run in declaration order and the witness has to still be here.
            .releasing_owned::<ambition_platformer2d::actors::character_runtime::PreparedMatch>(
                |plan, owner| plan.is_published_by(owner.as_str()),
            )
            // DROP THE DECLARATION. A match rule that outlives its match is a
            // rule the next game silently inherits, and "your allies can now
            // shoot you" is a bad surprise to bring into a co-op level.
            //
            // ⚠ there is nothing to put back, and that is the point: writing the
            // engine defaults reads as a restore and is not one.
            .releasing_owned::<ambition_platformer2d::combat::rules::DeclaredCombatRules>(
                |rules, owner| rules.is_declared_by(owner.as_str()),
            )
            .releasing_with("SessionSeatingSource", |world, owner| {
                if let Some(mut seating) = world.get_resource_mut::<
                    ambition_platformer2d::rollback::local_session::SessionSeatingSource,
                >() {
                    seating.release(owner.as_str());
                }
            });
    }
}

#[cfg(test)]
mod stage_rule_tests {
    use super::*;
    use ambition_platformer2d::actors::time::feel::Platformer2dFeelTuningMonolith;

    fn stage_rule_app() -> App {
        let mut app = App::new();
        app.init_resource::<Platformer2dFeelTuningMonolith>();
        app.init_resource::<ambition_platformer2d::combat::targeting::FriendlyFire>();
        app.init_resource::<super::super::versus_rules::VersusMatch>();
        app.init_resource::<ambition_platformer2d::actors::character_runtime::CharacterLoadDemand>(
        );
        app.init_resource::<ambition_platformer2d::input::LocalDeviceOrder>();
        app.insert_resource(ambition_platformer2d::game_shell::ShellRouter::default());
        // The projection normally runs in `Platformer2dSimulationPhaseMonolith::WorldPrep`; here it runs
        // straight after the declarer, which is the same ORDER and all these
        // tests need. Asserting on the resolved value rather than on a global is
        // the whole point of AE6 — a test that read the baseline would be
        // asserting the borrow it replaced.
        // The REAL exit mechanism, not a fixture copy: the versus scope
        // declaration (the same call `compose_versus_experience` makes) plus
        // the shell's release system. These tests went red for a day when the
        // exit moved from a teardown arm inside `track_versus_roster` into the
        // scope declaration and this fixture kept composing only the former —
        // asserting "DI leaves with the match" against an app that contained
        // no leaving mechanism at all.
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

    /// **A match leaves with its route — and takes only its OWN.**
    ///
    /// ⛔ **the bug this pins shipped for a day and read as correct in both
    /// files that had it.** Versus removed `ActiveMatch` and `PreparedMatch` by
    /// TYPE on the way out; so did smash, in a host (`shell_host.rs`) that lists
    /// both. Whichever experience left first deleted the other's live match, and
    /// the comment directly above each declaration explained — correctly, about
    /// the roster — why removing a shared global by type is exactly that bug.
    ///
    /// ⚠ **both halves, because either alone is green for the wrong reason.**
    /// Deleting the release entirely passes "a stranger's survives"; leaving it
    /// unconditional passes "mine is gone". Only the pair pins the fix.
    #[test]
    fn a_match_leaves_with_its_route() {
        use ambition_platformer2d::actors::character_runtime::{ActiveMatch, PreparedMatch};

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

    /// **...and never another game's.** The other half of the pair above.
    ///
    /// The plan is the WITNESS: `ActiveMatch` is rollback state that carries no
    /// identity by design, so the question "whose match is this" is answered by
    /// the plan it was activated from. A stranger's plan must protect the
    /// stranger's activation too, which is the property a release keyed only on
    /// the plan could get wrong in one direction.
    #[test]
    fn another_experiences_match_is_left_standing() {
        use ambition_platformer2d::actors::character_runtime::{ActiveMatch, PreparedMatch};

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

    /// **DI is a rule of the fighting stage, and it leaves with the stage.**
    ///
    /// Jon: *"In smash DI is critical!"* — and also that he only *"probably"*
    /// wants Smash physics in the flagship, without naming a number. So the
    /// budget is switched on by the versus route and gone again on the way out.
    ///
    /// The leaving half is the half worth pinning. A match rule that outlives its
    /// match is a rule the next game silently inherits, which is exactly the bug
    /// `FriendlyFire` had here: leaving the arena used to leave "your allies can
    /// now shoot you" behind in a co-op level. DI leaking would be quieter and
    /// worse — every knockback in Ambition would start steering, and nothing
    /// would say why.
    ///
    /// ⚠ **asserted on the RESOLVED tuning, and on the baseline NOT MOVING**
    /// (AE6). The stage used to reach this by writing `Platformer2dFeelTuningMonolith`; it
    /// declares now, so the second assertion in each pair — that the world's own
    /// tuning is exactly where it was — is the one that would catch a relapse.
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

    /// **The half the default-start test could not see, and what replaced it.**
    ///
    /// Exit used to write `Platformer2dFeelTuningMonolith::default()` and `FriendlyFire =
    /// false`, and the test above starts from exactly those, so reset and
    /// restore were indistinguishable in it. Starting from values NOBODY ships
    /// made the difference visible: an experience that authored its own DI budget
    /// and its own friendly-fire rule had to get them back, having lost them
    /// merely because the player visited the versus route.
    ///
    /// ⚠ **the restore is gone, because the borrow is** (AE6). The stage declares
    /// its rules and the projection folds them over the baseline, so the
    /// baseline is never written and there is nothing to hand back. This test
    /// keeps the same hostile starting values and asserts the stronger property:
    /// the authored world is byte-identical before, DURING, and after a match.
    /// A relapse to writing globals fails on the middle assertion, which is the
    /// one the old shape could not make at all.
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
        // DURING. The old shape could not express this: while a match was live,
        // the world's tuning WAS the match's tuning, so there was no moment at
        // which the two could be compared.
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
    use ambition_platformer2d::actors::character_runtime::{ActiveMatch, CharacterLoadDemand};
    use ambition_platformer2d::input::{LocalDeviceOrder, LocalSeatTopology};

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

    /// **Activation VALIDATES, and a roster this composition cannot fill is not
    /// activated.**
    ///
    /// `status.md`'s activation row asks for *"validate every participant,
    /// activate the roster atomically"*. The validation existed —
    /// `unsatisfiable_seats` — and its caller was `seat_match_participants`,
    /// which runs one step AFTER the roster is live. So a route could activate a
    /// match naming a brain profile its own `CharacterRoster` has never heard
    /// of, seating would refuse it every tick, and the stage would sit forever on
    /// a match that publishes a refusal instead of fighters. That is not
    /// hypothetical: the versus stage named `medium_striker` and the smash demo
    /// named `duelist` before either composition registered one, on 2026-07-31,
    /// and both shipped looking composed.
    ///
    /// ⚠ **the fixture registers an EMPTY archetype table, not none.** Absent is
    /// a different answer — a content-free composition legitimately has no
    /// archetypes and activating unvalidated is the right behaviour there, since
    /// refusing would strand it. Empty means "this composition has a table and
    /// your profile is not in it", which is the failure being caught.
    #[test]
    fn a_roster_this_composition_cannot_seat_is_not_activated() {
        let mut app = App::new();
        // ⚠ **ONE local player, and that is not a detail.** `versus_roster_from`
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

        // ⭐ and the SAME roster activates once the composition can answer for it,
        // or the refusal above would pass against a route that never activates.
        // ⚠ assembled from the stage's OWN catalog RON, not a hand-built table:
        // that file is what a shipped composition registers, so this half also
        // asserts `VERSUS_CPU_BRAIN` is actually in it — the exact thing that was
        // false on 2026-07-31 when the stage named `medium_striker`.
        //
        // ⛔ it registered an archetype FRAGMENT until 2026-08-13, because the
        // policy lived in `VERSUS_ROSTER_RON`. Same claim, one authority over:
        // a CPU seat's controller question is answered by published
        // `autonomous_profiles`, so that is what the composition must carry.
        {
            use ambition_platformer2d::characters::actor::character_catalog::{
                CharacterCatalogAppExt, CharacterCatalogFragment,
            };
            // ⭐ through the App seam the composition itself uses, so what is
            // published here is what a shipped versus route publishes — the
            // registration is what turns an authored profile into a resolvable
            // `versus::versus_duelist`.
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

    /// ⛔ **Another game's roster is not this one's to rebuild.**
    ///
    /// `versus_roster_from` stamps `published_by: ambition_versus`, so rebuilding
    /// a roster somebody else published does not resize it — it TRANSFERS
    /// OWNERSHIP, and `maintain_versus_stage` then deletes it, correctly, as its
    /// own on a route that is not versus. Smash's select screen published two
    /// fighters, this took them, versus threw them away, and the stage opened
    /// with one fighter (Jon, 2026-08-01).
    ///
    /// ⚠ the fixture has to FREEZE a topology, because that is the only
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

    /// **A roster decided before the session froze its seating is rebuilt.**
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

    /// **A HALF-SEATED match is not handed a different roster.**
    ///
    /// The test above is the safe case and was read as the general one: "no
    /// `ActiveMatch`" was taken to mean "nothing has a body yet". It does not.
    /// Seating retries across ticks and the latch closes only when every seat is
    /// filled, so a match with one fighter standing and one still pending sits in
    /// exactly this state — and swapping the roster there is not a rebuild, it is
    /// a match assembled from two rosters. Seating skips a seat index it finds
    /// occupied, so the standing body keeps the OLD character, team and
    /// controller kind while the latch closes over the NEW definitions, and the
    /// warning that used to follow could not undo any of it (GPT 5.6,
    /// 2026-07-30).
    #[test]
    fn a_half_seated_match_is_not_handed_a_different_roster() {
        let mut app = app_with(versus_roster_from(2, RosterSeating::Proposed), 3, false);
        // One fighter is already standing on the stage; the other has not seated
        // yet, so `ActiveMatch` is absent — the exact window.
        app.world_mut()
            .spawn(ambition_platformer2d::actors::character_runtime::MatchSeat(
                0,
            ));
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
            .spawn(ambition_platformer2d::actors::character_runtime::MatchSeat(
                0,
            ));
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

    /// **A topology that has decided nothing does not get to overrule live
    /// discovery.**
    ///
    /// `generation == 0` means never captured — the resource exists, no session
    /// has frozen its seating. Treating that as an answer would rebuild every
    /// roster down to the two-seat floor. Written because removing the
    /// `is_frozen` guard left the other three tests green: an unverified guard
    /// is one that gets deleted by the next person who reads it as noise.
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

    /// **The stamp is repaired when the match and the session agree about WHO.**
    ///
    /// The ordinary sequence, and the one that made this worth building: the
    /// route builds its roster from live device discovery on entry, stamped
    /// `None` because no session has frozen anything yet; the fighters seat; the
    /// rollback session starts and freezes its topology. Now the roster
    /// disagrees with the session about which decision produced it while
    /// agreeing completely about who is fighting.
    ///
    /// Before this, that warned — on a stage where nothing was wrong. Y′9 left
    /// the post-seating case "reported rather than repaired"; repairing the
    /// repairable half is what makes the remaining report mean something.
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

    /// **After seating it is bodies on a stage, and it is left alone.**
    ///
    /// Reseating mid-match would be a worse bug than the disagreement it fixes.
    /// The disagreement is reported instead — this is the case match ACTIVATION
    /// exists to make impossible, and pretending a rebuild is safe here would
    /// hide the reason that work is still open.
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
