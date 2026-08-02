//! The composition, asserted as behaviour: each trait acts alone, and both act
//! together, without either knowing the other exists.

use super::*;

fn app_with_body(traits: Empowerment, seconds: f32) -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    let body = app
        .world_mut()
        .spawn((
            Empowered::new(traits, seconds),
            BodyHealth::new(ambition_characters::actor::Health::new(5)),
            ae::BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(30.0, 48.0),
                facing: 1.0,
            },
        ))
        .id();
    app.add_systems(
        Update,
        (run_empowerments, despawn_orphaned_contact_hitboxes).chain(),
    );
    (app, body)
}

fn contact_volumes(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query::<&ContactHitbox>();
    q.iter(world).count()
}

fn untouchable(app: &App, body: Entity) -> bool {
    app.world()
        .get::<BodyHealth>(body)
        .unwrap()
        .health
        .invulnerable
        .holds(Invulnerability::EMPOWERED)
}

/// **Untouchable alone.** Sanic's super form will ask for exactly this and needs
/// no engine change to get it.
#[test]
fn untouchable_alone_grants_no_contact_volume() {
    let (mut app, body) = app_with_body(Empowerment::UNTOUCHABLE, 1.0);
    app.update();
    assert!(untouchable(&app, body), "it holds the EMPOWERED reason");
    assert_eq!(
        contact_volumes(&mut app),
        0,
        "and publishes no strike volume — it was not asked for one"
    );
}

/// **Harming alone.** A body that flattens what it touches without being safe
/// itself is a legitimate thing to want, and asking for it does not drag
/// invulnerability along.
#[test]
fn harming_alone_grants_no_invulnerability() {
    let (mut app, body) = app_with_body(Empowerment::HARMS_ON_CONTACT, 1.0);
    app.update();
    assert_eq!(contact_volumes(&mut app), 1, "it publishes one volume");
    assert!(
        !untouchable(&app, body),
        "and stays hittable — the traits do not imply each other"
    );
}

/// **Both, composed** — Mary-O's cosmic quasar. Nothing in the engine names it.
#[test]
fn the_two_traits_compose_into_one_super_state() {
    let quasar = Empowerment::UNTOUCHABLE.with(Empowerment::HARMS_ON_CONTACT);
    let (mut app, body) = app_with_body(quasar, 1.0);
    app.update();
    assert!(untouchable(&app, body));
    assert_eq!(contact_volumes(&mut app), 1);
}

/// One volume, not one per tick. A body running for ten seconds must not leave a
/// trail of six hundred hitboxes behind it.
#[test]
fn the_contact_volume_is_leased_not_re_spawned_every_tick() {
    let (mut app, _) = app_with_body(Empowerment::HARMS_ON_CONTACT, 1.0);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        contact_volumes(&mut app),
        1,
        "the volume is found and its lease topped up, never duplicated"
    );
}

/// It ENDS: the reason is released and the volume is swept.
#[test]
fn expiry_releases_the_reason_and_removes_the_volume() {
    let quasar = Empowerment::UNTOUCHABLE.with(Empowerment::HARMS_ON_CONTACT);
    let (mut app, body) = app_with_body(quasar, 0.1);
    for _ in 0..12 {
        app.update();
    }
    assert!(
        app.world().get::<Empowered>(body).is_none(),
        "a spent empowerment leaves the body"
    );
    assert!(!untouchable(&app, body), "the reason is released");
    assert_eq!(contact_volumes(&mut app), 0, "and the volume is gone");
}

/// The half that is always forgotten: the volume must not outlive its owner and
/// keep hurting things where the body used to be.
#[test]
fn a_contact_volume_whose_owner_vanished_is_swept() {
    let (mut app, body) = app_with_body(Empowerment::HARMS_ON_CONTACT, 5.0);
    app.update();
    assert_eq!(contact_volumes(&mut app), 1);

    app.world_mut().entity_mut(body).despawn();
    app.update();

    assert_eq!(
        contact_volumes(&mut app),
        0,
        "no orphaned strike volume is left following a dead body"
    );
}
