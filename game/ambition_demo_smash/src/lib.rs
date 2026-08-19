//! **The stocks demo — a platform fighter where the world does the killing.**
//!
//! Jon, 2026-07-31: *"stocks are more important. maybe start the smash demo.
//! versus can be a generic fighter demo and test things smash doesn't."* So the
//! shipped versus stage keeps its ROUNDS settled on health — a generic fighter,
//! testing what a health-based ruleset tests — and this is the stocks one.
//!
//! ## What it is for, in order
//!
//! 1. **It is the stocks loop's first real consumer.** `ambition_combat::stocks`
//!    owns the COUNT — spend one, decide whether that was the last, mark the
//!    fighter eliminated, clear the meter. It deliberately does not know where a
//!    body goes or when a match is over, because those need a stage and a
//!    scoreboard. This crate is what supplies them, and the split is only real
//!    once something on the other side of it exists.
//!
//! 2. **It is the E9 oracle for a stocks game.** Like the Sanic demo it depends
//!    on `ambition_platformer2d` + `bevy` and nothing else. If declaring a stocks match needs
//!    a type the umbrella does not re-export, that is an engine leak and it
//!    fails to compile HERE — which is the whole reason a second consumer is
//!    worth its weight.
//!
//! ## Why "the world does the killing" is the entire design
//!
//! A stocks fighter is `DeathPolicy::Unbounded`: its damage meter climbs past
//! 100% and never kills it. What kills it is leaving the stage. That is not a
//! rule this crate implements — the engine's blast-zone gate already owns it,
//! and `BodyKnockedOut` is written from the same `RulesetOwnsDeath` arm that
//! already decided a match rather than the world owns the body's death.
//!
//! What this crate owns is the two answers the engine refuses to guess: WHERE a
//! respawning fighter comes back, and WHAT HAPPENS when one side is left.

// ⚠ **no `ambition_platformer2d::prelude::*`.** Declaring a match needs the ACTOR
// vocabulary, not the room-authoring one, and reaching for the prelude here
// would import nothing this file uses. That the prelude does not cover a match
// is a fact about what a prelude is for, not a gap.
use ambition_platformer2d::actor::{ControllerBinding, MatchParticipant, MatchParticipantRoster};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::Vec2;
use ambition_platformer2d::world::rooms::RoomSpec;

pub mod capture;
pub mod george_booul_moveset;
pub mod moveset;
pub mod select;
pub mod select_screen;
pub mod smash_pack;

/// The game-MODE tag this demo's rules gate on, so they sleep everywhere else.
pub const SMASH_MODE: &str = "smash";

/// Stocks each fighter starts with.
///
/// Three, because it is the smallest number that makes the middle of a match
/// feel different from its start and its end: at three you can lose one and
/// still be playing the same match, which is the thing rounds cannot express.
pub const STARTING_STOCKS: u32 = 3;

/// **What 100% means.**
///
/// The denominator of `damage_percent()`. Under `DeathPolicy::Unbounded` the
/// pool never kills, so this is purely the scale a percent is read against —
/// which is exactly why it has to be declared: an undeclared pool is whatever
/// the CHARACTER authored, and a meter divided by one reports 14000%.
///
/// ⛔⛔ **THE MATCH declares it, not the characters** (queue D131). It was
/// stamped onto the three ids this demo registers until 2026-08-16, which fixed
/// three fighters out of fourteen: everybody else walks onto this stage carrying
/// a pool their own game authored, and Mary-O and Sanic are one-hit-kill
/// platformer protagonists whose games say `1`. See `apply_smash_match_rules`.
pub const SMASH_PERCENT_REFERENCE: i32 = 100;

/// The **published controller policy** a CPU seat asks for — `smash::duelist`,
/// resolved in this stage's own provider.
///
/// ⭐ **and it is now what its name always claimed.** Until 2026-08-11 a
/// `ControllerBinding::Cpu { brain_profile }` was a `CharacterRoster` ARCHETYPE
/// key, so asking for a fighting style meant declaring a whole creature — this
/// demo shipped `SMASH_ROSTER_RON`, six near-identical archetype rows carrying a
/// body no seat had read since a fighter's body came from its character, whose
/// only difference from each other was `fighter_level`. They are deleted; the
/// six are `autonomous_profiles` in the catalog above.
///
/// ⚠ the older bug this doc recorded — CPU seats standing still because the
/// lookup consulted a namespace the catalog did not publish into — had a second
/// life worth remembering: publishing the policy did not fix it, because
/// `seat_brain_profile` resolved a BARE key against a registry keyed
/// `provider::name`. Two vocabularies sharing one word cost the same day twice.
pub const SMASH_DUELIST_BRAIN: &str = "duelist";

/// Where a respawning fighter comes back, above the stage centre.
///
/// ⚠ **above**, not at the spawn point. A fighter that reappears on the floor
/// reappears inside whatever is standing there — and in a fight, what is
/// standing there is the opponent who just knocked it off. Respawn height is the
/// oldest rule in the genre and it is a rule about SAFETY, not about drama.
pub const RESPAWN_HEIGHT_PX: f32 = 160.0;

/// Build the roster for a stocks match between `characters`.
///
/// ⚠ **`fighter_stocks` declares BOTH halves at once** — the count AND
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
        .collect();
    apply_smash_match_rules(&mut roster);
    roster.published_by(SMASH_EXPERIENCE)
}

/// **WHAT KIND OF MATCH THIS IS** — the Smash ruleset, in one place.
///
/// ⛔ **it was in TWO places and they had drifted.** `smash_roster` (the fixed
/// two-fighter rig the tests and the standalone open with) and the select
/// screen's assembled roster each declared stocks, the opening hold and the
/// ability floor separately, with the same forty lines of comment copied
/// between them. Adding a 3–2–1–GO countdown to one of them produced a shipped
/// stage whose fighters were never released — held forever by a hold whose
/// owner only existed on the other path, which is exactly the failure two
/// copies of a rule are for.
pub fn apply_smash_match_rules(roster: &mut MatchParticipantRoster) {
    roster.opens_suspended = true;
    // **THE OPENING CEREMONY: 3 — 2 — 1 — GO.**
    //
    // Three beats at 60Hz. The hold was already here and had nothing to wait
    // for, so it came off on the tick the cast was built and the round began
    // with two fighters already moving before a player had looked at the stage.
    //
    // ⛔ ticks rather than seconds, because the release is a comparison against
    // the sim clock — see `MatchRules::opening_countdown_ticks`.
    roster.opening_countdown_ticks = 3 * 60;
    roster.fighter_stocks = Some(STARTING_STOCKS);
    // **EVERY FIGHTER IN THIS MATCH IS READ AGAINST THE SAME 100%.**
    //
    // ⛔⛔ **the crossover cast does not author its percent, and cannot be asked
    // to** (queue D131, measured through the shipped host 2026-08-16). An
    // authored `max_health` is a statement made under the AUTHORING GAME's
    // rules: Mary-O and Sanic are one-hit-kill platformer protagonists and both
    // author `max_health: 1`, which is exactly right in their own games. Seated
    // here, `damage_percent()` divided ordinary melee damage by ONE — a
    // seven-second match read `mary_o 4200%` and `sanic 800%` beside
    // `player_robot_v3 18%` and `smash_george_booul 9%`, off 42, 8, 11 and 9
    // points of damage. It looked like percent accruing on a clock on half the
    // cast. It was four fighters divided by 1, 1, 60 and 100.
    //
    // ⛔ **and this REPLACES three per-character writes**, which is why it is
    // here and not there. This demo used to stamp the reference onto the three
    // characters it happens to register — a fix that could only ever cover its
    // own fighters, and the roster is fourteen. The pool a percent is read
    // against is a rule of the MATCH, exactly like the stock count above it and
    // the death policy that count implies.
    roster.fighter_health_pool = Some(SMASH_PERCENT_REFERENCE);
    // **EVERY FIGHTER IN THIS MATCH HAS THE SAME VERBS.**
    //
    // ⛔ Measured 2026-08-01, both seats wearing the right duelist:
    //
    //     seat 0 (ADOPTED)     every ability true - fly, blink,
    //                          blink_through_hard_walls, glide, swim, shield
    //     seat 1 (SPAWNED)     move, jump, variable_jump, double_jump, attack
    //
    // Player one fought as the exploration protagonist and player two as a
    // duelist, on the same stage. The touch bezel advertised it (Blink / Fly
    // Toggle / Ranged / Bubble Shield) and was the only honest thing in the
    // picture - it reports what the CONTROLLED SUBJECT can do, and it was right.
    //
    // Seating already levels this and says so in its own comment, found the same
    // way on the VERSUS stage in July: "a SPAWNED seat's abilities come from
    // `AncillaryMovementBundle`; the ADOPTED primary player brought whatever the
    // session granted it". It is gated on the roster DECLARING a set, because
    // "what a fighter may do is a rule of the match" - and this demo declared
    // nothing, so the levelling never ran.
    //
    // ⚠ WHICH verbs is a product call and [`SMASH_FIGHTER_KIT`] is the one place
    // to change it; that the two seats agree is not.
    //
    // ⭐⭐ **IT IS A LEVELLING, AND IT SAYS SO IN ONE WORD** (Jon, 2026-08-16:
    // *"in smash all characters should be sure they are granted the basic smash
    // abilities"*). `MatchAbilities::levelled` GRANTS this kit to every fighter
    // and PERMITS nothing outside it, so the answer does not depend on what a
    // character happened to author.
    //
    // ⛔ **it was a lone MASK for five days and that could not guarantee
    // anything.** A mask can only ever REMOVE, so a character whose kit was
    // written somewhere else arrived here missing verbs the stage thought it had
    // handed out: the Perfect Cellular Automaton's kit is a duel arena's, built
    // on `AbilitySet::basic()`, and it reached this stage with no double jump,
    // no fast fall, no dodge and no ledge grab. Twelve of the fourteen author no
    // kit at all, so nothing exercised the rule and the gap was invisible.
    //
    // ⚠ **and it is not a GRANT either**, which is the trap on the other side —
    // see `MatchAbilities`. The ceiling is what keeps an exploration
    // protagonist's flight and blink out of the fight, and what stops a mode
    // handing back a verb a character deliberately refused.
    //
    // ⭐ the day a fighter should bring its own flavour here — a wall jump on
    // the characters that have one, the way a real platform fighter does — this
    // becomes a `MatchAbilities` whose `permitted` is wider than its `granted`,
    // and nothing else changes.
    roster.fighter_abilities =
        Some(ambition_platformer2d::engine_core::MatchAbilities::levelled(SMASH_FIGHTER_KIT));
    // **AND THE BODY THOSE VERBS RUN ON.**
    //
    // ⛔⛔ **a granted verb whose WINDOW is zero is a DEAD GRANT**, and the line
    // above was handing out exactly that. `dodge` reached all fourteen fighters;
    // `DEFAULT_TUNING.air_dodge_time` is `0.0`, so `available_dodge` fell
    // straight through for every fighter whose character had not authored a
    // fighter's body — which, measured on the composed host on 2026-08-16, was
    // TWELVE of the fourteen (`player_robot_v3` among them: the demo's careful
    // tuning was on `smash_duelist_a`, the STAND-IN the host drops).
    //
    // ⚠ **it did not read as broken until slice 1.** Those twelve had `dash`
    // from the kit, so an airborne burst press fell out of the dodge and into
    // `apply_dash` and they air-dashed. Removing `dash` — correctly — left the
    // press meaning nothing at all.
    //
    // ⭐ the same shape as `fighter_abilities` one line up, one layer down: what
    // a fighter's body IS was a property of the CHARACTER, so a stage could
    // promise a verb and had no way to supply what the verb needs.
    roster.fighter_body = Some(SMASH_FIGHTER_BODY);
}

/// **SMASH'S READING OF A CHARACTER** — a function from what the character
/// AUTHORED to what this match's seat plays with.
///
/// ⭐⭐ **PURE, and that is the requirement** (Jon, 2026-08-16). Two of this
/// ruleset's three adjustments already go through one named composition site —
/// [`apply_smash_match_rules`] declares them and `MatchRules::body_over` /
/// `MatchRules::pool_over` compose them. The third did not: the registration
/// loop in `install_smash_content` reached into `definition.vitals` and ASSIGNED
/// a weight, mid-loop, on the way past. That reach-in is now this function, and
/// grepping the name below finds every place the smash ruleset interprets
/// authored character data.
///
/// ⭐ **the orthogonality this expresses is not new here.** Character authoring
/// and ruleset specificity are independent axes: data may live WITH the
/// character while being owned SEMANTICALLY by the smash capability. Mary-O's
/// move table already works exactly this way — it sits in her own crate, is
/// unreachable in her own game, and speaks smash's vocabulary.
///
/// ⛔⛔ **IT TAKES NO POSITION ON WHERE WEIGHT ULTIMATELY BELONGS.** Jon
/// deliberately deferred that (*"do not design the final universal
/// character/game composition model from one weight customer"*), so this is one
/// customer and one seam and no facet type. What it buys is that the eventual
/// answer — character-owned, game-owned, or composed — is ONE edit either way.
///
/// ⚠ **the authored numbers and their reasoning are unchanged.** Weight is a
/// SPREAD around the reference body rather than three absolute numbers: v3 is
/// the middleweight the stage is tuned against, v2 is the lighter older build,
/// George is the heavy. `scaled_knockback` divides the growth term by the
/// victim's weight, so this is what decides who dies early and who survives to
/// 150% — without it all three seat through `combatant` and weigh the same,
/// which is three of the same fighter (D73 phase 1).
///
/// ⚠ **and it is still only three of fourteen**, for the same reason the percent
/// reference used to be: it can only reach the characters this demo REGISTERS.
/// That is the deferred question, stated rather than fixed.
pub fn smash_reading_of_character(
    definition: ambition_platformer2d::actors::character_runtime::CharacterDefinition,
) -> ambition_platformer2d::actors::character_runtime::CharacterDefinition {
    use ambition_platformer2d::actors::character_runtime::{CharacterDefinition, Vitals};
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

/// **THE PLATFORM-FIGHTER BODY** — the movement feel every fighter on this
/// stage plays with, said ONCE.
///
/// ⚠ **it states SIX numbers and disturbs nothing else** — see
/// [`MatchBody`](ambition_platformer2d::engine_core::MatchBody). Mary-O keeps
/// her SMB1 gravity and jump arc on this stage and gets an air dodge; the
/// crawler keeps its crawl. The composition is stated in
/// `MatchRules::body_over` and nowhere else.
///
/// ⛔ **authored HERE, not in the engine.** Every number below is zero or
/// otherwise in `DEFAULT_TUNING` for a reason that is still correct, and the
/// reasons are the point:
///
/// * **`slash_recoil: 0.0`.** The engine's 110 px/s backwards on every melee
///   press is a feel detail for the exploration protagonist, whose swings are
///   occasional and whose rooms have walls. A fighter brain presses attack on
///   most decisions, so the recoils RATCHET — measured 2026-07-31 with
///   `AMBITION_FIGHTER_TRACE=1`: 200, 310, 420, 530 px/s in exact 110 steps
///   against a 270 px/s run, while the brain's own emitted input pointed the
///   other way. **Every CPU on this stage swung itself off the edge,
///   backwards.** The A/B, same build, this number alone: at 110 a level 9
///   survives 5.2 s and loses 3 stocks; at 0 it survives 15.1 s and at rollout
///   depth 0 does not self-KO AT ALL in a 60 s match.
/// * **`jump_squat_time: 3/60`.** A fighter's jump is COMMITTAL; an explorer's
///   is not. Three frames of grounded crouch before takeoff is the universal
///   jump squat in Smash Ultimate, and it is what makes an opponent's jump a
///   READ rather than an instant escape. Everything downstream already exists:
///   a body struck during the crouch loses the leap, and a tap released inside
///   it still short-hops. `DEFAULT_TUNING` keeps `0.0` because a squat is not a
///   better jump, it is a different game's jump — Mary-O's SMB1 convergence
///   requires the leap on the press tick, and the exploration protagonist was
///   tuned without one.
/// * **the AIR DODGE.** The engine default is `0.0` — no window — because an
///   airborne burst press is the exploration protagonist's air dash and a
///   default-on evade would take that press away from every wandering body in
///   the game. A platform fighter is the body that wants it: one directional
///   evade per trip through the air, refunded on landing, with endlag on the
///   far side so it is a read rather than a panic button.
/// * **`tumble_speed: 500.0` — THE FLOOR GAME.** Above this launch speed a hit
///   sends the body tumbling, and the landing that follows is a knockdown
///   unless it is teched. 500 px/s sits above a jab's shove and below a smash's
///   launch, so the state a player enters is the one a player earned. The
///   engine default is `0.0` (no floor game) because a wandering enemy that had
///   to stand up after every hit would be a different game for the exploration
///   side.
///
/// ⭐ **it was authored on THREE CHARACTERS and is now authored on the STAGE**
/// (D146 slice 1b). The three blocks were one expression in one loop, so they
/// could not disagree with each other — and they could not reach the other
/// eleven fighters either, two of the three are stand-ins the composed host
/// drops, and the demo's own comment had already named the shape: *"The match
/// then stamped one flat set over every body, so what a fighter could do was a
/// property of the MATCH."*
pub const SMASH_FIGHTER_BODY: ambition_platformer2d::engine_core::MatchBody =
    ambition_platformer2d::engine_core::MatchBody {
        slash_recoil: 0.0,
        jump_squat_time: 3.0 / 60.0,
        air_dodge_time: ambition_platformer2d::engine_core::AIR_DODGE_TIME,
        air_dodge_speed: ambition_platformer2d::engine_core::AIR_DODGE_SPEED,
        air_dodge_endlag: ambition_platformer2d::engine_core::AIR_DODGE_ENDLAG,
        tumble_speed: 500.0,
    };

/// **THE BASIC SMASH ABILITIES** — the verbs every fighter on this stage has.
///
/// Jon named this list on 2026-08-16 (*"all characters should be sure they are
/// granted the basic smash abilities"*) and it is one constant so that the
/// stage, the tests and any future reader read the same one.
///
/// ⚠ **SPELLED OUT rather than a named engine set, and both candidates were
/// tried and measured first.** `basic()` has no double jump and no attack, so it
/// would REMOVE verbs the duelists already had. `sane_subset()` reads like a
/// fighter's kit in its first ten lines and is not one — measured, it also
/// grants fly, blink, precision_blink, wall climb and pogo, so declaring it made
/// two seats agree that they could both FLY.
///
/// ⚠ **`fly` and `blink` are absent deliberately**: this is a platform fighter's
/// ground game, not the exploration protagonist's traversal kit, and the July
/// measurement of two seats disagreeing was exactly a duelist meeting a body
/// that could fly. `interact` and `reset` are absent for the same reason a
/// fighter has no talk button and no teleport home.
///
/// ⚠ **`shield`, `dodge` and `ledge_grab` are what make this a platform fighter**
/// rather than two bodies running at each other. All three already existed in
/// the engine with nothing switched on.
///
/// ⭐⭐ **`dash` IS ABSENT, AND THAT IS THE POINT** (Jon, 2026-08-16: *"now that
/// each character has an up-b, I think we can likely also remove everyone's
/// ability to dash in smash. Dash should be an ability for ambition, it doesn't
/// map into a smash vocabulary."*). `AbilitySet::dash` is not running — running
/// is `move_horizontal` against the body's own top speed, and it consults no
/// ability bit beyond that one. `dash` is a DISCRETE charge-gated burst that
/// REPLACES the velocity vector for a window (`apply_dash`), which is a
/// traversal verb from Ambition's exploration kit and not one of a platform
/// fighter's sixteen presses. Dropping it leaves the burst BUTTON meaning
/// exactly one thing here — the dodge — which is what it means in the genre.
///
/// ⛔ removing it was a two-part change, not a deleted line: the kernel used to
/// fill the shared burst buffer only for `abilities.dash`, so this edit alone
/// would have deleted the DODGE from all fourteen fighters in silence. See
/// `apply_intent` in `movement/abilities.rs`.
pub const SMASH_FIGHTER_KIT: ambition_platformer2d::engine_core::AbilitySet =
    ambition_platformer2d::engine_core::AbilitySet {
        move_horizontal: true,
        jump: true,
        variable_jump: true,
        double_jump: true,
        fast_fall: true,
        attack: true,
        pogo: true,
        directional_primary: true,
        shield: true,
        // **The capture verb.** Granting it here does NOT invent a grab: the
        // action scheme wants `abilities.grab` AND an authored `"grab"` move, so
        // a fighter joins the mechanic on the day its table does and the other
        // thirteen are unchanged until theirs do.
        grab: true,
        dodge: true,
        ledge_grab: true,
        ..ambition_platformer2d::engine_core::AbilitySet::NONE
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

/// **Where a knocked-out fighter comes back.**
///
/// The engine spends the stock and clears the meter; it refuses to place the
/// body, because placing it needs a stage. This is that answer.
/// **Two CPU fighters at DIFFERENT levels — the ladder's own roster.**
///
/// [`smash_roster_at_level`] puts every CPU seat on one rung, which is what a
/// probe wants (*"how does level N behave"*) and not what a LADDER wants
/// (*"does level N beat level N−1"*). And [`smash_roster`] makes seat 0 HUMAN,
/// so the only opponent a probe could offer was a controller-less body that
/// never acts — every stock lost was a self-KO, which made the number clean and
/// made it impossible to measure a fight.
///
/// ⚠ **`opens_suspended` and the stock count are inherited deliberately.** A rig
/// that quietly ran a different ruleset from the shipped stage would measure a
/// game nobody plays; the ONLY difference from a real match is who is holding
/// the controllers.
///
/// ⛔ **it takes one level per seat, not a base and an offset.** `N vs N−1` is
/// the ladder's first question and not its last — `N vs N−3`, `N vs N`, and a
/// rung against a fixed reference are all things this measures — and encoding
/// the subtraction here would have to be undone by the second caller.
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
    // ⛔⛔ **AND IT SAYS WHOSE ROSTER IT IS.** `smash_roster` above ends with the
    // same call and this one silently did not — which cost nothing while a CPU
    // seat's `brain_profile` could still be an ARCHETYPE key, because an
    // archetype table is global. It costs everything now that a published POLICY
    // is the only thing a seat can name (P2.18):
    // `seat_brain_profile` resolves a provider-relative name in the MATCH's
    // provider, an unpublished roster has none, and every levelled seat this
    // helper builds was refused with *"`duelist_l1` … Known keys: [combatant]"*.
    //
    // ⚠ the regression was invisible to the run's gate — `cargo check -p
    // ambition_app --all-targets` plus `app_it` never builds
    // `ambition_demo_smash_app`'s tests, where all four of its CPU-roster
    // regressions were red (ledger D88).
    roster.published_by(SMASH_EXPERIENCE)
}

/// Horizontal spread between adjacent respawn points, in stage pixels.
///
/// Two 32px tiles — wider than a standing body, so two fighters returning on the
/// same frame land clear of each other rather than inside one another. ⭐ derived
/// against [`PLATFORM_WIDTH`]: seat `n` sits at most `(n/2 + 0.5)` spacings from
/// the centre, so even eight seats stay within ±224px of a 480px platform.
const RESPAWN_SEAT_SPACING_PX: f32 = 64.0;

/// **Where a fighter comes back, and it is not where its opponent comes back.**
///
/// ⛔⛔ **this took no seat at all**, so every fighter respawned on one point:
/// two knockouts on the same frame put both bodies inside each other over the
/// centre of the stage, at the exact moment neither has information or options.
/// (D128 defect 3.)
///
/// ⭐ **seats alternate outward from the centre** — 0 left, 1 right, 2 further
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

/// **Fifteen 32px tiles, or ten standing-body heights.**
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

/// **The stage: a platform surrounded by nothing.**
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
    world.blast_margin = FALL_BLAST_MARGIN_PX;
    // The SIDES are the interesting ones and they are not the default. A body
    // launched horizontally leaves through them, and without an explicit value
    // they inherit a margin sized for "fell through the floor" — generous enough
    // that a fighter knocked off the edge would drift for a second and a half
    // before anything noticed.
    world.side_blast_margin = Some(SIDE_BLAST_MARGIN_PX);
    world.ceiling_blast_margin = Some(CEILING_BLAST_MARGIN_PX);

    let mut room = RoomSpec::new(SMASH_STAGE_ROOM_ID, world);
    room.metadata.mode = Some(SMASH_MODE.to_string());
    room
}

/// The stage centre a respawn is measured from.
pub fn stage_centre() -> Vec2 {
    Vec2::new(STAGE_SIZE.x / 2.0, PLATFORM_TOP)
}

/// What the match announces when it ends.
///
/// ⭐ **"WINNER: <name>", in Jon's own words** (2026-08-16): *"the time in the
/// game should freeze with 'WINNER: <name>' to show the match is over"*. It read
/// `seat 2 wins` before — which is what he was looking at when he asked — and
/// the SIDE is not a name. `announce_the_winner` resolves the winning side into
/// the fighter's own name before it gets here; this owns the wording alone, so
/// the card and any test of it read one function.
pub fn victory_banner(winner: Option<&str>) -> String {
    match winner {
        Some(side) => format!("WINNER: {side}"),
        // A draw is reachable and cheaply: two fighters on their last stock,
        // knocked off together. A `winner: String` shape would have needed a
        // sentinel for this, which is why the engine's message carries an
        // `Option`.
        None => "Draw — everybody fell".to_string(),
    }
}

/// **The two answers the engine refuses to guess, wired to the messages it
/// writes.**
///
/// `ambition_combat::stocks` spends the stock, clears the meter and marks the
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
        app.add_message::<ambition_platformer2d::actor::StocksMatchDecided>();
        // The capture request channels. The ADAPTER below writes them and the
        // body runtime reads them, so this plugin owns them the same way it owns
        // the two above.
        app.add_message::<ambition_platformer2d::combat::capture::CaptureAttemptRequested>();
        app.add_message::<ambition_platformer2d::combat::capture::CapturePummelRequested>();
        app.add_message::<ambition_platformer2d::combat::capture::CaptureThrowRequested>();

        let sim = ambition_platformer2d::platformer::schedule::SimScheduleExt::sim_schedule(app);
        // **THE CAPTURE LOOP, in the order the facts become available.**
        //
        // `dispatch_move_events` turns a live grab window into an authored
        // `Effect` during `CombatSet::Playback`; the adapter recognises the key
        // and writes a typed request; acquisition turns that into a relationship.
        // Chained so a grab that goes active this tick catches this tick — the
        // alternative is a frame of latency on every grab, which in a fighting
        // game is a mechanic change rather than a rounding error.
        //
        // ⚠ **`Materialize`, beside the projectile spawns**, because that set's
        // own doc says what it is for: *"a thing must EXIST before it can hit
        // anything"*. A capture relationship is exactly such a thing — the
        // pummel and throw that target it are moves that come later.
        app.add_systems(
            sim,
            (
                crate::capture::translate_smash_capture_effects,
                ambition_platformer2d::actors::features::ecs::capture::acquire_captures,
                // ⭐ **and posed the SAME tick it is caught.** The pose sync also
                // runs in `WorldPrep`, which is EARLIER in the tick than this —
                // so without this second call a body grabbed now would hang where
                // it stood until the next frame, one visible frame of a captive
                // standing free inside somebody's grab animation.
                // The pummel lands BEFORE the pose sync below, so the damage and
                // the frame the captive is drawn in belong to the same tick.
                ambition_platformer2d::actors::features::ecs::capture::apply_capture_pummels,
                // The throw releases and launches in one step. AFTER the pummel
                // so a tick carrying both resolves in authored order, and BEFORE
                // the pose sync so a thrown body is not snapped back into a hold
                // it has just left.
                ambition_platformer2d::actors::features::ecs::capture::apply_capture_throws,
                ambition_platformer2d::actors::features::ecs::capture::constrain_captive_bodies,
            )
                .chain()
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Materialize),
        );
        // **A capture ends in `Settle`, where post-damage bookkeeping belongs.**
        // Hitstun and the recoil lock are written by damage resolution in
        // `Resolve`, so a release that ran earlier would read last tick's answer
        // and let a grab survive by one frame the hit that should have broken it.
        app.add_systems(
            sim,
            ambition_platformer2d::actors::features::ecs::capture::release_interrupted_captures
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
            // ⭐ **`run_empowerments` used to sit right here**, and the note
            // beside it said a component whose expiry depends on each game
            // scheduling a system is a grant that silently becomes permanent in
            // the game that forgets — *"probably an engine-side registration
            // later"*. That landed (queue D152): the engine installs the clock
            // in `EmpowermentExpiry`, so the respawn protection this file grants
            // ends whether or not anybody remembers. Nothing here reads the
            // grant, so nothing here needs an ordering edge against it — the
            // stamp lands in `GameplayEffects`, still ahead of the next frame's
            // `CombatSet::Resolve` that consults it, exactly as it did from this
            // slot.
            announce_the_winner,
        )
            .chain()
            .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle)
            .after(ambition_platformer2d::combat::stocks::FighterStocksSpent);
        // ⛔⛔ **THE ONE RULE THAT CANNOT RUN ALONGSIDE THE DECISION**, pulled out
        // of the chain above and ordered behind it.
        //
        // Reported from the couch (2026-08-15): *"there seems like several cases
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
        // ⚠ **only this one waits.** The HUD, the countdown and the respawn
        // placement are still meant to run beside the engine's answer rather than
        // behind it — see `FighterStocksSpent`'s own note — and putting the whole
        // chain behind the decision would take that away to fix one member.
        let remove_the_eliminated = take_eliminated_fighters_out_of_play
            .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle)
            .after(ambition_platformer2d::combat::stocks::MatchOutcomeDecided);
        if self.hosted {
            let gate = ambition_platformer2d::runtime::in_mode(SMASH_MODE);
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
/// ⛔ Until 2026-08-01 this demo declared NO HUD, so it inherited Ambition's
/// adventure one and a platform-fighter match was drawn with `HP 100/100`,
/// `MP 100` and `$0` — a health bar, a mana bar and a money counter, none of
/// which describe this game. Photographed, not reasoned about.
///
/// ⚠ the data was already shaped for these readouts and nothing consumed it:
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
pub const SMASH_ANNOUNCE_HUD_SLOT: &str = "smash_announce";

/// **What plays on the stage.**
pub const SMASH_STAGE_TRACK: &str = "super_smash_siblings_theme";
/// **What plays over the character select**, in a host whose frontend audio
/// this demo owns. See `SMASH_TRACKS` for why it is registered either way.
pub const SMASH_SELECT_TRACK: &str = "super_smash_siblings_character_select";

/// **The scores written for this demo**, rendered from
/// `tools/ambition_music_renderer/scores/active/super_smash_siblings_*.music.yaml`.
///
/// ⚠ **all three are registered, not only the one that plays.** A track in this
/// fragment is a track this experience is ALLOWED to play — the radio, a future
/// stage select, and the winner card all pick from it — so registering only the
/// default would make the other two unreachable from inside a smash session
/// even though they were written for it. The default is what plays with nobody
/// asking.
///
/// ⚠ the asset path is derived (`audio/music/generated/<id>/full.ogg`) rather
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
/// ⚠ percent is NOT health and the gauge fill says so: it fills as damage
/// ACCUMULATES, and the number keeps counting past 100% because a platform
/// fighter's does. Clamping the fill is a rendering decision; clamping the
/// number would be a lie about the game.

/// **THE COMBAT RULES THIS STAGE DECLARES**, in one place so the publisher and
/// its guard cannot hold different copies.
///
/// ⛔⛔ **they did.** The roster publisher read the `DeclaredCombatRules`
/// RESOURCE for its unarmed floor while the same system inserted that resource
/// fifty lines later through a deferred `Commands::insert_resource` — so on the
/// frame the match is decided the resource does not exist and `None` was
/// published (measured on the shipped select screen: `present = false`). Every
/// kit-less fighter reached the stage unable to hit anybody.
/// ⚠ and the guard could not catch it, because it passed the swipe in BY HAND:
/// *"a fixture that manufactures the value under test cannot fail on its
/// absence."* Both now call this.
///
/// ⚠ **reading the resource would be wrong even when it exists**: on a second
/// visit it holds the PREVIOUS match's declaration. A function has no such tense.
pub fn smash_declared_combat_rules() -> ambition_platformer2d::combat::rules::DeclaredCombatRules {
    ambition_platformer2d::combat::rules::DeclaredCombatRules {
        // ⛔ BY OWNER. The versus route declares combat rules too, and a
        // giveback that removed this by TYPE would delete ITS live rules the
        // moment smash left — the lesson the roster and the prepared match each
        // taught once already.
        declared_by: SMASH_EXPERIENCE.to_string(),
        di_max_angle: SMASH_DI_MAX_ANGLE,
        knockback_growth: SMASH_KNOCKBACK_GROWTH,
        // ⭐⭐ **A DOWN-AIR IS A SPIKE HERE**, not a pogo (ledger D82). The robot's
        // down-air is ONE authored swing that says it can rebound its attacker;
        // Ambition takes it up on that, and a platform fighter must not — a
        // d-air that bounced you back to safety offstage would be the opposite
        // of a kill. Same move, two games, and the difference is declared rather
        // than authored twice.
        downward_hit: ambition_platformer2d::combat::rules::DownwardHitStyle::Spike,
        // ⚠ teams already decide who may hit whom. Switching global friendly
        // fire on to let two humans trade would make TEAMMATES hittable too.
        friendly_fire: false,
        // ⭐⭐ **THE STAGE'S FLOOR, DECLARED** (P3.24/P2.20, 2026-08-12). This
        // lived in `select::smash_fighter_kit()` — a helper this crate applied to
        // every seat whose character says nothing — while EXPLORATION answered
        // the same question with a different swipe. Two spellings of "what does
        // an unarmed body swing", neither owned by anybody.
        //
        // ⛔ these numbers are the helper's VERBATIM: 0.22 / 0.08 / 0.26, 4
        // damage, 34 reach. A stage's floor is faster, harder and longer than an
        // exploration provoke's, and moving it here is not the place to decide
        // that differently.
        unarmed_melee: Some(ambition_platformer2d::character::MeleeActionSpec::Swipe(
            ambition_platformer2d::character::SwipeSpec {
                windup_s: 0.22,
                active_s: 0.08,
                damage: 4,
                reach_px: 34.0,
                recover_s: 0.26,
            },
        )),
    }
}

pub fn publish_smash_hud(
    fighters: bevy::prelude::Query<(
        &ambition_platformer2d::actors::character_runtime::MatchSeat,
        &ambition_platformer2d::characters::actor::BodyHealth,
        Option<&ambition_platformer2d::actor::FighterStocks>,
        &bevy::prelude::Name,
    )>,
    mut readouts: bevy::prelude::ResMut<ambition_platformer2d::presentation::HudReadouts>,
) {
    let mut rows: Vec<(usize, String, f32, Option<(u32, u32)>)> = fighters
        .iter()
        .map(|(seat, health, stocks, name)| {
            (
                seat.0,
                name.as_str().to_string(),
                health.damage_percent(),
                stocks.map(|s| (s.remaining, s.started_with)),
            )
        })
        .collect();
    // Sorted by SEAT. Query order is not an order, and a scoreboard whose sides
    // swap mid-match is worse than none — the same reason the versus stage sorts.
    rows.sort_by_key(|(seat, ..)| *seat);

    let mut written = [false; FIGHTER_HUD_SLOTS.len()];
    for (seat, name, percent, stocks) in &rows {
        let Some(slot) = FIGHTER_HUD_SLOTS.get(*seat) else {
            continue;
        };
        written[*seat] = true;
        let value = match stocks {
            Some((remaining, started)) => {
                format!("{:.0}%  ·  {remaining}/{started}", percent * 100.0)
            }
            None => format!("{:.0}%", percent * 100.0),
        };
        readouts.set(
            *slot,
            ambition_platformer2d::presentation::HudReadout::gauge(name.clone(), value, *percent),
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

/// **3 — 2 — 1 — GO.**
///
/// The roster opens `opens_suspended`, which stamps `ScriptedControl` on every
/// fighter in the same flush that creates them, and declares
/// `opening_countdown_ticks`. The ENGINE takes the hold off when the ceremony
/// ends (`release_the_opening_hold`), atomically, for every seat on one tick.
/// This system is the part a stage owns: saying the numbers out loud.
///
/// ⚠ **this demo had no countdown and released the instant the match went
/// live.** The comment that stood here recorded the day it had no release
/// either — the fighters seated, stood exactly where seating put them, and
/// never moved while every test passed, because they existed, wore seats,
/// carried stocks and were correctly suspended forever. The tell was a diagram
/// printing `travel: [0.0, 0.0]`.
///
/// ⭐ **DERIVED from the clock, so it cannot drift from the release.** The
/// number on screen and the tick the bodies are freed are two readings of one
/// pure function of `now - activated_on`; a separate timer for the card would
/// be a second authority on when the round starts, and the two would disagree
/// on the frame anybody looked closely.
///
/// ⛔⛔ **AND IT USED TO BE INVISIBLE** — reported from the couch, 2026-08-15:
/// *"I think there is also a countdown to start the match, but there is no
/// visual indication of that countdown, like a 3, 2, 1, go."* Exactly right, and
/// the reason is that this wrote a `GameplayBannerRequested`. Nothing DRAWS a
/// `GameplayBanner`: its one reader in the whole workspace is the app's debug
/// HUD text, which prefixes it `FEATURE:` and is gated on `player.single()` — so
/// in a CPU-versus-CPU match, which has no primary player, not even the debug
/// line appeared. The ceremony ran, the fighters were held and released, and the
/// screen said nothing.
///
/// ⇒ it writes [`SMASH_ANNOUNCE_HUD_SLOT`] instead — the centred 34pt gold card
/// this demo has DECLARED since the HUD landed and never once written to. Same
/// road as the fighter percents beside it, which are visibly drawn.
///
/// ⭐ **and the `Local` is gone with the banner.** A readout is idempotent (a map
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
    // ⛔⛔ **THE CEREMONY STOPS TALKING THE MOMENT THE MATCH IS DECIDED** (D140).
    //
    // The card has exactly one owner at a time and the ORDER is the whole rule:
    // the opening owns it until there is an outcome, and then the outcome does,
    // for as long as the results stand. This used to be expressed as "do not
    // CLEAR the winner's card", which guarded the wrong half — the `Some(word)`
    // arm was ungated, so a match decided while GO! was still up (its card holds
    // one beat past the release, and a knockout can land inside that beat)
    // overwrote the victory card with GO! on the very next tick. Measured on
    // this stage: `["3", "2", "1", "GO!", "seat 1 wins", "GO!"]`.
    //
    // ⚠ and the same line is why a stuck GO! was UNCLEARABLE. Once the previous
    // match's verdict could not be retracted (the defect above this one), the
    // clear was gated off forever and the card sat on a live match announcing
    // its start. Both halves of D140 met on this one `if`.
    if settled.is_some_and(|settled| settled.settled(&active)) {
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
/// ⚠ through `reset_body_clusters`, not `transit_body`, and the difference is a
/// leak. Both re-resolve a body's pose against the world (ADR 0024 — a body
/// appearing somewhere has to ARRIVE there, not be teleported into whatever is
/// standing at the coordinates), but `transit_body` documents that "axis
/// maneuver state (coyote, buffers, dash timers) is deliberately KEPT — those
/// are time facts, not place facts". That is right for a blink and wrong for
/// losing a stock: a fighter came back holding the dash timer and buffered jump
/// it died with.
///
/// `reset_body_clusters` is the verb that means "this body starts again" — the
/// same one the sandbox reset and the versus round boundary use — and it raises
/// `BodyLifetime::restart_pending`, so `announce_body_restarts` triggers
/// `ae::BodyRestarted` and every PROVIDER hears about the respawn too. Through
/// `transit_body` none of them did: a ball-dash charge or a rolling form would
/// have survived a knockout in silence.
///
/// This is the versus stage's 2026-07-28 bug reintroduced in a new demo three
/// days later, which is the argument for the announcement being DERIVED rather
/// than announced by hand — the derivation is what makes this a one-line fix
/// instead of a hunt for every provider that cares.
fn place_respawning_fighters(
    mut commands: bevy::prelude::Commands,
    mut spent: bevy::prelude::MessageReader<ambition_platformer2d::actor::FighterStockSpent>,
    mut bodies: bevy::prelude::Query<(
        ambition_platformer2d::actor::BodyClusterQueryData,
        &mut ambition_platformer2d::actors::features::MotionModel,
        // ⭐ the SEAT, so two fighters returning on one frame do not land inside
        // each other. `Option` because a body without one is not a seated
        // fighter, and this system must not stop placing it.
        Option<&ambition_platformer2d::actor::MatchSeat>,
    )>,
) {
    for event in spent.read() {
        // An ELIMINATED fighter is not placed. It has no stock to come back on,
        // and putting it back would make the last knockout the only one that did
        // not count.
        if event.eliminated {
            continue;
        }
        let Ok((clusters, mut model, seat)) = bodies.get_mut(event.body) else {
            continue;
        };
        // A body with no seat falls back to seat 0's point — the same place it
        // used to get, so an unseated body is no worse off than before.
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
        // **RESPAWN PROTECTION.**
        //
        // A fighter materialising over the stage was hittable on its first
        // frame, at the exact moment it has no information and no options — the
        // opponent that just took the stock is standing there. Every platform
        // fighter answers this the same way, and so does the engine already:
        // `Empowered` is the generic timed-untouchable grant a star pickup uses,
        // it is rollback-registered, and it expires on its own.
        //
        // ⛔ **the RULESET grants it, not the character.** The same fighter in
        // Ambition has no stocks to lose and gets none of this; a mode that
        // wants none simply does not insert it. That is why this is here rather
        // than on a `CharacterDefinition`.
        //
        // ⚠ it expires on TIME alone today. Jon's brief also allows clearing it
        // when the returning fighter commits an attack ("and/or"), which is the
        // stricter rule and wants a system watching the attack edge — worth
        // adding once there is a real complaint about a protected fighter
        // swinging.
        commands.entity(event.body).try_insert(
            ambition_platformer2d::actors::features::empowerment::Empowered::for_seconds(
                ambition_platformer2d::actors::features::empowerment::Empowerment::UNTOUCHABLE,
                RESPAWN_PROTECTION_SECONDS,
            ),
        );
    }
}

/// **How long a returning fighter cannot be hit**, in seconds.
///
/// Long enough to fall in, read the stage and choose a landing; short enough
/// that camping the spawn point is not free. Smash Ultimate's respawn platform
/// holds for about three seconds and releases on the first action; this is the
/// no-platform version of the same idea.
const RESPAWN_PROTECTION_SECONDS: f32 = 2.0;

/// **Take an eliminated fighter OUT OF PLAY.**
///
/// `ambition_combat::stocks` says this in as many words — a fighter with no
/// stocks left "is still standing until a ruleset removes it" — and for a day
/// this ruleset did not. The result, measured over sixty seconds of real
/// fighting: the loser fell out of the world, was correctly eliminated, and then
/// KEPT FALLING, taking a fresh `LeftTheWorld` hit every tick. It reached
/// y=34430 and **270900%**.
///
/// Nothing was wrong upstream. The stock was spent exactly once — the engine's
/// `Without<FighterEliminated>` filter held — the match was decided, and the
/// body simply never stopped being a body. That is the difference between "the
/// count is correct" and "the match is over", and it is the ruleset's half.
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

/// **When the match ends, go back and choose again.**
///
/// Jon, 2026-07-29, specifying Smash Siblings: *"Its 3 stocks, and then when the
/// game ends it goes back to the character select screen."* Recorded as a
/// decision that day (`maintainer-decisions.md`) and never built — the banner
/// went up, and the demo then sat on a decided stage with nothing left to
/// decide and no way back but the pause menu.
///
/// ⚠ **`Update` and REAL time, not the sim schedule.** Leaving a match is shell
/// lifecycle: the countdown must keep running while the simulation is over, and
/// the route change is a shell command, not a rule.
///
/// ⚠ **armed once.** `StocksMatchDecided` is written from the sim, so a rollback
/// can re-deliver it; re-arming on the second copy would restart the countdown
/// and hold the players on a finished match.
fn return_to_the_select_screen_when_the_match_ends(
    mut decided: bevy::prelude::MessageReader<ambition_platformer2d::actor::StocksMatchDecided>,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    time: bevy::prelude::Res<bevy::prelude::Time>,
    mut shell: bevy::prelude::MessageWriter<ambition_platformer2d::game_shell::ShellCommand>,
    mut readouts: bevy::prelude::ResMut<ambition_platformer2d::presentation::HudReadouts>,
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
        decided.clear();
        // ⭐ **AND THE CARD BELONGS TO THIS VISIT TOO** (D140). It used to be
        // left standing on the argument that "the experience's whole HUD
        // declaration goes with the route", which is true about what is DRAWN
        // and not about what is HELD — so the next match arrived on the stage
        // with the previous winner still in the slot, and it showed for the two
        // frames before that match activated and the ceremony took the slot
        // over. A card that outlives its match is exactly the defect D140 is
        // about, in miniature.
        readouts.clear_slot(SMASH_ANNOUNCE_HUD_SLOT);
        return;
    }
    let ended = decided.read().count() > 0;
    if ended && countdown.is_none() {
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

// ⛔ **`dress_the_primary_player_as_their_own_pick` IS GONE, and so is the whole
// class of bug it was made of.**
//
// It existed to reconcile two facts that should never have needed reconciling:
// the stage spawned a privileged home body when it was PREPARED, and the select
// screen decided who seat 0 was afterwards. Seating adopted that body for the
// human seat and refused until it already wore the picked fighter, so something
// had to re-dress it — and that something dressed it as `participants.first()`,
// which is only the human seat by coincidence. Put a CPU on an earlier card and
// the body wore the CPU's costume, the human seat waited for one it would never
// be given, and because one unresolved seat returned from the whole system,
// NOBODY was seated. The stage opened with the home body standing on it and
// nothing anywhere said why (Jon, 2026-08-06).
//
// A match now builds its own cast — every fighter by one path, control attached
// afterwards — so there is no pre-existing body to reconcile with and no costume
// handshake to lose. The two repairs this system carried survive where they
// belong: the roster is owned by the experience that published it, and it leaves
// with that experience (see the scope in `SmashExperiencePlugin`).

/// Say who won, once.
///
/// ⛔⛔ **this wrote a `GameplayBannerRequested` too, and so the winner card was
/// as invisible as the countdown was** — see
/// [`announce_the_opening_countdown`] for why nothing draws that channel. The
/// demo has declared a centred announce slot the whole time; this writes it.
///
/// ⚠ **written once, and it stands until the stage is LEFT.** A readout
/// persists until somebody replaces or clears it, and nothing on this stage
/// replaces it — [`announce_the_opening_countdown`] hands the slot over the
/// moment a match is decided. 4.5 s later
/// [`return_to_the_select_screen_when_the_match_ends`] takes the card down as it
/// goes, so the card stands for exactly as long as the results screen does with
/// no timer to keep in step with the route change.
///
/// ⛔ **it used to rely on the HUD DECLARATION leaving with the route instead**,
/// and that is a claim about what is DRAWN, not about what is HELD — the next
/// match arrived on the stage still holding the last one's winner (D140).
fn announce_the_winner(
    mut decided: bevy::prelude::MessageReader<ambition_platformer2d::actor::StocksMatchDecided>,
    // **WHO IS IN THIS MATCH — the FROZEN answer** (D148). Whether a side is a
    // person or a team is a fact about the match that was prepared, and the plan
    // is the only thing that still knows it once fighters start being removed.
    prepared: Option<
        bevy::prelude::Res<ambition_platformer2d::actors::character_runtime::PreparedMatch>,
    >,
    // **THE CAST, so the card can say a NAME.** The engine decides a SIDE — a
    // team, or `seat 2` when nobody declared one — and a side is the right
    // answer to "who won" and the wrong thing to put on a screen. The bodies
    // still standing are the ones that know their names.
    //
    // ⚠ read for the NAME only. It used to decide the team-or-person question
    // too, by counting the rows on the winning side — see below.
    fighters: bevy::prelude::Query<(
        &ambition_platformer2d::actors::character_runtime::MatchSeat,
        Option<&ambition_platformer2d::combat::targeting::MatchTeam>,
        &bevy::prelude::Name,
    )>,
    mut readouts: bevy::prelude::ResMut<ambition_platformer2d::presentation::HudReadouts>,
) {
    for outcome in decided.read() {
        // ⚠ **a TEAM keeps its own name.** Substituting one member's name for
        // "Red" would name a player on a side that won together, so the swap
        // only happens for a side of one — which is the only case where the
        // side and the fighter are the same thing.
        //
        // ⛔⛔ **and "a side of one" is asked of the PLAN, not of the bodies**
        // (D148). This counted the fighters still standing on the winning side,
        // which is a different question the moment anybody dies:
        // `take_eliminated_fighters_out_of_play` despawns an eliminated body, so
        // Red = Alice + Bob with Alice knocked out early has exactly ONE Red
        // body at victory and the card announced `WINNER: Bob` — contradicting
        // the rule stated one paragraph above it. Body residency recovering
        // match-participant identity, which is the error this campaign keeps
        // paying for.
        //
        // ⚠ **and the fallback is the SIDE, not a panic.** A simultaneous
        // ring-out despawns every body, so a card can legitimately be asked to
        // name a winner with nobody left standing to ask.
        let named = outcome.winner.as_deref().map(|side| {
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
            ambition_platformer2d::presentation::HudReadout::bare(victory_banner(named.as_deref())),
        );
    }
}

/// **The screen the demo opens on, and the transition out of it.**
///
/// The decision itself is [`select::SmashSelect`], which has no Bevy in it. This
/// is the part that has to: it holds the value, and when the value says the
/// match is decided it publishes the roster and asks the shell to go to the
/// stage.
///
/// ⚠ **the roster is inserted BEFORE the route changes**, and the order is the
/// whole correctness argument. Seating runs on the sim schedule and reads
/// `MatchParticipantRoster`; if the route changed first, the stage would come up
/// with no roster, seating would find nothing to do, and the match would open
/// with an empty cast that nothing retries into existence — the roster arrives
/// once, and it has to arrive before the thing that reads it.
pub struct SmashSelectPlugin;

/// **When the select screen reads its input**, as something another system can
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
        // **AND IT SOUNDS LIKE ITSELF.** The select screen has a score written
        // for it, and this is the declaration that carries it into any host —
        // the standalone demo, Ambition, or a composition that does not exist
        // yet. Declared HERE, beside the route, because the two are one fact
        // about one screen; a host naming smash's music would be a host knowing
        // a provider's content.
        //
        // ⚠ this was impossible until 2026-08-07: frontend audio was one
        // process-global resource, so the select score played in the standalone
        // app and NOWHERE else, and the comment in `ambition_demo_smash_app`
        // recorded that as a gap in the seam rather than a decision.
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
        app.init_resource::<select::SmashSelect>();
        // The pointer, and the one thing it can ask for that the value does not
        // hold. Both live outside `SmashSelect` on purpose: where a cursor is
        // pointing is not part of what the screen DECIDED, and a decision value
        // that carried a screen position would change every time somebody moved
        // the mouse.
        app.init_resource::<select_screen::cursor::SelectCursor>();
        app.init_resource::<select_screen::StartRequested>();
        // **THE ROSTER IS A COMPOSITION FACT, so it is resolved once, late.**
        //
        // ⚠ `Startup` rather than `build`, and the ordering is the reason: the
        // assembled `CharacterCatalog` is replaced every time ANY plugin
        // registers a fragment, so a roster computed while plugins are still
        // being added would see whichever cast had been registered so far. By
        // `Startup` every provider in the composition has declared itself.
        app.init_resource::<select::SmashRoster>();
        app.add_systems(bevy::prelude::Startup, assemble_the_smash_roster);
        // **THE PORTRAIT SHEETS' OWN MANIFESTS, so a face is one FRAME.**
        //
        // ⛔ without this the grid drew each portrait PNG whole, which is right
        // for the single-frame sheets that are most of them and visibly wrong
        // for `alice` and `oiler` — 2048x320 each, eight frames of a
        // default/speaking/focused clip set, drawn as a strip of eight tiny
        // Alices. Found by looking at a capture.
        //
        // ⚠ **guarded, because Ambition's dialogue box installs the same plugin**
        // and Bevy panics on a duplicate. This demo is composed both standalone
        // and inside that host; whichever gets there first wins and the registry
        // is the same baked table either way.
        if !app
            .is_plugin_added::<ambition_platformer2d::sprite_sheet::PortraitSheetRegistryPlugin>()
        {
            app.add_plugins(ambition_platformer2d::sprite_sheet::PortraitSheetRegistryPlugin);
        }
        // **THE SCREEN DECLARES ITS OWN INPUT PORT.** The host fills
        // `SeatMenuFrames` when a windowed host is installed; `init_resource`
        // will not clobber one that already exists. Declaring it here means the
        // screen is drivable in a headless app too — which is what lets a TEST
        // press a button instead of reaching into `SmashSelect` and setting the
        // answer, and reaching into the answer is how this screen came to be
        // fully unit-tested and completely inert.
        app.init_resource::<ambition_platformer2d::input::SeatMenuFrames>();
        // **AND THE SEATS IT OFFERS.** A host seats input participants from the
        // match roster, and this screen is what PRODUCES the roster — so until
        // it declares them, only player one exists and the other panels are
        // chairs no controller can reach. See `DeclaredInputSeats`.
        app.init_resource::<ambition_platformer2d::input::DeclaredInputSeats>();
        // The couch policy this demo drives. Defaults to `UnifiedPrimary`, so
        // installing it changes nothing until the select screen says otherwise.
        app.init_resource::<ambition_platformer2d::input::sources::InputAssignmentPolicy>();
        // **ONE CHAIN, IN `InputSet::Consume`.** Two things were ambiguous and
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
        // **THE SCREEN CLAIMS ITS SEATS' INPUT while it is up.**
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
        // **AND IT SAYS WHAT ITS CONFIRM CONTROL DOES.**
        //
        // A claim says who the presses are FOR; a cue says what confirming
        // MEANS, in this screen's own words. They are two facts and the screen
        // only published the first, so every prompt surface fell back to the
        // generic "Select" — measured in the host, `top_cue` was `None` on this
        // route while the screen was up.
        //
        // ⚠ **the cue is also the only evidence a prompt has when no context
        // resolver is installed.** `publish_frontend_context_prompt` reads the
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
                    present_the_select_screen,
                    // ⚠ **DRIVE BEFORE DRAW, and the order is not cosmetic.**
                    // The cursor hit-tests the MEASURED layout, which Bevy
                    // computes in `PostUpdate` — so both of these read last
                    // frame's rectangles. Drawing first would place the tokens
                    // from a decision the click on this frame is about to
                    // change, and a dragged token would lag the cursor by a
                    // frame for no reason anybody could see.
                    bevy::prelude::IntoScheduleConfigs::run_if(
                        select_screen::drive_the_cursor,
                        the_select_screen_owns_its_input,
                    ),
                    select_screen::place_the_screen,
                    select_screen::update_the_select_screen,
                    start_the_battle_when_asked,
                    return_to_the_select_screen_when_the_match_ends,
                )),
                SmashSelectSet,
            ),
        );
    }
}

/// **Who can be picked, in THIS composition.**
///
/// `select::SMASH_ROSTER` filtered to the ids this host can SEAT — so a
/// multi-game host offers the whole crossover cast and the standalone demo
/// offers the fighters it declares itself, from one list.
fn assemble_the_smash_roster(
    // ⛔ the SEATABLE authority, not the catalog — see `SmashRoster::assemble`.
    // Optional because a composition may reach this route before any character
    // is registered; an empty grid then says so honestly rather than offering
    // portraits nothing can build.
    registry: Option<
        bevy::prelude::Res<
            ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry,
        >,
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

/// Spawn the screen's UI on arrival and tear it down on leaving.
///
/// Route-driven rather than state-driven: the screen is a ROUTE, and tying the
/// panels to `SmashSelect` would leave them standing through the match (the
/// resource keeps its decision, which is what the match was built from).
fn present_the_select_screen(
    mut commands: bevy::prelude::Commands,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    mut select: bevy::prelude::ResMut<select::SmashSelect>,
    roster: Option<bevy::prelude::Res<MatchParticipantRoster>>,
    devices: Option<bevy::prelude::Res<ambition_platformer2d::input::LocalDeviceOrder>>,
    mut lobby_seats: bevy::prelude::ResMut<ambition_platformer2d::input::DeclaredInputSeats>,
    mut assignment: bevy::prelude::ResMut<
        ambition_platformer2d::input::sources::InputAssignmentPolicy,
    >,
    // Whether THIS demo is the one holding the couch policy, so leaving its
    // routes restores the default exactly once and never stamps over a policy
    // some other experience set.
    mut claimed_policy: bevy::prelude::Local<bool>,
    mut pointer: bevy::prelude::ResMut<select_screen::cursor::SelectCursor>,
    mut start: bevy::prelude::ResMut<select_screen::StartRequested>,
    fighters: bevy::prelude::Res<select::SmashRoster>,
    // ⚠ ONE parameter, not four. See `select_screen::ScreenArt` — four separate
    // `Res` arguments pushed this system past Bevy's parameter tuple ceiling,
    // and the three that make a portrait belong together anyway.
    art: select_screen::ScreenArt,
    existing: bevy::prelude::Query<(), bevy::prelude::With<select_screen::SmashSelectUiRoot>>,
    roots: bevy::prelude::Query<
        bevy::prelude::Entity,
        bevy::prelude::With<select_screen::SmashSelectUiRoot>,
    >,
) {
    let on_select = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_SELECT_ROUTE);
    // **WHILE THIS SCREEN IS UP, THE PADS ARE SEATS.** Declared here and dropped
    // on the way out, so the participants it asks for live exactly as long as
    // the question does — the same lifetime rule the match's own seats have.
    // **THIS DEMO IS A COUCH GAME**, so its sources CLAIM seats rather than all
    // driving player one — for the SELECT SCREEN *and the match it starts*.
    //
    // ⛔ this said `if on_select` and reverted to unified everywhere else, which
    // meant the assignment a lobby made was undone the instant the stage loaded.
    // Measured: the pad's DPadRight arrived on BOTH seats' `ActionState` during
    // the match (`move_right=true` on slot 0 and slot 1) while `MenuSelect` on
    // the select screen had correctly reached slot 1 alone. Menu input looked
    // isolated and gameplay input was not, and the reason was not two input
    // paths — it was the same path under two different policies, because the
    // policy was keyed on the ROUTE and the route had changed.
    //
    // Jon's brief says it directly: *"Before the match starts, freeze:
    // participant, session seat, control channel, input sources."* A source
    // assignment that expires when the lobby closes is the opposite of frozen.
    //
    // ⚠ still scoped to this demo's own routes. Ambition's rooms keep the
    // unified default, where a spare controller is another way to move the same
    // character (milestone 8).
    let on_smash_route = router.active.as_ref().is_some_and(|active| {
        matches!(
            active.route_id.as_str(),
            SMASH_SELECT_ROUTE | SMASH_GAMEPLAY_ROUTE
        )
    });
    // ⛔ **write only what THIS demo claimed** (GPT 5.6, 2026-08-01). The first
    // version set `JoinToClaim` on smash routes and `UnifiedPrimary` on every
    // other one — so a demo plugin was stamping a global host resource while
    // another game owned the screen, and no other experience could hold its own
    // assignment policy. The comment said "route-scoped"; the code was global.
    //
    // A claim is released by whoever made it: this restores the default only on
    // the frame it leaves its own routes, and is silent everywhere else.
    let couch = ambition_platformer2d::input::sources::InputAssignmentPolicy::JoinToClaim;
    if on_smash_route {
        if *assignment != couch {
            *assignment = couch;
        }
        *claimed_policy = true;
    } else if *claimed_policy {
        *claimed_policy = false;
        // Only undo OUR value. If something else has since set a policy, that is
        // its business and this demo has no opinion about it.
        if *assignment == couch {
            *assignment =
                ambition_platformer2d::input::sources::InputAssignmentPolicy::UnifiedPrimary;
        }
    }
    let policy = *assignment;
    let offered = devices
        .as_deref()
        .map(|devices| select::seats_offered_under(devices, policy))
        .unwrap_or(1) as u8;
    let want_seats =
        ambition_platformer2d::input::DeclaredInputSeats(if on_select { offered } else { 0 });
    if *lobby_seats != want_seats {
        *lobby_seats = want_seats;
    }
    if on_select {
        // **ARRIVING is where a rematch becomes possible.** The screen's own
        // exit condition is "everyone is locked in AND no roster exists yet", so
        // a match that ended and came home left both of those permanently
        // wrong: the roster it was built from is a plain resource that outlives
        // the session, and every seat was still locked in. The result was a
        // select screen you could look at and never leave — reachable only from
        // a host that can return here, which is exactly what listing this demo
        // in a multi-game launcher made possible.
        if existing.is_empty() {
            *select = select::SmashSelect::default();
            // ⚠ **the CURSOR and the START request are reset with it.** They are
            // separate resources and would otherwise be the residue that makes
            // a rematch behave differently from a first match: a `StartRequested`
            // left true re-publishes the roster on the frame the screen opens,
            // which is the same "you could look at it and never leave" bug the
            // paragraph above is about, in a second resource.
            *pointer = select_screen::cursor::SelectCursor::default();
            *start = select_screen::StartRequested::default();
            // THIS demo's roster. Another stage in the same host publishes its
            // own into the same global resource, and clearing "the roster" is
            // how one game deletes another's match.
            if roster.is_some_and(|roster| roster.is_published_by(SMASH_EXPERIENCE)) {
                commands.remove_resource::<MatchParticipantRoster>();
            }
            // ⭐ **AND THE SEATING IS THIS EXPERIENCE'S TO DECIDE, from now
            // until it leaves.** Claimed the moment the question opens rather
            // than when it is answered: a session that started in this window
            // would freeze a topology from connected DEVICES, and it is never
            // resized afterwards, so the lobby's answer would arrive too late to
            // matter. `start_the_battle_when_asked` turns this into a decision.
            commands.insert_resource(
                ambition_platformer2d::rollback::local_session::SessionSeatingSource::pending(
                    SMASH_EXPERIENCE,
                ),
            );
        }
        select_screen::spawn_select_screen(commands, existing, fighters, art);
    } else {
        select_screen::despawn_select_screen(commands, roots);
    }
}

/// **Claim input for the seats this screen drives, while it is up.**
///
/// ⛔ without this the screen was a surface nothing arbitrated. With the
/// universal pause menu open OVER it, the arrows drove BOTH — the menu's cursor
/// and the CPU count — because the two read different channels
/// (`MenuControlFrame` and `SeatMenuFrames`) and neither could consume the
/// other's edge.
///
/// ⚠ **the fix is not a feature edge to the shell.** This demo cannot name
/// `ShellPauseMenu`: `basic_shell_presentation` is not in `all_capabilities`,
/// which is the oracle rule working as intended. It names an input CONTEXT —
/// vocabulary the facade already exposes — and the pause menu's higher-priority
/// capturing claim does the rest. Neither side knows the other exists.
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

/// **Is the select screen the active route?**
///
/// One answer, three askers — the context claim, the cue, and the "may I drive"
/// gate. It was written out three times; the third copy is how a screen ends up
/// claiming input on a route it no longer draws on.
fn on_the_select_route(router: &ambition_platformer2d::game_shell::ShellRouter) -> bool {
    router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_SELECT_ROUTE)
}

/// **Publish this screen's submit verb while it is up.**
///
/// ⚠ `sync` rather than a declare/retract pair, so LEAVING retracts. A cue left
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

/// **Is this screen the one the presses belong to?**
///
/// ⛔ without this the screen was a surface nothing arbitrated. With the
/// universal pause menu open OVER it, the arrows drove BOTH — the menu's cursor
/// and the lobby — because the two read different channels (`MenuControlFrame`
/// and `SeatMenuFrames`) and neither could consume the other's edge.
///
/// ⚠ **it asks whether ANY seat still owns `SELECT_CONTEXT`, not whether seat 0
/// does.** There is one cursor and four people may drive it, so the screen
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
/// ⚠ **it waits for START to be CLICKED**, where the previous version left the
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
    // **WHAT THIS SCREEN'S SOURCE NUMBERS MEAN.** A slot's occupant is an index
    // into the sources the screen offered, and whether index zero is the
    // keyboard or the first pad is the policy's answer — the same one
    // `source_name_under` labels the slot with. Reading it here is what stops
    // the roster and the label disagreeing about who is holding what.
    assignment: bevy::prelude::Res<ambition_platformer2d::input::sources::InputAssignmentPolicy>,
    // **WHO ALREADY HAS A REPERTOIRE**, so a seat whose character authors its own
    // moves is not handed this stage's generic kit (Jon's redirect §17).
    // `Option`, like every other reader of the cast.
    prepared: Option<
        bevy::prelude::Res<
            ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry,
        >,
    >,
    // ⭐ **THE STAGE'S OWN DECLARATION**, read rather than re-stated. The floor a
    // kit-less seat gets used to be a helper in `select.rs`; it is
    // `DeclaredCombatRules::unarmed_melee` now, which is where a ruleset fact
    // lives. `Option` because this system runs before the resource exists on the
    // very first frame of a boot, and a screen with no rules yet has no floor to
    // hand out either.
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
    // **THE SEED FOR THIS MATCH'S RANDOM SQUARES.**
    //
    // ⚠ ADR 0023: no ambient RNG. This is the shell ACTIVATION this select
    // screen is running under — a monotonic id minted per route entry — so two
    // visits to the screen draw differently and one visit draws the same thing
    // twice if it somehow started twice. Mixed with the participant count so a
    // three-way and a two-way opened from the same visit do not walk the same
    // sequence.
    //
    // ⛔ NOT the wall clock, and not a thread RNG. A match is decided in
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
        *assignment,
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
        // ⭐⭐ **FROM THE DECLARATION THIS FUNCTION IS ABOUT TO MAKE, not from the
        // resource** (D143). `rules` was read here and the swipe never arrived:
        // this same system inserts `DeclaredCombatRules` fifty lines below, and
        // `Commands::insert_resource` is DEFERRED — so on the frame that decides
        // the match the resource does not exist yet and `None` was published.
        // Measured on the shipped select screen through its own taps:
        // `DeclaredCombatRules present = false`.
        // ⚠ and reading the resource would be wrong even once it existed: on a
        // SECOND visit it holds the PREVIOUS match's declaration, which is a
        // stale answer dressed as a live one. `declared_rules` is the value this
        // match declares, so the roster and the resource cannot disagree.
        declared_rules.unarmed_melee.clone(),
    ) else {
        return;
    };
    // ⭐ **THE SEAT COUNT THIS MATCH DECIDED, published with the roster and
    // under this experience's name.** Devices are not participants — a keyboard
    // seat has no controller entity, a spare pad may not be playing, a CPU seat
    // has none at all — so a session sized from what is plugged in is sized
    // wrong for every lobby that seats a CPU. Both land in the same flush that
    // asks for the route, so the session, which is built at least a frame later,
    // has never seen a smash gameplay world without them.
    commands.insert_resource(
        ambition_platformer2d::rollback::local_session::SessionSeatingSource::decided(
            SMASH_EXPERIENCE,
            // ⛔ **CHANNELS, not participants.** This said `participants.len()`,
            // so a one-person-one-CPU lobby built a two-handle rollback session
            // whose second handle nothing ever wrote — while
            // `freeze_local_seating_for_the_decided_match` counted humans for
            // the same decision, each citing itself as authoritative. A CPU is a
            // participant and occupies no channel; a lobby of two CPUs needs
            // none at all, which is the case that makes the difference
            // impossible to ignore.
            //
            // ⛔ **and the whole PLAN, not the count of it.** A count sizes the
            // session and leaves every consumer to guess which controller feeds
            // each handle — which they did, from the lobby's SPARSE source
            // numbers, so seating the CPU first put the human's fighter on a
            // channel the session never opened.
            decided.local_channel_plan(),
        ),
    );
    commands.insert_resource(decided);
    commands.insert_resource(declared_rules);
    // ⭐⭐ **AND THE PAD THIS GAME IS PLAYED ON** (D146 slice 3). Jon:
    // *"my preferred smash layout for an xbox controller is a=normal,
    // x=special, b=jump, y=grab (we don't have grab yet), left trigger is
    // shield. The rest of the bindings are normal I think"* — followed by the
    // ruling that makes it a declaration rather than an edit:
    // *"B=jump is the way I like my smash controller, it's probably non
    // standard. **Will need to have control profiles eventually.**"*
    //
    // ⛔ so this is emphatically NOT a change to `insert_gamepad_bindings`.
    // A=Jump stays right for Ambition; a fighting game says otherwise for the
    // duration of its own experience, and gives the pad back on the way out.
    // Same declare-don't-borrow shape as the rules above, owner and all —
    // the versus route is another provider in the same binary that could
    // eventually declare its own.
    //
    // ⭐ this is also the ONLY thing that gives gamepad-Special a button.
    // The default pad is fully assigned (`presets.rs` refuses to double-bind),
    // so Special was keyboard- and touch-only; a layout PERMUTES an assigned
    // pad, which is exactly the freedom an addition does not have.
    commands.insert_resource(ambition_platformer2d::input::DeclaredBindingLayout::new(
        SMASH_EXPERIENCE,
        ambition_platformer2d::input::BindingLayout::Smash,
    ));
    shell.write(ambition_platformer2d::game_shell::ShellCommand::GoTo(
        ambition_platformer2d::game_shell::ShellRouteId::new(SMASH_GAMEPLAY_ROUTE),
    ));
}

/// **The experience: what a launcher lists and a player can enter.**
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
        // **A LAUNCHER ROW LEADS TO THE QUESTION, NOT TO THE STAGE.** Without
        // this the only way into the select screen was to make it a whole app's
        // home route — which is what the demo's own shell does and no
        // multi-game host can, because its home lists games. Selecting "Smash"
        // in the Ambition title screen would have dropped a lone duelist onto
        // the platform with nobody to fight.
        // **THE STAGE'S OWN READOUTS.** Without this the route inherited
        // Ambition's adventure HUD and drew a health bar, a mana bar and a money
        // counter over a platform fighter (photographed 2026-08-01). Four slots
        // because the stage seats four; a 1v1 fills two and the publisher clears
        // the rest, the same rule the versus stage states.
        .with_hud({
            let mut hud = ambition_platformer2d::presentation::HudDeclaration::new();
            for (seat, slot) in FIGHTER_HUD_SLOTS.iter().enumerate() {
                hud = hud.slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(*slot)
                        .with_region(ambition_platformer2d::presentation::SurroundRegion::Top)
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
        .install(app, smash_prepared_session_world);
        app.add_plugins(SmashRulesPlugin::hosted());

        // **WHAT THIS EXPERIENCE OWNS, AND WHAT LEAVES WITH IT.**
        //
        // ⛔ the roster is what the reported regression was made of: pick Oni
        // Leader, quit to the title, enter Ambition, and the body you control is
        // still Oni Leader. A global resource with no lifetime is inherited by
        // whoever comes next.
        //
        // ⚠ **`covering` the select screen is load-bearing.** The lobby and the
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
                // ⛔ **by OWNER, and the owner is on the PLAN.** This removed the
                // activation by TYPE until 2026-08-07, as did the versus stage's
                // scope in the host that lists BOTH of us — so whichever left
                // first deleted the other's live match. `ActiveMatch` names the
                // SESSION whose plan it receipts and no publisher, so the plan
                // it came from is what answers "which GAME is this".
                //
                // ⚠ **two different questions, and both are needed.** The
                // session id (added 2026-08-07 so a finished match cannot be
                // rebuilt by its own activation) says WHICH ACTIVATION of one
                // game; the witness here says WHICH GAME, which is the only one
                // that matters when two providers share a host.
                .releasing_witnessed::<
                    ambition_platformer2d::actors::character_runtime::ActiveMatch,
                    ambition_platformer2d::actors::character_runtime::PreparedMatch,
                >(|plan, owner| plan.is_published_by(owner.as_str()))
                // ⛔ **AND THE PLAN, which is the same lesson one resource later.**
                // `PreparedMatch` is global and had no lifetime when it was
                // introduced, so it outlived every smash route — and because the
                // latch above IS released, the next experience's session found a
                // plan with no activation and dutifully built smash's fighters
                // into Ambition's world. Two bodies then carried
                // `Brain::Player(PRIMARY)` and `resolve_controlled_subject`
                // panicked on its own hard invariant, which is the loudest this
                // class of bug has ever been and the reason it took minutes
                // rather than the afternoon the roster version took.
                //
                // ⚠ declared AFTER the activation above, which reads it as its
                // witness: releases run in declaration order.
                .releasing_owned::<
                    ambition_platformer2d::actors::character_runtime::PreparedMatch,
                >(|plan, owner| plan.is_published_by(owner.as_str()))
                // **AND THE RULES LEAVE WITH THE MATCH.** Removing the
                // declaration IS the exit (AE6) — the projection folds it over
                // the world's baseline every tick and writes nothing back, so
                // there is no restore to skip. Left standing, this stage's DI
                // budget would follow the player into Ambition's PvE, which
                // answers `0.0` on purpose.
                //
                // ⚠ `releasing_owned`, not `resetting`: every reader takes it
                // as `Option<Res<_>>`, so absence is the meaningful "no
                // declaration" answer — and the OWNED form is what keeps two
                // stages that both declare rules from deleting each other's.
                .releasing_owned::<
                    ambition_platformer2d::combat::rules::DeclaredCombatRules,
                >(|rules, owner| rules.is_declared_by(owner.as_str()))
                // **AND THE PAD GOES BACK TO NORMAL.** Removing the declaration
                // IS the exit, exactly like the rules above: the layout is a
                // layer inside `BindingRecipe::build`, so the next rebuild
                // returns every seat to the base preset with nothing to restore.
                //
                // ⛔ this release is the whole difference between "a profile"
                // and "we changed the defaults". Left standing, B would jump in
                // Ambition after one smash match, and the bug would look like
                // the engine forgetting its own controls.
                .releasing_owned::<
                    ambition_platformer2d::input::DeclaredBindingLayout,
                >(|layout, owner| layout.is_declared_by(owner.as_str()))
                // A RESTART IS FRESH. `resetting`, never `releasing`: the
                // screen's systems take these as plain `ResMut`, so REMOVING
                // them panics the app on the frame the experience ends — which
                // is what the first draft did, and what the reproduction caught.
                // They must exist and must not carry the last match's answer.
                .resetting::<select::SmashSelect>()
                .resetting::<select_screen::StartRequested>()
                .resetting::<select_screen::cursor::SelectCursor>()
                .releasing_with("SessionSeatingSource", |world, owner| {
                    if let Some(mut seating) = world.get_resource_mut::<
                        ambition_platformer2d::rollback::local_session::SessionSeatingSource,
                    >() {
                        seating.release(owner.as_str());
                    }
                });
        }
    }
}

/// **How far a launched fighter may steer its own knockback**, in radians —
/// ~18°, Smash Ultimate's DI budget.
///
/// ⭐ this is the difference between a knock-off that is a READ and one that is
/// a coin flip: the victim of a launch is still playing. Authored per game
/// because Ambition's PvE answers `0.0` — being hit there is a punishment, not
/// the opening of a negotiation.
const SMASH_DI_MAX_ANGLE: f32 = 0.31;

/// **How hard a launch grows with the victim's percent** — a fraction of the
/// move's own base launch, per point of damage. `0.01` doubles a hit's launch at
/// 100%.
///
/// ⭐ **this is the mechanic Jon reported missing**: *"in smash there does not
/// seem to be any knockback."* Every piece of the engine was already there — the
/// growth term, hitstun and hitlag scaling off the resulting launch, DI steering
/// it — and the duelists reached none of it, because their swings come from the
/// `simple_melee` prefab and a prefab swing authors `knockback_growth: 0.0`. A hit at
/// 150% launched exactly as far as a hit at 0%, so percent accumulated and moved
/// nothing.
///
/// ⚠ **authored HERE, like the DI budget and the jump squat**, and for the same
/// reason: knockback that grows with damage is what a platform fighter IS and it
/// is wrong for Ambition's PvE, where being hit is a punishment rather than a
/// meter. The world baseline stays flat; a stage that wants the loop says so.
///
/// The number: a duelist's swipe launches at 120 px/s, so at 100 damage it
/// launches at 360 and at 200 damage at 600 — a fresh opponent is hard to move
/// and a worn one flies, which is the read the whole stage is built around.
///
/// ⚠ **bumped 0.01 → 0.02 (Jon, 2026-08-11)**: *"knockback multiplier in smash
/// is currently zero? I'd like to bump that number up so it's non zero."* It was
/// not literally zero, but doubling at 100% is barely a curve when a stock ends
/// somewhere north of 120% — the launch a player feels grows over the whole
/// match rather than at the end of it. Tripling at 100% is the genre's shape.
/// See `moveset.rs` for the unit trap that made the authored moves ignore this
/// entirely, which is the half that actually read as zero.
/// ⚠ **`pub` so the ROSTER-WIDE guard can read it.** `moveset.rs`'s unit check
/// only ever swept `fighter_moveset()` — the eleven-verb fallback the two robot
/// stand-ins carry — so the fourteen fighters who author their own tables were
/// outside the one guard that exists to catch this. The host census
/// (`smash_roster_movesets`) sweeps all of them and needs the declaration to
/// compare against.
pub const SMASH_KNOCKBACK_GROWTH: f32 = 0.02;

/// Stable ids the shell routes and lists this demo by.
pub const SMASH_EXPERIENCE: &str = "smash";
pub const SMASH_GAMEPLAY_ROUTE: &str = "smash_gameplay";
/// **Where the demo STARTS.** (Jon, 2026-07-31)
///
/// Not the stage. A platform fighter that opens on the stage has already decided
/// who you are, and the whole point of up-to-four-players is that it has not.
///
/// It is the demo app's HOME route (leaving a match returns to the screen that
/// chose it) AND the ENTRY route this experience advertises to any launcher, so
/// a multi-game host's "Smash" row opens the same question rather than dropping
/// a lone duelist onto the platform.
pub const SMASH_SELECT_ROUTE: &str = "smash_select";
/// **The select screen is its OWN shell experience, and it has to be.**
///
/// Not `smash`: an activation carrying the gameplay experience id starts a
/// gameplay SESSION, and this screen has no prepared world to activate — the
/// shell would panic with *"requires an exact prepared-session publication"*
/// before a single panel drew. Not the basic launcher's id either, which is
/// what the standalone demo used to say and why the select panels rendered on
/// top of a list of experiences. A screen a provider draws itself is a frontend
/// experience of its own.
pub const SMASH_SELECT_EXPERIENCE: &str = "smash.select";
/// The fighter a lone visitor wears. The MATCH seats its own cast from the
/// roster; this is who is standing there before one starts.
pub const SMASH_CHARACTER_ID: &str = "smash_duelist_a";
/// The opponent.
pub const SMASH_OPPONENT_ID: &str = "smash_duelist_b";

/// The logician.
pub const SMASH_GEORGE_BOOUL: &str = "smash_george_booul";

// **THE ONE FIGHTER THIS DEMO ADDS TO THE CROSSOVER.**
//
// ⭐ he wears a sheet that ALREADY SHIPS and that no other catalog claims, which
// is the only kind of fighter this demo may declare: the rest of the grid is
// Ambition's own cast and the other demos' protagonists, named by ID in
// `select::SMASH_ROSTER` rather than copied here. The two robot rows below are
// STAND-INS for the lineage the content catalog owns; see `select::STAND_INS`.
//
// ⚠ **he could not be SEATED for two commits and nothing on the select screen
// said so** — he rendered perfectly, and the tell was a stage test finding no
// stocks. Jon: *"I liked him there."* So the seating gap is the thing that got
// fixed, not the roster.
//
// ⚠ **every fighter shares one kit.** See `SmashSelect::roster` — one ability
// set, one brain, one action set. Different LOOKS and one game. Per-character
// movement, reach and weight is the obvious next question and is deliberately
// not this one; a roster where the choice already changed the match would have
// made the select screen impossible to judge on its own terms.

/// ⚠ **this demo authors its own two fighters, and the reason is a leak worth
/// recording.**
///
/// The first version borrowed Ambition's robot lineage — a crossover stage
/// fighting the cast the game already ships, which is the more interesting
/// claim. It does not compile as a claim: that lineage lives in
/// `game/ambition_content`, which is ABOVE the facade, so a demo naming it would
/// break the `ambition_platformer2d` + `bevy` rule that makes this crate an oracle at all.
///
/// The engine caught it the only way it could — at BOOT, with
/// `character_catalog: Resource does not exist`, because the demo had declared a
/// starting character no catalog in its own composition contained. Not at
/// compile time, and not by any test in the content crate: only by running.
///
/// So the demo is self-contained, and the crossover claim moves to where it
/// belongs — Ambition HOSTING this experience alongside its own, where both
/// catalogs are present.
const SMASH_CATALOG_RON: &str = r#"(
    autonomous_profiles: {
        // ⭐⭐ **THE STAGE'S CPU POLICY, PUBLISHED** (Jon's second redirect, P4).
        // A CPU seat named `duelist` and the match resolved it through
        // `CharacterRoster` — an enemy ARCHETYPE table — so the controller half
        // of `character + controller + team` was arriving by way of a body
        // definition. This is what a controller policy IS.
        //
        // ⚠ the numbers are the archetype row's controller half verbatim. Its
        // BODY half (100 HP, 200 run speed, a 4-damage contact) stays on the row
        // until the fighters that use it author their own, which is a different
        // migration.
        "duelist": (
            template: Fighter,
            aggro_radius: 600.0,
            attack_range: 48.0,
            patrol_effort: 1.0,
            chase_effort: 1.0,
            fighter_level: 5,
        ),
        // ⭐⭐ **THE DIFFICULTY LADDER, AS POLICIES** — the whole of what
        // `SMASH_ROSTER_RON`'s six archetype rows were (deleted 2026-08-11).
        // Those rows differed from each other in ONE field, `fighter_level`, and
        // carried a body (100 HP, 200 run speed, a 4-damage contact) that no
        // seat has read since a fighter's body came from its character.
        //
        // ⚠ that is the whole D73 thesis in six rows: a difficulty setting is a
        // CONTROLLER fact, and stating it required declaring a whole creature.
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
    },
    brain_presets: {
        "stand_still": StandStill,
        // **The FB4b fighter brain, selected from content.** Until 2026-07-31
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
            // ⚠ NOT "Duelist A" (Jon, 2026-08-05). It wears `player_robot_v3`'s
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
            // ⭐ **AUTHORED, 2026-08-11.** This said `HostCode` while the line
            // above authored a `duelist` action set — so the row declared a kit
            // and then asked engine code to build a different one. `HostCode`
            // exists to be deleted (GPT 5.6 §5); a row that already authors its
            // kit is the cheapest adopter to remove.
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
/// ⚠ **thin, but not empty — and the difference is a refusal that fired.** The
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
    // **REGISTER the characters, not only their catalog rows.** A catalog
    // fragment declares what a character IS; registration is what makes the art
    // pipeline know it exists — `declare_registered_characters` reads the
    // PREPARED REGISTRY, so a catalog-only character draws the marked
    // placeholder. Pocket shipped that way and nobody noticed until somebody
    // looked at the screen.
    {
        use ambition_platformer2d::actors::character_runtime::{
            CharacterDefinition, CharacterDefinitionAppExt,
        };
        // ⛔ **EVERY id this demo can SEAT, not just the two it opens with.**
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
            // ⛔⛔ **THE PERCENT REFERENCE IS NOT WRITTEN HERE ANY MORE**
            // (2026-07-31 found it; queue D131 moved it, 2026-08-16).
            //
            // It used to be `definition.vitals.max_health =
            // Some(SMASH_PERCENT_REFERENCE)`, on this line, for these three ids
            // — and it was right about the symptom and wrong about the owner. A
            // character that authors no vitals gets a ONE-HIT pool, and
            // `damage_percent()` is `accumulated / max`, so a 140-damage hit
            // read as **14000%**. Stamping the reference onto the characters
            // this demo happens to REGISTER fixed the three fighters it could
            // reach and could never reach the other eleven: `mary_o` and `sanic`
            // walked onto the same stage carrying the `max_health: 1` their own
            // one-hit-kill games authored, and read 4200% and 800%.
            //
            // ⭐ what 100% means is a rule of the MATCH, so
            // `apply_smash_match_rules` declares it and seating applies it to
            // every seat — see `MatchParticipantRoster::fighter_health_pool`.
            // These three now author what they ARE and nothing about how a
            // stocks match reads them.
            //
            // ⛔⛔ **AND NEITHER IS THE KNOCKBACK WEIGHT** (D146 slice 4,
            // 2026-08-16). `definition.vitals.knockback_weight = Some(match id
            // { .. })` stood on this line: a reach-in performed mid-loop, on the
            // way past, while the ruleset's other two adjustments went through
            // one named composition site. The values and their reasoning are
            // unchanged — they are in
            // [`smash_reading_of_character`], which is a pure function from what
            // a character authored to what this match's seat plays with.
            //
            // ⚠ **that move implies NO direction.** Whether per-character
            // per-game properties belong to the character or to the game is
            // deliberately still open; the seam exists so the answer is one edit
            // either way.
            let mut definition = smash_reading_of_character(definition);
            // ⛔⛔ **THE PLATFORM FIGHTER'S BODY IS NOT AUTHORED HERE ANY MORE**
            // (2026-07-31 authored it; D146 slice 1b moved it, 2026-08-16).
            //
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
            // ⭐⭐ **WHAT IS LEFT IS THE OTHER HALF OF WHAT THAT BLOCK WAS
            // SAYING, and it was invisible until the six moved.** It spread
            // `..DEFAULT_TUNING`, so it also declared these three to be
            // PLAYER-GRADE bodies — gravity 2500, run accel 5200, air accel
            // 3100 — while a seat that authors nothing takes
            // `BodyMovementTuning::BASELINE`, the generic ACTOR body: gravity
            // 1450, run accel 650. Deleting the line outright made George
            // floaty and sluggish, and the smash app's own repertoire probes
            // caught it in one run (three distinct moves out of sixteen, and no
            // recovery thrown in 1800 ticks).
            //
            // ⚠ **so it is stated deliberately now, as the one thing it means.**
            // ⛔ **and it is a FINDING, not a resolution: eleven of the fourteen
            // fighters on the grid still play on the ACTOR baseline** — a
            // levelled stage where thirteen bodies are floatier than the
            // fourteenth is half a decision, and which base a platform fighter
            // uses is a product call rather than a side effect of this commit.
            // Filed for a later slice.
            definition.movement_tuning = Some(ambition_platformer2d::engine_core::DEFAULT_TUNING);
            // **WHAT THIS FIGHTER'S BODY CAN DO — authored on the CHARACTER,
            // which is why the shield, the dodge and the ledge exist in this
            // demo at all.**
            //
            // ⛔ the machinery was all already there and unreachable. The engine
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
            // ⚠ `fly`/`blink` deliberately absent: this is a platform fighter's
            // ground game, not the exploration protagonist's traversal kit, and
            // the July measurement of two seats disagreeing was exactly a
            // duelist meeting a body that could fly. `dash` left for the same
            // reason on 2026-08-16 — see [`SMASH_FIGHTER_KIT`], which this must
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
            // **THE REPERTOIRE, ON THE CHARACTER.**
            //
            // ⭐ this is what stops the seat needing `smash_fighter_kit()`: a
            // definition that authors its own moveset says something more
            // specific than anything derivable from an action-set preset, and
            // preparation uses it verbatim. Eleven moves — jab, two tilts, three
            // smashes, five aerials — where the seat used to carry one swipe
            // that answered every direction and both strengths.
            // ⚠ **the shared table is the STAND-INS', and George has his own.**
            // `smash_duelist_a/b` stand in for `player_robot_v3`/`v2`, whose
            // canonical repertoire lives on the real Robot provider and reaches
            // them when a host composes it (redirect §15) — so a third robot
            // table here would be the copy that redirect forbids. George is the
            // one fighter this demo owns, and he is the one who gets authored.
            definition = definition.with_moveset(if id == SMASH_GEORGE_BOOUL {
                crate::george_booul_moveset::george_booul_moveset()
            } else {
                crate::moveset::fighter_moveset()
            });
            app.register_character(definition);
        }
    }
    // ⛔⛔ **A COMMENT ABOUT A DELETED ARCHETYPE FRAGMENT STOOD HERE, ABOVE THE
    // AUDIO REGISTRATION** (removed 2026-08-13). It read *"THE ARCHETYPE A CPU
    // SEAT ACTUALLY NAMES… `ControllerBinding::Cpu { brain_profile }` is a
    // `CharacterRoster` key, not a catalog preset… Without this fragment the seat
    // is now REFUSED"*, and the fragment it described went with `SMASH_ROSTER_RON`
    // in ledger D87. It survived the deletion, drifted onto the next statement,
    // and told a reader that a `brain_profile` is an archetype key — which is the
    // opposite of what this demo proves. A CPU seat names a PUBLISHED policy
    // (`autonomous_profiles`, above), and since campaign P2.18 there is nowhere
    // else it could come from.
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
    // ⛔ **`for_match`, so this stage builds NO home body.** It used to build one
    // wearing `SMASH_CHARACTER_ID`, and match seating then adopted that body for
    // the human seat — which is where every symptom of Jon's 2026-08-06 report
    // came from. The match realizes its own cast; the id below is only this
    // experience's catalog DEFAULT, which its worn fighters still fall back to.
    ambition_platformer2d::runtime::PreparedPlatformerSource::for_match(
        SMASH_EXPERIENCE,
        RoomSet::from_parts(SMASH_STAGE_ROOM_ID, vec![room], Vec::new()),
        geometry,
        metadata,
        StartingCharacter::new(SMASH_CHARACTER_ID),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::engine_core::AabbExt;

    /// **THE STAGE OPENS A WINDOW FOR EVERY VERB IT GRANTS.** (D146 slice 1b)
    ///
    /// ⛔⛔ **a granted verb whose tuning window is zero is a DEAD GRANT**, and
    /// it is invisible: nothing refuses it, nothing logs it, and the press
    /// simply means nothing. [`MatchAbilities::is_coherent`] asks the same
    /// question about the two ABILITY statements — *is everything granted also
    /// permitted* — and this is the same question one layer down, against the
    /// numbers the verbs run on.
    ///
    /// ⚠ **the pairs are hand-listed and that is the point**: adding a verb to
    /// [`SMASH_FIGHTER_KIT`] whose window the engine defaults to zero is exactly
    /// the mistake this catches, and only a list written against the KIT can
    /// catch it. The air dodge is here because it was the one that bit; the
    /// others are here because they are the rest of what the stage promises.
    #[test]
    fn the_stages_body_opens_a_window_for_every_verb_the_stage_grants() {
        // What a fighter that brought nothing of its own plays with here: the
        // stage's numbers over the engine's, which is the body twelve of the
        // fourteen grid fighters actually get.
        let body = SMASH_FIGHTER_BODY.over(ambition_platformer2d::engine_core::DEFAULT_TUNING);
        let kit = SMASH_FIGHTER_KIT;
        let dead: Vec<&str> = [
            // (granted?, the number without which the verb does nothing, name)
            (kit.dodge, body.air_dodge_time, "dodge (in the air)"),
            (kit.dodge, body.dodge_roll_time, "dodge (on the ground)"),
            (kit.dodge, body.dodge_roll_speed, "dodge (on the ground)"),
            (kit.double_jump, f32::from(body.air_jumps), "double_jump"),
            (kit.fast_fall, body.fast_fall_speed, "fast_fall"),
            (kit.shield, body.parry_window_time, "shield (the parry)"),
            (
                kit.ledge_grab,
                body.ledge_momentum.window,
                "ledge_grab (the momentum carry)",
            ),
            (kit.pogo, body.pogo_speed, "pogo"),
        ]
        .into_iter()
        .filter(|(granted, window, _)| *granted && *window <= 0.0)
        .map(|(_, _, verb)| verb)
        .collect();
        assert!(
            dead.is_empty(),
            "the stage GRANTS {dead:?} and supplies a body in which the verb \
             does nothing — see `MatchParticipantRoster::fighter_body`"
        );
        // ⛔ **NON-VACUITY, and it is the whole test.** Every window above is
        // non-zero in `DEFAULT_TUNING` EXCEPT the air dodge, which the engine
        // holds at 0.0 deliberately — so a body that had stopped carrying the
        // stage's own numbers would still pass the loop above.
        assert_eq!(
            ambition_platformer2d::engine_core::DEFAULT_TUNING.air_dodge_time,
            0.0,
            "the engine opened an air-dodge window by default, which changes \
             every exploration body in the game and makes this test vacuous"
        );
        assert!(
            body.air_dodge_time > 0.0,
            "the stage's body no longer opens the one window the engine \
             deliberately leaves shut"
        );
    }

    /// **AND THE ROSTER IS WHERE IT SAYS SO.** The test above measures the
    /// constant; this measures that the stage actually declares it, which is the
    /// half that can be deleted without breaking a compile.
    #[test]
    fn the_roster_supplies_the_fighters_body() {
        let roster = smash_roster(["player_robot_v3", "player_robot_v2"]);
        assert_eq!(
            roster.fighter_body,
            Some(SMASH_FIGHTER_BODY),
            "the stage grants a platform fighter's verbs and supplies no body \
             to run them on"
        );
    }

    /// **A stocks roster declares the pair the engine insists on.**
    #[test]
    fn the_roster_declares_stocks_for_every_seat() {
        let roster = smash_roster(["player_robot_v3", "player_robot_v2"]);
        assert_eq!(roster.fighter_stocks, Some(STARTING_STOCKS));
        assert_eq!(roster.participants.len(), 2);
        assert!(
            roster.opens_suspended,
            "a fighter that can act during the countdown gets a free hit"
        );
    }

    /// Seat 0 is the human; everyone else is a CPU. The demo is playable with
    /// one controller, which is the difference between a demo and a fixture.
    #[test]
    fn the_first_seat_is_the_player_and_the_rest_are_cpus() {
        let roster = smash_roster(["a", "b", "c"]);
        assert!(matches!(
            roster.participants[0].controller,
            ControllerBinding::Human {
                source: ambition_platformer2d::actor::LocalInputSource::Pad(0)
            }
        ));
        for participant in &roster.participants[1..] {
            assert!(matches!(
                participant.controller,
                ControllerBinding::Cpu { .. }
            ));
        }
    }

    /// Every seat is its own side, so a free-for-all actually resolves: a
    /// four-way where everyone shares a team can never have a last side standing.
    #[test]
    fn every_seat_is_its_own_side() {
        let roster = smash_roster(["a", "b", "c", "d"]);
        let sides: std::collections::BTreeSet<_> = roster
            .participants
            .iter()
            .filter_map(|participant| participant.team.clone())
            .collect();
        assert_eq!(
            sides.len(),
            4,
            "seats share a side, so this match cannot end: the last-side-standing \
             rule never sees fewer than two"
        );
    }

    /// **Two fighters do not come back to the same point.**
    ///
    /// ⛔⛔ **they did** — `respawn_placement` took no seat, so a double knockout
    /// put both bodies over the centre of the stage inside one another, at the
    /// exact moment neither has information or options (D128 defect 3).
    ///
    /// ⭐ the arrangement is symmetric about the centre and stays ON the
    /// platform, which is the pair of properties that makes it a placement
    /// rather than an offset: an eight-seat roster is still a fair start.
    #[test]
    fn every_seat_comes_back_to_its_own_point_on_the_platform() {
        let centre = stage_centre();
        let seats: Vec<Vec2> = (0..8).map(|seat| respawn_placement(centre, seat)).collect();

        for (a, first) in seats.iter().enumerate() {
            for second in seats.iter().skip(a + 1) {
                assert!(
                    (first.x - second.x).abs() >= RESPAWN_SEAT_SPACING_PX - 0.01,
                    "two seats respawn within {RESPAWN_SEAT_SPACING_PX}px of each \
                     other, which is narrower than a standing body: {first:?} vs {second:?}"
                );
            }
        }

        // Symmetric about the centre: seat 0 and seat 1 straddle it evenly, so
        // no seat is handed the better return.
        assert!(
            ((seats[0].x - centre.x) + (seats[1].x - centre.x)).abs() < 0.01,
            "the first two seats are not symmetric about the stage centre"
        );

        // ⚠ and every one of them is still over the platform, not past its lip —
        // an offset that grew without bound would respawn seat 7 into the blast
        // zone, which is a worse bug than the overlap it fixed.
        let half = PLATFORM_WIDTH / 2.0;
        for (seat, at) in seats.iter().enumerate() {
            assert!(
                (at.x - centre.x).abs() < half,
                "seat {seat} respawns {:.0}px from centre, past the {half:.0}px platform edge",
                (at.x - centre.x).abs()
            );
            assert!(
                at.y < centre.y,
                "seat {seat} respawns at or below the stage"
            );
        }
    }

    /// **A respawn is ABOVE the stage, not on it.** A fighter that comes back on
    /// the floor comes back inside the opponent who just knocked it off.
    ///
    /// ⚠ this asserted `respawn.x == centre.x` until 2026-08-18, which was the
    /// SEAT-INDEPENDENCE defect stated as an invariant — every fighter returning
    /// to one point is exactly what D128 defect 3 was. The height is this test's
    /// subject; the column belongs to
    /// `every_seat_comes_back_to_its_own_point_on_the_platform`.
    #[test]
    fn a_respawn_is_above_the_stage_centre() {
        let centre = Vec2::new(400.0, 300.0);
        let respawn = respawn_placement(centre, 0);
        assert!(
            (respawn.x - centre.x).abs() <= RESPAWN_SEAT_SPACING_PX,
            "a respawn is within a seat spacing of the centre, not off across the stage"
        );
        assert!(
            respawn.y < centre.y,
            "the respawn is at or below the stage floor, so a returning fighter \
             materialises inside whatever is standing there"
        );
    }

    /// **The stage is a platform surrounded by nothing**, which is the one room
    /// shape this engine had not loaded. Every other room is a box you cannot
    /// leave.
    #[test]
    fn the_stage_is_a_platform_you_can_be_knocked_off() {
        let room = smash_stage();
        assert_eq!(room.id, SMASH_STAGE_ROOM_ID);
        assert_eq!(
            room.world.blocks.len(),
            1,
            "a fighter stage with walls is a room, and a body knocked into one \
             comes back — the emptiness IS the mechanic"
        );
        let platform = room.world.blocks[0].aabb;
        assert!(
            platform.width() < room.world.size.x,
            "the platform spans the stage, so there is no off to be knocked"
        );
    }

    /// **The blast envelope is authored from the fighting platform.**
    ///
    /// The room rectangle is an implementation seam, not the thing whose size
    /// should determine knockout timing. Pin the normalized Final Destination
    /// proportions directly so a future room resize cannot silently move the
    /// death lines relative to the ledges.
    #[test]
    fn the_stage_and_blast_envelope_keep_their_authored_proportions() {
        let room = smash_stage();
        let world = &room.world;
        let platform = world.blocks[0].aabb;
        let side_margin = world
            .side_blast_margin
            .expect("the smash stage authors side blast lines");
        let ceiling_margin = world
            .ceiling_blast_margin
            .expect("the smash stage authors a ceiling blast line");

        let left_ledge_to_blast = platform.left() + side_margin;
        let right_ledge_to_blast = (world.size.x - platform.right()) + side_margin;
        let surface_to_ceiling_blast = platform.top() + ceiling_margin;
        let surface_to_fall_blast = (world.size.y - platform.top()) + world.blast_margin;

        assert_eq!(platform.width(), PLATFORM_WIDTH);
        assert_eq!(left_ledge_to_blast, PLATFORM_WIDTH);
        assert_eq!(right_ledge_to_blast, PLATFORM_WIDTH);
        assert_eq!(surface_to_ceiling_blast, PLATFORM_WIDTH * 1.125);
        assert_eq!(surface_to_fall_blast, PLATFORM_WIDTH * 0.875);
        assert_eq!(world.size.x + side_margin * 2.0, PLATFORM_WIDTH * 3.0);
        assert_eq!(
            world.size.y + ceiling_margin + world.blast_margin,
            PLATFORM_WIDTH * 2.0
        );
    }

    /// The room carries the demo's MODE, so its rules sleep everywhere else.
    #[test]
    fn the_stage_carries_the_smash_mode() {
        assert_eq!(smash_stage().metadata.mode.as_deref(), Some(SMASH_MODE));
    }

    /// A respawn lands above the PLATFORM, not above the stage's arbitrary
    /// middle — the two coincide here and a future stage will separate them.
    #[test]
    fn a_respawn_lands_over_the_platform() {
        let room = smash_stage();
        let platform = room.world.blocks[0].aabb;
        let respawn = respawn_placement(stage_centre(), 0);
        assert!(
            respawn.x >= platform.left() && respawn.x <= platform.right(),
            "a respawning fighter is dropped past the edge of the platform it is \
             supposed to come back to"
        );
        assert!(
            respawn.y < platform.top(),
            "the respawn is not above the stage"
        );
    }

    /// Run `announce_the_winner` over one decision and hand back the announce
    /// slot's text, or `None` if nothing was written to it.
    fn announced_outcome(winner: Option<&str>) -> Option<String> {
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_message::<ambition_platformer2d::actor::StocksMatchDecided>();
        app.init_resource::<ambition_platformer2d::presentation::HudReadouts>();
        app.add_systems(Update, announce_the_winner);
        app.update();

        app.world_mut()
            .resource_mut::<Messages<ambition_platformer2d::actor::StocksMatchDecided>>()
            .write(ambition_platformer2d::actor::StocksMatchDecided {
                winner: winner.map(str::to_string),
            });
        app.update();

        app.world()
            .resource::<ambition_platformer2d::presentation::HudReadouts>()
            .get(&SMASH_ANNOUNCE_HUD_SLOT.into())
            .map(ambition_platformer2d::presentation::HudReadout::text)
    }

    /// **The CARD says who won.**
    ///
    /// The plugin's half of the seam, driven through the message the engine
    /// actually writes rather than by calling `victory_banner` directly — which
    /// would test the string and not the wiring.
    ///
    /// ⛔ **it asserted a `GameplayBannerRequested` until 2026-08-15, and that is
    /// why the winner card was invisible while this test was green.** Nothing in
    /// the workspace DRAWS a `GameplayBanner`: its only reader is the app's debug
    /// HUD line, gated on `player.single()`, so a CPU-versus-CPU ending showed
    /// nothing at all. The claim is now made against the readout the stage
    /// declares and the HUD actually renders — the same road as the fighter
    /// percents — which is strictly the stronger thing to assert.
    ///
    /// ⚠ the old test also guarded *"a ruleset that announces twice announces on
    /// every frame after the match ends"*. That hazard is gone by construction
    /// rather than by assertion: a readout is a map insert, so writing it twice
    /// is writing it once.
    #[test]
    fn deciding_the_match_shows_a_card_naming_the_winner() {
        // ⚠ the WORDING comes from `victory_banner`, which is where it is
        // decided; this fixture seats no bodies, so the card falls back to the
        // side label and that fallback is part of what is being asserted.
        assert_eq!(
            announced_outcome(Some("seat 2")).as_deref(),
            Some(victory_banner(Some("seat 2")).as_str()),
            "the ending wrote no announce card, so the stage says nothing about \
             who won"
        );
    }

    /// A DRAW reaches the card as a draw, not as a winner with an empty name.
    #[test]
    fn a_drawn_match_is_announced_as_one() {
        let said = announced_outcome(None).expect("a draw is still an ending");
        assert!(
            said.contains("Draw"),
            "a draw was announced as a win: {said}"
        );
    }

    /// **The demo is something a player can ENTER.**
    ///
    /// Until this existed the crate was three correct pieces nobody could reach:
    /// a roster, a stage and a ruleset, all unit-true and unassembled. That is
    /// the shape this repo keeps catching — everything passes and nothing runs —
    /// and a demo is the one kind of crate where it is indistinguishable from
    /// working, because nobody notices a game they cannot start.
    #[test]
    fn a_host_composing_this_plugin_can_route_to_the_stage() {
        use ambition_platformer2d::game_shell::{
            MinimalShellPlugins, ShellExperienceId, ShellExperienceRegistry, ShellRouteCatalog,
            ShellRouteId,
        };
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_plugins(MinimalShellPlugins);
        app.add_plugins(ambition_platformer2d::load::AmbitionLoadPlugin);
        app.add_plugins(SmashExperiencePlugin);

        let registration = app
            .world()
            .resource::<ShellExperienceRegistry>()
            .get(&ShellExperienceId::new(SMASH_EXPERIENCE))
            .expect("a host that composed this plugin lists the smash experience");
        assert_eq!(
            registration.launch_route.as_str(),
            SMASH_SELECT_ROUTE,
            "a launcher row for this demo opens CHARACTER SELECT; entering at the \
             stage would seat whoever the host happened to have lying around"
        );
        let select = app
            .world()
            .resource::<ShellRouteCatalog>()
            .get(&ShellRouteId::new(SMASH_SELECT_ROUTE))
            .expect("the select screen is a registered route, not an app's home only");
        assert_eq!(
            select.experience.as_str(),
            SMASH_SELECT_EXPERIENCE,
            "the screen is a frontend experience of its own: under the gameplay id \
             the shell would try to activate a session that has nothing prepared"
        );
        assert!(
            select.preparation.is_none(),
            "nothing is loading on a character select"
        );

        let route = app
            .world()
            .resource::<ShellRouteCatalog>()
            .get(&ShellRouteId::new(SMASH_GAMEPLAY_ROUTE))
            .expect("the session route is registered");
        assert!(
            route.preparation.is_some(),
            "the route has no preparation, so entering it would drop a player into \
             a stage whose content was never prepared"
        );

        let authored = app
            .world()
            .resource::<ambition_platformer2d::provider::PlatformerAuthoredCatalogRegistry>()
            .get(SMASH_EXPERIENCE)
            .expect("the host sees this demo's authored catalogs");
        assert_eq!(authored.starting_character, SMASH_CHARACTER_ID);
    }

    /// ⭐⭐ **the stage declares a DI budget, and gives it back on the way out.**
    ///
    /// ⛔ the DI law, its tuning field and the victim's live stick were all
    /// wired, and this demo declared no combat rules at all — so `di_max_angle`
    /// fell to the engine baseline of `0.0` and directional influence was OFF on
    /// the one stage built to need it. Nothing failed; a launched fighter simply
    /// had no say, and a knock-off was a coin flip instead of a read.
    ///
    /// The release is the other half and the more dangerous one: left standing,
    /// this budget follows the player into Ambition's PvE, which answers `0.0`
    /// on purpose.
    #[test]
    fn the_stage_declares_its_di_budget_and_releases_it() {
        use ambition_platformer2d::game_shell::{MinimalShellPlugins, ShellExperienceScopes};
        use bevy::prelude::*;

        assert!(
            SMASH_DI_MAX_ANGLE > 0.0,
            "⛔ a zero budget makes `di_adjust` a no-op, so declaring the rules              at all would be theatre — DI would be off and every test still green"
        );
        // ⛔ **the same trap, one field over** (queue D75). Zero growth makes
        // `scaled_knockback` return the base immediately, so percent would
        // accumulate and launch nothing — which is exactly the state Jon
        // reported as "there does not seem to be any knockback", with every
        // test green because the engine was working on a number nobody set.
        assert!(
            SMASH_KNOCKBACK_GROWTH > 0.0,
            "⛔ a platform fighter whose launch does not grow with percent is a \
             fighting game with no comeback and no kill: every basic swing here \
             is prefab-derived and authors `knockback_growth: 0.0`, so this declaration \
             is the ONLY thing that makes a worn opponent fly"
        );

        let mut app = App::new();
        app.add_plugins(MinimalShellPlugins);
        app.add_plugins(ambition_platformer2d::load::AmbitionLoadPlugin);
        app.add_plugins(SmashExperiencePlugin);

        let rules =
            std::any::type_name::<ambition_platformer2d::combat::rules::DeclaredCombatRules>();
        let released: Vec<&str> = app
            .world()
            .resource::<ShellExperienceScopes>()
            .iter()
            .filter(|scope| scope.owner().as_str() == SMASH_EXPERIENCE)
            .flat_map(|scope| scope.released_state())
            .collect();
        assert!(
            released.contains(&rules),
            "⛔ the stage's DI budget outlives its own experience and follows the              player into a game that authored none. Released: {released:?}"
        );
    }

    /// **The prepared source carries the stage, not a default room.**
    ///
    /// The preparation seam takes a closure, and a closure that returns the
    /// wrong room fails nowhere: the route prepares, the session starts, and the
    /// player lands in somebody else's level.
    #[test]
    fn the_prepared_session_is_the_smash_stage() {
        let prepared = smash_prepared_session_world();
        assert_eq!(
            prepared.starting_character().character_id.as_str(),
            SMASH_CHARACTER_ID
        );
        assert_eq!(
            prepared.geometry().0.blocks.len(),
            1,
            "the prepared geometry is not the one-platform stage"
        );
        assert_eq!(
            prepared.geometry().0.side_blast_margin,
            Some(SIDE_BLAST_MARGIN_PX),
            "the prepared geometry lost the stage's blast margins, so a fighter \
             knocked off would drift instead of dying"
        );
    }

    /// **The MATCH declares what 100% means, so a crossover fighter cannot bring
    /// its own.** (2026-07-31 found the number; queue D131 found the owner,
    /// 2026-08-16)
    ///
    /// A character that authors no vitals gets a ONE-HIT pool, and under
    /// `DeathPolicy::Unbounded` the pool never kills — so nothing goes wrong
    /// except the number, and the number is the entire user-facing output of the
    /// stocks model. A 140-damage hit read as 14000%, with every test green:
    /// the meter accumulated correctly and divided correctly, by a denominator
    /// nobody had authored.
    ///
    /// ⛔⛔ **and the first fix was per-CHARACTER, which is why it held for a
    /// fortnight and then failed on eleven fighters.** This demo stamped the
    /// reference onto the three ids it registers; every other name on
    /// [`select::SMASH_ROSTER`] belongs to another game. Mary-O and Sanic author
    /// `max_health: 1` — correct for a one-hit-kill platformer — and read 4200%
    /// and 800% off ordinary melee damage on this stage.
    ///
    /// ⚠ so the assertion is about the ROSTER, not about a catalog row: the
    /// character-side write is DELETED and re-adding it would not make this pass.
    #[test]
    fn the_match_declares_the_pool_every_fighters_percent_is_read_against() {
        let mut roster =
            ambition_platformer2d::actor::MatchParticipantRoster::of(["mary_o", "sanic"]);
        apply_smash_match_rules(&mut roster);
        assert_eq!(
            roster.fighter_health_pool,
            Some(SMASH_PERCENT_REFERENCE),
            "a stocks match that does not declare its own pool reads each seat's \
             percent against whatever that character's HOME GAME authored"
        );
        // The reference is what makes a percent comparable across characters.
        // One would make every hit read in the thousands.
        assert!(
            SMASH_PERCENT_REFERENCE >= 50,
            "a percent reference of {SMASH_PERCENT_REFERENCE} makes a single hit \
             read in the hundreds, which is the 14000% bug in a smaller hat"
        );
    }

    /// **The `duelist` preset resolves to the FIGHTER brain.**
    ///
    /// A preset name that does not resolve falls back to standing still, and a
    /// fighter that stands still is indistinguishable from one whose brain was
    /// never installed — which is what the match diagram printed for an hour
    /// **EVERY DIFFICULTY THIS DEMO CAN ASK FOR IS A PUBLISHED POLICY.**
    ///
    /// ⭐ the guard on a deletion (ledger D87). `SMASH_ROSTER_RON` was six
    /// archetype rows existing only to answer a CPU seat's controller question —
    /// each carrying a body (100 HP, 200 run speed, a 4-damage contact) that no
    /// seat has read since a fighter's body came from its character, and
    /// differing from one another in exactly one field. They are gone, and
    /// `smash_roster_at_levels` builds `duelist_l{level}` keys that now have to
    /// resolve as `autonomous_profiles`.
    ///
    /// ⚠ what a miss looks like: `seat_brain_profile` finds nothing in either
    /// authority and preparation REFUSES the seat — loud, not a fighter that
    /// quietly stands still, which is how the same lookup failed twice before.
    #[test]
    fn every_authored_difficulty_is_a_published_controller_policy() {
        use ambition_platformer2d::characters::actor::character_catalog::{
            parse_catalog, CharacterCatalog,
        };

        let catalog = CharacterCatalog::from_data(parse_catalog(SMASH_CATALOG_RON));
        let profiles = &catalog.data().autonomous_profiles;
        for level in [1u8, 3, 5, 6, 9] {
            let key = format!("{SMASH_DUELIST_BRAIN}_l{level}");
            let profile = profiles.get(&key).unwrap_or_else(|| {
                panic!(
                    "`smash_roster_at_levels` builds the key `{key}`, and no policy \
                     publishes it — that seat is refused. Published: {:?}",
                    profiles.keys().collect::<Vec<_>>()
                )
            });
            assert_eq!(
                profile.fighter_level, level,
                "`{key}` publishes level {} — the ladder was the ONLY thing the \
                 six deleted archetype rows differed in, so getting it wrong \
                 loses the entire content of that deletion",
                profile.fighter_level
            );
            assert_eq!(
                profile.template,
                ambition_platformer2d::characters::brain::CharacterBrainTemplate::Fighter,
                "`{key}` is not a Fighter, so this seat is not a fighter"
            );
        }
        // ⚠ and the unlevelled name the roster's default seats use.
        assert!(
            profiles.contains_key(SMASH_DUELIST_BRAIN),
            "the bare `duelist` policy is gone, so an ordinary CPU seat is refused"
        );
    }

    /// (`travel: [0.0, 0.0]`) before anything said why.
    #[test]
    fn the_duelist_preset_is_a_fighter_brain() {
        use ambition_platformer2d::characters::actor::character_catalog::{
            parse_catalog, CharacterCatalog,
        };

        let catalog = CharacterCatalog::from_data(parse_catalog(SMASH_CATALOG_RON));
        assert!(
            catalog.has_brain_preset("duelist"),
            "the catalog does not know the `duelist` preset at all, so every \
             fighter asking for it silently stands still"
        );
        let brain = catalog
            .build_brain_from_preset(
                "duelist",
                &ambition_platformer2d::characters::actor::character_catalog::BrainBuildContext::at(
                    0.0,
                ),
            )
            .expect("the `duelist` preset builds a brain");
        assert_eq!(
            brain.label(),
            "fighter",
            "`duelist` resolved to `{}` — a preset that does not resolve falls \
             back to standing still, and a fighter that stands still looks \
             exactly like one with no brain at all",
            brain.label()
        );
    }

    /// A draw has a name. The engine's `winner: Option<String>` exists so this
    /// case does not need a sentinel, and the banner has to honour that.
    ///
    /// ⭐ **and a winner is announced as one, in Jon's words** (D140): the card
    /// reads `WINNER: <name>` rather than `<side> wins`, because a player
    /// looking at the end of a match should be told who WON rather than be
    /// handed the engine's word for a side.
    #[test]
    fn a_draw_is_announced_as_a_draw_rather_than_as_a_winner() {
        assert_eq!(victory_banner(Some("Robot v3")), "WINNER: Robot v3");
        assert!(victory_banner(None).contains("Draw"));
    }
}

#[cfg(test)]
mod pause_arbitration_tests {
    use super::*;
    use ambition_platformer2d::input::participant::{
        context_priority, resolve_active_input_context, ContextClaim, ParticipantContexts,
    };
    use ambition_platformer2d::input::{
        InputParticipant, MenuControlFrame, SeatInputContexts, SeatMenuFrames, PAUSE_CONTEXT,
    };
    use bevy::prelude::*;

    /// A seat that is browsing this screen, plus whatever else is claiming.
    fn app_with(pause_open: bool) -> App {
        let mut app = App::new();
        app.init_resource::<SeatInputContexts>();
        app.init_resource::<SeatMenuFrames>();
        app.init_resource::<select::SmashSelect>();
        app.init_resource::<ambition_platformer2d::game_shell::ShellRouter>();
        app.init_resource::<select_screen::cursor::SelectCursor>();
        app.init_resource::<select_screen::StartRequested>();
        // ⚠ the DEFAULT roster (this demo's own fighters), not an assembled one:
        // there is no catalog in this fixture and none is needed. What is under
        // test is the arbitration, and the roster only has to be non-empty so
        // the layout has a grid to put a cursor on.
        app.init_resource::<select::SmashRoster>();
        app.add_systems(
            Update,
            (
                resolve_active_input_context,
                select_screen::drive_the_cursor.run_if(the_select_screen_owns_its_input),
            )
                .chain(),
        );

        // On the select route, with this screen's own claim declared — the same
        // claim `declare_the_select_input_context` writes in production.
        let mut contexts = ParticipantContexts::default();
        contexts.declare(ContextClaim::capturing(
            ambition_platformer2d::input::SELECT_CONTEXT,
            context_priority::SELECT,
        ));
        // The pause menu's claim, at its real priority. ⚠ this test names it
        // only because it is standing in for the host; neither the screen nor
        // the pause menu names the other.
        if pause_open {
            contexts.declare(ContextClaim::capturing(
                PAUSE_CONTEXT,
                context_priority::PAUSE,
            ));
        }
        app.world_mut().spawn((
            InputParticipant {
                id: ambition_platformer2d::input::ParticipantId(0),
            },
            contexts,
        ));

        app.world_mut()
            .resource_mut::<ambition_platformer2d::game_shell::ShellRouter>()
            .active = Some(ambition_platformer2d::game_shell::ActiveShellExperience {
            activation_id: ambition_platformer2d::game_shell::ShellActivationId(1),
            route_id: ambition_platformer2d::game_shell::ShellRouteId::new(SMASH_SELECT_ROUTE),
            experience_id: ambition_platformer2d::game_shell::ShellExperienceId::new(
                SMASH_SELECT_EXPERIENCE,
            ),
            parameters: Default::default(),
            load_authorization: None,
            prepared_session: None,
        });

        // **THE CURSOR IS ON SLOT 1's BUTTON.** No window and no `UiPlugin` here,
        // and it does not matter: the screen's rectangles come from
        // `select_screen::layout`, which lays out against `HEADLESS_VIEWPORT`
        // when there is no window. That is what makes this test press a real
        // button rather than reach into the value — and the control below is
        // what proves the press lands at all.
        let button = select_screen::layout::SelectLayout::for_viewport(
            None,
            select::SmashRoster::default().cell_count(),
        )
        .role_button(0);
        app.world_mut()
            .resource_mut::<select_screen::cursor::SelectCursor>()
            .move_to(button.center());

        // Seat 0 presses confirm on that button, which cycles the slot.
        app.world_mut().resource_mut::<SeatMenuFrames>().set(
            0,
            MenuControlFrame {
                select: true,
                ..Default::default()
            },
        );
        app
    }

    /// **The screen drives when it owns its seat.** The control: without this,
    /// the test below passes on a screen that never worked.
    #[test]
    fn the_select_screen_reads_its_seat_when_nothing_is_over_it() {
        let mut app = app_with(false);
        app.update();
        assert_eq!(
            app.world()
                .resource::<select::SmashSelect>()
                .participating(),
            1,
            "a click on slot 1's button did nothing while this screen owned the seat"
        );
    }

    /// **One press moves ONE thing.**
    ///
    /// With the universal pause menu open OVER this screen the arrows drove
    /// BOTH — the menu's cursor and the CPU count. They read different channels
    /// (`MenuControlFrame` and `SeatMenuFrames`), so neither could consume the
    /// other's edge, and this demo cannot name `ShellPauseMenu` at all:
    /// `basic_shell_presentation` is not in `all_capabilities`, which is the
    /// oracle rule working as intended.
    ///
    /// So the arbitration is the CLAIM system. A capturing claim above `SELECT`
    /// closes this screen's context, and the screen asks whether it still owns
    /// the seat. Neither side names the other.
    #[test]
    fn a_pause_claim_takes_the_arrows_away_from_the_select_screen() {
        let mut app = app_with(true);
        app.update();
        assert_eq!(
            app.world()
                .resource::<select::SmashSelect>()
                .participating(),
            0,
            "the pause menu owns the presses; the screen underneath must not \
             also act on them"
        );
    }

    /// **The screen publishes its submit verb while it is up, and takes it back
    /// when it leaves.**
    ///
    /// ⚠ the retraction is the half that bites. A cue outlives its surface if
    /// nothing withdraws it, and the next screen then inherits a prompt telling
    /// the player to choose a fighter on a screen with no fighters.
    #[test]
    fn the_select_screen_publishes_its_cue_and_retracts_it_on_the_way_out() {
        use ambition_platformer2d::input::{ActiveUiCues, SELECT_CONTEXT};

        let mut app = app_with(false);
        app.init_resource::<ActiveUiCues>();
        app.add_systems(Update, publish_the_select_ui_cue);
        app.update();
        assert_eq!(
            app.world()
                .resource::<ActiveUiCues>()
                .for_context(SELECT_CONTEXT)
                .map(|cue| cue.submit_label.as_str()),
            Some("Choose"),
            "the lobby is up and nothing says what confirming does"
        );

        // Leave the route — the only change.
        app.world_mut()
            .resource_mut::<ambition_platformer2d::game_shell::ShellRouter>()
            .active = None;
        app.update();
        assert!(
            app.world()
                .resource::<ActiveUiCues>()
                .for_context(SELECT_CONTEXT)
                .is_none(),
            "a cue left behind outlives its surface"
        );
    }
}
