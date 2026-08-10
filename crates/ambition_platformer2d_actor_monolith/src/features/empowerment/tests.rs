//! The composition, asserted as behaviour: each trait acts alone, and both act
//! together, without either knowing the other exists.
//!
//! ⚠ these used to assert that a strike VOLUME existed, and every one of them
//! passed while contact damage did nothing at all — the volume was published
//! correctly and `apply_hitbox_damage` dropped it every tick, because a
//! `HitSide::Player` FollowOwner hitbox is resolved as a melee swing and there
//! was no swing. Counting the artifact proved the artifact. So the harm tests
//! now assert the only thing that was ever the point: **something overlapping
//! takes the hit.**

use super::*;

use ambition_characters::actor::Health;
use ambition_combat::components::CenteredAabb;

/// A striker at the origin and a victim standing in it, overlapping by
/// construction — the geometry is not what these tests are about.
fn app_with_striker_and_victim(
    traits: Empowerment,
    seconds: f32,
    victim_faction: ActorFaction,
) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    app.add_message::<HitEvent>();
    app.add_message::<crate::combat::hitbox::LandedBodyHit>();
    let pos = ae::Vec2::new(100.0, 100.0);
    let size = ae::Vec2::new(30.0, 48.0);
    let striker = app
        .world_mut()
        .spawn((
            Empowered::for_seconds(traits, seconds),
            BodyHealth::new(Health::new(5)),
            ActorFaction::Player,
            ae::BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size,
                facing: 1.0,
            },
        ))
        .id();
    let victim = app
        .world_mut()
        .spawn((
            BodyHealth::new(Health::new(5)),
            victim_faction,
            CenteredAabb {
                center: pos,
                half_size: size * 0.5,
            },
            ae::BodyMotionFacts::default(),
            crate::actor::BodyShieldState::default(),
            BodyCombat::default(),
        ))
        .id();
    app.add_systems(Update, (run_empowerments, apply_contact_harm).chain());
    (app, striker, victim)
}

/// Every hit written this tick, drained.
fn hits(app: &mut App) -> Vec<HitEvent> {
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<HitEvent>>()
        .drain()
        .collect()
}

fn untouchable(app: &App, body: Entity) -> bool {
    app.world()
        .get::<BodyHealth>(body)
        .unwrap()
        .health
        .invulnerable
        .holds(Invulnerability::EMPOWERED)
}

/// **Untouchable alone.** Sanic's super form asks for exactly this and needs no
/// engine change to get it.
#[test]
fn untouchable_alone_harms_nothing() {
    let (mut app, striker, _) =
        app_with_striker_and_victim(Empowerment::UNTOUCHABLE, 1.0, ActorFaction::Enemy);
    app.update();
    assert!(untouchable(&app, striker), "it holds the EMPOWERED reason");
    assert!(
        hits(&mut app).is_empty(),
        "and harms nothing it stands inside — it was not asked to"
    );
}

/// **Harming alone.** A body that flattens what it touches without being safe
/// itself is a legitimate thing to want, and asking for it does not drag
/// invulnerability along.
#[test]
fn harming_alone_hurts_what_it_touches_and_grants_no_invulnerability() {
    let (mut app, striker, victim) =
        app_with_striker_and_victim(Empowerment::HARMS_ON_CONTACT, 1.0, ActorFaction::Enemy);
    app.update();
    let written = hits(&mut app);
    assert_eq!(
        written.len(),
        1,
        "the overlapping enemy takes exactly one hit"
    );
    assert_eq!(
        written[0].target,
        HitTarget::Actor(victim),
        "and it is aimed AT that body, so its own consumer resolves it"
    );
    assert_eq!(written[0].attacker, Some(striker));
    assert_eq!(written[0].source, HitSource::ContactHarm);
    assert!(
        !untouchable(&app, striker),
        "and the striker stays hittable — the traits do not imply each other"
    );
}

/// **Both, composed** — Mary-O's cosmic quasar. Nothing in the engine names it.
#[test]
fn the_two_traits_compose_into_one_super_state() {
    let quasar = Empowerment::UNTOUCHABLE.with(Empowerment::HARMS_ON_CONTACT);
    let (mut app, striker, _) = app_with_striker_and_victim(quasar, 1.0, ActorFaction::Enemy);
    app.update();
    assert!(untouchable(&app, striker));
    assert_eq!(hits(&mut app).len(), 1);
}

/// **Who may be hit is the SHARED relational rule.** An empowered body does not
/// flatten its own side — otherwise a co-op partner running past a star-powered
/// player would be the star's first victim.
#[test]
fn it_does_not_harm_its_own_faction() {
    let (mut app, _, _) =
        app_with_striker_and_victim(Empowerment::HARMS_ON_CONTACT, 1.0, ActorFaction::Player);
    app.update();
    assert!(
        hits(&mut app).is_empty(),
        "same faction, no grudge, friendly fire off — nothing lands"
    );
}

/// **A corpse is not hit again.** The shared tangibility rule, not a second
/// opinion about it.
#[test]
fn it_does_not_strike_a_corpse() {
    let (mut app, _, victim) =
        app_with_striker_and_victim(Empowerment::HARMS_ON_CONTACT, 1.0, ActorFaction::Enemy);
    let mut health = app.world_mut().get_mut::<BodyHealth>(victim).unwrap();
    assert!(health.health.damage(5), "kill it outright");
    assert!(!health.health.alive());
    app.update();
    assert!(hits(&mut app).is_empty(), "a corpse is intangible");
}

/// It ENDS: the reason is released and the harm stops with it.
#[test]
fn expiry_releases_the_reason_and_stops_the_harm() {
    let quasar = Empowerment::UNTOUCHABLE.with(Empowerment::HARMS_ON_CONTACT);
    let (mut app, striker, _) = app_with_striker_and_victim(quasar, 0.1, ActorFaction::Enemy);
    for _ in 0..12 {
        app.update();
    }
    let _ = hits(&mut app);
    app.update();

    assert!(
        app.world().get::<Empowered>(striker).is_none(),
        "a spent empowerment leaves the body"
    );
    assert!(!untouchable(&app, striker), "the reason is released");
    assert!(
        hits(&mut app).is_empty(),
        "and it stops flattening what it stands in"
    );
}

/// **The harm outlives nothing.** A striker that despawns mid-empowerment
/// cannot keep hurting whatever is standing where it used to be — which is free
/// here, and was NOT free when this was a hitbox entity that had to be swept.
#[test]
fn a_vanished_striker_harms_nothing() {
    let (mut app, striker, _) =
        app_with_striker_and_victim(Empowerment::HARMS_ON_CONTACT, 5.0, ActorFaction::Enemy);
    app.update();
    assert_eq!(hits(&mut app).len(), 1);

    app.world_mut().entity_mut(striker).despawn();
    app.update();

    assert!(
        hits(&mut app).is_empty(),
        "no strike survives the body that was making it"
    );
}

/// **Taking the empowerment back releases what it was projecting**, with no
/// second call at the removal site.
///
/// The failure this pins is silent and permanent: `run_empowerments` can only
/// write the `EMPOWERED` reason for bodies that still HAVE the component, so a
/// granter that removes it left the body untouchable forever. Sanic's super form
/// is exactly that granter, and it carried a hand-written
/// `invulnerable.set(EMPOWERED, false)` beside its `remove::<Empowered>()` to
/// cover for it — the two-step ritual whose second step is the one people forget.
///
/// So this test removes the component ALONE, which is what a caller who never
/// read the ritual would do.
#[test]
fn removing_the_empowerment_releases_its_invulnerability_without_a_second_call() {
    let (mut app, striker, _victim) =
        app_with_striker_and_victim(Empowerment::UNTOUCHABLE, 10.0, ActorFaction::Enemy);
    app.add_plugins(EmpowermentProjectionPlugin);
    app.update();
    assert!(
        app.world()
            .get::<BodyHealth>(striker)
            .expect("the striker has health")
            .health
            .invulnerable
            .any(),
        "an untouchable empowerment must project its reason while it is held",
    );

    app.world_mut().entity_mut(striker).remove::<Empowered>();

    assert!(
        !app.world()
            .get::<BodyHealth>(striker)
            .expect("the striker has health")
            .health
            .invulnerable
            .any(),
        "removing the empowerment must release the EMPOWERED reason — otherwise \
         the body stays untouchable for the rest of its life",
    );
}
