//! The boss-sprite wiring, pinned as far as it can be pinned headlessly.
//!
//! `upgrade_boss_sprites` draws the generic gradient-sentinel body for a boss
//! exactly when `GameAssets::boss_sprite(&boss_key)` misses, where `boss_key` is
//! the boss's lowercased BEHAVIOR ID. There are three ways to miss, and only the
//! last needs a window:
//!
//! 1. the sheet never loads — its catalog id resolves no path;
//! 2. the key disagrees — the registry is keyed by something other than the
//!    behavior id a live boss carries;
//! 3. the image is still loading — `images.get(...)` is `None` this frame, so
//!    the system skips and retries. Benign; the boss upgrades a frame later.

#![cfg(feature = "rl_sim")]

use ambition_platformer2d::boss_encounter::BossBehaviorProfileExt;
use std::collections::BTreeSet;

/// Load the provider-owned world manifest and assemble the same immutable
/// App-local boss contribution production uses.
fn content_boss_catalog() -> ambition_platformer2d::boss_encounter::BossCatalog {
    ambition_content::bosses::authored_boss_catalog()
}

#[test]
fn every_dedicated_boss_sheet_resolves_a_catalog_path() {
    let boss_catalog = content_boss_catalog();
    let character_catalog =
        ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog::from_data(
            ambition_platformer2d::characters::actor::character_catalog::parse_catalog(
                ambition_content::character_catalog::CHARACTER_CATALOG_RON,
            ),
        );
    let world_manifest = ambition_content::worlds::world_manifest();
    let catalog =
        ambition_platformer2d::actors::assets::platformer_assets::desktop_dev_default_catalog(
            &character_catalog,
            &boss_catalog,
            &ambition_content::audio_registries::load_music_registry(),
            &world_manifest,
        );

    let mut missing = Vec::new();
    for (key, _filename) in boss_catalog.sprite_filenames() {
        let id = ambition_platformer2d::asset_manager::platformer_assets::ids::boss_sprite(key);
        if catalog.try_path_for_load(&id).is_none() {
            missing.push(key);
        }
    }
    assert!(
        missing.is_empty(),
        "these boss sheets resolve no asset path, so `GameAssets::boss_sprites` \
         will not carry them and their bosses will draw the GENERIC body: {missing:?}. \
         Run `./scripts/regen/sprites.sh`, or fix the provider boss-catalog filename row."
    );
}

/// (2) The registry's keys and the renderer's lookup key are the same vocabulary.
///
/// `upgrade_boss_sprites` computes `boss_key = behavior.id.to_ascii_lowercase()`. So every authored
/// boss profile that HAS art must appear in the provider boss catalog under its own id — not under
/// its `sprite_target`, which is a different key the simulation uses for hurtbox metrics.
#[test]
fn the_render_key_is_the_behavior_id_not_the_sprite_target() {
    let boss_catalog = content_boss_catalog();

    let sheet_keys: BTreeSet<&str> = boss_catalog.authored_sheet_keys().collect();

    // Bosses that ship their own art.
    for id in [
        "mockingbird",
        "gnu_ton_rider",
        "smirking_behemoth_boss",
        "flying_spaghetti_monster_boss",
        "trex_boss",
    ] {
        let profile =
            ambition_platformer2d::boss_encounter::pattern::profile::BossBehaviorProfile::from_data(
                &boss_catalog,
                id,
            );
        assert_eq!(
            profile.id, id,
            "the profile registry must round-trip its own id"
        );
        assert!(
            sheet_keys.contains(id),
            "`upgrade_boss_sprites` looks up `boss_sprites[\"{id}\"]` (the lowercased \
             BEHAVIOR ID). It has no provider-authored sheet, so this boss draws \
             the generic body. Its `sprite_target` is {:?}, which is a DIFFERENT key, \
             used by the sim for hurtbox metrics. Add the behavior id to the provider boss \
             catalog; do not re-route the renderer through `sprite_target`.",
            profile.sprite_target,
        );
    }
}

/// The generic fallback is deliberate, and stays: a boss with no authored sheet draws the
/// gradient-sentinel body rather than nothing.
#[test]
fn a_boss_with_no_authored_sheet_is_absent_from_the_registry_on_purpose() {
    let boss_catalog = content_boss_catalog();
    let sheet_keys: BTreeSet<&str> = boss_catalog.authored_sheet_keys().collect();
    for id in ["clockwork_warden", "mode_collapse_boss", "overflow_boss"] {
        assert!(
            !sheet_keys.contains(id),
            "{id} has no dedicated art; it must fall back to the generic body, and \
             `upgrade_boss_sprites` warns once so it is never silent"
        );
    }
}

/// (2') The cause the other two tests could not see.
///
/// `the_render_key_is_the_behavior_id_not_the_sprite_target` resolves profiles
/// through `BossBehaviorProfile::from_data(&boss_catalog, id)` — the DIRECT path, which panics on
/// a miss. The SIM does not use that path. `BossConfig::new` runs
/// `canonical_boss_id_from(name, brain)` and then `for_authored_boss`, which
/// silently falls back to `BossBehaviorProfile::generic(key)`: a clone of the
/// clockwork warden's tuning wearing the authored slug as its id.
///
/// A generic profile draws the generic body, because `boss_sprites[slug]` misses.
/// So an LDtk placement whose display name slugs to anything but a registered
/// profile id renders generic no matter how correct its sheet wiring is — and
/// nothing upstream complains.
///
/// This test walks every boss placement in every shipped room and resolves it the
/// way the sim will. It is allowed to end on a generic profile; it is NOT allowed
/// to do so by accident.
#[test]
fn every_authored_boss_placement_resolves_the_profile_the_sim_will_spawn() {
    let boss_catalog = content_boss_catalog();

    let world_manifest = ambition_content::worlds::world_manifest();
    let project =
        ambition_platformer2d::ldtk_map::LdtkProject::load_default_for_dev(&world_manifest)
            .expect("the shipped LDtk project loads");
    let room_set = project
        .to_room_set(&world_manifest, &ambition_app::composed_ldtk_vocabulary())
        .expect("it lowers to rooms");

    let sheet_keys: BTreeSet<&str> = boss_catalog.authored_sheet_keys().collect();

    // Bosses the game deliberately ships without their own art. Everything else
    // that a room actually places must land on a sheet.
    // Bosses whose profile IS registered but which ship no dedicated art. They draw
    // the gradient-sentinel body on purpose, and `boss_sheet_wiring`'s third test
    // pins that choice.
    const DELIBERATELY_GENERIC: &[&str] = &[
        "clockwork_warden",
        "mode_collapse_boss",
        "overflow_boss",
        "exploding_gradient_boss",
    ];

    let mut placed = 0usize;
    let mut generic_by_accident = Vec::new();
    for room in &room_set.rooms {
        for spawn in &room.boss_spawns {
            placed += 1;
            let canonical = ambition_platformer2d::boss_encounter::behavior::canonical_boss_id_from(
                &spawn.name,
                &spawn.payload,
            );
            let profile = ambition_platformer2d::boss_encounter::pattern::profile::BossBehaviorProfile::for_authored_boss(
                &boss_catalog,
                &canonical,
            );
            let render_key = profile.id.to_ascii_lowercase().replace('-', "_");
            if sheet_keys.contains(render_key.as_str())
                || DELIBERATELY_GENERIC.contains(&render_key.as_str())
            {
                continue;
            }
            generic_by_accident.push(format!(
                "room `{}` places boss `{}` (name {:?}, brain {:?}) → canonical `{canonical}` \
                 → profile id `{}` → render key `{render_key}`",
                room.id, spawn.id, spawn.name, spawn.payload, profile.id
            ));
        }
    }

    assert!(placed > 0, "the shipped worlds place at least one boss");
    assert!(
        generic_by_accident.is_empty(),
        "these placements resolve to a profile with NO dedicated sheet, so they draw \
         the generic gradient-sentinel body. `for_authored_boss` fell back to \
         `BossBehaviorProfile::generic(slug)` without a word. Either the placement's \
         name/brain must slug to a registered profile id, or the boss belongs in \
         DELIBERATELY_GENERIC above:\n  {}",
        generic_by_accident.join("\n  ")
    );
}

/// A boss is also an actor — post-unification there is one body vocabulary — so
/// every boss's id appears in both `ActorRenderIndex` and `BossRenderIndex`.
/// `upgrade_actor_sprites` runs first. It resolved no character sheet for
/// "Mockingbird", fell back to the generic enemy sheet, and inserted a
/// `CharacterAnimator`. `upgrade_boss_sprites` is filtered
/// `Without<CharacterAnimator>` — so it skipped that boss forever, and its
/// dedicated sheet was never bound. Every boss in the game drew a generic body.
///
/// This test runs the REAL sim in the REAL boss rooms, because the collision is
/// between two `rebuild_*_render_index` systems and no amount of reading the
/// render code tells you which ids they actually emit. It asserts both halves:
///
/// 1. the collision is real (a boss id IS an actor id), so the rule is needed; and
/// 2. `actor_sprite_path_owns` makes the actor path yield on exactly those ids.
///
/// If a future refactor stops putting bosses in `ActorRenderIndex`, half (1) goes
/// red — and that is a fine outcome to have to think about, not a failure to
/// paper over.
#[test]
fn the_actor_sprite_path_yields_every_boss_to_the_boss_sprite_path() {
    use ambition_app::rl_sim::TimestepMode;
    use ambition_app::{AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions};
    use ambition_platformer2d::render::rendering::actor_sprite_path_owns;

    let mut rooms_checked = 0;
    let mut bosses_checked = 0;
    for room in [
        "mockingbird_arena",
        "gnu_ton_arena",
        "trex_arena",
        "flying_spaghetti_monster_arena",
        "basement_boss",
    ] {
        let opts = Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(room);
        let Ok(mut sim) = Platformer2dSimHarness::new_with_options(opts) else {
            continue; // a room the fixture cannot load is not this test's business
        };
        rooms_checked += 1;
        // A few ticks: the boss materializes, and both indices rebuild in
        // `FeatureViewSync` at the tail of the sim.
        for _ in 0..8 {
            sim.step(Default::default());
        }

        let boss_ids: Vec<String> = sim
            .world()
            .get_resource::<ambition_platformer2d::sim_view::BossRenderIndex>()
            .expect("the sim publishes the boss render index")
            .iter()
            .map(|(id, _)| id.to_string())
            .collect();
        assert!(
            !boss_ids.is_empty(),
            "room `{room}` places a boss but the boss render index is empty — the \
             renderer would have nothing to key a sheet from"
        );
        bosses_checked += boss_ids.len();

        let boss_index = sim
            .world()
            .get_resource::<ambition_platformer2d::sim_view::BossRenderIndex>()
            .unwrap();
        let actors = sim
            .world()
            .get_resource::<ambition_platformer2d::sim_view::ActorRenderIndex>()
            .expect("the sim publishes the actor render index");

        for id in &boss_ids {
            assert!(
                actors.get(id).is_some(),
                "room `{room}`: boss `{id}` is NOT in the actor index. The arbitration \
                 rule below is then unnecessary — check whether it still is."
            );
            // (2) The rule that resolves it.
            assert!(
                !actor_sprite_path_owns(id, boss_index),
                "room `{room}`: `upgrade_actor_sprites` would claim boss `{id}`, bind \
                 the generic enemy sheet + a `CharacterAnimator`, and lock \
                 `upgrade_boss_sprites` out of it forever."
            );
        }

        // The converse — the rule must not starve ordinary actors. An id the boss
        // index does not claim still belongs to the actor path, by construction:
        // `actor_sprite_path_owns` IS `boss_render.get(id).is_none()`. Pinned on a
        // name no boss will ever have rather than by iterating the actor index,
        // which would need a `HashMap` walk this read-model deliberately does not
        // expose (ADR 0023).
        assert!(
            actor_sprite_path_owns("EnemySpawn-not-a-boss", boss_index),
            "room `{room}`: the rule must yield ONLY bosses"
        );
    }

    assert!(rooms_checked >= 3, "checked {rooms_checked} boss rooms");
    assert!(bosses_checked >= 3, "checked {bosses_checked} bosses");
}

/// (4) DEDICATED BOSS SHEETS ARE DEMANDED BY A BOSS ROOM, NOT BY BOOT.
///
/// Every boss in the catalog used to decode at boot — seven sheets, 30 MP,
/// resident in the hall of characters with no boss in it, measured by the image
/// stage ledger as the hall's single largest owner (asset open work 2,
/// 2026-09-02). Now `ensure_boss_sheets_loaded` is the one seam, called when a
/// room that authors a `BossSpawn` is prepared; `load_game_assets` decodes none.
#[test]
fn boss_sheets_are_decoded_by_the_first_boss_room_and_not_at_boot() {
    use ambition_platformer2d::actors::assets::game_assets::{
        ensure_boss_sheets_loaded, load_game_assets,
    };
    use bevy::prelude::*;

    let boss_catalog = content_boss_catalog();
    let character_catalog =
        ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog::from_data(
            ambition_platformer2d::characters::actor::character_catalog::parse_catalog(
                ambition_content::character_catalog::CHARACTER_CATALOG_RON,
            ),
        );
    let world_manifest = ambition_content::worlds::world_manifest();
    let catalog =
        ambition_platformer2d::actors::assets::platformer_assets::desktop_dev_default_catalog(
            &character_catalog,
            &boss_catalog,
            &ambition_content::audio_registries::load_music_registry(),
            &world_manifest,
        );
    let dedicated: BTreeSet<&str> = boss_catalog
        .sprite_filenames()
        .map(|(key, _)| key)
        .filter(|key| Some(*key) != boss_catalog.fallback_sheet_key())
        .collect();
    assert!(
        dedicated.len() >= 5,
        "premise: the catalog names dedicated boss sheets ({dedicated:?})"
    );

    let mut app = App::new();
    app.add_plugins(bevy::app::TaskPoolPlugin::default());
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(bevy::image::ImagePlugin::default());
    app.init_asset::<TextureAtlasLayout>();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let mut layouts = app.world_mut().resource_mut::<Assets<TextureAtlasLayout>>();

    // BOOT: no dedicated sheet.
    let config = ambition_platformer2d::sprite_sheet::game_assets::GameAssetConfig::default();
    let mut assets = load_game_assets(
        &config,
        &character_catalog,
        &Default::default(),
        &boss_catalog,
        &catalog,
        &asset_server,
        &mut layouts,
        &ambition_platformer2d::world::rooms::RoomMetadata::default(),
        None,
    );
    assert!(
        assets.boss_sprites.is_empty(),
        "boot decoded dedicated boss sheets again: {:?}",
        assets.boss_sprites.keys().collect::<Vec<_>>()
    );

    // A BOSS ROOM: its own bosses' keys, and no other's. The authored worlds
    // supply the room; the keys come from the same derivation the renderer
    // makes for a live boss.
    let project =
        ambition_platformer2d::ldtk_map::LdtkProject::load_default_for_dev(&world_manifest)
            .expect("the shipped LDtk project loads");
    let room_set = project
        .to_room_set(&world_manifest, &ambition_app::composed_ldtk_vocabulary())
        .expect("it lowers to rooms");
    let boss_rooms: Vec<_> = room_set
        .rooms
        .iter()
        .filter(|room| !room.boss_spawns.is_empty())
        .collect();
    assert!(
        boss_rooms.len() >= 2,
        "premise: the worlds author more than one boss room"
    );
    // The first boss room whose bosses HAVE dedicated art (`basement_boss` is
    // deliberately generic and names none, which is also correct).
    let (first, first_keys) = boss_rooms
        .iter()
        .map(|room| {
            (
                *room,
                ambition_platformer2d::actors::assets::game_assets::boss_sheet_keys_for_room(
                    room,
                    &boss_catalog,
                ),
            )
        })
        .find(|(_, keys)| !keys.is_empty())
        .expect("premise: some authored boss room names a boss with a dedicated sheet");
    assert!(
        first_keys.len() < dedicated.len(),
        "premise: room `{}` names some but not all dedicated sheets ({first_keys:?})",
        first.id
    );
    let decoded = ensure_boss_sheets_loaded(
        &mut assets,
        &boss_catalog,
        Some(&first_keys),
        &catalog,
        &asset_server,
        &mut layouts,
        None,
    );
    let resident: BTreeSet<String> = assets.boss_sprites.keys().cloned().collect();
    assert_eq!(
        resident, first_keys,
        "a boss room demands exactly its own bosses' sheets"
    );
    assert_eq!(decoded, first_keys.len());
    // The same room again decodes nothing new.
    assert_eq!(
        ensure_boss_sheets_loaded(
            &mut assets,
            &boss_catalog,
            Some(&first_keys),
            &catalog,
            &asset_server,
            &mut layouts,
            None
        ),
        0,
        "the seam is idempotent"
    );
    // A fixture with no room in hand may still ask for everything.
    ensure_boss_sheets_loaded(
        &mut assets,
        &boss_catalog,
        None,
        &catalog,
        &asset_server,
        &mut layouts,
        None,
    );
    let resident: BTreeSet<&str> = assets.boss_sprites.keys().map(String::as_str).collect();
    assert_eq!(resident, dedicated);
}
