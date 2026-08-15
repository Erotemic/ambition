//! **Snakes on a plane: the flying-swarm enemy archetypes.**
//!
//! Two characters, not two skins — the Cartesian one is a maths joke and the
//! paper one is an aviation joke. Their catalog rows, pedestals and
//! conversations landed 2026-08-05, the archetypes on 08-06, and 1-2 places two
//! of each on 08-06 (Jon: *"I would like to add the flying snakes on a plane as
//! enemies in this world"*).
//!
//! ⭐ **the engine already flies, and it is DATA.** The ledger row claimed a
//! flying swarm needed a motion authority that is neither the body sweep nor a
//! projectile. It needed neither: `CharacterBrainTemplate::Aerial` and
//! `MoveStyleSpec::Float` have existed the whole time (the comment beside
//! `Float` names *"aerial bosses, sharks"*), and the catalog rows already say
//! `body_kind: Floating`, which is the chain
//! `Floating -> is_aerial -> gravity_scale: 0.0` plus the fly ability from
//! spawn.
//!
//! ⛔ **but "so this file is a table, not a system" was too strong, and placing
//! one is what showed it.** That claim is true of BEHAVIOUR and false of the
//! ENEMY: a plane snake in 1-2 drew a red placeholder rectangle until
//! [`register_snakes_on_a_plane_sheets`] existed, because sheet publishing is
//! the provider's own job in this demo (`snake.rs` and `ai_slop.rs` each carry
//! their own). A new enemy SHAPE is an archetype row plus its art.

/// **Register both swarms as CHARACTERS** — the same shape `snake.rs` uses,
/// one creature over, twice.
///
/// ⭐⭐ **this retires the demo's last roster fragment.** The pair used to be
/// `mary_o_snakes_on_a_*` archetype ROWS, kept because a STANDALONE Mary-O had
/// no cast entry for Ambition-registered characters and fell back to them. The
/// fork is closed the other way now: Mary-O is the one provider that registers
/// `npc_snakes_on_a_*`, in every composition — characters are shared by ID
/// across providers, so the Hall's pedestals still stage them in the hosted
/// app, from the merged catalog.
///
/// ⚠ **`aggro_radius: 0.0` and `attack_range: 0.0` are deliberate, and the
/// reason first written here was WRONG.** It said *"the `Aerial` template
/// ignores both, and giving them numbers would state a rule nothing reads"*.
/// It reads both: `aerial_brain_for_enemy` passes `aggro_radius` straight into
/// `AerialCfg` and folds `attack_range` into the roam radius
/// (`(110..170).max(attack_range * 1.5)`). So the zeros are not inert — they are
/// what makes these two ROAM and never DIVE, and a nonzero aggro radius would
/// turn each of them into a homing dive-bomber the moment Mary-O walked under
/// it.
///
/// Which is the behaviour we want and the reason to keep the zeros: offense is
/// body contact, on a creature that patrols its own patch of air. A Mary-O enemy
/// is an obstacle with a rhythm, not a pursuer — and a flyer that chases is a
/// different design question than "1-2 has no enemies".
///
/// ⭐ **the two differ in SPEED and HEALTH, not in kind.** A paper plane is
/// light and quick and dies to anything; a Cartesian plane is a grid and moves
/// like one — slower, steadier, and it takes two hits. That is the whole
/// difference a player feels.
///
/// ⚠ **`patrol_effort: 1.0` is the deleted rows' own pace** (the same catch
/// `snake.rs` documents): a swarm PACES its patch of air at full speed, and
/// `BrainProfile`'s default is the half-speed amble. ⛔ the row-vs-character
/// fork had hidden exactly this — the hosted build's character authored no
/// efforts and ambled at half pace while the standalone row flew at full; the
/// row's numbers are the authored intent, so unification keeps them.
pub(crate) fn register_snakes_on_a_plane_characters(app: &mut bevy::prelude::App) {
    use ambition_platformer2d::actors::character_runtime::{
        CharacterDefinition, CharacterDefinitionAppExt,
    };
    use ambition_platformer2d::characters::actor::{CharacterLocomotion, ContactDamage};
    use ambition_platformer2d::characters::brain::{
        BrainProfile, CharacterBrainTemplate, MoveStyleSpec,
    };

    for (id, display, sheet, run_speed, max_health) in [
        (
            PAPER_PLANE_CHARACTER_ID,
            PAPER_PLANE_DISPLAY_NAME,
            PAPER_PLANE_SHEET_TARGET,
            58.0,
            1,
        ),
        (
            CARTESIAN_PLANE_CHARACTER_ID,
            CARTESIAN_PLANE_DISPLAY_NAME,
            CARTESIAN_PLANE_SHEET_TARGET,
            38.0,
            2,
        ),
    ] {
        let mut definition =
            CharacterDefinition::new(id, display, crate::provider::MARY_O_EXPERIENCE)
                .with_sheet(sheet)
                .with_locomotion(CharacterLocomotion {
                    run_speed,
                    move_style: MoveStyleSpec::Float,
                    // Stated on the CHARACTER so the fact travels: the catalog
                    // row's `body_kind: Floating` says the same thing, but a
                    // body that reads its gravity-freedom only from a row it
                    // cannot see falls out of the sky.
                    baseline_free_flight: Some(true),
                    ..Default::default()
                })
                .with_contact_damage(ContactDamage {
                    strength: 0.5,
                    amount: 1,
                })
                .with_autonomous_profile(BrainProfile {
                    template: CharacterBrainTemplate::Aerial,
                    aggro_radius: 0.0,
                    attack_range: 0.0,
                    patrol_effort: 1.0,
                    chase_effort: 1.0,
                    ..Default::default()
                });
        definition.vitals.max_health = Some(max_health);
        app.register_character(definition);
    }
}

/// The brain key 1-2's `EnemySpawn` placements still carry BESIDE their
/// `character_id`. Construction is character-first, so this no longer resolves
/// a row — it is placement metadata the binding test uses to find the swarms.
pub const PAPER_PLANE_BRAIN_KEY: &str = "mary_o_snakes_on_a_paper_plane";
/// See [`PAPER_PLANE_BRAIN_KEY`].
pub const CARTESIAN_PLANE_BRAIN_KEY: &str = "mary_o_snakes_on_a_cartesian_plane";

/// The catalog character each brain wears, for the `EnemySpawn.character_id` a
/// level authors beside the brain key.
pub const PAPER_PLANE_CHARACTER_ID: &str = "npc_snakes_on_a_paper_plane";
/// See [`PAPER_PLANE_CHARACTER_ID`].
pub const CARTESIAN_PLANE_CHARACTER_ID: &str = "npc_snakes_on_a_cartesian_plane";

/// **The sheet TARGETS, and they are not the catalog ids.**
///
/// ⚠ `solid_snake` and `ai_slop` are both at once, which is why neither of those
/// modules needs a pair of constants — and why copying their shape without
/// noticing would publish these two under a name nothing asks for. The renderer
/// draws `snakes_on_a_paper_plane`; the catalog calls the same creature
/// `npc_snakes_on_a_paper_plane`.
pub const PAPER_PLANE_SHEET_TARGET: &str = "snakes_on_a_paper_plane";
/// See [`PAPER_PLANE_SHEET_TARGET`].
pub const CARTESIAN_PLANE_SHEET_TARGET: &str = "snakes_on_a_cartesian_plane";

/// The display names the catalog rows carry, which is the key the enemy render
/// resolves an actor by.
pub const PAPER_PLANE_DISPLAY_NAME: &str = "Snakes on a Paper Plane";
/// See [`PAPER_PLANE_DISPLAY_NAME`].
pub const CARTESIAN_PLANE_DISPLAY_NAME: &str = "Snakes on a Cartesian Plane";

/// **Ensure both plane sheets are drawable**, for the reason
/// [`crate::snake::register_solid_snake_sheet`] gives at length: the catalog
/// defers a non-eager character's sheet to a room-staging barrier that lives in
/// the app host, and a demo-staged enemy does not reliably drive it. An enemy
/// whose sheet never resolves does not error — it draws the fighting-actor
/// placeholder, which is a red rectangle, and a red rectangle is what a hostile
/// placeholder is SUPPOSED to look like.
///
/// ⛔ **the archetype was not the whole enemy, and the ledger said it was.** The
/// row that closed on 2026-08-06 recorded *"the whole enemy is a TABLE"* because
/// the movement kernel already flew. True of BEHAVIOUR and false of the enemy:
/// placing one in 1-2 drew two red rectangles until this existed. What a table
/// bought was the brain; art has always been the provider's own job here, and
/// `snake.rs` and `ai_slop.rs` each say so in their own file.
///
/// Keyed three ways per creature — sheet target, catalog id, display name —
/// because three different seams ask three different questions and the demo owns
/// the answer to all of them.
pub fn register_snakes_on_a_plane_sheets(
    game_assets: Option<
        bevy::prelude::ResMut<ambition_platformer2d::sprite_sheet::game_assets::GameAssets>,
    >,
    config: Option<
        bevy::prelude::Res<ambition_platformer2d::sprite_sheet::game_assets::GameAssetConfig>,
    >,
    asset_server: Option<bevy::prelude::Res<bevy::prelude::AssetServer>>,
    layouts: Option<
        bevy::prelude::ResMut<bevy::prelude::Assets<bevy::prelude::TextureAtlasLayout>>,
    >,
) {
    let (Some(mut game_assets), Some(config), Some(asset_server), Some(mut layouts)) =
        (game_assets, config, asset_server, layouts)
    else {
        return;
    };
    if config.no_assets {
        return;
    }
    for (target, character_id, display_name) in [
        (
            PAPER_PLANE_SHEET_TARGET,
            PAPER_PLANE_CHARACTER_ID,
            PAPER_PLANE_DISPLAY_NAME,
        ),
        (
            CARTESIAN_PLANE_SHEET_TARGET,
            CARTESIAN_PLANE_CHARACTER_ID,
            CARTESIAN_PLANE_DISPLAY_NAME,
        ),
    ] {
        if game_assets.characters.sheet(display_name).is_some() {
            continue;
        }
        let Some(asset) =
            ambition_platformer2d::actors::character_sprites::load_prop_sheet_for_target(
                &asset_server,
                &mut layouts,
                &config.sprite_folder,
                target,
                &ambition_platformer2d::sprite_sheet::character::SheetTuning::new(1.0, 0),
            )
        else {
            continue;
        };
        game_assets.characters.publish_under(target, asset.clone());
        game_assets
            .characters
            .publish_under(character_id, asset.clone());
        game_assets.characters.publish_under(display_name, asset);
    }
}
