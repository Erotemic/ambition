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
#[cfg(test)]
mod george_booul_moveset;
pub mod homing;
pub mod limit;
pub mod mark;
pub mod mine;
#[cfg(test)]
mod module_road;
pub mod motion;
#[cfg(test)]
mod moveset;
#[cfg(test)]
mod riposte_tests;
#[cfg(test)]
mod spring_tests;
pub mod portal;
pub mod select;
pub mod select_screen;
pub mod shark_ride;
pub mod sing;
pub mod smash_pack;
pub mod rules;
pub mod stages;
pub mod tether;
pub mod text;

/// The game-MODE tag this demo's rules gate on, so they sleep everywhere else.
pub const SMASH_MODE: &str = "smash";

/// The published controller policy a CPU seat asks for: `smash::duelist`
/// (cite-ok: an authored provider::name key, not a Rust path), resolved in
/// this stage's own provider.
pub const SMASH_DUELIST_BRAIN: &str = "duelist";

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
        // No seat states a body: each fighter plays on its own `smash_fighter`
        // facet's body over Smash's default fighter, else its catalog feel,
        // else the default fighter (`MatchRules::seat_body`).
        .collect();
    apply_smash_match_rules(&mut roster, starting_stocks());
    roster.published_by(SMASH_EXPERIENCE)
}

/// The Smash ruleset, in one place.
pub fn apply_smash_match_rules(roster: &mut MatchParticipantRoster, stocks: u32) {
    // The pack's rules (`assets/data/match_rules.ron`): the countdown, the
    // clock, the respawn beat, the percent pool, the floor and ceiling, the
    // fighter body and fighter contact. This match grants no pilot licence:
    // which shark a body may board is answered on the mount (see
    // `ambition_mount::MountReservedFor`).
    smash_match_rules().apply_to(&mut roster.rules);
    // A parameter, not a resource read here: both roads (`smash_roster` and
    // `SmashSelect::roster_seeded`) must state the count.
    roster.rules.stocks = Some(stocks);
    // Percent is not health, so a fighter's hits deal the damage it states in
    // the platform-fighter scale. A move it does not name keeps its moveset
    // damage, which is right for a fighter whose home game is this one.
    roster.rules.move_damage =
        Some(ambition_platformer2d::characters::smash_fighter::FIGHTER_DAMAGE);
    // The Limit is the match's resource: every seat is built holding it, empty.
    roster.rules.resources = vec![crate::limit::smash_limit().declaration()];
    // No items for now (Jon's call). The machinery (`MatchItemSpawns`, the
    // spawner, the weighted table) is built and tested; only this declaration
    // is absent. The previous table: a drop every 8s at three points above the
    // platform, weighted bomb 4 / gravity_grenade 2 / gun_sword 1. Do not add
    // `UseSystem` items (meteor gauntlet, mark/recall); those are abilities.
    roster.rules.item_spawns = None;
}

/// The rules of Smash's matches, as its pack states them
/// (`assets/data/match_rules.ron`). [`apply_smash_match_rules`] writes them on
/// every roster; the accessors below read one rule each.
pub fn smash_match_rules() -> ambition_platformer2d::versus_match::match_rules::AuthoredMatchRules {
    crate::smash_pack::match_rules()
}

/// Stocks each fighter starts with, unless the select screen chooses.
pub fn starting_stocks() -> u32 {
    smash_match_rules().stocks.expect("Smash's match rules state a stock count")
}

/// The match clock, in ticks. Public so that measurement tools (for example
/// `ladder_rig`) use the real match length: a shorter one ends every bout on
/// the damage tiebreak.
pub fn smash_time_limit_ticks() -> u32 {
    smash_match_rules().time_limit_ticks
}

/// What 100% means: the denominator of `damage_percent()`.
pub fn smash_percent_reference() -> i32 {
    smash_match_rules().health_pool.expect("Smash's match rules state a percent pool")
}

/// The fighter body the match writes over each fighter's own movement.
pub fn smash_fighter_body() -> ambition_platformer2d::engine_core::MatchBody {
    smash_match_rules().body.expect("Smash's match rules state a fighter body")
}

/// The floor: the verbs every fighter has here.
pub fn smash_fighter_kit() -> ambition_platformer2d::engine_core::AbilitySet {
    smash_match_rules().abilities.expect("Smash's match rules state a floor and ceiling").granted
}

/// The ceiling: the floor plus the verbs a fighter may bring from home. The
/// difference is character identity (Robot v3 keeps its pogo because it
/// authors pogo). `MatchAbilities::levelled` treats the two as floor and
/// ceiling.
pub fn smash_fighter_ceiling() -> ambition_platformer2d::engine_core::AbilitySet {
    smash_match_rules().abilities.expect("Smash's match rules state a floor and ceiling").permitted
}

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

/// Where a fighter comes back; each seat has its own point.
///
/// Seats alternate outward from the centre (0 left, 1 right, 2 further left,
/// 3 further right), so the layout is symmetric at any roster size.
pub fn respawn_placement(stage_centre: Vec2, seat: usize) -> Vec2 {
    // 0,1 → half a spacing out; 2,3 → one and a half; and so on.
    let rank = (seat / 2) as f32 + 0.5;
    let side = if seat % 2 == 0 { -1.0 } else { 1.0 };
    Vec2::new(
        stage_centre.x + side * rank * crate::stages::smash_stages().respawn.seat_spacing,
        // Toward the sky: this demo is screen-down. A gravity-flipped stocks
        // stage is a question for the engine.
        stage_centre.y - crate::stages::smash_stages().respawn.rise,
    )
}

// The room ids of three rows of `smash_stages.ron`. The tools and the tests
// name a stage by them; the select screen and the match read `offered`.

/// Stable room id for the stage.
pub const SMASH_STAGE_ROOM_ID: &str = "smash_stage";

/// The room id of the platformed stage — see [`smash_platform_stage`].
pub const SMASH_PLATFORM_STAGE_ROOM_ID: &str = "smash_platform_stage";

/// The room id of the narrow stage — see [`smash_narrow_stage`].
pub const SMASH_NARROW_STAGE_ROOM_ID: &str = "smash_narrow_stage";

/// The stage `id`, built from the pack's `data/smash_stages.ron`.
fn smash_stage_room(id: &str) -> RoomSpec {
    stage_room(crate::stages::smash_stages(), id)
}

/// The stage `id` as `stages` states it.
///
/// The file states the room every stage shares (size, landing point, blast
/// envelope) and each stage's blocks. Mode and nameplate policy are what make
/// a room a Smash stage, so they are set here, once for all stages.
pub fn stage_room(stages: &crate::stages::SmashStages, id: &str) -> RoomSpec {
    let stage = stages
        .stages
        .get(id)
        .unwrap_or_else(|| panic!("the pack's admission refuses a `smash_stages` file with no `{id}`"));
    let blocks = stage.blocks.iter().map(ae::AuthoredBlock::block).collect();
    let mut world = ae::World::new(
        stage.name.clone(),
        Vec2::new(stages.size.0, stages.size.1),
        Vec2::new(stages.spawn.0, stages.spawn.1),
        blocks,
    );
    world.edges.fall = stages.blast.fall;
    // Set the sides explicitly. The default margin is sized for falling
    // through the floor and is much too generous for a side launch.
    world.edges.side = Some(stages.blast.side);
    world.edges.rise = Some(stages.blast.rise);

    let mut room = RoomSpec::new(id, world);
    room.metadata.mode = Some(SMASH_MODE.to_string());
    // Label every fighter the same way. The presentation default hides the
    // plate on a driven body, which is right for exploration but singles out
    // the human here. The stage decides; `Some(false)` is the other uniform
    // choice.
    room.metadata.nameplate_policy.label_driven_bodies = Some(true);
    room
}

/// The stage: a platform surrounded by nothing (`smash_stages.ron`).
pub fn smash_stage() -> RoomSpec {
    smash_stage_room(SMASH_STAGE_ROOM_ID)
}

/// Less ground, same blast envelope: an edgeguard-and-recovery stage
/// (`smash_stages.ron`).
pub fn smash_narrow_stage() -> RoomSpec {
    smash_stage_room(SMASH_NARROW_STAGE_ROOM_ID)
}

/// The main surface plus three drop-through tiers (`smash_stages.ron`).
/// Reachable through [`SmashStageChoice`] and the select screen's stage
/// button.
pub fn smash_platform_stage() -> RoomSpec {
    smash_stage_room(SMASH_PLATFORM_STAGE_ROOM_ID)
}

/// The top centre of [`smash_stage`]'s main platform. The tools and tests
/// that measure on the flat stage read it; a match reads [`StageCentre`].
pub fn stage_centre() -> Vec2 {
    stage_centre_of(&smash_stage().world)
}

/// The top centre of a stage's main platform (its first block): a respawn
/// comes back above it.
pub fn stage_centre_of(world: &ae::World) -> Vec2 {
    let platform = world.blocks[0].aabb;
    Vec2::new((platform.min.x + platform.max.x) / 2.0, platform.min.y)
}

/// The centre of the stage a body is on: [`stage_centre_of`] its own live
/// room. Each stage row puts its platform where it likes, so a match does not
/// read the flat stage's centre.
#[derive(bevy::ecs::system::SystemParam)]
pub struct StageCentre<'w, 's> {
    rooms: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<'w, 's, ae::RoomGeometry>,
}

impl StageCentre<'_, '_> {
    /// The centre of the stage `body` is on. Panics when the body is in no
    /// live room: a fighter on no stage has no place to come back to.
    pub fn of(&self, body: bevy::prelude::Entity) -> Vec2 {
        let geometry = self
            .rooms
            .of(body)
            .unwrap_or_else(|| panic!("body {body:?} is in no live room, so it has no stage centre"));
        stage_centre_of(&geometry.0)
    }
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
        MatchVerdict::Winner(side) => text::smash_text().fill("banner.winner", &[("name", &winner_name.unwrap_or(side))]),
        // A draw is reachable and cheaply: two fighters on their last stock,
        // knocked off together.
        MatchVerdict::Draw => text::line("banner.draw"),
        // Say so: "Draw" would tell players the fighters settled something.
        MatchVerdict::NoContest => text::line("banner.no_contest"),
    }
}

/// The two answers the engine refuses to guess, wired to the messages it
/// writes.
///
/// `ambition_platformer2d::combat::stocks` spends the stock, clears the meter
/// and marks the elimination. Placing a body needs a stage and announcing a
/// winner needs a scoreboard, so this plugin supplies both.
///
/// The rules sleep outside the smash stage, because a host runs this demo
/// alongside its own rooms.
pub struct SmashRulesPlugin;

impl bevy::prelude::Plugin for SmashRulesPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::IntoScheduleConfigs;

        // The plugin owns its channels. A full app registers these through the
        // engine plugins; a rules-only harness may not, and `add_message` is
        // idempotent.
        app.add_message::<ambition_platformer2d::actor::FighterStockSpent>();
        app.add_message::<ambition_platformer2d::actor::FighterRespawnDue>();
        app.add_message::<ambition_platformer2d::actor::StocksMatchDecided>();
        // The combat rules and the prompt vocabulary govern Smash's rooms, and
        // no other room. A room that leaves the mode leaves the rules, so
        // nothing publishes them on entry or gives them back on exit.
        {
            use ambition_platformer2d::combat::scoped_rules::DeclareRulesExt as _;
            // The combat rules and the heavy-hit line are the pack's
            // (`assets/data/room_rules.ron`).
            ambition_platformer2d::combat::room_rules::declare_room_rules(
                app,
                ambition_platformer2d::combat::scoped_rules::RulesScope::Mode(SMASH_MODE),
                smash_room_rules(),
            );
            app.add_plugins(ambition_platformer2d::content::live_reload::EmbeddedPackReloadPlugin::room_rules(
                &crate::smash_pack::PACK,
                ambition_platformer2d::combat::scoped_rules::RulesScope::Mode(SMASH_MODE),
            ));
            // What "CPU level N" means in Smash's rooms is Smash's (Q88). In
            // the shipped game this narrower scope wins over Ambition's
            // every-room declaration of the same file.
            app.declare_rules(
                ambition_platformer2d::combat::scoped_rules::RulesScope::Mode(SMASH_MODE),
                crate::smash_pack::fighter_ladder(),
            );
            app.add_plugins(ambition_platformer2d::content::live_reload::EmbeddedPackReloadPlugin::fighter_ladder(
                &crate::smash_pack::PACK,
                ambition_platformer2d::combat::scoped_rules::RulesScope::Mode(SMASH_MODE),
            ));
            // No named boss encounters and no dialogue on a Smash stage: the
            // engine's work for those sleeps there.
            app.declare_rules(
                ambition_platformer2d::combat::scoped_rules::RulesScope::Mode(SMASH_MODE),
                ambition_platformer2d::platformer::capability::UsedCapabilities::ALL
                    .without(ambition_platformer2d::platformer::capability::Capability::BossEncounters)
                    .without(ambition_platformer2d::platformer::capability::Capability::Dialogue),
            );
            // Name prompts by button. A Smash Attack slot hosts a dozen moves
            // chosen by direction and posture, so naming the move would change
            // as the body moves. Rooms of other games keep naming the move.
            app.declare_rules(
                ambition_platformer2d::combat::scoped_rules::RulesScope::Mode(SMASH_MODE),
                ambition_platformer2d::sim_view::PromptNaming::ByButton,
            );
            // Only the seats hold a Limit slot, so the fill reaches no other body.
            app.declare_rules(
                ambition_platformer2d::combat::scoped_rules::RulesScope::Mode(SMASH_MODE),
                crate::limit::SmashLimitFill(crate::limit::smash_limit()),
            );
            // A viewer-dependent cone is undefined with no primary player, and
            // seamless camera transit is a single-camera effect. The portal
            // presentation's own configuration stays as its owner set it.
            {
                use ambition_platformer2d::portal_presentation as portal_view;
                app.declare_rules(
                    ambition_platformer2d::combat::scoped_rules::RulesScope::Mode(SMASH_MODE),
                    portal_view::PortalCameraTransitMode::Pop,
                );
                app.declare_rules(
                    ambition_platformer2d::combat::scoped_rules::RulesScope::Mode(SMASH_MODE),
                    portal_view::PortalViewConeMode::Static,
                );
            }
        }

        let sim = ambition_platformer2d::platformer::schedule::SimScheduleExt::sim_schedule(app);
        // The fighter specials sleep where the active room's game uses none
        // (`install_special` puts each handler in their set); so do the
        // per-tick follow-ups below that only a special starts: the limit
        // meter and the ride.
        // Sing: the module `ambition_demo_smash_modules::sing`, so an edit to
        // the song compiles that crate alone. Authored sleeps today: the
        // performer's `speech` pulse (`performer_moveset`) and the Shadow Oni
        // leader's seal.
        ambition_platformer2d::runtime::install_module_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_sleep::SLEEP,
            TechniqueOffer {
                owner: "ambition_demo_smash_modules::sing",
                params: TechniqueParams::Checked(
                    // The domain rule, not only "serde can build it".
                    ambition_platformer2d::entity_catalog::smash_sleep::check_sleep_params,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            ambition_demo_smash_modules::sing::module(),
        );
        // The mash is this game's rule of how a sleeper buys time back. It
        // runs before the song's sleep is lowered, so a press cannot be spent
        // on a sleep that did not exist when it was made; the sleep is a
        // floor, so the landing tick is the same either way.
        app.add_systems(
            sim,
            crate::sing::mash_out_of_sleep
                .in_set(FIGHTER_SPECIALS)
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentSpecials)
                .before(ambition_platformer2d::extension::ExtensionSet::Collect(
                    ambition_platformer2d::extension::sdk::phases::TECHNIQUE_EXECUTION,
                )),
        );
        // Portal recovery, in `ContentSpecials`. The close is chained after the
        // open, so an aperture's lifetime is spent from the frame it appears.
        install_special(
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
        install_special(
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
        // Homing dash: the module `ambition_demo_smash_modules::homing`, clock,
        // cone and steering. It asks the engine only who is a foe
        // (`ambition.combat.foes`) and sets its fighter's velocity.
        ambition_platformer2d::runtime::install_module_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_homing::HOMING_DASH,
            TechniqueOffer {
                owner: "ambition_demo_smash_modules::homing",
                params: TechniqueParams::Checked(
                    check_hydrates::<
                        ambition_platformer2d::entity_catalog::smash_homing::HomingDashParams,
                    >,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            ambition_demo_smash_modules::homing::module(),
        );
        // Riposte: the module `ambition_demo_smash_modules::riposte`, so an
        // edit to the cut compiles that crate alone. It reads the same
        // `ActorActionMessage` the counter writes, so it needs no
        // counter-specific wiring.
        ambition_platformer2d::runtime::install_module_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_riposte::RIPOSTE_STRIKE,
            TechniqueOffer {
                owner: "ambition_demo_smash_modules::riposte",
                params: TechniqueParams::Checked(
                    // The domain rule, not only "serde can build it".
                    ambition_platformer2d::entity_catalog::smash_riposte::check_riposte_strike_params,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            ambition_demo_smash_modules::riposte::module(),
        );
        // Tether reel. Throw is chained before reel, so a line that bit a ledge
        // pulls on that tick. The reel does not catch the ledge; the movement
        // kernel's ledge authority does, one phase later. See `crate::tether`.
        install_special(
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
        // Placed spring: the module `ambition_demo_smash_modules::spring`.
        // The drop puts a module entity on the stage, and the module ticks
        // it; the match owns its end (`stamp_smash_module_entities`).
        ambition_platformer2d::runtime::install_module_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_spring::PLACE_SPRING,
            TechniqueOffer {
                owner: "ambition_demo_smash_modules::spring",
                params: TechniqueParams::Checked(
                    ambition_platformer2d::entity_catalog::smash_spring::check_place_spring_params,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            ambition_demo_smash_modules::spring::module(),
        );
        app.add_systems(sim, stamp_smash_module_entities);
        // Limit meter: a set of independent sources, not one rule, so a
        // mechanic can author only the source it wants. The fill rule is
        // declared above for Smash's rooms.
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
        app.add_systems(
            sim,
            crate::limit::fill_limit_meters
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::ContentFlavor)
                .in_set(FIGHTER_SPECIALS),
        );
        // The authored half, a move that fills its own meter, is the module
        // `ambition_demo_smash_modules::limit` (`ambition.resources.refill`).
        ambition_platformer2d::runtime::install_module_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_limit::FILL_METER,
            TechniqueOffer {
                owner: "ambition_demo_smash_modules::limit",
                params: TechniqueParams::Checked(
                    check_hydrates::<ambition_platformer2d::entity_catalog::smash_limit::FillMeterParams>,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            ambition_demo_smash_modules::limit::module(),
        );
        // Time dilation: the module `ambition_demo_smash_modules::dilation`.
        // The module keeps the clock too, as a status on the slowed body
        // (`ambition.body.status_tick`).
        ambition_platformer2d::runtime::install_module_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_time_dilation::TIME_DILATION,
            TechniqueOffer {
                owner: "ambition_demo_smash_modules::dilation",
                params: TechniqueParams::Checked(
                    // The domain rule, not only "serde can build it".
                    ambition_platformer2d::entity_catalog::smash_time_dilation::check_time_dilation_params,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::Action,
            },
            ambition_demo_smash_modules::dilation::module(),
        );
        // Remote mine: the module `ambition_demo_smash_modules::mine`. A placed
        // mine is a module entity that is a held item; it arms as it ages, and
        // the module entities age in the player phase, before this tick's
        // presses, so a mine that becomes live this tick answers this tick's
        // press.
        ambition_platformer2d::runtime::install_module_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_mine::PLACE_MINE,
            TechniqueOffer {
                owner: "ambition_demo_smash_modules::mine",
                params: TechniqueParams::Checked(
                    ambition_platformer2d::entity_catalog::smash_mine::check_place_mine_params,
                ),
                references: NestedReferences::HeldItems(
                    ambition_platformer2d::entity_catalog::smash_mine::mine_held_item_refs,
                ),
                delivery: TechniqueDelivery::Action,
            },
            ambition_demo_smash_modules::mine::module(),
        );
        // Mark: the module `ambition_demo_smash_modules::mark`, answered on the
        // body a volume struck (`ambition.combat.struck`, `hit_reaction`), so
        // the strike's own tick applies it and spends none of its fuse. Its
        // clock is a status on that body, which the body-clock view draws.
        ambition_platformer2d::runtime::install_module_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_mark::MARK_BODY,
            TechniqueOffer {
                owner: "ambition_demo_smash_modules::mark",
                params: TechniqueParams::Checked(
                    ambition_platformer2d::entity_catalog::smash_mark::check_mark_body_params,
                ),
                references: NestedReferences::None,
                delivery: TechniqueDelivery::OnHit,
            },
            ambition_demo_smash_modules::mark::module(),
        );
        // Counter stance: the module `ambition_demo_smash_modules::counter`.
        // It holds the parry window open each tick of the stance
        // (`ambition.combat.parry_window`, lowered in `ContentSpecials`, inside
        // `Materialize`: after the stance's effect is emitted in `Playback`
        // and before `apply_hitbox_damage` judges in `Resolve`), and answers a
        // parry caught in it (`ambition.combat.parried`, `hit_reaction`). The
        // answer lands on the next tick: its `ActorActionMessage` is read in
        // `Materialize`, which has passed. The wait makes the catch visible.
        ambition_platformer2d::runtime::install_module_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_counter::COUNTER,
            TechniqueOffer {
                owner: "ambition_demo_smash_modules::counter",
                params: TechniqueParams::Checked(
                    check_hydrates::<ambition_platformer2d::entity_catalog::smash_counter::CounterParams>,
                ),
                // The response is admitted as an effect of its own.
                references: NestedReferences::Effects(
                    ambition_platformer2d::entity_catalog::smash_counter::counter_response_refs,
                ),
                delivery: TechniqueDelivery::Action,
            },
            ambition_demo_smash_modules::counter::module(),
        );
        // Shark summon, in `ContentSpecials`: it must produce its effects
        // before the effect executors run (`ContentSpecials.before(
        // EffectExecutionSet)`). Name the set; do not add a leaf-to-leaf edge.
        install_special(
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
        // Bomb: the module `ambition_demo_smash_modules::bomb`. A dropped bomb
        // is a module entity that is a held item, ticked with the items after
        // item physics (`module_entity_tick`), so an impact is read on the
        // tick it happens.
        ambition_platformer2d::runtime::install_module_technique(
            app,
            ambition_platformer2d::entity_catalog::smash_bomb::DROP_BOMB,
            TechniqueOffer {
                owner: "ambition_demo_smash_modules::bomb",
                params: TechniqueParams::Checked(
                    ambition_platformer2d::entity_catalog::smash_bomb::check_drop_bomb_params,
                ),
                references: NestedReferences::HeldItems(
                    ambition_platformer2d::entity_catalog::smash_bomb::bomb_held_item_refs,
                ),
                delivery: TechniqueDelivery::Action,
            },
            ambition_demo_smash_modules::bomb::module(),
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
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle)
                .in_set(FIGHTER_SPECIALS),
        );
        app.add_systems(
            sim,
            (
                crate::shark_ride::depart_when_riderless,
                crate::shark_ride::send_away_a_shark_nobody_boarded,
            )
                .chain()
                .after(ambition_platformer2d::mount::DismountRequestsApplied)
                .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle)
                .in_set(FIGHTER_SPECIALS),
        );
        // Departure writes intent after the actor brains publish, so the
        // riderless shark's brain cannot overwrite it before movement, and
        // before the control gate, so every restriction sees the frame (A4).
        app.add_systems(
            sim,
            crate::shark_ride::tick_departures
                .after(ambition_platformer2d::platformer::schedule::ActorDecisionSet::Publish)
                .before(ambition_platformer2d::platformer::schedule::PlayerInputSet::ControlGate)
                .in_set(ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhase::WorldPrep)
                .in_set(FIGHTER_SPECIALS),
        );
        // The capture interruption release is in `CombatSchedulePlugin`. Do
        // not add it here; it would run twice.
        //
        // These rules run after the engine's `Settle` work: the stock must be
        // spent before a body is placed. The HUD publisher only presents; it
        // shares the gated set so a hosted build stops drawing it outside the
        // stage.
        //
        // The engine ticks the respawn protection (`RespawnGraceTicked`). The
        // placement that grants it runs before, so a fighter is protected on
        // the tick it arrives; the rules that spend it run after.
        let placement = (
            publish_smash_hud,
            announce_the_opening_countdown,
            place_respawning_fighters,
        )
            .chain()
            .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle)
            .after(ambition_platformer2d::combat::stocks::FighterStocksSpent)
            // Also after the return is decided: with a respawn interval, a
            // placement racing the tick-down would miss a due fighter.
            .after(ambition_platformer2d::combat::stocks::FighterRespawnsDue)
            .before(ambition_platformer2d::actor::RespawnGraceTicked);
        let protection = (
            a_swing_spends_the_respawn_protection,
            hold_the_respawn_platforms,
            leaving_the_platform_spends_the_respawn_protection,
            announce_the_winner,
        )
            .chain()
            .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle)
            .after(ambition_platformer2d::actor::RespawnGraceTicked);
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
        let gate = ambition_platformer2d::runtime::in_mode(SMASH_MODE);
        app.add_systems(sim, placement.run_if(gate.clone()));
        app.add_systems(sim, protection.run_if(gate.clone()));
        app.add_systems(sim, remove_the_eliminated.run_if(gate));
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
/// The winner card. One slot, because the stage says one thing at a time.
pub const SMASH_ANNOUNCE_HUD_SLOT: &str = "smash_announce";

/// What one remaining stock is drawn as, under the sprites asset root.
///
/// Generated, not committed: `scripts/regen/sprites.sh` lists it in its publish
/// roster, so a fresh clone can produce it.
pub const STOCK_ICON_ASSET: &str = "sprites/hud_stock_icon.png";

/// What plays over the character select, in a host whose frontend audio
/// this demo owns. The pack declares it either way.
pub const SMASH_SELECT_TRACK: &str = "super_smash_siblings_character_select";

/// The rules of Smash's rooms, as its pack states them
/// (`assets/data/room_rules.ron`): the combat rules and the heavy-hit line.
/// [`SmashRulesPlugin`] declares them for the rooms tagged [`SMASH_MODE`].
///
/// An edit to the file is played at the next start, with no Rust build.
pub fn smash_room_rules() -> ambition_platformer2d::combat::room_rules::RoomRules {
    *ambition_platformer2d::combat::room_rules::lowered_room_rules(crate::smash_pack::PACK.prepared())
        .expect("Smash's pack states the rules of its rooms (`data/room_rules.ron`)")
}

/// The combat rules Smash plays under: the `combat` block of
/// [`smash_room_rules`]. Tools that fold them outside a match read this.
pub fn smash_combat_rules() -> ambition_platformer2d::combat::rules::CombatRules {
    smash_room_rules()
        .combat
        .expect("Smash's `data/room_rules.ron` states a `combat` block")
}

/// The kit this experience gives a fighter that authors none: the
/// `seating_melee` of `smash_rules.ron`, read when a match seats its roster.
///
/// A roster-preparation policy, not a combat rule: `CombatRules` does
/// not own a kit. Many of Ambition's cast author `default_action_set:
/// "peaceful"`; seating one in an arena adapts it into a fighter.
/// `roster_seeded` folds this into the seat's `ActionSet` at seating time, so
/// the body has one move authority and nothing reads a fallback.
pub fn smash_seating_melee() -> ambition_platformer2d::character::MeleeActionSpec {
    rules::smash_rules().seating_melee
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
    // The running session's frozen cast, or the published one with no session.
    declared: ambition_platformer2d::characters::prepared::SessionCast,
    mut readouts: bevy::prelude::ResMut<ambition_platformer2d::presentation::HudReadouts>,
) {
    let mut rows: Vec<(usize, String, f32, Option<(u32, u32)>, Option<HudFace>, f32)> = fighters
        .iter()
        .map(|(seat, health, stocks, combat, name, worn)| {
            let face = worn.zip(catalog.as_deref()).and_then(|(worn, catalog)| {
                hud_face(
                    catalog,
                    portraits.as_deref(),
                    declared.get(),
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
            ambition_platformer2d::presentation::HudReadout::bare(text::line("announce.sudden_death")),
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
        OpeningPhase::Live if elapsed < total + per_beat => Some(text::line("announce.go")),
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
    stage: StageCentre,
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
        let centre = stage.of(event.body);
        let placement = respawn_placement(centre, seat);
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
                centre.x - placement.x,
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
                remaining: crate::stages::smash_stages().respawn.protection_seconds,
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
    // The platforms of each live room: a respawn platform is in the live room
    // of the body it protects.
    mut rooms: bevy::prelude::Query<
        (
            &ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
            &mut ambition_platformer2d::world::collision::MovingPlatformSet,
        ),
        bevy::prelude::With<ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot>,
    >,
    live: ambition_platformer2d::platformer::lifecycle::LiveRooms,
    // `RespawnGrace` removes itself when it runs out, so the platform's
    // presence is the component's presence.
    protected: bevy::prelude::Query<
        (
            bevy::prelude::Entity,
            &ambition_platformer2d::actor::MatchSeat,
            &ambition_platformer2d::engine_core::BodyKinematics,
        ),
        bevy::prelude::With<ambition_platformer2d::actor::RespawnGrace>,
    >,
) {
    for (room, mut platforms) in &mut rooms {
        hold_the_respawn_platforms_of_one_room(
            &mut platforms,
            protected
                .iter()
                .filter(|(body, _, _)| live.of(*body) == Some(*room))
                .map(|(_, seat, kin)| (seat.0, kin.pos)),
        );
    }
}

/// [`hold_the_respawn_platforms`] for one live room: `protected` is the seat
/// and the position of each protected body in that room.
fn hold_the_respawn_platforms_of_one_room(
    platforms: &mut ambition_platformer2d::world::collision::MovingPlatformSet,
    protected: impl Iterator<Item = (usize, Vec2)>,
) {
    let mut wanted: Vec<(String, Vec2)> = Vec::new();
    for (seat, pos) in protected {
        wanted.push((
            respawn_platform_id(seat),
            Vec2::new(pos.x, pos.y + respawn_platform_drop()),
        ));
    }
    // Sort by id so the order depends only on which seats are protected. The
    // visuals reconcile by index and the set is rollback-canonical.
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
                respawn_platform_size(),
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
    // The platforms of the live room each body is in.
    platforms: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<
        ambition_platformer2d::world::collision::MovingPlatformSet,
    >,
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
        let Some(platform) = platforms
            .of(body)
            .and_then(|platforms| platforms.0.iter().find(|platform| platform.id == id))
        else {
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

/// The platform a returning fighter materialises on (`respawn.platform` in
/// `smash_stages.ron`).
pub fn respawn_platform_size() -> Vec2 {
    let (width, height) = crate::stages::smash_stages().respawn.platform;
    Vec2::new(width, height)
}

/// How far below the fighter's centre the platform's centre sits: half a
/// standing body plus half the platform, so its top is under the feet.
fn respawn_platform_drop() -> f32 {
    24.0 + respawn_platform_size().y * 0.5
}

/// The freeze a FULL punch is measured against, in seconds.
///
/// Measured: `hitlag_duration` scales with damage, and this is what a heavy
/// connect produces under this stage's feel. A lower value saturates on jabs.
const HUD_PUNCH_REFERENCE_HITLAG: f32 = 0.12;

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


/// Return to character select after a decided match has shown its winner card
/// for `return_to_select_seconds` (`smash_rules.ron`), or at once for a `NoContest`.
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
        *countdown = Some(crate::rules::smash_rules().return_to_select_seconds);
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
            // Move-opened apertures and their clock: a restore without the
            // clock closes the recovery route at a different moment.
            app.rollback_component_clone_probed::<crate::portal::MovePlacedPortal>(
                "ambition_demo_smash",
                "smash.move_placed_portal",
                crate::portal::move_placed_portal_probe,
            );

            // The bolt in flight: position, heading, lifetime, and whether it
            // cleared its caster. The heading matters: peers can agree on the
            // lifetime and disagree on where the bolt goes.
            app.rollback_component_clone_probed::<crate::bolt::SteeredBolt>(
                "ambition_demo_smash",
                "smash.steered_bolt",
                crate::bolt::steered_bolt_probe,
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
        app.init_resource::<select::SmashSkins>();
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
        // The seats each route offers. A host seats input participants from
        // the match roster, which this screen produces, so the lobby offers
        // one seat per source that can join. Gameplay seats come from the
        // frozen roster, so its route offers none but keeps the couch policy.
        {
            use ambition_platformer2d::game_shell::{RouteSeating, RouteSeatingAppExt, SeatCount};
            let couch = ambition_platformer2d::input::InputAssignmentPolicy::JoinToClaim;
            app.declare_route_seating(
                SMASH_SELECT_ROUTE,
                RouteSeating::new(
                    SeatCount::OnePerSource {
                        max: select::MAX_SELECT_SOURCES as u8,
                    },
                    couch,
                ),
            )
            .declare_route_seating(
                SMASH_GAMEPLAY_ROUTE,
                RouteSeating::new(SeatCount::Fixed(0), couch),
            );
        }
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
                label: text::line("pause.exit_label"),
                detail: text::line("pause.exit_detail"),
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

/// Who can be picked in this composition: `select::smash_roster()` filtered to
/// the ids this host can seat.
fn assemble_the_smash_roster(
    // The seatable authority, not the catalog; see `SmashRoster::assemble`.
    // Optional: with no characters registered, the grid is empty.
    registry: ambition_platformer2d::characters::prepared::SessionCast,
    mut fighters: bevy::prelude::ResMut<select::SmashRoster>,
    // Each fighter's skins: the catalog says which rows are looks of it.
    catalog: Option<bevy::prelude::Res<ambition_platformer2d::character::CharacterCatalog>>,
    mut skins: bevy::prelude::ResMut<select::SmashSkins>,
) {
    let Some(registry) = registry.get() else {
        return;
    };
    let assembled = select::SmashRoster::assemble(registry);
    if let Some(catalog) = catalog.as_deref() {
        let looks = select::SmashSkins::assemble(&assembled, catalog, registry);
        if *skins != looks {
            *skins = looks;
        }
    }
    if *fighters != assembled {
        *fighters = assembled;
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
    ambition_platformer2d::input::ActiveUiCues::sync_marked(
        &mut cues,
        ambition_platformer2d::input::UiCue {
            context: ambition_platformer2d::input::SELECT_CONTEXT,
            priority: ambition_platformer2d::input::participant::context_priority::SELECT,
            // The cursor takes a role, takes a fighter, or presses START: one
            // word fits all three.
            submit_label: text::line("prompt.choose"),
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
        (0..select::MAX_SELECT_SOURCES as u8).any(|seat| {
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
    skins: Option<bevy::prelude::Res<select::SmashSkins>>,
    router: bevy::prelude::Res<ambition_platformer2d::game_shell::ShellRouter>,
    roster: Option<bevy::prelude::Res<MatchParticipantRoster>>,
    // The seat policy decides whether source index zero is the keyboard or a
    // pad (as in `source_name_under`), so the roster and the slot label agree.
    assignment: bevy::prelude::Res<ambition_platformer2d::input::LocalSeatOffer>,
    // Characters that author their own moves do not get this stage's generic
    // kit. `Option`, like every other reader of the cast.
    prepared: ambition_platformer2d::characters::prepared::SessionCast,
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

    let Some(decided) = select.roster_seeded(
        &fighters,
        &skins.as_deref().cloned().unwrap_or_default(),
        seed,
        assignment.policy(),
        // Ids whose character authors its own move timelines. Only this side
        // can see the prepared cast.
        &prepared
            .get()
            .map_or_else(Default::default, |registry| {
                registry
                    .iter()
                    .filter(|(_, definition)| definition.authored_moveset.is_some())
                    .map(|(id, _)| id.to_string())
                    .collect()
            }),
        // The kit a fighter that authors none is seated with.
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
    // The Smash pad layout, declared for this experience and released on the
    // way out; `insert_gamepad_bindings` is unchanged (A=Jump stays right for
    // Ambition). The layout permutes the fully assigned default pad, which is
    // the only way gamepad Special gets a button.
    commands.insert_resource(ambition_platformer2d::input::DeclaredBindingLayout::new(
        SMASH_EXPERIENCE,
        ambition_platformer2d::input::BindingLayout::Smash,
    ));
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
/// the shell withdraws the route's declared seat offer,
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
            // The name of the game, as its row of a launcher shows it.
            text::line("launcher.label"),
            text::line("launcher.description"),
            text::line("launcher.preparing"),
            // No `.with_procedural_sfx()`: the stage is silent and the
            // fighters bring their own cues.
            ambition_platformer2d::provider::AuthoredCatalogFragments::new(
                SMASH_CHARACTER_ID,
                SMASH_EXPERIENCE,
            ),
        )
        // The look of its launcher row.
        .with_launcher_look("eclipse")
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
        app.add_plugins(SmashRulesPlugin);

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
// other demos' protagonists, named by id in `roster` (`smash_rules.ron`). The two
// robot rows are stand-ins for the lineage the content catalog owns; see
// `stand_ins` (`smash_rules.ron`). The rows are in `assets/data/character_catalog.ron`.

/// Register this demo's content.
///
/// The crossover cast is mostly Ambition's own, so the stocks loop is proven on
/// shipped content. This also declares the audio fragment: preparation refuses
/// an experience whose provider registered none. The stage declares music and
/// no SFX; the fighters bring their own cues.
fn install_smash_content(app: &mut bevy::prelude::App) {
    use ambition_platformer2d::audio::catalog::AudioCatalogAppExt;

    // The cast is the pack's: every catalog row, with its move table and its
    // platform-fighter facet. See `assets/data/character_catalog.ron`.
    crate::smash_pack::PACK
        .cast(SMASH_EXPERIENCE, Some(SMASH_CHARACTER_ID))
        .register(app);
    // A build that reads the pack off disk plays an edit to these from the
    // next match. The cast, its facets, the room rules and the audio are read
    // here, once.
    app.add_plugins(ambition_platformer2d::content::live_reload::EmbeddedPackReloadPlugin {
        pack: &crate::smash_pack::PACK,
        live: &[
            crate::rules::SMASH_RULES_SCHEMA,
            crate::stages::SMASH_STAGES_SCHEMA,
            ambition_platformer2d::versus_match::match_rules::MATCH_RULES_SCHEMA,
            // Each line is read where it is shown. The launcher row is read
            // once when the launcher is built, so it keeps its start words.
            crate::text::SMASH_TEXT_SCHEMA,
        ],
        restated: Vec::new(),
    });
    // The pack states the music; it states no procedural SFX, because the
    // fighters' moves name bank cues.
    app.register_audio_catalog_fragment(
        crate::smash_pack::PACK
            .audio(SMASH_EXPERIENCE)
            .with_resident_sfx_bank(),
    );
    app.add_plugins(ambition_platformer2d::content::live_reload::EmbeddedPackReloadPlugin::audio(&crate::smash_pack::PACK, SMASH_EXPERIENCE));
}

/// Which stage the next match is played on: one of the room ids the pack's
/// `smash_stages.ron` offers.
///
/// A resource because the preparation source (`PlatformerExperienceAuthoring::
/// install`) may read the provider's own resources; no engine change needed.
///
/// Defaults to the first offered stage.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SmashStageChoice {
    room: &'static str,
}

impl Default for SmashStageChoice {
    fn default() -> Self {
        Self::all()[0]
    }
}

/// How many stocks this match gives each fighter: one of `stock_choices` in
/// the pack's `smash_rules.ron`.
///
/// Feeds `MatchRules::stocks`, which the engine pairs with
/// `DeathPolicy::Unbounded`. Defaults to [`starting_stocks()`].
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SmashStockChoice(u32);

impl Default for SmashStockChoice {
    fn default() -> Self {
        Self(starting_stocks())
    }
}

impl SmashStockChoice {
    /// The choice of `count` stocks. Panics when the rules do not offer it.
    pub fn of(count: u32) -> Self {
        assert!(
            crate::rules::smash_rules().stock_choices.contains(&count),
            "`smash_rules.ron` does not offer {count} stocks"
        );
        Self(count)
    }

    /// The count this choice gives each fighter.
    pub fn count(self) -> u32 {
        self.0
    }

    /// What the stocks button shows: derived from [`Self::count`], so the
    /// button cannot disagree with the count it sets.
    pub fn label(self) -> String {
        self.count().to_string()
    }

    /// Every count, in cycle order (`stock_choices`).
    pub fn all() -> Vec<Self> {
        crate::rules::smash_rules().stock_choices.iter().copied().map(Self).collect()
    }

    /// The next count in the cycle, for a single button that walks them.
    pub fn next(self) -> Self {
        let all = Self::all();
        let here = all.iter().position(|count| *count == self).expect("a choice is made from `all`");
        all[(here + 1) % all.len()]
    }
}

impl SmashStageChoice {
    /// Every offered stage, in cycle order (`offered` in `smash_stages.ron`).
    ///
    /// Consumers (the select button, `ladder_rig --stage`, the prepared match)
    /// resolve through this list, so a new row reaches all of them.
    pub fn all() -> Vec<Self> {
        crate::stages::smash_stages().offered.iter().map(|room| Self { room: room.as_str() }).collect()
    }

    /// The offered stage `room_id`, or `None` when the pack does not offer it.
    pub fn find(room_id: &str) -> Option<Self> {
        Self::all().into_iter().find(|stage| stage.room == room_id)
    }

    /// The offered stage `room_id`. Panics when the pack does not offer it.
    pub fn of(room_id: &str) -> Self {
        Self::find(room_id).unwrap_or_else(|| panic!("`smash_stages.ron` does not offer `{room_id}`"))
    }

    /// The room this choice starts in.
    pub fn room_id(self) -> &'static str {
        self.room
    }

    /// What a stage button shows.
    pub fn label(self) -> &'static str {
        &crate::stages::smash_stages().stages[self.room].label
    }

    /// The next stage in the cycle, for a single button that walks them.
    pub fn next(self) -> Self {
        let all = Self::all();
        let here = all.iter().position(|stage| *stage == self).expect("a choice is made from `all`");
        all[(here + 1) % all.len()]
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
    let rooms: Vec<RoomSpec> = SmashStageChoice::all().into_iter().map(|stage| smash_stage_room(stage.room_id())).collect();
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
        // The look is agreed too: both peers must draw the same skin.
        mix(card.skin as u64);
    }
    digest
}

/// The set of the fighter specials this crate offers: they run only where the
/// active room's game uses them (`UsedCapabilities`).
const FIGHTER_SPECIALS: ambition_platformer2d::platformer::capability::CapabilitySet =
    ambition_platformer2d::platformer::capability::CapabilitySet(
        ambition_platformer2d::platformer::capability::Capability::FighterSpecials,
    );

/// A module entity of this game (`smash.*`, a plate) belongs to the match
/// that made it, as a bomb or a mine does: the match's end sweeps it
/// (`versus_match::lifetime`). The world spawns it, so the stamp is put on
/// the first tick it is seen.
fn stamp_smash_module_entities(
    mut commands: bevy::prelude::Commands,
    spawned: bevy::prelude::Query<
        (bevy::prelude::Entity, &ambition_platformer2d::abilities::module_entity::ModuleEntity),
        (
            bevy::prelude::Added<ambition_platformer2d::abilities::module_entity::ModuleEntity>,
            bevy::prelude::Without<ambition_platformer2d::versus_match::MatchScoped>,
        ),
    >,
    active_match: Option<bevy::prelude::Res<ambition_platformer2d::versus_match::ActiveMatch>>,
) {
    for (entity, module_entity) in &spawned {
        if module_entity.kind.starts_with("smash.") {
            ambition_platformer2d::versus_match::stamp_match_object(&mut commands, entity, active_match.as_deref());
        }
    }
}

/// [`install_technique`] for a fighter special: its handler also joins
/// [`FIGHTER_SPECIALS`].
fn install_special<M>(
    app: &mut bevy::prelude::App,
    key: &str,
    offer: TechniqueOffer,
    systems: impl bevy::prelude::IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>,
) {
    use bevy::prelude::IntoScheduleConfigs as _;
    install_technique(app, key, offer, systems.into_configs().in_set(FIGHTER_SPECIALS));
}

/// [`install_techniques`] for a fighter special that answers several keys.
fn install_specials<M>(
    app: &mut bevy::prelude::App,
    offers: &[(&str, TechniqueOffer)],
    systems: impl bevy::prelude::IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>,
) {
    use bevy::prelude::IntoScheduleConfigs as _;
    install_techniques(app, offers, systems.into_configs().in_set(FIGHTER_SPECIALS));
}
