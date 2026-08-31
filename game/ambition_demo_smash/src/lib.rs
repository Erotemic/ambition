//! Standalone stocks-based platform-fighter demo.
//!
//! Combat damage does not kill a stocks fighter; leaving the stage emits the
//! ruleset-owned knockout signal. The combat crate owns stock accounting, while
//! this demo supplies stage-specific respawn placement and match completion. It
//! also serves as an external-style consumer of the umbrella platformer API.

// no `ambition_platformer2d::prelude::*`. Declaring a match needs the ACTOR
// vocabulary, not the room-authoring one, and reaching for the prelude here
// would import nothing this file uses. That the prelude does not cover a match
// is a fact about what a prelude is for, not a gap.
use ambition_platformer2d::actor::{ControllerBinding, MatchParticipant, MatchParticipantRoster};
use ambition_platformer2d::character::CharacterDefinition;
use ambition_platformer2d::character::Vitals;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::Vec2;
use ambition_platformer2d::world::rooms::RoomSpec;

pub mod bomb;
pub mod capture;
pub mod george_booul_moveset;
pub mod moveset;
pub mod select;
pub mod select_screen;
pub mod shark_ride;
pub mod smash_pack;

/// The game-MODE tag this demo's rules gate on, so they sleep everywhere else.
pub const SMASH_MODE: &str = "smash";

/// Stocks each fighter starts with.
///
/// Three, because it is the smallest number that makes the middle of a match
/// feel different from its start and its end: at three you can lose one and
/// still be playing the same match, which is the thing rounds cannot express.
pub const STARTING_STOCKS: u32 = 3;

/// What 100% means.
///
/// The denominator of `damage_percent()`. Under `DeathPolicy::Unbounded` the
/// pool never kills, so this is purely the scale a percent is read against —
/// which is exactly why it has to be declared: an undeclared pool is whatever
/// the CHARACTER authored, and a meter divided by one reports 14000%.
///
/// THE MATCH declares it, not the characters. See `apply_smash_match_rules`.
pub const SMASH_PERCENT_REFERENCE: i32 = 100;

/// The published controller policy a CPU seat asks for — `smash::duelist`,
/// resolved in this stage's own provider.
///
/// They are deleted; the six are `autonomous_profiles` in the catalog above.
///
/// Two vocabularies sharing one word cost the same day twice.
pub const SMASH_DUELIST_BRAIN: &str = "duelist";

/// Where a respawning fighter comes back, above the stage centre.
///
/// above, not at the spawn point. A fighter that reappears on the floor
/// reappears inside whatever is standing there — and in a fight, what is
/// standing there is the opponent who just knocked it off. Respawn height is the
/// oldest rule in the genre and it is a rule about SAFETY, not about drama.
pub const RESPAWN_HEIGHT_PX: f32 = 160.0;

/// The one fighter on this grid whose up-B summons a mount.
pub const SMASH_SHARK_RIDER: &str = "npc_pirate_admiral";

/// Build the roster for a stocks match between `characters`.
///
/// `fighter_stocks` declares BOTH halves at once — the count AND
/// `DeathPolicy::Unbounded` — because neither is meaningful alone: stocks over a
/// meter that kills at max are never consulted, and an unbounded meter with no
/// stocks is a fighter that cannot lose. That pairing is the engine's, not this
/// crate's, which is exactly the kind of thing a demo should not be able to get
/// wrong.
pub fn smash_roster<I, S>(characters: I) -> MatchParticipantRoster
where
    I: IntoIterator<Item = S>,
    S: Into<ambition_platformer2d::entity_catalog::CharacterId>,
{
    let mut roster = MatchParticipantRoster::of(Vec::<String>::new());
    roster.participants = characters
        .into_iter()
        .enumerate()
        .map(|(index, character)| {
            let character: ambition_platformer2d::entity_catalog::CharacterId = character.into();
            // ⛔ THE MOUNT LICENCE IS NOT GRANTED HERE ANY MORE. It is a rule
            // of the MATCH, so `apply_smash_match_rules` states it for every
            // road that builds one — see the comment there.
            MatchParticipant::new(character)
                .driven_by(if index == 0 {
                    ControllerBinding::Human {
                        source: ambition_platformer2d::actor::LocalInputSource::FIRST_PAD,
                    }
                } else {
                    ControllerBinding::Cpu {
                        // The FB4b rig, by the catalog preset name. `medium_striker`
                        // was Ambition's generic swipe brain and this demo does not
                        // ship it — a CPU seat asking for a preset the composition
                        // has never heard of resolves to nothing and stands still.
                        brain_profile: Some(SMASH_DUELIST_BRAIN.to_string()),
                    }
                })
                // Every seat its own side. A stocks match with teams is a legal
                // and ordinary thing, but the demo's job is the SIMPLEST shape
                // that exercises the loop, and free-for-all is it.
                .on_team(format!("seat {}", index + 1))
        })
        // ⭐⭐ THE FIGHTER'S OWN BODY, where its character package authored one.
        //
        // A seat that says nothing here composes the stage's six numbers over
        // the WANDERING-ENEMY baseline, because that is what an unauthored actor
        // config carries — measured, and it is an eighth of the player's ground
        // acceleration. A character states its fighter body in its own
        // `smash_fighter` facet, because a catalog row would state it for the
        // same character standing in a hub as well.
        .map(
            |participant| match crate::smash_pack::fighter_body(participant.character.as_str()) {
                Some(body) => participant.with_body(body),
                None => participant,
            },
        )
        .collect();
    apply_smash_match_rules(&mut roster);
    roster.published_by(SMASH_EXPERIENCE)
}

/// WHAT KIND OF MATCH THIS IS — the Smash ruleset, in one place.
pub fn apply_smash_match_rules(roster: &mut MatchParticipantRoster) {
    // ⛔⛔ THIS MATCH GRANTS NO PILOT LICENCE, AND THE HISTORY IS WORTH KEEPING.
    // It briefly did, in two places and then in one: `smash_roster` granted the
    // shark class per seat, `SmashSelect::roster_seeded` — the road a player
    // actually travels from the character-select grid — assembled its
    // participants from scratch and never did, and the admiral reached the match
    // unable to board the shark its own up-B summons. Jon found it by playing.
    //
    // ⭐ THE REAL FIX WAS UPSTREAM OF BOTH ROADS. An admiral can ride a shark
    // because it is an admiral — Jon: *"Yes the admiral could fly on a shark in
    // ambition"* — so `npc_pirate_admiral` authors `pilotable_classes` like
    // `npc_pirate_raider` already did, and `prepared_match` unions it into
    // `CanPilot` on every road that builds a body. A match that manufactures a
    // capability its cast already owns has two answers to one question.
    //
    // ⚠ AND WHICH SHARK is a different question, answered on the MOUNT: see
    // `ambition_mount::MountReservedFor`, which is what stops the second admiral
    // in a mirror match from boarding the first one's summon.
    roster.rules.opens_suspended = true;
    // THE OPENING CEREMONY: 3 — 2 — 1 — GO.
    //
    // Three beats at 60Hz. The hold was already here and had nothing to wait
    // for, so it came off on the tick the cast was built and the round began
    // with two fighters already moving before a player had looked at the stage.
    //
    // ticks rather than seconds, because the release is a comparison against
    // the sim clock — see `MatchRules::opening_countdown_ticks`.
    //
    // ⚠⚠ TEMPORARY DEV MODE, Jon 2026-08-26: *"make the 3, 2, 1, countdown go
    // 10x as fast."* The ceremony is here to be WATCHED, and watching it once per
    // playtest iteration is three seconds a fighter is not being tuned. Revert by
    // setting `COUNTDOWN_SPEEDUP` back to 1 — the divisor is named rather than
    // folded into the number so that reverting is a one-token edit and so that
    // nobody later reads `18` as a considered ceremony length.
    //
    // ⛔ NOT A SETTING and not a feature flag: a knob would outlive the reason
    // for it and would need a menu row, a default and somewhere to persist. This
    // is a constant with a date and a sentence, which is what temporary means.
    roster.rules.opening_countdown_ticks = 3 * 60 / COUNTDOWN_SPEEDUP;
    // THE MATCH CLOCK. Ultimate's default stock match runs eight minutes,
    // and the clock exists so a match between two fighters who will not approach
    // each other still ends — the stock economy alone has no answer to that.
    // derived from `ActiveMatch::activated_on`, so it costs no rollback state;
    // see `MatchRules::time_remaining`.
    roster.rules.time_limit_ticks = 8 * 60 * 60;
    roster.rules.stocks = Some(STARTING_STOCKS);
    // The match supplies one health pool for percent calculation so crossover
    // characters are measured against this ruleset rather than their home games.
    roster.rules.health_pool = Some(SMASH_PERCENT_REFERENCE);
    // Every fighter gets the ruleset's FLOOR, keeps whatever of its own kit the
    // CEILING permits, and brings nothing else from its home game. The gap
    // between the two constants is exactly one verb, and it is the one Jon named:
    // Robot v3 keeps its pogo because Robot v3 authored it.
    roster.rules.abilities = Some(ambition_platformer2d::engine_core::MatchAbilities {
        granted: SMASH_FIGHTER_KIT,
        permitted: SMASH_FIGHTER_CEILING,
    });
    // Apply the ruleset's body baseline alongside its ability policy.
    roster.rules.body = Some(SMASH_FIGHTER_BODY);
    // ⛔ NO ITEMS, and that is Jon's call rather than a gap. 2026-08-24: *"we
    // don't need items in smash right now. We eventually will, but not right
    // now."* The MACHINERY is built and tested — `MatchItemSpawns`, the
    // deterministic spawner, the weighted table — and turning it on is this one
    // declaration. What is deliberately absent is the DECLARATION, not the
    // capability.
    //
    // ⭐ THE TABLE THAT WAS HERE, so the day it comes back nobody re-derives it:
    // a drop every 8s over three points above the platform, weighted
    // bomb 4 / gravity_grenade 2 / gun_sword 1 — items whose `Attack` does
    // something on its own. ⛔ not the `UseSystem` items (meteor gauntlet,
    // mark/recall): those are abilities a body wields, which is a different
    // mechanic from an item fight.
    roster.rules.item_spawns = None;
}

/// SMASH'S READING OF A CHARACTER — a function from what the character
/// AUTHORED to what this match's seat plays with.
///
/// PURE, and that is the requirement. Two of this
/// ruleset's three adjustments already go through one named composition site —
/// [`apply_smash_match_rules`] declares them and `MatchRules::body_over` /
/// `MatchRules::pool_over` compose them. The third did not: the registration
/// loop in `install_smash_content` reached into `definition.vitals` and ASSIGNED
/// a weight, mid-loop, on the way past. That reach-in is now this function, and
/// grepping the name below finds every place the smash ruleset interprets
/// authored character data.
///
/// the orthogonality this expresses is not new here. Character authoring
/// and ruleset specificity are independent axes: data may live WITH the
/// character while being owned SEMANTICALLY by the smash capability. Mary-O's
/// move table already works exactly this way — it sits in her own crate, is
/// unreachable in her own game, and speaks smash's vocabulary.
///
/// What it buys is that the eventual answer — character-owned, game-owned, or composed — is ONE
/// edit either way.
///
/// the authored numbers and their reasoning are unchanged. Weight is a SPREAD around the
/// reference body rather than three absolute numbers: v3 is the middleweight the stage is tuned
/// against, v2 is the lighter older build, George is the heavy.
pub fn smash_reading_of_character(
    definition: ambition_platformer2d::characters::actor::definition::CharacterDefinition,
) -> ambition_platformer2d::characters::actor::definition::CharacterDefinition {
    let knockback_weight = match definition.id.as_str() {
        SMASH_OPPONENT_ID => 0.85,
        SMASH_GEORGE_BOOUL => 1.35,
        _ => 1.0,
    };
    CharacterDefinition {
        vitals: Vitals {
            knockback_weight: Some(knockback_weight),
            ..definition.vitals
        },
        ..definition
    }
}

/// Match-level movement overrides shared by platform fighters on the Smash stage.
///
/// These values are authored by the match rather than engine defaults: melee recoil
/// is disabled, jump squat and the floor game are enabled, and fighters receive the
/// match air-dodge/SDI behavior. Character-specific movement outside these fields is
/// preserved by `MatchRules::body_over`.
pub const SMASH_FIGHTER_BODY: ambition_platformer2d::engine_core::MatchBody =
    ambition_platformer2d::engine_core::MatchBody {
        slash_recoil: 0.0,
        jump_squat_time: 3.0 / 60.0,
        air_dodge_time: ambition_platformer2d::engine_core::AIR_DODGE_TIME,
        air_dodge_speed: ambition_platformer2d::engine_core::AIR_DODGE_SPEED,
        air_dodge_endlag: ambition_platformer2d::engine_core::AIR_DODGE_ENDLAG,
        // ⭐⭐ THE ROLL OWES A BEAT. Jon, 2026-08-24: *"shield rolls have too
        // much motion to them... they probably should stop at the end of the
        // roll and leave the character punishable for a frame or two."*
        //
        // ⛔ THE DISTANCE IS NOT WHAT WAS WRONG, and it is deliberately
        // unchanged: 530px/s over a 0.22s window is ~117px, which is a step and
        // a half. What made a roll read as "flying across the stage" is that
        // NOTHING took the velocity back when the window closed, so the body
        // kept travelling at roll speed until friction or a wall caught it. The
        // roll comes to rest now; changing the speed on top of that would be
        // nerfing the same thing twice.
        dodge_roll_endlag: ambition_platformer2d::engine_core::DODGE_ROLL_ENDLAG,
        // ⭐⭐ DODGE STALING — the genre's answer to rolling being the answer to
        // everything, and the other half of the roll question Jon raised. A
        // quarter of the invulnerable window comes off per recent evade, floored
        // at a third, forgiven one at a time every 1.2s.
        //
        // ⛔ IT WEARS THE I-FRAMES, NOT THE DISTANCE. A stale roll still travels
        // and still recovers — it is simply no longer safe, which is a read a
        // player can see without a HUD. Shortening the roll instead would make
        // the fighter feel broken rather than punished.
        dodge_stale_step: 0.25,
        dodge_stale_floor: 0.34,
        dodge_stale_recovery: 1.2,
        // ⭐⭐ A KILL-POWER HIT COMMITS. Above 1400px/s the tumble cannot be
        // teched, so the hit that should end a stock is not survivable by a
        // well-timed press against the wall behind you.
        //
        // ⛔ WELL ABOVE THE TUMBLE THRESHOLD (500px/s), so ordinary launches
        // keep their escape and only the hard ones lose it — a threshold near
        // the tumble line would delete the tech instead of reserving it.
        untechable_launch_speed: 1400.0,
        // ⭐⭐ AN EVADE IS A COMMITMENT UNTIL ITS LAST FOUR FRAMES. Without this
        // a spot dodge is invulnerable AND cancellable into an attack on frame
        // one, which is strictly better than the genre's — the dodge answers
        // everything and costs nothing.
        //
        // ⛔ THE TAIL IS THE OPTION, not a nerf: spot-dodge-into-attack is a
        // real genre technique, and what it should cost is the frames before it.
        evade_cancel_tail: 4.0 / 60.0,
        // SPOT DODGE, 0.16s. The grounded evade had one shape, so the
        // option a cornered fighter takes — nowhere to roll TO, waiting out a
        // committed swing — did not exist. Shorter than the roll's window
        // because it covers no distance; a spot dodge that lasted as long would
        // be strictly better than the roll and the roll would stop being a
        // choice. The engine default is `0.0`: an exploration body keeps the
        // roll that press already means.
        spot_dodge_time: ambition_platformer2d::engine_core::SPOT_DODGE_TIME,
        // WHICH GAME'S PERFECT SHIELD. Smash 4 opens the window on the
        // press and Ultimate on the release, and the stage declares which — the
        // engine has no opinion. This stage plays the press-timed one for now
        // because it is what shipped; flipping it to `OnRelease` is a one-word
        // edit and the other setting is fully live (`resolve_shield`'s
        // `OnRelease` arm, guarded by
        // `the_parry_window_opens_where_the_ruleset_says_it_does`).
        parry_timing: ambition_platformer2d::engine_core::ParryTiming::OnRaise,
        tumble_speed: 500.0,
        // SDI, 3px a hitlag tick. DI already lets a launched fighter bend
        // where it is thrown; this is the other half — shifting out of the NEXT
        // hit's way while the current one is still frozen, which is what makes a
        // combo answerable rather than a sentence. The engine default is `0.0`:
        // a wandering enemy has no combo to escape.
        sdi_step: 3.0,
        // ⭐ ONE NUDGE PER HIT, twice a single SDI tick, paid when the freeze
        // lifts. It is what a defender gets out of a MULTIHIT, whose one-tick
        // freezes are worth almost nothing to `sdi_step`.
        asdi_step: 6.0,
        // ⭐ A JAB IS WORTH A FEW HUNDRED px/s and a smash is worth thousands,
        // so this threshold separates "poke a downed opponent" from "commit to
        // a launch" without naming a single move.
        jab_lock_speed: 320.0,
        // Three pins and the floor game resets — a real combo route, short of
        // an infinite.
        jab_lock_limit: 3,
        shield: ambition_platformer2d::engine_core::ShieldTuning::PLATFORM_FIGHTER,
        footstool: ambition_platformer2d::engine_core::FootstoolTuning::PLATFORM_FIGHTER,
        // A CROUCH PLANTS YOU. The genre's answer, and research rather than a
        // feel call: in every Smash, crouching stops you outright unless the
        // character has a crawl. What pays for the smaller hurtbox and the
        // shortened launch (`crouch_cancel_scale: 0.85` above) is your mobility,
        // and before this a crouching fighter kept both for free at full run
        // speed. ⛔ `0.0` rather than a shuffle because no fighter here authors a
        // crawl; the day one does, it declares its own.
        crouch_speed_frac: 0.0,
        // ⭐⭐ THE INITIAL DASH — the first 14 frames of a ground move, in
        // which a direction change is still free. It is what makes the ground
        // game a conversation: dash in, read the opponent, dash back out. The
        // same window is the foxtrot's re-tap and the dash-dance's reversal.
        // ⚠ a starting point taken from the genre, not a measurement of this
        // game: play it and move it.
        initial_dash_time: 14.0 / 60.0,
        // Inherit the run speed. The phase is about WHEN you may turn around,
        // not about being faster than a run.
        initial_dash_speed: 0.0,
        // ⭐⭐ AND REVERSING OUT OF A RUN COSTS YOU — the half that makes the
        // dash's free reversal above worth having, and what a pivot grab and a
        // reverse aerial rush are both thrown out of.
        //
        // ⚠ 3 FRAMES IS WHAT THE PROVING GROUND TOLERATES, not a measurement of
        // what feels right. At 7 frames `smash_it` lost two premise guards:
        // seat 0 (george_booul) stopped ever being knocked off the stage in a
        // 3600-tick match while seat 1 went off 57 times, so a CPU matchup that
        // used to trade became one-sided. The launch itself is fine — a body
        // launched mid-turnaround keeps its knockback, measured — so this is a
        // balance effect and not the velocity corruption the initial dash had.
        // ⇒ the number is a feel call and it is Jon's; 7 is where it visibly
        // tips this matchup.
        turnaround_time: 3.0 / 60.0,
        // ⭐ A QUARTER OF THE FOOTPRINT is the leading foot: step that far past
        // a ledge and the fighter is on the brink. Published only —
        // `BodyMotionFacts::teetering` is what animation and control read, and
        // nothing about collision changes.
        teeter_margin: 0.25,
    };

/// THE BASIC SMASH ABILITIES — the verbs every fighter on this stage has.
///
/// granted the basic smash abilities"*) and it is one constant so that the
/// stage, the tests and any future reader read the same one.
///
/// `fly` and `blink` are absent deliberately: this is a platform fighter's
/// ground game, not the exploration protagonist's traversal kit, and the July
/// measurement of two seats disagreeing was exactly a duelist meeting a body
/// that could fly. `interact` and `reset` are absent for the same reason a
/// fighter has no talk button and no teleport home.
///
/// `shield`, `dodge` and `ledge_grab` are what make this a platform fighter
/// rather than two bodies running at each other. All three already existed in
/// the engine with nothing switched on.
///
/// Dash should be an ability for ambition, it doesn't map into a smash vocabulary."*).
/// `AbilitySet::dash` is not running — running is `move_horizontal` against the body's own top
/// speed, and it consults no ability bit beyond that one. `dash` is a DISCRETE charge-gated burst
/// that REPLACES the velocity vector for a window (`apply_dash`), which is a traversal verb from
/// Ambition's exploration kit and not one of a platform fighter's sixteen presses. Dropping it
/// leaves the burst BUTTON meaning exactly one thing here — the dodge — which is what it means in
/// the genre.
///
/// See `apply_intent` in `movement/abilities.rs`.
pub const SMASH_FIGHTER_KIT: ambition_platformer2d::engine_core::AbilitySet =
    ambition_platformer2d::engine_core::AbilitySet {
        move_horizontal: true,
        jump: true,
        variable_jump: true,
        double_jump: true,
        fast_fall: true,
        attack: true,
        directional_primary: true,
        shield: true,
        // The capture verb. Granting it here does NOT invent a grab: the
        // action scheme wants `abilities.grab` AND an authored `"grab"` move, so
        // a fighter joins the mechanic on the day its table does and the other
        // thirteen are unchanged until theirs do.
        grab: true,
        dodge: true,
        ledge_grab: true,
        ..ambition_platformer2d::engine_core::AbilitySet::NONE
    };

/// THE CEILING — the floor above, PLUS the verbs a fighter may bring from home.
///
/// ⭐⭐ THE DIFFERENCE BETWEEN THESE TWO CONSTANTS IS CHARACTER IDENTITY. Jon,
/// W8 playtest: *"`robot_v3` should have Pogo available in Smash. **Do not make
/// Pogo a universal Smash action.** Robot v3 has Pogo because Robot v3 owns that
/// capability. Another fighter without Pogo should not acquire one merely by
/// entering Smash."*
///
/// ⛔⛔ AND POGO USED TO SIT IN THE FLOOR, so every one of the fourteen bodies
/// on this grid got a rebounding down-air by walking onto the stage. It read as
/// working — the fighter Jon tested is the one that authors it — and the defect
/// was in the thirteen it also reached.
///
/// ⭐ `MatchAbilities::levelled` is a floor and a ceiling at once, and its own
/// doc named this day: *"the day a stage wants a fighter's own flavour to
/// survive, it widens `permitted` past `granted` rather than reaching for a
/// third operator."* This is that widening, and one verb wide is the honest size
/// of it — `fly`, `blink` and `dash` stay out of BOTH, because those are the
/// exploration kit and the reason a ceiling exists at all.
pub const SMASH_FIGHTER_CEILING: ambition_platformer2d::engine_core::AbilitySet =
    ambition_platformer2d::engine_core::AbilitySet {
        pogo: true,
        ..SMASH_FIGHTER_KIT
    };

/// The same roster, at a named ladder level.
///
/// Exists for the ladder probe: the archetype authors one level, and measuring
/// whether L3 buys anything needs the SAME match at two of them. The brain
/// profile is a per-seat fact, so this is a per-seat override rather than a
/// second archetype — a second archetype would also vary its speed, reach and
/// health, and then the measurement would be about the archetype.
pub fn smash_roster_at_level<I, S>(characters: I, level: u8) -> MatchParticipantRoster
where
    I: IntoIterator<Item = S>,
    S: Into<ambition_platformer2d::entity_catalog::CharacterId>,
{
    let mut roster = smash_roster(characters);
    for participant in &mut roster.participants {
        if let ControllerBinding::Cpu { brain_profile } = &mut participant.controller {
            *brain_profile = Some(format!("{SMASH_DUELIST_BRAIN}_l{level}"));
        }
    }
    roster
}

/// The brain preset that DOES NOTHING, by name.
///
/// A stand-still seat is not a broken seat: the body is staged, damageable and
/// physical like any other, and its policy is to make no decisions. Naming the
/// preset here is what lets an inspection scenario ask for one without inventing
/// a way to freeze a fighter.
pub const SMASH_IDLE_BRAIN: &str = "stand_still";

/// The same roster, with every seat after the first STANDING STILL.
///
/// ⭐⭐ THE TRAINING-MODE TARGET, BUILT FROM MATCH POLICY. Inspecting a move
/// against a live CPU means measuring two decisions at once: the opponent walks
/// into a strike, or away from it, and the recording of the move changes because
/// of something the move did not do. A passive target removes that variable
/// without removing the target — contact rules, hurtboxes, hitstun and launch
/// all still run, because this is an ordinary seated fighter whose brain
/// declines to act.
///
/// ⛔ IT IS A SEAT WITH A DRIVER, NOT A SEAT WITHOUT ONE. `ControllerBinding::Cpu
/// { brain_profile: None }` is refused at preparation on purpose — "a seat with
/// no driver stands still, which is indistinguishable from a brain that failed
/// to install". This asks for the policy that stands still BY NAME, so the
/// distinction survives into the artifact.
pub fn smash_roster_with_passive_targets<I, S>(characters: I) -> MatchParticipantRoster
where
    I: IntoIterator<Item = S>,
    S: Into<ambition_platformer2d::entity_catalog::CharacterId>,
{
    let mut roster = smash_roster(characters);
    for participant in &mut roster.participants {
        if let ControllerBinding::Cpu { brain_profile } = &mut participant.controller {
            *brain_profile = Some(SMASH_IDLE_BRAIN.to_string());
        }
    }
    roster
}

/// Where a knocked-out fighter comes back.
///
/// The engine spends the stock and clears the meter; it refuses to place the
/// body, because placing it needs a stage. This is that answer.
/// Two CPU fighters at DIFFERENT levels — the ladder's own roster.
///
/// [`smash_roster_at_level`] puts every CPU seat on one rung, which is what a
/// probe wants (*"how does level N behave"*) and not what a LADDER wants
/// (*"does level N beat level N−1"*). And [`smash_roster`] makes seat 0 HUMAN,
/// so the only opponent a probe could offer was a controller-less body that
/// never acts — every stock lost was a self-KO, which made the number clean and
/// made it impossible to measure a fight.
///
/// `opens_suspended` and the stock count are inherited deliberately. A rig
/// that quietly ran a different ruleset from the shipped stage would measure a
/// game nobody plays; the ONLY difference from a real match is who is holding
/// the controllers.
pub fn smash_roster_at_levels<I, S>(characters: I, levels: &[u8]) -> MatchParticipantRoster
where
    I: IntoIterator<Item = S>,
    S: Into<ambition_platformer2d::entity_catalog::CharacterId>,
{
    let mut roster = smash_roster(characters);
    for (index, participant) in roster.participants.iter_mut().enumerate() {
        // Every seat is a CPU here, including seat 0 — which `smash_roster` made
        // human, because a human seat is what a player expects to occupy.
        let level = levels.get(index).copied().unwrap_or(1);
        participant.controller = ControllerBinding::Cpu {
            brain_profile: Some(format!("{SMASH_DUELIST_BRAIN}_l{level}")),
        };
    }
    // AND IT SAYS WHOSE ROSTER IT IS. `smash_roster` above ends with the
    // same call and this one silently did not — which cost nothing while a CPU
    // seat's `brain_profile` could still be an ARCHETYPE key, because an
    // archetype table is global. It costs everything now that a published POLICY
    // is the only thing a seat can name (P2.18):
    // `seat_brain_profile` resolves a provider-relative name in the MATCH's
    // provider, an unpublished roster has none, and every levelled seat this
    // helper builds was refused with *"`duelist_l1` … Known keys: [combatant]"*.
    roster.published_by(SMASH_EXPERIENCE)
}

/// Horizontal spread between adjacent respawn points, in stage pixels.
///
/// Two 32px tiles — wider than a standing body, so two fighters returning on the
/// same frame land clear of each other rather than inside one another. derived
/// against [`PLATFORM_WIDTH`]: seat `n` sits at most `(n/2 + 0.5)` spacings from
/// the centre, so even eight seats stay within ±224px of a 480px platform.
const RESPAWN_SEAT_SPACING_PX: f32 = 64.0;

/// Where a fighter comes back, and it is not where its opponent comes back.
///
/// seats alternate outward from the centre — 0 left, 1 right, 2 further
/// left, 3 further right — so the arrangement is symmetric at any roster size
/// and no seat is privileged. An offset that simply grew with the index would
/// push seat 3 twice as far out as seat 1 for no reason a player could read.
pub fn respawn_placement(stage_centre: Vec2, seat: usize) -> Vec2 {
    // 0,1 → half a spacing out; 2,3 → one and a half; and so on.
    let rank = (seat / 2) as f32 + 0.5;
    let side = if seat % 2 == 0 { -1.0 } else { 1.0 };
    Vec2::new(
        stage_centre.x + side * rank * RESPAWN_SEAT_SPACING_PX,
        // Toward the sky. The stage's own down is the gravity the room authored;
        // this demo is screen-down like every other platform fighter, and a
        // gravity-flipped stocks stage is a thing the ENGINE would have to
        // answer rather than this crate.
        stage_centre.y - RESPAWN_HEIGHT_PX,
    )
}

/// Stable room id for the stage.
pub const SMASH_STAGE_ROOM_ID: &str = "smash_stage";

/// The authored room around the fighting platform.
///
/// Keep this at 640x480 for the demo's presentation frame. The fighting stage
/// itself is [`PLATFORM_WIDTH`] wide; the room bounds are only an intermediate
/// seam between the platform and the blast envelope.
const STAGE_SIZE: Vec2 = Vec2::new(640.0, 480.0);
const PLATFORM_TOP: f32 = 300.0;

/// Fifteen 32px tiles, or ten standing-body heights.
///
/// Final Destination's main platform is roughly ten Mario-height units wide.
/// Ambition's default standing body is 48px tall, so 480px gives the demo the
/// same useful fighter-to-stage scale while staying exactly on the 32px floor
/// texture grid.
const PLATFORM_WIDTH: f32 = 480.0;

/// Blast margins chosen so the PLATFORM, not the room rectangle, has Final
/// Destination-like normalized proportions:
///
/// * ledge -> side blast line = 1.000 platform widths;
/// * platform surface -> ceiling blast line = 1.125 platform widths;
/// * platform surface -> fall blast line = 0.875 platform widths.
///
/// With a 480px platform centered in a 640px room there are 80px from either
/// ledge to the room edge, leaving 400px beyond the room edge for the side blast
/// margin. The platform surface is y=300 in a 480px room, so a 240px vertical
/// margin puts the ceiling 540px above the platform and the fall line 420px
/// below it. The complete blast envelope is therefore 1440x960: exactly 3x2
/// platform widths, matching Final Destination's normalized envelope.
const FALL_BLAST_MARGIN_PX: f32 = 240.0;
const SIDE_BLAST_MARGIN_PX: f32 = 400.0;
const CEILING_BLAST_MARGIN_PX: f32 = 240.0;

/// The stage: a platform surrounded by nothing.
///
/// That shape is the whole difference from every other room the engine has
/// loaded. A platformer room is a box you cannot leave; a fighter stage is a
/// thing you can be knocked OFF, and the emptiness around it is the mechanic
/// rather than the absence of one.
///
/// Authored in Rust rather than LDtk deliberately, and the repo's own rule says
/// which way that goes: LDtk is preferred for content, Rust rooms are for DEMOS.
/// A stage this shape is four numbers, and putting it in a level file would make
/// the ONE interesting fact about it — the blast margin — a field in an editor
/// nobody opens.
pub fn smash_stage() -> RoomSpec {
    let mut world = ae::World::new(
        "Smash Stage",
        STAGE_SIZE,
        // Spawn above the platform, like a respawn: the fighters are placed by
        // seating, and this is only where a lone visitor lands.
        Vec2::new(STAGE_SIZE.x / 2.0, PLATFORM_TOP - 96.0),
        vec![ae::Block::solid(
            "smash_platform",
            Vec2::new((STAGE_SIZE.x - PLATFORM_WIDTH) / 2.0, PLATFORM_TOP),
            Vec2::new(PLATFORM_WIDTH, 32.0),
        )],
    );
    world.edges.fall = FALL_BLAST_MARGIN_PX;
    // The SIDES are the interesting ones and they are not the default. A body
    // launched horizontally leaves through them, and without an explicit value
    // they inherit a margin sized for "fell through the floor" — generous enough
    // that a fighter knocked off the edge would drift for a second and a half
    // before anything noticed.
    world.edges.side = Some(SIDE_BLAST_MARGIN_PX);
    world.edges.rise = Some(CEILING_BLAST_MARGIN_PX);

    let mut room = RoomSpec::new(SMASH_STAGE_ROOM_ID, world);
    room.metadata.mode = Some(SMASH_MODE.to_string());
    // ⭐⭐ EVERY FIGHTER IS LABELLED THE SAME WAY. The presentation default hides
    // the plate over a body somebody is driving, which is the right EXPLORATION
    // rule — a plate names a body you are not inhabiting — and reads as
    // "everyone is labelled except the human" on a stage with a cast. Jon,
    // 2026-08-24: *"This is player 1 centric behavior, and we should have none
    // of it."*
    //
    // ⛔ THE STAGE DECIDES, NOT THE RENDERER. A four-way match wants to know who
    // is who; `Some(false)` is the other uniform answer if plates ever get in
    // the way of reading the fight.
    room.metadata.nameplate_policy.label_driven_bodies = Some(true);
    room
}

pub fn stage_centre() -> Vec2 {
    Vec2::new(STAGE_SIZE.x / 2.0, PLATFORM_TOP)
}

/// What the match announces when it ends.
///
/// It read `seat 2 wins` before — which is what he was looking at when he asked — and the SIDE is
/// not a name. `announce_the_winner` resolves the winning side into the fighter's own name before
/// it gets here; this owns the wording alone, so the card and any test of it read one function.
pub fn victory_banner(
    outcome: &ambition_platformer2d::actor::MatchVerdict,
    winner_name: Option<&str>,
) -> String {
    use ambition_platformer2d::actor::MatchVerdict;
    match outcome {
        // The NAME, resolved by the caller — a side label is not a name, which
        // is what Jon was looking at when he asked about `seat 2 wins`.
        MatchVerdict::Winner(side) => format!("WINNER: {}", winner_name.unwrap_or(side)),
        // A draw is reachable and cheaply: two fighters on their last stock,
        // knocked off together.
        MatchVerdict::Draw => "Draw — everybody fell".to_string(),
        // ⭐ AND IT SAYS SO. The card is the only place a player learns which of
        // the three happened, and an abandoned match wearing "Draw" would tell
        // them the fighters settled something.
        MatchVerdict::NoContest => "NO CONTEST".to_string(),
    }
}

/// The two answers the engine refuses to guess, wired to the messages it
/// writes.
///
/// `ambition_platformer2d::combat::stocks` spends the stock, clears the meter and marks the
/// elimination — then stops, because placing a body needs a stage and announcing
/// a winner needs a scoreboard. This plugin is the other side of that seam, and
/// it is the whole reason the split is a design rather than an omission.
pub struct SmashRulesPlugin {
    hosted: bool,
}

impl SmashRulesPlugin {
    /// Ambition hosts this demo alongside its own rooms: the rules sleep
    /// outside the smash stage.
    pub fn hosted() -> Self {
        Self { hosted: true }
    }

    /// The demo IS the game.
    pub fn global() -> Self {
        Self { hosted: false }
    }
}

impl bevy::prelude::Plugin for SmashRulesPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::IntoScheduleConfigs;

        // The plugin owns its channels. A full app registers these through the
        // engine plugins; a rules-only harness may not, and `add_message` is
        // idempotent.
        app.add_message::<ambition_platformer2d::actor::FighterStockSpent>();
        app.add_message::<ambition_platformer2d::actor::FighterRespawnDue>();
        app.add_message::<ambition_platformer2d::actor::StocksMatchDecided>();
        // D192 — THIS STAGE AUTHORS THE BEAT. The engine defaults to zero, which
        // is the same-tick placement every other ruleset already had; the beat is
        // a smash-stage decision, so it is declared here and nowhere else.
        app.insert_resource(ambition_platformer2d::actor::RespawnInterval {
            seconds: RESPAWN_INTERVAL_SECONDS,
        });
        // The stop-this-match channel, owned here for the same reason the two
        // above are: a rules-only harness may not have the engine plugins.
        // The capture request channels. The ADAPTER below writes them and the
        // body runtime reads them, so this plugin owns them the same way it owns
        // the two above.
        app.add_message::<ambition_platformer2d::combat::capture::CaptureAttemptRequested>();
        app.add_message::<ambition_platformer2d::combat::capture::CapturePummelRequested>();
        app.add_message::<ambition_platformer2d::combat::capture::CaptureThrowRequested>();

        let sim = ambition_platformer2d::platformer::schedule::SimScheduleExt::sim_schedule(app);
        // THE CAPTURE LOOP, in the order the facts become available.
        //
        // `dispatch_move_events` turns a live grab window into an authored
        // `Effect` during `CombatSet::Playback`; the adapter recognises the key
        // and writes a typed request; acquisition turns that into a relationship.
        // Chained so a grab that goes active this tick catches this tick — the
        // alternative is a frame of latency on every grab, which in a fighting
        // game is a mechanic change rather than a rounding error.
        //
        // `Materialize`, beside the projectile spawns, because that set's
        // own doc says what it is for: *"a thing must EXIST before it can hit
        // anything"*. A capture relationship is exactly such a thing — the
        // pummel and throw that target it are moves that come later.
        app.add_systems(
            sim,
            (
                crate::capture::translate_smash_capture_effects,
                ambition_platformer2d::combat::capture::systems::acquire_captures,
                // and posed the SAME tick it is caught. The pose sync also
                // runs in `WorldPrep`, which is EARLIER in the tick than this —
                // so without this second call a body grabbed now would hang where
                // it stood until the next frame, one visible frame of a captive
                // standing free inside somebody's grab animation.
                // The pummel lands BEFORE the pose sync below, so the damage and
                // the frame the captive is drawn in belong to the same tick.
                ambition_platformer2d::combat::capture::systems::apply_capture_pummels,
                // The throw releases and launches in one step. AFTER the pummel
                // so a tick carrying both resolves in authored order, and BEFORE
                // the pose sync so a thrown body is not snapped back into a hold
                // it has just left.
                ambition_platformer2d::combat::capture::systems::apply_capture_throws,
                ambition_platformer2d::combat::capture::systems::finalize_new_capture_pose,
                // the captive's POSE, published beside the constraint that
                // holds it. `CharacterAnim` has no held row, so this draws the
                // hurt one — a body in somebody's hands reading as idle was the
                // last thing about a grab that did not look like one.
                ambition_platformer2d::combat::capture::systems::mirror_capture_into_anim_facts,
            )
                .chain()
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Materialize),
        );
        // THE PIRATE'S SHARK. ⭐ `ContentSpecials`, which is the seam the runtime
        // already provides for exactly this: a CONTENT TECHNIQUE that must
        // produce its effects before the effect executors run
        // (`ContentSpecials.before(EffectExecutionSet)`, and `apply_effects` is
        // chained before `apply_summon_effects`).
        //
        // ⛔ NOT `Materialize` WITH A LEAF-TO-LEAF EDGE. The first version put
        // this beside the capture adapter and ordered nothing, so the writer of
        // an `EffectRequest` and its executor were unordered peers in one set —
        // a scheduler tie deciding whether a summon lands this tick or next,
        // which is the shape this repo has already been bitten by. Naming the
        // set says WHAT this system is instead of who it must beat.
        // Jostle is a fact the movement kernel reads, so it is established in the
        // simulation — see the system's own note. `WorldPrep` because it must be
        // true before anything integrates a body.
        app.add_systems(
            sim,
            smash_fighters_are_solid_to_each_other.in_set(
                ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep,
            ),
        );
        app.add_systems(
            sim,
            crate::shark_ride::translate_shark_summons
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials)
                // ⛔⛔ AND THE EXPLICIT EDGE, WHICH THE SET DOES NOT IMPLY.
                // `ContentSpecials.before(EffectExecutionSet)` orders this ahead
                // of `apply_effects`, and `apply_summon_effects` is CHAINED
                // after that system without being IN the set — so the summon
                // executor inherited no order from the phase at all. Measured,
                // not reasoned: with the set alone the executor ran every tick
                // and read zero requests, and the shark never appeared.
                .before(ambition_platformer2d::actors::features::apply_summon_effects),
        );
        // THE BOMB. Recognised where the shark's summon is, for the same reason
        // — both are authored techniques dispatched as `ActorActionMessage` —
        // and burnt in `Settle`, after the item physics has had its say about
        // whether the object hit anything this tick.
        app.add_systems(
            sim,
            crate::bomb::translate_bomb_drops
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials),
        );
        app.add_systems(
            sim,
            crate::bomb::burn_fuses_and_answer_impacts
                .after(ambition_platformer2d::actors::items::pickup::ItemPickupSet::CoreHeldItems)
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle),
        );
        // WHAT ENDS A RIDE, and what the shark does afterwards.
        //
        // ⛔ `Settle` and CHAINED, because all three are opinions about a state
        // the tick has already produced: whether the rider is tumbling, whether
        // it pressed jump, and whether its saddle emptied. The two that ASK run
        // before `apply_dismount_requests` (which the runtime schedules in the
        // same set) and the departure reads the announcement that one makes, so
        // a shark left riderless this tick is already leaving this tick.
        app.add_systems(
            sim,
            (
                crate::shark_ride::dissolve_the_ride_when_the_shark_dies,
                crate::shark_ride::dismount_launched_riders,
                crate::shark_ride::dismount_riders_who_left_play,
                crate::shark_ride::bail_out_of_the_saddle,
            )
                .chain()
                .before(ambition_platformer2d::mount::apply_dismount_requests)
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle),
        );
        app.add_systems(
            sim,
            (
                crate::shark_ride::depart_when_riderless,
                crate::shark_ride::send_away_a_shark_nobody_boarded,
            )
                .chain()
                .after(ambition_platformer2d::mount::apply_dismount_requests)
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle),
        );
        // ⛔⛔ THE DEPARTURE'S INTENT IS WRITTEN WHERE INTEGRATION WILL READ IT.
        // It used to run in `CombatSet::Settle`, at the end of the tick, and a
        // departing shark has no rider — so on the NEXT tick its own brain
        // republished `ActorControl` before the movement pass, overwriting the
        // heading this system had set. The shark was told to leave once per tick
        // and talked out of it once per tick.
        //
        // ⭐ `BeforeIntegrate` IS THE PHASE THAT MEANS "the intent integration
        // is about to consume". Writing a velocity target anywhere the brain
        // still speaks after you is writing it into a value somebody else owns.
        app.add_systems(
            sim,
            crate::shark_ride::tick_departures
                .in_set(ambition_platformer2d::platformer::schedule::WorldPrepSet::BeforeIntegrate),
        );
        // THE FOOTSTOOL CLAIMS THE PRESS BEFORE THE KERNEL SPENDS IT, so
        // it runs in `PlayerInput` and NOT in `Settle`. It shipped in `Settle`
        // on the argument that a later velocity write wins; that is true of the
        // velocity and false of the air jump, which the kernel had already spent
        // by then. A body with a charge paid one and a body without paid nothing
        // for the identical footstool. The claim now reaches
        // `BodyJumpState::footstool_claimed` ahead of the jump chain.
        app.add_systems(
            sim,
            ambition_platformer2d::combat::footstool::claim_footstools.in_set(
                ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith::PlayerInput,
            ),
        );
        // THE LEDGE TRUMP RESOLVES AFTER THE KERNEL, so it sees the grabs
        // this tick made. Two bodies can catch one edge on the same frame, and
        // arbitrating before `PlayerSimulation` would judge LAST tick's
        // occupancy and leave both hanging for a frame — which is the frame an
        // edge-guard reads. `CombatSet::Settle` is not a claim that a trump is
        // combat; it is the established post-kernel bookkeeping slot, beside the
        // capture release and the stale-move recorder.
        app.add_systems(
            sim,
            ambition_platformer2d::combat::ledge_trump::resolve_ledge_trumps
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle),
        );
        // A capture ends in `Settle`, where post-damage bookkeeping belongs.
        // Hitstun and the recoil lock are written by damage resolution in
        // `Resolve`, so a release that ran earlier would read last tick's answer
        // and let a grab survive by one frame the hit that should have broken it.
        app.add_systems(
            sim,
            ambition_platformer2d::combat::capture::systems::release_interrupted_captures
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle),
        );
        // AFTER the engine's own `CombatSet::Settle` work: the stock is spent
        // there, and placing a body before it has been spent would put the
        // fighter back on the stage for a knockout that had not been counted.
        // The HUD publisher is PRESENTATION, not a rule: it reads seats and
        // publishes readouts and decides nothing. It runs in the same gated set
        // so a hosted build stops drawing a fighter HUD the moment the stage is
        // not the active mode.
        let rules = (
            publish_smash_hud,
            announce_the_opening_countdown,
            place_respawning_fighters,
            ambition_platformer2d::actor::tick_respawn_grace,
            a_swing_spends_the_respawn_protection,
            hold_the_respawn_platforms,
            announce_the_winner,
        )
            .chain()
            .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle)
            .after(ambition_platformer2d::combat::stocks::FighterStocksSpent)
            // ⛔ AND after the RETURN is decided. Ordering only against the spend
            // was enough while placement happened on the knockout tick; with an
            // interval the two are different ticks, and a placement racing the
            // tick-down reads an empty queue on the tick a fighter was due.
            .after(ambition_platformer2d::combat::stocks::FighterRespawnsDue);
        // ⛔⛔ SUDDEN DEATH'S STAGE HALF IS A SIMULATION RULE, and it ran in
        // literal `Update` until a review caught it. It writes rollback-canonical
        // `BodyHealth` — putting every survivor on the authored damage — and a
        // rewind can execute several simulation steps without ordinary `Update`
        // replaying between them, so the resimulated match would reach sudden
        // death and never place its fighters.
        //
        // ⭐ ORDERED AFTER THE DECISION rather than merely after the stock spend:
        // the message it reads is written by `decide_stocks_match`, and the
        // `rules` chain above only promises to follow `FighterStocksSpent`.
        app.add_systems(
            sim,
            open_the_sudden_death_round
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle)
                .after(ambition_platformer2d::combat::stocks::MatchOutcomeDecided),
        );
        // THE ONE RULE THAT CANNOT RUN ALONGSIDE THE DECISION, pulled out
        // of the chain above and ordered behind it.
        //
        // Reported from the couch: *"there seems like several cases
        // where everyone but one player dying will not cause a match to end
        // correctly."* This system DESPAWNS an eliminated body, and
        // `decide_stocks_match` reads the sides off the bodies that still exist —
        // so despawning the last loser deletes its side from the question, and
        // `last_side_standing` sees ONE side, and one side is not a match. It
        // answers `None`, forever, and the match never ends.
        //
        // Both systems sat in `CombatSet::Settle` with nothing ordering them, and
        // the chain above inserts an `ApplyDeferred` between its members, so the
        // despawn lands part-way through the set. Whether a match ended depended
        // on how the scheduler broke a tie — which is why it was *"several
        // cases"* rather than always.
        //
        // only this one waits. The HUD, the countdown and the respawn
        // placement are still meant to run beside the engine's answer rather than
        // behind it — see `FighterStocksSpent`'s own note — and putting the whole
        // chain behind the decision would take that away to fix one member.
        let remove_the_eliminated = take_eliminated_fighters_out_of_play
            .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle)
            .after(ambition_platformer2d::combat::stocks::MatchOutcomeDecided);
        if self.hosted {
            let gate = ambition_platformer2d::runtime::in_mode(SMASH_MODE);
            // ⭐ THE RETRACTION IS AN OBSERVER, and it is UNGATED on purpose: it
            // fires when `RespawnGrace` leaves for ANY reason — its own clock, a
            // swing, a body being rebuilt, a mode teardown — and a reason bit
            // left set by a component that is gone is a fighter invulnerable for
            // the rest of the session.
            app.add_observer(ambition_platformer2d::actor::retract_respawn_grace_on_removal);
            app.add_systems(sim, rules.run_if(gate.clone()));
            app.add_systems(sim, remove_the_eliminated.run_if(gate));
        } else {
            app.add_systems(sim, rules);
            app.add_systems(sim, remove_the_eliminated);
        }
    }
}

/// The stage's own readouts: one per fighter, plus the match card.
///
/// the data was already shaped for these readouts and nothing consumed it:
/// `BodyHealth::damage_percent()` is deliberately UNCLAMPED with a test named
/// `damage_percent_is_unclamped_so_a_hud_can_print_188`, and `FighterStocks`
/// keeps `started_with` with the comment *"so a HUD can draw '2 of 3' rather
/// than inferring a maximum it was never told"*. Two APIs built for a consumer
/// that did not exist.
pub const FIGHTER_HUD_SLOTS: [&str; 4] = [
    "smash_fighter_0",
    "smash_fighter_1",
    "smash_fighter_2",
    "smash_fighter_3",
];
/// The winner card. One slot, because the stage says one thing at a time.
/// ⚠⚠ TEMPORARY: how much faster than authored the opening ceremony runs.
///
/// Jon, 2026-08-26, asking for a dev mode: *"make the 3, 2, 1, countdown go 10x
/// as fast."* `1` restores the authored three seconds, and that is the whole of
/// the revert.
///
/// ⛔ IT DIVIDES THE TICKS, NOT THE BEATS. `MatchRules::beats()` still counts
/// three numbers — the ceremony says the same thing, it just says it quicker —
/// and every test that waits out the countdown reads the roster's value rather
/// than a literal, so they follow this without knowing about it.
const COUNTDOWN_SPEEDUP: u32 = 10;

pub const SMASH_ANNOUNCE_HUD_SLOT: &str = "smash_announce";

/// What one remaining stock is drawn as, under the sprites asset root.
///
/// generated, not committed — `scripts/regen/sprites.sh` names it in its publish roster, which is what
/// lets a fresh clone produce it.
pub const STOCK_ICON_ASSET: &str = "sprites/hud_stock_icon.png";

/// What plays on the stage.
pub const SMASH_STAGE_TRACK: &str = "super_smash_siblings_theme";
/// What plays over the character select, in a host whose frontend audio
/// this demo owns. See `SMASH_TRACKS` for why it is registered either way.
pub const SMASH_SELECT_TRACK: &str = "super_smash_siblings_character_select";

/// The scores written for this demo, rendered from
/// `tools/ambition_music_renderer/scores/active/super_smash_siblings_*.music.yaml`.
///
/// all three are registered, not only the one that plays. A track in this
/// fragment is a track this experience is ALLOWED to play — the radio, a future
/// stage select, and the winner card all pick from it — so registering only the
/// default would make the other two unreachable from inside a smash session
/// even though they were written for it. The default is what plays with nobody
/// asking.
///
/// the asset path is derived (`audio/music/generated/<id>/full.ogg`) rather
/// than written out, because that layout is the renderer's own contract and
/// three hand-typed copies of it is three chances to typo one.
pub const SMASH_TRACKS: &[(&str, &str)] = &[
    (SMASH_STAGE_TRACK, "Super Smash Siblings"),
    (SMASH_SELECT_TRACK, "Choose Your Fighter"),
    (
        "super_smash_siblings_grand_symphony",
        "Super Smash Siblings — Grand Symphony",
    ),
];

/// Publish percent and stocks for every seated fighter.
///
/// percent is NOT health and the gauge fill says so: it fills as damage
/// ACCUMULATES, and the number keeps counting past 100% because a platform
/// fighter's does. Clamping the fill is a rendering decision; clamping the
/// number would be a lie about the game.

/// THE COMBAT RULES THIS STAGE DECLARES, in one place so the publisher and
/// its guard cannot hold different copies.
///
/// Every kit-less fighter reached the stage unable to hit anybody. and the guard could not catch
/// it, because it passed the swipe in BY HAND: *"a fixture that manufactures the value under test
/// cannot fail on its absence."* Both now call this.
///
/// reading the resource would be wrong even when it exists: on a second
/// visit it holds the PREVIOUS match's declaration. A function has no such tense.
pub fn smash_declared_combat_rules() -> ambition_platformer2d::combat::rules::DeclaredCombatRules {
    ambition_platformer2d::combat::rules::DeclaredCombatRules {
        // BY OWNER. The versus route declares combat rules too, and a
        // giveback that removed this by TYPE would delete ITS live rules the
        // moment smash left — the lesson the roster and the prepared match each
        // taught once already.
        declared_by: SMASH_EXPERIENCE.to_string(),
        di_max_angle: SMASH_DI_MAX_ANGLE,
        knockback_growth: SMASH_KNOCKBACK_GROWTH,
        // The robot's down-air is ONE authored swing that says it can rebound its attacker;
        // Ambition takes it up on that, and a platform fighter must not — a d-air that bounced you
        // back to safety offstage would be the opposite of a kill. Same move, two games, and the
        // difference is declared rather than authored twice.
        downward_hit: ambition_platformer2d::combat::rules::DownwardHitStyle::Spike,
        // and the spike is a SENTENCE, not just a shove. ~18 frames in
        // which a body knocked down out of the air cannot recover — long enough
        // that a spike offstage is a kill and short enough that one over the
        // stage is survivable. The window ENDING is what the genre calls the
        // meteor cancel; there is no second verb.
        meteor_lock_time: 0.30,
        // RAGE, capped at 1.4x. The percent mechanic already makes a hurt
        // fighter easier to launch; without this it is punished twice, and the
        // last stock stops being a fight. The cap is what keeps a comeback a
        // chance rather than a coin flip.
        rage_per_damage: 0.004,
        rage_max_scale: 1.4,
        // STALING, floored at 0.55. One reliable kill move should not be
        // the only answer a fighter needs; nine landings of it and it is worth
        // barely half. Vary and the old one recovers — the ring forgets.
        stale_step: 0.05,
        stale_floor: 0.55,
        // CROUCH CANCEL, 0.85x. Ducking is a defensive read, not just a
        // shorter hurtbox — and the 15% is what makes it one at low percent
        // without saving anybody from a kill move.
        crouch_cancel_scale: 0.85,
        // NO BLANKET MERCY WINDOW. This is the genre's answer, not a
        // preference: Smash has no post-hit invulnerability, and repeat
        // protection is a move's own business — one hitbox may not hit the same
        // body twice, and SEPARATED authored Active windows are meant to, which
        // is what a multi-hit move IS.
        //
        // ⛔ the engine's blanket window is 0.2s on the actor road, and George
        // Booul's `bivalence` authors its weak pop at 0.30s and its launcher at
        // 0.42s. A 0.2s window outlives that 0.12s gap, so the launcher could
        // never land on the body the pop had hit: the move's whole design — "an
        // early weak pop and a late strong throw" — was unreachable, and with
        // both fighters George Booul a thirty-second match produced 135% of
        // damage and not one launch.
        hit_repeat_window_scale: 0.0,
        // TWO ATTACKS MEETING TRADE. Before this, two fighters swinging into
        // each other both connected — both took damage, both were launched —
        // which is an interaction no game in this genre has. Nine damage is the
        // genre's neighbourhood: Melee, Brawl, Smash 4 and Ultimate all compare
        // the two attacks' damage and all four use a threshold about here.
        // Closer than this and both are refused; further and the stronger one
        // continues untouched, which is what makes a heavy swing beat a jab
        // instead of trading with it.
        // ⛔⛔ CLANKING IS DECLARED OFF, and the mechanism is finished and tested
        // — this is a TUNING decision, not a gap. Turned on at 9 damage it
        // reshaped the whole ground game: two CPU fighters traded so constantly
        // that `every_live_fighter_stays_inside_the_frame` measured ZERO
        // body-frames outside the stage in a full match (nobody was ever
        // launched) and `the_cpu_charges_a_smash_and_techs_a_landing_in_some_match`
        // stopped finding its beat. Restricting it to the ground game — the
        // genre's own rule, aerials do not clank — did not settle it either.
        //
        // ⇒ a mechanic that becomes real re-tunes everything built on its
        // absence, and what this needs is a play session rather than another
        // guessed threshold. `9.0` is the genre's number and the one to try
        // first.
        // ⭐ RECOVERY ENDS AT THE LIP. Landing an aerial on the edge of a
        // platform and sliding off cancels its lag — the genre's reward for
        // spacing a landing on purpose rather than just landing.
        edge_cancel_recovery: Some(true),
        // ⭐ B-REVERSE: a special pressed BACKWARD turns the fighter around, so
        // a recovery or a projectile can come out the way you came from.
        special_turn: Some(true),
        // ⭐ AND THE WAVEBOUNCE with it: the turn takes your drift too. The two
        // are one rule with two settings, which is what stops either becoming a
        // per-fighter velocity hack. ⚠ both are feel calls: play them.
        special_turn_reverses_drift: Some(true),
        clank_damage_window: 0.0,
        // …and the trade throws both fighters back. Well under a launch — this
        // is a reset of the exchange, not a punish — but far enough that the two
        // are no longer inside each other's next swing, which is what makes
        // trading a decision rather than a stutter.
        clank_rebound_speed: 190.0,
        // SUDDEN DEATH at 150%. A timed match that ends genuinely level does not
        // end: both sides go to the edge of a launch and the next clean hit
        // decides it. The number is what makes "short" short — at 150 almost any
        // connect is a kill, which is the genre's whole point.
        sudden_death_damage: Some(150),
        // ⭐⭐ LOSING THE LEDGE COSTS SOMETHING. Jon, 2026-08-24: *"A character
        // can just stay on the ledge, and there is no way to knock them off."*
        // Stealing the edge now throws the previous holder off it rather than
        // dropping them on the spot, so a trump is a real edge-guard option and
        // not just a swap.
        //
        // ⛔ 260px/s is a SHOVE, not a kill: enough that the loser has to
        // recover, short of sending them to the blast zone from a neutral trump.
        // ⚠ a starting point — play it and move it.
        ledge_trump_pop: Some(260.0),
        // ⭐ ULTIMATE'S RULE: a recovering fighter can steal the edge back, so
        // covering a ledge is a read rather than a denial. `Hog` is the other
        // generation's answer and it is one word away.
        ledge_occupancy: Some(ambition_platformer2d::combat::rules::LedgeOccupancy::Trump),
        // ⭐ THE DOUBLE-JUMP CANCEL: an aerial thrown out of an air jump kills
        // the rest of that jump's rise, so a double jump is an approach rather
        // than a commitment. ⚠ a feel call: play it.
        double_jump_cancel: Some(true),
        // ⭐⭐ ONE HIT IN SIX SPEAKS. Jon, 2026-08-24: *"not have barks happen
        // every time a character is hit. Make it a more rare event. Not never,
        // but I'd like it to happen less often."*
        //
        // ⛔ A RATE, NOT A COOLDOWN, and the difference is audible: a cooldown
        // makes the first hit of every exchange bark and the rest silent, which
        // a player learns as a rhythm. A rate stays unpredictable, which is what
        // "rare" sounds like. ⚠ a starting point, not a measured one — it is one
        // number, and the thing to do with it is play the match and move it.
        bark_chance: Some(1.0 / 6.0),
        // A GRAB HOLDS THE HURT FIGHTER LONGER, which is Ultimate's
        // 90 + 1.7p frames: 1.5s at 0%, ~4.3s at 100%. It makes the grab a
        // percent mechanic like the launch is, so the body that is losing is
        // the body a grab is worth spending your commitment on.
        //
        // the percent is read AT THE GRAB, so pummelling does not extend the
        // hold it earns you — a pummel is a decision, not a free extension.
        grab_hold_base_seconds: 90.0 / 60.0,
        grab_hold_per_damage: 1.7 / 60.0,
        // The captor's answer to the same question: however hurt the captive
        // is, a hold nobody ends still ends.
        grab_hold_max_seconds: 6.0,
        // 14.4 frames per press, Ultimate's rate, so mashing is the captive's
        // real option rather than a gesture at one.
        grab_mash_seconds: 14.4 / 60.0,
        // teams already decide who may hit whom. Switching global friendly
        // fire on to let two humans trade would make TEAMMATES hittable too.
        friendly_fire: false,
    }
}

/// THE KIT THIS EXPERIENCE HANDS A FIGHTER THAT AUTHORS NONE.
///
/// ⛔⛔ IT IS A ROSTER-PREPARATION POLICY, NOT A COMBAT RULE, and it lived on
/// `DeclaredCombatRules` for a while — which gave the engine's rules type a
/// second answer to *"what moves does this fighter have?"* beside the
/// character's own `MovesetContract`. Rules own DI, knockback growth, friendly
/// fire, grab timing, meteor lock and hitstop; they do not own a kit.
///
/// ⭐ THE ADAPTATION IS LEGITIMATE AND IT IS THIS LAYER'S. Most of Ambition's
/// cast authors `default_action_set: "peaceful"` on purpose — standing in a room
/// and talking is what they are for — and seating one in an arena means adapting
/// it into a platform fighter. `roster_seeded` folds this into the seat's
/// `ActionSet` at seating time, so by simulation time the body has ONE move
/// authority and nothing downstream consults a fallback.
///
/// The numbers are the exploration provoke's VERBATIM: 0.22 / 0.08 / 0.26, 4
/// damage, 34 reach. A stage's floor arguably wants to be faster, harder and
/// longer than a provoke's — moving the declaration is not the place to decide
/// that.
pub fn smash_seating_melee() -> ambition_platformer2d::character::MeleeActionSpec {
    ambition_platformer2d::character::MeleeActionSpec::Swipe(
        ambition_platformer2d::character::SwipeSpec {
            windup_s: 0.22,
            active_s: 0.08,
            damage: 4,
            reach_px: 34.0,
            recover_s: 0.26,
        },
    )
}

/// One fighter panel's face: the page, and which frame of it to draw.
#[derive(Clone, Debug)]
struct HudFace {
    image: String,
    /// `None` where nothing could crop it — see `HudStanding::portrait_frame`.
    frame: Option<bevy::prelude::Rect>,
}

/// Resolve a worn character's HUD face through the engine's portrait road.
///
/// A STILL, explicitly: this panel never ticks a frame. Asking for one is also
/// what keeps the page from being drawn whole — a portrait sheet holds every
/// clip the character can wear, and 56 pixels of eight-frame strip is nothing.
fn hud_face(
    catalog: &ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog,
    portraits: Option<&ambition_platformer2d::character::PortraitSheetRegistry>,
    declared: Option<&ambition_platformer2d::character::PreparedCharacterRegistry>,
    character_id: &str,
) -> Option<HudFace> {
    let target = declared
        .and_then(|registry| registry.get(character_id))
        .and_then(|prepared| prepared.portrait.as_deref());
    let reference = ambition_platformer2d::character::portrait_for_declared_character(
        portraits,
        catalog,
        target,
        character_id,
    )?;
    let frame = portraits
        .and_then(|registry| {
            registry.resolve_still(&reference.manifest, None, Some(&reference.still_clip))
        })
        .map(|(_, frame)| bevy::prelude::Rect::from(frame));
    Some(HudFace {
        image: reference.image,
        frame,
    })
}

pub fn publish_smash_hud(
    fighters: bevy::prelude::Query<(
        &ambition_platformer2d::actors::character_runtime::MatchSeat,
        &ambition_platformer2d::characters::actor::BodyHealth,
        Option<&ambition_platformer2d::actor::FighterStocks>,
        // ⭐ THE PUNCH READS THE FREEZE. `hitstop_timer` is non-zero exactly
        // when a hit has just landed and is already scaled by the damage
        // (`hitlag_duration`), so a HUD driven off it reads the SAME fact the
        // player felt and cannot disagree with it. ⛔ NOT a percent delta
        // tracked in presentation: that is a second answer to a question the sim
        // answers, and the two part company the frame a hit is blocked, absorbed
        // by armor, or lands for zero.
        Option<&ambition_platformer2d::characters::actor::BodyCombat>,
        &bevy::prelude::Name,
        // WHO this body is, which is what a PORTRAIT needs. The `Name` above
        // is a display string; a portrait is resolved from the character id.
        Option<&ambition_platformer2d::characters::actor::WornCharacter>,
    )>,
    // the GAME resolves the portrait, not the renderer. `HudFigure`'s
    // variants are presentation primitives and "which character" is content —
    // see the note on `HudStanding::portrait`. This is the side that knows.
    catalog: Option<
        bevy::prelude::Res<
            ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog,
        >,
    >,
    // A panel wants ONE FACE, so this asks the portrait manifests for a STILL.
    // Both are optional for the same reason the select screen's are: a
    // composition that installs neither still draws a portrait, just an
    // uncropped one.
    portraits: Option<bevy::prelude::Res<ambition_platformer2d::character::PortraitSheetRegistry>>,
    declared: Option<
        bevy::prelude::Res<ambition_platformer2d::character::PreparedCharacterRegistry>,
    >,
    mut readouts: bevy::prelude::ResMut<ambition_platformer2d::presentation::HudReadouts>,
) {
    let mut rows: Vec<(usize, String, f32, Option<(u32, u32)>, Option<HudFace>, f32)> = fighters
        .iter()
        .map(|(seat, health, stocks, combat, name, worn)| {
            let face = worn.zip(catalog.as_deref()).and_then(|(worn, catalog)| {
                hud_face(
                    catalog,
                    portraits.as_deref(),
                    declared.as_deref(),
                    worn.id(),
                )
            });
            (
                seat.0,
                name.as_str().to_string(),
                health.damage_percent(),
                stocks.map(|s| (s.remaining, s.started_with)),
                face,
                // Normalised against the longest freeze this feel tuning can
                // produce, so the strongest hit in the game is a full punch and
                // everything else is a share of it.
                combat.map_or(0.0, |combat| {
                    (combat.hitstop_timer / HUD_PUNCH_REFERENCE_HITLAG).clamp(0.0, 1.0)
                }),
            )
        })
        .collect();
    // Sorted by SEAT. Query order is not an order, and a scoreboard whose sides
    // swap mid-match is worse than none — the same reason the versus stage sorts.
    rows.sort_by_key(|(seat, ..)| *seat);

    let mut written = [false; FIGHTER_HUD_SLOTS.len()];
    for (seat, _name, percent, stocks, face, emphasis) in &rows {
        let Some(slot) = FIGHTER_HUD_SLOTS.get(*seat) else {
            continue;
        };
        written[*seat] = true;
        // Stocks are ICONS now and a fraction printed beside them would be the same fact said
        // twice.
        let value = format!("{:.0}%", percent * 100.0);
        let (remaining, started) = stocks.unwrap_or((0, 0));
        readouts.set(
            *slot,
            ambition_platformer2d::presentation::HudReadout::standing(
                // NO LABEL, and the first capture is why. `text()` joins
                // the label and the value, so passing the fighter's name here
                // drew "George Booul 0%" across a 132px panel — two panels'
                // worth of text colliding in the middle of the screen. The
                // PORTRAIT says who this is; the text says the one thing a
                // player reads mid-match.
                String::new(),
                value,
                ambition_platformer2d::presentation::HudStanding {
                    portrait: face.as_ref().map(|face| face.image.clone()),
                    portrait_frame: face.as_ref().and_then(|face| face.frame),
                    stock_icon: Some(STOCK_ICON_ASSET.to_string()),
                    remaining,
                    started,
                    emphasis: *emphasis,
                },
            ),
        );
    }
    // A 1v1 declares four slots and fills two. An unwritten slot must be
    // CLEARED, not left holding the previous match's fourth fighter.
    for (index, slot) in FIGHTER_HUD_SLOTS.iter().enumerate() {
        if !written[index] {
            readouts.clear_slot(*slot);
        }
    }
}

/// 3 — 2 — 1 — GO.
///
/// The roster opens `opens_suspended`, which stamps `ScriptedControl` on every
/// fighter in the same flush that creates them, and declares
/// `opening_countdown_ticks`. The ENGINE takes the hold off when the ceremony
/// ends (`release_the_opening_hold`), atomically, for every seat on one tick.
/// This system is the part a stage owns: saying the numbers out loud.
///
/// The tell was a diagram printing `travel: [0.0, 0.0]`.
///
/// DERIVED from the clock, so it cannot drift from the release. The
/// number on screen and the tick the bodies are freed are two readings of one
/// pure function of `now - activated_on`; a separate timer for the card would
/// be a second authority on when the round starts, and the two would disagree
/// on the frame anybody looked closely.
///
/// Same road as the fighter percents beside it, which are visibly drawn.
///
/// and the `Local` is gone with the banner. A readout is idempotent (a map
/// insert), so writing the same word every tick is free, while a banner message
/// re-requested every tick would never let the next card through — which is what
/// the state existed to prevent. The system is now a pure function of the clock
/// in fact as well as in prose.
fn announce_the_opening_countdown(
    active: Option<
        bevy::prelude::Res<ambition_platformer2d::actors::character_runtime::ActiveMatch>,
    >,
    prepared: Option<
        bevy::prelude::Res<ambition_platformer2d::actors::character_runtime::PreparedMatch>,
    >,
    tick: Option<bevy::prelude::Res<ambition_platformer2d::time::SimTick>>,
    settled: Option<
        bevy::prelude::Res<
            ambition_platformer2d::actors::features::stocks_match::StocksMatchSettled,
        >,
    >,
    // The sudden-death latch: see the stand-down below.
    sudden_death: Option<
        bevy::prelude::Res<
            ambition_platformer2d::actors::features::stocks_match::SuddenDeathEntered,
        >,
    >,
    mut readouts: bevy::prelude::ResMut<ambition_platformer2d::presentation::HudReadouts>,
) {
    use ambition_platformer2d::actors::character_runtime::OpeningPhase;
    let (Some(active), Some(prepared), Some(tick)) = (active, prepared, tick) else {
        return;
    };
    let Some(elapsed) = active.ticks_since_activation(tick.get()) else {
        return;
    };
    let rules = prepared.rules();
    if !rules.opens_suspended || rules.opening_countdown_ticks == 0 {
        return;
    }
    // THE CEREMONY STOPS TALKING THE MOMENT THE MATCH IS DECIDED.
    //
    // The card has exactly one owner at a time and the ORDER is the whole rule: the opening owns it
    // until there is an outcome, and then the outcome does, for as long as the results stand.
    if settled.is_some_and(|settled| settled.settled(&active)) {
        return;
    }
    // ⛔⛔ AND SUDDEN DEATH TAKES THE SLOT TOO, which the sentence above misses
    // because it names the wrong handover. "Until there is an outcome" reads
    // `StocksMatchSettled` — and sudden death deliberately leaves the match
    // UNSETTLED, because it is the match CONTINUING rather than a result. So
    // this system went on owning the card, cleared it on the very next tick, and
    // "SUDDEN DEATH" lasted about one simulation tick: unreadable.
    //
    // ⭐ THE LATCH IS THE AUTHORITY, NOT THE MESSAGE. `SuddenDeathBegan` fires
    // ONCE, so a card written from it cannot outlive a competing writer;
    // `SuddenDeathEntered` is the canonical, rollback-registered fact that the
    // round is on. The card then holds for the round, which is right — it is a
    // STATE the players are in, not a beat that passes.
    //
    // ⚠ REVIEWED, NOT PROVEN, and saying so is the point. The regression wants a
    // real match played to its time limit: `PreparedMatch` has private fields
    // and no constructor, so a unit fixture here can only build a system that
    // early-returns — a check that cannot fail. The follow-up is an integration
    // harness that runs a timed match to expiry.
    // ⛔⛔ AND THE BANNER IS DERIVED FROM THE LATCH, NOT WRITTEN BY THE SIM.
    // `open_the_sudden_death_round` used to set this slot itself, from inside the
    // rollback simulation — and `HudReadouts` is presentation, so it is not
    // rollback state and nothing retracts it. A rewind that unmade the timeout
    // took back the damage, the stocks and the message, and left "SUDDEN DEATH"
    // standing over a match that was no longer in it: a speculative simulation
    // result surviving as a fact on screen.
    //
    // ⭐ REPUBLISHED EVERY FRAME FROM ROLLBACK-REGISTERED STATE, which is the
    // shape presentation is supposed to have. `SuddenDeathEntered` IS rewound,
    // so the banner appears and disappears with the round it names and no
    // retraction has to be remembered anywhere.
    if sudden_death.is_some_and(|entered| entered.entered(&active)) {
        readouts.set(
            SMASH_ANNOUNCE_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::bare("SUDDEN DEATH".to_string()),
        );
        return;
    }
    let total = u64::from(rules.opening_countdown_ticks);
    // One beat, from the ruleset's own arithmetic rather than a second constant:
    // `opening_phase` divides the countdown by `opening_beats()` exactly this
    // way, so "GO!" holds for as long as each number did.
    let per_beat = total.div_ceil(u64::from(rules.opening_beats().max(1)));
    let word = match rules.opening_phase(elapsed) {
        OpeningPhase::Counting { beats_remaining } => Some(beats_remaining.to_string()),
        // GO holds one beat past the release and then the card comes down. The
        // fighters are already moving; a "GO!" that stayed up would be sitting
        // on the match it announced.
        OpeningPhase::Live if elapsed < total + per_beat => Some("GO!".to_string()),
        OpeningPhase::Live => None,
    };
    match word {
        Some(word) => readouts.set(
            SMASH_ANNOUNCE_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::bare(word),
        ),
        // Unconditional now, and it can be: the arm above this one already
        // handed the slot over for the rest of the match.
        None => readouts.clear_slot(SMASH_ANNOUNCE_HUD_SLOT),
    }
}

/// Put a respawning fighter back over the platform.
///
/// through `reset_body_clusters`, not `transit_body`, and the difference is a
/// leak. Both re-resolve a body's pose against the world (ADR 0024 — a body
/// appearing somewhere has to ARRIVE there, not be teleported into whatever is
/// standing at the coordinates), but `transit_body` documents that "axis
/// maneuver state (coyote, buffers, dash timers) is deliberately KEPT — those
/// are time facts, not place facts". That is right for a blink and wrong for
/// losing a stock: a fighter came back holding the dash timer and buffered jump
/// it died with.
///
/// `reset_body_clusters` is the verb that means "this body starts again" — the same one the
/// sandbox reset and the versus round boundary use — and it raises
/// `BodyLifetime::restart_pending`, so `announce_body_restarts` triggers `ae::BodyRestarted`
/// and every PROVIDER hears about the respawn too.
fn place_respawning_fighters(
    mut commands: bevy::prelude::Commands,
    mut due: bevy::prelude::MessageReader<ambition_platformer2d::actor::FighterRespawnDue>,
    mut bodies: bevy::prelude::Query<(
        ambition_platformer2d::actor::BodyClusterQueryData,
        &mut ambition_platformer2d::actor::MotionModel,
        // the SEAT, so two fighters returning on one frame do not land inside
        // each other. `Option` because a body without one is not a seated
        // fighter, and this system must not stop placing it.
        Option<&ambition_platformer2d::actor::MatchSeat>,
        // The swing this fighter was mid-way through when it lost the stock.
        // Needed by VALUE, not just as a component to strip: cancelling a move
        // means despawning the strike boxes it derived, and only the playback
        // knows which entities those are.
        Option<&mut ambition_platformer2d::combat::moveset::MovePlayback>,
    )>,
) {
    // ⭐ D192: the cue is the INTERVAL ELAPSING, not the stock being spent. An
    // eliminated fighter never opens a pending-respawn episode, so there is no
    // `eliminated` arm to skip here any more — the engine decides who is coming
    // back, and this decides where they land.
    for event in due.read() {
        let Ok((clusters, mut model, seat, playback)) = bodies.get_mut(event.body) else {
            continue;
        };
        let seat = seat.map_or(0, |seat| seat.0);
        let mut item = clusters;
        let mut clusters = item.as_clusters_mut();
        // Velocity is zeroed by the reset itself, which is what a fighter that
        // keeps the velocity that threw it off the stage needs: otherwise it
        // respawns already travelling toward the blast zone it just left.
        ambition_platformer2d::engine_core::reset_body_clusters(
            &mut model,
            &mut clusters,
            respawn_placement(stage_centre(), seat),
            // This demo's fighters run the engine's default air game; a stage
            // that tuned it would pass its own number here, which is the point
            // of the parameter.
            ambition_platformer2d::engine_core::DEFAULT_TUNING.air_jumps,
        );
        // RESPAWN PROTECTION.
        //
        // A fighter materialising over the stage was hittable on its first
        // frame, at the exact moment it has no information and no options — the
        // opponent that just took the stock is standing there. Every platform
        // fighter answers this the same way, and so does the engine already:
        // `Empowered` is the generic timed-untouchable grant a star pickup uses,
        // it is rollback-registered, and it expires on its own.
        //
        // the RULESET grants it, not the character. The same fighter in
        // Ambition has no stocks to lose and gets none of this; a mode that
        // wants none simply does not insert it. That is why this is here rather
        // than on a `CharacterDefinition`.
        // A fighter KO'd mid-swing still carries that swing. Its move did not
        // survive the stock it cost, and leaving it on would mean the returning
        // body is "acting" on the frame it materialises — which spends the
        // protection below before its owner has touched the controller.
        //  through the ONE teardown path, which despawns the strike boxes
        // the swing derived rather than leaving them for the next tick's
        // orphan sweep. Stripping the component alone is a second meaning of
        // "cancel this move", and the boxes outlive the move that owns them.
        if let Some(mut playback) = playback {
            ambition_platformer2d::combat::moveset::cancel_move_playback(
                &mut commands,
                event.body,
                &mut playback,
                // ⭐ THE BODY LEFT PLAY — this is the respawn after a stock. A
                // storing charge does NOT bank across it: see `MoveEnd`.
                ambition_platformer2d::combat::moveset::MoveEnd::LeftPlay,
            );
        }
        // ⛔⛔ ITS OWN GRANT, NOT A BORROWED `Empowered`. The first version
        // inserted an `Empowered(UNTOUCHABLE)` beside a marker and claimed the
        // marker made the removal safe. It did not: `Empowered` is ONE
        // component, so granting respawn protection OVERWROTE whatever power-up
        // the body was already carrying, and ending the beat removed the whole
        // component and every semantic in it. A marker cannot turn a single-slot
        // component into two independently owned grants.
        //
        // ⇒ `RespawnGrace` carries its own clock and publishes
        // `Invulnerability::RESPAWN`, a reason bit — the type whose entire
        // purpose is "take or release ONE reason, leaving every other reason
        // alone". A fighter that picked something up on the way down keeps it
        // through the respawn and past the end of it.
        commands
            .entity(event.body)
            .try_insert(ambition_platformer2d::actor::RespawnGrace {
                remaining: RESPAWN_PROTECTION_SECONDS,
            });
    }
}

/// ⭐ THE PLATFORM IS THE PROTECTION, MADE VISIBLE — it exists for exactly as
/// long as `RespawnGrace` does.
///
/// A returning fighter used to appear in free air with nothing but an invisible
/// timer saying it was safe. The genre materialises you on a platform because
/// that is how the protection is READ: you can see whose beat it is and when it
/// ends.
///
/// ⛔ ONE AUTHORITY, NOT TWO CLOCKS. The platform does not run its own timer —
/// it is present iff the seat's fighter carries the grace, so the release rule
/// (a swing spends it, or the grant expires) already decides the platform and
/// the two cannot disagree. A platform with its own duration is how a fighter
/// ends up standing on a beat it has already spent.
///
/// ⚠ it is ORDINARY collision, and that is the genre's answer too: anybody may
/// stand on a respawn platform, and anybody standing on one when it goes falls.
fn hold_the_respawn_platforms(
    mut platforms: bevy::prelude::ResMut<
        ambition_platformer2d::world::collision::MovingPlatformSet,
    >,
    // ⭐ ONE QUESTION, because `RespawnGrace` owns its own clock: a grant that
    // runs out REMOVES itself, so the platform's presence is simply the
    // component's presence. The first version borrowed an `Empowered` and had to
    // ask a second question — "is that still there?" — and retract the marker by
    // hand, which is a latch waiting for a second removal site.
    protected: bevy::prelude::Query<
        (
            &ambition_platformer2d::actor::MatchSeat,
            &ambition_platformer2d::engine_core::BodyKinematics,
        ),
        bevy::prelude::With<ambition_platformer2d::actor::RespawnGrace>,
    >,
) {
    let mut wanted: Vec<(String, Vec2)> = Vec::new();
    for (seat, kin) in &protected {
        wanted.push((
            respawn_platform_id(seat.0),
            Vec2::new(kin.pos.x, kin.pos.y + RESPAWN_PLATFORM_DROP_PX),
        ));
    }
    // Sorted by id, so the set's order is a function of WHICH seats are
    // protected and never of query order — the visuals reconcile by index and
    // the resource is rollback-canonical, so both want a deterministic Vec.
    wanted.sort_by(|a, b| a.0.cmp(&b.0));

    // ⛔⛔ PLACED ONCE, NOT REBUILT. This cleared every respawn platform and
    // re-pushed it from `kin.pos` on every tick, so a platform whose sweep is
    // genuinely zero still TRACKED THE BODY EXACTLY — walk 200px and it walked
    // with you. Its own comment below has always called it stationary.
    //
    // ⭐ THE COST IS NOT COSMETIC. A brain reads the floor it stands on to
    // answer every ledge question, and a floor defined as *"wherever I am"*
    // makes those questions CIRCULAR: the perceived distance to the edge is a
    // constant 48px however far the body walks. Measured — with the block
    // visible to perception, the fighter rollout judged every verb to walk off
    // it and vetoed all of them, every tick (`D-BRAIN-PLATFORM-FLOOR`).
    //
    // ⭐ AND IT IS THE GENRE'S ANSWER TOO: a respawn platform is somewhere you
    // LEAVE, and one that follows cannot be left.
    platforms.0.retain(|platform| {
        !platform.id.starts_with("respawn_platform_")
            || wanted.iter().any(|(id, _)| *id == platform.id)
    });
    for (id, centre) in wanted {
        // Already standing where it was placed — leave it exactly there.
        if platforms.0.iter().any(|platform| platform.id == id) {
            continue;
        }
        platforms.0.push(
            ambition_platformer2d::world::platforms::MovingPlatformState::from_sweep(
                id,
                "Respawn platform",
                centre,
                RESPAWN_PLATFORM_SIZE,
                // Stationary: a sweep of zero width at zero speed. The
                // vocabulary has no "still" variant because nothing wanted one
                // until now, and a zero sweep is exactly that rather than a
                // special case.
                0.0,
                0.0,
            ),
        );
    }
}

/// ⭐ SWINGING GIVES THE PROTECTION UP, which is the genre's anti-camping rule
/// and the half this demo was missing.
///
/// Respawn protection was a flat timer that nothing could end, so a returning
/// fighter had two full seconds in which it could attack and could not be
/// answered — a free hit every stock, taken from the opponent that had just
/// earned the knockout. Smash's platform releases you on your first action for
/// exactly this reason.
///
/// ⛔ ONLY THE GRANT THIS RULESET GAVE. `RespawnGrace` is the marker, not the
/// `UNTOUCHABLE` trait: a fighter that picked something up on the way down keeps
/// what the pickup gave it.
///
/// ⚠ AND ONLY A MOVE THE OWNER STARTED. The trigger is a move's PLAYBACK
/// appearing, which is a body committing to something — not a held button and
/// not a movement axis. A fighter still gets to fall in, drift, and choose a
/// landing under protection, which is what the window is for; it loses it the
/// moment it uses the window to attack from.
fn a_swing_spends_the_respawn_protection(
    mut commands: bevy::prelude::Commands,
    swinging: bevy::prelude::Query<
        bevy::prelude::Entity,
        (
            bevy::prelude::With<ambition_platformer2d::actor::RespawnGrace>,
            bevy::prelude::Added<ambition_platformer2d::combat::moveset::MovePlayback>,
        ),
    >,
) {
    for body in &swinging {
        // ⭐ ONE REMOVAL, and the reason retracts with it: nothing else writes
        // `Invulnerability::RESPAWN`, and the removal hook clears it. Whatever
        // else is holding this body untouchable is untouched.
        commands
            .entity(body)
            .remove::<ambition_platformer2d::actor::RespawnGrace>();
    }
}

/// The id one seat's respawn platform is keyed by.
///
/// Keyed by SEAT rather than by entity: a returning fighter is a body that may
/// be rebuilt, and the platform is a property of where that seat comes back.
fn respawn_platform_id(seat: usize) -> String {
    format!("respawn_platform_{seat}")
}

/// The platform a returning fighter materialises on: three body-widths across
/// and thin, so it reads as a ledge to step off rather than as stage.
const RESPAWN_PLATFORM_SIZE: Vec2 = Vec2::new(96.0, 12.0);

/// How far below the fighter's centre the platform's own centre sits — half a
/// standing body plus half the platform, so its TOP is under the feet.
const RESPAWN_PLATFORM_DROP_PX: f32 = 24.0 + RESPAWN_PLATFORM_SIZE.y * 0.5;

/// The freeze a FULL punch is measured against, in seconds.
///
/// ⭐ MEASURED, not chosen: `hitlag_duration` scales with damage, and this is
/// the length a heavy connect produces under this stage's feel. A reference
/// below it would saturate on ordinary jabs and the HUD would punch identically
/// for everything.
const HUD_PUNCH_REFERENCE_HITLAG: f32 = 0.12;

/// How long a returning fighter cannot be hit, in seconds.
///
/// Long enough to fall in, read the stage and choose a landing; short enough
/// that camping the spawn point is not free. Smash Ultimate's respawn platform
/// holds for about three seconds and releases on the first action; this is the
/// no-platform version of the same idea.
const RESPAWN_PROTECTION_SECONDS: f32 = 2.0;

/// D192 — how long the stage waits before putting a knocked-out fighter back.
///
/// ⭐ THE BEAT THE KO HAD NOWHERE TO HAPPEN IN. At zero the body was placed on
/// the same tick the stock was spent, so the KO cue played over a fighter who was
/// already back and the camera had to frame a live body that appeared ~500 units
/// away with no travel — measured as the three largest single-tick camera steps
/// in a 5,400-tick match, against a p99 of 13.1.
///
/// One second is the genre's pause between the knockout and the reappearance.
///
/// D201: SECONDS, because the beat is now the engine's `DeathInterlude` — the
/// same window a Mary-O death opens, counted on `WorldTime` and rewound with
/// everything else. The tick spelling D192 chose was a second clock for one
/// beat, argued from a determinism premise the component beside it disproves.
const RESPAWN_INTERVAL_SECONDS: f32 = 1.0;

/// Take an eliminated fighter OUT OF PLAY.
///
/// The stock was spent exactly once — the engine's `Without<FighterEliminated>` filter held —
/// the match was decided, and the body simply never stopped being a body. That is the
/// difference between "the count is correct" and "the match is over", and it is the ruleset's
/// half.
///
/// Despawn rather than park: a fighter that is out has no state anybody reads,
/// and leaving it somewhere off-screen is how a match ends with an invisible
/// participant still generating hit events.
fn take_eliminated_fighters_out_of_play(
    mut commands: bevy::prelude::Commands,
    eliminated: bevy::prelude::Query<
        bevy::prelude::Entity,
        (
            bevy::prelude::With<ambition_platformer2d::actor::FighterEliminated>,
            bevy::prelude::With<ambition_platformer2d::actor::MatchSeat>,
        ),
    >,
) {
    for body in eliminated.iter() {
        commands.entity(body).despawn();
    }
}

/// How long the winner card stands before the demo goes back to choosing.
///
/// The banner itself asks for 3.0s; this waits a beat longer so the card is
/// READ rather than glimpsed on the way out.
const RETURN_TO_SELECT_AFTER: f32 = 4.5;

/// Ensure the Smash gameplay route carries Smash-owned combat rules. The lobby
/// normally publishes them when a battle starts; this is a route-level safety
/// net for direct or stale entry and does not rewrite an already-correct
/// declaration.
fn the_stage_always_plays_by_smash_rules(
    mut commands: bevy::prelude::Commands,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    declared: Option<bevy::prelude::Res<ambition_platformer2d::combat::rules::DeclaredCombatRules>>,
) {
    let on_stage = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_GAMEPLAY_ROUTE);
    if !on_stage {
        return;
    }
    if declared.is_some_and(|rules| rules.declared_by == SMASH_EXPERIENCE) {
        return;
    }
    commands.insert_resource(smash_declared_combat_rules());
}

/// SMASH'S FIGHTERS ARE SOLID TO EACH OTHER — this is jostle.
///
/// ⛔⛔ IT RUNS IN THE SIMULATION, NOT `Update`. `BodyContact` is a fact the
/// movement kernel reads, so granting it from an ordinary `Update` system meant
/// a schedule that does NOT replay under rollback was establishing simulation
/// state — a resimulated frame could integrate a cast that had not been made
/// solid yet. Two more grants copied this shape before a review caught all
/// three; the other two are gone (the mount role is a seat fact now, and a
/// summoned mount's departure rides its own registered state).
///
/// ⚠ THE HONEST ENDPOINT IS `MatchBody`, which is already where a match states
/// what it believes about its fighters' bodies — jump squat, air dodge, dodge
/// staling. Jostle belongs beside them, applied in the same flush that builds
/// the bodies. That is a wire change to a snapshotted type and is deliberately
/// not folded into this repair.
///
/// The engine therefore owns an unnamed constraint — one body's proposed motion reduced by the
/// bodies it is touching (`ambition_platformer2d::engine_core::movement::body_contact`) — and this ruleset
/// grants it to its cast. Nothing in the kernel knows the word jostle.
///
/// A test that supplies its own precondition cannot prove the mechanism reaches production.
///
/// granted to `MatchBody`-seated fighters, which is the cast. A projectile
/// or a stage prop that happens to be a body is not a fighter and does not get
/// it; the grant follows the thing the ruleset seated.
fn smash_fighters_are_solid_to_each_other(
    mut commands: bevy::prelude::Commands,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    fighters: bevy::prelude::Query<
        bevy::prelude::Entity,
        (
            bevy::prelude::With<ambition_platformer2d::actor::FighterStocks>,
            bevy::prelude::Without<ambition_platformer2d::platformer::body::BodyContact>,
        ),
    >,
) {
    let on_stage = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_GAMEPLAY_ROUTE);
    if !on_stage {
        return;
    }
    // `Without<BodyContact>` in the filter IS the idempotence: a body that
    // already has it is not in the query, so nothing is written on the frames
    // where nothing changed, and no change tick moves.
    for fighter in &fighters {
        commands
            .entity(fighter)
            .try_insert(ambition_platformer2d::platformer::body::BodyContact::FIRM);
    }
}

/// Return to character select after a decided match has left its winner card
/// visible for [`RETURN_TO_SELECT_AFTER`] — or IMMEDIATELY, when the verdict is
/// a `NoContest` and there is no card to leave visible.
///
/// ⛔⛔ AND IT ARMS ONLY ON A CONFIRMED FRAME, because leaving the stage is not
/// retractable. It used to arm on `StocksMatchDecided`, which a SPECULATIVE
/// frame can write: the countdown is a `Local` that GGRS never rewinds, so a
/// decision that was later rolled back still sent the player back to the lobby
/// out of a match that was still being fought. There is no retraction to write —
/// the fix is not to commit in the first place.
///
/// ⭐ TWO CHANGES, AND THE SECOND IS WHAT MAKES THE FIRST SAFE.
///
/// It reads `StocksMatchSettled` — rollback STATE, stamped with the match it is
/// about — instead of the message, so a rewound decision simply un-settles and
/// there is nothing left claiming the match ended. And it waits for
/// `ConfirmedFrameBoundary::fully_confirmed`, so by the time the countdown arms
/// the settlement can never be simulated again.
///
/// ⛔ THE STAMP ALSO REPLACES THE `decided.clear()` this used to do on leaving
/// the stage: a verdict for the PREVIOUS match cannot arm the next one, because
/// the instance differs. Same rule as the abandon latch.
fn return_to_the_select_screen_when_the_match_ends(
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    time: bevy::prelude::Res<bevy::prelude::Time>,
    mut shell: bevy::prelude::MessageWriter<ambition_platformer2d::game_shell::ShellCommand>,
    mut readouts: bevy::prelude::ResMut<ambition_platformer2d::presentation::HudReadouts>,
    // WHETHER THIS MATCH IS OVER, from the authority that rewinds.
    settled: Option<
        bevy::prelude::Res<
            ambition_platformer2d::actors::features::stocks_match::StocksMatchSettled,
        >,
    >,
    active: Option<
        bevy::prelude::Res<ambition_platformer2d::actors::character_runtime::ActiveMatch>,
    >,
    // ⛔ `Option`: absent means there is no rollback host, and the module's own
    // doc says that case confirms everything.
    boundary: Option<
        bevy::prelude::Res<ambition_platformer2d::engine_core::ConfirmedFrameBoundary>,
    >,
    // WHICH SESSION OWNS THE WORLD RIGHT NOW. See the leftover-match note below.
    scope: Option<bevy::prelude::Res<ambition_platformer2d::actor::ActiveSessionScope>>,
    mut countdown: bevy::prelude::Local<Option<f32>>,
) {
    let on_stage = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_GAMEPLAY_ROUTE);
    if !on_stage {
        // Left by some other road — the pause menu, a host quitting home. The
        // countdown belongs to THIS visit to the stage.
        *countdown = None;
        readouts.clear_slot(SMASH_ANNOUNCE_HUD_SLOT);
        return;
    }
    // ⛔⛔ A RETIRED SESSION'S `ActiveMatch` OUTLIVES IT BY AT LEAST A FRAME, and
    // reading one applies the previous match's verdict to the match that
    // replaced it. Jon, 2026-08-27: picking a cast for a SECOND match and
    // pressing start bounced straight back to the select screen — the log shows
    // `session-start scope=1`, `room-loaded smash_stage`, `session-end scope=1`
    // one frame apart, three times running.
    //
    // ⭐ THE VERDICT'S OWN SCOPING COULD NOT CATCH IT. `StocksMatchSettled` names
    // the match it decided and `verdict()` compares instances — but BOTH sides
    // of that comparison were the retired match, so it agreed. The stale half is
    // the `ActiveMatch` resource, not the latch, and only the SESSION knows which
    // of those two is current.
    //
    // ⚠ THE OLD ROAD HID IT: the countdown below waits 4.5s and a confirmed
    // frame, which the new match's activation always beat. The immediate exit
    // Jon asked for on `NoContest` runs on the first frame the router says
    // "on stage", which is exactly the frame the leftover is still there.
    let live = active.as_deref().filter(|active| match scope.as_deref() {
        // A composition with no session lifecycle has nothing to be stale about.
        None => true,
        Some(scope) => active.session() == scope.current(),
    });
    let ended = match (settled.as_deref(), live) {
        (Some(settled), Some(active)) => settled.settled(active),
        _ => false,
    };
    // ⭐⭐ AN ABANDONED MATCH GOES HOME ON THE PRESS. Jon, 2026-08-26: *"skip
    // the no contest presentation for now and just exit to the character select
    // menu immediately."* A knockout earns its card and its beat; a match
    // somebody asked to stop has no result to show, and three seconds of a card
    // reading NO CONTEST is the only thing between the press and the lobby.
    //
    // ⛔ AND IT DOES NOT WAIT FOR CONFIRMATION, which is safe for exactly this
    // verdict and no other. The other two are reached by the SIMULATION, so a
    // rewind can retract them and leaving the stage cannot be taken back.
    // `NoContest` is reached only by `MatchAbandonRequest` — a latch made
    // OUTSIDE the simulation that does not rewind — so the resim reaches the
    // same verdict and there is nothing to retract. That is the same argument
    // that gave the request its shape; see `MatchAbandonRequest`.
    let abandoned = settled
        .as_deref()
        .zip(live)
        .and_then(|(settled, active)| settled.verdict(active))
        .is_some_and(|verdict| {
            matches!(
                verdict,
                ambition_platformer2d::actor::MatchVerdict::NoContest
            )
        });
    if abandoned {
        *countdown = None;
        readouts.clear_slot(SMASH_ANNOUNCE_HUD_SLOT);
        shell.write(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(SMASH_SELECT_ROUTE),
        ));
        return;
    }
    let confirmed = boundary
        .as_deref()
        .is_none_or(|boundary| boundary.fully_confirmed());
    if ended && confirmed && countdown.is_none() {
        *countdown = Some(RETURN_TO_SELECT_AFTER);
    }
    let Some(remaining) = countdown.as_mut() else {
        return;
    };
    *remaining -= time.delta_secs();
    if *remaining <= 0.0 {
        *countdown = None;
        shell.write(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(SMASH_SELECT_ROUTE),
        ));
    }
}

/// Announce the winner in the stage's persistent centered readout.
///
/// The readout remains until the stage is left, so its lifetime follows the
/// results route rather than a separate timer.
fn announce_the_winner(
    // ⛔⛔ THE LATCH, NOT THE MESSAGE, and the card is why the latch grew a
    // verdict. `StocksMatchDecided` can be written on a SPECULATIVE frame, and a
    // HUD readout is not retractable — a rolled-back verdict left NO CONTEST on
    // screen over a match that was still being fought. Its sibling (the return
    // countdown) was fixed by reading `StocksMatchSettled`, which rewinds; this
    // one could not follow while the latch said only WHETHER.
    //
    // ⛔ AND DECLINING TO READ UNTIL CONFIRMED IS NOT THE SAME FIX. A reader that
    // keeps its cursor is still bounded by a two-frame channel, so a confirmation
    // arriving later loses the announcement rather than delaying it. State has no
    // cursor.
    settled: Option<
        bevy::prelude::Res<
            ambition_platformer2d::actors::features::stocks_match::StocksMatchSettled,
        >,
    >,
    active: Option<
        bevy::prelude::Res<ambition_platformer2d::actors::character_runtime::ActiveMatch>,
    >,
    // ⛔ `Option`: absent means there is no rollback host, and that module's own
    // doc says the absent case confirms everything.
    boundary: Option<
        bevy::prelude::Res<ambition_platformer2d::engine_core::ConfirmedFrameBoundary>,
    >,
    // Whether a side is a person or a team is a fact about the match that was prepared, and the
    // plan is the only thing that still knows it once fighters start being removed.
    prepared: Option<
        bevy::prelude::Res<ambition_platformer2d::actors::character_runtime::PreparedMatch>,
    >,
    // WHICH MATCH has already had its card written — see the rising-edge note.
    mut announced: bevy::prelude::Local<
        Option<ambition_platformer2d::actors::character_runtime::MatchInstance>,
    >,
    // Use surviving fighters only to resolve display names for the winning side.
    fighters: bevy::prelude::Query<(
        &ambition_platformer2d::actors::character_runtime::MatchSeat,
        Option<&ambition_platformer2d::combat::targeting::MatchTeam>,
        &bevy::prelude::Name,
    )>,
    mut readouts: bevy::prelude::ResMut<ambition_platformer2d::presentation::HudReadouts>,
) {
    let Some((settled, active)) = settled.as_deref().zip(active.as_deref()) else {
        return;
    };
    if !boundary
        .as_deref()
        .is_none_or(|boundary| boundary.fully_confirmed())
    {
        return;
    }
    // ⛔ THE RISING EDGE, not every tick the latch is true. The message this
    // replaced fired ONCE; a state read writes the readout for the rest of the
    // match, which is churn on a HUD slot and a second announcement in any log
    // that records what changed.
    let this_match = active.instance();
    if announced.as_ref() == Some(&this_match) {
        return;
    }
    if let Some(verdict) = settled.verdict(active) {
        *announced = Some(this_match);
        // ⛔ A NO CONTEST GETS NO CARD. Jon, 2026-08-26: *"skip the no contest
        // presentation for now."* The card exists to tell a player which of the
        // three endings happened; somebody who just picked `Exit Match` already
        // knows, and the announcement only delays the lobby they asked for. The
        // rising edge is consumed above regardless, so this does not re-ask the
        // question every tick.
        if matches!(
            verdict,
            ambition_platformer2d::actor::MatchVerdict::NoContest
        ) {
            return;
        }
        // Keep a team's name unless the winning side has exactly one participant.
        // Resolve participant identity from the match roster, not surviving bodies;
        // simultaneous ring-outs may leave no resident winner body, so the side name
        // remains the fallback.
        let named = verdict.winner().map(|side| {
            // A composition with no prepared plan cannot say how big a side is,
            // and the honest answer for an unknown size is the side's own name.
            let solo = prepared
                .as_deref()
                .is_some_and(|prepared| prepared.seats_on_side(side) == 1);
            let name = solo
                .then(|| {
                    fighters
                        .iter()
                        .find(|(seat, team, _)| {
                            ambition_platformer2d::combat::stocks::side_label(seat.0, *team) == side
                        })
                        .map(|(_, _, name)| name.as_str().to_string())
                })
                .flatten();
            // A real team won together — naming one of its members would put a
            // player's name on somebody else's victory — or nobody is left
            // standing to ask. Either way the side is the honest answer.
            name.unwrap_or_else(|| side.to_string())
        });
        readouts.set(
            SMASH_ANNOUNCE_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::bare(victory_banner(
                verdict,
                named.as_deref(),
            )),
        );
    }
}

/// The screen the demo opens on, and the transition out of it.
///
/// The decision itself is [`select::SmashSelect`], which has no Bevy in it. This
/// is the part that has to: it holds the value, and when the value says the
/// match is decided it publishes the roster and asks the shell to go to the
/// stage.
///
/// the roster is inserted BEFORE the route changes, and the order is the
/// whole correctness argument. Seating runs on the sim schedule and reads
/// `MatchParticipantRoster`; if the route changed first, the stage would come up
/// with no roster, seating would find nothing to do, and the match would open
/// with an empty cast that nothing retries into existence — the roster arrives
/// once, and it has to arrive before the thing that reads it.
pub struct SmashSelectPlugin;

/// When the select screen reads its input, as something another system can
/// be ordered against.
///
/// Exists because "before the screen" is a real question with no other answer: a
/// windowed host REBUILDS `SeatMenuFrames` from its participants every frame
/// (clearing first), so anything that wants to put a press into that port —
/// a test, a replay, a remote seat — has to land between the producer and this.
/// Without a named set, a system that tried ran wherever Bevy put it and the
/// press was silently dropped about half the time.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SmashSelectSet;

impl bevy::prelude::Plugin for SmashSelectPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        // The screen is a ROUTE, so it says so. `get_resource_or_insert_with`
        // rather than `resource_mut` because the rules-and-screen plugins are
        // also composed in harnesses that never installed the shell.
        app.world_mut()
            .get_resource_or_insert_with(
                ambition_platformer2d::game_shell::ShellRouteCatalog::default,
            )
            .register(ambition_platformer2d::game_shell::ShellRouteSpec::new(
                SMASH_SELECT_ROUTE,
                SMASH_SELECT_EXPERIENCE,
            ));
        // AND IT SOUNDS LIKE ITSELF. The select screen has a score written
        // for it, and this is the declaration that carries it into any host —
        // the standalone demo, Ambition, or a composition that does not exist
        // yet. Declared HERE, beside the route, because the two are one fact
        // about one screen; a host naming smash's music would be a host knowing
        // a provider's content.
        {
            use ambition_platformer2d::audio::selection::FrontendAudioAppExt;
            app.declare_route_frontend_audio(
                SMASH_SELECT_ROUTE,
                ambition_platformer2d::audio::selection::FrontendAudioProfile::new(
                    SMASH_EXPERIENCE,
                )
                .with_title_track(SMASH_SELECT_TRACK)
                .with_sfx([
                    ambition_platformer2d::sfx::ids::UI_MENU_MOVE,
                    ambition_platformer2d::sfx::ids::UI_MENU_ACCEPT,
                    ambition_platformer2d::sfx::ids::UI_MENU_BACK,
                ]),
            );
        }
        // THE RULESET'S OWN ROLLBACK STATE.
        //
        // ⛔⛔ THROUGH `AmbitionRollbackApp`, NOT a `SchemaRollbackRegistrar`.
        // The schema registrar RECORDS a registration and installs no probe, so
        // a component registered that way appears in the baseline and is still
        // invisible to the localizer — `rollback_exit_oracle` fails by name,
        // which is how this was caught. Same road `ambition_demo_sanic` takes
        // for its own content state.
        {
            use ambition_platformer2d::rollback::AmbitionRollbackApp;
            app.rollback_component_clone_probed::<crate::shark_ride::Departing>(
                "ambition_demo_smash",
                "smash.departing_mount",
                crate::shark_ride::departing_probe,
            );
            // The bomb's fuse and its remembered speed. Both outlive the tick
            // that made them, so a rewind that put the bomb back without putting
            // its fuse back would give the resimulated timeline a different
            // explosion from the confirmed one.
            app.rollback_component_clone_probed::<crate::bomb::LiveBomb>(
                "ambition_demo_smash",
                "smash.live_bomb",
                crate::bomb::live_bomb_probe,
            );
        }
        app.init_resource::<select::SmashSelect>();
        // The pointer, and the one thing it can ask for that the value does not
        // hold. Both live outside `SmashSelect` on purpose: where a cursor is
        // pointing is not part of what the screen DECIDED, and a decision value
        // that carried a screen position would change every time somebody moved
        // the mouse.
        app.init_resource::<select_screen::cursor::SelectCursors>();
        app.init_resource::<select_screen::SelectPage>();
        app.init_resource::<select_screen::SelectInteractionPolicy>();
        app.init_resource::<select_screen::StartRequested>();
        app.init_resource::<select_screen::LeaveRequested>();
        // THE ROSTER IS A COMPOSITION FACT, so it is resolved once, late.
        //
        // By `Startup` every provider in the composition has declared itself.
        app.init_resource::<select::SmashRoster>();
        app.add_systems(bevy::prelude::Startup, assemble_the_smash_roster);
        // THE PORTRAIT SHEETS' OWN MANIFESTS, so a face is one FRAME.
        //
        // without this the grid drew each portrait PNG whole, which is right
        // for the single-frame sheets that are most of them and visibly wrong
        // for `alice` and `oiler` — 2048x320 each, eight frames of a
        // default/speaking/focused clip set, drawn as a strip of eight tiny
        // Alices. Found by looking at a capture.
        //
        // guarded, because Ambition's dialogue box installs the same plugin
        // and Bevy panics on a duplicate. This demo is composed both standalone
        // and inside that host; whichever gets there first wins and the registry
        // is the same baked table either way.
        if !app
            .is_plugin_added::<ambition_platformer2d::sprite_sheet::PortraitSheetRegistryPlugin>()
        {
            app.add_plugins(ambition_platformer2d::sprite_sheet::PortraitSheetRegistryPlugin);
        }
        // THE SCREEN DECLARES ITS OWN INPUT PORT. The host fills
        // `SeatMenuFrames` when a windowed host is installed; `init_resource`
        // will not clobber one that already exists. Declaring it here means the
        // screen is drivable in a headless app too — which is what lets a TEST
        // press a button instead of reaching into `SmashSelect` and setting the
        // answer, and reaching into the answer is how this screen came to be
        // fully unit-tested and completely inert.
        app.init_resource::<ambition_platformer2d::input::SeatMenuFrames>();
        // AND THE SEATS IT OFFERS. A host seats input participants from the
        // match roster, and this screen is what PRODUCES the roster — so until
        // it declares them, only player one exists and the other panels are
        // chairs no controller can reach. See `LocalSeatOffer`, which carries
        // the couch POLICY with the count because the two are one statement:
        // seats without a policy are seats the default hands straight back to
        // player one.
        app.init_resource::<ambition_platformer2d::input::LocalSeatOffer>();
        // ONE CHAIN, IN `InputSet::Consume`. Two things were ambiguous and
        // both are the same mistake — a reader with no stated order.
        //
        // 1. Against the PRODUCER. A windowed host rebuilds `SeatMenuFrames`
        //    from the participants every frame (`frames.clear()` first), so
        //    unordered, whether this screen saw a press at all depended on where
        //    Bevy happened to put it. In the demo's own app the producer is not
        //    installed and it always worked; in the multi-game host it is.
        // 2. Against ITSELF. Arriving at the screen resets the previous match's
        //    decision, and the transition out reads that decision — running in
        //    the other order, re-entering the screen leaves for the stage again
        //    on the frame it arrives.
        app.configure_sets(
            bevy::prelude::Update,
            bevy::prelude::IntoScheduleConfigs::in_set(
                SmashSelectSet,
                ambition_platformer2d::input::InputSet::Consume,
            ),
        );
        // THE SCREEN CLAIMS ITS SEATS' INPUT while it is up.
        //
        // Declared in `ResolveContext`, ahead of every router — so a HIGHER
        // capturing claim (the universal pause menu, at `context_priority::PAUSE`)
        // simply outranks it and this screen stops driving, with neither side
        // naming the other. See `drive_the_select_screen`.
        app.add_systems(
            bevy::prelude::Update,
            bevy::prelude::IntoScheduleConfigs::in_set(
                declare_the_select_input_context,
                ambition_platformer2d::input::InputSet::ResolveContext,
            ),
        );
        // AND IT SAYS WHAT ITS CONFIRM CONTROL DOES.
        //
        // A claim says who the presses are FOR; a cue says what confirming MEANS, in this screen's
        // own words.
        //
        // the cue is also the only evidence a prompt has when no context
        // resolver is installed. `publish_frontend_context_prompt` reads the
        // resolved owner, but a composition without the host's resolver has none
        // and falls through to the no-subject exit, which now asks for a cue and
        // answers `Empty` without one — and `Empty` is how the touch overlay
        // decides to hide the move stick and the confirm buttons.
        //
        // `init_resource` will not clobber one the host already owns
        // (`ambition_platformer2d_host` initialises it), and cues are keyed by
        // context, so this screen owns its KEY rather than the map.
        app.init_resource::<ambition_platformer2d::input::ActiveUiCues>();
        app.add_systems(
            bevy::prelude::Update,
            bevy::prelude::IntoScheduleConfigs::in_set(
                publish_the_select_ui_cue,
                ambition_platformer2d::input::InputSet::PublishCues,
            ),
        );
        app.add_systems(
            bevy::prelude::Update,
            bevy::prelude::IntoScheduleConfigs::in_set(
                bevy::prelude::IntoScheduleConfigs::chain((
                    maintain_smash_local_seat_offer,
                    reset_select_frontend_on_arrival,
                    present_select_screen_ui,
                    bevy::prelude::IntoScheduleConfigs::run_if(
                        select_screen::drive_the_cursor,
                        the_select_screen_owns_its_input,
                    ),
                    select_screen::place_the_screen,
                    // Four small projections instead of one screen-wide mutable
                    // query bundle. Their internal order is not semantic; keeping
                    // them inside this drive-before-draw fence preserves the
                    // existing frame contract without a B0001 exclusion matrix.
                    select_screen::sync_select_grid,
                    select_screen::sync_select_cards,
                    select_screen::sync_select_chrome,
                    select_screen::sync_select_tokens_and_cursors,
                    start_the_battle_when_asked,
                    // the safety net for every entry that skips the lobby —
                    // the dev bins and the stage tests. See its doc.
                    the_stage_always_plays_by_smash_rules,
                    // AFTER the driver that sets the flag, and in the same
                    // chain, so a press and the route change it asks for are
                    // one frame apart at most. The screen would otherwise keep
                    // drawing a lobby somebody has already left.
                    leave_the_select_screen_when_asked,
                    return_to_the_select_screen_when_the_match_ends,
                    // The pause menu's contributed row: what it says, and what
                    // picking it means.
                    offer_to_exit_the_match,
                    abandon_the_match_when_the_shell_asks,
                )),
                SmashSelectSet,
            ),
        );
    }
}

/// OFFER `Exit Match` WHILE A MATCH IS RUNNING, and withdraw it when one is not.
///
/// Jon, W8 playtest: *"During an active Smash match, the system/pause menu needs
/// an explicit `Exit Match`, which ends the match as No Contest."*
///
/// ⭐ THE SHELL DRAWS THE ROW AND DOES NOT KNOW WHAT IT MEANS. The universal
/// pause menu has no idea what a match is — it cannot, without every hosted
/// experience adding an arm to it — so this states the WORDS and
/// [`abandon_the_match_when_the_shell_asks`] states the MEANING.
///
/// ⛔ AND THE OFFER IS RETRACTED, not merely set. A stale offer left behind by a
/// finished match puts an `Exit Match` row on the character select screen's own
/// pause menu, pointing at nothing.
fn offer_to_exit_the_match(
    mut commands: bevy::prelude::Commands,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    active: Option<
        bevy::prelude::Res<ambition_platformer2d::actors::character_runtime::ActiveMatch>,
    >,
    // The SETTLEMENT, and it is what the comment below is about. Optional
    // because a composition may reach this route before the stocks feature has
    // installed anything, and there the honest answer is "not settled".
    settled: Option<
        bevy::prelude::Res<
            ambition_platformer2d::actors::features::stocks_match::StocksMatchSettled,
        >,
    >,
    offered: Option<bevy::prelude::Res<ambition_platformer2d::game_shell::ShellAbandonOffer>>,
) {
    let on_stage = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_GAMEPLAY_ROUTE);
    // ⭐⭐ A MATCH THAT HAS BEEN DECIDED IS STILL ACTIVE — the winner card is up
    // and the return countdown is running — so `ActiveMatch` alone cannot answer
    // this, and offering to abandon then is offering to stop something already
    // stopped. The press would reach the abandon latch, which the once-only
    // settle discards because the match already ended: a row that does nothing.
    //
    // ⛔ THE CONDITION, NOT A PROXY. Not the winner card's presence, not a menu
    // state, not a countdown — `StocksMatchSettled::settled` is the authority
    // that decided the match, and it is the same one the abandon road reads.
    let running = active.as_deref().is_some_and(|active| {
        settled
            .as_deref()
            .is_none_or(|settled| !settled.settled(active))
    });
    let offer = on_stage && running;
    match (offer, offered.is_some()) {
        (true, false) => {
            commands.insert_resource(ambition_platformer2d::game_shell::ShellAbandonOffer {
                label: "Exit Match".to_owned(),
                detail: "End this match as a No Contest.".to_owned(),
            });
        }
        (false, true) => {
            commands.remove_resource::<ambition_platformer2d::game_shell::ShellAbandonOffer>();
        }
        _ => {}
    }
}

/// Translate the shell's abandon request into the engine's match-level verb.
///
/// ⭐ TWO LINES, AND THAT IS THE POINT. Jon: *"Reuse the existing match outcome
/// / route transition machinery. Do not introduce a one-off scene teardown
/// path."* Everything after this already exists: `decide_stocks_match` settles
/// the match as a [`MatchVerdict::NoContest`] and
/// [`return_to_the_select_screen_when_the_match_ends`] brings the player back to
/// the lobby — the same systems an ordinary knockout goes through.
///
/// ⛔ WITH ONE STEP SKIPPED, and it is skipped by those systems rather than by a
/// road of its own: [`announce_the_winner`] writes no card for a `NoContest` and
/// the return takes no countdown. Jon, 2026-08-26: *"skip the no contest
/// presentation for now and just exit to the character select menu
/// immediately."*
///
/// ⛔ NOT GATED ON A SEAT. It is a match-level command, so it works the same in
/// CPU-vs-CPU as in a human match; the person who opened the menu is not
/// necessarily playing.
fn abandon_the_match_when_the_shell_asks(
    mut commands: bevy::prelude::Commands,
    mut asked: bevy::prelude::MessageReader<
        ambition_platformer2d::game_shell::ShellAbandonRequested,
    >,
    // WHICH MATCH is being stopped. The ask is made outside the simulation, so
    // it cannot be re-made by a resimulation and cannot ride a channel that
    // rewinds — it names its match instead. See `MatchAbandonRequest`.
    active: Option<
        bevy::prelude::Res<ambition_platformer2d::actors::character_runtime::ActiveMatch>,
    >,
) {
    let asked_to_stop = asked.read().count() > 0;
    if !asked_to_stop {
        return;
    }
    let Some(active) = active else {
        // Nothing is running; there is no match to name.
        return;
    };
    commands.insert_resource(
        ambition_platformer2d::actors::features::stocks_match::MatchAbandonRequest::stop(&active),
    );
}

/// SUDDEN DEATH'S STAGE HALF: put the survivors on the edge of death.
///
/// ⭐ THE ENGINE REFUSED TO DECIDE and said so; what that MEANS to a body is
/// this stage's business. The count knows stocks and the clock; it does not know
/// that this ruleset measures a fighter in percent, and a rule that reached into
/// health from `decide_stocks_match` would be a stocks loop with an opinion
/// about a damage mechanic.
///
/// ⛔ ELIMINATED FIGHTERS ARE NOT REVIVED. A level timeout means the sides are
/// level on what the tiebreak measures, not that everybody is still alive —
/// putting a body that already lost its last stock back on the stage would
/// invent a fighter the match had finished with.
///
/// ⭐⭐ AND ONLY THE TIED SIDES FIGHT IT. With three or more sides alive at the
/// timeout, a side the clock had already put behind is not part of the tie the
/// round exists to break; carrying it in would hand a losing side an even
/// restart, and leaving it on the stage at its own low damage would hand it a
/// BETTER one. A non-contender is out on the clock, said with the same
/// `FighterEliminated` an exhausted fighter is out with — so
/// [`take_eliminated_fighters_out_of_play`] clears its body and
/// `last_side_standing` decides the round among the contenders, with no second
/// notion of "out of the match" to keep in step with the first.
fn open_the_sudden_death_round(
    mut commands: bevy::prelude::Commands,
    mut began: bevy::prelude::MessageReader<
        ambition_platformer2d::actors::features::stocks_match::SuddenDeathBegan,
    >,
    mut fighters: bevy::prelude::Query<
        (
            bevy::prelude::Entity,
            &ambition_platformer2d::actors::character_runtime::MatchSeat,
            Option<&ambition_platformer2d::combat::targeting::MatchTeam>,
            &mut ambition_platformer2d::characters::actor::BodyHealth,
            // ⛔⛔ THE STOCKS, WHICH THIS ROUND IS DEFINED BY AND NEVER TOUCHED.
            &mut ambition_platformer2d::combat::components::FighterStocks,
        ),
        bevy::prelude::Without<ambition_platformer2d::combat::stocks::FighterEliminated>,
    >,
) {
    for round in began.read() {
        for (body, seat, team, mut health, mut stocks) in &mut fighters {
            // The SIDE, not the seat: a team's members stand or fall together,
            // which is the same fold the tiebreak used to name the contenders.
            let side = ambition_platformer2d::combat::stocks::side_label(seat.0, team);
            if round.contenders.iter().any(|contender| *contender == side) {
                health.set_damage_taken(round.starting_damage);
                // ⛔⛔ ONE STOCK, WHICH IS THE WHOLE ROUND. A genuine tie can
                // happen with several stocks each — the existing arms tie at TWO
                // — and this only set the damage. So the first KO spent a stock,
                // the loser was NOT eliminated, the ordinary respawn reset the
                // damage this round had just staged, and sudden death simply
                // went on. "Both at 300%, one stock, first hit decides" was the
                // stated rule and the transition implemented a third of it.
                //
                // ⭐ `remaining`, NOT `started_with`: the latter is what the
                // MATCH began with and the HUD reads it to draw the stock icons.
                stocks.remaining = 1;
            } else {
                commands
                    .entity(body)
                    .try_insert(ambition_platformer2d::combat::stocks::FighterEliminated);
                // ⛔⛔ AND THE OTHER HALF OF LEAVING THE MATCH. `spend_fighter_stocks`
                // does BOTH — insert the marker and remove `ActiveCombatant` —
                // and says why: the body stays standing until a ruleset removes
                // it, so a marker alone leaves a corpse holding attack state and
                // a place on the anti-clump board. Doing half of it here made a
                // second, weaker definition of "out of the match", and command
                // deferral means cleanup cannot be relied on to cover the gap.
                commands
                    .entity(body)
                    .remove::<ambition_platformer2d::combat::components::ActiveCombatant>();
            }
        }
    }
}

/// Who can be picked, in THIS composition.
///
/// `select::SMASH_ROSTER` filtered to the ids this host can SEAT — so a
/// multi-game host offers the whole crossover cast and the standalone demo
/// offers the fighters it declares itself, from one list.
fn assemble_the_smash_roster(
    // the SEATABLE authority, not the catalog — see `SmashRoster::assemble`.
    // Optional because a composition may reach this route before any character
    // is registered; an empty grid then says so honestly rather than offering
    // portraits nothing can build.
    registry: Option<
        bevy::prelude::Res<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>,
    >,
    mut fighters: bevy::prelude::ResMut<select::SmashRoster>,
) {
    let Some(registry) = registry else {
        return;
    };
    let assembled = select::SmashRoster::assemble(&registry);
    if *fighters != assembled {
        *fighters = assembled;
    }
}

/// Maintain Smash's local-seat offer across its frontend and gameplay routes.
///
/// The lobby offers connected local seats; gameplay gets its seats from the
/// frozen match roster but keeps the same JoinToClaim assignment policy. The
/// claim is owner-scoped, so leaving Smash cannot retract another route's offer.
fn maintain_smash_local_seat_offer(
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    devices: Option<bevy::prelude::Res<ambition_platformer2d::input::LocalDeviceOrder>>,
    mut offer: bevy::prelude::ResMut<ambition_platformer2d::input::LocalSeatOffer>,
) {
    let on_select = on_the_select_route(&router);
    let on_smash_route = router.active.as_ref().is_some_and(|active| {
        matches!(
            active.route_id.as_str(),
            SMASH_SELECT_ROUTE | SMASH_GAMEPLAY_ROUTE
        )
    });
    let couch = ambition_platformer2d::input::sources::InputAssignmentPolicy::JoinToClaim;
    let offered = devices
        .as_deref()
        .map(|devices| select::seats_offered_under(devices, couch))
        .unwrap_or(1) as u8;

    if on_smash_route {
        let seats = if on_select { offered } else { 0 };
        if !offer.is_owned_by(SMASH_SELECT_EXPERIENCE)
            || offer.seats() != seats
            || offer.policy() != couch
        {
            offer.claim(SMASH_SELECT_EXPERIENCE, seats, couch);
        }
    } else {
        offer.release(SMASH_SELECT_EXPERIENCE);
    }
}

/// Reset frontend-only select state exactly once when this route is entered.
///
/// `SmashSelect` is the lobby decision, while cursor/page/request state is interaction state. A
/// rematch must start with neither the previous decision nor the previous hand positions.
///
/// ⛔⛔ "EXACTLY ONCE PER ARRIVAL" IS THE ACTIVATION, NOT AN ENTITY COUNT. This
/// used to gate on the select UI root not existing yet, as a stand-in for "the
/// reset has not run this visit" — and the stand-in is false on the SECOND
/// visit, because the first visit's root outlives the route change. Measured
/// 2026-08-27 on the second match: arriving back at the lobby with
/// `ui_roots=1`, so the body below never ran and `MatchParticipantRoster`,
/// `StartRequested` and the previous decision all stood. `start_the_battle_when_asked`
/// refuses while a roster stands (`!on_select || roster.is_some()`), so pressing
/// start did nothing at all — Jon: *"in the second match I select characters
/// press start, but it just brings me back to the character screen"*.
///
/// ⭐ THE ROUTER ALREADY NAMES THE ARRIVAL. `ShellActivationId` is minted per
/// activation and is the same id the world-event log prints, so remembering the
/// one this ran for says what the entity count was only guessing at.
fn reset_select_frontend_on_arrival(
    mut commands: bevy::prelude::Commands,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    mut select: bevy::prelude::ResMut<select::SmashSelect>,
    roster: Option<bevy::prelude::Res<MatchParticipantRoster>>,
    mut cursors: bevy::prelude::ResMut<select_screen::cursor::SelectCursors>,
    mut page: bevy::prelude::ResMut<select_screen::SelectPage>,
    mut start: bevy::prelude::ResMut<select_screen::StartRequested>,
    // WHICH ARRIVAL THIS ALREADY RAN FOR.
    mut done_for: bevy::prelude::Local<
        Option<ambition_platformer2d::game_shell::ShellActivationId>,
    >,
) {
    if !on_the_select_route(&router) {
        return;
    }
    let arrival = router.active.as_ref().map(|active| active.activation_id);
    if *done_for == arrival {
        return;
    }
    *done_for = arrival;

    *select = select::SmashSelect::default();
    *cursors = select_screen::cursor::SelectCursors::default();
    *page = select_screen::SelectPage::default();
    *start = select_screen::StartRequested::default();

    // A roster published by another experience is not ours to remove.
    if roster.is_some_and(|roster| roster.is_published_by(SMASH_EXPERIENCE)) {
        commands.remove_resource::<MatchParticipantRoster>();
    }
    commands.insert_resource(ambition_platformer2d::input::SessionSeatingSource::pending(
        SMASH_EXPERIENCE,
    ));
}

/// Spawn/despawn the select UI from route state. This system owns presentation
/// lifetime only; seat policy and frontend-state initialization live in the two
/// systems above.
fn present_select_screen_ui(
    commands: bevy::prelude::Commands,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    fighters: bevy::prelude::Res<select::SmashRoster>,
    art: select_screen::ScreenArt,
    host: Option<bevy::prelude::Res<ambition_platformer2d::game_shell::ShellHostConfiguration>>,
    existing: bevy::prelude::Query<(), bevy::prelude::With<select_screen::SmashSelectUiRoot>>,
    roots: bevy::prelude::Query<
        bevy::prelude::Entity,
        bevy::prelude::With<select_screen::SmashSelectUiRoot>,
    >,
) {
    if on_the_select_route(&router) {
        select_screen::spawn_select_screen(
            commands,
            existing,
            fighters,
            art,
            select_screen::exit_leads_somewhere(host.as_deref()),
        );
    } else {
        select_screen::despawn_select_screen(commands, roots);
    }
}

/// Claim input for the seats this screen drives, while it is up.
///
/// without this the screen was a surface nothing arbitrated. With the
/// universal pause menu open OVER it, the arrows drove BOTH — the menu's cursor
/// and the CPU count — because the two read different channels
/// (`MenuControlFrame` and `SeatMenuFrames`) and neither could consume the
/// other's edge.
///
/// It names an input CONTEXT — vocabulary the facade already exposes — and the pause menu's
/// higher-priority capturing claim does the rest. Neither side knows the other exists.
fn declare_the_select_input_context(
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    mut participants: bevy::prelude::Query<
        &mut ambition_platformer2d::input::participant::ParticipantContexts,
        bevy::prelude::With<ambition_platformer2d::input::InputParticipant>,
    >,
) {
    let on_select = on_the_select_route(&router);
    for mut contexts in &mut participants {
        // Touch the component only when the claim actually moves.
        if contexts.is_declared(ambition_platformer2d::input::SELECT_CONTEXT) != on_select {
            contexts.sync(
                ambition_platformer2d::input::participant::ContextClaim::capturing(
                    ambition_platformer2d::input::SELECT_CONTEXT,
                    ambition_platformer2d::input::participant::context_priority::SELECT,
                ),
                on_select,
            );
        }
    }
}

/// Is the select screen the active route?
///
/// One answer, three askers — the context claim, the cue, and the "may I drive" gate.
fn on_the_select_route(router: &ambition_platformer2d::game_shell::ShellRouter) -> bool {
    router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_SELECT_ROUTE)
}

/// Publish this screen's submit verb while it is up.
///
/// `sync` rather than a declare/retract pair, so LEAVING retracts. A cue left
/// behind outlives its surface, and the next screen inherits a prompt telling
/// the player to choose a fighter.
fn publish_the_select_ui_cue(
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    mut cues: bevy::prelude::ResMut<ambition_platformer2d::input::ActiveUiCues>,
) {
    cues.sync(
        ambition_platformer2d::input::UiCue {
            context: ambition_platformer2d::input::SELECT_CONTEXT,
            priority: ambition_platformer2d::input::participant::context_priority::SELECT,
            // What the cursor does wherever it is parked: take a role, take a
            // fighter, press START. "Choose" is the one verb true of all three.
            submit_label: "Choose".to_owned(),
        },
        on_the_select_route(&router),
    );
}

/// Is this screen the one the presses belong to?
///
/// without this the screen was a surface nothing arbitrated. With the
/// universal pause menu open OVER it, the arrows drove BOTH — the menu's cursor
/// and the lobby — because the two read different channels (`MenuControlFrame`
/// and `SeatMenuFrames`) and neither could consume the other's edge.
///
/// it asks whether ANY seat still owns `SELECT_CONTEXT`, not whether seat 0
/// does. There is one cursor and four people may drive it, so the screen
/// stops when the whole screen is outranked and not when player one's claim
/// happens to be the one that lost. `None` (no resolver installed, as in a bare
/// unit fixture) reads as owned: a test that wires no contexts is testing the
/// screen, not the arbitration.
fn the_select_screen_owns_its_input(
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    contexts: Option<bevy::prelude::Res<ambition_platformer2d::input::SeatInputContexts>>,
) -> bool {
    if !on_the_select_route(&router) {
        return false;
    }
    contexts.as_deref().is_none_or(|contexts| {
        (0..select::MAX_SMASH_SEATS as u8).any(|seat| {
            contexts
                .for_seat(seat)
                .allows(ambition_platformer2d::input::SELECT_CONTEXT)
        })
    })
}

/// Publish the decided roster and leave the select screen.
///
/// Runs on `Update`, not the sim schedule: choosing a fighter is shell
/// lifecycle, and the sim is not even running yet — the stage has no session
/// until the route this system requests actually resolves.
///
/// it waits for START to be CLICKED, where the previous version left the
/// instant `ready()` became true. Two reasons, and the second is the one that
/// mattered: the real thing has a ready button, and a screen that launches on
/// the frame its last token lands is a screen nobody can photograph. Every
/// attempt to capture a decided lobby photographed the match instead.
fn start_the_battle_when_asked(
    mut commands: bevy::prelude::Commands,
    select: bevy::prelude::Res<select::SmashSelect>,
    asked: bevy::prelude::Res<select_screen::StartRequested>,
    fighters: bevy::prelude::Res<select::SmashRoster>,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    roster: Option<bevy::prelude::Res<MatchParticipantRoster>>,
    // WHAT THIS SCREEN'S SOURCE NUMBERS MEAN. A slot's occupant is an index
    // into the sources the screen offered, and whether index zero is the
    // keyboard or the first pad is the policy's answer — the same one
    // `source_name_under` labels the slot with. Reading it here is what stops
    // the roster and the label disagreeing about who is holding what.
    assignment: bevy::prelude::Res<ambition_platformer2d::input::LocalSeatOffer>,
    // WHO ALREADY HAS A REPERTOIRE, so a seat whose character authors its own
    // moves is not handed this stage's generic kit.
    // `Option`, like every other reader of the cast.
    prepared: Option<
        bevy::prelude::Res<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>,
    >,
    // THE STAGE'S OWN DECLARATION, read rather than re-stated. `Option` because this system
    // runs before the resource exists on the very first frame of a boot, and a screen with no rules
    // yet has no floor to hand out either.
    mut shell: bevy::prelude::MessageWriter<ambition_platformer2d::game_shell::ShellCommand>,
) {
    if !asked.0 {
        return;
    }
    // Only from the select screen. Without this the system would re-fire during
    // the match — `ready()` stays true while the roster stands — and ask the
    // shell to re-enter the stage on every frame of the fight.
    let on_select = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_SELECT_ROUTE);
    if !on_select || roster.is_some() {
        return;
    }
    // THE SEED FOR THIS MATCH'S RANDOM SQUARES.
    //
    // ADR 0023: no ambient RNG. This is the shell ACTIVATION this select
    // screen is running under — a monotonic id minted per route entry — so two
    // visits to the screen draw differently and one visit draws the same thing
    // twice if it somehow started twice. Mixed with the participant count so a
    // three-way and a two-way opened from the same visit do not walk the same
    // sequence.
    //
    // NOT the wall clock, and not a thread RNG. A match is decided in
    // `Update`, but everything it produces is read inside the rollback window,
    // and "where did this fighter come from" must have an answer that survives a
    // replay.
    let seed = router
        .active
        .as_ref()
        .map_or(0, |active| active.activation_id.0 as u64)
        .rotate_left(17)
        ^ select.participating() as u64;
    let declared_rules = smash_declared_combat_rules();

    let Some(decided) = select.roster_seeded(
        &fighters,
        seed,
        assignment.policy(),
        // The ids whose CHARACTER states its own move timelines. Computed here
        // because only this side can see the prepared cast.
        &prepared
            .as_deref()
            .map_or_else(Default::default, |registry| {
                registry
                    .iter()
                    .filter(|(_, definition)| definition.authored_moveset.is_some())
                    .map(|(id, _)| id.to_string())
                    .collect()
            }),
        // ⛔ THE VALUE, NOT A RESOURCE READ. This was once read off
        // `DeclaredCombatRules` and the swipe never arrived: the same system
        // inserts that resource fifty lines below and `Commands::insert_resource`
        // is DEFERRED, so on the frame that decides the match it did not exist
        // yet and `None` was published.
        Some(smash_seating_melee()),
    ) else {
        return;
    };
    // THE SEAT COUNT THIS MATCH DECIDED, published with the roster and
    // under this experience's name. Devices are not participants — a keyboard
    // seat has no controller entity, a spare pad may not be playing, a CPU seat
    // has none at all — so a session sized from what is plugged in is sized
    // wrong for every lobby that seats a CPU. Both land in the same flush that
    // asks for the route, so the session, which is built at least a frame later,
    // has never seen a smash gameplay world without them.
    commands.insert_resource(ambition_platformer2d::input::SessionSeatingSource::decided(
        SMASH_EXPERIENCE,
        // A CPU is a participant and occupies no channel; a lobby of two CPUs needs none at
        // all, which is the case that makes the difference impossible to ignore.
        //
        // and the whole PLAN, not the count of it. A count sizes the
        // session and leaves every consumer to guess which controller feeds
        // each handle — which they did, from the lobby's SPARSE source
        // numbers, so seating the CPU first put the human's fighter on a
        // channel the session never opened.
        decided.local_channel_plan(),
    ));
    commands.insert_resource(decided);
    commands.insert_resource(declared_rules);
    // so this is emphatically NOT a change to `insert_gamepad_bindings`.
    // A=Jump stays right for Ambition; a fighting game says otherwise for the
    // duration of its own experience, and gives the pad back on the way out.
    // Same declare-don't-borrow shape as the rules above, owner and all —
    // the versus route is another provider in the same binary that could
    // eventually declare its own.
    //
    // this is also the ONLY thing that gives gamepad-Special a button.
    // The default pad is fully assigned (`presets.rs` refuses to double-bind),
    // so Special was keyboard- and touch-only; a layout PERMUTES an assigned
    // pad, which is exactly the freedom an addition does not have.
    commands.insert_resource(ambition_platformer2d::input::DeclaredBindingLayout::new(
        SMASH_EXPERIENCE,
        ambition_platformer2d::input::BindingLayout::Smash,
    ));
    // the default reads well for a platformer and badly for a fighter, and
    // that is the whole reason this is a knob. Mary-O has a handful of
    // signature techniques, so naming her button "Spin Dash" tells the player
    // something true and stable. A smash fighter's Attack slot hosts a dozen
    // moves selected by stick direction and posture, so the same rule names
    // whichever one happens to be resolvable where the body is standing — a
    // label that changes as you walk and never answers the question the prompt
    // exists for, which is WHICH BUTTON.
    //
    // "at least not yet" — the move-naming road stays live and default
    // everywhere else; this only declines it here.
    commands.insert_resource(ambition_platformer2d::sim_view::PromptNaming::ByButton);
    shell.write(ambition_platformer2d::game_shell::ShellCommand::GoTo(
        ambition_platformer2d::game_shell::ShellRouteId::new(SMASH_GAMEPLAY_ROUTE),
    ));
}

/// Leave the lobby through the character-select screen's own Back affordance.
///
/// to title, you can only do this if you start a match."* There are TWO useful
/// roads now, and they should stay distinct: Esc/Start opens the universal
/// system menu, whose `Quit to Title` row is available on frontend subroutes as
/// well as live sessions; this handler is the CSS-native Back / held-B route.
/// Both emit the same host-relative `QuitToHome` command.
///
/// Spelling a `GoTo(some_title_route)` here would be this demo naming a route it does not own,
/// and it would be wrong in the next composition.
///
/// nothing to unwind by hand, and that is a claim worth stating. What this
/// route CLAIMED on arrival is released by the systems that claimed it, because
/// each is keyed on the route rather than on a shutdown hook:
/// `maintain_smash_local_seat_offer` releases its seat claim and
/// `present_select_screen_ui` despawns the UI the moment the route is no
/// longer active; `declare_the_select_input_context` retracts `SELECT_CONTEXT`;
/// `publish_the_select_ui_cue` retracts the cue; and the experience scope
/// declared in [`SmashExperiencePlugin`] resets `SmashSelect`,
/// `StartRequested`, [`select_screen::LeaveRequested`] and the cursor, and
/// releases this experience's `SessionSeatingSource` hold. A lobby that was only
/// half joined publishes NO `MatchParticipantRoster` — that is written by
/// `start_the_battle_when_asked` and by nothing else — so there is no match
/// state to strand.
fn leave_the_select_screen_when_asked(
    mut asked: bevy::prelude::ResMut<select_screen::LeaveRequested>,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    // Optional for the same reason every other shell reader here is: a bare unit
    // fixture composes no host, and `exit_leads_somewhere` reads that as "no way
    // out" rather than inventing one.
    host: Option<bevy::prelude::Res<ambition_platformer2d::game_shell::ShellHostConfiguration>>,
    mut shell: bevy::prelude::MessageWriter<ambition_platformer2d::game_shell::ShellCommand>,
) {
    if !asked.0 {
        return;
    }
    // spend the request WHATEVER happens next. A latch that says "leave"
    // and survives its own frame is the shape `StartRequested` is reset on
    // arrival to avoid — one left standing re-fires on the next route this
    // system happens to run under. Cleared before the refusals below, never
    // after them.
    asked.0 = false;
    if !on_the_select_route(&router) {
        return;
    }
    if !select_screen::exit_leads_somewhere(host.as_deref()) {
        return;
    }
    shell.write(ambition_platformer2d::game_shell::ShellCommand::QuitToHome);
}

/// The experience: what a launcher lists and a player can enter.
///
/// Until this existed the demo was three correct pieces nobody could reach — a
/// roster, a stage and a ruleset, all unit-true and unassembled. A slice that
/// stops one step short of bootable is the shape this repo keeps catching:
/// everything passes and nothing runs.
pub struct SmashExperiencePlugin;

impl bevy::prelude::Plugin for SmashExperiencePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        install_smash_content(app);
        // BEFORE the authoring below, which advertises the select screen as this
        // experience's entry and refuses a route nobody has registered. The
        // ordering is load-bearing and the refusal says so by name.
        app.add_plugins(SmashSelectPlugin);
        ambition_platformer2d::provider::PlatformerExperienceAuthoring::new(
            SMASH_EXPERIENCE,
            SMASH_GAMEPLAY_ROUTE,
            "Smash",
            "Stocks, a platform, and nothing underneath it",
            "Prepare Smash",
            // No `.with_procedural_sfx()`: this stage declares SILENCE and the
            // fighters bring their own cues. Claiming procedural sfx it never
            // registers would be the same shape as the empty function above —
            // a declaration with nothing behind it.
            ambition_platformer2d::provider::AuthoredCatalogFragments::new(
                SMASH_CHARACTER_ID,
                SMASH_EXPERIENCE,
            ),
        )
        // A LAUNCHER ROW LEADS TO THE QUESTION, NOT TO THE STAGE. Without
        // this the only way into the select screen was to make it a whole app's
        // home route — which is what the demo's own shell does and no
        // multi-game host can, because its home lists games. Selecting "Smash"
        // in the Ambition title screen would have dropped a lone duelist onto
        // the platform with nobody to fight.
        // THE STAGE'S OWN READOUTS. Without this the route inherited
        // Ambition's adventure HUD and drew a health bar, a mana bar and a money
        // counter over a platform fighter. Four slots
        // because the stage seats four; a 1v1 fills two and the publisher clears
        // the rest, the same rule the versus stage states.
        .with_hud({
            let mut hud = ambition_platformer2d::presentation::HudDeclaration::new();
            for (seat, slot) in FIGHTER_HUD_SLOTS.iter().enumerate() {
                hud = hud.slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(*slot)
                        .with_region(ambition_platformer2d::presentation::SurroundRegion::Bottom)
                        .with_font_size(22.0)
                        .with_min_px(ambition_platformer2d::engine_core::Vec2::new(220.0, 30.0))
                        // Coloured by seat parity, so a partner's meter reads as
                        // a partner's at a glance.
                        .with_color(if seat % 2 == 0 {
                            [0.55, 0.85, 1.0, 1.0]
                        } else {
                            [1.0, 0.6, 0.55, 1.0]
                        }),
                );
            }
            hud.slot(
                ambition_platformer2d::presentation::HudSlotSpec::new(SMASH_ANNOUNCE_HUD_SLOT)
                    .centered()
                    .with_font_size(34.0)
                    .with_color([1.0, 0.85, 0.3, 1.0]),
            )
        })
        .entered_at(SMASH_SELECT_ROUTE)
        .with_loading_activity(
            ambition_platformer2d::load_presentation::DETERMINISTIC_LOADING_ACTIVITY_ID,
        )
        .with_defense_presentation(
            ambition_platformer2d::presentation::DefensePresentationPolicy::shared_iframe_blink(),
        )
        .install(app, smash_prepared_session_world);
        app.add_plugins(SmashRulesPlugin::hosted());

        // WHAT THIS EXPERIENCE OWNS, AND WHAT LEAVES WITH IT.
        //
        // `covering` the select screen is load-bearing. The lobby and the
        // match are two shell experiences of one provider, and the lobby
        // publishes the roster FOR the match — a scope that named only the
        // gameplay id would delete it on the way in.
        {
            use ambition_platformer2d::game_shell::ShellExperienceScopeAppExt;
            app.experience_owns(SMASH_EXPERIENCE)
                .covering(SMASH_SELECT_EXPERIENCE)
                // By OWNER: another game stages its own cast into this same
                // resource, and removing it by type would delete their match.
                .releasing_owned::<MatchParticipantRoster>(|roster, owner| {
                    roster.is_published_by(owner.as_str())
                })
                // A match that ended with the route it ran on. Left standing, it
                // is the next game's seating refusing to run because a match is
                // already "live".
                //
                // two different questions, and both are needed. The
                // session id ( so a finished match cannot be
                // rebuilt by its own activation) says WHICH ACTIVATION of one
                // game; the witness here says WHICH GAME, which is the only one
                // that matters when two providers share a host.
                .releasing_witnessed::<
                    ambition_platformer2d::actors::character_runtime::ActiveMatch,
                    ambition_platformer2d::actors::character_runtime::PreparedMatch,
                >(|plan, owner| plan.is_published_by(owner.as_str()))
                // declared AFTER the activation above, which reads it as its
                // witness: releases run in declaration order.
                .releasing_owned::<
                    ambition_platformer2d::actors::character_runtime::PreparedMatch,
                >(|plan, owner| plan.is_published_by(owner.as_str()))
                // AND THE RULES LEAVE WITH THE MATCH. Removing the
                // declaration IS the exit (AE6) — the projection folds it over
                // the world's baseline every tick and writes nothing back, so
                // there is no restore to skip. Left standing, this stage's DI
                // budget would follow the player into Ambition's PvE, which
                // answers `0.0` on purpose.
                //
                // `releasing_owned`, not `resetting`: every reader takes it
                // as `Option<Res<_>>`, so absence is the meaningful "no
                // declaration" answer — and the OWNED form is what keeps two
                // stages that both declare rules from deleting each other's.
                .releasing_owned::<
                    ambition_platformer2d::combat::rules::DeclaredCombatRules,
                >(|rules, owner| rules.is_declared_by(owner.as_str()))
                // AND THE PAD GOES BACK TO NORMAL. Removing the declaration
                // IS the exit, exactly like the rules above: the layout is a
                // layer inside `BindingRecipe::build`, so the next rebuild
                // returns every seat to the base preset with nothing to restore.
                //
                // this release is the whole difference between "a profile" and "we changed the
                // defaults".
                .releasing_owned::<
                    ambition_platformer2d::input::DeclaredBindingLayout,
                >(|layout, owner| layout.is_declared_by(owner.as_str()))
                // Restart resources in place: systems require them as `ResMut`, but
                // they must not retain the previous match's state.
                .resetting::<select::SmashSelect>()
                .resetting::<select_screen::StartRequested>()
                // The same rule one latch over: a "leave" that outlived the
                // lobby would ask the NEXT experience's first frame to quit.
                .resetting::<select_screen::LeaveRequested>()
                .resetting::<select_screen::cursor::SelectCursors>()
                .resetting::<select_screen::SelectPage>()
                .releasing_with("SessionSeatingSource", |world, owner| {
                    if let Some(mut seating) = world.get_resource_mut::<
                        ambition_platformer2d::input::SessionSeatingSource,
                    >() {
                        seating.release(owner.as_str());
                    }
                });
        }
    }
}

/// Maximum directional influence on launch angle, in radians.
const SMASH_DI_MAX_ANGLE: f32 = 0.31;

/// Fraction of base launch added per point of victim damage. This is a Smash
/// game rule; the shared PvE movement baseline does not scale knockback this way.
/// Public so roster-wide validation can check every authored fighter moveset.
pub const SMASH_KNOCKBACK_GROWTH: f32 = 0.02;

/// Stable ids the shell routes and lists this demo by.
pub const SMASH_EXPERIENCE: &str = "smash";
pub const SMASH_GAMEPLAY_ROUTE: &str = "smash_gameplay";
/// Where the demo STARTS.
///
/// Not the stage. A platform fighter that opens on the stage has already decided
/// who you are, and the whole point of up-to-four-players is that it has not.
///
/// It is the demo app's HOME route (leaving a match returns to the screen that
/// chose it) AND the ENTRY route this experience advertises to any launcher, so
/// a multi-game host's "Smash" row opens the same question rather than dropping
/// a lone duelist onto the platform.
pub const SMASH_SELECT_ROUTE: &str = "smash_select";
/// The select screen is its OWN shell experience, and it has to be.
///
/// Not `smash`: an activation carrying the gameplay experience id starts a gameplay SESSION, and
/// this screen has no prepared world to activate — the shell would panic with *"requires an exact
/// prepared-session publication"* before a single panel drew. A screen a provider draws itself is a
/// frontend experience of its own.
pub const SMASH_SELECT_EXPERIENCE: &str = "smash.select";
/// The fighter a lone visitor wears. The MATCH seats its own cast from the
/// roster; this is who is standing there before one starts.
pub const SMASH_CHARACTER_ID: &str = "smash_duelist_a";
/// The opponent.
pub const SMASH_OPPONENT_ID: &str = "smash_duelist_b";

/// The logician.
pub const SMASH_GEORGE_BOOUL: &str = "smash_george_booul";

// THE ONE FIGHTER THIS DEMO ADDS TO THE CROSSOVER.
//
// he wears a sheet that ALREADY SHIPS and that no other catalog claims, which
// is the only kind of fighter this demo may declare: the rest of the grid is
// Ambition's own cast and the other demos' protagonists, named by ID in
// `select::SMASH_ROSTER` rather than copied here. The two robot rows below are
// STAND-INS for the lineage the content catalog owns; see `select::STAND_INS`.
//
// every fighter shares one kit. See `SmashSelect::roster` — one ability
// set, one brain, one action set. Different LOOKS and one game. Per-character
// movement, reach and weight is the obvious next question and is deliberately
// not this one; a roster where the choice already changed the match would have
// made the select screen impossible to judge on its own terms.

/// This demo authors its own fighters so it depends only on the public facade.
/// Cross-game roster composition belongs in the host, where both catalogs exist.
const SMASH_CATALOG_RON: &str = r#"(
    autonomous_profiles: {
        // THE STAGE'S CPU POLICY, PUBLISHED.
        // A CPU seat named `duelist` and the match resolved it through
        // `CharacterRoster` — an enemy ARCHETYPE table — so the controller half
        // of `character + controller + team` was arriving by way of a body
        // definition. This is what a controller policy IS.
        //
        // the numbers are the archetype row's controller half verbatim.
        "duelist": (
            template: Fighter,
            aggro_radius: 600.0,
            attack_range: 48.0,
            patrol_effort: 1.0,
            chase_effort: 1.0,
            fighter_level: 5,
        ),
        // that is the whole thesis in six rows: a difficulty setting is a CONTROLLER fact, and
        // stating it required declaring a whole creature.
        "duelist_l1": (
            template: Fighter, aggro_radius: 600.0, attack_range: 48.0,
            patrol_effort: 1.0, chase_effort: 1.0, fighter_level: 1,
        ),
        "duelist_l3": (
            template: Fighter, aggro_radius: 600.0, attack_range: 48.0,
            patrol_effort: 1.0, chase_effort: 1.0, fighter_level: 3,
        ),
        "duelist_l5": (
            template: Fighter, aggro_radius: 600.0, attack_range: 48.0,
            patrol_effort: 1.0, chase_effort: 1.0, fighter_level: 5,
        ),
        "duelist_l6": (
            template: Fighter, aggro_radius: 600.0, attack_range: 48.0,
            patrol_effort: 1.0, chase_effort: 1.0, fighter_level: 6,
        ),
        "duelist_l9": (
            template: Fighter, aggro_radius: 600.0, attack_range: 48.0,
            patrol_effort: 1.0, chase_effort: 1.0, fighter_level: 9,
        ),
        // THE TRAINING TARGET, AND IT IS A POLICY LIKE ANY OTHER.
        //
        // ⛔⛔ A SEAT WITH NO DRIVER IS REFUSED, on purpose: `Cpu { brain_profile:
        // None }` cannot be told apart from a brain that failed to install. So
        // "stands there and takes it" is stated as a controller policy, and a
        // fighter seated on it is an ordinary staged body — damageable, launchable,
        // and subject to every rule — that makes no decisions.
        //
        // ⭐ Named beside the ladder because it belongs to the same axis: what
        // this seat DOES is a controller fact, and zero is a rung.
        "stand_still": (
            template: StandStill,
            aggro_radius: 0.0,
            attack_range: 0.0,
            patrol_effort: 0.0,
            chase_effort: 0.0,
        ),
    },
    brain_presets: {
        "stand_still": StandStill,
        // The FB4b fighter brain, selected from content. Until
        // there was no `BrainPreset` variant for it, so the rig existed and no
        // catalog row could ask for it — the demo's duelists stood still because
        // standing still was the only thing they could be told to do.
        "duelist": Fighter(level: 5),
    },
    action_set_presets: {
        "duelist": (
            move_style: Walk,
            // A real swipe, not a placeholder: the whole point of the stage is
            // that a hit LAUNCHES, and a fighter with no melee cannot knock
            // anybody off anything.
            melee: Some(Swipe(
                windup_s: 0.22,
                active_s: 0.08,
                recover_s: 0.26,
                damage: 4,
                reach_px: 34.0,
            )),
            ranged: None,
            special: None,
        ),
    },
    characters: {
        "smash_duelist_a": (
            // NOT "Duelist A". It wears `player_robot_v3`'s
            // sheet and is a STAND-IN for that character in compositions that do
            // not carry it; naming it anything else pretended it was somebody
            // new. Distinct from the content catalog's "Player Robot v3",
            // because the assembled catalog refuses two rows sharing a name.
            display_name: "Robot v3",
            spritesheet: "sprites/player_robot_v3_spritesheet.png",
            manifest: "sprites/player_robot_v3_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "duelist",
            default_action_set: "duelist",
            tags: ["player", "smash"],
            fallback_dialogue: ["Off the edge is the only way out."],
        ),
        "smash_duelist_b": (
            display_name: "Robot v2",
            spritesheet: "sprites/player_robot_v2_spritesheet.png",
            manifest: "sprites/player_robot_v2_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "duelist",
            default_action_set: "duelist",
            tags: ["smash"],
            fallback_dialogue: ["Percent is not health. I learned that the hard way."],
        ),
        "smash_george_booul": (
            display_name: "George Booul",
            spritesheet: "sprites/george_booul_spritesheet.png",
            manifest: "sprites/george_booul_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "duelist",
            default_action_set: "duelist",
            tags: ["smash"],
            fallback_dialogue: ["Either you are on the stage or you are not."],
        ),
    },
)"#;

/// Register this demo's content.
///
/// thin, but not empty — and the difference is a refusal that fired. The
/// fighters are Ambition's own robot lineage, which is the point of a crossover
/// stage: a demo that authored its own duelists would prove the stocks loop
/// against content nobody else has, and the interesting claim is that it works
/// on the cast the game already ships. So there is no character to register.
///
/// There is still AUDIO to declare. Preparation refuses an experience whose
/// provider registered no audio fragment, and this function being empty is
/// exactly what that refusal is for — the shell panicked with *"frontend audio
/// provider 'smash' registered no audio fragment"* on its first boot. Declaring
/// SILENCE is a registration, not the absence of one: the fighters bring their
/// own cues, which is what a crossover stage means.
fn install_smash_content(app: &mut bevy::prelude::App) {
    use ambition_platformer2d::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
    use ambition_platformer2d::characters::actor::character_catalog::{
        CharacterCatalogAppExt, CharacterCatalogFragment,
    };

    app.register_character_catalog_fragment(
        CharacterCatalogFragment::from_ron(
            SMASH_EXPERIENCE,
            Some(SMASH_CHARACTER_ID),
            SMASH_CATALOG_RON,
        )
        .expect("the smash character catalog is valid"),
    );
    // REGISTER the characters, not only their catalog rows. A catalog
    // fragment declares what a character IS; registration is what makes the art
    // pipeline know it exists — `declare_registered_characters` reads the
    // PREPARED REGISTRY, so a catalog-only character draws the marked
    // placeholder. Pocket shipped that way and nobody noticed until somebody
    // looked at the screen.
    {
        use ambition_platformer2d::actors::character_runtime::CharacterDefinitionAppExt;
        use ambition_platformer2d::character::CharacterDefinition;
        // EVERY id this demo can SEAT, not just the two it opens with.
        // A catalog row declares what a character IS; registration is what makes
        // it spawnable, and the comment above says what a catalog-only character
        // draws. `smash_george_booul` was added to the grid and left off this
        // list for one commit, and the tell was a stocks fighter that never
        // seated — not a missing sprite.
        for (id, name, sheet) in [
            (SMASH_CHARACTER_ID, "Robot v3", "player_robot_v3"),
            (SMASH_OPPONENT_ID, "Robot v2", "player_robot_v2"),
            (SMASH_GEORGE_BOOUL, "George Booul", "george_booul"),
        ] {
            let definition = CharacterDefinition::new(id, name, SMASH_EXPERIENCE).with_sheet(sheet);
            // THE PERCENT REFERENCE IS NOT WRITTEN HERE ANY MORE
            // .
            //
            // A character that authors no vitals gets a ONE-HIT pool, and `damage_percent()` is
            // `accumulated / max`, so a 140-damage hit read as 14000%.
            //
            // what 100% means is a rule of the MATCH, so
            // `apply_smash_match_rules` declares it and seating applies it to
            // every seat — see `MatchParticipantRoster::fighter_health_pool`.
            // These three now author what they ARE and nothing about how a
            // stocks match reads them.
            //
            // that move implies NO direction. Whether per-character
            // per-game properties belong to the character or to the game is
            // deliberately still open; the seam exists so the answer is one edit
            // either way.
            let mut definition = smash_reading_of_character(definition);
            // Six numbers stood on this line — `slash_recoil: 0.0`, a
            // three-frame jump squat, the air-dodge window and a 500 px/s tumble
            // floor. Every one was right and none of them could reach the other
            // eleven fighters, and two of the three ids this loop registers are
            // STAND-INS that the composed host drops — so on the shipped host
            // they reached exactly ONE fighter (George), and `player_robot_v3`
            // fought with the exploration protagonist's melee recoil and no air
            // dodge at all. What a fighter's body is on THIS STAGE is a rule of
            // the MATCH, so `apply_smash_match_rules` declares
            // [`SMASH_FIGHTER_BODY`] once and seating composes it onto every
            // seat — see `MatchParticipantRoster::fighter_body`.
            //
            // Deleting the line outright made George floaty and sluggish, and the smash app's
            // own repertoire probes caught it in one run (three distinct moves out of sixteen,
            // and no recovery thrown in 1800 ticks).
            //
            // so it is stated deliberately now, as the one thing it means.
            // and it is a FINDING, not a resolution: eleven of the fourteen
            // fighters on the grid still play on the ACTOR baseline — a
            // levelled stage where thirteen bodies are floatier than the
            // fourteenth is half a decision, and which base a platform fighter
            // uses is a product call rather than a side effect of this commit.
            // Filed for a later slice.
            definition.movement_tuning = Some(ambition_platformer2d::engine_core::DEFAULT_TUNING);
            // WHAT THIS FIGHTER'S BODY CAN DO — authored on the CHARACTER,
            // which is why the shield, the dodge and the ledge exist in this
            // demo at all.
            //
            // the machinery was all already there and unreachable. The engine
            // has a bubble shield with a parry window, a grounded dodge roll
            // with i-frames, and a full ledge system (grab / hang / climb /
            // roll / getup attack / jump / drop / regrab cooldown) — and none of
            // the fighters ran any of it, because a capability had exactly one
            // authoring surface, the enemy ARCHETYPE, and these three seat
            // through `combatant`. The match then stamped one flat set over
            // every body, so what a fighter could do was a property of the
            // MATCH. Three verbs were simply missing from that set and nothing
            // could add them per character.
            //
            // `fly`/`blink` deliberately absent: this is a platform fighter's
            // ground game, not the exploration protagonist's traversal kit, and
            // the July measurement of two seats disagreeing was exactly a
            // duelist meeting a body that could fly. `dash` left for the same
            // reason — see [`SMASH_FIGHTER_KIT`], which this must
            // keep agreeing with or the stage's ceiling silently trims it.
            definition =
                definition.with_abilities(ambition_platformer2d::engine_core::AbilitySet {
                    move_horizontal: true,
                    jump: true,
                    variable_jump: true,
                    double_jump: true,
                    fast_fall: true,
                    attack: true,
                    pogo: true,
                    directional_primary: true,
                    // The three the flat match set could never grant.
                    shield: true,
                    dodge: true,
                    ledge_grab: true,
                    ..ambition_platformer2d::engine_core::AbilitySet::NONE
                });
            // THE REPERTOIRE, ON THE CHARACTER.
            //
            // this is what stops the seat needing `smash_fighter_kit()`: a definition that authors
            // its own moveset says something more specific than anything derivable from an
            // action-set preset, and preparation uses it verbatim. George is the one fighter this
            // demo owns, and he is the one who gets authored.
            definition = definition.with_moveset(if id == SMASH_GEORGE_BOOUL {
                crate::george_booul_moveset::george_booul_moveset()
            } else {
                crate::moveset::fighter_moveset()
            });
            app.register_character(definition);
        }
    }
    app.register_audio_catalog_fragment(
        AudioCatalogFragment::new(
            SMASH_EXPERIENCE,
            Some(ambition_platformer2d::audio::spec::MusicRegistry {
                default_track: SMASH_STAGE_TRACK.to_string(),
                tracks: SMASH_TRACKS
                    .iter()
                    .map(
                        |(id, display)| ambition_platformer2d::audio::spec::MusicTrack {
                            id: (*id).to_string(),
                            display_name: (*display).to_string(),
                            asset_path: Some(format!("audio/music/generated/{id}/full.ogg")),
                            one_shot: false,
                        },
                    )
                    .collect(),
            }),
            // Still no SFX registry: the stage declares silence and the
            // FIGHTERS bring their own cues. Claiming procedural sfx it never
            // registers would be a declaration with nothing behind it.
            None,
        )
        .expect("the smash audio fragment is valid"),
    );
}

/// The stage, as the shared preparation lifecycle wants it.
fn smash_prepared_session_world() -> ambition_platformer2d::runtime::PreparedPlatformerSource {
    use ambition_platformer2d::runtime::demo_fixture::{
        ActiveRoomMetadata, RoomSet, StartingCharacter,
    };

    let room = smash_stage();
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    // The match realizes its own cast; the id below is only this experience's catalog DEFAULT,
    // which its worn fighters still fall back to.
    ambition_platformer2d::runtime::PreparedPlatformerSource::for_match(
        SMASH_EXPERIENCE,
        RoomSet::from_parts(SMASH_STAGE_ROOM_ID, vec![room], Vec::new()),
        geometry,
        metadata,
        StartingCharacter::new(SMASH_CHARACTER_ID),
    )
}

#[cfg(test)]
mod pause_arbitration_tests;
#[cfg(test)]
mod tests;
