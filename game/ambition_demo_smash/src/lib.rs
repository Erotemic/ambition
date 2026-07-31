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
//!    on `ambition` + `bevy` and nothing else. If declaring a stocks match needs
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

// ⚠ **no `ambition::prelude::*`.** Declaring a match needs the ACTOR
// vocabulary, not the room-authoring one, and reaching for the prelude here
// would import nothing this file uses. That the prelude does not cover a match
// is a fact about what a prelude is for, not a gap.
use ambition::actor::{ControllerBinding, MatchParticipant, MatchParticipantRoster};
use ambition::engine_core as ae;
use ambition::engine_core::Vec2;
use ambition::world::rooms::RoomSpec;

/// The game-MODE tag this demo's rules gate on, so they sleep everywhere else.
pub const SMASH_MODE: &str = "smash";

/// Stocks each fighter starts with.
///
/// Three, because it is the smallest number that makes the middle of a match
/// feel different from its start and its end: at three you can lose one and
/// still be playing the same match, which is the thing rounds cannot express.
pub const STARTING_STOCKS: u32 = 3;

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
                        brain_profile: Some("medium_striker".to_string()),
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
const STAGE_SIZE: Vec2 = Vec2::new(960.0, 640.0);
const PLATFORM_TOP: f32 = 420.0;
const PLATFORM_WIDTH: f32 = 420.0;

/// **How far past the stage a body may travel before the world takes it.**
///
/// Tight ON PURPOSE, and this is the number that makes the demo a fighter rather
/// than a room with two people in it. Every other room in this game is ENCLOSED —
/// its margin exists to catch a body that fell through the floor, so it is
/// generous and rarely reached. Here it is the win condition, so it has to be
/// close enough that a good hit reaches it.
const BLAST_MARGIN_PX: f32 = 220.0;

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
        app.add_message::<ambition::actor::FighterStockSpent>();
        app.add_message::<ambition::actor::StocksMatchDecided>();

        let sim = ambition::platformer::schedule::SimScheduleExt::sim_schedule(app);
        // AFTER the engine's own `CombatSet::Settle` work: the stock is spent
        // there, and placing a body before it has been spent would put the
        // fighter back on the stage for a knockout that had not been counted.
        let rules = (place_respawning_fighters, announce_the_winner)
            .chain()
            .in_set(ambition::platformer::schedule::CombatSet::Settle)
            .after(ambition::combat::stocks::spend_fighter_stocks);
        if self.hosted {
            app.add_systems(sim, rules.run_if(ambition::runtime::in_mode(SMASH_MODE)));
        } else {
            app.add_systems(sim, rules);
        }
    }
}

/// Put a respawning fighter back over the platform.
///
/// ⚠ through `transit_body`, not by writing the position. That seam is the ONE
/// authority that re-resolves a body's pose against the world (ADR 0024), and a
/// respawn is exactly the case it exists for: a body appearing at a new place
/// has to arrive there, not be teleported into whatever is standing at the
/// coordinates.
fn place_respawning_fighters(
    mut spent: bevy::prelude::MessageReader<ambition::actor::FighterStockSpent>,
    mut bodies: bevy::prelude::Query<(
        ambition::actor::BodyClusterQueryData,
        &mut ambition::actors::features::MotionModel,
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
        ambition::actor::transit_body(
            &mut model,
            &mut clusters,
            respawn_placement(stage_centre()),
            // ZERO. A fighter that keeps the velocity that threw it off the stage
            // respawns already travelling toward the blast zone it just left.
            ambition::actor::TransitVelocity::Zero,
        );
    }
}

/// Say who won, once.
fn announce_the_winner(
    mut decided: bevy::prelude::MessageReader<ambition::actor::StocksMatchDecided>,
    mut banner: bevy::prelude::MessageWriter<ambition::combat::GameplayBannerRequested>,
) {
    for outcome in decided.read() {
        banner.write(ambition::combat::GameplayBannerRequested::new(
            victory_banner(outcome.winner.as_deref()),
            3.0,
        ));
    }
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
        ambition::provider::PlatformerExperienceAuthoring::new(
            SMASH_EXPERIENCE,
            SMASH_GAMEPLAY_ROUTE,
            "Smash",
            "Stocks, a platform, and nothing underneath it",
            "Prepare Smash",
            // No `.with_procedural_sfx()`: this stage declares SILENCE and the
            // fighters bring their own cues. Claiming procedural sfx it never
            // registers would be the same shape as the empty function above —
            // a declaration with nothing behind it.
            ambition::provider::AuthoredCatalogFragments::new(
                SMASH_CHARACTER_ID,
                SMASH_EXPERIENCE,
            ),
        )
        .with_loading_activity(ambition::load_presentation::DETERMINISTIC_LOADING_ACTIVITY_ID)
        .install(app, smash_prepared_session_world);
        app.add_plugins(SmashRulesPlugin::hosted());
    }
}

/// Stable ids the shell routes and lists this demo by.
pub const SMASH_EXPERIENCE: &str = "smash";
pub const SMASH_GAMEPLAY_ROUTE: &str = "smash_gameplay";
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
/// break the `ambition` + `bevy` rule that makes this crate an oracle at all.
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
    brain_presets: { "stand_still": StandStill },
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
            default_brain: "stand_still",
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
            default_brain: "stand_still",
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
    use ambition::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
    use ambition::characters::actor::character_catalog::{
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
        use ambition::actors::character_runtime::{CharacterDefinition, CharacterDefinitionAppExt};
        for (id, name, sheet) in [
            (SMASH_CHARACTER_ID, "Duelist A", "player_robot_v3"),
            (SMASH_OPPONENT_ID, "Duelist B", "player_robot_v2"),
        ] {
            app.register_character(
                CharacterDefinition::new(id, name, SMASH_EXPERIENCE).with_sheet(sheet),
            );
        }
    }
    app.register_audio_catalog_fragment(
        AudioCatalogFragment::new(SMASH_EXPERIENCE, None, None)
            .expect("the silent smash audio fragment is valid"),
    );
}

/// The stage, as the shared preparation lifecycle wants it.
fn smash_prepared_session_world() -> ambition::runtime::PreparedPlatformerSource {
    use ambition::runtime::demo_fixture::{
        ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
    };

    let room = smash_stage();
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    ambition::runtime::PreparedPlatformerSource::new(
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
    use ambition::engine_core::AabbExt;

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
        assert!(respawn.y < platform.top(), "the respawn is not above the stage");
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
        app.add_message::<ambition::actor::StocksMatchDecided>();
        app.add_message::<ambition::combat::GameplayBannerRequested>();
        app.add_systems(Update, announce_the_winner);
        app.update();

        app.world_mut()
            .resource_mut::<Messages<ambition::actor::StocksMatchDecided>>()
            .write(ambition::actor::StocksMatchDecided {
                winner: Some("seat 2".to_string()),
            });
        app.update();

        let messages = app
            .world()
            .resource::<Messages<ambition::combat::GameplayBannerRequested>>();
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
        app.add_message::<ambition::actor::StocksMatchDecided>();
        app.add_message::<ambition::combat::GameplayBannerRequested>();
        app.add_systems(Update, announce_the_winner);
        app.update();
        app.world_mut()
            .resource_mut::<Messages<ambition::actor::StocksMatchDecided>>()
            .write(ambition::actor::StocksMatchDecided { winner: None });
        app.update();

        let messages = app
            .world()
            .resource::<Messages<ambition::combat::GameplayBannerRequested>>();
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
        use ambition::game_shell::{
            MinimalShellPlugins, ShellExperienceId, ShellExperienceRegistry, ShellRouteCatalog,
            ShellRouteId,
        };
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_plugins(MinimalShellPlugins);
        app.add_plugins(ambition::load::AmbitionLoadPlugin);
        app.add_plugins(SmashExperiencePlugin);

        let registration = app
            .world()
            .resource::<ShellExperienceRegistry>()
            .get(&ShellExperienceId::new(SMASH_EXPERIENCE))
            .expect("a host that composed this plugin lists the smash experience");
        assert_eq!(registration.launch_route.as_str(), SMASH_GAMEPLAY_ROUTE);

        let route = app
            .world()
            .resource::<ShellRouteCatalog>()
            .get(&ShellRouteId::new(SMASH_GAMEPLAY_ROUTE))
            .expect("the launch route is registered");
        assert!(
            route.preparation.is_some(),
            "the route has no preparation, so entering it would drop a player into \
             a stage whose content was never prepared"
        );

        let authored = app
            .world()
            .resource::<ambition::provider::PlatformerAuthoredCatalogRegistry>()
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
        assert_eq!(prepared.starting_character().character_id, SMASH_CHARACTER_ID);
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

    /// A draw has a name. The engine's `winner: Option<String>` exists so this
    /// case does not need a sentinel, and the banner has to honour that.
    #[test]
    fn a_draw_is_announced_as_a_draw_rather_than_as_a_winner() {
        assert_eq!(victory_banner(Some("seat 1")), "seat 1 wins");
        assert!(victory_banner(None).contains("Draw"));
    }
}
