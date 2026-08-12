//! CI guard for runtime-projection spatial bugs across every room.
//!
//! The LDtk validator checks LDtk-level data; this checks the lowered
//! `RoomSpec` that the game actually runs: no authored entity center
//! may sit outside the room bounds (it would fall/float forever) and no
//! player spawn may be embedded in a Solid block (the player would load
//! stuck). The `render_room_geometry -- report` example prints the same
//! check for humans; this fails the build if a future room regresses.

use ambition_platformer2d::actors as sb;
use ambition_platformer2d::engine_core::{self as ae, AabbExt};

/// Footprints of placement records of a given kind (families migrated to the
/// single `placements` channel — fable audit F9.2).
fn placement_aabbs(
    room: &sb::rooms::RoomSpec,
    label: &'static str,
    kind: ambition_platformer2d::entity_catalog::placements::PlacementKind,
) -> Vec<(&'static str, ae::Aabb)> {
    room.placements
        .iter()
        .filter(|r| r.kind() == kind)
        .map(|r| (label, r.aabb))
        .collect()
}

fn entity_aabbs(room: &sb::rooms::RoomSpec) -> Vec<(&'static str, ae::Aabb)> {
    use ambition_platformer2d::entity_catalog::placements::PlacementKind;
    let mut v: Vec<(&'static str, ae::Aabb)> = Vec::new();
    v.extend(room.enemy_spawns.iter().map(|e| ("enemy", e.aabb)));
    v.extend(room.boss_spawns.iter().map(|b| ("boss", b.aabb)));
    v.extend(placement_aabbs(
        room,
        "interactable",
        PlacementKind::Interactable,
    ));
    v.extend(placement_aabbs(room, "pickup", PlacementKind::Pickup));
    v.extend(placement_aabbs(room, "chest", PlacementKind::Chest));
    v.extend(placement_aabbs(room, "breakable", PlacementKind::Breakable));
    v.extend(placement_aabbs(room, "hazard", PlacementKind::Hazard));
    v.extend(room.loading_zones.iter().map(|z| ("loading_zone", z.aabb)));
    v
}

/// **The controller gallery demonstrates both directions of composition.**
///
/// ⭐ **this replaces the one-of-each-archetype museum** at sandbox placements
/// `EnemySpawn-0140`..`0146`, which was a row of six bodies whose only reason to
/// exist was that the old ontology made body and AI one thing. Jon's decision
/// (redirect §13) was: delete it, or recast it as a gallery over REAL characters
/// showing that *the same controller policy can drive distinct bodies, and the
/// same body can use distinct policies*. This asserts the recast still says
/// that.
///
/// ⛔ both directions, because either alone is the museum again: six bodies each
/// with its own policy proves nothing about reuse, and one body with six
/// policies proves nothing about portability.
#[test]
fn the_controller_gallery_shows_one_policy_on_many_bodies_and_many_on_one() {
    use std::collections::{BTreeMap, BTreeSet};

    let project = load_project_for_test().expect("sandbox LDtk should load");
    let room_set = project
        .to_room_set(
            &ambition_content::worlds::world_manifest(),
            &ambition_app::composed_ldtk_vocabulary(),
        )
        .expect("room_set should build");
    // (character, policy) for every placement that names both.
    let pairs: Vec<(String, String)> = room_set
        .rooms
        .iter()
        .flat_map(|room| room.enemy_spawns.iter())
        .filter_map(|spawn| {
            let character = spawn.payload.character_id.as_ref()?.as_str().to_string();
            let policy = spawn.payload.brain_profile.as_ref()?.as_str().to_string();
            Some((character, policy))
        })
        .collect();
    assert!(
        !pairs.is_empty(),
        "no placement names a controller policy at all, so the gallery is gone \
         and this guard is measuring nothing"
    );

    let mut bodies_per_policy: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut policies_per_body: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (character, policy) in &pairs {
        bodies_per_policy
            .entry(policy.as_str())
            .or_default()
            .insert(character.as_str());
        policies_per_body
            .entry(character.as_str())
            .or_default()
            .insert(policy.as_str());
    }
    assert!(
        bodies_per_policy.values().any(|bodies| bodies.len() >= 2),
        "no policy drives more than one body, so nothing shows a policy is \
         REUSABLE: {bodies_per_policy:?}"
    );
    assert!(
        policies_per_body
            .values()
            .any(|policies| policies.len() >= 2),
        "no body is driven by more than one policy, so nothing shows a body's \
         driver is a PLACEMENT decision: {policies_per_body:?}"
    );
}

/// **Every archetype row is placed in a level, or named by the engine on
/// purpose.**
///
/// ⛔ **`small_lurker` and `large_colossus` were neither** — authored,
/// validated, iterated by two key lists, asserted about by three tests, and
/// PLACED IN ZERO LEVELS (found 2026-08-11, the same way `sniper_default` was:
/// by counting placements rather than trusting a list). They were deleted, and
/// this is what would have caught them the day they became unreachable.
///
/// ⭐ **it matters most now**, while `character_archetypes.ron` is being
/// retired: a row whose last placement migrates to a character should go red on
/// the change that migrated it, rather than lingering as a body nobody can spawn
/// and a migration nobody needs to do.
///
/// The allowlist is the honest half. Some rows are selected by NAME from Rust
/// rather than placed — a fallback, a protagonist body, a provocation target —
/// and each has to say which it is.
#[test]
fn every_archetype_row_is_placed_somewhere_or_deliberately_code_selected() {
    use ambition_platformer2d::entity_catalog::placements::CharacterBrain;

    /// Rows the engine names directly, with the reason. ⚠ adding to this list is
    /// how a row stops needing a placement — so it is a decision, not a fix.
    const CODE_SELECTED: &[(&str, &str)] = &[(
        "combatant",
        "the fallback every unresolved brain key lands on",
    )];

    let project = load_project_for_test().expect("sandbox LDtk should load");
    let room_set = project
        .to_room_set(
            &ambition_content::worlds::world_manifest(),
            &ambition_app::composed_ldtk_vocabulary(),
        )
        .expect("room_set should build");
    let placed: std::collections::BTreeSet<String> = room_set
        .rooms
        .iter()
        .flat_map(|room| room.enemy_spawns.iter())
        .filter_map(|spawn| match &spawn.payload.brain {
            CharacterBrain::Custom(key) => Some(key.clone()),
            _ => None,
        })
        .collect();
    assert!(
        !placed.is_empty(),
        "no placement names an archetype at all, so this sweep proved nothing"
    );

    let rows: Vec<&str> = ambition_content::enemy_roster::CHARACTER_ROSTER_RON
        .lines()
        .filter_map(|line| {
            let line = line.trim_end();
            let key = line.strip_prefix("    \"")?;
            let (key, rest) = key.split_once('"')?;
            rest.starts_with(": (").then_some(key)
        })
        .collect();
    // ⛔ **THIS SAID `rows.len() > 3` AND THE FILE SHRANK PAST IT** (2026-08-12,
    // deleting `gradient_seeker`). The number was never the invariant — it was a
    // stand-in for *the scan is still reading this file*, written when four rows
    // felt like a floor. But D73's acceptance signal is that
    // `character_archetypes.ron` reaches ZERO rows and is deleted, so any floor
    // above zero turns the campaign's success into a red test, and the obvious
    // "fix" is to lower the number again next time — which is how a guard becomes
    // a chore that eventually gets deleted instead of obeyed.
    //
    // ⭐ the invariant with no number in it: **if the file still declares rows,
    // the scan must see them.** That is exactly the drift this catches (a
    // re-indent, a different quoting style, a nested map), it stays true at three
    // rows, and it stays true at zero — where the whole guard becomes vacuous on
    // purpose, because there is nothing left to place.
    let declared = ambition_content::enemy_roster::CHARACTER_ROSTER_RON
        .matches("\": (")
        .count();
    assert_eq!(
        rows.len(),
        declared,
        "the row scan found {} of the {declared} rows the file declares, so its \
         shape changed under this guard and it is reading less than is there: \
         {rows:?}",
        rows.len(),
    );

    // ⛔ **AND THE ALLOWLIST CANNOT ROT** (2026-08-12). It carried three entries
    // for rows that no longer exist — `cellular_automaton_fighter`,
    // `pirate_heavy`, `pirate_raider`, all migrated to characters and deleted
    // from this file — and two of them explained themselves by naming
    // `hostile_brain_id_for_actor`'s by-name matcher, which is also deleted. A
    // dead exemption is worse than a dead row: it is a REASON, and the next
    // reader believes it.
    //
    // ⇒ every entry must name a row the file actually has. Adding one is a
    // decision (the doc above says so); leaving one behind after the row goes is
    // not a decision at all.
    let stale: Vec<&str> = CODE_SELECTED
        .iter()
        .map(|(named, _)| *named)
        .filter(|named| !rows.contains(named))
        .collect();
    assert!(
        stale.is_empty(),
        "these rows are exempted from needing a placement but no longer EXIST in \
         the file: {stale:?}. Delete the exemption — its reason outlives the row \
         and the next reader believes it."
    );

    let orphans: Vec<&str> = rows
        .into_iter()
        .filter(|key| !placed.contains(*key))
        .filter(|key| !CODE_SELECTED.iter().any(|(named, _)| named == key))
        .collect();
    assert!(
        orphans.is_empty(),
        "archetype rows no level places and no engine path names: {orphans:?}. \
         Delete them, or add them to CODE_SELECTED with the reason they are \
         reachable"
    );
}

/// **A creature that is not your enemy says so on its PLACEMENT.**
///
/// ⛔ **the guard that replaces a deleted field.** `BrainProfile.attacks_player`
/// came across from `ArchetypeSpec` and was removed 2026-08-11 (Jon's redirect
/// §6): a controller policy answers *how do I play this body*, never *who are my
/// enemies*, and it must not carry player-centric vocabulary at all. The giant
/// GNU was the field's motivating case — a mount whose RIDER is the threat — so
/// the moment it is gone, the giant's placement is the only thing standing
/// between a lumbering prop and a hostile one.
///
/// Two terms, both OBSERVED, because either alone passes for the wrong reason:
/// the placement must EXIST (a rename or a lost `character_id` would otherwise
/// make this vacuous) and it must be `Peaceful`.
#[test]
fn the_giant_mount_is_peaceful_by_placement_now_that_no_policy_says_so() {
    use ambition_platformer2d::entity_catalog::placements::SpawnDisposition;
    let project = load_project_for_test().expect("sandbox LDtk should load");
    let room_set = project
        .to_room_set(
            &ambition_content::worlds::world_manifest(),
            &ambition_app::composed_ldtk_vocabulary(),
        )
        .expect("room_set should build");
    let giants: Vec<_> = room_set
        .rooms
        .iter()
        .flat_map(|room| room.enemy_spawns.iter())
        .filter(|spawn| {
            spawn
                .payload
                .character_id
                .as_ref()
                .is_some_and(|id| id.as_str() == "npc_giant_gnu")
        })
        .collect();
    assert!(
        !giants.is_empty(),
        "no placement names `npc_giant_gnu`, so this guard is measuring nothing \
         — the giant moved, was renamed, or lost its character_id"
    );
    for giant in giants {
        assert_eq!(
            giant.payload.disposition,
            Some(SpawnDisposition::Peaceful),
            "the giant GNU placement does not say `Peaceful`, and nothing else \
             says it any more: its policy is `StandStill` with a zero aggro \
             radius, which stops it SEEKING but leaves it hostile, so the mount \
             the scholar rides publishes contact damage at whoever walks past"
        );
    }
}

#[test]
fn no_room_has_out_of_bounds_entities_or_spawn_in_solid() {
    let project = load_project_for_test().expect("sandbox LDtk should load");
    let room_set = project
        .to_room_set(
            &ambition_content::worlds::world_manifest(),
            &ambition_app::composed_ldtk_vocabulary(),
        )
        .expect("room_set should build");
    assert!(
        !room_set.rooms.is_empty(),
        "no rooms loaded — the integrity scan would pass vacuously"
    );

    let mut anomalies: Vec<String> = Vec::new();
    for room in &room_set.rooms {
        let world = &room.world;
        for (label, aabb) in entity_aabbs(room) {
            let c = aabb.center();
            if c.x < 0.0 || c.y < 0.0 || c.x > world.size.x || c.y > world.size.y {
                anomalies.push(format!(
                    "{}: {label} center ({:.1},{:.1}) outside bounds ({:.0},{:.0})",
                    room.id, c.x, c.y, world.size.x, world.size.y
                ));
            }
        }
        let point_in_solid = |p: ae::Vec2| {
            world.blocks.iter().any(|block| {
                matches!(block.kind, ae::BlockKind::Solid)
                    && p.x >= block.aabb.min.x
                    && p.x <= block.aabb.max.x
                    && p.y >= block.aabb.min.y
                    && p.y <= block.aabb.max.y
            })
        };

        // Spawn embedded in a Solid block → player loads stuck.
        if point_in_solid(world.spawn) {
            anomalies.push(format!(
                "{}: spawn ({:.1},{:.1}) embedded in a Solid block",
                room.id, world.spawn.x, world.spawn.y
            ));
        }

        // Small open-space entities embedded in a Solid block →
        // unreachable / stuck. Bosses (large, specially placed) and
        // interactables / loading zones (legitimately mounted on walls)
        // are excluded to avoid false positives.
        let mut embeddable: Vec<(&'static str, ae::Aabb)> = Vec::new();
        embeddable.extend(room.enemy_spawns.iter().map(|e| ("enemy", e.aabb)));
        embeddable.extend(placement_aabbs(
            room,
            "pickup",
            ambition_platformer2d::entity_catalog::placements::PlacementKind::Pickup,
        ));
        embeddable.extend(placement_aabbs(
            room,
            "chest",
            ambition_platformer2d::entity_catalog::placements::PlacementKind::Chest,
        ));
        embeddable.extend(placement_aabbs(
            room,
            "breakable",
            ambition_platformer2d::entity_catalog::placements::PlacementKind::Breakable,
        ));
        for (label, aabb) in embeddable {
            if point_in_solid(aabb.center()) {
                anomalies.push(format!(
                    "{}: {label} center ({:.1},{:.1}) embedded in a Solid block",
                    room.id,
                    aabb.center().x,
                    aabb.center().y
                ));
            }
        }
    }

    assert!(
        anomalies.is_empty(),
        "spatial anomalies found in {} room(s):\n{}",
        anomalies.len(),
        anomalies.join("\n")
    );
}

/// Load the game's merged LDtk project the way a sim entry point does:
/// install the world manifest first — post-R3.2 the engine ships no worlds
/// and panics without a provider-owned manifest.
fn load_project_for_test() -> Result<ambition_platformer2d::actors::ldtk_world::LdtkProject, String>
{
    ambition_platformer2d::actors::ldtk_world::LdtkProject::load_default_for_dev(
        &ambition_content::worlds::world_manifest(),
    )
}
