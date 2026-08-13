//! Ambition's LDtk WORLD payload + its `WorldManifest` — CONTENT, evicted
//! from the engine core (R3.2, the #1 violation: the engine shipped the
//! game's worlds).
//!
//! [`world_manifest`] BUILDS the declaration; nothing installs it anywhere.
//! `AmbitionContentPlugin::build` publishes one into its App as a resource
//! for in-schedule readers, and boot preparation passes the same value by
//! reference to the loader, catalog, hot-reload, and conversion seams that
//! run before any App exists (K2a).
//!
//! The engine keeps the room kit (`RoomSpec`/`RoomSet`, projection,
//! validators) and the manifest-driven loader; THIS module declares which
//! `.ldtk` files exist, where play starts, and how each file is reachable:
//!
//! - `loose_path` — the checked-in file under this crate's `assets/worlds/`
//!   (desktop dev + hot reload; the LDtk python tooling edits these).
//! - `embedded_text` — the JSON embedded into the binary under the
//!   `static_map` feature (web / Android / bundled builds; also the
//!   desktop disk-failure fallback).
//! - `asset_path` — the Bevy `AssetPath` the bevy_ecs_ldtk tile-render
//!   spine loads, under the `game://` asset source the app registers
//!   (rooted at this crate's `assets/` in dev, the shipped `assets/` dir
//!   in installed builds).

use std::path::Path;

use ambition_asset_manager::AssetId;
use ambition_platformer2d_actor_monolith::ldtk_world::{WorldManifest, WorldSource};

macro_rules! static_world_text {
    ($name:ident, $path:literal) => {
        #[cfg(feature = "static_map")]
        const $name: Option<&'static str> = Some(include_str!($path));
        #[cfg(not(feature = "static_map"))]
        const $name: Option<&'static str> = None;
    };
}

static_world_text!(SANDBOX_LDTK_STATIC, "../assets/worlds/sandbox.ldtk");
static_world_text!(INTRO_LDTK_STATIC, "../assets/worlds/intro.ldtk");
static_world_text!(
    CUT_ROPE_LDTK_STATIC,
    "../assets/worlds/you_have_to_cut_the_rope.ldtk"
);
static_world_text!(HALL_LDTK_STATIC, "../assets/worlds/hall_of_characters.ldtk");

/// The game's world declaration. The first row (sandbox) is boot-critical
/// and hot-reload-watched; the story side-worlds are tolerated missing so
/// a partial checkout still boots.
pub fn world_manifest() -> WorldManifest {
    let worlds_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/worlds");
    let source = |id: &str,
                  file: &str,
                  embedded_text: Option<&'static str>,
                  embedded_bevy_path: &'static str,
                  required: bool| WorldSource {
        id: AssetId::new(id),
        asset_path: format!("game://worlds/{file}"),
        loose_path: Some(worlds_dir.join(file)),
        embedded_text,
        embedded_bevy_path: Some(embedded_bevy_path),
        required,
    };
    WorldManifest {
        entry_room: "central_hub_complex".to_string(),
        // No baked ron-rooms shipped yet: generated rooms land here when a
        // bake tool emits them (W2 loader is live; see world::ron_room).
        ron_rooms: Vec::new(),
        worlds: vec![
            source(
                "world.sandbox_ldtk",
                "sandbox.ldtk",
                SANDBOX_LDTK_STATIC,
                "ambition_content/worlds/sandbox.ldtk",
                true,
            ),
            source(
                "world.intro_ldtk",
                "intro.ldtk",
                INTRO_LDTK_STATIC,
                "ambition_content/worlds/intro.ldtk",
                false,
            ),
            source(
                "world.cut_rope_ldtk",
                "you_have_to_cut_the_rope.ldtk",
                CUT_ROPE_LDTK_STATIC,
                "ambition_content/worlds/you_have_to_cut_the_rope.ldtk",
                false,
            ),
            source(
                "world.hall_ldtk",
                "hall_of_characters.ldtk",
                HALL_LDTK_STATIC,
                "ambition_content/worlds/hall_of_characters.ldtk",
                false,
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The authored `mounted_on` refs have to survive all the way to a
    /// `RoomSpec`, and for a month they did not.** Jon, 2026-08-08: *"The
    /// pirates in the pirate sky no longer ride their sharks."* An LDtk editor
    /// session (`6e48e5988`) rewrote `sandbox.ldtk` while a different level was
    /// open and brought every `EntityRef` in the file back as `null`; the four
    /// refs were restored from the `5e4d6448e` blob on 2026-08-09.
    ///
    /// ⭐ **the boss side of this chain was already pinned and the ENEMY side
    /// was not.** `bosses::gnu_ton::tests::arena_spawns_the_adr0020_linked_pair`
    /// covers `convert_boss_spawn`; `convert_enemy_spawn` carries its own copy
    /// of the same four lines (`entity_converters.rs`) and nothing exercised it
    /// against a real world file. This does, off the shipped `sandbox.ldtk`.
    ///
    /// ⚠ scoped to the one level, exactly as the GNU-ton test is: composing the
    /// whole sandbox pulls in portal entities whose feature is off in this test
    /// build. `pirate_sky_lookout` authors none.
    /// **EVERY SHIPPED `EnemySpawn` CAN BE BUILT** — the invariant that stands
    /// between the D102 refusal and a panic in a room nobody tests.
    ///
    /// ⛔⛔ **construction refuses an identifier nothing can resolve** (D102, and
    /// deliberately: an unresolvable spawn used to become a silent generic). Three
    /// things can resolve one — a COMPLETE character (one whose prepared
    /// definition yields a body blueprint), an archetype ROW under the placement's
    /// brain key, or the provider's own `with_open_casting_decision` waiver. A
    /// placement with none of the three panics at spawn.
    ///
    /// ⚠ **and that is not covered by the lowering tests beside this one.**
    /// `the_pirate_sky_riders_lower_into_authored_mount_links` proves
    /// `pirate_sky_lookout` LOWERS; nothing built its bodies. The two are
    /// different failures — lowering reads the LDtk file, construction asks the
    /// cast — and a room can pass the first and panic on the second.
    ///
    /// ⭐ asked of every world this provider ships, so a new room is covered the
    /// day it is authored rather than the day somebody remembers to list it.
    #[test]
    fn every_shipped_enemy_placement_can_be_built() {
        use ambition_platformer2d_actor_monolith::ldtk_world::{LdtkProject, LdtkVocabulary};

        let mut app = bevy::prelude::App::new();
        crate::character_catalog::register(&mut app);
        crate::player_robot_lineage::register_declared_cast(&mut app);
        crate::enemy_roster::register(&mut app);
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        let prepared = app
            .world()
            .resource::<ambition_platformer2d_actor_monolith::character_runtime::PreparedCharacterRegistry>()
            .clone();
        let roster = app
            .world()
            .resource::<ambition_platformer2d_actor_monolith::features::CharacterRoster>()
            .clone();

        // ⚠ ONE manifest, every world: `world_manifest()` declares the sandbox
        // plus the story side-worlds, so this covers a new room the day it is
        // authored rather than the day somebody remembers to list it here.
        let manifest = world_manifest();
        let project =
            LdtkProject::load_default_for_dev(&manifest).expect("the shipped worlds load");
        let room_set = project
            .to_room_set(&manifest, &LdtkVocabulary::engine())
            .expect("the shipped worlds compose");
        let mut unbuildable: Vec<String> = Vec::new();
        {
            for room in &room_set.rooms {
                for enemy in &room.enemy_spawns {
                    let complete = enemy
                        .payload
                        .character_id
                        .as_ref()
                        .and_then(|id| prepared.get(id.as_str()))
                        .is_some_and(|character| character.body_blueprint().is_ok());
                    if complete {
                        continue;
                    }
                    // The archetype road, asked as construction asks it: a ROW
                    // under this brain key, or this provider's own declared
                    // waiver. ⚠ the waiver comes from `OPEN_CASTING`, the same
                    // list `enemy_roster::register` hands the fragment, so this
                    // guard cannot drift from what construction was given.
                    let key = match &enemy.payload.brain {
                        ambition_entity_catalog::placements::CharacterBrain::Custom(name) => {
                            Some(name.as_str())
                        }
                        // A brain that names no identifier states a POLICY, and
                        // construction builds it a plain body — there is nothing
                        // to have gotten wrong.
                        _ => None,
                    };
                    let Some(key) = key else { continue };
                    if roster.has_brain_key(key)
                        || crate::enemy_roster::OPEN_CASTING
                            .iter()
                            .any(|(identifier, ..)| *identifier == key)
                    {
                        continue;
                    }
                    unbuildable.push(format!(
                        "{}/{} names character {:?} and brain {:?}",
                        room.id, enemy.id, enemy.payload.character_id, enemy.payload.brain
                    ));
                }
            }
        }
        assert!(
            unbuildable.is_empty(),
            "shipped `EnemySpawn` placements that construction would REFUSE — \
             each names no complete character, no archetype row, and no declared \
             open casting:\n  {}",
            unbuildable.join("\n  ")
        );
    }

    #[test]
    fn the_pirate_sky_riders_lower_into_authored_mount_links() {
        use ambition_entity_catalog::placements::CharacterBrain;
        use ambition_platformer2d_actor_monolith::ldtk_world::{LdtkProject, LdtkVocabulary};
        use ambition_platformer2d_core::AabbExt;

        const LOOKOUT: &str = "pirate_sky_lookout";

        let manifest = world_manifest();
        let mut project =
            LdtkProject::load_default_for_dev(&manifest).expect("sandbox LDtk should load");
        project.levels.retain(|level| level.identifier == LOOKOUT);
        let room_set = project
            .to_room_set(&manifest, &LdtkVocabulary::engine())
            .expect("pirate_sky_lookout composes");
        let lookout = room_set
            .rooms
            .iter()
            .find(|room| room.id == LOOKOUT)
            .expect("pirate_sky_lookout room exists");

        // Every rider in the room reaches a mount — asserted against the room's
        // OWN rider census rather than a typed-in four, so a fifth pirate is
        // covered the day it is authored and a deleted one cannot quietly lower
        // the bar.
        let riders: Vec<&str> = lookout
            .enemy_spawns
            .iter()
            .filter(|spawn| {
                matches!(&spawn.payload.brain, CharacterBrain::Custom(id) if id.ends_with("_shark_rider"))
            })
            .map(|spawn| spawn.id.as_str())
            .collect();
        assert!(
            !riders.is_empty(),
            "no shark rider is authored in {LOOKOUT}, so this test checks nothing"
        );
        let mounted: Vec<&str> = lookout
            .mount_links
            .iter()
            .map(|(rider, _)| rider.as_str())
            .collect();
        for rider in &riders {
            assert!(
                mounted.contains(rider),
                "{rider} is a shark rider with no authored mount link; the room \
                 lowered {:?}",
                lookout.mount_links,
            );
        }

        // And each link's far end is a shark the rider is standing on. A ref
        // that resolves to the wrong body spawns a pirate riding thin air just
        // as convincingly as no ref at all.
        for (rider_id, mount_id) in &lookout.mount_links {
            let mount = lookout
                .enemy_spawns
                .iter()
                .find(|spawn| &spawn.id == mount_id)
                .unwrap_or_else(|| panic!("mount link {rider_id} -> {mount_id} dangles"));
            assert!(
                matches!(&mount.payload.brain, CharacterBrain::Custom(id) if id == "burning_flying_shark"),
                "{rider_id} rides {mount_id}, which is {:?} and not a shark",
                mount.payload.brain,
            );
            let rider = lookout
                .enemy_spawns
                .iter()
                .find(|spawn| &spawn.id == rider_id)
                .unwrap_or_else(|| panic!("mount link source {rider_id} is not a spawn"));
            assert!(
                rider.aabb.strict_intersects(mount.aabb),
                "{rider_id} at {:?} does not touch the {mount_id} it rides at {:?}",
                rider.aabb,
                mount.aabb,
            );
        }
    }

    #[test]
    fn manifest_names_the_four_worlds_and_the_hub_entry() {
        let manifest = world_manifest();
        assert_eq!(manifest.entry_room, "central_hub_complex");
        assert_eq!(manifest.worlds.len(), 4);
        assert!(manifest.primary().required);
        assert_eq!(manifest.primary().id.as_str(), "world.sandbox_ldtk");
        for world in &manifest.worlds {
            assert!(
                world.loose_path.as_ref().is_some_and(|path| path.is_file()),
                "world file missing on disk: {:?}",
                world.loose_path
            );
        }
    }
}
