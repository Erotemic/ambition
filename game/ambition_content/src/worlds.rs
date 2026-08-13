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

    /// **HOW FAR IS `character_archetypes.ron` FROM BEING DELETED** — P2.22's
    /// acceptance signal, as a countdown rather than a survey.
    ///
    /// The campaign's acceptance signal is a DELETION, and the roster file is
    /// the last of it. This asks the question directly instead of reasoning
    /// about it: rebuild the resolution every placement goes through, but with
    /// an EMPTY roster, and list what stops resolving.
    ///
    /// ⭐ **that list IS the remaining work, and it is content rather than
    /// engineering.** Each entry is a creature nobody has decided on yet — the
    /// three identifiers this provider declares as open casting, which borrow
    /// `combatant` while the decision stands, and the placements that name a
    /// brain key and no character.
    ///
    /// ⚠ **this does NOT say the file can be deleted the moment the list is
    /// empty.** `spawn_actors` still ASKS the roster (`try_spec_for_brain`,
    /// `has_brain_key`); with no row left those arms answer `None` forever and
    /// become dead code to delete, which is checklist item 22. This measures the
    /// CONTENT half, which is the half that is blocked on Jon.
    #[test]
    fn what_still_needs_an_archetype_row() {
        use ambition_platformer2d_actor_monolith::ldtk_world::{LdtkProject, LdtkVocabulary};

        let mut app = bevy::prelude::App::new();
        crate::character_catalog::register(&mut app);
        crate::player_robot_lineage::register_declared_cast(&mut app);
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        let prepared = app
            .world()
            .resource::<ambition_platformer2d_actor_monolith::character_runtime::PreparedCharacterRegistry>()
            .clone();

        let manifest = world_manifest();
        let project =
            LdtkProject::load_default_for_dev(&manifest).expect("the shipped worlds load");
        let room_set = project
            .to_room_set(&manifest, &LdtkVocabulary::engine())
            .expect("the shipped worlds compose");

        let mut needs_a_row: Vec<String> = Vec::new();
        for room in &room_set.rooms {
            for enemy in &room.enemy_spawns {
                let builds_as_a_character = enemy
                    .payload
                    .gameplay_character_id()
                    .and_then(|id| prepared.get(id.as_str()))
                    .is_some_and(|character| character.body_blueprint().is_ok());
                if builds_as_a_character {
                    continue;
                }
                // A brain naming no identifier states a POLICY and gets a plain
                // body — it never needed a row.
                if let ambition_entity_catalog::placements::CharacterBrain::Custom(key) =
                    &enemy.payload.brain
                {
                    needs_a_row.push(format!("{}/{} wants row `{key}`", room.id, enemy.id));
                }
            }
        }
        // The identifiers this provider has declared open: they name no
        // character on purpose and borrow a row until Jon casts them.
        for (identifier, temporary_row, _) in crate::enemy_roster::OPEN_CASTING {
            needs_a_row.push(format!(
                "OPEN_CASTING `{identifier}` borrows row `{temporary_row}`"
            ));
        }
        needs_a_row.sort();

        // ⭐⭐ **FOUR → TWO on 2026-08-13, both by DECISION.** Jon cast
        // `SmallSkitter` ("skitters are Puppy Slug") and `large_brute` (a real
        // authored Goblin Brute, with the separate sprite generator that already
        // existed). Neither needed new architecture; both needed an answer.
        //
        // ⚠ **ONE LEFT, and it is the only genuinely open casting question in
        // the game.** `small_lurker` is the Gradient Sentinel's gradient-cascade
        // summon. Jon's skitter ruling does not reach it — a lurker is not a
        // skitter — so it is neither cast nor deleted, and inventing a creature
        // to empty this list is the exact move his handoff forbids.
        //
        // ⇒ what closes it is a decision or a deletion, not engineering. Until
        // then the cascade spawns generic combatants and the summon road warns
        // every time it does.
        let expected = ["OPEN_CASTING `small_lurker` borrows row `combatant`"];
        assert_eq!(
            needs_a_row.as_slice(),
            expected.as_slice(),
            "the distance to deleting `character_archetypes.ron` has changed. \
             SHORTER means a casting decision landed — delete the row it \
             borrowed if nothing else wants it, and update this list. LONGER \
             means something new took a dependency on the archetype table, \
             which is the direction the campaign exists to prevent. EMPTY means \
             the content half is DONE: the two surviving rows have no user left, \
             and what remains is deleting the code that still asks \
             (`try_spec_for_brain` / `has_brain_key` in `spawn_actors`)"
        );
    }

    /// **"CHANGING THE CONTROLLER DOES NOT CHANGE THE BODY" — ASSERTED OF
    /// SHIPPED CONTENT**, not of a fixture.
    ///
    /// The `basement_enemies` gallery used to be a row of one-of-each ARCHETYPE
    /// — `patrol cutter`, `small skitter`, `guard striker`, `medium striker`,
    /// `gradient seeker`, `large brute` — six slots with no creature identity,
    /// which left the campaign a product question: invent characters for them,
    /// or delete the row. Neither was needed. Once the archetypes separated into
    /// a body and a controller policy, the same six slots became a MATRIX.
    ///
    /// ⭐ **this is the campaign's central proposition standing up in a room a
    /// player can walk into**: one body wearing three different controllers, and
    /// one controller worn by four different bodies. A demo that shows six
    /// archetypes proves nothing about composition; this one cannot be authored
    /// at all unless body and controller are genuinely separable.
    ///
    /// ⚠ **both directions, because either alone is satisfiable by accident.**
    /// Several bodies under one controller is just "a shared policy"; several
    /// controllers on one body is just "a versatile creature". Only the two
    /// together say the axes are independent.
    #[test]
    fn the_basement_gallery_shows_one_body_under_many_controllers_and_the_reverse() {
        use ambition_platformer2d_actor_monolith::ldtk_world::{LdtkProject, LdtkVocabulary};
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
            let Some(body) = enemy.payload.gameplay_character_id() else {
                continue;
            };
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

    /// **HOW MANY NPC PLACEMENTS A CHARACTER-FIRST ROAD WOULD ACTUALLY CHANGE** —
    /// checklist items 6 and 7, which had no number.
    ///
    /// The item calls the NPC road *"wide, mechanical, wants its own slice"* and
    /// sizes it by construction SITES (29). That is the plumbing. This is the
    /// population: of the shipped `NpcSpawn` placements, how many name a
    /// character the road could build a body FROM?
    ///
    /// ⭐ **26 of 163 — 16%, not a blanket change**, and that matters because
    /// P2.20 records the campaign already paying for a ~100-NPC regression from
    /// "exactly that shape of blanket change". A migration that touches 26
    /// enumerable placements is a different bet from one that touches 163.
    ///
    /// ⚠ **the fallback still carries the trunk** — 137 placements keep today's
    /// construction, because their character is unregistered (109) or registered
    /// without a body (28). That is the opposite of the enemy road, where the
    /// fallback carries the tail, and it is why this wants its own slice.
    ///
    /// ⇒ **a FLOOR, so the number only grows**: every character that becomes
    /// body-complete moves placements from the fallback onto the character-first
    /// road, and this says so instead of somebody re-running the census.
    #[test]
    fn the_npc_placements_a_character_first_road_would_build_only_grow() {
        use ambition_platformer2d_actor_monolith::ldtk_world::{LdtkProject, LdtkVocabulary};

        let mut app = bevy::prelude::App::new();
        crate::character_catalog::register(&mut app);
        crate::player_robot_lineage::register(&mut app);
        crate::player_robot_lineage::register_declared_cast(&mut app);
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        let prepared = app
            .world()
            .resource::<ambition_platformer2d_actor_monolith::character_runtime::PreparedCharacterRegistry>()
            .clone();

        let manifest = world_manifest();
        let project = LdtkProject::load_default_for_dev(&manifest).expect("worlds load");
        let room_set = project
            .to_room_set(&manifest, &LdtkVocabulary::engine())
            .expect("worlds compose");

        use ambition_entity_catalog::placements::{InteractionKindSpec, PlacementSchema};
        let (mut total, mut registered, mut complete) = (0usize, 0usize, 0usize);
        let mut complete_ids: std::collections::BTreeSet<String> = Default::default();
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
                let Some(def) = prepared.get(cid) else {
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
        assert!(
            registered > complete,
            "every REGISTERED NPC character is body-complete, so \
             `body_blueprint` has stopped distinguishing — {registered} \
             registered, {complete} complete"
        );
    }

    /// **WHO IS STILL RIDING THE DISPLAY-NAME ROAD** — the measurement that
    /// sizes checklist item 15, kept as an exact set rather than a count.
    ///
    /// `EnemySpawnSpec::presentation_identity` falls back to the placement's
    /// display NAME when no `character_id` is authored. That road is
    /// presentation compatibility: tolerable for pixels, because a wrong sheet
    /// is visible, and intolerable for gameplay, which is why
    /// `gameplay_character_id` deliberately has no fallback. Item 15 deletes it
    /// once every placement names its character.
    ///
    /// ⭐ **an EXACT set, so this ratchets in both directions.** A new
    /// unnamed placement fails it, and so does casting the last one — at which
    /// point the fallback has no shipped user left and item 15 is a deletion
    /// rather than a survey. A count would only catch the first.
    ///
    /// ⚠ what is left is a CONTENT decision, not migration work: a thing called
    /// "Target" in a dive-drill room is plausibly the sandbag, but that changes
    /// the drill (ledger D96). ⚠ `BossSpawn` is a different population that
    /// carries no `character_id` field at all and resolves through boss
    /// profiles — it never rode this road.
    #[test]
    fn only_the_uncast_placements_still_ride_the_display_name_fallback() {
        use ambition_platformer2d_actor_monolith::ldtk_world::{LdtkProject, LdtkVocabulary};

        let manifest = world_manifest();
        let project =
            LdtkProject::load_default_for_dev(&manifest).expect("the shipped worlds load");
        let room_set = project
            .to_room_set(&manifest, &LdtkVocabulary::engine())
            .expect("the shipped worlds compose");

        let mut unnamed: Vec<String> = Vec::new();
        let mut total = 0usize;
        for room in &room_set.rooms {
            for enemy in &room.enemy_spawns {
                total += 1;
                if enemy.payload.gameplay_character_id().is_none() {
                    unnamed.push(format!("{}/{}", room.id, enemy.id));
                }
            }
        }
        assert!(
            total > 20,
            "only {total} enemy placements were walked, so this census is not \
             seeing the shipped worlds and an empty result would read as success"
        );
        unnamed.sort();

        // ⭐⭐ **EMPTY, as of 2026-08-13.** Every `EnemySpawn` in every world this
        // provider ships names the character it is. The last two were content
        // decisions Jon made that day: `under_town_skitter` is a Puppy Slug, and
        // the dive-drill's anonymous "Target" — AI-invented placeholder content
        // he does not care about preserving — was deleted rather than cast.
        //
        // ⇒ **so this asserts the INVARIANT now, not a countdown.** A placement
        // authored without a character resolves its art by display name, which is
        // a string matching a sprite by luck; the list being empty is what makes
        // "an enemy placement names its character" a rule rather than a target.
        //
        // ⚠ **it unblocks checklist item 15** — deleting the `presentation_identity`
        // name fallback and making `EnemySpawnSpec::character_id` required — and
        // that is the thing to do next, not another entry here. When the type
        // makes the absence unrepresentable, delete this test with the fallback.
        assert!(
            unnamed.is_empty(),
            "these placements name no character and are resolving art by display \
             name: {unnamed:?}\n\nEvery shipped enemy placement named its \
             character as of 2026-08-13. A new one that does not is either a \
             casting decision nobody made or an authoring slip — and the fix is \
             the `character_id` field, never a wider fallback"
        );
    }

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
