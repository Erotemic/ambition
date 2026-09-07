//! The Author's low poke leaves a note, and the note is a whole feature.
//!
//! ⭐⭐ THE ROAD, END TO END, IN THE SHIPPED COMPOSITION: the Author's real
//! `author_tilt_down` → a real strike volume → the real collision resolver →
//! `OnHitEffectMessage` → `BodyMark` on the victim → a clock the player can read,
//! drawn above the body → the authored fuse → a `DamageBox` → real combat
//! damage, credited to the Author. Every mark test before this one began by
//! WRITING the on-hit message into a minimal app, which proves the ruleset's
//! arithmetic and nothing about whether the move reaches it. A GPT review asked
//! for this road, 2026-09-07.
//!
//! ⛔⛔ AND THE NOTE DOES NOT SURVIVE A STOCK. The second test KOs the marked
//! fighter before the fuse runs out and asks that the mark is gone during the
//! death interlude and that nothing goes off on the next stock. With the
//! authored 1.4s fuse and the stage's 1.0s interlude, the shipped code before
//! this detonated a previous stock's mark on a fresh body.
//!
//! ⚠ IN `ambition_app`, NOT THE STANDALONE DEMO, because the Author is content
//! this composition carries and the demo's grid cannot seat him.

use ambition_demo_smash::mark::BodyMark;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::combat::moveset::{ActorMoveset, MovePlayback};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use bevy::prelude::*;

const AUTHOR: &str = "author";
const TILT_DOWN: &str = "author_tilt_down";

/// Every `DamageBox` request the ruleset made, as it was made. ⛔ Accumulated
/// as written: a message buffer read after the fact has already dropped it.
#[derive(Resource, Default)]
struct Blasts {
    detonations: usize,
    last_owner: Option<Entity>,
}

fn capture_blasts(
    mut reader: MessageReader<ambition_platformer2d::vfx::EffectRequest>,
    mut blasts: ResMut<Blasts>,
) {
    for request in reader.read() {
        if let ambition_platformer2d::vfx::Effect::DamageBox(b) = &request.effect {
            if b.name == Some("mark detonation") {
                blasts.detonations += 1;
                blasts.last_owner = Some(request.owner);
            }
        }
    }
}

fn body_of_seat(app: &mut App, seat: usize) -> Entity {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &MatchSeat)>();
    q.iter(world)
        .find(|(_, s)| s.0 == seat)
        .map(|(e, _)| e)
        .unwrap_or_else(|| panic!("no body is seated at {seat}"))
}

/// The stocks METER, not the pool: a stocks fighter's death policy is
/// unbounded, so `health.current` sits at its maximum while `damage_taken`
/// climbs — the number the HUD prints as a percentage.
fn damage_taken_by(app: &App, body: Entity) -> i32 {
    app.world()
        .get::<BodyHealth>(body)
        .expect("a fighter has a meter")
        .damage_taken()
}

/// A settled match: the Author at seat 0, a target at seat 1. ⭐ BOTH HUMAN
/// SEATS ON PADS NOBODY IS HOLDING, so neither body moves unless this test
/// moves it — a CPU target walks out of the volume the test stands it in.
fn a_settled_match() -> (App, Entity, Entity) {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.init_resource::<Blasts>();
    app.add_systems(Last, capture_blasts);
    for _ in 0..30 {
        app.update();
    }
    let mut roster = ambition_demo_smash::smash_roster([AUTHOR, AUTHOR]);
    roster.participants[1].controller = ambition_platformer2d::actor::ControllerBinding::Human {
        source: ambition_platformer2d::actor::LocalInputSource::Pad(1),
    };
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));
    for _ in 0..900 {
        app.update();
        let (seated, held) = {
            let world = app.world_mut();
            let mut all = world.query::<&MatchSeat>();
            let seated = all.iter(world).count();
            let mut q = world.query_filtered::<
                &MatchSeat,
                With<ambition_platformer2d::characters::control::ScriptedControl>,
            >();
            (seated, q.iter(world).count())
        };
        if seated >= 2 && held == 0 {
            break;
        }
    }
    let author = body_of_seat(&mut app, 0);
    let target = body_of_seat(&mut app, 1);
    (app, author, target)
}

/// Put the target inside the tilt's first damaging volume and start the real
/// move on the Author. Returns the tick the mark appeared on, or panics.
fn land_the_tilt(app: &mut App, author: Entity, target: Entity) -> usize {
    let spec = app
        .world()
        .get::<ActorMoveset>(author)
        .expect("the Author wears a moveset")
        .0
        .moves
        .iter()
        .find(|m| m.id == TILT_DOWN)
        .unwrap_or_else(|| panic!("the Author has no `{TILT_DOWN}`"))
        .clone();
    // ⚠ THE MOVE'S OWN GEOMETRY decides where the target stands: a position
    // typed here would be a second answer to "where does this hit" that drifts
    // from the authored one without failing.
    let offset = spec
        .windows
        .iter()
        .flat_map(|w| w.volumes.iter())
        .find(|v| v.damage > 0)
        .map(|v| match v.shape {
            ambition_platformer2d::entity_catalog::VolumeShape::Rect { offset, .. } => offset,
            ambition_platformer2d::entity_catalog::VolumeShape::Circle { offset, .. } => offset,
        })
        .expect("the tilt has a damaging volume, or it could not carry a mark");
    let facing = {
        let world = app.world_mut();
        let author_kin = *world
            .get::<ae::BodyKinematics>(author)
            .expect("the Author has a body");
        let facing = if author_kin.facing == 0.0 {
            1.0
        } else {
            author_kin.facing
        };
        let mut target_kin = world
            .get_mut::<ae::BodyKinematics>(target)
            .expect("the target has a body");
        target_kin.pos = author_kin.pos + ae::Vec2::new(facing * offset.0, offset.1);
        target_kin.vel = ae::Vec2::ZERO;
        facing
    };
    app.world_mut()
        .entity_mut(author)
        .insert(MovePlayback::new(spec, facing));
    for tick in 0..90 {
        app.update();
        if app.world().get::<BodyMark>(target).is_some() {
            return tick;
        }
    }
    panic!(
        "the Author's real `{TILT_DOWN}` never marked a target standing inside \
         its authored volume: the road from the move to the mark is broken \
         somewhere between the strike and `apply_authored_body_marks`"
    );
}

#[test]
fn the_real_down_tilt_marks_the_target_who_can_read_it_and_is_then_hit_by_the_author() {
    let (mut app, author, target) = a_settled_match();
    land_the_tilt(&mut app, author, target);

    // THE READ, both halves: the sim publishes a clock row for the body, and
    // the presentation draws a bar that names that body.
    app.update();
    app.update();
    let clocks = app
        .world()
        .resource::<ambition_platformer2d::sim_view::BodyClocksView>()
        .0
        .clone();
    assert!(
        clocks
            .iter()
            .any(|c| c.body == target && c.remaining_fraction > 0.5),
        "the target is marked and no readable clock names them: {clocks:?}"
    );
    {
        use ambition_platformer2d::platformer::lifecycle::PresentationOf;
        use ambition_platformer2d::render::rendering::body_clock::BodyClockVisual;
        let world = app.world_mut();
        let mut bars = world.query::<(&BodyClockVisual, &PresentationOf, &Sprite)>();
        let drawn: Vec<_> = bars
            .iter(world)
            .filter(|(bar, of, _)| bar.body == target && of.0 == target)
            .collect();
        assert_eq!(
            drawn.len(),
            1,
            "the marked target has a clock and nothing on screen draws it"
        );
    }

    // Let the tilt finish so its own damage is out of the comparison.
    for _ in 0..90 {
        app.update();
        if app.world().get::<MovePlayback>(author).is_none() {
            break;
        }
    }
    assert!(
        app.world().get::<BodyMark>(target).is_some(),
        "the mark did not outlive the move that applied it, so nothing below \
         measures the fuse"
    );
    let before = damage_taken_by(&app, target);
    assert_eq!(
        app.world().resource::<Blasts>().detonations,
        0,
        "the note went off before its fuse"
    );

    // THE FUSE, then the blast, then real damage.
    let mut went_off_on = None;
    for tick in 0..180 {
        app.update();
        if app.world().get::<BodyMark>(target).is_none() {
            went_off_on = Some(tick);
            break;
        }
    }
    let went_off_on = went_off_on.expect("the mark never went off");
    assert!(
        went_off_on > 30,
        "the fuse ran out after {went_off_on} ticks; 1.4s is about 84"
    );
    for _ in 0..12 {
        app.update();
    }
    let blasts = app.world().resource::<Blasts>();
    assert_eq!(blasts.detonations, 1, "one note, one detonation");
    assert_eq!(
        blasts.last_owner,
        Some(author),
        "the detonation is credited to the marked target rather than to the \
         Author who left the note"
    );
    let after = damage_taken_by(&app, target);
    assert!(
        after > before,
        "the note went off and the target's meter did not move ({before} -> {after}): \
         the blast was requested and never became combat damage"
    );
}

#[test]
fn a_note_on_a_fighter_who_loses_the_stock_does_not_go_off_on_the_next_one() {
    use ambition_platformer2d::actor::{FighterStocks, PendingRespawn};

    let (mut app, author, target) = a_settled_match();
    land_the_tilt(&mut app, author, target);
    let stocks_before = app
        .world()
        .get::<FighterStocks>(target)
        .expect("a stocks fighter")
        .remaining;

    // THE KO, well inside the fuse. ⚠ PLACED PAST THE BLAST LINE AND LAUNCHED
    // OUTWARD, rather than launched from the stage: a launch from centre-stage
    // takes the ruleset longer to settle than the 1.4s fuse this test needs the
    // KO to land inside, and a mark that expired first would make the
    // assertions below true for the wrong reason.
    {
        let mut kin = app
            .world_mut()
            .get_mut::<ae::BodyKinematics>(target)
            .expect("the target has a body");
        kin.pos = ae::Vec2::new(-400.0, kin.pos.y);
        kin.vel = ae::Vec2::new(-2_400.0, -200.0);
    }
    let mut spent_on = None;
    for tick in 0..60 {
        app.update();
        let now = app
            .world()
            .get::<FighterStocks>(target)
            .expect("still a stocks fighter")
            .remaining;
        if now < stocks_before {
            spent_on = Some(tick);
            break;
        }
    }
    let spent_on = spent_on.expect("the launched target never spent a stock");
    // One more tick for the ruleset's decision to reach the mark.
    app.update();
    assert!(
        app.world().get::<PendingRespawn>(target).is_some(),
        "premise: the target is waiting out its death interlude"
    );
    assert!(
        app.world().get::<BodyMark>(target).is_none(),
        "the stock was spent on tick {spent_on} and the mark is still on the \
         body during the death interlude — it will go off on the next stock"
    );
    assert_eq!(
        app.world().resource::<Blasts>().detonations,
        0,
        "a note detonated on or before the KO"
    );

    // Through the interlude, the respawn, and well past where the old fuse
    // would have run out.
    for _ in 0..240 {
        app.update();
    }
    assert!(
        app.world().get::<PendingRespawn>(target).is_none(),
        "premise: the target came back on a fresh stock"
    );
    assert_eq!(
        app.world().resource::<Blasts>().detonations,
        0,
        "a previous stock's note went off on the fresh one"
    );
    assert!(
        app.world().get::<BodyMark>(target).is_none(),
        "the fresh stock came back carrying the old note"
    );
}
