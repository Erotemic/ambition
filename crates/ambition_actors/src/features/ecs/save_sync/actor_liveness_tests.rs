//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod actor_liveness_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

//! ADR 0022: a persisted death flag zeroes HP on load for EVERY
//! persistent actor — including a killed but NEVER-PROVOKED peaceful
//! NPC, the exact case that used to fall through both branches of
//! `sync_ecs_actors_with_save` and respawn alive forever.
use super::*;
use ambition_persistence::save::SandboxSave;
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
        },
    );
    let (seed, _render) = super::super::actor_clusters::ActorClusterSeed::new_peaceful_npc(
        id,
        "Guide",
        aabb,
        &interactable,
        &[],
    );
    let (identity, disposition, combat, intent, cooldowns) =
        crate::features::actor_component_snapshot(&seed, ActorDisposition::Peaceful);
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new(id),
            identity,
            disposition,
            combat,
            intent,
            cooldowns,
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
    app.insert_resource(crate::features::enemies::test_roster());
    let mut save = SandboxSave::default();
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
