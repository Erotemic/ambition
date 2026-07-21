use super::*;

#[test]
fn the_crony_roster_fragment_parses() {
    // The archetype RON must be a valid roster fragment — the standalone demo
    // has no other roster, so a malformed row would leave the crony as the
    // inert engine fallback (no walk, no contact).
    let mut app = App::new();
    register_crony_roster(&mut app);
    assert!(app
        .world()
        .contains_resource::<ambition::actors::features::CharacterRoster>());
}

fn kin(pos: ae::Vec2, vel: ae::Vec2) -> ae::BodyKinematics {
    ae::BodyKinematics {
        pos,
        vel,
        size: ae::Vec2::new(28.0, 32.0),
        facing: 1.0,
    }
}

fn spawn_pair(app: &mut App, player_vel: ae::Vec2) -> (Entity, Entity) {
    use ambition::characters::actor::Health;
    // Crony at the origin; its head (min.y) sits at y = -16.
    let crony = app
        .world_mut()
        .spawn((
            kin(ae::Vec2::ZERO, ae::Vec2::ZERO),
            BodyHealth::new(Health::new(1)),
        ))
        .id();
    // Player directly above, feet (max.y) exactly on the crony's head.
    let player = app
        .world_mut()
        .spawn((
            PrimaryPlayer,
            PlayerEntity,
            kin(ae::Vec2::new(0.0, -32.0), player_vel),
        ))
        .id();
    (crony, player)
}

/// **The shell mechanic, end to end: kick it, it runs a crony down, it turns
/// around at a wall.**
///
/// Worth pinning because none of it is visible without playing: a shell that
/// silently fails to reverse parks against the first wall forever, and one
/// whose hit test is a hair off sails through cronies — both look like
/// "nothing happened", which is the failure you cannot tell from "not
/// implemented".
#[test]
fn a_kicked_shell_slides_kills_cronies_and_reverses_at_a_wall() {
    use ambition::characters::actor::Health;

    let mut app = App::new();
    app.add_message::<ambition::vfx::VfxMessage>();
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
    app.add_systems(Update, (kick_mary_o_shells, drive_mary_o_shells).chain());

    // A resting shell at the origin, the player just to its LEFT (touching),
    // and a crony to the right in the shell's path.
    let shell = app
        .world_mut()
        .spawn((kin(ae::Vec2::ZERO, ae::Vec2::ZERO), MaryOShell::Resting))
        .id();
    app.world_mut().spawn((
        PrimaryPlayer,
        PlayerEntity,
        kin(ae::Vec2::new(-20.0, 0.0), ae::Vec2::ZERO),
    ));
    let crony = app
        .world_mut()
        .spawn((
            kin(ae::Vec2::new(24.0, 0.0), ae::Vec2::ZERO),
            BodyHealth::new(Health::new(1)),
            // `FeatureName`, because that is what the PRODUCTION spawner
            // puts on an actor. The first version of this fixture used
            // `Name`, which the spawner decorates — so the test passed
            // against components no real crony has, while the shipped
            // mechanic matched nothing and did nothing.
            FeatureName::new(CRONY_DISPLAY_NAME),
        ))
        .id();

    app.update();

    // Kicked AWAY from the player, so rightward.
    assert_eq!(
        app.world().get::<MaryOShell>(shell).copied(),
        Some(MaryOShell::Sliding(1.0)),
        "a shell is kicked away from the side you touch it from"
    );
    assert!(
        app.world().get::<ae::BodyKinematics>(shell).unwrap().vel.x > 0.0,
        "and it actually carries velocity, not just a state label"
    );
    assert!(
        app.world().get_entity(crony).is_err(),
        "the crony in its path is run down — this is the payoff of a stomp"
    );

    // Now the wall. The shell has slid well clear of the player by the time
    // it reaches one, so put it there — leaving it inside her box would
    // instead exercise the stop-a-sliding-shell rule, which is a different
    // rule. The body kernel zeroes horizontal velocity on contact, so a
    // shell still commanded to slide but no longer moving has hit something.
    {
        let mut kin = app
            .world_mut()
            .get_mut::<ae::BodyKinematics>(shell)
            .unwrap();
        kin.pos.x = 400.0;
        kin.vel.x = 0.0;
    }
    app.update();
    assert_eq!(
        app.world().get::<MaryOShell>(shell).copied(),
        Some(MaryOShell::Sliding(-1.0)),
        "a blocked shell reverses instead of parking against the wall"
    );
}

#[test]
fn a_descending_player_bounces_off_and_squashes_a_crony() {
    let mut app = App::new();
    app.add_message::<ambition::vfx::VfxMessage>();
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
    // The stomp now REQUESTS a shell spawn, so this channel is part of its
    // contract rather than optional scenery.
    app.add_message::<SpawnActorRequest>();
    app.add_systems(Update, bounce_squash_cronies);
    // Falling onto the head (screen gravity: +y is down, so vel.y > 0 falls).
    let (crony, player) = spawn_pair(&mut app, ae::Vec2::new(0.0, 240.0));
    app.update();

    assert!(
        app.world().get_entity(crony).is_err(),
        "a stomped crony is squashed (despawned)"
    );
    let vel = app.world().get::<ae::BodyKinematics>(player).unwrap().vel;
    assert!(
        vel.y < 0.0,
        "the stomp bounces the player back UP (screen gravity: up is -y), got {vel:?}"
    );
    // The squash leaves a visible mark: a dust burst through the engine seam.
    let bursts = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ambition::vfx::VfxMessage>>()
        .drain()
        .filter(|m| matches!(m, ambition::vfx::VfxMessage::Burst { .. }))
        .count();
    assert_eq!(bursts, 1, "a squash pops exactly one dust burst");
}

#[test]
fn a_rising_player_does_not_squash_a_crony() {
    let mut app = App::new();
    app.add_message::<ambition::vfx::VfxMessage>();
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
    // The stomp now REQUESTS a shell spawn, so this channel is part of its
    // contract rather than optional scenery.
    app.add_message::<SpawnActorRequest>();
    app.add_systems(Update, bounce_squash_cronies);
    // Overlapping the crony's head band but moving UP — a side/undercut hit,
    // which the engine's contact-damage pass owns, not a stomp.
    let (crony, _player) = spawn_pair(&mut app, ae::Vec2::new(0.0, -200.0));
    app.update();
    assert!(
        app.world().get_entity(crony).is_ok(),
        "only a DESCENDING player stomps; a rising one must not squash"
    );
}

#[test]
fn cronies_spawn_on_the_flats_named_for_the_ai_slop_sheet() {
    let spawn = ae::Vec2::new(2.0 * T, 400.0);
    let reqs = crony_spawn_requests(spawn);
    assert_eq!(reqs.len(), CRONY_TILE_COLUMNS.len());
    for req in &reqs {
        assert_eq!(
            req.name, CRONY_DISPLAY_NAME,
            "every crony must carry the display name the ai_slop sheet resolves from"
        );
        assert!(
            matches!(&req.kind, SpawnActorKind::Enemy { brain }
                if matches!(brain, CharacterBrain::Custom(k) if k == CRONY_BRAIN_KEY)),
            "cronies spawn on the demo's own roster archetype"
        );
        assert_eq!(
            req.pos.y, spawn.y,
            "dropped in at standing height to settle"
        );
    }
}
