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
//! validators) and the manifest-driven loader. Which `.ldtk` files exist and
//! where play starts is the pack's `data/worlds.ron`; THIS module says how
//! each file is reachable:
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

use ambition_platformer2d_world::world_manifest::{AuthoredWorldManifest, WorldManifest, WorldRoots, WORLD_MANIFEST_SCHEMA};

/// `(file, text)` of each world file `data/worlds.ron` lists, embedded by
/// `build.rs` under `static_map` (web / Android / bundled builds).
#[cfg(feature = "static_map")]
const EMBEDDED_WORLDS: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/worlds_embedded.rs"));
#[cfg(not(feature = "static_map"))]
const EMBEDDED_WORLDS: &[(&str, &str)] = &[];

/// The embedded text of the world file `file`, when this build carries one.
fn embedded_world(file: &str) -> Option<&'static str> {
    EMBEDDED_WORLDS.iter().find(|(name, _)| *name == file).map(|(_, text)| *text)
}

/// The intro world's embedded text, when `static_map` baked one in.
///
/// Exposed for the guard that every intro `NpcSpawn` names a character the
/// catalog knows — read from the SAME table the manifest ships, so the test
/// cannot pass against a different copy of the world than the game loads.
#[cfg(test)]
pub(crate) fn intro_ldtk_text() -> Option<&'static str> {
    embedded_world("intro.ldtk")
}

/// The game's world declaration, as the boot pack states it
/// (`data/worlds.ron`). It is read before any App exists (boot preparation
/// hands it to the loader, the catalog and the hot reload), so it is the
/// process's boot pack and not an App's selection.
pub fn world_manifest() -> WorldManifest {
    let authored = ambition_content_pack::lowered_rules::<AuthoredWorldManifest>(crate::pack::shipped(), WORLD_MANIFEST_SCHEMA)
        .expect("Ambition's pack states its worlds (`data/worlds.ron`)");
    let worlds_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/worlds");
    authored.manifest(&WorldRoots {
        loose_dir: &worlds_dir,
        asset_prefix: "game://worlds/",
        embedded_prefix: "ambition_content/worlds/",
        embedded_text: &embedded_world,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The world declaration is the pack's `data/worlds.ron`: each row the
    /// file states is a manifest row in its order, an added world is a row of
    /// the edited pack, and a declaration the loader cannot use is refused
    /// at admission with its file and field named.
    #[test]
    fn the_worlds_are_the_packs_world_manifest() {
        let manifest = world_manifest();
        let authored = ambition_content_pack::lowered_rules::<AuthoredWorldManifest>(crate::pack::shipped(), WORLD_MANIFEST_SCHEMA)
            .expect("the pack states its worlds");
        assert_eq!(manifest.entry_room, authored.entry_room);
        let rows: Vec<(String, String, bool)> = manifest
            .worlds
            .iter()
            .map(|source| (source.id.as_str().to_string(), source.asset_path.clone(), source.required))
            .collect();
        let stated: Vec<(String, String, bool)> = authored
            .worlds
            .iter()
            .map(|world| (world.id.clone(), format!("game://worlds/{}", world.file), world.required))
            .collect();
        assert_eq!(rows, stated, "the manifest is the file's rows, in order");
        for source in &manifest.worlds {
            let path = source.loose_path.as_ref().expect("a loose path");
            let on_disk = std::fs::read_to_string(path).unwrap_or_else(|_| panic!("`{}` is on disk", source.id.as_str()));
            // A build that embeds the worlds carries each listed file's text.
            if cfg!(feature = "static_map") {
                assert!(source.embedded_text == Some(on_disk.as_str()), "`{}` is embedded as it is on disk", source.id.as_str());
            } else {
                assert!(source.embedded_text.is_none(), "`{}`: no build embeds without `static_map`", source.id.as_str());
            }
        }

        let last = r#"(id: "world.magical_ldtk", file: "magical.ldtk"),"#;
        let with = |replacement: &'static str| {
            crate::pack::compile_pack_with(move |path, text| {
                if path != "data/worlds.ron" {
                    return text;
                }
                assert_eq!(text.matches(last).count(), 1, "the probe edits the last world");
                text.replace(last, replacement)
            })
        };
        let edited = with(r#"(id: "world.magical_ldtk", file: "magical.ldtk"), (id: "world.next", file: "next.ldtk"),"#)
            .expect("an added world compiles");
        let next = ambition_content_pack::lowered_rules::<AuthoredWorldManifest>(&edited, WORLD_MANIFEST_SCHEMA).unwrap();
        assert_eq!(next.worlds.last().map(|world| world.file.as_str()), Some("next.ldtk"));
        let refused = format!("{:?}", with(r#"(id: "world.magical_ldtk", file: "maps/magical.json"),"#).expect_err("not a world file"));
        let index = authored.worlds.len() - 1;
        assert!(
            refused.contains(&format!("`worlds[{index}].file` is `maps/magical.json`")) && refused.contains("data/worlds.ron"),
            "{refused}"
        );
    }

    /// EVERY SHIPPED `EnemySpawn` CAN BE BUILT — the invariant that stands between the
    /// refusal and a panic in a room nobody tests.
    ///
    /// Three things can resolve one — a COMPLETE character (one whose prepared definition yields a
    /// body blueprint), an archetype ROW under the placement's brain key, or the provider's own
    /// `with_open_casting_decision` waiver. A placement with none of the three panics at spawn.
    ///
    /// and that is not covered by the lowering tests beside this one.
    /// `the_pirate_sky_riders_lower_into_authored_mount_links` proves
    /// `pirate_sky_lookout` LOWERS; nothing built its bodies. The two are
    /// different failures — lowering reads the LDtk file, construction asks the
    /// cast — and a room can pass the first and panic on the second.
    ///
    /// asked of every world this provider ships, so a new room is covered the
    /// day it is authored rather than the day somebody remembers to list it.
    #[test]
    fn every_shipped_enemy_placement_can_be_built() {
        use ambition_platformer2d_ldtk::{LdtkProject, LdtkVocabulary};

        let mut app = bevy::prelude::App::new();
        crate::character_catalog::register_cast(&mut app);
        // ⛔ A LOW-LEVEL CONTENT QUESTION, NOT AN ADMISSION ONE. This fixture
        // installs no technique handlers, so real admission would correctly
        // withhold every character naming a native effect — the right answer to a
        // question this test is not asking. The raw road is named explicitly so
        // it cannot be reached by accident; see its doc for why the implicit
        // escape was removed.
        ambition_characters::prepared::close_preparation_barrier_without_admission(
            app.world_mut(),
        );
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        let prepared = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
            .clone();

        // ONE manifest, every world: `world_manifest()` declares the sandbox
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
                    let complete = prepared
                        .get(enemy.payload.character_id.as_str())
                        .is_some_and(|character| character.body_blueprint().is_ok());
                    if complete {
                        continue;
                    }
                    // there is no second road left to check (AC6). This
                    // then asked whether the brain key named an archetype ROW or
                    // this provider's declared casting waiver, because either
                    // would have built the body. Both are deleted: a placement
                    // that names no complete character is refused.
                    //
                    // a brain that names no identifier at all states a POLICY,
                    // and construction builds it a plain body — nothing to get
                    // wrong.
                    if !matches!(
                        &enemy.payload.brain,
                        ambition_entity_catalog::placements::CharacterBrain::Custom(_)
                    ) {
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
             each names no character that can build a body, and there is no \
             second road:\n  {}",
            unbuildable.join("\n  ")
        );
    }

    // The old archetype roster and its placement-resolution census are gone;
    // shipped placements resolve through character definitions instead.

    /// Shipped content exercises body/controller independence in both directions:
    /// one body under multiple controllers and one controller across multiple bodies.
    #[test]
    fn the_basement_gallery_shows_one_body_under_many_controllers_and_the_reverse() {
        use ambition_platformer2d_ldtk::{LdtkProject, LdtkVocabulary};
        use std::collections::{BTreeMap, BTreeSet};

        let manifest = world_manifest();
        let project =
            LdtkProject::load_default_for_dev(&manifest).expect("the shipped worlds load");
        let room_set = project
            .to_room_set(&manifest, &LdtkVocabulary::engine())
            .expect("the shipped worlds compose");

        let room = room_set
            .rooms
            .iter()
            .find(|room| room.id == "basement_enemies")
            .expect("`basement_enemies` is a shipped sandbox room");

        // body -> the distinct controllers it is placed under, and the reverse.
        let mut controllers_per_body: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut bodies_per_controller: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for enemy in &room.enemy_spawns {
            let body = enemy.payload.gameplay_character_id();
            let controller = format!("{:?}", enemy.payload.brain);
            controllers_per_body
                .entry(body.as_str().to_string())
                .or_default()
                .insert(controller.clone());
            bodies_per_controller
                .entry(controller)
                .or_default()
                .insert(body.as_str().to_string());
        }

        let most_controllers = controllers_per_body
            .iter()
            .max_by_key(|(_, controllers)| controllers.len())
            .map(|(body, controllers)| (body.clone(), controllers.len()))
            .unwrap_or_default();
        assert!(
            most_controllers.1 >= 3,
            "no body in `basement_enemies` is placed under 3 or more distinct \
             controllers — the best is {} at {}. The gallery no longer shows \
             that a controller can be changed without changing the body.\n\
             bodies -> controllers: {controllers_per_body:#?}",
            most_controllers.0,
            most_controllers.1
        );

        let most_bodies = bodies_per_controller
            .iter()
            .max_by_key(|(_, bodies)| bodies.len())
            .map(|(controller, bodies)| (controller.clone(), bodies.len()))
            .unwrap_or_default();
        assert!(
            most_bodies.1 >= 4,
            "no controller in `basement_enemies` is worn by 4 or more distinct \
             bodies — the best is {} at {}. The gallery no longer shows that one \
             policy serves many creatures.\n\
             controllers -> bodies: {bodies_per_controller:#?}",
            most_bodies.0,
            most_bodies.1
        );
    }

    /// HOW MANY NPC PLACEMENTS A CHARACTER-FIRST ROAD WOULD ACTUALLY CHANGE —
    /// checklist items 6 and 7, which had no number.
    ///
    /// The item calls the NPC road *"wide, mechanical, wants its own slice"* and
    /// sizes it by construction SITES (29). That is the plumbing. This is the
    /// population: of the shipped `NpcSpawn` placements, how many name a
    /// character the road could build a body FROM?
    ///
    /// the fallback still carries the trunk — 137 placements keep today's
    /// construction, because their character is unregistered (109) or registered
    /// without a body (28). That is the opposite of the enemy road, where the
    /// fallback carries the tail, and it is why this wants its own slice.
    ///
    ///  a FLOOR, so the number only grows: every character that becomes
    /// body-complete moves placements from the fallback onto the character-first
    /// road, and this says so instead of somebody re-running the census.
    #[test]
    fn the_npc_placements_a_character_first_road_would_build_only_grow() {
        use ambition_platformer2d_ldtk::{LdtkProject, LdtkVocabulary};

        let mut app = bevy::prelude::App::new();
        crate::character_catalog::register_cast(&mut app);
        // ⛔ A LOW-LEVEL CONTENT QUESTION, NOT AN ADMISSION ONE. This fixture
        // installs no technique handlers, so real admission would correctly
        // withhold every character naming a native effect — the right answer to a
        // question this test is not asking. The raw road is named explicitly so
        // it cannot be reached by accident; see its doc for why the implicit
        // escape was removed.
        ambition_characters::prepared::close_preparation_barrier_without_admission(
            app.world_mut(),
        );
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        let prepared = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
            .clone();

        let manifest = world_manifest();
        let project = LdtkProject::load_default_for_dev(&manifest).expect("worlds load");
        let room_set = project
            .to_room_set(&manifest, &LdtkVocabulary::engine())
            .expect("worlds compose");

        use ambition_entity_catalog::placements::{InteractionKindSpec, PlacementSchema};
        let (mut total, mut named, mut registered, mut complete) = (0usize, 0usize, 0usize, 0usize);
        let mut complete_ids: std::collections::BTreeSet<String> = Default::default();
        let mut unknown: std::collections::BTreeSet<String> = Default::default();
        for room in &room_set.rooms {
            for placement in &room.placements {
                let PlacementSchema::Interactable(spec) = &placement.schema else {
                    continue;
                };
                let InteractionKindSpec::Npc { character_id, .. } = &spec.kind else {
                    continue;
                };
                total += 1;
                let Some(cid) = character_id.as_deref() else {
                    continue;
                };
                named += 1;
                let Some(def) = prepared.get(cid) else {
                    unknown.insert(cid.to_string());
                    continue;
                };
                registered += 1;
                if def.body_blueprint().is_ok() {
                    complete += 1;
                    complete_ids.insert(cid.to_string());
                }
            }
        }
        assert!(
            total > 100,
            "only {total} NPC placements were walked, so this census is not \
             seeing the shipped worlds and any number below is meaningless"
        );
        assert!(
            complete >= 26,
            "only {complete} of {total} NPC placements name a body-complete \
             character, and it was 26 on 2026-08-13. This number is a FLOOR: a \
             character becoming body-complete moves its placements onto the \
             character-first road, so it does not fall. Complete: \
             {complete_ids:?}"
        );
        assert!(
            complete < total,
            "every NPC placement now names a body-complete character ({total} of \
             {total}) — the fallback road has no traffic left, so checklist item \
             6 is a deletion rather than a migration, and this ratchet goes with it"
        );
        // EVERY NAMED NPC CHARACTER IS PREPARED (AP30): the barrier prepares
        // each catalog row nobody authored as a bare definition. So "registered"
        // no longer separates migrated characters from the rest; what separates
        // them is `complete`, a character that authors its own locomotion. The
        // only names this Ambition-only fixture cannot resolve are the Hall's
        // guests from the Mary-O, Sanic and Smash providers; any other is a typo. A
        // guest's transform (Super Sanic) has no placement of its own: the Hall
        // shows it on the guest's body. Mary-O tall has one: she meets the robot
        // in the Magical dimension's gate garden in that form. Her swallow and
        // her rescued robot live there too.
        let guests: std::collections::BTreeSet<String> = [
            "laden_swallow",
            "mary_o",
            "mary_o_robot",
            "mary_o_tall",
            "npc_snakes_on_a_cartesian_plane",
            "npc_snakes_on_a_paper_plane",
            "sanic",
            "smash_george_booul",
            "snake_plant",
            "solid_snake",
            "void_lizard",
            "void_star",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        assert_eq!(
            unknown, guests,
            "NPC placements name characters no catalog row or definition knows \
             ({registered} of {named} prepared)"
        );
    }

    // When the type makes the absence unrepresentable, delete this test with the fallback."*
    //
    // A census of a state the type cannot hold measures nothing, and the refusal it was watching
    // for is pinned where it now happens — `convert_enemy_spawn`, in `conversion::mod`'s tests.

    /// the boss side of this chain was already pinned and the ENEMY side
    /// was not. `bosses::gnu_ton::tests::arena_spawns_the_adr0020_linked_pair`
    /// covers `convert_boss_spawn`; `convert_enemy_spawn` carries its own copy
    /// of the same four lines (`entity_converters.rs`) and nothing exercised it
    /// against a real world file. This does, off the shipped worlds (the
    /// lookout is in the PvN dimension's `pvn.ldtk`).
    ///
    /// scoped to the one level, exactly as the GNU-ton test is: composing the
    /// whole sandbox pulls in portal entities whose feature is off in this test
    /// build. `pirate_sky_lookout` authors none.
    #[test]
    fn the_pirate_sky_riders_lower_into_authored_mount_links() {
        use ambition_entity_catalog::placements::CharacterBrain;
        use ambition_platformer2d_core::AabbExt;
        use ambition_platformer2d_ldtk::{LdtkProject, LdtkVocabulary};

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
    fn manifest_names_the_eleven_worlds_and_the_hub_entry() {
        let manifest = world_manifest();
        assert_eq!(manifest.entry_room, "central_hub_complex");
        assert_eq!(manifest.worlds.len(), 11);
        for (id, what) in [
            ("world.dinosaur_ldtk", "the dinosaur dimension"),
            ("world.noodle_ldtk", "the noodle dimension"),
            ("world.math_ldtk", "the Math dimension"),
            ("world.science_ldtk", "the Science dimension"),
            ("world.evil_ldtk", "the Evil dimension"),
            ("world.pvn_ldtk", "the PvN dimension (pirates and ninjas)"),
            ("world.magical_ldtk", "the Magical dimension (Mary-O's)"),
        ] {
            assert!(manifest.worlds.iter().any(|world| world.id.as_str() == id), "{what}'s world is not in the manifest");
        }
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
