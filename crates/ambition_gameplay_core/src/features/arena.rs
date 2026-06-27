//! Spectator-duel staging — two brain fighters dueling while the player observes.
//!
//! The reusable mechanism behind the `<<duel>>` dialog command and the headless
//! duel test: it spawns the PCA and the robot on DIFFERENT factions (so the
//! physical damage rule lets them hurt each other — same-faction would be
//! friendly-fire-safe) and configures targeting so the two fighters aim at each
//! other and NEITHER aims at the player. Damage stays physical, so a stray can
//! still catch the observer who walks into the crossfire.
//!
//! This is the "second instance" duel: it spawns its own fighters with their own
//! ids and does not touch the dialog-challenged PCA. Both share the smash brain,
//! so tuning the brain improves both.

use ambition_engine_core as ae;

use crate::combat::components::ActorFaction;
use crate::combat::targeting::FactionRelations;
use crate::features::{SpawnActorKind, SpawnActorRequest};

/// Feature id of the duel's PCA fighter.
pub const DUEL_PCA_ID: &str = "duel_pca";
/// Feature id of the duel's robot fighter.
pub const DUEL_ROBOT_ID: &str = "duel_robot";

/// The two fighter spawn requests for a PCA-vs-robot duel centered at `center`.
/// PCA spawns to the left as `Enemy`, the robot to the right as `Boss` — DIFFERENT
/// factions, the prerequisite for them to damage each other under the physical rule.
pub fn duel_spawn_requests(center: ae::Vec2) -> [SpawnActorRequest; 2] {
    [
        SpawnActorRequest {
            id: DUEL_PCA_ID.to_string(),
            name: "Perfect Cell-ular Automaton".to_string(),
            pos: center + ae::Vec2::new(-75.0, 0.0),
            half_size: ae::Vec2::new(14.0, 23.0),
            faction: ActorFaction::Enemy,
            kind: SpawnActorKind::Enemy {
                brain: ambition_characters::actor::EnemyBrain::Custom(
                    "cellular_automaton_fighter".to_string(),
                ),
            },
        },
        SpawnActorRequest {
            id: DUEL_ROBOT_ID.to_string(),
            name: "Player Robot".to_string(),
            pos: center + ae::Vec2::new(75.0, 0.0),
            half_size: ae::Vec2::new(14.0, 23.0),
            faction: ActorFaction::Boss,
            kind: SpawnActorKind::Enemy {
                brain: ambition_characters::actor::EnemyBrain::Custom("player_robot".to_string()),
            },
        },
    ]
}

/// Configure faction TARGETING for a duel: the two fighters (Enemy + Boss) are
/// hostile to EACH OTHER so they aim at each other. The player's relations are left
/// at the combat baseline ON PURPOSE — the fighters target each other by proximity
/// (stage them near each other, away from the observer), but a player who wades into
/// the duel becomes the nearest target and gets caught, which is the intended "you
/// can get in the way" behavior. Damage is unaffected (physical) — a stray hits any
/// different-faction bystander regardless.
///
/// NOTE: this mutates the GLOBAL relations resource (adds Enemy↔Boss hostility),
/// which lingers after the duel. That's benign in practice (a normal room rarely has
/// both an Enemy and a Boss), but a dedicated arena room should set+restore its
/// relations per-room — the clean scoping is a follow-up.
pub fn apply_duel_relations(relations: &mut FactionRelations) {
    relations.set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
}
