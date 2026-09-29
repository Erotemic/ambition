//! Unit tests for the New Game's fresh-run reducers. The room rebuild, the
//! body placement and the domain restores are the checkpoint restore's, and
//! `session::checkpoint`'s tests cover them.

use super::*;
use ambition_platformer2d_core as ae;

/// The run's save and registries are wiped only inside a fresh-run commit. The
/// first run, without the marker, is a death's restore and is the control.
#[test]
fn a_fresh_run_wipes_the_save_the_registries_and_the_encounters() {
    let mut app = App::new();
    app.init_resource::<AmbitionGameSave>();
    app.init_resource::<EncounterRegistry>();
    app.init_resource::<BossEncounterRegistry>();
    app.init_resource::<QuestRegistry>();
    app.init_resource::<ambition_combat::events::GameplayBanner>();
    app.add_systems(Update, begin_fresh_run);
    {
        let mut save = app.world_mut().resource_mut::<AmbitionGameSave>();
        save.data_mut().set_flag("npc_kira_hostile", true);
        save.data_mut().set_encounter(
            "goblin_encounter",
            ambition_persistence::save_data::PersistedEncounterState::Cleared,
        );
    }
    app.world_mut().resource_mut::<EncounterRegistry>().specs_loaded = true;
    app.world_mut().resource_mut::<BossEncounterRegistry>().specs_loaded = true;
    app.world_mut().resource_mut::<QuestRegistry>().initialized = true;
    let encounter = app
        .world_mut()
        .spawn(ambition_encounter::Encounter::new("goblin_encounter".to_string()))
        .id();

    for fresh in [false, true] {
        if fresh {
            app.insert_resource(FreshRunRestore);
        }
        app.update();
        let save = app.world().resource::<AmbitionGameSave>().data();
        assert_eq!(save.flag("npc_kira_hostile"), !fresh, "fresh = {fresh}");
        assert_eq!(
            save.encounter("goblin_encounter")
                == ambition_persistence::save_data::PersistedEncounterState::Untouched,
            fresh,
        );
        assert_eq!(app.world().resource::<EncounterRegistry>().specs_loaded, !fresh);
        assert_eq!(app.world().resource::<BossEncounterRegistry>().specs_loaded, !fresh);
        assert_eq!(app.world().resource::<QuestRegistry>().initialized, !fresh);
        assert_eq!(app.world().get_entity(encounter).is_ok(), !fresh);
        assert_eq!(
            app.world().resource::<ambition_combat::events::GameplayBanner>().text == "SANDBOX RESET",
            fresh,
        );
    }
}

/// The transient clear runs only inside a fresh-run commit. The same schedule
/// run without the marker is a death's restore, and it clears nothing.
#[test]
fn sandbox_reset_clears_portals_held_items_and_summons() {
    let mut app = App::new();
    app.add_systems(Update, clear_transient_on_sandbox_reset);

    let ground = app
        .world_mut()
        .spawn(ambition_held_items::GroundItem::at_rest(
            ambition_held_items::axe_spec(),
            ae::Vec2::ZERO,
            ae::Vec2::splat(18.0),
        ))
        .id();
    let ally = app
        .world_mut()
        .spawn(crate::abilities::thrown::puppy_slug_gun::PuppySlugAlly)
        .id();
    let player =
        app.world_mut()
            .spawn((
                ambition_platformer2d_shared_tangle::markers::PlayerEntity,
                ambition_characters::brain::ActionSet::default(),
                ambition_characters::brain::action_set::IdentityKit::default(),
                ambition_combat::moveset::ActorMoveset::default(),
                ambition_combat::held_items::HeldItem::new(ambition_held_items::axe_spec()),
            ))
            .id();
    #[cfg(feature = "portal")]
    app.world_mut()
        .entity_mut(player)
        .insert(ambition_portal2d::PortalGun::default());

    // No reset queued → nothing changes.
    app.update();
    assert!(app
        .world()
        .get::<ambition_held_items::GroundItem>(ground)
        .is_some());
    assert!(app
        .world()
        .get::<ambition_combat::held_items::HeldItem>(player)
        .is_some());

    // Committed → transient entities despawn + player held-state stripped.
    app.insert_resource(FreshRunRestore);
    app.update();
    assert!(
        app.world()
            .get::<ambition_held_items::GroundItem>(ground)
            .is_none(),
        "ground item despawned on reset"
    );
    assert!(
        app.world()
            .get::<crate::abilities::thrown::puppy_slug_gun::PuppySlugAlly>(ally)
            .is_none(),
        "summoned ally despawned on reset"
    );
    assert!(
        app.world()
            .get::<ambition_combat::held_items::HeldItem>(player)
            .is_none(),
        "held item removed from player"
    );
    #[cfg(feature = "portal")]
    assert!(
        app.world()
            .get::<ambition_portal2d::PortalGun>(player)
            .is_none(),
        "portal gun removed from player"
    );
}

/// THE ROOM IS ALREADY REBUILT WHEN THIS SYSTEM RUNS, SO IT MAY NOT SWEEP THE
/// ROOM.
///
/// The commit runs `CheckpointDomainApply` after it publishes the start room,
/// so every room-scoped ground item this system can see is one the rebuild
/// AUTHORED from the room's own records. A blanket `With<GroundItem>` sweep
/// despawned exactly those, and a reset taken in a room with an authored pickup
/// rebuilt that room permanently one pickup short of itself. ROOM scope is the
/// line because the room rebuild owns that side.
#[test]
fn the_transient_clear_spares_the_rebuilt_rooms_own_items() {
    let mut app = App::new();
    app.add_systems(Update, clear_transient_on_sandbox_reset);

    // What the reset's room plan just authored: room-scoped, exactly as
    // `spawn_ground_item_resolved_into` builds it (`insert_room_in_session`).
    let authored = app
        .world_mut()
        .spawn((
            RoomScopedEntity,
            ambition_held_items::GroundItem::at_rest(
                ambition_held_items::axe_spec(),
                ae::Vec2::new(64.0, 0.0),
                ae::Vec2::splat(18.0),
            ),
        ))
        .id();
    // Residue of the session that is being thrown away: a weapon dropped by a
    // defeated body is `spawn_session_scoped`, so no room sweep will ever take
    // it back — this system is its only retirement.
    let dropped = app
        .world_mut()
        .spawn(ambition_held_items::GroundItem::at_rest(
            ambition_held_items::axe_spec(),
            ae::Vec2::new(-64.0, 0.0),
            ae::Vec2::splat(18.0),
        ))
        .id();

    app.insert_resource(FreshRunRestore);
    app.update();

    assert!(
        app.world()
            .get::<ambition_held_items::GroundItem>(authored)
            .is_some(),
        "the reset despawned the pickup it had just authored from the room's \
         own records, so the rebuilt room came back short of itself"
    );
    assert!(
        app.world()
            .get::<ambition_held_items::GroundItem>(dropped)
            .is_none(),
        "a session-scoped dropped weapon survived the sandbox reset — nothing \
         else retires one, so sparing it leaks the old attempt into the new game"
    );
}

