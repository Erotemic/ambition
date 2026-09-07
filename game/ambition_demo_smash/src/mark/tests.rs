use super::*;
use ambition_platformer2d::characters::smash_mark::MarkBodyParams;
use ambition_platformer2d::combat::on_hit::OnHitEffectMessage;
use ambition_platformer2d::engine_core as ae;

fn app() -> App {
    let mut app = App::new();
    app.add_message::<OnHitEffectMessage>();
    app.add_message::<ambition_platformer2d::vfx::EffectRequest>();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    {
        let mut time = app
            .world_mut()
            .resource_mut::<ambition_platformer2d::time::WorldTime>();
        time.scaled_dt = 1.0 / 60.0;
        time.raw_dt = 1.0 / 60.0;
    }
    app.init_resource::<CapturedBlasts>();
    app.add_systems(
        Update,
        (apply_authored_body_marks, detonate_body_marks, capture_blasts).chain(),
    );
    app
}

fn body(app: &mut App, at: ae::Vec2) -> Entity {
    let mut kin = ae::BodyKinematics::default();
    kin.pos = at;
    app.world_mut().spawn(kin).id()
}

fn params(fuse_s: f32) -> MarkBodyParams {
    MarkBodyParams {
        fuse_s,
        damage: 6,
        blast_radius: 44.0,
        knockback: 1.2,
    }
}

fn land_a_marking_hit(app: &mut App, owner: Entity, victim: Entity, fuse_s: f32) {
    let effect = ambition_platformer2d::entity_catalog::EffectRef {
        key: ambition_platformer2d::characters::smash_mark::MARK_BODY.to_string(),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params(fuse_s))
            .expect("params serialize"),
    };
    app.world_mut().write_message(OnHitEffectMessage {
        owner,
        victim,
        volume: ae::CombatVolume::Circle {
            center: ae::Vec2::ZERO,
            radius: 1.0,
        },
        contact: ae::Vec2::ZERO,
        effect,
    });
}

/// ⛔⛔ ACCUMULATED AS THEY ARE WRITTEN, NOT READ AT THE END — and the first
/// draft of this file did it the wrong way and reported ZERO blasts from tests
/// whose marks plainly detonated. Bevy's messages are double-buffered: after a
/// couple of `app.update()` calls the write is gone, so "no blast" and "a blast I
/// can no longer see" are the same number.
///
/// ⇒ Second time in one day I have written that bug — the hostile-threshold
/// poison did it too. A test that steps a clock cannot ask a message buffer what
/// happened; it has to have been listening.
///
/// ⚠ A COUNT AND THE LAST ONE, NOT A `Vec` — and that is not style. A `Resource`
/// holding a collection is what `per_attempt_resource_census` exists to find,
/// because a collection in a resource is the shape that survives a match it
/// should not; the census does not exempt test modules, and it is right not to
/// (a fixture is where the shape gets normalised). Every assertion here needs
/// "how many" and "where was the last", so nothing is lost.
#[derive(Resource, Default)]
struct CapturedBlasts {
    count: usize,
    last: ae::Vec2,
}

fn capture_blasts(
    mut reader: MessageReader<ambition_platformer2d::vfx::EffectRequest>,
    mut cap: ResMut<CapturedBlasts>,
) {
    for request in reader.read() {
        if let ambition_platformer2d::vfx::Effect::DamageBox(b) = &request.effect {
            cap.count += 1;
            cap.last = b.center;
        }
    }
}

fn blast_count(app: &App) -> usize {
    app.world().resource::<CapturedBlasts>().count
}

fn last_blast(app: &App) -> ae::Vec2 {
    app.world().resource::<CapturedBlasts>().last
}

/// ⛔ THE MARK RIDES THE BODY, WHICH IS THE ONLY REASON THIS IS NOT A MINE.
/// The blast happens where the victim IS when the clock runs out, not where they
/// were when they were hit — so running away does not help, and that is the whole
/// mechanic. A mark that detonated at the contact point would be a slow mine with
/// extra steps.
#[test]
fn the_blast_follows_the_body_rather_than_the_contact_point() {
    let mut app = app();
    let owner = body(&mut app, ae::Vec2::new(0.0, 0.0));
    let victim = body(&mut app, ae::Vec2::new(100.0, 0.0));
    land_a_marking_hit(&mut app, owner, victim, 0.1);
    app.update();
    assert!(
        app.world().get::<BodyMark>(victim).is_some(),
        "the struck body carries no mark"
    );

    // They run before it goes off.
    app.world_mut()
        .get_mut::<ae::BodyKinematics>(victim)
        .expect("victim has kinematics")
        .pos = ae::Vec2::new(400.0, 0.0);
    for _ in 0..12 {
        app.update();
    }

    assert_eq!(blast_count(&app), 1, "expected exactly one detonation");
    assert_eq!(
        last_blast(&app),
        ae::Vec2::new(400.0, 0.0),
        "the blast must land where the marked body ran TO, not where it was hit"
    );
    assert!(
        app.world().get::<BodyMark>(victim).is_none(),
        "a spent mark must be removed, or it detonates every frame after"
    );
}

/// ⛔⛔ A MARK THAT NEVER CLEARS IS A BLAST EVERY FRAME. The removal above is one
/// line and its absence would be invisible to a test that only checks the FIRST
/// detonation — so this one keeps running afterwards.
#[test]
fn a_spent_mark_does_not_keep_detonating() {
    let mut app = app();
    let owner = body(&mut app, ae::Vec2::ZERO);
    let victim = body(&mut app, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, owner, victim, 0.05);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        blast_count(&app),
        1,
        "one mark must produce exactly one blast however long the match runs"
    );
}

/// ⚠ THE FUSE IS A DECISION AND THEREFORE MUST BE OBSERVABLE. A mark that went
/// off on the tick it was applied would be a hit with extra steps, and the
/// authored `fuse_s` would be documentation.
#[test]
fn the_mark_waits_out_its_fuse() {
    let mut app = app();
    let owner = body(&mut app, ae::Vec2::ZERO);
    let victim = body(&mut app, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, owner, victim, 0.5);
    app.update();
    assert_eq!(
        blast_count(&app),
        0,
        "the mark detonated on the tick it was applied; the fuse buys nothing"
    );
    for _ in 0..40 {
        app.update();
    }
    assert_eq!(blast_count(&app), 1, "and it must go off eventually");
}

/// ⛔ AN UNRECOGNISED KEY MUST FALL THROUGH UNTOUCHED. Every on-hit effect in the
/// game passes through this system; one that marked on any key would attach a
/// detonation to every technique that authors an on-hit at all.
#[test]
fn a_hit_carrying_somebody_elses_effect_leaves_no_mark() {
    let mut app = app();
    let owner = body(&mut app, ae::Vec2::ZERO);
    let victim = body(&mut app, ae::Vec2::new(50.0, 0.0));
    app.world_mut().write_message(OnHitEffectMessage {
        owner,
        victim,
        volume: ae::CombatVolume::Circle {
            center: ae::Vec2::ZERO,
            radius: 1.0,
        },
        contact: ae::Vec2::ZERO,
        // ⛔⛔ CARRYING MARK-SHAPED PARAMS, DELIBERATELY, and the first version of
        // this test did not — it sent an EMPTY payload, which fails to hydrate
        // whatever the key says. ⇒ Deleting the key check entirely still passed,
        // because hydration was refusing the payload rather than the filter
        // refusing the key, and the test could not tell the two apart.
        //
        // A real collision is another technique whose params happen to have this
        // shape, so that is what this sends: only the KEY separates them.
        effect: ambition_platformer2d::entity_catalog::EffectRef {
            key: "smash.some_other_effect".to_string(),
            params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params(0.1))
                .expect("params serialize"),
        },
    });
    for _ in 0..30 {
        app.update();
    }
    assert!(
        app.world().get::<BodyMark>(victim).is_none(),
        "an unrelated on-hit effect marked the body"
    );
    assert_eq!(blast_count(&app), 0, "and it produced a blast");
}

/// ⭐ A SECOND MARK REFRESHES RATHER THAN STACKS, which the authored params say
/// too. Stacking would turn one read ("how long have I got") into arithmetic
/// nobody can do mid-match, and the read is what the move sells.
#[test]
fn a_second_mark_refreshes_the_clock_and_does_not_add_a_second_blast() {
    let mut app = app();
    let owner = body(&mut app, ae::Vec2::ZERO);
    let victim = body(&mut app, ae::Vec2::new(50.0, 0.0));
    land_a_marking_hit(&mut app, owner, victim, 0.2);
    for _ in 0..6 {
        app.update();
    }
    land_a_marking_hit(&mut app, owner, victim, 0.2);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        blast_count(&app),
        1,
        "two marks on one body must still be one detonation"
    );
}
