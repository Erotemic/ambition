//! A snake is GEOMETRY-COMPLETE FROM ITS FIRST OBSERVABLE SNAPSHOT, on both the
//! fresh-start and the replay road.
//!
//! Construction resolves the body from the character's own `BodySource`, so the
//! collision box, the render size and the sprite offset a snake is born with are
//! the ones it keeps. Nothing patches them afterwards — not
//! `sync_sprite_posed_bodies` on the next tick, not `tag_mary_o_snakes`, which
//! is behavioural only.
//!
//! That matters because presentation cannot take a second answer: an actor bind
//! is keyed on kind + collision size alone, so a render size corrected after the
//! bind never reaches the sprite (locked in `ambition_render`'s
//! `an_actor_bind_is_one_shot_so_its_geometry_must_be_complete_before_it`). A
//! snake that arrived 108x48 and was corrected to 21.3x9.5 a tick later drew at
//! five times its box for as long as the bind survived.
//!
//! ⛔ THIS INSTRUMENT SAMPLES ONCE PER TICK, so it cannot see a correction that
//! opens and closes INSIDE one update. Poisoning `ambition_body_seed` back to
//! the catalog answer leaves this test green: declaring the body source on the
//! character alone moves the resize early enough to hide inside tick 2. The
//! construction seam itself is guarded where its answer is directly readable —
//! `ambition_body_seed`'s `a_sprite_authored_body_is_constructed_from_its_sheet`
//! — and this file is the end-to-end statement that both roads agree, not the
//! proof that only one authority exists.
//!
//! ⛔ THE PLAYER HALF OF THE SAME RULE IS NOT HERE. `build_demo_app` ships no
//! `GameAssets`, so no worn player ever binds a sheet in this composition; the
//! readiness invariant is guarded where it can be observed, in
//! `ambition_render`'s `a_worn_player_is_not_finally_bound_until_its_pose_exists`.
//!
//!   cargo test -p ambition_demo_mary_o_app --test mary_o_it \
//!     snake_geometry_timeline -- --nocapture

use ambition_platformer2d::combat::components::{ActorRenderSize, ActorSpriteOffset, CenteredAabb};
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::sprite_sheet::character::SpritePosedBody;

/// Every geometry fact presentation reads, as one comparable line.
fn geometry_row(
    kin: Option<&BodyKinematics>,
    aabb: Option<&CenteredAabb>,
    render: Option<&ActorRenderSize>,
    offset: Option<&ActorSpriteOffset>,
    posed: Option<&SpritePosedBody>,
) -> String {
    format!(
        "kin={} aabb={} render={} offset={} posed={}",
        kin.map_or("-".into(), |k| format!("{:.1}x{:.1}", k.size.x, k.size.y)),
        aabb.map_or("-".into(), |a| format!(
            "{:.1}x{:.1}",
            a.half_size.x * 2.0,
            a.half_size.y * 2.0
        )),
        render.map_or("-".into(), |r| format!("{:.1}x{:.1}", r.0.x, r.0.y)),
        offset.map_or("-".into(), |o| format!("{:.1},{:.1}", o.0.x, o.0.y)),
        posed.map_or("-".into(), |p| format!("{:.3}", p.world_per_pixel)),
    )
}

/// Run `ticks` updates and return, per snake, its first geometry row and every
/// LATER row that differed from it.
fn observe(app: &mut bevy::prelude::App, ticks: usize, label: &str) -> Vec<(String, String, String)> {
    use std::collections::BTreeMap;
    let mut first: BTreeMap<String, String> = BTreeMap::new();
    let mut drift: Vec<(String, String, String)> = Vec::new();

    for tick in 0..ticks {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<(
            bevy::prelude::Entity,
            &ambition_demo_mary_o::snake::SnakeShell,
            Option<&BodyKinematics>,
            Option<&CenteredAabb>,
            Option<&ActorRenderSize>,
            Option<&ActorSpriteOffset>,
            Option<&SpritePosedBody>,
        )>();
        for (e, shell, kin, aabb, render, offset, posed) in q.iter(world) {
            // A shell that is no longer walking has legitimately changed its
            // box (a stomp shrinks it). Nothing stomps in this composition, but
            // saying so keeps the arm about construction rather than about
            // gameplay never happening to fire.
            if !matches!(shell, ambition_demo_mary_o::snake::SnakeShell::Walking) {
                continue;
            }
            let key = format!("{e:?}");
            let row = geometry_row(kin, aabb, render, offset, posed);
            match first.get(&key) {
                None => {
                    println!("[snake] {label} t={tick:>3} {key} FIRST {row}");
                    first.insert(key, row);
                }
                Some(seen) if seen != &row => {
                    println!("[snake] {label} t={tick:>3} {key} DRIFT {row}");
                    drift.push((key, seen.clone(), row));
                }
                Some(_) => {}
            }
        }
    }

    // ⛔ THE POPULATION FIRST: a road that staged no snake proves nothing, and
    // an empty `drift` would read exactly like a clean one.
    assert!(
        !first.is_empty(),
        "no snake was ever staged on the {label} road, so this measured nothing"
    );
    println!("[snake] {label}: {} snakes observed", first.len());
    drift
}

#[test]
fn a_snake_is_geometry_complete_from_its_first_observable_snapshot() {
    let mut app = ambition_demo_mary_o_app::build_demo_app();

    let fresh = observe(&mut app, 200, "fresh");
    assert!(
        fresh.is_empty(),
        "a freshly staged snake's geometry changed after it was first observable: {fresh:#?}"
    );

    // ⭐ AND THE REPLAY, because the reported symptom is fresh-start VERSUS
    // restart-after-death. Both roads run the same construction, so both must
    // produce a complete body on arrival; a rule stated on only one of them is
    // not stated.
    app.world_mut()
        .write_message(ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual());
    let replayed = observe(&mut app, 120, "replay");
    assert!(
        replayed.is_empty(),
        "a replayed snake's geometry changed after it was first observable: {replayed:#?}"
    );
}
