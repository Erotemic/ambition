//! ADR 0022: a persisted death flag zeroes HP on load for EVERY
//! persistent actor — including a killed but NEVER-PROVOKED peaceful
//! NPC, the exact case that used to fall through both branches of
//! `sync_ecs_actors_with_save` and respawn alive forever.
use super::*;
use ambition_persistence::save::AmbitionGameSave;
use bevy::prelude::{App, Update};

fn spawn_guide_npc(app: &mut App, id: &str) -> bevy::prelude::Entity {
    let center = ae::Vec2::new(120.0, 180.0);
    let size = ae::Vec2::new(32.0, 48.0);
    let aabb = ae::Aabb::new(center, size * 0.5);
    let interactable = ambition_interaction::Interactable::new(
        id,
        "Talk",
        aabb,
        ambition_interaction::InteractionKind::Npc {
            character_id: None,
            dialogue_id: Some("hub_guide".into()),
            patrol_radius: 0.0,
            patrol_path_id: None,
            brain_override: None,
        },
    );
    let (seed, _render) = ambition_body_seed::ActorClusterSeed::new_peaceful_npc(
        id,
        "Guide",
        aabb,
        &interactable,
        &[],
    );
    let (identity, disposition, combat) =
        crate::features::actor_component_snapshot(&seed, ActorDisposition::Peaceful);
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new(id),
            identity,
            disposition,
            combat,
            ActorAggression::default(),
            CombatKit::default(),
            ActorInteraction {
                interactable,
                talk_radius: 64.0,
            },
            seed.into_components(),
        ))
        .id()
}

#[test]
fn a_killed_unprovoked_npc_stays_dead_on_load() {
    let mut app = App::new();
    let mut save = AmbitionGameSave::default();
    // The kill hook wrote the DeadStaysDead flag; the NPC was never
    // provoked, so its `npc_<id>_hostile` flag is absent.
    save.data_mut()
        .set_flag(&format!("enemy_{}_dead", "kernel_guide"), true);
    app.insert_resource(save);
    app.add_systems(Update, sync_ecs_actors_with_save);
    let npc = spawn_guide_npc(&mut app, "kernel_guide");
    let alive_npc = spawn_guide_npc(&mut app, "other_guide");

    app.update();

    let dead_hp = app
        .world_mut()
        .query::<&ambition_characters::actor::BodyHealth>()
        .get(app.world(), npc)
        .expect("npc has BodyHealth")
        .health
        .current;
    assert_eq!(
        dead_hp, 0,
        "a killed, never-provoked NPC must load dead (ADR 0022) — this \
         is the fall-through that made NPCs respawn forever"
    );
    let alive_hp = app
        .world_mut()
        .query::<&ambition_characters::actor::BodyHealth>()
        .get(app.world(), alive_npc)
        .expect("npc has BodyHealth")
        .health
        .current;
    assert!(alive_hp > 0, "an unflagged NPC loads alive");
}

/// ⭐⭐ A BODY WHOSE KIND NEVER WRITES A FLAG MUST NOT READ ONE.
///
/// ⛔⛔ THE BUG THIS PINS. The death path writes `enemy_<id>_dead` only for
/// `DeadStaysDead` and `enemy_<id>_dead_until_rest` only for `OnRest`; the other
/// two policies persist nothing. This sweep asked the flag of every actor alive
/// regardless, so a body under `OnRoomReenter` had its liveness decided by a
/// record its own kind never keeps.
///
/// ⭐ AND THAT IS NOT A TIDINESS POINT. A SUMMONED body shares one `config.id`
/// with every instance ever made of it — the pirate's recovery shark is always
/// `smash_ride_shark` — so under the old default the first shark to die wrote a
/// flag, and this sweep (which runs every SIM TICK, not at load) then zeroed the
/// pool of every shark summoned afterwards on its first tick, in that save,
/// permanently. Summons now decline persisted liveness at construction; this arm
/// is the half that rescues a save already carrying the flag.
///
/// ⛔ THE TWO NPCs CARRY THE SAME FLAG and differ ONLY in policy, so a green here
/// cannot be bought by the flag being absent.
#[test]
fn a_body_that_never_persists_its_death_ignores_a_flag_bearing_its_name() {
    use ambition_entity_catalog::placements::RespawnPolicy;

    let mut app = App::new();
    let mut save = AmbitionGameSave::default();
    save.data_mut()
        .set_flag(&format!("enemy_{}_dead", "transient_guide"), true);
    save.data_mut()
        .set_flag(&format!("enemy_{}_dead", "permanent_guide"), true);
    app.insert_resource(save);
    app.add_systems(Update, sync_ecs_actors_with_save);

    let transient = spawn_guide_npc(&mut app, "transient_guide");
    let permanent = spawn_guide_npc(&mut app, "permanent_guide");
    // `spawn_guide_npc` builds both under the default `DeadStaysDead`; one of
    // them declines to persist, the way a summon does.
    app.world_mut()
        .get_mut::<ambition_combat::actor_tuning::ActorConfig>(transient)
        .expect("the actor carries its config")
        .tuning
        .respawn = RespawnPolicy::OnRoomReenter;

    app.update();

    let hp = |app: &mut App, entity: bevy::prelude::Entity| -> i32 {
        app.world_mut()
            .query::<&ambition_characters::actor::BodyHealth>()
            .get(app.world(), entity)
            .expect("npc has BodyHealth")
            .health
            .current
    };
    assert!(
        hp(&mut app, transient) > 0,
        "an actor under `OnRoomReenter` was zeroed by `enemy_transient_guide_dead` \
         — a flag its own death path would never have written. That is how one \
         summoned body's death killed every later summon of the same id."
    );
    // ⛔ THE OTHER HALF, or the arm above is satisfied by a sweep that stopped
    // reading flags at all.
    assert_eq!(
        hp(&mut app, permanent),
        0,
        "a `DeadStaysDead` actor stopped honouring its own death flag, which is \
         ADR 0022's whole contract"
    );
}
