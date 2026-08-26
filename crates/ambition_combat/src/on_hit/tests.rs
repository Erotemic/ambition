use super::*;
use bevy::prelude::*;

#[derive(Resource, Default)]
struct CapturedEffects(Vec<OnHitEffectMessage>);

fn capture_effects(
    mut messages: MessageReader<OnHitEffectMessage>,
    mut captured: ResMut<CapturedEffects>,
) {
    captured.0.extend(messages.read().cloned());
}

fn landed(
    hitbox: Entity,
    attacker: Entity,
    victim: Entity,
    volume: ae::CombatVolume,
) -> LandedBodyHit {
    LandedBodyHit {
        hitbox,
        attacker,
        victim,
        contact: volume.center(),
        volume,
    }
}

#[test]
fn landed_body_hit_projects_the_authored_effect_without_re_resolving_contact() {
    let mut app = App::new();
    app.add_message::<LandedBodyHit>();
    app.add_message::<OnHitEffectMessage>();
    app.init_resource::<CapturedEffects>();
    app.add_systems(
        Update,
        (dispatch_landed_hit_effects, capture_effects).chain(),
    );

    let attacker = app.world_mut().spawn_empty().id();
    let victim = app.world_mut().spawn_empty().id();
    let effect = EffectRef::new("lifesteal");
    let hitbox = app.world_mut().spawn(HitboxOnHit::new(effect.clone())).id();
    let volume: ae::CombatVolume =
        ae::Aabb::new(ae::Vec2::new(40.0, 50.0), ae::Vec2::new(8.0, 6.0)).into();

    app.world_mut()
        .write_message(landed(hitbox, attacker, victim, volume.clone()));
    app.update();

    let captured = &app.world().resource::<CapturedEffects>().0;
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].owner, attacker);
    assert_eq!(captured[0].victim, victim);
    assert_eq!(captured[0].volume, volume);
    assert_eq!(captured[0].effect.key, effect.key);
}

fn pogo_app(
    policy: PogoPolicy,
    pogo_volumes: Option<PogoTargetVolumes>,
) -> (App, Entity, Entity, Entity) {
    let mut app = App::new();
    app.add_message::<LandedBodyHit>();
    app.add_message::<OnHitEffectMessage>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_systems(
        Update,
        (dispatch_landed_hit_effects, apply_pogo_bounce).chain(),
    );

    let owner = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(28.0, 46.0),
                facing: 1.0,
            },
            ambition_platformer2d_core::BodyGroundState {
                head_contact: false,
                on_ground: true,
                ..Default::default()
            },
            ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default(),
        ))
        .id();
    let mut victim = app.world_mut().spawn(policy);
    if let Some(pogo_volumes) = pogo_volumes {
        victim.insert(pogo_volumes);
    }
    let victim = victim.id();
    let hitbox = app
        .world_mut()
        .spawn(HitboxOnHit::new(EffectRef::new(POGO_BOUNCE_KEY)))
        .id();
    (app, owner, victim, hitbox)
}

#[test]
fn from_damageable_pogo_uses_the_resolved_body_hit_as_its_contact_fact() {
    let (mut app, owner, victim, hitbox) = pogo_app(PogoPolicy::FromDamageable, None);
    let volume: ae::CombatVolume =
        ae::Aabb::new(ae::Vec2::new(100.0, 130.0), ae::Vec2::new(18.0, 18.0)).into();
    app.world_mut()
        .write_message(landed(hitbox, owner, victim, volume));
    app.update();

    let kin = app.world().get::<ae::BodyKinematics>(owner).unwrap();
    assert!(
        kin.vel.y < -1.0,
        "the resolved victim contact should rebound the owner, vel={:?}",
        kin.vel
    );
    assert!(
        !app.world()
            .get::<ambition_platformer2d_core::BodyGroundState>(owner)
            .unwrap()
            .on_ground
    );
}

#[test]
fn disabled_pogo_policy_rejects_an_otherwise_landed_body_hit() {
    let (mut app, owner, victim, hitbox) = pogo_app(PogoPolicy::Disabled, None);
    let volume: ae::CombatVolume =
        ae::Aabb::new(ae::Vec2::new(100.0, 130.0), ae::Vec2::new(18.0, 18.0)).into();
    app.world_mut()
        .write_message(landed(hitbox, owner, victim, volume));
    app.update();

    assert_eq!(
        app.world().get::<ae::BodyKinematics>(owner).unwrap().vel,
        ae::Vec2::ZERO
    );
}

#[test]
fn custom_pogo_policy_uses_its_own_volume_against_the_landed_strike() {
    let custom = ae::Aabb::new(ae::Vec2::new(200.0, 200.0), ae::Vec2::new(8.0, 8.0));
    let (mut app, owner, victim, hitbox) = pogo_app(
        PogoPolicy::Custom,
        Some(PogoTargetVolumes {
            volumes: vec![custom],
        }),
    );
    let miss: ae::CombatVolume =
        ae::Aabb::new(ae::Vec2::new(100.0, 130.0), ae::Vec2::new(18.0, 18.0)).into();
    app.world_mut()
        .write_message(landed(hitbox, owner, victim, miss));
    app.update();
    assert_eq!(
        app.world().get::<ae::BodyKinematics>(owner).unwrap().vel,
        ae::Vec2::ZERO,
        "a landed damage hit outside the custom pogo silhouette is not a pogo"
    );

    let hit: ae::CombatVolume = custom.into();
    app.world_mut()
        .write_message(landed(hitbox, owner, victim, hit));
    app.update();
    assert!(app.world().get::<ae::BodyKinematics>(owner).unwrap().vel.y < -1.0);
}

#[test]
fn body_pogo_runs_from_the_shared_strike_resolver_end_to_end() {
    use crate::components::ActorFaction;
    use crate::events::HitEvent;
    use crate::hitbox::{
        apply_hitbox_damage, HitSide, Hitbox, HitboxAnchor, HitboxHits, HitboxKnockback,
        HitboxLifetime,
    };
    use ambition_platformer2d_core::AabbExt;
    use ambition_vfx::vfx::VfxMessage;

    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<LandedBodyHit>();
    app.add_message::<OnHitEffectMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_systems(
        Update,
        (
            apply_hitbox_damage,
            dispatch_landed_hit_effects,
            apply_pogo_bounce,
        )
            .chain(),
    );

    let owner_center = ae::Vec2::new(100.0, 100.0);
    let victim_center = ae::Vec2::new(100.0, 140.0);
    let owner = app
        .world_mut()
        .spawn((
            ActorFaction::Player,
            ae::CenteredAabb::from_center_size(owner_center, ae::Vec2::new(20.0, 40.0)),
            ae::BodyKinematics {
                pos: owner_center,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(20.0, 40.0),
                facing: 1.0,
            },
            ambition_platformer2d_core::BodyGroundState {
                head_contact: false,
                on_ground: true,
                ..Default::default()
            },
            ambition_platformer2d_core::BodyOffense::default(),
            ambition_platformer2d_core::BodyMotionFacts::default(),
            ambition_platformer2d_core::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
            ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default(),
            PogoPolicy::FromDamageable,
            PogoTargetVolumes::default(),
        ))
        .id();
    let victim = app
        .world_mut()
        .spawn((
            ActorFaction::Enemy,
            ae::CenteredAabb::from_center_size(victim_center, ae::Vec2::new(20.0, 40.0)),
            ambition_platformer2d_core::BodyOffense::default(),
            ambition_platformer2d_core::BodyMotionFacts::default(),
            ambition_platformer2d_core::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
            PogoPolicy::FromDamageable,
            PogoTargetVolumes::default(),
        ))
        .id();

    let owner_body = app.world().get::<ae::CenteredAabb>(owner).unwrap().aabb();
    let victim_body = app.world().get::<ae::CenteredAabb>(victim).unwrap().aabb();
    assert!(
        !owner_body.strict_intersects(victim_body),
        "the body-contact regression requires separated collision bodies"
    );

    let hitbox = app
        .world_mut()
        .spawn((
            Hitbox {
                strike_sfx: None,
                owner,
                source: HitSide::Player,
                anchor: HitboxAnchor::FollowOwner {
                    local_offset: ae::Vec2::new(0.0, 20.0),
                },
                half_extent: ae::Vec2::new(18.0, 24.0),
                shape: None,
                facing: 1.0,
                damage: 4,
                knockback: HitboxKnockback::FeelScale(0.0),
                launch_dir: None,
                frame_down: ae::Vec2::new(0.0, 1.0),
                reaction: None,
            },
            HitboxLifetime { remaining_s: 0.1 },
            HitboxHits::default(),
            HitboxOnHit::new(EffectRef::new(POGO_BOUNCE_KEY)),
        ))
        .id();

    app.update();

    let hits = app.world().get::<HitboxHits>(hitbox).expect("live hitbox");
    assert!(
        hits.hit.contains(&victim),
        "the separated victim is the landed body"
    );
    assert!(
        !hits.hit.contains(&owner),
        "the attacking body never becomes its own landed-hit victim"
    );
    let kin = app.world().get::<ae::BodyKinematics>(owner).unwrap();
    assert!(
        kin.vel.y < -1.0,
        "the victim's resolved body hit drives the pogo rebound, vel={:?}",
        kin.vel
    );
}
