//! Tests for hit-event application to ECS actors/bosses/breakables and the
//! death-driven drop/explosion/split spawners.

use super::super::damage_drops::{
    drop_ability_pickup, drop_health_pickup, id_drops_health, spawn_death_explosion,
    spawn_split_offspring,
};
use super::*;
use crate::features::ecs::enemy_component_snapshot;
use ambition_combat::events::{HitMode, HitTarget};
use ambition_boss_encounter::behavior::BossBehaviorProfileExt;
use ambition_characters::actor::BodyHealth;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use bevy::prelude::{App, IntoScheduleConfigs, Update};

/// Register every message the shared feature-hit pipeline writes.
///
/// `apply_feature_hit_events` fans out to sfx / vfx / debris / stimuli / wallet facts, and a Bevy
/// `MessageWriter` for an unregistered message PANICS the system rather than no-opping. One list,
/// one edit.
fn register_hit_pipeline_messages(app: &mut App) {
    app.add_message::<HitEvent>();
    app.add_message::<ambition_combat::hitbox::LandedBodyHit>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<ActorStimulus>();
    app.add_message::<ambition_combat::stocks::BodyKnockedOut>();
    app.add_message::<crate::features::ecs::damage_apply::WalletShieldSpent>();
}

fn spawn_hostile_actor(app: &mut App) -> bevy::prelude::Entity {
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let mut enemy = crate::features::ecs::actor_clusters::ActorClusterSeed::new(
        "kernel_guide".to_string(),
        "Kernel Guide".to_string(),
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Custom("medium_striker".into()),
        &[],
    );
    enemy.health =
        ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(5));
    let (identity, disposition, combat) = enemy_component_snapshot(&enemy);
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new("kernel_guide"),
            CenteredAabb::from_center_size(aabb.center(), aabb.half_size() * 2.0),
            enemy.into_components(),
            ambition_platformer2d_core::movement::MotionModel::default(),
            // Production hostile actors receive this from `EnemyActorBundle`.
            // Keep the shared damage fixture structurally representative so
            // body-generic contact resolution can see it as a `StrikeVictim`.
            crate::features::ActorFaction::Enemy,
            identity,
            disposition,
            combat,
        ))
        .id()
}

#[test]
fn victim_side_enemy_body_hit_does_not_damage_features() {
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    register_hit_pipeline_messages(&mut app);
    app.add_systems(Update, apply_feature_hit_events);

    let actor_entity = spawn_hostile_actor(&mut app);
    let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    app.world_mut().write_message(HitEvent {
        strike_sfx: None,
        volume: event_volume.into(),
        damage: 1,
        source: HitSource::Contact,
        // With one `Contact` cause there is no direction left to hide behind, and a hit with no
        // attacker has no self to exclude, so the fixture was describing a contact nobody made.
        attacker: Some(actor_entity),
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });

    app.update();

    let health = app
        .world()
        .get::<BodyHealth>(actor_entity)
        .expect("hostile actor exists");
    assert_eq!(
        health.health.current, 5,
        "enemy body contact should not damage the enemy that emitted it"
    );
}

/// The attack's authored `strike_sfx` overrides only the sound, so a sword and the victim's
/// default are heard apart without any `is_player` branch selecting the payload.
#[test]
fn an_enemy_victim_reacts_with_its_own_profile_not_the_players() {
    use bevy::ecs::message::Messages;

    fn played_sounds(app: &App) -> Vec<ambition_sfx::SfxId> {
        let msgs = app
            .world()
            .resource::<Messages<ambition_sfx::OwnedSfxMessage>>();
        let mut cursor = msgs.get_cursor();
        cursor
            .read(msgs)
            .filter_map(|m| match m.request {
                ambition_sfx::SfxMessage::Play { id, .. } => Some(id),
                _ => None,
            })
            .collect()
    }
    fn red_hurt_bursts(app: &App) -> usize {
        let msgs = app.world().resource::<Messages<VfxMessage>>();
        let mut cursor = msgs.get_cursor();
        cursor
            .read(msgs)
            .filter(|m| {
                matches!(
                    m,
                    VfxMessage::Burst { color, .. } if *color == [1.0, 0.34, 0.28, 0.88]
                )
            })
            .count()
    }
    fn strike_an_enemy(strike_sfx: Option<ambition_sfx::SfxId>) -> App {
        let mut app = App::new();
        app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
        app.insert_resource(GameplayBanner::default());
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        );
        app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
        register_hit_pipeline_messages(&mut app);
        app.add_systems(Update, apply_feature_hit_events);
        let victim = spawn_hostile_actor(&mut app);
        // A non-lethal (damage 1 vs health 5) hit PRE-RESOLVED to this enemy —
        // an enemy-vs-enemy contact, `HitTarget::Body`.
        app.world_mut().write_message(HitEvent {
            strike_sfx,
            volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0)).into(),
            damage: 1,
            source: HitSource::Contact,
            attacker: None,
            target: HitTarget::Body(victim),
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
        app.update();
        app
    }

    // Authored strike sound (a sword): it plays, and NOT the player's grunt.
    let sword = ambition_sfx::SfxId::new("weapon.sword");
    let by_sword = strike_an_enemy(Some(sword));
    let sounds = played_sounds(&by_sword);
    assert!(sounds.contains(&sword), "the sword's authored sound plays");
    assert!(
        !sounds.contains(&ambition_sfx::ids::PLAYER_DAMAGE),
        "the enemy must NOT play the player's hurt grunt (the CM8 bug)"
    );
    assert_eq!(
        red_hurt_bursts(&by_sword),
        0,
        "an enemy victim throws NO red player-hurt burst"
    );

    // Unauthored: the enemy's own default tick (PLAYER_HIT), still not the grunt.
    let unauthored = strike_an_enemy(None);
    let sounds = played_sounds(&unauthored);
    assert!(
        sounds.contains(&ambition_sfx::ids::PLAYER_HIT),
        "an unauthored hit uses the enemy's default hurt sound"
    );
    assert!(
        !sounds.contains(&ambition_sfx::ids::PLAYER_DAMAGE),
        "still never the player's grunt"
    );
    assert_eq!(red_hurt_bursts(&unauthored), 0);
}

/// Wave-1 follow-up: the player's OUTGOING power slider scales their MELEE
/// damage (projectiles already scale at spawn), through the ONE
/// `apply_feature_hit_events` seam — and it does NOT touch a non-player
/// (`EnemyBody`) melee source, nor is it the separate incoming difficulty scale.
#[test]
fn player_melee_damage_scales_with_the_outgoing_slider() {
    fn damage_dealt(multiplier: f32, source: HitSource) -> i32 {
        damage_dealt_from(multiplier, source, true)
    }

    fn damage_dealt_from(multiplier: f32, source: HitSource, human_controlled: bool) -> i32 {
        let mut app = App::new();
        app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
        app.insert_resource(GameplayBanner::default());
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        );
        app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
        let mut settings = ambition_persistence::settings::UserSettings::default();
        settings.gameplay.player_damage_multiplier = multiplier;
        app.insert_resource(settings);
        register_hit_pipeline_messages(&mut app);
        app.add_systems(Update, apply_feature_hit_events);
        let victim = spawn_hostile_actor(&mut app); // health 5
        let mut attacker = app.world_mut().spawn_empty();
        if human_controlled {
            attacker.insert(ambition_platformer2d_shared_tangle::markers::PlayerEntity);
        }
        let attacker = attacker.id();
        let before = app
            .world()
            .get::<BodyHealth>(victim)
            .unwrap()
            .health
            .current;
        app.world_mut().write_message(HitEvent {
            strike_sfx: None,
            volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0)).into(),
            damage: 2,
            source,
            attacker: Some(attacker),
            target: HitTarget::Body(victim),
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
        app.update();
        let after = app
            .world()
            .get::<BodyHealth>(victim)
            .unwrap()
            .health
            .current;
        before - after
    }

    // Player slash: base 2 × 2.0 slider = 4.
    assert_eq!(
        damage_dealt(2.0, HitSource::Melee),
        4,
        "a strong slider raises the player's own melee damage"
    );
    // Weak slider: 2 × 0.25 = 0.5, floored to the always-≥1 minimum.
    assert_eq!(
        damage_dealt(0.25, HitSource::Melee),
        1,
        "a weak slider lowers it, but a hit still deals at least 1"
    );
    // The SAME slider must NOT scale a non-player melee source.
    assert_eq!(
        damage_dealt(2.0, HitSource::Contact),
        2,
        "the OUTGOING player slider never touches enemy melee"
    );
    // the poison, and the one the source word could never catch. An
    // uncontrolled body swinging the very same melee cause must not be scaled by
    // a HUMAN's difficulty slider. While the gate read `matches!(source,
    // PlayerSlash)` this was unassertable — the spelling WAS the claim — and it
    // is exactly what would have broken silently once one `Melee` covers every
    // swing in the game.
    assert_eq!(
        damage_dealt_from(2.0, HitSource::Melee, false),
        2,
        "a swing by a body no human drives is not the human's outgoing damage"
    );
}

#[test]
fn enemy_charge_crash_is_processed_as_enemy_damage() {
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    register_hit_pipeline_messages(&mut app);
    app.add_systems(Update, apply_feature_hit_events);

    let actor_entity = spawn_hostile_actor(&mut app);
    let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    app.world_mut().write_message(HitEvent {
        strike_sfx: None,
        volume: event_volume.into(),
        damage: 10,
        source: HitSource::Contact,
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });

    app.update();

    let health = app
        .world()
        .get::<BodyHealth>(actor_entity)
        .expect("hostile actor exists");
    assert_eq!(
        health.health.current, 0,
        "enemy charge crash should damage and kill the crashing enemy"
    );
    let health = app
        .world()
        .get::<ambition_characters::actor::BodyHealth>(actor_entity)
        .expect("hostile actor cluster health exists");
    assert!(
        !health.alive(),
        "charge crash should mark the enemy dead through the normal kill path"
    );
}

#[test]
fn enemy_charge_crash_with_an_explicit_attacker_never_credits_the_primary_player() {
    use ambition_combat::moveset::{MovePlayback, SimpleMeleeParams, simple_melee};

    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    register_hit_pipeline_messages(&mut app);
    app.add_systems(Update, apply_feature_hit_events);

    let player = app
        .world_mut()
        .spawn((
            crate::actor::PlayerEntity,
            crate::actor::PrimaryPlayer,
            ambition_characters::actor::BodyCombat::default(),
            MovePlayback::new(simple_melee(&SimpleMeleeParams::default()), 1.0),
        ))
        .id();
    let shell = app.world_mut().spawn_empty().id();
    let victim = spawn_hostile_actor(&mut app);
    let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    app.world_mut().write_message(HitEvent {
        strike_sfx: None,
        volume: event_volume.into(),
        damage: 2,
        source: HitSource::Contact,
        attacker: Some(shell),
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });

    app.update();

    assert_eq!(
        app.world().get::<BodyHealth>(victim).unwrap().current(),
        3,
        "the shell hit still lands on its feature victim"
    );
    let combat = app
        .world()
        .get::<ambition_characters::actor::BodyCombat>(player)
        .unwrap();
    assert_eq!(combat.hitstop_timer, 0.0);
    assert_eq!(combat.hit_flash, 0.0);
    assert!(
        !app.world().get::<MovePlayback>(player).unwrap().landed_hit,
        "a remote shell must not confirm the primary player's move"
    );
}

#[test]
fn player_slash_damages_and_can_kill_a_hostile_actor() {
    // The core attack loop through the unified HitEvent path: a
    // player slash (attacker-side source) reduces a hostile
    // actor's HP, and enough damage routes through the normal kill
    // path. Complements the enemy-side tests above.
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    register_hit_pipeline_messages(&mut app);
    app.add_systems(Update, apply_feature_hit_events);

    let actor_entity = spawn_hostile_actor(&mut app); // HP 5
    let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));

    // First slash: 2 damage → 3 HP, still alive.
    app.world_mut().write_message(HitEvent {
        strike_sfx: None,
        volume: event_volume.into(),
        damage: 2,
        source: HitSource::Melee,
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<BodyHealth>(actor_entity)
            .unwrap()
            .health
            .current,
        3,
        "a 2-damage player slash should bring the 5-HP enemy to 3"
    );
    assert!(
        app.world().get::<BodyHealth>(actor_entity).unwrap().alive(),
        "the enemy should still be alive after one slash"
    );

    app.world_mut()
        .get_mut::<ambition_characters::actor::BodyCombat>(actor_entity)
        .unwrap()
        .damage_invuln_timer = 0.0;

    // Lethal slash: 5 damage → dead through the normal kill path.
    app.world_mut().write_message(HitEvent {
        strike_sfx: None,
        volume: event_volume.into(),
        damage: 5,
        source: HitSource::Melee,
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<BodyHealth>(actor_entity)
            .unwrap()
            .health
            .current,
        0,
        "a lethal slash should bring the enemy to 0 HP"
    );
    assert!(
        !app.world().get::<BodyHealth>(actor_entity).unwrap().alive(),
        "the killed enemy should be marked dead"
    );
}

#[derive(bevy::prelude::Resource, Default)]
struct CapturedBubbles(usize);

fn capture_bubbles(
    mut reader: bevy::prelude::MessageReader<VfxMessage>,
    mut cap: bevy::prelude::ResMut<CapturedBubbles>,
) {
    for m in reader.read() {
        if matches!(m, VfxMessage::SpeechBubble { .. }) {
            cap.0 += 1;
        }
    }
}

/// A talkable peaceful NPC with a provoke accumulator + interaction payload, so
/// its strike branch reaches the on-hit bark. `hp` sets the initial HP so the
/// same body can be exercised alive or as a corpse.
fn spawn_talkable_npc(app: &mut App, hp: i32) -> bevy::prelude::Entity {
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let interactable = ambition_interaction::Interactable::new(
        "alice",
        "Talk",
        aabb,
        ambition_interaction::InteractionKind::Npc {
            character_id: None,
            dialogue_id: None,
            patrol_radius: 0.0,
            patrol_path_id: None,
            brain_override: None,
        },
    );
    let (seed, _spawn) = crate::features::ecs::actor_clusters::ActorClusterSeed::new_peaceful_npc(
        "alice",
        "Alice",
        aabb,
        &interactable,
        &[],
    );
    let (identity, disposition, combat) = crate::features::ecs::actors::actor_component_snapshot(
        &seed,
        crate::features::ActorDisposition::Peaceful,
    );
    let aggression = crate::features::ecs::ActorAggression {
        mode: crate::features::ecs::AggressionMode::RetaliatesWhenHit {
            strike_threshold: crate::features::NPC_HOSTILE_STRIKE_THRESHOLD as u8,
        },
        target: None,
        strikes: 0,
        grudge: None,
    };
    let npc = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new("alice"),
            CenteredAabb::from_center_size(aabb.center(), aabb.half_size() * 2.0),
            aggression,
            crate::features::ecs::CombatKit::default(),
            seed.into_components(),
            ambition_platformer2d_core::movement::MotionModel::default(),
            crate::features::ecs::ActorInteraction {
                interactable,
                talk_radius: crate::features::NPC_TALK_RADIUS,
            },
            identity,
            disposition,
            combat,
        ))
        .id();
    app.world_mut()
        .get_mut::<BodyHealth>(npc)
        .unwrap()
        .health
        .current = hp;
    npc
}

/// A peaceful NPC has no death path of its own (it accumulates strikes and turns
/// hostile), so its strike branch never consulted `alive()`: a body forced to a
/// zero-HP corpse would keep barking a hit line. The structural tangibility gate
/// in `apply_feature_hit_events` (`body_is_corpse` → skip) closes that: a living
/// peaceful NPC still barks when struck, a dead one is silent — a dead thing
/// does not present. Poison: remove the gate and the corpse's SpeechBubble
/// reappears.
#[test]
fn a_struck_peaceful_corpse_is_silent_but_a_living_one_barks() {
    fn strike_and_count_bubbles(hp: i32) -> usize {
        let mut app = App::new();
        app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
        app.insert_resource(GameplayBanner::default());
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        );
        app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
        register_hit_pipeline_messages(&mut app);
        app.init_resource::<CapturedBubbles>();
        app.add_systems(Update, (apply_feature_hit_events, capture_bubbles).chain());

        spawn_talkable_npc(&mut app, hp);
        let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
        app.world_mut().write_message(HitEvent {
            strike_sfx: None,
            volume: event_volume.into(),
            damage: 1,
            source: HitSource::Melee,
            attacker: None,
            target: HitTarget::Volume,
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
        app.update();
        app.world().resource::<CapturedBubbles>().0
    }

    assert_eq!(
        strike_and_count_bubbles(1),
        1,
        "a LIVING peaceful NPC barks a hit line when struck (control)"
    );
    assert_eq!(
        strike_and_count_bubbles(0),
        0,
        "a peaceful CORPSE says nothing when struck — a dead thing does not present"
    );
}

/// `apply_actor_hit` read the DISPOSITION to decide whether a hit takes
/// health, so `ActorDisposition` answered two questions at once: how an actor
/// regards combat, and whether its body can be hurt. A match fighter therefore
/// had to stay `Hostile` merely to be damageable — and two participant-driven
/// fighters hold no AI target, so both stood down to `Peaceful` and neither
/// could hurt the other.
///
/// the same body, struck the same way, twice: a bystander is PROVOKED, and
/// one that is IN A FIGHT takes the blow. Nothing about its brain, its mood or
/// its faction changed between them.
///
/// The third case below is exactly that body.
#[test]
fn a_peaceful_body_in_a_fight_takes_damage_instead_of_barking() {
    fn strike(in_a_fight: bool, ruleset_owns_death: bool) -> (i32, usize) {
        let mut app = App::new();
        app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
        app.insert_resource(GameplayBanner::default());
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        );
        app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
        register_hit_pipeline_messages(&mut app);
        app.init_resource::<CapturedBubbles>();
        app.add_systems(Update, (apply_feature_hit_events, capture_bubbles).chain());

        let body = spawn_talkable_npc(&mut app, 9);
        if ruleset_owns_death {
            app.world_mut()
                .entity_mut(body)
                .insert(ambition_combat::components::RulesetOwnsDeath);
        }
        if in_a_fight {
            app.world_mut()
                .entity_mut(body)
                .insert(ambition_combat::components::ActiveCombatant);
        }
        let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
        app.world_mut().write_message(HitEvent {
            strike_sfx: None,
            volume: event_volume.into(),
            damage: 3,
            source: HitSource::Melee,
            attacker: None,
            target: HitTarget::Volume,
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
        app.update();
        (
            app.world().get::<BodyHealth>(body).unwrap().health.current,
            app.world().resource::<CapturedBubbles>().0,
        )
    }

    let (bystander_hp, bystander_bubbles) = strike(false, false);
    assert_eq!(
        bystander_hp, 9,
        "a town NPC must still be PROVOKED rather than hurt — the \
         strike-then-turn-hostile behaviour is what a peaceful body is for"
    );
    assert_eq!(bystander_bubbles, 1, "and it barks about it");

    let (combatant_hp, combatant_bubbles) = strike(true, true);
    assert!(
        combatant_hp < 9,
        "a body IN a fight takes the blow, and a fighter that cannot be hurt \
         cannot lose: {combatant_hp}"
    );
    assert_eq!(
        combatant_bubbles, 0,
        "and it does not bark a provocation line at somebody it is fighting"
    );

    // THE POISON, and it is the eliminated fighter. Death ownership without participation: the
    // match still owns this body's KO and the body is out of the fight.
    let (eliminated_hp, eliminated_bubbles) = strike(false, true);
    assert_eq!(
        eliminated_hp, 9,
        "a fighter that is OUT is not in the fight, however the match feels \
         about its corpse: {eliminated_hp}"
    );
    assert_eq!(eliminated_bubbles, 1);
}

#[test]
fn a_sustained_overlap_lands_one_hit_per_iframe_window_not_one_per_frame() {
    // With the body-generic `ActorStatus::damage_invuln_timer`, the SAME hit fired twice with the
    // window still hot lands exactly once. (This minimal app runs no integration tick, so the
    // window never decays between the two updates — exactly the sustained-overlap case.)
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    register_hit_pipeline_messages(&mut app);
    app.add_systems(Update, apply_feature_hit_events);

    let actor_entity = spawn_hostile_actor(&mut app); // HP 5
    let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let slash = || HitEvent {
        strike_sfx: None,
        volume: event_volume.into(),
        damage: 2,
        source: HitSource::Melee,
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    };

    app.world_mut().write_message(slash());
    app.update();
    let hp_after_first = app
        .world()
        .get::<BodyHealth>(actor_entity)
        .unwrap()
        .health
        .current;
    assert_eq!(hp_after_first, 3, "first hit lands (5 → 3)");

    // Second identical hit while the i-frame is still hot (no tick decayed it):
    // ignored, so HP is unchanged — the sustained-overlap stream is collapsed.
    app.world_mut().write_message(slash());
    app.update();
    assert_eq!(
        app.world()
            .get::<BodyHealth>(actor_entity)
            .unwrap()
            .health
            .current,
        3,
        "a re-hit within the i-frame window must be ignored (no per-frame stream)"
    );
}

/// Shared setup for the cling-break tests: spawn a hostile actor, make it an
/// adhesive crawler clung to a LEFT wall (outward normal +x), then slash it.
fn slash_clung_surface_walker(cling_breaks_on_hit: bool) -> (App, bevy::prelude::Entity) {
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    register_hit_pipeline_messages(&mut app);
    app.add_systems(Update, apply_feature_hit_events);

    let actor = spawn_hostile_actor(&mut app); // HP 5 — survives one slash
    {
        let mut cfg = app
            .world_mut()
            .get_mut::<super::super::actor_clusters::ActorConfig>(actor)
            .unwrap();
        cfg.tuning.surface_walker = true;
        cfg.tuning.cling_breaks_on_hit = cling_breaks_on_hit;
    }
    {
        // The crawler POLICY with a live attachment — the explicit model the
        // spawn selector installs for `surface_walker` archetypes.
        let mut model = app
            .world_mut()
            .get_mut::<ambition_platformer2d_core::movement::MotionModel>(actor)
            .unwrap();
        *model = ambition_platformer2d_core::movement::MotionModel::AdhesiveCrawler(ae::AdhesiveCrawlerMotion {
            params: ae::CrawlerParams::default(),
            state: ae::CrawlerState::attached(ae::Vec2::new(1.0, 0.0)),
        });
    }
    {
        app.world_mut()
            .get_mut::<crate::actor::BodyGroundState>(actor)
            .unwrap()
            .on_ground = true;
        app.world_mut()
            .get_mut::<crate::features::ActorSurfaceState>(actor)
            .unwrap()
            .surface_normal = ae::Vec2::new(1.0, 0.0);
    }
    app.world_mut().write_message(HitEvent {
        strike_sfx: None,
        volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0)).into(),
        damage: 1,
        source: HitSource::Melee,
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.update();
    (app, actor)
}

#[test]
fn struck_cling_breaker_loses_its_surface_and_falls() {
    // The puppy-slug "panic on hit": a struck surface-walker authored
    // `cling_breaks_on_hit` leaves its surface and peels away along that surface
    // normal. It keeps the last contact normal while airborne; the surface-walk
    // integration reorients it to the active acceleration frame when it next lands.
    let (app, actor) = slash_clung_surface_walker(true);
    let surf = app
        .world()
        .get::<crate::features::ActorSurfaceState>(actor)
        .unwrap();
    assert!(
        !app.world()
            .get::<crate::actor::BodyGroundState>(actor)
            .unwrap()
            .on_ground,
        "a struck cling-breaker should leave its surface and fall"
    );
    assert_eq!(
        surf.surface_normal,
        ae::Vec2::new(1.0, 0.0),
        "detaching preserves the last contact normal until gravity-relative landing"
    );
    let kin = app
        .world()
        .get::<super::super::actor_clusters::BodyKinematics>(actor)
        .unwrap();
    assert!(
        kin.vel.x > 0.0,
        "it peels away along the +x wall normal, got vel {:?}",
        kin.vel
    );
}

#[test]
fn struck_surface_walker_holds_on_when_cling_does_not_break() {
    // Crawlers authored `cling_breaks_on_hit: false` keep clinging when struck —
    // their surface state is untouched by the hit.
    let (app, actor) = slash_clung_surface_walker(false);
    let surf = app
        .world()
        .get::<crate::features::ActorSurfaceState>(actor)
        .unwrap();
    assert!(
        app.world()
            .get::<crate::actor::BodyGroundState>(actor)
            .unwrap()
            .on_ground,
        "a non-breaking crawler keeps its footing"
    );
    assert_eq!(
        surf.surface_normal,
        ae::Vec2::new(1.0, 0.0),
        "and stays oriented to its wall"
    );
}

#[test]
fn player_slash_shatters_a_breakable() {
    // Completes the attacker-side hit matrix: a player slash on a
    // 1-HP breakable shatters it through apply_feature_hit_events.
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    register_hit_pipeline_messages(&mut app);
    app.add_systems(Update, apply_feature_hit_events);

    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 20.0));
    let breakable = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new("crate"),
            FeatureName::new("crate"),
            // The identity the construction executor stamps on every authored
            // placement before its recipe runs. The coin needs it: a drop states
            // the prop it fell out of, or no render family claims it.
            ambition_platformer2d_shared_tangle::sim_id::SimId::placement("crate"),
            CenteredAabb::from_center_size(aabb.center(), aabb.half_size() * 2.0),
            BreakableFeature::new(ambition_interaction::Breakable::new("crate", 1)),
        ))
        .id();
    assert!(
        !app.world()
            .get::<BreakableFeature>(breakable)
            .unwrap()
            .broken()
    );

    app.world_mut().write_message(HitEvent {
        strike_sfx: None,
        volume: aabb.into(),
        damage: 2,
        source: HitSource::Melee,
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.update();

    assert!(
        app.world()
            .get::<BreakableFeature>(breakable)
            .unwrap()
            .broken(),
        "a player slash should shatter a 1-HP breakable"
    );

    // Shattering a crate drops one collectible coin.
    let mut q = app.world_mut().query::<&PickupFeature>();
    let coins = q
        .iter(app.world())
        .filter(|p| matches!(p.kind(), ambition_interaction::PickupKind::Currency { .. }))
        .count();
    assert_eq!(coins, 1, "shattering a crate drops one coin");
}

#[test]
fn enemy_defeat_drops_a_collectible_currency_coin() {
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.add_systems(Update, |mut c: Commands| {
        drop_currency_coin(
            &mut c,
            ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
            &ambition_platformer2d_shared_tangle::sim_id::SimId::placement("goblin_1"),
            "goblin_1",
            ae::Vec2::new(40.0, 50.0),
            ENEMY_BOUNTY,
        );
    });
    app.update();
    let mut q = app.world_mut().query::<(&PickupFeature, &FeatureId)>();
    let rows: Vec<(ambition_interaction::PickupKind, String)> = q
        .iter(app.world())
        .map(|(p, id)| (p.kind().clone(), id.as_str().to_string()))
        .collect();
    assert_eq!(rows.len(), 1, "exactly one coin dropped");
    assert_eq!(rows[0].1, "coin:goblin_1", "coin id is keyed to the enemy");
    assert_eq!(
        rows[0].0,
        ambition_interaction::PickupKind::Currency {
            amount: ENEMY_BOUNTY
        },
        "the drop is a currency coin worth the bounty",
    );
}

#[test]
fn defeated_boss_drops_its_signature_ability() {
    use crate::features::BossBehaviorProfile;
    // Each boss's reward ability is content data (`boss_profiles.ron`):
    // verify the authored pairings and that each resolves to a real catalog
    // item. Read off the RON-loaded profile by id — the engine names none.
    let expect: &[(&str, Option<&str>)] = &[
        ("flying_spaghetti_monster_boss", Some("blink")),
        ("trex_boss", Some("grapple")),
        ("gnu_ton_rider", Some("fireball")),
        ("clockwork_warden", Some("markrecall")),
        ("mockingbird", None),
        ("smirking_behemoth_boss", None),
    ];
    for (id, ability) in expect {
        let profile =
            BossBehaviorProfile::from_data(ambition_boss_encounter::test_boss_catalog(), id);
        assert_eq!(
            profile.reward_ability.as_deref(),
            *ability,
            "{id} reward ability drifted from boss_profiles.ron",
        );
        if let Some(a) = ability {
            assert!(
                crate::items::Item::from_dialog_id(a).is_some(),
                "boss {id} -> ability {a} must be a real catalog item",
            );
        }
    }

    // The drop spawns a single collectible Ability pickup.
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.add_systems(Update, |mut c: Commands| {
        drop_ability_pickup(
            &mut c,
            ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
            &ambition_platformer2d_shared_tangle::sim_id::SimId::placement("trex_boss"),
            "trex_boss",
            ae::Vec2::new(10.0, 20.0),
            "grapple",
            "Grapple",
        );
    });
    app.update();
    let mut q = app.world_mut().query::<&PickupFeature>();
    let kinds: Vec<ambition_interaction::PickupKind> =
        q.iter(app.world()).map(|p| p.kind().clone()).collect();
    assert_eq!(kinds.len(), 1, "one ability pickup dropped");
    assert_eq!(
        kinds[0],
        ambition_interaction::PickupKind::Ability {
            ability_id: "grapple".to_string()
        },
    );
}

#[test]
fn boss_signature_gauntlets_map_to_real_wielded_held_items() {
    use crate::abilities::ranged::{beam, meteor, sentry, shockwave, volley, vortex};
    use crate::abilities::traversal::dive;
    use crate::features::BossBehaviorProfile;
    // Signature gauntlets are content data (`boss_profiles.ron`): each must resolve to a real
    // held-item spec so the dropped GroundItem is pick-up-able. The expected values pin the RON
    // against the ability id consts so the two can't drift apart.
    let expect: &[(&str, Option<&str>)] = &[
        ("trex_boss", Some(shockwave::SHOCKWAVE_ID)),
        ("mockingbird", Some(volley::VOLLEY_ID)),
        ("smirking_behemoth_boss", Some(beam::BEAM_ID)),
        ("mode_collapse_boss", Some(vortex::VORTEX_ID)),
        ("exploding_gradient_boss", Some(sentry::SENTRY_ID)),
        ("overflow_boss", Some(dive::DIVE_ID)),
        ("gnu_ton_rider", Some(meteor::METEOR_ID)),
        ("clockwork_warden", None),
        ("flying_spaghetti_monster_boss", None),
    ];
    let mut gauntlets = 0;
    let mut abilities = 0;
    for (id, gauntlet) in expect {
        let profile =
            BossBehaviorProfile::from_data(ambition_boss_encounter::test_boss_catalog(), id);
        assert_eq!(
            profile.signature_gauntlet.as_deref(),
            *gauntlet,
            "{id} signature gauntlet drifted from boss_profiles.ron",
        );
        if let Some(g) = profile.signature_gauntlet.as_deref() {
            assert!(
                ambition_characters::brain::held_item_by_id(g).is_some(),
                "boss {id} -> gauntlet {g} must be a registered held item",
            );
            gauntlets += 1;
        }
        if profile.reward_ability.is_some() {
            abilities += 1;
        }
    }
    // trex + mockingbird + smirking + mode_collapse + exploding_gradient +
    // overflow + the gnu_ton rider each arm a wielded gauntlet (seven "learn its
    // attack" drops; trex and the rider also grant a catalog ability).
    assert_eq!(gauntlets, 7, "seven bosses drop a signature gauntlet");
    // FSM(blink) + trex(grapple) + gnu(fireball) + clockwork(markrecall).
    assert_eq!(abilities, 4, "four bosses grant a catalog ability");
}

#[test]
fn exploding_mite_blast_is_a_player_damaging_enemy_hitbox() {
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.add_systems(Update, |mut c: Commands| {
        spawn_death_explosion(
            &mut c,
            ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
            Entity::PLACEHOLDER,
            ae::Vec2::new(50.0, 60.0),
        );
    });
    app.update();
    let mut q = app.world_mut().query::<&crate::features::Hitbox>();
    let boxes: Vec<crate::features::Hitbox> = q.iter(app.world()).cloned().collect();
    assert_eq!(boxes.len(), 1, "the mite's death spawns one blast hitbox");
    assert_eq!(
        boxes[0].source,
        ambition_vfx::HitSide::Enemy,
        "enemy side -> the blast damages the player, not other mites (no chain)",
    );
    assert_eq!(boxes[0].damage, EXPLODER_BLAST_DAMAGE);
    if let crate::features::HitboxAnchor::World { center } = boxes[0].anchor {
        assert_eq!(
            center,
            ae::Vec2::new(50.0, 60.0),
            "the blast centers on the corpse"
        );
    } else {
        panic!("the blast should be world-anchored at the death site");
    }
}

#[test]
fn dividing_mite_splits_into_two_hostile_offspring_on_death() {
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    app.add_systems(
        Update,
        |mut c: Commands,
         catalog: bevy::prelude::Res<
            ambition_characters::actor::character_catalog::CharacterCatalog,
        >| {
            spawn_split_offspring(
                &mut c,
                &catalog,
                &Default::default(),
                Some(&crate::character_runtime::fixture_cast(&["npc_puppy_slug"])),
                ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
                "divider_1",
                ae::Vec2::new(100.0, 100.0),
                "npc_puppy_slug",
            );
        },
    );
    app.update();
    let mut q = app.world_mut().query::<&crate::features::ActorFaction>();
    let factions: Vec<crate::features::ActorFaction> = q.iter(app.world()).cloned().collect();
    assert_eq!(
        factions.len(),
        2,
        "a dividing mite splits into exactly two offspring"
    );
    assert!(
        factions
            .iter()
            .all(|f| *f == crate::features::ActorFaction::Enemy),
        "the offspring are hostile (Enemy faction), not player-allies",
    );
}

#[test]
fn enemy_health_drop_is_deterministic_and_spawns_a_heart() {
    // The gate is a pure function of the id, so the headless sim is reproducible.
    assert_eq!(id_drops_health("goblin_42"), id_drops_health("goblin_42"));
    // The drop spawns one collectible Health pickup.
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.add_systems(Update, |mut c: Commands| {
        drop_health_pickup(
            &mut c,
            ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
            &ambition_platformer2d_shared_tangle::sim_id::SimId::placement("any"),
            "any",
            ae::Vec2::ZERO,
            ENEMY_HEALTH_DROP,
        );
    });
    app.update();
    let mut q = app.world_mut().query::<&PickupFeature>();
    let kinds: Vec<ambition_interaction::PickupKind> =
        q.iter(app.world()).map(|p| p.kind().clone()).collect();
    assert_eq!(kinds.len(), 1, "one heart dropped");
    assert!(
        matches!(kinds[0], ambition_interaction::PickupKind::Health { .. }),
        "the drop is a health pickup",
    );
}

// It asked a fixture archetype row for its `held_item_spec()` and asserted a gun-sword came
// back — a test of the row's field resolver. A body's weapon is authored on its CHARACTER
// (`held_item`) and inserted by the road that believes the character, so the surviving claim is
// `drops_held_item`'s: what a body drops is its LIVE `HeldItem`, not a table row.

// ── S3c: body-enforced reactive block ───────────────────────────────────────
//
// The shield is a body capability: the controller only sets `shield_held` (which
// the resolver lands on `status.shield_raised`, gated by `caps.can_shield`); the
// BODY negates a guarded hit from the side it faces. These drive the REAL actor
// damage system (`apply_feature_hit_events` → `apply_actor_hit`), so they prove
// the enforcement, not a mocked rule. A possessing human and an AI brain block
// identically because both only feed `shield_held` (invariants I2/I3).

/// Spawn a hostile actor with the shield capability, body facing +x (right),
/// 5 HP, with its guard raised iff `shield_raised`.
fn spawn_shielding_actor(app: &mut App, shield_raised: bool) -> bevy::prelude::Entity {
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let mut enemy = crate::features::ecs::actor_clusters::ActorClusterSeed::new(
        "guard".to_string(),
        "Guard".to_string(),
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Custom(
            "cellular_automaton_fighter".into(),
        ),
        &[],
    );
    enemy.health =
        ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(5));
    enemy.kin.facing = 1.0;
    // The damage path reads the body's ONE shield component (`BodyShieldState`) —
    // set it directly, the way the pipeline shield limb would. (The `shield`
    // movement capability gates whether the pipeline RAISES the guard; the
    // resolver itself only reads the resulting `shield.active`.)
    enemy.body.0.shield.active = shield_raised;
    let (identity, disposition, combat) = enemy_component_snapshot(&enemy);
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new("guard"),
            CenteredAabb::from_center_size(aabb.center(), aabb.half_size() * 2.0),
            enemy.into_components(),
            ambition_platformer2d_core::movement::MotionModel::default(),
            identity,
            disposition,
            combat,
        ))
        .id()
}

fn shield_test_app() -> App {
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    register_hit_pipeline_messages(&mut app);
    app.add_systems(Update, apply_feature_hit_events);
    app
}

/// A player slash whose hitbox is centered at `center` (must overlap the actor's
/// body AABB to land), dealing `damage`.
/// The shove a player slash carries, spelled the one way knockback is spelled.
fn slash_knockback(center: ae::Vec2, dir: f32) -> ambition_combat::events::HitKnockback {
    ambition_combat::events::HitKnockback {
        // An ordinary hit: it stuns.
        reaction: ambition_platformer2d_core::hit_response::HitReaction::Strike,
        dir,
        magnitude: ambition_combat::events::HitKnockbackMagnitude::FeelScale(1.0),
        source_pos: center,
        impact_pos: center,
        launch_dir: None,
        follow: None,
    }
}

fn slash_at(center: ae::Vec2, damage: i32) -> HitEvent {
    HitEvent {
        strike_sfx: None,
        volume: ae::Aabb::new(center, ae::Vec2::new(32.0, 40.0)).into(),
        damage,
        source: HitSource::Melee,
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        // Same resolution as before: side +1, standard feel strength.
        knockback: Some(slash_knockback(center, 1.0)),
        ignored_targets: Vec::new(),
    }
}

fn actor_hp(app: &App, entity: bevy::prelude::Entity) -> i32 {
    app.world()
        .get::<BodyHealth>(entity)
        .expect("actor exists")
        .health
        .current
}

#[test]
fn raised_shield_negates_a_hit_from_the_faced_side() {
    let mut app = shield_test_app();
    let actor = spawn_shielding_actor(&mut app, true);
    // Body faces +x; the slash comes from the front (+x). The hitbox is wide
    // enough to overlap the body at the origin while its center sits forward.
    app.world_mut()
        .write_message(slash_at(ae::Vec2::new(14.0, 0.0), 2));
    app.update();
    assert_eq!(
        actor_hp(&app, actor),
        5,
        "a guarded hit from the faced side must be fully negated by the body"
    );
}

#[test]
fn a_lowered_shield_does_not_block() {
    let mut app = shield_test_app();
    let actor = spawn_shielding_actor(&mut app, false);
    app.world_mut()
        .write_message(slash_at(ae::Vec2::new(14.0, 0.0), 2));
    app.update();
    assert_eq!(
        actor_hp(&app, actor),
        3,
        "with the guard down the same front hit lands full damage"
    );
}

#[test]
fn a_raised_shield_does_not_guard_the_back() {
    let mut app = shield_test_app();
    let actor = spawn_shielding_actor(&mut app, true);
    // Body faces +x; this hit comes from BEHIND (-x). You can't guard your back.
    app.world_mut()
        .write_message(slash_at(ae::Vec2::new(-14.0, 0.0), 2));
    app.update();
    assert_eq!(
        actor_hp(&app, actor),
        3,
        "a hit from behind the guard still lands — the block is directional"
    );
}

// ── §A2 step 6: a struck actor rides the shared knockback resolution ─────────

/// A knockback-carrying hit (an aggressor swing pre-resolved to an actor
/// victim) launches the actor through `resolved_body_knockback_velocity` —
/// away from the source along its frame's side, rising against its gravity —
/// exactly the resolution a player victim gets.
#[test]
fn a_knockback_carrying_hit_launches_the_actor_like_a_player() {
    let mut app = shield_test_app();
    let victim = spawn_hostile_actor(&mut app);
    let feel = ambition_combat::feel::Platformer2dFeelTuningMonolith::default();
    // Attacker to the LEFT of the victim (victim at origin): expect a launch
    // toward +x with the feel-tuned enemy knockback, rising (world -y).
    app.world_mut().write_message(HitEvent {
        strike_sfx: None,
        volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(40.0, 50.0)).into(),
        damage: 2,
        source: HitSource::Melee,
        attacker: None,
        target: HitTarget::Body(victim),
        mode: HitMode::Knockback,
        knockback: Some(ambition_combat::events::HitKnockback {
            // An ordinary hit: it stuns.
            reaction: ambition_platformer2d_core::hit_response::HitReaction::Strike,
            dir: 1.0,
            magnitude: ambition_combat::events::HitKnockbackMagnitude::FeelScale(1.0),
            source_pos: ae::Vec2::new(-40.0, 0.0),
            impact_pos: ae::Vec2::ZERO,
            launch_dir: None,
            follow: None,
        }),
        ignored_targets: Vec::new(),
    });
    app.update();
    let kin = app
        .world()
        .get::<super::super::actor_clusters::BodyKinematics>(victim)
        .unwrap();
    let expected = ae::Vec2::new(feel.enemy_knockback_x, -feel.enemy_knockback_y);
    assert!(
        (kin.vel - expected).length() < 1e-3,
        "actor knockback should be the shared feel-tuned resolution, got {:?} want {expected:?}",
        kin.vel
    );
    // §A2 step 7: the launch also arms the shared stagger set on `BodyCombat`,
    // exactly like the player's knockback path.
    let combat = app.world().get::<BodyCombat>(victim).unwrap();
    assert!(
        combat.hitstun_timer > 0.0 && combat.recoil_lock_timer > 0.0 && combat.hitstop_timer > 0.0,
        "a knockback hit arms hitstun/recoil/hitstop on the struck body: {combat:?}"
    );
}

/// GETTING HIT TAKES THE HANG, on the road a platform fighter's roster
/// actually travels.
///
/// Jon, 2026-08-24: *"A character can just stay on the ledge, and there is no
/// way to knock them off. If you get hit you should fall off the ledge at
/// least."*
///
/// ⛔ THE RULE WAS NEVER MISSING — only this caller was. `knock_off_ledge` has
/// its own unit test in the kernel and both of the PLAYER's `HitMode::Knockback`
/// arms call it; the generic actor road did not, and in the arena every fighter
/// is an actor. So the pure rule was green while an edge-guard could not
/// dislodge anybody.
///
/// The re-grab lockout is asserted beside the hang because they are one
/// operation: dropping the hang without arming it re-latches the body on the
/// next frame and the hit reads as nothing at all.
///
/// ⛔⛔ AND BOTH KNOCKBACKS ARE MEASURED. A damage-only hit used to skip the
/// shared reaction outright on this road — the whole call sat inside `if let
/// Some(k) = knockback` — so a hazard, a chip or a poison tick left the hang
/// standing. Only the launching half was ever exercised, and the launching half
/// is the easy one.
#[test]
fn a_hit_knocks_a_hanging_actor_off_the_ledge() {
    for knockback in [
        Some(ambition_combat::events::HitKnockback {
            // An ordinary hit: it stuns.
            reaction: ambition_platformer2d_core::hit_response::HitReaction::Strike,
            dir: 1.0,
            magnitude: ambition_combat::events::HitKnockbackMagnitude::FeelScale(1.0),
            source_pos: ae::Vec2::new(-40.0, 0.0),
            impact_pos: ae::Vec2::ZERO,
            launch_dir: None,
            follow: None,
        }),
        None,
    ] {
        a_hit_knocks_a_hanging_actor_off_the_ledge_with(knockback);
    }
}

fn a_hit_knocks_a_hanging_actor_off_the_ledge_with(
    knockback: Option<ambition_combat::events::HitKnockback>,
) {
    let launched = knockback.is_some();
    let mut app = shield_test_app();
    let victim = spawn_hostile_actor(&mut app);
    // Hang it. The kernel writes this from contact geometry; here it is placed
    // directly, which is what makes the hit the only variable.
    {
        let mut model = app
            .world_mut()
            .get_mut::<ambition_platformer2d_core::movement::MotionModel>(victim)
            .expect("the actor carries a motion model");
        *model = ae::MotionModel::axis_swept(ae::AxisSweptParams::default());
        let ae::MotionModel::AxisSwept(axis) = &mut *model else {
            unreachable!("just written")
        };
        axis.state.ledge_grab = Some(ae::LedgeGrabState::hanging(ae::LedgeContact {
            wall_normal_x: 1.0,
            anchor: ae::Vec2::ZERO,
            climb_target: ae::Vec2::new(8.0, -16.0),
        }));
    }

    app.world_mut().write_message(HitEvent {
        strike_sfx: None,
        volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(40.0, 50.0)).into(),
        damage: 2,
        source: HitSource::Melee,
        attacker: None,
        target: HitTarget::Body(victim),
        mode: HitMode::Knockback,
        knockback,
        ignored_targets: Vec::new(),
    });
    app.update();

    let model = app
        .world()
        .get::<ambition_platformer2d_core::movement::MotionModel>(victim)
        .expect("the actor carries a motion model");
    let ae::MotionModel::AxisSwept(axis) = model else {
        unreachable!("the fixture installed one")
    };
    assert!(
        axis.state.ledge_grab.is_none(),
        "a struck fighter kept its ledge hang — an edge-guard cannot dislodge anybody"
    );
    let ledge = app
        .world()
        .get::<ae::BodyLedgeState>(victim)
        .expect("the actor carries the ledge cluster");
    assert!(
        ledge.release_cooldown > 0.0,
        "the hang was dropped without arming the re-grab lockout, so the body \
         re-latches on the next frame and the hit reads as nothing \
         (launched: {launched})"
    );
}

/// ⛔⛔ A HIT GIVES BACK THE AIR DODGE AND **NOT** THE DOUBLE JUMP.
///
/// This test used to assert the opposite, and the opposite is what the code did.
/// The reasoning that put it there was: the player road refreshed all three air
/// resources after a hit, the actor road did not, therefore "a hit refreshes the
/// air budget" is a fact of being hit. It is not — it was one road's overreach,
/// generalised into engine law by a unification pass.
///
/// The genre's rule is that a spent double jump STAYS spent through an ordinary
/// edge-guard hit; taking somebody's second jump is a thing you do to them, and
/// a hit that handed it straight back would delete the reason to. The jump
/// returns from a cause that re-seats the body — landing, catching the ledge,
/// being grabbed, a respawn — and Ambition's traversal dash is its own
/// capability that no Smash-shaped hit reaction recharges at all.
///
/// ⭐ ALL THREE ARE ASSERTED, because the failure this replaces was a rule that
/// swept up two resources it never named.
#[test]
fn a_hit_returns_the_air_dodge_and_leaves_the_double_jump_spent() {
    let mut app = shield_test_app();
    let victim = spawn_hostile_actor(&mut app);
    {
        let mut model = app
            .world_mut()
            .get_mut::<ambition_platformer2d_core::movement::MotionModel>(victim)
            .expect("the actor carries a motion model");
        *model = ae::MotionModel::axis_swept(ae::AxisSweptParams::default());
    }
    // ⭐ GRANTED, not assumed. `air_jump_count` returns 0 without the ability
    // whatever the tuning authors, so "the jump stays spent" would hold for a
    // body that never had one — a test that cannot fail.
    {
        let mut abilities = app
            .world_mut()
            .get_mut::<ae::BodyAbilities>(victim)
            .expect("the actor carries the ability cluster");
        abilities.abilities.double_jump = true;
        abilities.abilities.dash = true;
    }
    let authored = {
        let world = app.world();
        world
            .get::<ae::BodyAbilities>(victim)
            .unwrap()
            .abilities
            .air_jump_count(
                world
                    .get::<ambition_platformer2d_core::movement::MotionModel>(victim)
                    .expect("just written")
                    .air_jumps(),
            )
    };
    let dash_charges = app
        .world()
        .get::<ae::BodyAbilities>(victim)
        .unwrap()
        .abilities
        .dash_charge_count();
    assert!(
        authored > 0 && dash_charges > 0,
        "the fixture grants neither resource, so nothing below can fail"
    );
    {
        let mut jump = app
            .world_mut()
            .get_mut::<ae::BodyJumpState>(victim)
            .expect("the actor carries the jump cluster");
        jump.air_jumps_available = 0;
        let mut dodge = app
            .world_mut()
            .get_mut::<ae::BodyDodgeState>(victim)
            .expect("the actor carries the dodge cluster");
        dodge.air_dodge_spent = true;
        let mut dash = app
            .world_mut()
            .get_mut::<ae::BodyDashState>(victim)
            .expect("the actor carries the dash cluster");
        dash.charges_available = 0;
    }

    app.world_mut().write_message(HitEvent {
        strike_sfx: None,
        volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(40.0, 50.0)).into(),
        damage: 2,
        source: HitSource::Melee,
        attacker: None,
        target: HitTarget::Body(victim),
        mode: HitMode::Knockback,
        knockback: Some(ambition_combat::events::HitKnockback {
            // An ordinary hit: it stuns.
            reaction: ambition_platformer2d_core::hit_response::HitReaction::Strike,
            dir: 1.0,
            magnitude: ambition_combat::events::HitKnockbackMagnitude::FeelScale(1.0),
            source_pos: ae::Vec2::new(-40.0, 0.0),
            impact_pos: ae::Vec2::ZERO,
            launch_dir: None,
            follow: None,
        }),
        ignored_targets: Vec::new(),
    });
    app.update();

    assert!(
        !app.world()
            .get::<ae::BodyDodgeState>(victim)
            .expect("the actor carries the dodge cluster")
            .air_dodge_spent,
        "the air dodge stayed spent through a hit — a launched fighter with no \
         evade has no answer to the follow-up"
    );
    assert_eq!(
        app.world()
            .get::<ae::BodyJumpState>(victim)
            .expect("the actor carries the jump cluster")
            .air_jumps_available,
        0,
        "a hit handed the double jump back, which deletes the point of taking it"
    );
    assert_eq!(
        app.world()
            .get::<ae::BodyDashState>(victim)
            .expect("the actor carries the dash cluster")
            .charges_available,
        0,
        "a hit recharged the traversal dash, which is not a platform-fighter \
         resource and has nothing to do with being struck"
    );
}

/// A heavy attacker hits heavy because of WHAT IT IS, not what its hit is
/// called. `boss_hit` used to be `matches!(source, BossBody | BossAttack)` — a
/// source-specific formula for a fact about the striker, and one that could only
/// ever be true for a body the cause vocabulary happened to have a word for.
///
/// Here the very same `EnemyAttack` source lands twice, from two attackers, and
/// the hitstun differs — so the fact is being read off the attacker. The second
/// half is the poison: an ordinary attacker must NOT get the heavy launch, or
/// the query is matching everything and the assertion above proves nothing.
#[test]
fn a_heavy_attacker_is_read_off_the_attacker_not_the_hit_source() {
    let feel = ambition_combat::feel::Platformer2dFeelTuningMonolith::default();
    assert!(
        feel.boss_hitstun_time > feel.enemy_hitstun_time,
        "the fixture only distinguishes the two paths if the tuning does"
    );

    let hitstun_from = |heavy: bool| {
        let mut app = shield_test_app();
        let victim = spawn_hostile_actor(&mut app);
        let attacker = if heavy {
            app.world_mut()
                .spawn(ambition_boss_encounter::BossConfig {
                    id: "heavy".into(),
                    name: "Heavy".into(),
                    spawn: ae::Vec2::ZERO,
                    brain: ambition_entity_catalog::placements::BossBrain::Dormant,
                    behavior: crate::features::BossBehaviorProfile::generic(
                        ambition_boss_encounter::test_boss_catalog(),
                        "heavy",
                    ),
                })
                .id()
        } else {
            app.world_mut().spawn_empty().id()
        };
        let center = ae::Vec2::ZERO;
        app.world_mut().write_message(HitEvent {
            strike_sfx: None,
            volume: ae::Aabb::new(center, ae::Vec2::new(32.0, 40.0)).into(),
            damage: 1,
            // the SAME cause in both runs. If the vocabulary were still
            // deciding, both would land identically.
            source: HitSource::Melee,
            attacker: Some(attacker),
            target: HitTarget::Body(victim),
            mode: HitMode::Knockback,
            knockback: Some(slash_knockback(center, 1.0)),
            ignored_targets: Vec::new(),
        });
        app.update();
        app.world()
            .get::<BodyCombat>(victim)
            .expect("victim exists")
            .hitstun_timer
    };

    let heavy = hitstun_from(true);
    let ordinary = hitstun_from(false);
    assert!(
        (heavy - feel.boss_hitstun_time).abs() < 1e-4,
        "a boss-class attacker stuns for the heavy duration, got {heavy}"
    );
    assert!(
        (ordinary - feel.enemy_hitstun_time).abs() < 1e-4,
        "an ordinary attacker must not inherit the heavy duration, got {ordinary}"
    );
}

/// A slash's knockback rides the shared resolution: side from the payload's
/// `dir`, strength from its magnitude.
#[test]
fn a_slash_knockback_rides_the_shared_resolution() {
    let mut app = shield_test_app();
    let victim = spawn_hostile_actor(&mut app);
    let feel = ambition_combat::feel::Platformer2dFeelTuningMonolith::default();
    // Slash volume centered on the victim (side derivation degenerates) with a
    // -x impulse: the stored dir carries the launch side.
    app.world_mut()
        .write_message(slash_at(ae::Vec2::new(0.0, 0.0), 1));
    app.update();
    let kin = app
        .world()
        .get::<super::super::actor_clusters::BodyKinematics>(victim)
        .unwrap();
    // `slash_at` attaches dir +1 at standard feel strength.
    let expected = ae::Vec2::new(feel.enemy_knockback_x, -feel.enemy_knockback_y);
    assert!(
        (kin.vel - expected).length() < 1e-3,
        "slash knockback should ride the shared resolution, got {:?} want {expected:?}",
        kin.vel
    );
}

// ── S3e: relational actor-vs-actor damage application ────────────────────────

/// A `HitTarget::Body(victim)` event (the pre-resolved actor-vs-actor hit an
/// Enemy/Boss swing emits) damages EXACTLY that actor, even though its source is
/// the victim-side `EnemyAttack` — and never spills onto other overlapping actors.
#[test]
fn an_actor_targeted_hit_damages_only_the_named_actor() {
    let mut app = shield_test_app();
    // Two hostile actors at the same spot (overlapping). HP 5 each.
    let victim = spawn_hostile_actor(&mut app);
    let bystander = spawn_hostile_actor(&mut app);
    app.world_mut().write_message(HitEvent {
        strike_sfx: None,
        volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(40.0, 50.0)).into(),
        damage: 2,
        // Victim-side source, yet the Actor target routes it to the actor consumer.
        source: HitSource::Melee,
        attacker: None,
        target: HitTarget::Body(victim),
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.update();
    assert_eq!(
        actor_hp(&app, victim),
        3,
        "the named actor takes the relational hit"
    );
    assert_eq!(
        actor_hp(&app, bystander),
        5,
        "an overlapping non-target actor is untouched (pre-resolved, not broadcast)"
    );
}

/// The player-melee one-hit-per-target dedup must PERSIST on the attacker's
/// `MovePlayback` (the moveset read-model swing is wiped each frame). This drives
/// `apply_feature_hit_events` directly: a `PlayerSlash` Volume hit whose attacker
/// carries a live melee move must (a) land, and (b) fold the struck target's key
/// onto `MovePlayback.hit_targets` so the next tick's emit ignores it.
#[test]
fn a_player_slash_folds_the_struck_target_onto_the_move_accumulator() {
    use ambition_combat::moveset::{MovePlayback, SimpleMeleeParams, simple_melee};
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    register_hit_pipeline_messages(&mut app);
    app.add_systems(Update, apply_feature_hit_events);

    let attacker = app
        .world_mut()
        .spawn(MovePlayback::new(
            simple_melee(&SimpleMeleeParams::default()),
            1.0,
        ))
        .id();
    let enemy = spawn_hostile_actor(&mut app); // HP 5, box at origin
    let volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    app.world_mut().write_message(HitEvent {
        strike_sfx: None,
        volume: volume.into(),
        damage: 2,
        source: HitSource::Melee,
        attacker: Some(attacker),
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.update();

    assert_eq!(
        app.world().get::<BodyHealth>(enemy).unwrap().health.current,
        3,
        "the slash lands (5 -> 3)"
    );
    let acc = app
        .world()
        .get::<MovePlayback>(attacker)
        .unwrap()
        .hit_targets
        .clone();
    assert!(
        acc.iter().any(|k| k.starts_with("enemy:")),
        "the struck target must be folded onto MovePlayback.hit_targets so the \
         next active tick ignores it; got {acc:?}"
    );
}

/// END-TO-END isolation: a moveset player's FollowOwner strike stays live across
/// multiple active ticks, but the direct body-victim resolver must emit exactly
/// ONE targeted hit. Victim i-frames are cleared each tick so the strike's own
/// `HitboxHits` memory is what prevents the sustained overlap from draining the
/// enemy every frame.
#[test]
fn a_moveset_player_strike_hits_a_target_once_across_a_multi_tick_window() {
    use ambition_combat::moveset::{
        MovePlayback, MovesetMelee, SimpleMeleeParams, project_moveset_melee_to_body_melee,
        simple_melee,
    };
    use bevy::prelude::IntoScheduleConfigs;
    fn clear_iframes(mut q: bevy::prelude::Query<&mut ambition_characters::actor::BodyCombat>) {
        for mut c in &mut q {
            c.damage_invuln_timer = 0.0;
        }
    }
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    register_hit_pipeline_messages(&mut app);
    app.add_systems(
        Update,
        (
            clear_iframes,
            project_moveset_melee_to_body_melee,
            crate::features::apply_hitbox_damage,
            apply_feature_hit_events,
        )
            .chain(),
    );

    let player = app
        .world_mut()
        .spawn((
            MovePlayback::new(simple_melee(&SimpleMeleeParams::default()), 1.0),
            MovesetMelee,
            crate::features::BodyMelee::default(),
            ambition_platformer2d_core::BodyKinematics {
                pos: ae::Vec2::ZERO,
                size: ae::Vec2::new(20.0, 40.0),
                facing: 1.0,
                ..Default::default()
            },
            ambition_platformer2d_core::movement::MotionModel::default(),
            ambition_platformer2d_core::CenteredAabb::from_center_size(
                ae::Vec2::ZERO,
                ae::Vec2::new(20.0, 40.0),
            ),
        ))
        .id();
    let enemy = spawn_hostile_actor(&mut app); // HP 5
    let enemy_center = ae::Vec2::new(50.0, 0.0);
    app.world_mut()
        .get_mut::<CenteredAabb>(enemy)
        .expect("hostile fixture publishes a body box")
        .center = enemy_center;
    app.world_mut()
        .get_mut::<ae::BodyKinematics>(enemy)
        .expect("hostile fixture publishes body kinematics")
        .pos = enemy_center;

    let hitbox = ambition_combat::strike::Hitbox {
        // An ordinary hit, not a gust.
        strike_sfx: None,
        owner: player,
        source: ambition_vfx::HitSide::Player,
        anchor: ambition_combat::strike::HitboxAnchor::FollowOwner {
            local_offset: ae::Vec2::new(32.0, 0.0),
        },
        half_extent: ae::Vec2::new(20.0, 30.0),
        shape: None,
        facing: 1.0,
        damage: 2,
        knockback: ambition_combat::strike::HitboxKnockback::FeelScale(0.0),
        launch_dir: None,
        frame_down: ae::Vec2::new(0.0, 1.0),
        reaction: None,
    };
    let player_body = app.world().get::<CenteredAabb>(player).unwrap().aabb();
    let enemy_body = app.world().get::<CenteredAabb>(enemy).unwrap().aabb();
    assert!(
        !player_body.strict_intersects(enemy_body),
        "the regression must not be satisfiable by body-to-body contact"
    );
    assert!(
        hitbox
            .world_volume(ae::Vec2::ZERO)
            .intersects_aabb(enemy_body),
        "the attack volume must reach the separated victim body"
    );
    app.world_mut()
        .spawn((hitbox, ambition_combat::strike::HitboxHits::default()));

    for _ in 0..6 {
        app.update();
    }
    assert_eq!(
        app.world().get::<BodyHealth>(enemy).unwrap().health.current,
        3,
        "a separated-body strike must land from its attack volume exactly once (5 -> 3)"
    );
}

#[test]
fn a_lethal_hit_kills_without_speaking_a_hit_bark() {
    // Reproduces the observed "it barks when it's dead". A dying body should present its death
    // (the Death SFX + burst + debris), not a conversational "ow!". Non-lethal hits still bark.
    // NOTE: striking an ALREADY-dead corpse was never the culprit here — `resolve_body_hit` has
    // always dropped hits on a zero-HP body, so that path is silent with or without this
    // change; the reproducible bark was the death FRAME. Poison: drop the `&& !killed` guard in
    // `apply_actor_hit` and the lethal case barks again.
    fn hit_and_count_bubbles(start_hp: i32, damage: i32) -> (usize, bool) {
        let mut app = App::new();
        app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
        app.insert_resource(GameplayBanner::default());
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        );
        app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
        let mut banter = crate::features::banter::CombatBanterRegistry::default();
        banter.set_hit_barks("Kernel Guide", vec!["ow!", "argh!", "stop!"]);
        app.insert_resource(banter);
        register_hit_pipeline_messages(&mut app);
        app.init_resource::<CapturedBubbles>();
        app.add_systems(Update, (apply_feature_hit_events, capture_bubbles).chain());
        let e = spawn_hostile_actor(&mut app);
        app.world_mut()
            .get_mut::<BodyHealth>(e)
            .unwrap()
            .health
            .current = start_hp;
        app.world_mut().write_message(HitEvent {
            strike_sfx: None,
            volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0)).into(),
            damage,
            source: HitSource::Melee,
            attacker: None,
            target: HitTarget::Volume,
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
        app.update();
        (
            app.world().resource::<CapturedBubbles>().0,
            app.world().get::<BodyHealth>(e).unwrap().alive(),
        )
    }
    // Non-lethal (control): the enemy survives and barks a hit line.
    let (bubbles, alive) = hit_and_count_bubbles(5, 2);
    assert!(alive, "a 2-damage hit leaves the 5-HP enemy alive");
    assert_eq!(bubbles, 1, "a surviving struck enemy barks a hit line");
    // Lethal: the enemy dies WITHOUT speaking a hit line.
    let (bubbles, alive) = hit_and_count_bubbles(2, 5);
    assert!(!alive, "a 5-damage hit kills the 2-HP enemy");
    assert_eq!(
        bubbles, 0,
        "a dying enemy does not speak a hit line — it presents its death instead"
    );
}

#[test]
fn a_peaceful_actor_owns_one_victim_side_hit_sound() {
    use bevy::ecs::message::Messages;

    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    register_hit_pipeline_messages(&mut app);
    app.add_systems(Update, apply_feature_hit_events);

    spawn_talkable_npc(&mut app, 1);
    app.world_mut().write_message(HitEvent {
        strike_sfx: None,
        volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0)).into(),
        damage: 1,
        source: HitSource::Melee,
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.update();

    let messages = app
        .world()
        .resource::<Messages<ambition_sfx::OwnedSfxMessage>>();
    let mut cursor = messages.get_cursor();
    let plays = cursor
        .read(messages)
        .filter(|message| matches!(message.request, ambition_sfx::SfxMessage::Play { .. }))
        .count();
    assert_eq!(plays, 1, "the peaceful victim emits exactly one hit sound");
}

/// A body that left the world does not come back where it fell.
///
/// `RespawnPolicy::InPlace` respawns a body AT ITS CURRENT POSITION. That
/// position is the whole precondition of the policy, and leaving the world is
/// exactly what destroys it: the body is outside the room, so an in-place
/// respawn puts it straight back where the blast gate is waiting. It dies again
/// on the next tick, respawns again, and the room acquires a body whose entire
/// behaviour is dying — each death arming a hitstop, which is a global clock
/// beat, so the cost is paid by every other body in the room.
///
/// The `sandbag_finite` archetype is the one that authors `InPlace(0.85)`, so
/// it is the subject: the ONE archetype for which "gone" and "authored policy"
/// actually disagree.
#[test]
fn leaving_the_world_outranks_an_authored_in_place_respawn() {
    use super::actor_hit::{KillDisposition, kill_disposition};
    use ambition_entity_catalog::placements::RespawnPolicy;

    // the policy is STATED here rather than resolved from a fixture archetype row (deleted,
    // AC6). The claim under test belongs to `kill_disposition`: given a body that respawns in
    // place, does leaving the world outrank it.
    let sandbag = RespawnPolicy::InPlace(0.85);

    assert_eq!(
        kill_disposition(&HitSource::Melee, sandbag),
        KillDisposition::RespawnInPlace(0.85),
        "an ordinary kill still honours the authored respawn policy"
    );
    assert_eq!(
        kill_disposition(&HitSource::LeftTheWorld, sandbag),
        KillDisposition::GoneFromTheWorld,
        "the blast zone outranks the policy: there is no 'in place' out there"
    );

    // And a body that stays dead is defeated rather than gone, so the
    // exploration economy still pays out for an ordinary kill.
    assert_eq!(
        kill_disposition(&HitSource::Melee, RespawnPolicy::DeadStaysDead),
        KillDisposition::Defeated
    );
    assert_eq!(
        kill_disposition(&HitSource::LeftTheWorld, RespawnPolicy::DeadStaysDead),
        KillDisposition::GoneFromTheWorld,
        "gone is gone whatever the policy says — no coin dropped into the void"
    );
}

/// A projectile hit does not flash its thrower.
#[test]
fn a_projectile_hit_flashes_its_victim_but_never_its_thrower() {
    fn thrower_flash_after(source: HitSource) -> f32 {
        let mut app = App::new();
        app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
        app.insert_resource(GameplayBanner::default());
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        );
        app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
        app.insert_resource(ambition_persistence::settings::UserSettings::default());
        register_hit_pipeline_messages(&mut app);
        app.add_systems(Update, apply_feature_hit_events);

        let victim = spawn_hostile_actor(&mut app);
        // The thrower: a player body the attacker query can find.
        let thrower = app
            .world_mut()
            .spawn((
                crate::actor::PlayerEntity,
                crate::actor::PrimaryPlayer,
                ambition_characters::actor::BodyCombat::default(),
            ))
            .id();

        app.world_mut().write_message(HitEvent {
            strike_sfx: None,
            volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0)).into(),
            damage: 1,
            source,
            attacker: Some(thrower),
            target: HitTarget::Body(victim),
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
        app.update();

        app.world()
            .get::<ambition_characters::actor::BodyCombat>(thrower)
            .expect("the thrower keeps its combat state")
            .hit_flash
    }

    assert_eq!(
        thrower_flash_after(HitSource::Projectile),
        0.0,
        "her fireball landed across the room and SHE flashed — the attacker \
         flash is contact feel and a shot is not contact"
    );
    assert!(
        thrower_flash_after(HitSource::Melee) > 0.0,
        "a SLASH must still flash the attacker: this half is what stops the \
         projectile assertion from passing against a `hit_flash` that was simply \
         deleted"
    );
}

/// That encoding lives in another crate, by omission, and it does not survive every body being able
/// to swing under one shared melee cause.
///
/// So the boss is adjudicated by the same `damage_lands_between` every other
/// victim already is. This pins the three answers that differ.
#[test]
fn a_boss_is_adjudicated_by_the_same_relationship_rule_as_any_other_body() {
    use ambition_combat::components::ActorFaction;
    use ambition_combat::targeting::FriendlyFire;
    use ambition_characters::control::DrivingParticipant;

    let boss_entity = bevy::prelude::Entity::from_raw_u32(7).expect("nonzero raw index");
    let ff = FriendlyFire::default();
    let side = |faction: &'static ActorFaction, driver: Option<&'static DrivingParticipant>| {
        Some((faction, driver, None))
    };

    assert!(
        super::boss_damage_allowed(
            side(&ActorFaction::Player, None),
            side(&ActorFaction::Boss, None),
            ff,
            boss_entity,
        ),
        "the shipped case must keep working: a player's hit reaches the boss"
    );
    assert!(
        !super::boss_damage_allowed(
            side(&ActorFaction::Boss, None),
            side(&ActorFaction::Boss, None),
            ff,
            boss_entity,
        ),
        "an ally may not damage a boss just because its volume arrived"
    );
    // Allegiance is EFFECTIVE: a POSSESSED boss fights as its driver's side, so
    // another boss-faction body is now a legal victim of it. A policy reading the
    // authored faction would have it defending the team it was taken from.
    assert!(
        super::boss_damage_allowed(
            side(
                &ActorFaction::Boss,
                Some(&DrivingParticipant(
                    ambition_characters::control::PlayerSlot(0)
                ))
            ),
            side(&ActorFaction::Boss, None),
            ff,
            boss_entity,
        ),
        "a possessed attacker fights as its driver's side"
    );
    // An unattributed broadcast still lands — hazards and scripted blasts carry
    // no entity and cannot be adjudicated.
    assert!(
        super::boss_damage_allowed(None, side(&ActorFaction::Boss, None), ff, boss_entity),
        "a hit with no attacker cannot be adjudicated, so it is not refused"
    );
}

/// ⭐⭐ BARKS ARE RARE, NOT GONE — and the rate is what makes them unpredictable.
///
/// Jon, 2026-08-24: *"not have barks happen every time a character is hit. Make
/// it a more rare event. Not never, but I'd like it to happen less often."*
///
/// ⛔ ALL FOUR CLAUSES ARE ASSERTED, because three of them are the ways this
/// goes quietly wrong: a rate that silences everything, one that silences
/// nothing, one that is not actually random, and one that makes every fighter
/// struck on the same tick speak together.
mod bark_rate {
    use super::super::bark_is_allowed;
    use ambition_combat::rules::ResolvedCombatTuning;
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    /// A victim named the way the simulation names one. ⛔ NOT an `Entity`: the
    /// draw is salted by SIMULATION IDENTITY, because an entity index is
    /// allocator history and two peers do not agree on it.
    fn victim_named(name: &str) -> SimId {
        SimId::placement(name)
    }

    fn rules(chance: f32) -> ResolvedCombatTuning {
        ResolvedCombatTuning {
            bark_chance: chance,
            ..Default::default()
        }
    }

    fn spoke(chance: f32, ticks: u64, victim: &SimId) -> usize {
        (0..ticks)
            .filter(|t| {
                bark_is_allowed(
                    Some(&rules(chance)),
                    Some(&ambition_time::SimTick(*t)),
                    None,
                    Some(victim),
                )
            })
            .count()
    }

    /// A world that declares no rate barks on every hit — what every body did
    /// before the knob existed.
    #[test]
    fn an_undeclared_world_still_barks_on_every_hit() {
        let victim = victim_named("fighter_seven");
        assert_eq!(spoke(1.0, 200, &victim), 200);
        assert!(
            bark_is_allowed(None, None, None, Some(&victim)),
            "a composition with no combat rules at all went silent"
        );
    }

    /// ⛔ AND "RARE" IS NOT "NEVER". Jon said so in the same sentence.
    #[test]
    fn a_low_rate_speaks_sometimes_and_is_mostly_quiet() {
        let victim = victim_named("fighter_seven");
        let spoke_at_a_fifth = spoke(0.2, 400, &victim);
        assert!(
            spoke_at_a_fifth > 0,
            "a 0.2 rate never spoke in 400 hits, which is the 'never' Jon ruled out"
        );
        assert!(
            spoke_at_a_fifth < 200,
            "a 0.2 rate spoke {spoke_at_a_fifth} times in 400 hits — that is not rarer"
        );
        // Zero is still authorable, and it is the only value that means silence.
        assert_eq!(spoke(0.0, 100, &victim), 0);
    }

    /// ⭐⭐ TWO FIGHTERS STRUCK ON THE SAME TICK DECIDE INDEPENDENTLY.
    ///
    /// ⛔ THE DEFECT THIS PINS: one salt for the whole draw would make every
    /// body hit on a tick answer identically — so a multi-hit that caught two
    /// fighters would have them CHORUS, which is louder than the thing being
    /// fixed. The victim is the salt.
    #[test]
    fn two_victims_on_one_tick_do_not_chorus() {
        let a = victim_named("fighter_seven");
        let b = victim_named("fighter_nine");
        let disagreements = (0..400u64)
            .filter(|t| {
                let tick = ambition_time::SimTick(*t);
                bark_is_allowed(Some(&rules(0.5)), Some(&tick), None, Some(&a))
                    != bark_is_allowed(Some(&rules(0.5)), Some(&tick), None, Some(&b))
            })
            .count();
        assert!(
            disagreements > 40,
            "two victims disagreed on only {disagreements} of 400 ticks — they \
             are sharing one draw and will speak in unison"
        );
    }

    /// ⛔⛔ AND IT DOES NOT DEPEND ON ENTITY ALLOCATION. The salt was
    /// `victim.to_bits()` until 2026-08-25 — allocator history, which two peers
    /// that spawned the same cast in a different order do not agree on. Rollback
    /// hides it (a rewind reuses the same ids), so only a peer-vs-peer test can
    /// see it; this arm is that test's cheap local form.
    ///
    /// ⭐ THE PREMISE IS THE POINT: the two draws below share a NAME and nothing
    /// else. If identity ever leaks back into the salt, they diverge.
    #[test]
    fn one_fighter_draws_the_same_however_its_entity_was_allocated() {
        // The same fighter, as two independently constructed identities — which
        // is what the same fighter looks like on two machines.
        let here = victim_named("fighter_seven");
        let there = victim_named("fighter_seven");
        let other = victim_named("fighter_nine");

        let mut agreed = 0;
        let mut differed_from_other = 0;
        for t in 0..400u64 {
            let tick = ambition_time::SimTick(t);
            let a = bark_is_allowed(Some(&rules(0.5)), Some(&tick), None, Some(&here));
            let b = bark_is_allowed(Some(&rules(0.5)), Some(&tick), None, Some(&there));
            let c = bark_is_allowed(Some(&rules(0.5)), Some(&tick), None, Some(&other));
            if a == b {
                agreed += 1;
            }
            if a != c {
                differed_from_other += 1;
            }
        }
        assert_eq!(
            agreed,
            400,
            "two peers holding the SAME fighter disagreed on {} of 400 ticks —              something allocator-derived is back in the salt",
            400 - agreed
        );
        // ⛔ NON-VACUITY: if the salt stopped mattering entirely, every body
        // would agree and the arm above would pass while saying nothing.
        assert!(
            differed_from_other > 40,
            "a DIFFERENT fighter drew the same answer on all but {differed_from_other}              of 400 ticks, so the name is not reaching the draw at all"
        );
    }

    /// ⛔ AND IT IS REPRODUCIBLE, because it is read inside the rollback window:
    /// a resimulated hit that answered differently would flicker the bubble on
    /// every rewind.
    #[test]
    fn the_same_hit_answers_the_same_way_twice() {
        let victim = victim_named("fighter_seven");
        let tick = ambition_time::SimTick(123);
        let first = bark_is_allowed(Some(&rules(0.3)), Some(&tick), None, Some(&victim));
        for _ in 0..8 {
            assert_eq!(
                bark_is_allowed(Some(&rules(0.3)), Some(&tick), None, Some(&victim)),
                first,
                "the same hit answered differently on a re-ask, so a rewind \
                 changes what the fighter said"
            );
        }
    }
}
