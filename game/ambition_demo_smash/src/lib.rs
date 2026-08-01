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

pub mod select;
pub mod select_ui;

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
/// which is exactly why it has to be authored: an unauthored pool is ONE, and a
/// meter divided by one reports 14000%.
pub const SMASH_PERCENT_REFERENCE: i32 = 100;

/// The name a CPU seat asks for.
///
/// ⛔ **`ControllerBinding::Cpu { brain_profile }` is NOT a catalog brain
/// preset**, and the field name says otherwise. It is a `CharacterRoster`
/// ARCHETYPE key: `spec_for_brain` looks it up in the roster fragment's
/// `by_brain` map and falls back to a default spec — whose brain is
/// `stand_still` — when the key is absent. The catalog's `brain_presets` are a
/// different namespace that a seated CPU never consults.
///
/// So this demo's CPU seats stand still, and will until it registers a
/// `CharacterRosterFragment` with a `duelist` archetype. `BrainPreset::Fighter`
/// (added the same day) is the authoring path for a CATALOG-driven body — an
/// NPC, a placement, a `default_brain` — and it works; it is simply not the road
/// a match seat travels.
///
/// Found by a diagram printing `travel: [0.0, 0.0]` next to a brain label
/// reading `stand_still`. The preset resolved correctly in isolation, which is
/// exactly what made it confusing: the catalog was right, the lookup was
/// somewhere else, and two vocabularies share one word.
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
    S: Into<String>,
{
    let mut roster = MatchParticipantRoster::of(Vec::<String>::new());
    roster.participants = characters
        .into_iter()
        .enumerate()
        .map(|(index, character)| {
            MatchParticipant::new(character)
                .driven_by(if index == 0 {
                    ControllerBinding::Human { device_slot: 0 }
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
    roster.opens_suspended = true;
    roster.fighter_stocks = Some(STARTING_STOCKS);
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
    // ⚠ SPELLED OUT rather than a named set, and both named candidates were
    // tried and measured first. `basic()` has no double jump and no attack, so
    // it would REMOVE verbs both duelists already had. `sane_subset()` reads
    // like a fighter's kit in its first ten lines and is not one - measured, it
    // also grants fly, blink, precision_blink, wall climb and pogo, so declaring
    // it made the two seats agree that they could both FLY. This is the actual
    // floor of a platform fighter: run, jump, double jump, fast fall, dash,
    // attack. WHICH verbs is a product call and this is the one place to change
    // it; that the two seats agree is not.
    roster.fighter_abilities = Some(ambition_platformer2d::engine_core::AbilitySet {
        move_horizontal: true,
        jump: true,
        variable_jump: true,
        double_jump: true,
        fast_fall: true,
        dash: true,
        attack: true,
        ..ambition_platformer2d::engine_core::AbilitySet::NONE
    });
    roster.published_by(SMASH_EXPERIENCE)
}

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
    S: Into<String>,
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
pub fn respawn_placement(stage_centre: Vec2) -> Vec2 {
    Vec2::new(
        stage_centre.x,
        // Toward the sky. The stage's own down is the gravity the room authored;
        // this demo is screen-down like every other platform fighter, and a
        // gravity-flipped stocks stage is a thing the ENGINE would have to
        // answer rather than this crate.
        stage_centre.y - RESPAWN_HEIGHT_PX,
    )
}

/// Stable room id for the stage.
pub const SMASH_STAGE_ROOM_ID: &str = "smash_stage";

/// Stage size, and the platform's top.
/// ⚠ **these numbers were WRONG until somebody drew them** (2026-07-31).
///
/// The first cut was a 960x640 world around a 420px platform with a 220px
/// margin, and every test passed. The diagram
/// (`cargo run -p ambition_demo_smash_app --bin stage_diagram`) showed what the
/// tests could not: a fighter knocked off the side had to cross ~490px — MORE
/// than the whole platform's width — before the world took it. On a platform
/// fighter that is a body drifting through empty space for about a second while
/// nothing happens.
///
/// The test that was supposed to catch this asserted `distance < world.size.x`,
/// which is 490 < 960: true, and meaningless. A bound loose enough to hold for
/// any stage holds for a broken one.
const STAGE_SIZE: Vec2 = Vec2::new(640.0, 480.0);
const PLATFORM_TOP: f32 = 300.0;
const PLATFORM_WIDTH: f32 = 420.0;

/// **How far past the stage a body may travel before the world takes it.**
///
/// Tight ON PURPOSE, and this is the number that makes the demo a fighter rather
/// than a room with two people in it. Every other room in this game is ENCLOSED —
/// its margin exists to catch a body that fell through the floor, so it is
/// generous and rarely reached. Here it is the win condition, so it has to be
/// close enough that a good hit reaches it.
const BLAST_MARGIN_PX: f32 = 120.0;

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
    world.blast_margin = BLAST_MARGIN_PX;
    // The SIDES are the interesting ones and they are not the default. A body
    // launched horizontally leaves through them, and without an explicit value
    // they inherit a margin sized for "fell through the floor" — generous enough
    // that a fighter knocked off the edge would drift for a second and a half
    // before anything noticed.
    world.side_blast_margin = Some(BLAST_MARGIN_PX);
    world.ceiling_blast_margin = Some(BLAST_MARGIN_PX);

    let mut room = RoomSpec::new(SMASH_STAGE_ROOM_ID, world);
    room.metadata.mode = Some(SMASH_MODE.to_string());
    room
}

/// The stage centre a respawn is measured from.
pub fn stage_centre() -> Vec2 {
    Vec2::new(STAGE_SIZE.x / 2.0, PLATFORM_TOP)
}

/// What the match announces when it ends.
pub fn victory_banner(winner: Option<&str>) -> String {
    match winner {
        Some(side) => format!("{side} wins"),
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

        let sim = ambition_platformer2d::platformer::schedule::SimScheduleExt::sim_schedule(app);
        // AFTER the engine's own `CombatSet::Settle` work: the stock is spent
        // there, and placing a body before it has been spent would put the
        // fighter back on the stage for a knockout that had not been counted.
        // The HUD publisher is PRESENTATION, not a rule: it reads seats and
        // publishes readouts and decides nothing. It runs in the same gated set
        // so a hosted build stops drawing a fighter HUD the moment the stage is
        // not the active mode.
        let rules = (
            publish_smash_hud,
            release_the_opening_hold,
            place_respawning_fighters,
            take_eliminated_fighters_out_of_play,
            announce_the_winner,
        )
            .chain()
            .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle)
            .after(ambition_platformer2d::combat::stocks::spend_fighter_stocks);
        if self.hosted {
            app.add_systems(sim, rules.run_if(ambition_platformer2d::runtime::in_mode(SMASH_MODE)));
        } else {
            app.add_systems(sim, rules);
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

/// Publish percent and stocks for every seated fighter.
///
/// ⚠ percent is NOT health and the gauge fill says so: it fills as damage
/// ACCUMULATES, and the number keeps counting past 100% because a platform
/// fighter's does. Clamping the fill is a rendering decision; clamping the
/// number would be a lie about the game.
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
            ambition_platformer2d::presentation::HudReadout::gauge(
                name.clone(),
                value,
                *percent,
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

/// **Let the fighters move once the match is live.**
///
/// The roster opens `opens_suspended`, which stamps `ScriptedControl` on every
/// fighter in the same flush that creates them — so no body is ever observable
/// in a state the ruleset did not ask for. Something has to take it OFF, and in
/// the versus stage that is the countdown reaching zero.
///
/// ⚠ **this demo has no countdown, and for a day it had no release either.** The
/// fighters seated, stood exactly where seating put them, and never moved. Every
/// test passed: they existed, wore seats, carried stocks, and were correctly
/// suspended forever. The tell was a diagram printing `travel: [0.0, 0.0]` —
/// a number no assertion in the tree was looking at.
///
/// Releasing on "the match is live" is the honest reading of the flag for a
/// ruleset with no opening ceremony: the hold exists to cover the gap between
/// construction and the round starting, and here those are the same tick.
fn release_the_opening_hold(
    mut commands: bevy::prelude::Commands,
    active: Option<bevy::prelude::Res<ambition_platformer2d::actors::character_runtime::ActiveMatch>>,
    held: bevy::prelude::Query<
        bevy::prelude::Entity,
        (
            bevy::prelude::With<ambition_platformer2d::actor::MatchSeat>,
            bevy::prelude::With<ambition_platformer2d::characters::brain::ScriptedControl>,
        ),
    >,
) {
    if active.is_none() {
        return;
    }
    for body in held.iter() {
        commands
            .entity(body)
            .try_remove::<ambition_platformer2d::characters::brain::ScriptedControl>();
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
    mut spent: bevy::prelude::MessageReader<ambition_platformer2d::actor::FighterStockSpent>,
    mut bodies: bevy::prelude::Query<(
        ambition_platformer2d::actor::BodyClusterQueryData,
        &mut ambition_platformer2d::actors::features::MotionModel,
    )>,
) {
    for event in spent.read() {
        // An ELIMINATED fighter is not placed. It has no stock to come back on,
        // and putting it back would make the last knockout the only one that did
        // not count.
        if event.eliminated {
            continue;
        }
        let Ok((clusters, mut model)) = bodies.get_mut(event.body) else {
            continue;
        };
        let mut item = clusters;
        let mut clusters = item.as_clusters_mut();
        // Velocity is zeroed by the reset itself, which is what a fighter that
        // keeps the velocity that threw it off the stage needs: otherwise it
        // respawns already travelling toward the blast zone it just left.
        ambition_platformer2d::engine_core::reset_body_clusters(
            &mut model,
            &mut clusters,
            respawn_placement(stage_centre()),
            // This demo's fighters run the engine's default air game; a stage
            // that tuned it would pass its own number here, which is the point
            // of the parameter.
            ambition_platformer2d::engine_core::DEFAULT_TUNING.air_jumps,
        );
    }
}

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

/// Say who won, once.
fn announce_the_winner(
    mut decided: bevy::prelude::MessageReader<ambition_platformer2d::actor::StocksMatchDecided>,
    mut banner: bevy::prelude::MessageWriter<ambition_platformer2d::combat::GameplayBannerRequested>,
) {
    for outcome in decided.read() {
        banner.write(ambition_platformer2d::combat::GameplayBannerRequested::new(
            victory_banner(outcome.winner.as_deref()),
            3.0,
        ));
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
            .get_resource_or_insert_with(ambition_platformer2d::game_shell::ShellRouteCatalog::default)
            .register(ambition_platformer2d::game_shell::ShellRouteSpec::new(
                SMASH_SELECT_ROUTE,
                SMASH_SELECT_EXPERIENCE,
            ));
        app.init_resource::<select::SmashSelect>();
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
        app.add_systems(
            bevy::prelude::Update,
            bevy::prelude::IntoScheduleConfigs::in_set(
                bevy::prelude::IntoScheduleConfigs::chain((
                    present_the_select_screen,
                    select_ui::update_select_ui,
                    drive_the_select_screen,
                    start_the_battle_when_everyone_is_ready,
                )),
                SmashSelectSet,
            ),
        );
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
    existing: bevy::prelude::Query<(), bevy::prelude::With<select_ui::SmashSelectUiRoot>>,
    roots: bevy::prelude::Query<
        bevy::prelude::Entity,
        bevy::prelude::With<select_ui::SmashSelectUiRoot>,
    >,
) {
    let on_select = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_SELECT_ROUTE);
    // **WHILE THIS SCREEN IS UP, THE PADS ARE SEATS.** Declared here and dropped
    // on the way out, so the participants it asks for live exactly as long as
    // the question does — the same lifetime rule the match's own seats have.
    let offered = devices.as_deref().map(select::seats_offered).unwrap_or(1) as u8;
    let want_seats = ambition_platformer2d::input::DeclaredInputSeats(if on_select { offered } else { 0 });
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
            // THIS demo's roster. Another stage in the same host publishes its
            // own into the same global resource, and clearing "the roster" is
            // how one game deletes another's match.
            if roster.is_some_and(|roster| roster.is_published_by(SMASH_EXPERIENCE)) {
                commands.remove_resource::<MatchParticipantRoster>();
            }
        }
        select_ui::spawn_select_ui(commands, existing);
    } else {
        select_ui::despawn_select_ui(commands, roots);
    }
}

/// **Turn what each seat pressed into what each seat decided.**
///
/// ⚠ this is the system the screen went without, and its absence is the exact
/// defect shape this repo keeps catching: `SmashSelect` was initialised, read by
/// the transition below, unit-tested through every state — and NOTHING ever
/// wrote to it. `ready()` could not become true, so the battle could not start,
/// and every test passed because they all drove the resource directly. A state
/// machine with no driver is a state machine that has never run.
///
/// Reads [`SeatMenuFrames`] rather than the global `MenuControlFrame`, because
/// on this screen "who pressed it" is the entire question.
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
    let on_select = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_SELECT_ROUTE);
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

fn drive_the_select_screen(
    mut select: bevy::prelude::ResMut<select::SmashSelect>,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    frames: Option<bevy::prelude::Res<ambition_platformer2d::input::SeatMenuFrames>>,
    devices: Option<bevy::prelude::Res<ambition_platformer2d::input::LocalDeviceOrder>>,
    contexts: Option<bevy::prelude::Res<ambition_platformer2d::input::SeatInputContexts>>,
) {
    let on_select = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_SELECT_ROUTE);
    if !on_select {
        return;
    }
    let Some(frames) = frames else { return };
    // A keyboard-only desktop has no device rows and still has a player, so the
    // offered seats never fall below one (`seats_offered` clamps).
    let offered = devices.as_deref().map(select::seats_offered).unwrap_or(1);
    for seat in 0..offered {
        // **DOES THIS SEAT STILL OWN THE SCREEN?**
        //
        // A capturing claim above `SELECT` — the pause menu — closes this
        // context and the screen stops driving, so one press moves the menu
        // cursor and nothing else. `None` (no resolver installed, as in a bare
        // unit fixture) reads as owned: a test that wires no contexts is testing
        // the screen, not the arbitration.
        let owned = contexts.as_deref().is_none_or(|c| {
            c.for_seat(seat as u8)
                .allows(ambition_platformer2d::input::SELECT_CONTEXT)
        });
        if !owned {
            continue;
        }
        let frame = frames.for_seat(seat as u8);
        if frame.back {
            select.cancel(seat);
            continue;
        }
        // **DOWN ADDS A CPU, UP TAKES ONE AWAY — from ANY seat, in every state.**
        //
        // The seats nobody holds a controller for are the ones being edited, so
        // the press cannot come from them; it comes from whoever is here. Up and
        // down are the two directions this screen has no other use for
        // (left/right browse), which is what makes them free — and unlike
        // `start`, they exist on a keyboard. `Platformer2dInputActionMonolith::Start` is ESCAPE
        // there, and Escape opens the pause menu: the one press that made the
        // demo playable alone was the one press a keyboard could not make.
        if frame.down {
            select.add_cpu(seat);
        }
        if frame.up {
            select.remove_cpu();
        }
        match select.seat(seat) {
            // Confirm at an empty seat IS the join. There is no separate
            // "press start" to sit down, and a second button to learn is a
            // second button somebody at a party does not know about.
            select::SeatSelection::Empty => {
                if frame.select {
                    select.join(seat);
                }
            }
            select::SeatSelection::Browsing { .. } => {
                if frame.left {
                    select.browse(seat, -1);
                }
                if frame.right {
                    select.browse(seat, 1);
                }
                if frame.select {
                    select.lock_in(seat);
                }
            }
            // A locked seat listens for nothing but `back` (handled above) and
            // the CPU keys. Ignoring `select` here is deliberate: a double-tap of
            // confirm must not reach through to anything else.
            select::SeatSelection::LockedIn { .. } => {}
            // A CPU is holding this chair. Confirm sits down in it; `back` (above)
            // sends the machine away.
            select::SeatSelection::Cpu { .. } => {
                if frame.select {
                    select.join(seat);
                }
            }
        }
    }
}

/// Publish the decided roster and leave the select screen.
///
/// Runs on `Update`, not the sim schedule: choosing a fighter is shell
/// lifecycle, and the sim is not even running yet — the stage has no session
/// until the route this system requests actually resolves.
fn start_the_battle_when_everyone_is_ready(
    mut commands: bevy::prelude::Commands,
    select: bevy::prelude::Res<select::SmashSelect>,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    roster: Option<bevy::prelude::Res<MatchParticipantRoster>>,
    mut shell: bevy::prelude::MessageWriter<ambition_platformer2d::game_shell::ShellCommand>,
) {
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
    let Some(decided) = select.roster() else {
        return;
    };
    commands.insert_resource(decided);
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
            ambition_platformer2d::provider::AuthoredCatalogFragments::new(SMASH_CHARACTER_ID, SMASH_EXPERIENCE),
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
        .with_loading_activity(ambition_platformer2d::load_presentation::DETERMINISTIC_LOADING_ACTIVITY_ID)
        .install(app, smash_prepared_session_world);
        app.add_plugins(SmashRulesPlugin::hosted());
    }
}

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
            display_name: "Duelist A",
            spritesheet: "sprites/player_robot_v3_spritesheet.png",
            manifest: "sprites/player_robot_v3_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "duelist",
            default_action_set: "duelist",
            playable_kit: HostCode,
            tags: ["player", "smash"],
            fallback_dialogue: ["Off the edge is the only way out."],
        ),
        "smash_duelist_b": (
            display_name: "Duelist B",
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
        use ambition_platformer2d::actors::character_runtime::{CharacterDefinition, CharacterDefinitionAppExt};
        for (id, name, sheet) in [
            (SMASH_CHARACTER_ID, "Duelist A", "player_robot_v3"),
            (SMASH_OPPONENT_ID, "Duelist B", "player_robot_v2"),
        ] {
            let mut definition =
                CharacterDefinition::new(id, name, SMASH_EXPERIENCE).with_sheet(sheet);
            // **THE PERCENT REFERENCE.** (found by drawing it, 2026-07-31)
            //
            // A character that authors no vitals gets a ONE-HIT pool — the
            // seating code names this exact trap in its own comment — and under
            // `DeathPolicy::Unbounded` the pool never kills, so nothing goes
            // wrong except the number. `damage_percent()` is
            // `accumulated / max`, so with `max = 1` a 140-damage hit read as
            // **14000%**. Every test passed: the meter was accumulating
            // correctly and the division was correct, over a denominator nobody
            // had authored.
            //
            // 100 makes percent read the way a platform fighter's does, where a
            // player learns "around 120 I get launched" as a number that means
            // something across characters.
            definition.vitals.max_health = Some(SMASH_PERCENT_REFERENCE);
            // **A PLATFORM FIGHTER DOES NOT RECOIL LIKE AN EXPLORER.**
            // (measured 2026-07-31, `ladder_probe` + `AMBITION_FIGHTER_TRACE=1`)
            //
            // `SLASH_RECOIL` is 110 px/s BACKWARDS on every melee press — a feel
            // detail for the exploration protagonist, whose swings are
            // occasional and whose rooms have walls. A fighter brain presses an
            // attack on most decisions, so the recoils RATCHET: the trace reads
            // 200, 310, 420, 530 px/s in exact 110 steps, against a 270 px/s
            // run, while the brain's own emitted input points the other way.
            // **Every CPU on this stage swung itself off the edge, backwards.**
            //
            // The A/B is unambiguous. Same build, `slash_recoil` alone:
            //
            //     110 (engine default)   level 9 survives 5.2s, loses 3 stocks
            //       0                    level 9 survives 15.1s; at rollout
            //                            depth 0 it does not self-KO AT ALL in
            //                            a 60s match
            //
            // So the number is authored HERE rather than changed in the engine:
            // it is this game's kit that is wrong for it, not every game's.
            // Mary-O and the exploration protagonist keep the recoil they were
            // tuned with.
            definition.movement_tuning = Some(ambition_platformer2d::engine_core::MovementTuning {
                slash_recoil: 0.0,
                ..ambition_platformer2d::engine_core::DEFAULT_TUNING
            });
            app.register_character(definition);
        }
    }
    // **THE ARCHETYPE A CPU SEAT ACTUALLY NAMES.**
    //
    // `ControllerBinding::Cpu { brain_profile }` is a `CharacterRoster` key, not
    // a catalog preset — two vocabularies sharing one word, which cost an hour
    // on 2026-07-31. Without this fragment the seat is now REFUSED (seating
    // stopped falling back to a generic enemy the same day); before that it
    // silently became a stand-still body.
    {
        use ambition_platformer2d::actors::features::{CharacterRosterAppExt, CharacterRosterFragment};
        app.register_character_roster_fragment(
            CharacterRosterFragment::from_ron(SMASH_EXPERIENCE, None::<String>, SMASH_ROSTER_RON)
                .expect("the smash duelist roster fragment is valid"),
        );
    }
    app.register_audio_catalog_fragment(
        AudioCatalogFragment::new(SMASH_EXPERIENCE, None, None)
            .expect("the silent smash audio fragment is valid"),
    );
}

/// The duelist archetype, which is what makes a CPU seat a FIGHTER.
///
/// `brain_template: Fighter` is the FB4b rig on the path a match seat travels.
/// The catalog's `duelist` preset (also `Fighter`) covers the other path — an
/// NPC, a placement, a `default_brain` — and both exist because the engine has
/// two brain vocabularies and a rig has to appear in both to be selectable.
const SMASH_ROSTER_RON: &str = r#"{
    "duelist_l1": (
        max_health: 100,
        run_speed: 200.0,
        patrol_effort: 1.0,
        chase_effort: 1.0,
        aggro_radius: 600.0,
        attack_range: 48.0,
        contact_strength: 0.0,
        damage_amount: 4,
        brain_template: Fighter,
        fighter_level: Some(1),
        move_style: Walk,
        attacks_player: true,
    ),
    "duelist_l3": (
        max_health: 100,
        run_speed: 200.0,
        patrol_effort: 1.0,
        chase_effort: 1.0,
        aggro_radius: 600.0,
        attack_range: 48.0,
        contact_strength: 0.0,
        damage_amount: 4,
        brain_template: Fighter,
        fighter_level: Some(3),
        move_style: Walk,
        attacks_player: true,
    ),
    "duelist_l5": (
        max_health: 100,
        run_speed: 200.0,
        patrol_effort: 1.0,
        chase_effort: 1.0,
        aggro_radius: 600.0,
        attack_range: 48.0,
        contact_strength: 0.0,
        damage_amount: 4,
        brain_template: Fighter,
        fighter_level: Some(5),
        move_style: Walk,
        attacks_player: true,
    ),
    "duelist_l6": (
        max_health: 100,
        run_speed: 200.0,
        patrol_effort: 1.0,
        chase_effort: 1.0,
        aggro_radius: 600.0,
        attack_range: 48.0,
        contact_strength: 0.0,
        damage_amount: 4,
        brain_template: Fighter,
        fighter_level: Some(6),
        move_style: Walk,
        attacks_player: true,
    ),
    "duelist_l9": (
        max_health: 100,
        run_speed: 200.0,
        patrol_effort: 1.0,
        chase_effort: 1.0,
        aggro_radius: 600.0,
        attack_range: 48.0,
        contact_strength: 0.0,
        damage_amount: 4,
        brain_template: Fighter,
        fighter_level: Some(9),
        move_style: Walk,
        attacks_player: true,
    ),
    "duelist": (
        max_health: 100,
        run_speed: 200.0,
        patrol_effort: 1.0,
        chase_effort: 1.0,
        aggro_radius: 600.0,
        attack_range: 48.0,
        contact_strength: 0.0,
        damage_amount: 4,
        brain_template: Fighter,
        fighter_level: Some(5),
        move_style: Walk,
        attacks_player: true,
    ),
}"#;

/// The stage, as the shared preparation lifecycle wants it.
fn smash_prepared_session_world() -> ambition_platformer2d::runtime::PreparedPlatformerSource {
    use ambition_platformer2d::runtime::demo_fixture::{
        ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
    };

    let room = smash_stage();
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    ambition_platformer2d::runtime::PreparedPlatformerSource::new(
        SMASH_EXPERIENCE,
        RoomSet::from_parts(SMASH_STAGE_ROOM_ID, vec![room], Vec::new()),
        geometry,
        metadata,
        StartingCharacter::new(SMASH_CHARACTER_ID),
        LdtkRuntimeIndex::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::engine_core::AabbExt;

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
            ControllerBinding::Human { device_slot: 0 }
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

    /// **A respawn is ABOVE the stage, not on it.** A fighter that comes back on
    /// the floor comes back inside the opponent who just knocked it off.
    #[test]
    fn a_respawn_is_above_the_stage_centre() {
        let centre = Vec2::new(400.0, 300.0);
        let respawn = respawn_placement(centre);
        assert_eq!(respawn.x, centre.x);
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

    /// **The blast margin has to be REACHABLE**, or stocks never spend and the
    /// whole loop is decorative.
    ///
    /// The number is the win condition here, not a safety net. Every other room
    /// sizes its margin for "fell through the floor" — generous, rarely reached —
    /// and inheriting that default is how a fighter knocked cleanly off the edge
    /// drifts for a second and a half before anything notices.
    #[test]
    fn a_body_knocked_off_the_side_leaves_the_world() {
        let room = smash_stage();
        let platform = room.world.blocks[0].aabb;
        assert!(
            platform.left() > BLAST_MARGIN_PX * 0.0,
            "the platform starts at the world origin, so there is no space to be \
             knocked into on the left"
        );
        assert_eq!(
            room.world.side_blast_margin,
            Some(BLAST_MARGIN_PX),
            "the sides fell back to the enclosed-room default, which is sized for \
             a body that fell through the floor rather than one that was hit"
        );
        assert_eq!(room.world.ceiling_blast_margin, Some(BLAST_MARGIN_PX));
        assert!(
            BLAST_MARGIN_PX < room.world.size.x / 2.0,
            "the margin is wider than half the stage, so a body would have to \
             cross the whole screen before the world took it"
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
        let respawn = respawn_placement(stage_centre());
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

    /// **The banner says who won, once per ending.**
    ///
    /// The plugin's half of the seam, driven through the message the engine
    /// actually writes rather than by calling `victory_banner` directly — which
    /// would test the string and not the wiring.
    #[test]
    fn deciding_the_match_raises_a_banner_naming_the_winner() {
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_message::<ambition_platformer2d::actor::StocksMatchDecided>();
        app.add_message::<ambition_platformer2d::combat::GameplayBannerRequested>();
        app.add_systems(Update, announce_the_winner);
        app.update();

        app.world_mut()
            .resource_mut::<Messages<ambition_platformer2d::actor::StocksMatchDecided>>()
            .write(ambition_platformer2d::actor::StocksMatchDecided {
                winner: Some("seat 2".to_string()),
            });
        app.update();

        let messages = app
            .world()
            .resource::<Messages<ambition_platformer2d::combat::GameplayBannerRequested>>();
        let mut cursor = messages.get_cursor();
        let raised: Vec<_> = cursor.read(messages).collect();
        assert_eq!(
            raised.len(),
            1,
            "the ending raised {} banners; a ruleset that announces twice \
             announces on every frame after the match ends",
            raised.len()
        );
        assert_eq!(raised[0].text, "seat 2 wins");
    }

    /// A DRAW reaches the banner as a draw, not as a winner with an empty name.
    #[test]
    fn a_drawn_match_is_announced_as_one() {
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_message::<ambition_platformer2d::actor::StocksMatchDecided>();
        app.add_message::<ambition_platformer2d::combat::GameplayBannerRequested>();
        app.add_systems(Update, announce_the_winner);
        app.update();
        app.world_mut()
            .resource_mut::<Messages<ambition_platformer2d::actor::StocksMatchDecided>>()
            .write(ambition_platformer2d::actor::StocksMatchDecided { winner: None });
        app.update();

        let messages = app
            .world()
            .resource::<Messages<ambition_platformer2d::combat::GameplayBannerRequested>>();
        let mut cursor = messages.get_cursor();
        let raised: Vec<_> = cursor.read(messages).collect();
        assert!(
            raised[0].text.contains("Draw"),
            "a draw was announced as a win: {}",
            raised[0].text
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

    /// **The prepared source carries the stage, not a default room.**
    ///
    /// The preparation seam takes a closure, and a closure that returns the
    /// wrong room fails nowhere: the route prepares, the session starts, and the
    /// player lands in somebody else's level.
    #[test]
    fn the_prepared_session_is_the_smash_stage() {
        let prepared = smash_prepared_session_world();
        assert_eq!(
            prepared.starting_character().character_id,
            SMASH_CHARACTER_ID
        );
        assert_eq!(
            prepared.geometry().0.blocks.len(),
            1,
            "the prepared geometry is not the one-platform stage"
        );
        assert_eq!(
            prepared.geometry().0.side_blast_margin,
            Some(BLAST_MARGIN_PX),
            "the prepared geometry lost the stage's blast margins, so a fighter \
             knocked off would drift instead of dying"
        );
    }

    /// **A fighter's percent is read against an AUTHORED pool.** (found by
    /// drawing a match, 2026-07-31)
    ///
    /// A character that authors no vitals gets a ONE-HIT pool, and under
    /// `DeathPolicy::Unbounded` the pool never kills — so nothing goes wrong
    /// except the number, and the number is the entire user-facing output of the
    /// stocks model. A 140-damage hit read as 14000%, with every test green:
    /// the meter accumulated correctly and divided correctly, by a denominator
    /// nobody had authored.
    #[test]
    fn each_duelist_authors_the_pool_its_percent_is_read_against() {
        assert!(
            SMASH_CATALOG_RON.contains("smash_duelist_a"),
            "the catalog rows moved; this test guards the pool that goes with them"
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
    /// (`travel: [0.0, 0.0]`) before anything said why.
    #[test]
    fn the_duelist_preset_is_a_fighter_brain() {
        use ambition_platformer2d::characters::actor::character_catalog::{CharacterCatalog, parse_catalog};

        let catalog = CharacterCatalog::from_data(parse_catalog(SMASH_CATALOG_RON));
        assert!(
            catalog.has_brain_preset("duelist"),
            "the catalog does not know the `duelist` preset at all, so every \
             fighter asking for it silently stands still"
        );
        let brain = catalog
            .build_brain_from_preset(
                "duelist",
                &ambition_platformer2d::characters::actor::character_catalog::BrainBuildContext::at(0.0),
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
    #[test]
    fn a_draw_is_announced_as_a_draw_rather_than_as_a_winner() {
        assert_eq!(victory_banner(Some("seat 1")), "seat 1 wins");
        assert!(victory_banner(None).contains("Draw"));
    }
}

#[cfg(test)]
mod pause_arbitration_tests {
    use super::*;
    use ambition_platformer2d::input::participant::{
        ContextClaim, ParticipantContexts, context_priority, resolve_active_input_context,
    };
    use ambition_platformer2d::input::{
        InputParticipant, MenuControlFrame, PAUSE_CONTEXT, SeatInputContexts, SeatMenuFrames,
    };
    use bevy::prelude::*;

    /// A seat that is browsing this screen, plus whatever else is claiming.
    fn app_with(pause_open: bool) -> App {
        let mut app = App::new();
        app.init_resource::<SeatInputContexts>();
        app.init_resource::<SeatMenuFrames>();
        app.init_resource::<select::SmashSelect>();
        app.init_resource::<ambition_platformer2d::game_shell::ShellRouter>();
        app.add_systems(
            Update,
            (resolve_active_input_context, drive_the_select_screen).chain(),
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
            experience_id: ambition_platformer2d::game_shell::ShellExperienceId::new(SMASH_SELECT_EXPERIENCE),
            parameters: Default::default(),
            load_authorization: None,
            prepared_session: None,
        });

        // Seat 0 holds DOWN, which on this screen adds a CPU.
        app.world_mut().resource_mut::<SeatMenuFrames>().set(
            0,
            MenuControlFrame {
                down: true,
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
            app.world().resource::<select::SmashSelect>().cpus(),
            1,
            "down adds a CPU when this screen owns the seat"
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
            app.world().resource::<select::SmashSelect>().cpus(),
            0,
            "the pause menu owns the arrows; the screen underneath must not \
             also act on them"
        );
    }
}
