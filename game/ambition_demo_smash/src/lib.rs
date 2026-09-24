//! Standalone stocks-based platform-fighter demo.
//!
//! Combat damage does not kill a stocks fighter; leaving the stage emits the
//! ruleset-owned knockout signal. The combat crate owns stock accounting, while
//! this demo supplies stage-specific respawn placement and match completion. It
//! also serves as an external-style consumer of the umbrella platformer API.

// No prelude: a match needs the actor vocabulary, not the room-authoring one.
// `install_technique(s)` registers a technique handler and declares its key in
// one statement, so the installed technique set is a fact of the composition.
use ambition_platformer2d::actor::{ControllerBinding, MatchParticipant, MatchParticipantRoster};
use ambition_platformer2d::combat::technique::{
    check_hydrates, NestedReferences, TechniqueDelivery, TechniqueOffer, TechniqueParams,
};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::Vec2;
use ambition_platformer2d::runtime::{install_technique, install_techniques};
use ambition_platformer2d::world::rooms::RoomSpec;

pub mod bolt;
pub mod bomb;
mod capture;
pub mod counter;
pub mod dilation;
pub mod george_booul_moveset;
pub mod homing;
pub mod limit;
pub mod mark;
pub mod match_scope;
pub mod mine;
pub mod motion;
pub mod moveset;
pub mod portal;
pub mod riposte;
pub mod select;
pub mod select_screen;
pub mod shark_ride;
pub mod sing;
pub mod smash_pack;
pub mod spring;
pub mod tether;

/// The game-MODE tag this demo's rules gate on, so they sleep everywhere else.
pub const SMASH_MODE: &str = "smash";

/// The match clock, in ticks: eight minutes at 60Hz.
///
/// Public so that measurement tools (for example `ladder_rig`) use the real
/// match length. A shorter length ends every bout on the damage tiebreak.
pub const SMASH_TIME_LIMIT_TICKS: u32 = 8 * 60 * 60;

/// Stocks each fighter starts with. Three is the smallest count where the
/// middle of a match differs from its start and its end.
pub const STARTING_STOCKS: u32 = 3;

/// What 100% means: the denominator of `damage_percent()`.
///
/// Under `DeathPolicy::Unbounded` the pool never kills, so this is only the
/// scale for percent. The match declares it, not the characters: an undeclared
/// pool is whatever the character authored. See `apply_smash_match_rules`.
pub const SMASH_PERCENT_REFERENCE: i32 = 100;

/// The published controller policy a CPU seat asks for: `smash::duelist`
/// (cite-ok: an authored provider::name key, not a Rust path), resolved in
/// this stage's own provider.
pub const SMASH_DUELIST_BRAIN: &str = "duelist";

/// Where a respawning fighter comes back, above the stage centre.
///
/// Above, not at the spawn point: a fighter that reappears on the floor can
/// reappear inside the opponent that knocked it off.
pub const RESPAWN_HEIGHT_PX: f32 = 160.0;

/// The one fighter on this grid whose up-B summons a mount.
pub const SMASH_SHARK_RIDER: &str = "npc_pirate_admiral";

/// Build the roster for a stocks match between `characters`.
///
/// `fighter_stocks` declares the count and `DeathPolicy::Unbounded` together,
/// because neither is meaningful alone. The engine owns that pairing.
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
            // The mount licence is a match rule; see `apply_smash_match_rules`.
            MatchParticipant::new(character)
                .driven_by(if index == 0 {
                    ControllerBinding::Human {
                        source: ambition_platformer2d::actor::LocalInputSource::FIRST_PAD,
                    }
                } else {
                    ControllerBinding::Cpu {
                        // The catalog preset name. A CPU seat that asks for a
                        // preset the composition does not ship stands still.
                        brain_profile: Some(SMASH_DUELIST_BRAIN.to_string()),
                    }
                })
                // Every seat is its own team: free-for-all is the simplest
                // shape that exercises the loop.
                .on_team(format!("seat {}", index + 1))
        })
        // Use the fighter body from the character's `smash_fighter` facet, if
        // it authors one. Without it, the seat composes the stage numbers over
        // the wandering-enemy baseline of an unauthored actor config.
        .map(
            |participant| match crate::smash_pack::fighter_body(participant.character.as_str()) {
                Some(body) => participant.with_body(body),
                None => participant,
            },
        )
        .collect();
    apply_smash_match_rules(&mut roster, STARTING_STOCKS);
    roster.published_by(SMASH_EXPERIENCE)
}

/// The Smash ruleset, in one place.
pub fn apply_smash_match_rules(roster: &mut MatchParticipantRoster, stocks: u32) {
    // This match grants no pilot licence. The admiral authors
    // `pilotable_classes`, and `prepared_match` unions it into `CanPilot` on
    // every road that builds a body. Which shark a body may board is answered
    // on the mount: see `ambition_mount::MountReservedFor`.
    roster.rules.opens_suspended = true;
    // Opening countdown: 3, 2, 1, go. Ticks, not seconds, because the release
    // compares against the sim clock (`MatchRules::opening_countdown_ticks`).
    //
    // Temporary dev speed-up: set `COUNTDOWN_SPEEDUP` back to 1 to restore the
    // full three seconds. It is a named divisor so the revert is one token.
    roster.rules.opening_countdown_ticks = 3 * 60 / COUNTDOWN_SPEEDUP;
    // Match clock (eight minutes, as in Ultimate's default stock match), so a
    // match between two passive fighters still ends. Derived from
    // `ActiveMatch::activated_on`, so it costs no rollback state; see
    // `MatchRules::time_remaining`.
    roster.rules.time_limit_ticks = SMASH_TIME_LIMIT_TICKS;
    // A parameter, not a constant or a resource read here: both roads
    // (`smash_roster` and `SmashSelect::roster_seeded`) must state the count.
    roster.rules.stocks = Some(stocks);
    // The match supplies one health pool for percent calculation so crossover
    // characters are measured against this ruleset rather than their home games.
    roster.rules.health_pool = Some(SMASH_PERCENT_REFERENCE);
    // The Limit is the match's resource: every seat is built holding it, empty.
    roster.rules.resources = vec![crate::limit::SMASH_LIMIT.declaration()];
    // Every fighter gets the ruleset floor, keeps what of its own kit the
    // ceiling permits, and brings nothing else. The gap between the two
    // constants is one verb: Robot v3 keeps its pogo.
    roster.rules.abilities = Some(ambition_platformer2d::engine_core::MatchAbilities {
        granted: SMASH_FIGHTER_KIT,
        permitted: SMASH_FIGHTER_CEILING,
    });
    // Apply the ruleset's body baseline alongside its ability policy.
    roster.rules.body = Some(SMASH_FIGHTER_BODY);
    // No items for now (Jon's call). The machinery (`MatchItemSpawns`, the
    // spawner, the weighted table) is built and tested; only this declaration
    // is absent. The previous table: a drop every 8s at three points above the
    // platform, weighted bomb 4 / gravity_grenade 2 / gun_sword 1. Do not add
    // `UseSystem` items (meteor gauntlet, mark/recall); those are abilities.
    roster.rules.item_spawns = None;
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
        // The roll ends at rest and leaves the fighter punishable. The distance
        // (530px/s over 0.22s, about 117px) is unchanged on purpose.
        dodge_roll_endlag: ambition_platformer2d::engine_core::DODGE_ROLL_ENDLAG,
        // Dodge staling: each recent evade removes a quarter of the
        // invulnerable window, floored at a third, forgiven one per 1.2s.
        // It reduces the i-frames, not the distance.
        dodge_stale_step: 0.25,
        dodge_stale_floor: 0.34,
        dodge_stale_recovery: 1.2,
        // A kill-power hit cannot be teched. Keep this well above the tumble
        // threshold (500px/s) so ordinary launches keep their tech.
        untechable_launch_speed: 1400.0,
        // An evade can cancel into an attack only in its last four frames.
        // Spot-dodge-into-attack stays a real option; it costs the frames
        // before the tail.
        evade_cancel_tail: 4.0 / 60.0,
        // Spot dodge, 0.16s: shorter than the roll because it covers no
        // distance. The engine default is `0.0`, so an exploration body keeps
        // the roll.
        spot_dodge_time: ambition_platformer2d::engine_core::SPOT_DODGE_TIME,
        // Smash 4 opens the parry window on the press, Ultimate on the
        // release. The stage chooses; `OnRelease` is fully live (guarded by
        // `the_parry_window_opens_where_the_ruleset_says_it_does`).
        parry_timing: ambition_platformer2d::engine_core::ParryTiming::OnRaise,
        tumble_speed: 500.0,
        // SDI, 3px per hitlag tick: shift out of the next hit while frozen.
        // The engine default is `0.0`.
        sdi_step: 3.0,
        // ASDI: one nudge per hit, paid when the freeze lifts. It gives a
        // defender something against multihits, where `sdi_step` gives little.
        asdi_step: 6.0,
        // A jab is a few hundred px/s and a smash is thousands, so this
        // separates a poke on a downed opponent from a launch.
        jab_lock_speed: 320.0,
        // Three pins, then the floor game resets: a combo route, not an
        // infinite.
        jab_lock_limit: 3,
        shield: ambition_platformer2d::engine_core::ShieldTuning::PLATFORM_FIGHTER,
        footstool: ambition_platformer2d::engine_core::FootstoolTuning::PLATFORM_FIGHTER,
        // Crouching stops the fighter, as in every Smash for characters
        // without a crawl. Mobility pays for the smaller hurtbox and the
        // `crouch_cancel_scale`. A fighter with a crawl declares its own.
        crouch_speed_frac: 0.0,
        // Initial dash: the first 14 frames of a ground move, where a
        // direction change is free (dash-dance, foxtrot re-tap). A genre
        // starting point, not a measured value.
        initial_dash_time: 14.0 / 60.0,
        // Inherit the run speed; the phase controls when you may turn.
        initial_dash_speed: 0.0,
        // Reversing out of a run costs frames. 3 frames is what the proving
        // ground tolerates: at 7, `smash_it` lost two premise guards because
        // the CPU matchup became one-sided. The value is a feel call for Jon.
        turnaround_time: 3.0 / 60.0,
        // A quarter of the footprint past a ledge puts the fighter on the
        // brink. Published only: `BodyMotionFacts::teetering` is read by
        // animation and control; collision does not change.
        teeter_margin: 0.25,
    };

/// The basic Smash abilities: the verbs every fighter on this stage has.
///
/// `fly` and `blink` are absent: they are the exploration traversal kit, not a
/// platform fighter's ground game. `interact` and `reset` are absent because a
/// fighter has no talk button and no teleport home. `dash` is absent: it is a
/// charge-gated burst that replaces velocity (`apply_dash`), not running.
/// Running is `move_horizontal` against the body's top speed. Without `dash`,
/// the burst button means only the dodge here.
///
/// `shield`, `dodge` and `ledge_grab` make this a platform fighter.
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
        // Granting `grab` does not invent a grab: the action scheme also needs
        // an authored `"grab"` move, so only fighters with one use it.
        grab: true,
        dodge: true,
        ledge_grab: true,
        ..ambition_platformer2d::engine_core::AbilitySet::NONE
    };

/// The ceiling: the floor above, plus the verbs a fighter may bring from home.
///
/// The difference between the two constants is character identity. Robot v3
/// has pogo because it authors pogo; a fighter without pogo does not get one
/// by entering Smash. Do not move pogo into the floor.
///
/// `MatchAbilities::levelled` treats these as floor and ceiling. `fly`, `blink`
/// and `dash` stay out of both: they are the exploration kit.
pub const SMASH_FIGHTER_CEILING: ambition_platformer2d::engine_core::AbilitySet =
    ambition_platformer2d::engine_core::AbilitySet {
        pogo: true,
        ..SMASH_FIGHTER_KIT
    };

/// The same roster, at a named ladder level.
///
/// For the ladder probe: it measures the same match at two levels. The brain
/// profile is a per-seat override, not a second archetype, because a second
/// archetype would also vary speed, reach and health.
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

/// The brain preset that does nothing, by name.
///
/// The body is staged, damageable and physical like any other; its policy
/// makes no decisions. Inspection scenarios use it to get a still fighter.
pub const SMASH_IDLE_BRAIN: &str = "stand_still";

/// The training dummy an inspection scenario faces by default.
///
/// A dummy, not a mirror: every subject is measured against the same target.
/// A copy of the subject would bring its own body size, hurtbox and stocks.
/// `never_dies` means a long grid run cannot end a take by killing it.
pub const INSPECTION_TARGET: &str = "sandbag_infinite";

/// The same roster, with every seat after the first standing still.
///
/// A training-mode target: a live CPU would move into or away from the strike
/// and change the recording. Contact, hurtboxes, hitstun and launch still run.
///
/// The seat has a driver that stands still by name. `ControllerBinding::Cpu {
/// brain_profile: None }` is refused at preparation, because a seat with no
/// driver looks the same as a brain that failed to install.
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

/// Two CPU fighters at different levels: the ladder's own roster.
///
/// [`smash_roster_at_level`] puts every CPU seat on one rung, and
/// [`smash_roster`] makes seat 0 human. A ladder needs "does level N beat
/// level N-1", so every seat here is a CPU.
///
/// `opens_suspended` and the stock count are inherited, so the rig measures
/// the shipped ruleset. Only the controllers differ from a real match.
pub fn smash_roster_at_levels<I, S>(characters: I, levels: &[u8]) -> MatchParticipantRoster
where
    I: IntoIterator<Item = S>,
    S: Into<ambition_platformer2d::entity_catalog::CharacterId>,
{
    let mut roster = smash_roster(characters);
    for (index, participant) in roster.participants.iter_mut().enumerate() {
        // Every seat is a CPU here, including seat 0.
        let level = levels.get(index).copied().unwrap_or(1);
        participant.controller = ControllerBinding::Cpu {
            brain_profile: Some(format!("{SMASH_DUELIST_BRAIN}_l{level}")),
        };
    }
    // Publish the roster so `seat_brain_profile` can resolve the
    // provider-relative policy name (`duelist_l1`) in the match's provider.
    roster.published_by(SMASH_EXPERIENCE)
}

/// Horizontal spread between adjacent respawn points, in stage pixels.
///
/// Two 32px tiles: wider than a standing body, so two fighters that return on
/// the same frame do not overlap. Seat `n` sits at most `(n/2 + 0.5)` spacings
/// from the centre, so eight seats stay within ±224px on a 480px platform.
const RESPAWN_SEAT_SPACING_PX: f32 = 64.0;

/// Where a fighter comes back; each seat has its own point.
///
/// Seats alternate outward from the centre (0 left, 1 right, 2 further left,
/// 3 further right), so the layout is symmetric at any roster size.
pub fn respawn_placement(stage_centre: Vec2, seat: usize) -> Vec2 {
    // 0,1 → half a spacing out; 2,3 → one and a half; and so on.
    let rank = (seat / 2) as f32 + 0.5;
    let side = if seat % 2 == 0 { -1.0 } else { 1.0 };
    Vec2::new(
        stage_centre.x + side * rank * RESPAWN_SEAT_SPACING_PX,
        // Toward the sky: this demo is screen-down. A gravity-flipped stocks
        // stage is a question for the engine.
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
/// Final Destination's main platform is about ten Mario heights wide. The
/// default standing body is 48px tall, so 480px gives the same scale and stays
/// on the 32px floor texture grid.
const PLATFORM_WIDTH: f32 = 480.0;

/// Blast margins chosen so the PLATFORM, not the room rectangle, has Final
/// Destination-like normalized proportions:
///
/// * ledge -> side blast line = 1.000 platform widths;
/// * platform surface -> ceiling blast line = 1.125 platform widths;
/// * platform surface -> fall blast line = 0.875 platform widths.
///
/// A 480px platform centered in a 640px room leaves 80px from each ledge to the
/// room edge, so the side margin is 400px. The surface is at y=300 in a 480px
/// room, so a 240px vertical margin puts the ceiling 540px above and the fall
/// line 420px below. The envelope is 1440x960: 3x2 platform widths.
const FALL_BLAST_MARGIN_PX: f32 = 240.0;
const SIDE_BLAST_MARGIN_PX: f32 = 400.0;
/// Public because authored moves must stay inside it: Alice's up-B exit portal
/// rises a fixed height above her. Guarded by
/// `an_authored_portal_rise_stays_inside_the_stage`.
pub const CEILING_BLAST_MARGIN_PX: f32 = 240.0;

/// Everything that makes a room a Smash stage, stated once.
///
/// Each stage states only its name, room id and geometry. Size, spawn, blast
/// envelope, mode and nameplate policy live here, so all stages share one blast
/// envelope (see `test_the_stage_choice_decides_which_stage_the_match_prepares`).
fn smash_stage_room(name: &str, id: &str, blocks: Vec<ae::Block>) -> RoomSpec {
    let mut world = ae::World::new(
        name,
        STAGE_SIZE,
        // Spawn above the platform. Seating places the fighters; this is only
        // where a lone visitor lands.
        Vec2::new(STAGE_SIZE.x / 2.0, PLATFORM_TOP - 96.0),
        blocks,
    );
    world.edges.fall = FALL_BLAST_MARGIN_PX;
    // Set the sides explicitly. The default margin is sized for falling
    // through the floor and is much too generous for a side launch.
    world.edges.side = Some(SIDE_BLAST_MARGIN_PX);
    world.edges.rise = Some(CEILING_BLAST_MARGIN_PX);

    let mut room = RoomSpec::new(id, world);
    room.metadata.mode = Some(SMASH_MODE.to_string());
    // Label every fighter the same way. The presentation default hides the
    // plate on a driven body, which is right for exploration but singles out
    // the human here. The stage decides; `Some(false)` is the other uniform
    // choice.
    room.metadata.nameplate_policy.label_driven_bodies = Some(true);
    room
}

/// The stage: a platform surrounded by nothing.
///
/// A fighter stage is a place you can be knocked off; the empty space is the
/// mechanic. Authored in Rust, not LDtk: demo rooms may be Rust, and this
/// stage is four numbers whose key fact is the blast margin.
pub fn smash_stage() -> RoomSpec {
    smash_stage_room(
        "Smash Stage",
        SMASH_STAGE_ROOM_ID,
        vec![ae::Block::solid(
            "smash_platform",
            Vec2::new((STAGE_SIZE.x - PLATFORM_WIDTH) / 2.0, PLATFORM_TOP),
            Vec2::new(PLATFORM_WIDTH, 32.0),
        )],
    )
}

/// The room id of the platformed stage — see [`smash_platform_stage`].
pub const SMASH_PLATFORM_STAGE_ROOM_ID: &str = "smash_platform_stage";

/// The room id of the narrow stage — see [`smash_narrow_stage`].
pub const SMASH_NARROW_STAGE_ROOM_ID: &str = "smash_narrow_stage";

/// Ten 32px tiles. Two thirds of [`PLATFORM_WIDTH`], still exactly on the floor
/// texture grid.
const NARROW_PLATFORM_WIDTH: f32 = 320.0;

/// The same flat block, two thirds as wide, with the blast envelope unchanged.
///
/// The unchanged envelope is the design. Scaling the margins with the width
/// would give the same stage at a smaller size.
///
/// | measured in platform widths | [`smash_stage`] | here |
/// |---|---|---|
/// | ledge → side blast line | 1.000 | **1.750** |
/// | surface → ceiling blast line | 1.125 | **1.688** |
/// | surface → fall blast line | 0.875 | **1.313** |
///
/// Less ground, same envelope: an edgeguard-and-recovery stage. It is not
/// tuned. It exists so the ladder rig can compare across geometries. It is a
/// separate stage so that numbers recorded on [`smash_stage`] keep their
/// meaning.
pub fn smash_narrow_stage() -> RoomSpec {
    smash_stage_room(
        "Smash Stage — Narrow",
        SMASH_NARROW_STAGE_ROOM_ID,
        vec![ae::Block::solid(
            "smash_platform",
            Vec2::new((STAGE_SIZE.x - NARROW_PLATFORM_WIDTH) / 2.0, PLATFORM_TOP),
            Vec2::new(NARROW_PLATFORM_WIDTH, 32.0),
        )],
    )
}

/// Height of each soft platform above the main stage surface, in world pixels.
///
/// Sized from the fighter's jump arc. Apex is `v²/(2·gravity)` (see
/// `FighterBodyAuthoring::jump_speed`). With the shipped defaults (`GRAVITY`
/// 2250, `JUMP_SPEED` 630, `DOUBLE_JUMP_SPEED` 520):
///
/// | arc | rise |
/// |---|---:|
/// | single jump | **88.2px** |
/// | + air jump taken AT the apex (the best case) | **148.3px** |
///
/// The low tier is a comfortable single jump (24px headroom). The top tier
/// needs the air jump and leaves 28px. `the_tiers_sit_inside_the
/// _fighters_measured_jump_arc` recomputes both from the engine constants.
const SOFT_PLATFORM_LOW_RISE: f32 = 64.0;
const SOFT_PLATFORM_HIGH_RISE: f32 = 120.0;
/// Width and thickness of a soft platform.
const SOFT_PLATFORM_SIZE: Vec2 = Vec2::new(168.0, 16.0);
/// How far the two low platforms sit either side of centre.
const SOFT_PLATFORM_SPREAD: f32 = 148.0;

/// A platform-fighter stage with platforms: the main surface plus three
/// drop-through tiers.
///
/// It uses the engine's one-way platforms (`BlockKind::OneWay`,
/// `resolve_one_way_hit`, `drop_through_timer`) and both drop gestures:
/// down+jump (`wants_drop_through`) and guard+down (`wants_platform_drop`).
///
/// It is a second stage, not an edit to the first. Changing the main stage is
/// Jon's design call, and the ladder rig's recorded numbers were taken on
/// [`smash_stage`]'s flat block. Reachable through [`SmashStageChoice`] and the
/// select screen's stage button.
///
/// Known issue: the top tier (y 180–196, x 236–404) is 10px under the respawn
/// platforms (`respawn_placement` puts a body at y 140 with its platform near
/// y 170). A fighter whose respawn platform expires lands on the tier, not the
/// stage. It is not adjusted yet because the flat-versus-platforms comparison
/// in `fighter-brain.md` used this geometry. Change the geometry and the
/// measurement together.
pub fn smash_platform_stage() -> RoomSpec {
    let centre_x = STAGE_SIZE.x / 2.0;
    let main = ae::Block::solid(
        "smash_platform",
        Vec2::new((STAGE_SIZE.x - PLATFORM_WIDTH) / 2.0, PLATFORM_TOP),
        Vec2::new(PLATFORM_WIDTH, 32.0),
    );
    // y grows downward, so a platform above the stage has a smaller y.
    let soft = |name: &str, x: f32, rise: f32| {
        ae::Block::one_way(
            name.to_string(),
            Vec2::new(x - SOFT_PLATFORM_SIZE.x / 2.0, PLATFORM_TOP - rise),
            SOFT_PLATFORM_SIZE,
        )
    };
    smash_stage_room(
        "Smash Stage (platforms)",
        SMASH_PLATFORM_STAGE_ROOM_ID,
        vec![
            main,
            soft(
                "smash_soft_left",
                centre_x - SOFT_PLATFORM_SPREAD,
                SOFT_PLATFORM_LOW_RISE,
            ),
            soft(
                "smash_soft_right",
                centre_x + SOFT_PLATFORM_SPREAD,
                SOFT_PLATFORM_LOW_RISE,
            ),
            soft("smash_soft_top", centre_x, SOFT_PLATFORM_HIGH_RISE),
        ],
    )
}

pub fn stage_centre() -> Vec2 {
    Vec2::new(STAGE_SIZE.x / 2.0, PLATFORM_TOP)
}

/// What the match announces when it ends.
///
/// `announce_the_winner` resolves the winning side into the fighter's name
/// before the call; this function owns only the wording.
pub fn victory_banner(
    outcome: &ambition_platformer2d::actor::MatchVerdict,
    winner_name: Option<&str>,
) -> String {
    use ambition_platformer2d::actor::MatchVerdict;
    match outcome {
        // The name from the caller; a side label is not a name.
        MatchVerdict::Winner(side) => format!("WINNER: {}", winner_name.unwrap_or(side)),
        // A draw is reachable and cheaply: two fighters on their last stock,
        // knocked off together.
        MatchVerdict::Draw => "Draw — everybody fell".to_string(),
        // Say so: "Draw" would tell players the fighters settled something.
        MatchVerdict::NoContest => "NO CONTEST".to_string(),
    }
}

/// The two answers the engine refuses to guess, wired to the messages it
/// writes.
///
/// `ambition_platformer2d::combat::stocks` spends the stock, clears the meter
/// and marks the elimination. Placing a body needs a stage and announcing a
/// winner needs a scoreboard, so this plugin supplies both.
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
        // D192: this stage authors the respawn beat. The engine default is
        // zero (same-tick placement).
        app.insert_resource(ambition_platformer2d::actor::RespawnInterval {
            seconds: RESPAWN_INTERVAL_SECONDS,
        });

        let sim = ambition_platformer2d::platformer::schedule::SimScheduleExt::sim_schedule(app);
        // Sing, in `ContentSpecials`: its effect must exist before the effect
        // executors run. Authored sleeps today: the performer's `speech` pulse
        // (`performer_moveset`) and the Shadow Oni leader's seal. The bridge
        // from `MoveEventKind::Effect` is the keyed-technique dispatch in
        // `crates/ambition_combat/src/moveset/mod.rs`.
        //
        // The mash runs first, so a press cannot be spent on a sleep that did
        // not exist when it was made. `apply_authored_sleep` takes a `max`, so
        // the landing tick is the same either way.
        install_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_sleep::SLEEP,
            TechniqueOffer {
                owner: "ambition_demo_smash::sing",
                params: TechniqueParams::Checked(
                    check_hydrates::<ambition_platformer2d::entity_catalog::smash_sleep::SleepParams>,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            (
                crate::sing::mash_out_of_sleep,
                crate::sing::apply_authored_sleep,
            )
                .chain()
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials),
        );
        // Portal recovery, in `ContentSpecials`. The close is chained after the
        // open, so an aperture's lifetime is spent from the frame it appears.
        install_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_portal::PORTAL_PAIR,
            TechniqueOffer {
                owner: "ambition_demo_smash::portal",
                params: TechniqueParams::Checked(
                    check_hydrates::<
                        ambition_platformer2d::entity_catalog::smash_portal::PortalPairParams,
                    >,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            (
                crate::portal::open_authored_portal_pairs,
                crate::portal::close_expired_move_portals,
            )
                .chain()
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials),
        );
        // Steered bolt. Fire is chained before flight, so a bolt moves on the
        // tick it appears and does not sit inside the caster for a frame.
        install_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_bolt::STEERED_BOLT,
            TechniqueOffer {
                owner: "ambition_demo_smash::bolt",
                params: TechniqueParams::Checked(
                    // The domain rule, not only "serde can build it": rejects
                    // an invisible bolt, a trail never redrawn, an unsteerable bolt.
                    ambition_platformer2d::entity_catalog::smash_bolt::check_steered_bolt_params,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            (
                crate::bolt::fire_authored_bolts,
                crate::bolt::steer_and_fly_bolts,
            )
                .chain()
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials),
        );
        // Homing dash. Begin is chained before carry, so a dash steers on the
        // tick it starts.
        install_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_homing::HOMING_DASH,
            TechniqueOffer {
                owner: "ambition_demo_smash::homing",
                params: TechniqueParams::Checked(
                    check_hydrates::<
                        ambition_platformer2d::entity_catalog::smash_homing::HomingDashParams,
                    >,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            (
                crate::homing::begin_authored_homing_dashes,
                crate::homing::carry_homing_dashes,
            )
                .chain()
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials),
        );
        // Riposte. It reads the same `ActorActionMessage` the counter writes,
        // so it needs no counter-specific wiring.
        install_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_riposte::RIPOSTE_STRIKE,
            TechniqueOffer {
                owner: "ambition_demo_smash::riposte",
                params: TechniqueParams::Checked(
                    // The domain rule, not only "serde can build it".
                    ambition_platformer2d::entity_catalog::smash_riposte::check_riposte_strike_params,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            crate::riposte::cut_where_a_riposte_answers
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials),
        );
        // Tether reel. Throw is chained before reel, so a line that bit a ledge
        // pulls on that tick. The reel does not catch the ledge; the movement
        // kernel's ledge authority does, one phase later. See `crate::tether`.
        install_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_tether::TETHER_PULL,
            TechniqueOffer {
                owner: "ambition_demo_smash::tether",
                params: TechniqueParams::Checked(
                    check_hydrates::<
                        ambition_platformer2d::entity_catalog::smash_tether::TetherPullParams,
                    >,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            (
                crate::tether::begin_authored_tether_pulls,
                crate::tether::reel_tethered_fighters,
            )
                .chain()
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials),
        );
        // Placed spring. Drop is chained before fire, so a plate starts its
        // arming clock on the tick it lands (while its dropper stands in it).
        install_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_spring::PLACE_SPRING,
            TechniqueOffer {
                owner: "ambition_demo_smash::spring",
                params: TechniqueParams::Checked(
                    check_hydrates::<
                        ambition_platformer2d::entity_catalog::smash_spring::PlaceSpringParams,
                    >,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            (
                crate::spring::drop_authored_springs,
                crate::spring::fire_and_expire_springs,
            )
                .chain()
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials),
        );
        // Limit meter: a set of independent sources, not one rule, so a
        // mechanic can author only the source it wants.
        //
        // The rule is not inserted here: a rule inserted at plugin build reaches
        // every body in the composing app, including Ambition's player. The stage
        // declares it and gives it back; see
        // `the_stage_declares_smashs_presentation_and_gives_it_back`.
        //
        // The two halves run in different phases. `ContentSpecials` is inside
        // `Materialize`, before `Resolve`, so a damage reader there reads the
        // previous frame's `ResolvedBodyHit`. Under GGRS that cross-frame read
        // diverges (`clear_message_on_rollback` drops the leftover at load), and
        // `BodyMana` desyncs. So the damage half runs in `ContentFlavor`
        // (between `Resolve` and `Settle`), and reads each hit on the frame it
        // is emitted.
        //
        // A latching reader (sets booleans, like
        // `mark_move_playback_resolved_hits`) can read a frame late. An
        // accumulator (`+=`) cannot.
        // Sweep objects from ended matches in `CombatSet::Trigger`, the
        // earliest combat phase, so no fighter can trip a leftover mine. The
        // match owns cleanup: techniques stamp a marker; they do not despawn.
        app.add_systems(
            sim,
            crate::match_scope::sweep_objects_from_ended_matches
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Trigger),
        );
        app.add_systems(
            sim,
            crate::limit::fill_limit_meters
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentFlavor),
        );
        // The authored half runs where a dispatched special belongs, on the
        // frame it is pressed.
        install_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_limit::FILL_METER,
            TechniqueOffer {
                owner: "ambition_demo_smash::limit",
                params: TechniqueParams::Checked(
                    check_hydrates::<ambition_platformer2d::entity_catalog::smash_limit::FillMeterParams>,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            crate::limit::apply_authored_meter_fills
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials),
        );
        // Time dilation, chained expire then apply. `PlayerSimulation` runs
        // before `Combat`, so a scale written here is first observed on the
        // next tick. If apply ran before expire, the same tick would spend one
        // tick of the dilation, and N authored ticks would give N-1 slowed ones.
        // Guarded by `a_one_tick_dilation_is_still_in_force_next_tick`.
        //
        // Re-application is safe: `apply` keeps the original `prior` while a
        // dilation is live.
        install_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_time_dilation::TIME_DILATION,
            TechniqueOffer {
                owner: "ambition_demo_smash::dilation",
                params: TechniqueParams::Checked(
                    // The domain rule, not only "serde can build it".
                    ambition_platformer2d::entity_catalog::smash_time_dilation::check_time_dilation_params,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            (
                crate::dilation::expire_time_dilations,
                crate::dilation::apply_authored_time_dilations,
            )
                .chain()
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials),
        );
        // Remote mine. Arm is chained before press, so a mine that becomes
        // live this tick answers this tick's press. The other order adds a frame
        // to the authored arming delay.
        install_techniques(
            app,
            &[
                (
                    ambition_platformer2d::entity_catalog::smash_mine::PLACE_MINE,
                    TechniqueOffer {
                        owner: "ambition_demo_smash::mine",
                        params: TechniqueParams::Checked(
                            check_hydrates::<
                                ambition_platformer2d::entity_catalog::smash_mine::PlaceMineParams,
                            >,
                        ),
                        references: NestedReferences::HeldItems(
                            ambition_platformer2d::entity_catalog::smash_mine::mine_held_item_refs,
                        ),
                        delivery: TechniqueDelivery::Action,
                    },
                ),
                (
                    ambition_platformer2d::entity_catalog::smash_mark::MARK_BODY,
                    TechniqueOffer {
                        owner: "ambition_demo_smash::mark",
                        params: TechniqueParams::Checked(
                            check_hydrates::<
                                ambition_platformer2d::entity_catalog::smash_mark::MarkBodyParams,
                            >,
                        ),
                        references: NestedReferences::None,
                        delivery: TechniqueDelivery::OnHit,
                    },
                ),
            ],
            (
                crate::mine::arm_placed_mines,
                crate::mine::place_or_detonate_authored_mines,
                // Mark: tick, then apply. The tick that applies a mark spends
                // none of its fuse, so the victim gets the full authored fuse.
                crate::mark::detonate_body_marks,
                crate::mark::apply_authored_body_marks,
            )
                .chain()
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials),
        );
        // Counter stance, in `Materialize`: `apply_hitbox_damage` judges in
        // `Resolve`, and the arming effect is emitted in `Playback`. This runs
        // between them. It runs every frame of the stance, because
        // `parry_window_timer` decays.
        app.add_systems(
            sim,
            crate::counter::hold_counter_parry_windows
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Materialize),
        );
        // Mark clocks. The body-clock view owner clears it in the sim tail;
        // this contributes the marks after the clear.
        app.add_systems(
            sim,
            crate::mark::publish_mark_clocks
                .in_set(ambition_platformer2d::sim_view::BodyClockViewSet::Contribute),
        );
        // Counter answer, once the verdict is in. `Settle` needs no explicit
        // edge: `ParriedBodyHit` is written in the earlier `Resolve` phase. An
        // explicit edge is needed only for a message written in the reader's
        // own set.
        //
        // The response lands on the next tick: its `ActorActionMessage` is read
        // in `Materialize`, which has passed. The wait makes the catch visible.
        install_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_counter::COUNTER,
            TechniqueOffer {
                owner: "ambition_demo_smash::counter",
                params: TechniqueParams::Checked(
                    check_hydrates::<ambition_platformer2d::entity_catalog::smash_counter::CounterParams>,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            crate::counter::answer_a_parry_with_the_authored_counter
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle),
        );

        // Jostle is a fact the movement kernel reads, so it is set in the
        // simulation in `WorldPrep`, before anything integrates a body.
        app.add_systems(
            sim,
            smash_fighters_are_solid_to_each_other.in_set(
                ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep,
            ),
        );
        // Shark summon, in `ContentSpecials`: it must produce its effects
        // before the effect executors run (`ContentSpecials.before(
        // EffectExecutionSet)`). Name the set; do not add a leaf-to-leaf edge.
        install_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_ride::SUMMON_RIDE,
            TechniqueOffer {
                owner: "ambition_demo_smash::shark_ride",
                params: TechniqueParams::Checked(
                    check_hydrates::<ambition_platformer2d::entity_catalog::smash_ride::SummonRideParams>,
                ),
                references: NestedReferences::Characters(
                    ambition_platformer2d::entity_catalog::smash_ride::summon_ride_character_refs,
                ),
                delivery: TechniqueDelivery::Action,
            },
            crate::shark_ride::translate_shark_summons
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials), // ⛔⛔ ~~AND THE EXPLICIT EDGE, WHICH THE SET DOES NOT IMPLY~~ —
                                                                                                  // the runtime puts `apply_summon_effects` in
                                                                                                  // `EffectExecutionSet`, so set membership is the whole
                                                                                                  // ordering. Without it the shark never appears.
        );
        // Bomb. Recognised like the shark summon (an `ActorActionMessage`
        // technique); the fuse burns in `Settle`, after item physics.
        install_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_bomb::DROP_BOMB,
            TechniqueOffer {
                owner: "ambition_demo_smash::bomb",
                params: TechniqueParams::Checked(
                    check_hydrates::<ambition_platformer2d::entity_catalog::smash_bomb::DropBombParams>,
                ),
                references: NestedReferences::HeldItems(
                    ambition_platformer2d::entity_catalog::smash_bomb::bomb_held_item_refs,
                ),
                delivery: TechniqueDelivery::Action,
            },
            crate::bomb::translate_bomb_drops
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials),
        );
        app.add_systems(
            sim,
            crate::bomb::burn_fuses_and_answer_impacts
                .after(ambition_platformer2d::platformer::schedule::ItemPickupSet::CoreHeldItems)
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle),
        );
        // What ends a ride, and what the shark does afterwards. `Settle`,
        // chained: each reads state the tick already produced. The two that
        // request a dismount run before `DismountRequestsApplied` (the set
        // `ambition_mount` publishes).
        app.add_systems(
            sim,
            (
                crate::shark_ride::dissolve_the_ride_when_the_shark_dies,
                crate::shark_ride::dismount_launched_riders,
                crate::shark_ride::dismount_riders_who_left_play,
                crate::shark_ride::bail_out_of_the_saddle,
            )
                .chain()
                .before(ambition_platformer2d::mount::DismountRequestsApplied)
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle),
        );
        app.add_systems(
            sim,
            (
                crate::shark_ride::depart_when_riderless,
                crate::shark_ride::send_away_a_shark_nobody_boarded,
            )
                .chain()
                .after(ambition_platformer2d::mount::DismountRequestsApplied)
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle),
        );
        // Departure writes intent in `BeforeIntegrate`. In `Settle`, the
        // riderless shark's brain would overwrite `ActorControl` on the next
        // tick before movement.
        app.add_systems(
            sim,
            crate::shark_ride::tick_departures
                .in_set(ambition_platformer2d::platformer::schedule::WorldPrepSet::BeforeIntegrate),
        );
        // The footstool claims the press in `PlayerInput`, before the kernel
        // spends the air jump. The claim sets
        // `BodyJumpState::footstool_claimed` ahead of the jump chain.
        app.add_systems(
            sim,
            ambition_platformer2d::combat::footstool::claim_footstools.in_set(
                ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith::PlayerInput,
            ),
        );
        // Ledge trump resolves after the kernel, so it sees this tick's grabs.
        // Before `PlayerSimulation` it would judge last tick's occupancy and
        // leave both bodies hanging for a frame. `Settle` is the post-kernel
        // bookkeeping slot.
        app.add_systems(
            sim,
            ambition_platformer2d::combat::ledge_trump::resolve_ledge_trumps
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle),
        );
        // The capture interruption release is in `CombatSchedulePlugin`. Do
        // not add it here; it would run twice.
        //
        // These rules run after the engine's `Settle` work: the stock must be
        // spent before a body is placed. The HUD publisher only presents; it
        // shares the gated set so a hosted build stops drawing it outside the
        // stage.
        let rules = (
            publish_smash_hud,
            announce_the_opening_countdown,
            place_respawning_fighters,
            ambition_platformer2d::actor::tick_respawn_grace,
            a_swing_spends_the_respawn_protection,
            hold_the_respawn_platforms,
            leaving_the_platform_spends_the_respawn_protection,
            announce_the_winner,
        )
            .chain()
            .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle)
            .after(ambition_platformer2d::combat::stocks::FighterStocksSpent)
            // Also after the return is decided: with a respawn interval, a
            // placement racing the tick-down would miss a due fighter.
            .after(ambition_platformer2d::combat::stocks::FighterRespawnsDue);
        // Sudden death's stage half writes rollback-canonical `BodyHealth`, so
        // it runs in the simulation, not `Update`. It reads the message from
        // `decide_stocks_match`, so it runs after `MatchOutcomeDecided`.
        app.add_systems(
            sim,
            open_the_sudden_death_round
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle)
                .after(ambition_platformer2d::combat::stocks::MatchOutcomeDecided),
        );
        // This rule runs after the decision, outside the chain above. It
        // despawns eliminated bodies, and `decide_stocks_match` reads sides from
        // the bodies that exist. If it despawned the last loser first,
        // `last_side_standing` would see one side and the match would never end.
        // Only this system waits: the HUD, countdown and placement run beside
        // the decision (see `FighterStocksSpent`).
        let remove_the_eliminated = take_eliminated_fighters_out_of_play
            .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle)
            .after(ambition_platformer2d::combat::stocks::MatchOutcomeDecided);
        if self.hosted {
            let gate = ambition_platformer2d::runtime::in_mode(SMASH_MODE);
            // The retraction observer is ungated: `RespawnGrace` can leave for
            // any reason (its clock, a swing, a rebuild, a mode teardown), and a
            // stale reason bit would keep a fighter invulnerable.
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
/// `BodyHealth::damage_percent()` is unclamped (see
/// `damage_percent_is_unclamped_so_a_hud_can_print_188`), and `FighterStocks`
/// keeps `started_with` so the HUD can draw "2 of 3".
pub const FIGHTER_HUD_SLOTS: [&str; 4] = [
    "smash_fighter_0",
    "smash_fighter_1",
    "smash_fighter_2",
    "smash_fighter_3",
];
/// Temporary: how much faster than authored the opening countdown runs. Set to
/// `1` to restore the authored three seconds.
///
/// It divides the ticks, not the beats: `MatchRules::beats()` still counts
/// three. Tests read the roster's value, not a literal.
const COUNTDOWN_SPEEDUP: u32 = 10;

/// The winner card. One slot, because the stage says one thing at a time.
pub const SMASH_ANNOUNCE_HUD_SLOT: &str = "smash_announce";

/// What one remaining stock is drawn as, under the sprites asset root.
///
/// Generated, not committed: `scripts/regen/sprites.sh` lists it in its publish
/// roster, so a fresh clone can produce it.
pub const STOCK_ICON_ASSET: &str = "sprites/hud_stock_icon.png";

/// What plays on the stage.
pub const SMASH_STAGE_TRACK: &str = "super_smash_siblings_theme";
/// What plays over the character select, in a host whose frontend audio
/// this demo owns. See `SMASH_TRACKS` for why it is registered either way.
pub const SMASH_SELECT_TRACK: &str = "super_smash_siblings_character_select";

/// The scores written for this demo, rendered from
/// `tools/ambition_music_renderer/scores/active/super_smash_siblings_*.music.yaml`.
///
/// All three are registered, not only the one that plays: a track in this
/// fragment is one the experience may play (radio, stage select, winner card).
/// The default plays when nobody asks.
///
/// The asset path is derived (`audio/music/generated/<id>/full.ogg`), because
/// that layout is the renderer's contract.
pub const SMASH_TRACKS: &[(&str, &str)] = &[
    (SMASH_STAGE_TRACK, "Super Smash Siblings"),
    (SMASH_SELECT_TRACK, "Choose Your Fighter"),
    (
        "super_smash_siblings_grand_symphony",
        "Super Smash Siblings — Grand Symphony",
    ),
];

/// The combat rules this stage declares, in one place so the publisher and its
/// guard use the same copy.
///
/// A function, not a resource read: on a second visit the resource holds the
/// previous match's declaration.
pub fn smash_declared_combat_rules() -> ambition_platformer2d::combat::rules::DeclaredCombatRules {
    ambition_platformer2d::combat::rules::DeclaredCombatRules {
        // By owner. The versus route also declares combat rules, and a
        // giveback by type would delete its live rules when smash left.
        declared_by: SMASH_EXPERIENCE.to_string(),
        di_max_angle: SMASH_DI_MAX_ANGLE,
        knockback_growth: SMASH_KNOCKBACK_GROWTH,
        // The robot's down-air can rebound its attacker. Ambition uses that;
        // a platform fighter must not, because offstage it would save the
        // attacker instead of killing the victim.
        downward_hit: ambition_platformer2d::combat::rules::DownwardHitStyle::Spike,
        // Meteor lock: about 18 frames in which a spiked body cannot recover.
        // Long enough to kill offstage, short enough to survive over the stage.
        // The end of the window is the meteor cancel.
        meteor_lock_time: 0.30,
        // Rage, capped at 1.4x. Percent already makes a hurt fighter easier
        // to launch; the cap keeps a comeback a chance, not a coin flip.
        rage_per_damage: 0.004,
        rage_max_scale: 1.4,
        // Staling, floored at 0.55: nine landings of one move and it is worth
        // about half. Varying moves lets it recover.
        stale_step: 0.05,
        stale_floor: 0.55,
        // Staling reduces damage in full but launch only a little. At high
        // percent the launch is mostly the percent term, so full staling would
        // stop a fighter's best kill move from killing. At 0.30 a fully stale
        // move keeps `1 - 0.30 * (1 - 0.55)` = 86.5% of its reach and deals 55%
        // of its damage.
        stale_knockback_influence: Some(0.30),
        // The percent curve's steepness; see
        // `SMASH_VICTIM_PERCENT_KNOCKBACK_SCALE`.
        victim_percent_knockback_scale: Some(SMASH_VICTIM_PERCENT_KNOCKBACK_SCALE),
        // Kill moves are authored explicitly, not inferred. Base knockback is
        // not role: a growth curve on base knockback overrode authored
        // `knockback_growth`, distorted deliberate high-BKB/low-KBG shoves, and
        // did not reach throws. Movesets author the separation instead (pirate
        // f-smash `growth/base` 0.0256, up-smash 0.0413, jab 0.020), derived
        // from measured stage thresholds (`G_new = G_old * p0/p1` for strikes,
        // `* (p0+d)/(p1+d)` for throws).
        growth_base: None,
        // Crouch cancel, 0.85x: a defensive read at low percent that does not
        // save anyone from a kill move.
        crouch_cancel_scale: 0.85,
        // No blanket mercy window, as in Smash. Repeat protection belongs to
        // the move: one hitbox cannot hit a body twice, and separate Active
        // windows can. A 0.2s window would also stop George Booul's `bivalence`
        // launcher (0.42s) from landing after its pop (0.30s).
        hit_repeat_window_scale: 0.0,
        // Edge cancel: an aerial landed on a platform edge that slides off
        // cancels its landing lag.
        edge_cancel_recovery: Some(true),
        // B-reverse: a special pressed backward turns the fighter around.
        special_turn: Some(true),
        // Wavebounce: the turn also reverses drift. One rule with two
        // settings, so neither becomes a per-fighter hack. Both are feel calls.
        special_turn_reverses_drift: Some(true),
        // Clanking is off: a tuning decision, the mechanism is tested. At 9
        // damage (the genre's threshold) CPUs traded so often that nobody was
        // launched (`every_live_fighter_stays_inside_the_frame`) and
        // `the_cpu_charges_a_smash_and_techs_a_landing_in_some_match` failed,
        // even ground-only. Try `9.0` first after a play session.
        clank_damage_window: 0.0,
        // A clank pushes both fighters back: less than a launch, but out of
        // each other's next swing.
        clank_rebound_speed: 190.0,
        // Sudden death at 150%: a timed match that ends level goes to a
        // point where almost any clean hit kills.
        sudden_death_damage: Some(150),
        // Losing the ledge to a trump pushes the previous holder off, so a
        // trump is a real edge-guard option. 260px/s is a shove, not a kill.
        // A starting value; tune by play.
        ledge_trump_pop: Some(260.0),
        // Ultimate's rule: a recovering fighter can steal the edge back.
        // `Hog` is the other generation's rule.
        ledge_occupancy: Some(ambition_platformer2d::combat::rules::LedgeOccupancy::Trump),
        // Double-jump cancel: an aerial from an air jump ends the jump's rise.
        // A feel call.
        double_jump_cancel: Some(true),
        // One hit in six barks. A rate, not a cooldown: a cooldown makes the
        // first hit of every exchange bark, which players hear as a rhythm.
        // A starting value; tune by play.
        bark_chance: Some(1.0 / 6.0),
        // Grab hold grows with percent (Ultimate's 90 + 1.7p frames: 1.5s at
        // 0%, about 4.3s at 100%). Percent is read at the grab, so pummels do
        // not extend the hold.
        grab_hold_base_seconds: 90.0 / 60.0,
        grab_hold_per_damage: 1.7 / 60.0,
        // Every hold ends, however hurt the captive is.
        grab_hold_max_seconds: 6.0,
        // 14.4 frames per press (Ultimate's rate), so mashing is a real
        // option.
        grab_mash_seconds: 14.4 / 60.0,
        // Teams decide who may hit whom. Global friendly fire would also make
        // teammates hittable.
        friendly_fire: false,
    }
}

/// The kit this experience gives a fighter that authors none.
///
/// A roster-preparation policy, not a combat rule: `DeclaredCombatRules` does
/// not own a kit. Many of Ambition's cast author `default_action_set:
/// "peaceful"`; seating one in an arena adapts it into a fighter.
/// `roster_seeded` folds this into the seat's `ActionSet` at seating time, so
/// the body has one move authority and nothing reads a fallback.
///
/// The numbers copy the exploration provoke: 0.22 / 0.08 / 0.26, 4 damage,
/// 34 reach.
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
/// A still: this panel never ticks a frame. It also crops the portrait sheet,
/// which holds every clip the character can wear.
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

/// Publish percent and stocks for every seated fighter.
///
/// Percent is not health: the gauge fills as damage accumulates and the number
/// counts past 100%. The fill may clamp for rendering; the number does not.
pub fn publish_smash_hud(
    fighters: bevy::prelude::Query<(
        &ambition_platformer2d::versus_match::MatchSeat,
        &ambition_platformer2d::characters::actor::BodyHealth,
        Option<&ambition_platformer2d::actor::FighterStocks>,
        // The HUD punch reads `hitstop_timer`, which is non-zero just after a
        // hit and already scaled by damage (`hitlag_duration`). Do not track a
        // percent delta in presentation: it disagrees with the sim on blocked,
        // armored or zero-damage hits.
        Option<&ambition_platformer2d::characters::actor::BodyCombat>,
        &bevy::prelude::Name,
        // The character id, for the portrait. `Name` is only a display string.
        Option<&ambition_platformer2d::characters::actor::WornCharacter>,
    )>,
    // The game resolves the portrait, not the renderer: "which character" is
    // content (see `HudStanding::portrait`).
    catalog: Option<
        bevy::prelude::Res<
            ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog,
        >,
    >,
    // Ask the portrait manifests for a still. Both are optional: without
    // them the panel draws an uncropped portrait.
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
                // Normalised against the longest possible freeze, so the
                // strongest hit is a full punch.
                combat.map_or(0.0, |combat| {
                    (combat.hitstop_timer / HUD_PUNCH_REFERENCE_HITLAG).clamp(0.0, 1.0)
                }),
            )
        })
        .collect();
    // Sort by seat: query order is not stable, and a scoreboard must not
    // swap sides mid-match.
    rows.sort_by_key(|(seat, ..)| *seat);

    let mut written = [false; FIGHTER_HUD_SLOTS.len()];
    for (seat, _name, percent, stocks, face, emphasis) in &rows {
        let Some(slot) = FIGHTER_HUD_SLOTS.get(*seat) else {
            continue;
        };
        written[*seat] = true;
        // Stocks are icons, so no fraction text.
        let value = format!("{:.0}%", percent * 100.0);
        let (remaining, started) = stocks.unwrap_or((0, 0));
        readouts.set(
            *slot,
            ambition_platformer2d::presentation::HudReadout::standing(
                // No label: `text()` joins label and value, and a name would
                // overflow the 132px panel. The portrait identifies the fighter.
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
    // A 1v1 declares four slots and fills two. Clear unwritten slots, so they
    // do not keep the previous match's fighters.
    for (index, slot) in FIGHTER_HUD_SLOTS.iter().enumerate() {
        if !written[index] {
            readouts.clear_slot(*slot);
        }
    }
}

/// 3, 2, 1, go.
///
/// The roster opens `opens_suspended`, which stamps `ControlHolds` on every
/// fighter when it is created, and declares `opening_countdown_ticks`. The
/// engine removes the hold for every seat on one tick
/// (`release_the_opening_hold`). This system only shows the numbers.
///
/// The card is derived from `now - activated_on`, the same function that drives
/// the release, so the two cannot drift. It holds no state: a readout write is
/// idempotent, so writing the same word every tick is free.
fn announce_the_opening_countdown(
    active: Option<bevy::prelude::Res<ambition_platformer2d::versus_match::ActiveMatch>>,
    prepared: Option<bevy::prelude::Res<ambition_platformer2d::versus_match::PreparedMatch>>,
    tick: Option<bevy::prelude::Res<ambition_platformer2d::time::SimTick>>,
    settled: Option<bevy::prelude::Res<ambition_platformer2d::versus_match::StocksMatchSettled>>,
    // The sudden-death latch: see the stand-down below.
    sudden_death: Option<
        bevy::prelude::Res<ambition_platformer2d::versus_match::SuddenDeathEntered>,
    >,
    mut readouts: bevy::prelude::ResMut<ambition_platformer2d::presentation::HudReadouts>,
) {
    use ambition_platformer2d::versus_match::OpeningPhase;
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
    // The opening owns the card until the match is decided; then the outcome
    // owns it.
    if settled.is_some_and(|settled| settled.settled(&active)) {
        return;
    }
    // Sudden death also takes the card. It leaves the match unsettled (the
    // match continues), so the check above does not catch it.
    //
    // Read the rollback-registered `SuddenDeathEntered` latch every frame, not
    // the one-shot `SuddenDeathBegan` message. The sim must not write this
    // slot: `HudReadouts` is not rollback state, so a rewind that undid the
    // timeout would leave the banner standing. Deriving it from the latch
    // makes it appear and disappear with the round.
    //
    // Not covered by a test: `PreparedMatch` has no public constructor, so it
    // needs an integration harness that runs a timed match to expiry.
    if sudden_death.is_some_and(|entered| entered.entered(&active)) {
        readouts.set(
            SMASH_ANNOUNCE_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::bare("SUDDEN DEATH".to_string()),
        );
        return;
    }
    let total = u64::from(rules.opening_countdown_ticks);
    // One beat, computed as `opening_phase` computes it, so "GO!" holds as
    // long as each number did.
    let per_beat = total.div_ceil(u64::from(rules.opening_beats().max(1)));
    let word = match rules.opening_phase(elapsed) {
        OpeningPhase::Counting { beats_remaining } => Some(beats_remaining.to_string()),
        // GO holds one beat past the release, then the card comes down.
        OpeningPhase::Live if elapsed < total + per_beat => Some("GO!".to_string()),
        OpeningPhase::Live => None,
    };
    match word {
        Some(word) => readouts.set(
            SMASH_ANNOUNCE_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::bare(word),
        ),
        // Unconditional: the arms above already handed the slot over.
        None => readouts.clear_slot(SMASH_ANNOUNCE_HUD_SLOT),
    }
}

/// Put a respawning fighter back over the platform.
///
/// Uses `reset_body_clusters`, not `transit_body`. Both re-resolve the pose
/// against the world (ADR 0024), but `transit_body` keeps maneuver state
/// (coyote, buffers, dash timers). A fighter that lost a stock must not keep
/// them. `reset_body_clusters` is the "this body starts again" verb (also used
/// by the sandbox reset and versus rounds). It raises `BodyRestartLatch`, so
/// `announce_body_restarts` triggers `ae::BodyRestarted` for providers.
fn place_respawning_fighters(
    mut commands: bevy::prelude::Commands,
    mut due: bevy::prelude::MessageReader<ambition_platformer2d::actor::FighterRespawnDue>,
    mut bodies: bevy::prelude::Query<(
        ambition_platformer2d::actor::BodyClusterQueryData,
        &mut ambition_platformer2d::actor::MotionModel,
        // The seat, so two fighters returning on one frame do not overlap.
        // Optional: an unseated body must still be placed.
        Option<&ambition_platformer2d::actor::MatchSeat>,
        // The swing in progress at the knockout. Needed by value: only the
        // playback knows which strike boxes to despawn.
        Option<&mut ambition_platformer2d::combat::moveset::MovePlayback>,
    )>,
) {
    // D192: the cue is the interval elapsing. Eliminated fighters never get a
    // pending respawn, so the engine decides who returns; this decides where.
    for event in due.read() {
        let Ok((clusters, mut model, seat, playback)) = bodies.get_mut(event.body) else {
            continue;
        };
        let seat = seat.map_or(0, |seat| seat.0);
        let mut item = clusters;
        let mut clusters = item.as_clusters_mut();
        // The reset zeroes velocity, so the fighter does not keep flying
        // toward the blast zone it just left.
        let placement = respawn_placement(stage_centre(), seat);
        ambition_platformer2d::engine_core::reset_body_clusters(
            &mut model,
            &mut clusters,
            placement,
            // Face the stage centre. `respawn_placement` alternates seats
            // either side of centre, so a fixed facing sends odd seats back
            // looking away, and mirror bouts stop mirroring after a respawn.
            // Centre, not the opponent: this system places one body, and with
            // four seats "the opponent" has no single direction.
            ambition_platformer2d::engine_core::ResetFacing::Toward(
                stage_centre().x - placement.x,
            ),
            // The engine's default air game. A stage that tunes it passes its
            // own number here.
            ambition_platformer2d::engine_core::DEFAULT_TUNING.air_jumps,
        );
        // Cancel the swing the fighter carried into the knockout, through the
        // one teardown path that also despawns its strike boxes. Otherwise the
        // returning body is "acting" when it appears and spends the protection
        // below at once.
        if let Some(mut playback) = playback {
            ambition_platformer2d::combat::moveset::cancel_move_playback(
                &mut commands,
                event.body,
                &mut playback,
                // The body left play; a storing charge does not bank across a
                // stock (see `MoveEnd`).
                ambition_platformer2d::combat::moveset::MoveEnd::LeftPlay,
            );
        }
        // Respawn protection: the ruleset grants it, not the character. The
        // opponent that took the stock is standing there. `RespawnGrace` has
        // its own clock and publishes the `Invulnerability::RESPAWN` reason
        // bit. Do not borrow `Empowered`: it is one component, so it would
        // overwrite a power-up the body already carries.
        commands
            .entity(event.body)
            .try_insert(ambition_platformer2d::actor::RespawnGrace {
                remaining: RESPAWN_PROTECTION_SECONDS,
            });
    }
}

/// The respawn platform exists exactly as long as `RespawnGrace` does, so the
/// protection is visible.
///
/// The platform has no timer of its own; it is present while the seat's
/// fighter has the grace, so the two cannot disagree. It is ordinary
/// collision: anyone may stand on it, and anyone on it falls when it goes.
fn hold_the_respawn_platforms(
    mut platforms: bevy::prelude::ResMut<
        ambition_platformer2d::world::collision::MovingPlatformSet,
    >,
    // `RespawnGrace` removes itself when it runs out, so the platform's
    // presence is the component's presence.
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
    // Sort by id so the order depends only on which seats are protected. The
    // visuals reconcile by index and the resource is rollback-canonical.
    wanted.sort_by(|a, b| a.0.cmp(&b.0));

    // Place each platform once; do not rebuild it from `kin.pos` every tick.
    // A platform that tracks the body makes a brain's ledge distance constant,
    // so it vetoes every verb (`D-BRAIN-PLATFORM-FLOOR`). A respawn platform
    // is somewhere you leave.
    platforms.0.retain(|platform| {
        !is_respawn_platform_id(&platform.id)
            || wanted.iter().any(|(id, _)| *id == platform.id)
    });
    for (id, centre) in wanted {
        // Already placed: leave it where it is.
        if platforms.0.iter().any(|platform| platform.id == id) {
            continue;
        }
        platforms.0.push(
            ambition_platformer2d::world::platforms::MovingPlatformState::from_sweep(
                id,
                "Respawn platform",
                centre,
                RESPAWN_PLATFORM_SIZE,
                // Stationary: a zero-width sweep at zero speed.
                0.0,
                0.0,
            ),
        );
    }
}

/// Attacking gives up the respawn protection (the genre's anti-camping rule).
///
/// Only the grant this ruleset gave: `RespawnGrace` is removed, so any other
/// invulnerability the body has stays. The trigger is a move's playback
/// appearing, not a held button or movement axis. Leaving the platform also
/// ends it; see `leaving_the_platform_spends_the_respawn_protection`.
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
        // The removal hook clears `Invulnerability::RESPAWN`; other reasons
        // stay.
        commands
            .entity(body)
            .remove::<ambition_platformer2d::actor::RespawnGrace>();
    }
}

/// Leaving the platform also spends the protection, as in Smash.
///
/// The platform is placed once and does not track the body
/// (`the_respawn_platform_stays_where_it_was_placed`). The rule is leaving,
/// not input: a body keeps its fall and landing, and loses the platform when
/// it moves off the footprint.
fn leaving_the_platform_spends_the_respawn_protection(
    mut commands: bevy::prelude::Commands,
    platforms: bevy::prelude::Res<ambition_platformer2d::world::collision::MovingPlatformSet>,
    standing: bevy::prelude::Query<
        (
            bevy::prelude::Entity,
            &ambition_platformer2d::actor::MatchSeat,
            &ambition_platformer2d::engine_core::BodyKinematics,
        ),
        bevy::prelude::With<ambition_platformer2d::actor::RespawnGrace>,
    >,
) {
    for (body, seat, kin) in &standing {
        let id = respawn_platform_id(seat.0);
        let Some(platform) = platforms.0.iter().find(|platform| platform.id == id) else {
            // No platform for this seat yet: this is the grant's first tick.
            continue;
        };
        // Horizontal only: a body falling toward its platform has not left
        // it; one that walks off the end has.
        let half_width = platform.size.x * 0.5;
        if (kin.pos.x - platform.pos.x).abs() > half_width {
            commands
                .entity(body)
                .remove::<ambition_platformer2d::actor::RespawnGrace>();
        }
    }
}

/// The id one seat's respawn platform is keyed by.
///
/// Keyed by seat, not entity: the body may be rebuilt, and the platform belongs
/// to where that seat comes back.
fn respawn_platform_id(seat: usize) -> String {
    format!("{RESPAWN_PLATFORM_PREFIX}{seat}")
}

/// The one place this id family is spelled.
///
/// The builder and the reader must share it (D-ID-CONVENTION-DRIFT). If they
/// drift, `hold_the_respawn_platforms` keeps no platform and rebuilds every
/// block each tick. Only `the_respawn_platform_lives_exactly_as_long_as_the_grant`
/// catches that, indirectly. `scripts/measure_id_prefixes_spelled_twice.py`
/// reports the drift.
const RESPAWN_PLATFORM_PREFIX: &str = "respawn_platform_";

/// Is this the id of a respawn platform? The parse half of the pair.
fn is_respawn_platform_id(id: &str) -> bool {
    id.starts_with(RESPAWN_PLATFORM_PREFIX)
}

/// The platform a returning fighter materialises on: three body-widths across
/// and thin, so it reads as a ledge to step off rather than as stage.
const RESPAWN_PLATFORM_SIZE: Vec2 = Vec2::new(96.0, 12.0);

/// How far below the fighter's centre the platform's centre sits: half a
/// standing body plus half the platform, so its top is under the feet.
const RESPAWN_PLATFORM_DROP_PX: f32 = 24.0 + RESPAWN_PLATFORM_SIZE.y * 0.5;

/// The freeze a FULL punch is measured against, in seconds.
///
/// Measured: `hitlag_duration` scales with damage, and this is what a heavy
/// connect produces under this stage's feel. A lower value saturates on jabs.
const HUD_PUNCH_REFERENCE_HITLAG: f32 = 0.12;

/// How long a returning fighter cannot be hit, in seconds.
///
/// Long enough to fall in, read the stage and choose a landing; short enough
/// that spawn camping is not free. Attacking or leaving the platform ends it
/// early.
const RESPAWN_PROTECTION_SECONDS: f32 = 2.0;

/// D192: how long the stage waits before putting a knocked-out fighter back.
///
/// With zero, the KO cue played over a fighter already back, and the camera
/// jumped to a body that appeared far away. One second is the genre's pause.
///
/// D201: seconds, because the beat is the engine's `DeathInterlude`, counted
/// on `WorldTime` and rewound with everything else.
const RESPAWN_INTERVAL_SECONDS: f32 = 1.0;

/// Take an eliminated fighter out of play.
///
/// The engine spends the last stock and decides the match; the ruleset removes
/// the body. Despawn, not park: a parked body could still generate hit events.
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
/// The banner asks for 3.0s; this waits a little longer so players read it.
const RETURN_TO_SELECT_AFTER: f32 = 4.5;

/// Ensure the Smash gameplay route carries Smash-owned combat rules. The lobby
/// normally publishes them when a battle starts; this is a safety net for
/// direct or stale entry and does not rewrite a correct declaration.
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

/// What Smash's presentation override replaced, so leaving can put it back.
///
/// Restore, do not remove: `PortalPresentationPlugin` inits
/// `PortalCameraContinuitySelection` and `PortalViewConeConfig`, and
/// `sync_portal_view_cones` requires the config. A developer- or
/// Ambition-owned configuration must come back unchanged.
///
/// Each field is an `Option` because absence is a real prior: restoring `None`
/// removes the resource.
#[derive(bevy::prelude::Resource, Clone, Debug)]
struct SmashPresentationPrior {
    transit: Option<ambition_platformer2d::portal_presentation::PortalCameraContinuitySelection>,
    cone: Option<ambition_platformer2d::portal_presentation::PortalViewConeConfig>,
    /// The Limit rule is stage state too: a process-wide fill rule would run in
    /// every experience composed beside Smash.
    limit: Option<crate::limit::SmashLimitFill>,
}

/// Smash's presentation and meter policy, for as long as Smash is on the stage.
///
/// The ruleset owns these, not the binary, so the standalone demo and the
/// versus route draw the same portal cone.
///
/// The lifetime is the active route, not plugin install: `ambition_app`
/// installs this plugin beside Ambition, Sanic and Mary-O, and a build-time
/// insert would change their mana and portal presentation.
///
/// The saved prior's presence means "already declared". Do not infer it from
/// one of the three resources.
fn the_stage_declares_smashs_presentation_and_gives_it_back(
    mut commands: bevy::prelude::Commands,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    prior: Option<bevy::prelude::Res<SmashPresentationPrior>>,
    transit: Option<
        bevy::prelude::Res<
            ambition_platformer2d::portal_presentation::PortalCameraContinuitySelection,
        >,
    >,
    cone: Option<
        bevy::prelude::Res<ambition_platformer2d::portal_presentation::PortalViewConeConfig>,
    >,
    limit: Option<bevy::prelude::Res<crate::limit::SmashLimitFill>>,
) {
    use ambition_platformer2d::portal_presentation as portal_view;

    let on_stage = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_GAMEPLAY_ROUTE);
    let declared = prior.is_some();

    if on_stage && !declared {
        commands.insert_resource(SmashPresentationPrior {
            transit: transit.map(|r| *r),
            cone: cone.map(|r| r.clone()),
            limit: limit.map(|r| *r),
        });
        commands.insert_resource(crate::limit::SmashLimitFill(crate::limit::SMASH_LIMIT));
        // A viewer-dependent cone is undefined with no primary player, and
        // seamless camera transit is a single-camera effect.
        commands.insert_resource(portal_view::PortalCameraContinuitySelection {
            mode: portal_view::PortalCameraTransitMode::Pop,
        });
        commands.insert_resource(portal_view::PortalViewConeConfig {
            mode: portal_view::PortalViewConeMode::Static,
            ..Default::default()
        });
    } else if !on_stage && declared {
        let prior = prior.expect("checked").clone();
        match prior.transit {
            Some(value) => commands.insert_resource(value),
            None => commands.remove_resource::<portal_view::PortalCameraContinuitySelection>(),
        }
        match prior.cone {
            Some(value) => commands.insert_resource(value),
            None => commands.remove_resource::<portal_view::PortalViewConeConfig>(),
        }
        match prior.limit {
            Some(value) => commands.insert_resource(value),
            None => commands.remove_resource::<crate::limit::SmashLimitFill>(),
        }
        commands.remove_resource::<SmashPresentationPrior>();
    }
}

/// Smash's fighters are solid to each other (jostle).
///
/// Runs in the simulation, not `Update`: `BodyContact` is read by the
/// movement kernel, and `Update` does not replay under rollback.
///
/// The engine owns the constraint (one body's motion reduced by the bodies it
/// touches, `ambition_platformer2d::engine_core::movement::body_contact`); this
/// ruleset grants it to its cast (bodies with `FighterStocks`). Projectiles and
/// props do not get it.
///
/// A better home is `MatchBody`, applied in the flush that builds the bodies.
/// That is a wire change to a snapshotted type, so it is not done here.
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
    // `Without<BodyContact>` makes this idempotent: nothing is written once
    // the component is present.
    for fighter in &fighters {
        commands
            .entity(fighter)
            .try_insert(ambition_platformer2d::platformer::body::BodyContact::FIRM);
    }
}

/// Return to character select after a decided match has shown its winner card
/// for [`RETURN_TO_SELECT_AFTER`], or at once for a `NoContest`.
///
/// Leaving the stage cannot be retracted, so this arms only on a confirmed
/// frame. It reads `StocksMatchSettled` (rollback state, stamped with its
/// match) instead of the message, and waits for
/// `ConfirmedFrameBoundary::fully_confirmed`. The stamp also stops the previous
/// match's verdict from arming the next one.
fn return_to_the_select_screen_when_the_match_ends(
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    time: bevy::prelude::Res<bevy::prelude::Time>,
    mut shell: bevy::prelude::MessageWriter<ambition_platformer2d::game_shell::ShellCommand>,
    mut readouts: bevy::prelude::ResMut<ambition_platformer2d::presentation::HudReadouts>,
    // Whether this match is over, from the authority that rewinds.
    settled: Option<bevy::prelude::Res<ambition_platformer2d::versus_match::StocksMatchSettled>>,
    active: Option<bevy::prelude::Res<ambition_platformer2d::versus_match::ActiveMatch>>,
    // Absent means no rollback host, which confirms everything.
    boundary: Option<
        bevy::prelude::Res<ambition_platformer2d::engine_core::ConfirmedFrameBoundary>,
    >,
    // Which session owns the world now; see the leftover-match note below.
    scope: Option<bevy::prelude::Res<ambition_platformer2d::actor::ActiveSessionScope>>,
    mut countdown: bevy::prelude::Local<Option<f32>>,
) {
    let on_stage = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_GAMEPLAY_ROUTE);
    if !on_stage {
        // Left by another road (pause menu, host quit). The countdown belongs
        // to this visit.
        *countdown = None;
        readouts.clear_slot(SMASH_ANNOUNCE_HUD_SLOT);
        return;
    }
    // A retired session's `ActiveMatch` outlives it by at least a frame.
    // Reading it would apply the previous verdict to the new match (a second
    // match bounced back to select at once). `StocksMatchSettled::verdict`
    // cannot catch this, because both sides are the retired match. Only the
    // session scope knows which `ActiveMatch` is current.
    let live = active.as_deref().filter(|active| match scope.as_deref() {
        // No session lifecycle: nothing can be stale.
        None => true,
        Some(scope) => active.session() == scope.current(),
    });
    let ended = match (settled.as_deref(), live) {
        (Some(settled), Some(active)) => settled.settled(active),
        _ => false,
    };
    // An abandoned match goes straight to select, with no card (Jon's call).
    //
    // It does not wait for confirmation. That is safe only for `NoContest`:
    // it comes from `MatchAbandonRequest`, a latch outside the simulation that
    // does not rewind, so a resim reaches the same verdict. See
    // `MatchAbandonRequest`.
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
    // Read the latch, not `StocksMatchDecided`: the message can come from a
    // speculative frame, and a HUD readout cannot be retracted. Waiting for
    // confirmation before reading the message is not enough, because the
    // two-frame channel can drop it. State has no cursor.
    settled: Option<bevy::prelude::Res<ambition_platformer2d::versus_match::StocksMatchSettled>>,
    active: Option<bevy::prelude::Res<ambition_platformer2d::versus_match::ActiveMatch>>,
    // Absent means no rollback host, which confirms everything.
    boundary: Option<
        bevy::prelude::Res<ambition_platformer2d::engine_core::ConfirmedFrameBoundary>,
    >,
    // Whether a side is a person or a team comes from the prepared match; it
    // is the only record left once fighters are removed.
    prepared: Option<bevy::prelude::Res<ambition_platformer2d::versus_match::PreparedMatch>>,
    // Which match already has its card; see the rising-edge note.
    mut announced: bevy::prelude::Local<Option<ambition_platformer2d::versus_match::MatchInstance>>,
    // Use surviving fighters only to resolve display names for the winning side.
    fighters: bevy::prelude::Query<(
        &ambition_platformer2d::versus_match::MatchSeat,
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
    // Write on the rising edge only, not every tick the latch is true.
    let this_match = active.instance();
    if announced.as_ref() == Some(&this_match) {
        return;
    }
    if let Some(verdict) = settled.verdict(active) {
        *announced = Some(this_match);
        // A no contest gets no card (Jon's call): the player who picked
        // `Exit Match` already knows. The rising edge is consumed above anyway.
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
            // Without a prepared plan the side size is unknown; use the side
            // name.
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
            // A team won together, or nobody is left to name: use the side.
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
/// The roster is inserted before the route changes. Seating runs on the sim
/// schedule and reads `MatchParticipantRoster` once; if the route changed
/// first, the match would open with an empty cast and nothing would retry.
pub struct SmashSelectPlugin;

/// When the select screen reads its input, as something another system can
/// be ordered against.
///
/// A windowed host rebuilds `SeatMenuFrames` from its participants every frame
/// (clearing first). Anything that injects a press (a test, a replay, a remote
/// seat) must run between that producer and this set, or the press can drop.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SmashSelectSet;

impl bevy::prelude::Plugin for SmashSelectPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        // The screen is a route. `get_resource_or_insert_with`, because
        // harnesses compose these plugins without the shell.
        app.world_mut()
            .get_resource_or_insert_with(
                ambition_platformer2d::game_shell::ShellRouteCatalog::default,
            )
            .register(ambition_platformer2d::game_shell::ShellRouteSpec::new(
                SMASH_SELECT_ROUTE,
                SMASH_SELECT_EXPERIENCE,
            ));
        // The select screen's music, declared beside the route so any host
        // carries it without naming Smash's content.
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
        // The ruleset's own rollback state. Register through
        // `AmbitionRollbackApp`, not a `SchemaRollbackRegistrar`: the schema
        // registrar installs no probe, so `rollback_exit_oracle` cannot
        // localize the component.
        {
            use ambition_platformer2d::rollback::AmbitionRollbackApp;
            app.rollback_component_clone_probed::<crate::shark_ride::Departing>(
                "ambition_demo_smash",
                "smash.departing_mount",
                crate::shark_ride::departing_probe,
            );
            // The bomb's fuse and remembered speed outlive the tick that made
            // them; a restore without them gives a different explosion.
            app.rollback_component_clone_probed::<crate::bomb::LiveBomb>(
                "ambition_demo_smash",
                "smash.live_bomb",
                crate::bomb::live_bomb_probe,
            );
            // Move-opened apertures and their clock: a restore without the
            // clock closes the recovery route at a different moment.
            app.rollback_component_clone_probed::<crate::portal::MovePlacedPortal>(
                "ambition_demo_smash",
                "smash.move_placed_portal",
                crate::portal::move_placed_portal_probe,
            );
            // Which match each spawned object belongs to, so the sweep never
            // sees a restored object without an identity. Clone-snapshotted:
            // it is a stable identity copied at spawn.
            app.rollback_component_clone_probed::<ambition_platformer2d::versus_match::MatchScoped>(
                "ambition_demo_smash",
                "smash.match_scoped",
                crate::match_scope::match_scoped_probe,
            );
            // The mine's arming clock: a restore without it could answer a
            // press the confirmed timeline ignored.
            app.rollback_component_clone_probed::<crate::mine::PlacedMine>(
                "ambition_demo_smash",
                "smash.placed_mine",
                crate::mine::placed_mine_probe,
            );

            // The mark and its clock: without the clock the mark detonates on
            // different frames on two peers. The probe is the clock, because a
            // presence-only probe cannot see that.
            app.rollback_component_clone_probed::<crate::mark::BodyMark>(
                "ambition_demo_smash",
                "smash.body_mark",
                crate::mark::body_mark_probe,
            );
            // Delayed combat attribution can outlive the fighter body. The
            // stand-in never counts as a participant because it has no
            // `MatchSeat`. See `SeatCreditStandIn`.
            app.rollback_component_clone_probed::<crate::mark::SeatCreditStandIn>(
                "ambition_demo_smash",
                "smash.seat_credit_stand_in",
                crate::mark::seat_credit_stand_in_probe,
            );

            // The bolt in flight: position, heading, lifetime, and whether it
            // cleared its caster. The heading matters: peers can agree on the
            // lifetime and disagree on where the bolt goes.
            app.rollback_component_clone_probed::<crate::bolt::SteeredBolt>(
                "ambition_demo_smash",
                "smash.steered_bolt",
                crate::bolt::steered_bolt_probe,
            );

            // The plate's three clocks and remaining uses: a restore without
            // them could replay a launch the confirmed timeline already spent.
            app.rollback_component_clone_probed::<crate::spring::PlacedSpring>(
                "ambition_demo_smash",
                "smash.placed_spring",
                crate::spring::placed_spring_probe,
            );

            // A dilation's remaining seconds (the scale itself is canonical as
            // `actor.proper_time_scale`). Peers that disagree on it resimulate
            // different swings. `prior` travels with it so a restore puts the
            // body back on the right clock.
            app.rollback_component_clone_probed::<crate::dilation::TimeDilated>(
                "ambition_demo_smash",
                "smash.time_dilated",
                crate::dilation::time_dilated_probe,
            );

            // The homing dash's clock and committed direction: both decide
            // where a fighter is.
            app.rollback_component_clone_probed::<crate::homing::HomingDash>(
                "ambition_demo_smash",
                "smash.homing_dash",
                crate::homing::homing_dash_probe,
            );

            // The tether reel's clock and latched anchor. A peer that lost the
            // anchor would re-probe its own solids and could latch a different
            // ledge.
            app.rollback_component_clone_probed::<crate::tether::TetherReel>(
                "ambition_demo_smash",
                "smash.tether_reel",
                crate::tether::tether_reel_probe,
            );
        }
        // Init before the preparation source reads it: a missing
        // `Res<SmashStageChoice>` panics when a match prepares. The default is
        // the stage all recorded measurements used.
        app.init_resource::<SmashStageChoice>();
        app.init_resource::<SmashStockChoice>();
        app.init_resource::<select::SmashSelect>();
        // The pointer state lives outside `SmashSelect`: where a cursor points
        // is not part of what the screen decided.
        app.init_resource::<select_screen::cursor::SelectCursors>();
        app.init_resource::<select_screen::SelectPage>();
        app.init_resource::<select_screen::SelectInteractionPolicy>();
        app.init_resource::<select_screen::StartRequested>();
        app.init_resource::<select_screen::LeaveRequested>();
        // The roster is a composition fact, resolved at `Startup`, once every
        // provider has declared itself.
        app.init_resource::<select::SmashRoster>();
        app.add_systems(bevy::prelude::Startup, assemble_the_smash_roster);
        // Portrait sheet manifests, so a face is one frame. Without them,
        // multi-frame sheets (`alice`, `oiler`) draw as a strip.
        //
        // Guarded: Ambition's dialogue box installs the same plugin, and Bevy
        // panics on a duplicate. The registry is the same either way.
        if !app
            .is_plugin_added::<ambition_platformer2d::sprite_sheet::PortraitSheetRegistryPlugin>()
        {
            app.add_plugins(ambition_platformer2d::sprite_sheet::PortraitSheetRegistryPlugin);
        }
        // The screen declares its own input port. The windowed host fills
        // `SeatMenuFrames`; `init_resource` does not replace an existing one.
        // Headless apps and tests can then press buttons instead of setting
        // `SmashSelect` directly.
        app.init_resource::<ambition_platformer2d::input::SeatMenuFrames>();
        // The seats it offers. A host seats input participants from the match
        // roster, which this screen produces, so the screen must declare its
        // seats. `LocalSeatOffer` carries the couch policy with the count.
        app.init_resource::<ambition_platformer2d::input::LocalSeatOffer>();
        // One chain, in `InputSet::Consume`, ordered against:
        // 1. The producer: a windowed host rebuilds `SeatMenuFrames` every
        //    frame, so an unordered reader can miss presses.
        // 2. Itself: arrival resets the previous match's decision before the
        //    transition out reads it. The other order re-enters the stage on
        //    arrival.
        app.configure_sets(
            bevy::prelude::Update,
            bevy::prelude::IntoScheduleConfigs::in_set(
                SmashSelectSet,
                ambition_platformer2d::input::InputSet::Consume,
            ),
        );
        // The screen claims its seats' input in `ResolveContext`, ahead of
        // every router. A higher claim (the pause menu at
        // `context_priority::PAUSE`) outranks it. See `drive_the_select_screen`.
        app.add_systems(
            bevy::prelude::Update,
            bevy::prelude::IntoScheduleConfigs::in_set(
                declare_the_select_input_context,
                ambition_platformer2d::input::InputSet::ResolveContext,
            ),
        );
        // The screen's confirm cue: what confirming means here. It is also
        // the prompt's only evidence without a context resolver; with no cue,
        // `publish_frontend_context_prompt` answers `Empty` and the touch
        // overlay hides its controls.
        //
        // `init_resource` does not replace the host's map. Cues are keyed by
        // context, so this screen owns only its key.
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
                    // Four small projections instead of one wide mutable query.
                    // Their order does not matter; the fence keeps
                    // drive-before-draw without a B0001 exclusion matrix.
                    select_screen::sync_select_grid,
                    select_screen::sync_select_cards,
                    select_screen::sync_select_chrome,
                    select_screen::sync_select_tokens_and_cursors,
                    start_the_battle_when_asked,
                    // Safety net for entries that skip the lobby (dev bins,
                    // stage tests).
                    the_stage_always_plays_by_smash_rules,
                    the_stage_declares_smashs_presentation_and_gives_it_back,
                    // After the driver that sets the flag, in the same chain,
                    // so a press and its route change are at most a frame apart.
                    leave_the_select_screen_when_asked,
                    return_to_the_select_screen_when_the_match_ends,
                    // The pause menu row: its label, and what picking it does.
                    offer_to_exit_the_match,
                    abandon_the_match_when_the_shell_asks,
                )),
                SmashSelectSet,
            ),
        );
    }
}

/// Offer `Exit Match` while a match is running, and withdraw it otherwise. The
/// match ends as No Contest.
///
/// The shell draws the row but does not know what a match is. This states the
/// words; [`abandon_the_match_when_the_shell_asks`] states the meaning.
///
/// Retract the offer, do not only set it: a stale offer puts `Exit Match` on
/// the select screen's pause menu.
fn offer_to_exit_the_match(
    mut commands: bevy::prelude::Commands,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    active: Option<bevy::prelude::Res<ambition_platformer2d::versus_match::ActiveMatch>>,
    // Optional: a composition may reach this route before the stocks feature
    // installs anything; then the match is not settled.
    settled: Option<bevy::prelude::Res<ambition_platformer2d::versus_match::StocksMatchSettled>>,
    offered: Option<bevy::prelude::Res<ambition_platformer2d::game_shell::ShellAbandonOffer>>,
) {
    let on_stage = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_GAMEPLAY_ROUTE);
    // A decided match is still active (winner card up, countdown running), so
    // `ActiveMatch` alone is not enough; the abandon latch would ignore the
    // press. Use `StocksMatchSettled::settled`, the same authority the abandon
    // road reads, not a proxy such as the card or a menu state.
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
/// Everything after this exists already: `decide_stocks_match` settles the
/// match as a [`MatchVerdict::NoContest`], and
/// [`return_to_the_select_screen_when_the_match_ends`] returns to the lobby, as
/// for a knockout. Do not add a separate teardown path. For `NoContest`,
/// [`announce_the_winner`] writes no card and the return has no countdown.
///
/// Not gated on a seat: it is a match-level command, so it works in
/// CPU-vs-CPU too.
fn abandon_the_match_when_the_shell_asks(
    mut commands: bevy::prelude::Commands,
    mut asked: bevy::prelude::MessageReader<
        ambition_platformer2d::game_shell::ShellAbandonRequested,
    >,
    // Which match to stop. The ask is made outside the simulation, so it names
    // its match instead of riding a rewinding channel. See `MatchAbandonRequest`.
    active: Option<bevy::prelude::Res<ambition_platformer2d::versus_match::ActiveMatch>>,
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

/// Sudden death's stage half: put the survivors on the edge of death.
///
/// The engine's stocks loop knows stocks and the clock, not percent, so the
/// stage applies the damage.
///
/// Eliminated fighters are not revived. Only the tied sides fight: a side the
/// clock already put behind gets `FighterEliminated`, like an exhausted
/// fighter. [`take_eliminated_fighters_out_of_play`] then removes it, and
/// `last_side_standing` decides among the contenders.
fn open_the_sudden_death_round(
    mut commands: bevy::prelude::Commands,
    mut began: bevy::prelude::MessageReader<
        ambition_platformer2d::actors::features::stocks_match::SuddenDeathBegan,
    >,
    mut fighters: bevy::prelude::Query<
        (
            bevy::prelude::Entity,
            &ambition_platformer2d::versus_match::MatchSeat,
            Option<&ambition_platformer2d::combat::targeting::MatchTeam>,
            &mut ambition_platformer2d::characters::actor::BodyHealth,
            // The stocks: this round sets them to one.
            &mut ambition_platformer2d::combat::components::FighterStocks,
        ),
        bevy::prelude::Without<ambition_platformer2d::combat::stocks::FighterEliminated>,
    >,
) {
    for round in began.read() {
        for (body, seat, team, mut health, mut stocks) in &mut fighters {
            // The side, not the seat: team members stand or fall together, as
            // in the tiebreak.
            let side = ambition_platformer2d::combat::stocks::side_label(seat.0, team);
            if round.contenders.iter().any(|contender| *contender == side) {
                health.set_damage_taken(round.starting_damage);
                // One stock: the first KO decides. A tie can happen with
                // several stocks each; without this the loser would respawn and
                // the round would continue. Set `remaining`, not
                // `started_with`: the HUD reads the latter for the icons.
                stocks.remaining = 1;
            } else {
                commands
                    .entity(body)
                    .try_insert(ambition_platformer2d::combat::stocks::FighterEliminated);
                // Also remove `ActiveCombatant`, as `spend_fighter_stocks`
                // does. A marker alone leaves a body with attack state and a
                // place on the anti-clump board.
                commands
                    .entity(body)
                    .remove::<ambition_platformer2d::combat::components::ActiveCombatant>();
            }
        }
    }
}

/// Who can be picked in this composition: `select::SMASH_ROSTER` filtered to
/// the ids this host can seat.
fn assemble_the_smash_roster(
    // The seatable authority, not the catalog; see `SmashRoster::assemble`.
    // Optional: with no characters registered, the grid is empty.
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
/// `SmashSelect` is the lobby decision; cursor, page and request state are
/// interaction state. A rematch starts with neither.
///
/// "Once per arrival" is keyed on `ShellActivationId`, minted per activation.
/// Do not key it on the UI root's absence: the first visit's root outlives the
/// route change, so the reset would not run on the second visit, and start
/// would do nothing.
fn reset_select_frontend_on_arrival(
    mut commands: bevy::prelude::Commands,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    mut select: bevy::prelude::ResMut<select::SmashSelect>,
    roster: Option<bevy::prelude::Res<MatchParticipantRoster>>,
    mut cursors: bevy::prelude::ResMut<select_screen::cursor::SelectCursors>,
    mut page: bevy::prelude::ResMut<select_screen::SelectPage>,
    mut start: bevy::prelude::ResMut<select_screen::StartRequested>,
    // Which arrival this already ran for.
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

/// Spawn/despawn the select UI from route state. This owns presentation
/// lifetime only; seat policy and frontend-state reset live in the two systems
/// above.
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
/// Without this, the pause menu over the screen and the screen both read the
/// arrows (through `MenuControlFrame` and `SeatMenuFrames`). The screen names
/// an input context; the pause menu's higher-priority capturing claim wins.
/// Neither knows the other.
fn declare_the_select_input_context(
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    mut participants: bevy::prelude::Query<
        &mut ambition_platformer2d::input::participant::ParticipantContexts,
        bevy::prelude::With<ambition_platformer2d::input::InputParticipant>,
    >,
) {
    let on_select = on_the_select_route(&router);
    for mut contexts in &mut participants {
        // Touch the component only when the claim changes.
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
/// Used by the context claim, the cue, and the drive gate.
fn on_the_select_route(router: &ambition_platformer2d::game_shell::ShellRouter) -> bool {
    router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_SELECT_ROUTE)
}

/// Publish this screen's submit verb while it is up.
///
/// `sync`, not declare/retract, so leaving retracts. A stale cue would tell the
/// next screen's player to choose a fighter.
fn publish_the_select_ui_cue(
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    mut cues: bevy::prelude::ResMut<ambition_platformer2d::input::ActiveUiCues>,
) {
    cues.sync(
        ambition_platformer2d::input::UiCue {
            context: ambition_platformer2d::input::SELECT_CONTEXT,
            priority: ambition_platformer2d::input::participant::context_priority::SELECT,
            // The cursor takes a role, takes a fighter, or presses START.
            // "Choose" fits all three.
            submit_label: "Choose".to_owned(),
        },
        on_the_select_route(&router),
    );
}

/// Is this screen the one the presses belong to?
///
/// The pause menu can outrank this screen; see
/// `declare_the_select_input_context`.
///
/// It asks whether any seat still owns `SELECT_CONTEXT`, not only seat 0: one
/// cursor may be driven by four people. `None` (no resolver, as in a bare unit
/// fixture) counts as owned.
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
/// lifecycle, and the stage has no session until the route resolves.
///
/// It waits for START to be clicked, not for `ready()`: the genre has a ready
/// button, and a lobby that launches at once cannot be captured.
fn start_the_battle_when_asked(
    mut commands: bevy::prelude::Commands,
    select: bevy::prelude::Res<select::SmashSelect>,
    asked: bevy::prelude::Res<select_screen::StartRequested>,
    fighters: bevy::prelude::Res<select::SmashRoster>,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    roster: Option<bevy::prelude::Res<MatchParticipantRoster>>,
    // The seat policy decides whether source index zero is the keyboard or a
    // pad (as in `source_name_under`), so the roster and the slot label agree.
    assignment: bevy::prelude::Res<ambition_platformer2d::input::LocalSeatOffer>,
    // Characters that author their own moves do not get this stage's generic
    // kit. `Option`, like every other reader of the cast.
    prepared: Option<
        bevy::prelude::Res<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>,
    >,
    mut shell: bevy::prelude::MessageWriter<ambition_platformer2d::game_shell::ShellCommand>,
    // The lobby's stocks choice, passed to `roster_seeded` as a value (see
    // that parameter's note).
    stocks: bevy::prelude::Res<SmashStockChoice>,
) {
    if !asked.0 {
        return;
    }
    // Only from the select screen. `ready()` stays true during the match, so
    // without this the shell would re-enter the stage every frame.
    let on_select = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == SMASH_SELECT_ROUTE);
    if !on_select || roster.is_some() {
        return;
    }
    // The seed for this match's random squares (ADR 0023: no ambient RNG).
    // It is a digest of what both peers agree on: the seats, their occupants,
    // and each pick. Do not use the wall clock, a thread RNG, or
    // `ShellActivationId` (a host-local counter that differs between peers).
    //
    // Cost: two identical setups now draw the same fighter. Per-rematch
    // variation needs a nonce the peers agree on at setup, which needs a
    // handshake the project does not have yet.
    let seed = agreed_match_seed(&select);
    let declared_rules = smash_declared_combat_rules();

    let Some(decided) = select.roster_seeded(
        &fighters,
        seed,
        assignment.policy(),
        // Ids whose character authors its own move timelines. Only this side
        // can see the prepared cast.
        &prepared
            .as_deref()
            .map_or_else(Default::default, |registry| {
                registry
                    .iter()
                    .filter(|(_, definition)| definition.authored_moveset.is_some())
                    .map(|(id, _)| id.to_string())
                    .collect()
            }),
        // The value, not a `DeclaredCombatRules` read: this system inserts
        // that resource below, and `insert_resource` is deferred.
        Some(smash_seating_melee()),
        // What the lobby's stocks button decided.
        stocks.count(),
    ) else {
        return;
    };
    // The seat plan this match decided, published under this experience's
    // name. Devices are not participants (a CPU seat has no device), so the
    // session must not be sized from what is plugged in. It lands in the same
    // flush as the route request, so the session always sees it.
    commands.insert_resource(ambition_platformer2d::input::SessionSeatingSource::decided(
        SMASH_EXPERIENCE,
        // The whole channel plan, not a count: a CPU needs no channel, and
        // consumers must not guess which controller feeds each handle from the
        // lobby's sparse source numbers.
        decided.local_channel_plan(),
    ));
    commands.insert_resource(decided);
    commands.insert_resource(declared_rules);
    // The Smash pad layout, declared for this experience and released on the
    // way out; `insert_gamepad_bindings` is unchanged (A=Jump stays right for
    // Ambition). The layout permutes the fully assigned default pad, which is
    // the only way gamepad Special gets a button.
    commands.insert_resource(ambition_platformer2d::input::DeclaredBindingLayout::new(
        SMASH_EXPERIENCE,
        ambition_platformer2d::input::BindingLayout::Smash,
    ));
    // Name prompts by button. A Smash Attack slot hosts a dozen moves chosen
    // by direction and posture, so naming the move would change as the body
    // moves. Move naming stays the default elsewhere.
    commands.insert_resource(ambition_platformer2d::sim_view::PromptNaming::ByButton);
    shell.write(ambition_platformer2d::game_shell::ShellCommand::GoTo(
        ambition_platformer2d::game_shell::ShellRouteId::new(SMASH_GAMEPLAY_ROUTE),
    ));
}

/// Leave the lobby through the character-select screen's own Back affordance.
///
/// Esc/Start opens the universal system menu, whose `Quit to Title` row works
/// on frontend subroutes too. This handler is the select screen's Back / held-B
/// road. Both emit the host-relative `QuitToHome`; do not `GoTo` a title route
/// this demo does not own.
///
/// Nothing needs unwinding by hand. Each claim is keyed on the route:
/// `maintain_smash_local_seat_offer` releases its seat claim,
/// `present_select_screen_ui` despawns the UI,
/// `declare_the_select_input_context` retracts `SELECT_CONTEXT`,
/// `publish_the_select_ui_cue` retracts the cue, and the experience scope in
/// [`SmashExperiencePlugin`] resets `SmashSelect`, `StartRequested`,
/// [`select_screen::LeaveRequested`] and the cursor, and releases the
/// `SessionSeatingSource` hold. Only `start_the_battle_when_asked` writes a
/// `MatchParticipantRoster`, so a half-joined lobby leaves no match state.
fn leave_the_select_screen_when_asked(
    mut asked: bevy::prelude::ResMut<select_screen::LeaveRequested>,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    // Optional: a bare unit fixture has no host, and `exit_leads_somewhere`
    // reads that as "no way out".
    host: Option<bevy::prelude::Res<ambition_platformer2d::game_shell::ShellHostConfiguration>>,
    mut shell: bevy::prelude::MessageWriter<ambition_platformer2d::game_shell::ShellCommand>,
) {
    if !asked.0 {
        return;
    }
    // Spend the request before the refusals below, so a stale "leave" cannot
    // fire on a later route.
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
/// It assembles the roster, stage and ruleset into something bootable.
pub struct SmashExperiencePlugin;

impl bevy::prelude::Plugin for SmashExperiencePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        install_smash_content(app);
        // Before the authoring below: it advertises the select screen as the
        // entry and refuses an unregistered route.
        app.add_plugins(SmashSelectPlugin);
        ambition_platformer2d::provider::PlatformerExperienceAuthoring::new(
            SMASH_EXPERIENCE,
            SMASH_GAMEPLAY_ROUTE,
            "Smash",
            "Stocks, a platform, and nothing underneath it",
            "Prepare Smash",
            // No `.with_procedural_sfx()`: the stage is silent and the
            // fighters bring their own cues.
            ambition_platformer2d::provider::AuthoredCatalogFragments::new(
                SMASH_CHARACTER_ID,
                SMASH_EXPERIENCE,
            ),
        )
        // The launcher row enters the select screen, not the stage (see
        // `.entered_at`).
        //
        // The stage's own HUD, so it does not inherit Ambition's health, mana
        // and money readouts. Four slots because the stage seats four; the
        // publisher clears unused ones.
        .with_hud({
            let mut hud = ambition_platformer2d::presentation::HudDeclaration::new();
            for (seat, slot) in FIGHTER_HUD_SLOTS.iter().enumerate() {
                hud = hud.slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(*slot)
                        .with_region(ambition_platformer2d::presentation::SurroundRegion::Bottom)
                        .with_font_size(22.0)
                        .with_min_px(ambition_platformer2d::engine_core::Vec2::new(220.0, 30.0))
                        // Coloured by seat parity, so partners read as partners.
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

        // What this experience owns, and what leaves with it.
        //
        // `covering` the select screen: the lobby publishes the roster for the
        // match, so a scope naming only the gameplay id would delete it.
        {
            use ambition_platformer2d::game_shell::ShellExperienceScopeAppExt;
            app.experience_owns(SMASH_EXPERIENCE)
                .covering(SMASH_SELECT_EXPERIENCE)
                // By owner: another game may stage its own cast into this
                // resource.
                .releasing_owned::<MatchParticipantRoster>(|roster, owner| {
                    roster.is_published_by(owner.as_str())
                })
                // A match that ended with its route. Left standing, it blocks
                // the next game's seating. The session id identifies which
                // activation; this witness identifies which game.
                .releasing_witnessed::<
                    ambition_platformer2d::versus_match::ActiveMatch,
                    ambition_platformer2d::versus_match::PreparedMatch,
                >(|plan, owner| plan.is_published_by(owner.as_str()))
                // After the activation above, which reads it as its witness:
                // releases run in declaration order.
                .releasing_owned::<
                    ambition_platformer2d::versus_match::PreparedMatch,
                >(|plan, owner| plan.is_published_by(owner.as_str()))
                // The rules leave with the match. Removing the declaration is
                // the exit (AE6): the projection folds it over the baseline each
                // tick, so there is nothing to restore. Otherwise this DI budget
                // would follow the player into Ambition's PvE. Owned, not
                // `resetting`: readers take `Option<Res<_>>`, and ownership stops
                // two stages deleting each other's rules.
                .releasing_owned::<
                    ambition_platformer2d::combat::rules::DeclaredCombatRules,
                >(|rules, owner| rules.is_declared_by(owner.as_str()))
                // The pad layout goes back too: it is a layer inside
                // `BindingRecipe::build`, so the next rebuild returns every seat
                // to the base preset.
                .releasing_owned::<
                    ambition_platformer2d::input::DeclaredBindingLayout,
                >(|layout, owner| layout.is_declared_by(owner.as_str()))
                // Reset in place: systems take them as `ResMut`, but they must
                // not keep the previous match's state.
                .resetting::<select::SmashSelect>()
                .resetting::<select_screen::StartRequested>()
                // A "leave" that outlived the lobby would make the next
                // experience quit on its first frame.
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

/// How steep the victim-percent curve is, as a multiplier on the percent term
/// alone. `1.0` is the original law.
///
/// One global number instead of editing every fighter: 38 of 40 authored
/// knockback volumes already state `knockback_growth` at a median of 0.0200 of
/// base, matching [`SMASH_KNOCKBACK_GROWTH`].
///
/// `1.25` is the smallest swept value that ends the stock. Guards in
/// `smash_in_the_host::ring_out`:
///
/// | scale | 700% stale jab | lateral | 0% fresh |
/// |-------|----------------|---------|----------|
/// | 1.00  | ALIVE          | 246.4px | poke     |
/// | **1.25** | **KO**      | 523.4px | **poke** |
/// | 1.50  | KO             | 627.7px | poke     |
/// | 1.75+ | KO             | 722px+  | poke     |
///
/// `1.50` is a ceiling the table cannot show: two CPU floors fail at `1.50`
/// and pass at `1.25`:
/// `smash_cpus_damage_each_other::two_cpus_in_the_shipped_composition_damage_each_other`
/// and `the_repertoire_gets_used::every_authored_route_gets_pressed`. A steeper
/// curve separates the CPUs, so they exchange less. Re-run both floors before
/// raising this (see `docs/planning/queue.md`).
///
/// Measurement traps: this jab has `launch_dir: None`, so it kills through the
/// ceiling, not a side; measure the resolved launch, not the trajectory
/// maximum; pin the attacker's rage (an unpinned CPU at 62% multiplies every
/// cell by 1.248). A launch past the tumble threshold is not a ring-out, so
/// calibrate on the stage's knockout verdict.
///
/// The companion is `DeclaredCombatRules::stale_knockback_influence`: that
/// makes a worn move still convert; this makes percent itself convert.
pub const SMASH_VICTIM_PERCENT_KNOCKBACK_SCALE: f32 = 1.25;

/// The base knockback at which the kill curve is exactly `1.0` — a JAB.
///
/// Measured: jab base knockback over the seatable cast is n=21, min 40, median
/// 48, max 60. Pokes at or below the pivot keep their percent curve.
pub const SMASH_GROWTH_BASE_PIVOT: f32 = 48.0;

/// How sharply the kill curve climbs above a jab.
///
/// Calibrated against a measured KO threshold: at `0.25` a base-160 forward
/// smash gets `(160/48)^0.25` = 1.351x its authored growth, which brings a 145%
/// centre knockout into the 80-120% band.
pub const SMASH_GROWTH_BASE_EXPONENT: f32 = 0.25;

/// The most the kill curve may steepen anything.
///
/// `1.40` is the factor at the largest smash base (185). Past that a move is a
/// finisher, so huge-base outliers such as `bivalence` (367.2) do not get the
/// biggest multiplier. See `GrowthBaseCurve::ceiling`.
pub const SMASH_GROWTH_BASE_CEILING: f32 = 1.40;

/// Stable ids the shell routes and lists this demo by.
pub const SMASH_EXPERIENCE: &str = "smash";
pub const SMASH_GAMEPLAY_ROUTE: &str = "smash_gameplay";
/// Where the demo starts: the select screen, not the stage, because up to four
/// players choose who they are.
///
/// It is the demo app's home route (a match returns to it) and the entry route
/// this experience advertises to launchers.
pub const SMASH_SELECT_ROUTE: &str = "smash_select";
/// The select screen is its own shell experience.
///
/// Not `smash`: an activation with the gameplay experience id starts a gameplay
/// session, and this screen has no prepared world, so the shell would panic
/// ("requires an exact prepared-session publication").
pub const SMASH_SELECT_EXPERIENCE: &str = "smash.select";
/// The fighter a lone visitor wears. A match seats its own cast from the
/// roster.
pub const SMASH_CHARACTER_ID: &str = "smash_duelist_a";
/// The opponent.
pub const SMASH_OPPONENT_ID: &str = "smash_duelist_b";

/// The logician.
pub const SMASH_GEORGE_BOOUL: &str = "smash_george_booul";

// The one fighter this demo adds to the crossover. It wears a shipped sheet
// that no other catalog claims. The rest of the grid is Ambition's cast and the
// other demos' protagonists, named by id in `select::SMASH_ROSTER`. The two
// robot rows below are stand-ins for the lineage the content catalog owns; see
// `select::STAND_INS`.

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
/// The crossover cast is mostly Ambition's own, so the stocks loop is proven on
/// shipped content. This also declares the audio fragment: preparation refuses
/// an experience whose provider registered none. The stage declares music and
/// no SFX; the fighters bring their own cues.
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
    // Register the characters, not only their catalog rows:
    // `declare_registered_characters` reads the prepared registry, so a
    // catalog-only character draws the placeholder.
    {
        use ambition_platformer2d::actors::character_runtime::CharacterDefinitionAppExt;
        use ambition_platformer2d::character::CharacterDefinition;
        // Every id this demo can seat. A missing id gives a fighter that never
        // seats.
        //
        // The fourth column is a weight, used only by the two stand-ins: v3 is
        // the middleweight the stage is tuned against, v2 the lighter build.
        // `None` means the character states its own (George; see
        // `smash_pack::fighter_knockback_weight`).
        for (id, name, sheet, stand_in_weight) in [
            (SMASH_CHARACTER_ID, "Robot v3", "player_robot_v3", Some(1.0)),
            (SMASH_OPPONENT_ID, "Robot v2", "player_robot_v2", Some(0.85)),
            (SMASH_GEORGE_BOOUL, "George Booul", "george_booul", None),
        ] {
            let mut definition =
                CharacterDefinition::new(id, name, SMASH_EXPERIENCE).with_sheet(sheet);
            // The character's own facet outranks the stand-in column.
            definition.vitals.knockback_weight =
                crate::smash_pack::fighter_knockback_weight(id).or(stand_in_weight);
            // Percent reference and fighter body are match rules, declared by
            // `apply_smash_match_rules` and applied at seating (see
            // `MatchParticipantRoster::rules`). These characters author only
            // what they are.
            //
            // `DEFAULT_TUNING` is stated on purpose: without it George plays
            // floaty and sluggish (the repertoire probes catch it). Most of the
            // grid still plays on the actor baseline; which base a platform
            // fighter uses is an open product call.
            definition.movement_tuning = Some(ambition_platformer2d::engine_core::DEFAULT_TUNING);
            // What this fighter's body can do, authored on the character. The
            // engine's shield, dodge roll and ledge system need these bits.
            // `fly`, `blink` and `dash` are absent; keep this in step with
            // [`SMASH_FIGHTER_KIT`], or the stage's ceiling trims it.
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
                    // The platform-fighter verbs.
                    shield: true,
                    dodge: true,
                    ledge_grab: true,
                    ..ambition_platformer2d::engine_core::AbilitySet::NONE
                });
            // The repertoire, on the character: preparation uses an authored
            // moveset as-is, so the seat needs no generic floor. George is the
            // fighter this demo owns and authors.
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
            // No SFX registry: the fighters bring their own cues.
            None,
        )
        .expect("the smash audio fragment is valid"),
    );
}

/// Which stage the next match is played on.
///
/// A resource because the preparation source (`PlatformerExperienceAuthoring::
/// install`) may read the provider's own resources; no engine change needed.
///
/// Defaults to the flat [`smash_stage`].
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SmashStageChoice {
    /// The original single-surface stage. Older recorded spacing, recovery and
    /// edgeguard numbers were measured here.
    #[default]
    Flat,
    /// [`smash_platform_stage`] — the same floor with three drop-through tiers.
    Platforms,
    /// [`smash_narrow_stage`] — two thirds the ground, the same blast envelope.
    Narrow,
}

/// How many stocks this match gives each fighter.
///
/// Feeds `MatchRules::stocks`, which the engine pairs with
/// `DeathPolicy::Unbounded`. Defaults to [`STARTING_STOCKS`].
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SmashStockChoice {
    /// One stock: no room to learn the matchup inside the match.
    One,
    /// [`STARTING_STOCKS`]: the count every ladder number was measured at.
    #[default]
    Three,
    /// Five: an early mistake is recoverable.
    Five,
}

impl SmashStockChoice {
    /// The count this choice gives each fighter.
    pub fn count(self) -> u32 {
        match self {
            SmashStockChoice::One => 1,
            SmashStockChoice::Three => STARTING_STOCKS,
            SmashStockChoice::Five => 5,
        }
    }

    /// What the stocks button shows: derived from [`Self::count`], so the
    /// button cannot disagree with the count it sets.
    pub fn label(self) -> String {
        self.count().to_string()
    }

    /// Every count, in cycle order.
    pub const ALL: [Self; 3] = [Self::One, Self::Three, Self::Five];

    /// The next count in the cycle, for a single button that walks them.
    ///
    /// Derived from [`Self::ALL`], so the cycle order has one authority.
    pub fn next(self) -> Self {
        let here = Self::ALL
            .iter()
            .position(|count| *count == self)
            .expect("every variant is in ALL — asserted by `all_lists_every_variant`");
        Self::ALL[(here + 1) % Self::ALL.len()]
    }
}

impl SmashStageChoice {
    /// Every stage, in cycle order.
    ///
    /// Consumers (the select button, `ladder_rig --stage`) resolve through this
    /// list, so a new variant reaches all of them.
    pub const ALL: [Self; 3] = [Self::Flat, Self::Platforms, Self::Narrow];

    /// The room this choice starts in.
    pub fn room_id(self) -> &'static str {
        match self {
            SmashStageChoice::Flat => SMASH_STAGE_ROOM_ID,
            SmashStageChoice::Platforms => SMASH_PLATFORM_STAGE_ROOM_ID,
            SmashStageChoice::Narrow => SMASH_NARROW_STAGE_ROOM_ID,
        }
    }

    /// What a stage button would show.
    pub fn label(self) -> &'static str {
        match self {
            SmashStageChoice::Flat => "Flat",
            SmashStageChoice::Platforms => "Platforms",
            SmashStageChoice::Narrow => "Narrow",
        }
    }

    /// The next stage in the cycle, for a single button that walks them.
    ///
    /// Derived from [`Self::ALL`], so the cycle order has one authority.
    pub fn next(self) -> Self {
        let here = Self::ALL
            .iter()
            .position(|stage| *stage == self)
            .expect("every variant is in ALL — asserted by `all_lists_every_variant`");
        Self::ALL[(here + 1) % Self::ALL.len()]
    }
}

/// The stage, as the shared preparation lifecycle wants it.
///
/// All stages are in the set; the choice picks the starting one.
/// `RoomSet::from_parts_or_panic` takes a `Vec<RoomSpec>` so a set can hold
/// rooms it does not start in. The geometry passed along is the starting
/// room's.
fn smash_prepared_session_world(
    choice: bevy::prelude::Res<SmashStageChoice>,
) -> ambition_platformer2d::runtime::PreparedPlatformerSource {
    use ambition_platformer2d::runtime::demo_fixture::{RoomSet, StartingCharacter};

    let choice = *choice;
    let rooms = vec![smash_stage(), smash_platform_stage(), smash_narrow_stage()];
    let started = rooms
        .iter()
        .find(|room| room.id == choice.room_id())
        .expect("every stage room is in the set the line above built");
    let geometry = ae::RoomGeometry(started.world.clone());
    // The match realizes its own cast; this id is only the catalog default
    // that worn fighters fall back to.
    ambition_platformer2d::runtime::PreparedPlatformerSource::for_match(
        SMASH_EXPERIENCE,
        RoomSet::from_parts_or_panic(choice.room_id(), rooms.clone(), Vec::new()),
        geometry,
        StartingCharacter::new(SMASH_CHARACTER_ID),
    )
}

#[cfg(test)]
mod pause_arbitration_tests;
#[cfg(test)]
mod tests;

/// The peer-agreed match seed (ID-PEER).
///
/// A digest of what two peers agree a match is: the seats, their occupants,
/// and each pick. Every mixed field must be peer-agreed on its own;
/// `SmashSelect` also carries host-local facts, such as the local device index.
///
/// FNV-1a over explicit tags, not `Hash`, so the value does not depend on the
/// standard library's hasher.
///
/// Cost: two identical setups draw the same fighter. Per-rematch variation
/// needs a nonce agreed at setup, which needs a handshake. Do not add a
/// host-local token.
pub fn agreed_match_seed(select: &select::SmashSelect) -> u64 {
    let mut digest: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |n: u64| {
        digest ^= n;
        digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
    };
    mix(select.participating() as u64);
    for (index, card) in select.slots() {
        mix(index as u64);
        mix(match card.occupant {
            select::SlotOccupant::Absent => 1,
            select::SlotOccupant::Cpu => 2,
            // The category, not the device index: `device` is local, so the
            // same human could be pad 0 here and pad 2 on the other peer.
            select::SlotOccupant::Controller { device: _ } => 3,
        });
        mix(match card.pick {
            None => 11,
            Some(select::SlotPick::Random) => 12,
            Some(select::SlotPick::Fighter(i)) => 13 ^ ((i as u64) << 8),
        });
    }
    digest
}
