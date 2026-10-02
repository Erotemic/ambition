//! A clock a body is carrying, drawn where the player is already looking.
//!
//! The render half of `BodyClocksView`. The sim publishes how much of a
//! countdown a body has left; this draws a bar above the body's head that
//! shrinks with it. It does not know why the body has a clock (the delayed
//! mark today; a poison or a fuse would need no change here).
//!
//! One persistent drawable per clocked body, not a clear-and-respawn each
//! frame. The portal compositor classifies and clips a drawable that names
//! its body (`PresentationOf`), and that state (`PortalDependantHidden`, the
//! compositing candidate) lives on the entity across frames.
//!
//! A plain colour sprite on purpose: an unparented `Sprite` with a
//! `custom_size` is what the portal publisher evaluates and the far-side
//! compositor rebuilds. Bevy's 1x1 white image under the default handle is
//! what `sprite_frame_basis` needs.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, PresentationOf, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::BodyClocksView;
use bevy::prelude::*;

/// The bar drawn for one body's clock.
#[derive(Component, Debug, Clone, Copy)]
pub struct BodyClockVisual {
    pub body: Entity,
}

/// Width of a full clock, in world px. About a body's width, so it reads as
/// part of the fighter, not a HUD element.
const FULL_WIDTH: f32 = 28.0;
const HEIGHT: f32 = 4.0;
/// Gap between the top of the body and the bar.
const RISE: f32 = 10.0;
/// Above the fighters, below the portal band at `WORLD_Z_DUMMY` and up. The
/// compositor decides what a pane hides, not z.
const Z: f32 = 9.5;
/// A warning colour: something is about to happen to this body.
const COLOUR: Color = Color::srgb(1.0, 0.55, 0.1);

/// Keep one bar per clocked body, sized to what is left on the clock, and drop
/// the bar when the clock is gone.
pub fn sync_body_clock_visuals(
    mut commands: Commands,
    // Each drawable is placed by the geometry of its body's own live room.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    clocks: Res<BodyClocksView>,
    mut bars: Query<(
        Entity,
        &BodyClockVisual,
        &mut Sprite,
        &mut Transform,
        &mut Visibility,
    )>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    let mut drawn: Vec<Entity> = Vec::with_capacity(clocks.0.len());
    for (entity, bar, mut sprite, mut transform, mut visibility) in &mut bars {
        let Some(fact) = clocks.0.iter().find(|fact| fact.body == bar.body) else {
            // The clock ran out or the body left: the bar goes with it.
            commands.entity(entity).despawn();
            continue;
        };
        drawn.push(bar.body);
        // A body whose live room cannot be told keeps its last placement.
        let Some(world) = rooms.of(bar.body) else {
            continue;
        };
        sprite.custom_size = Some(bar_size(fact.remaining_fraction));
        transform.translation = bar_translation(&world.0, fact);
        // This system owns the bar's visibility every frame. The portal
        // resolver hides the bar while a pane covers it and releases without
        // writing a value when the pane moves, expecting each owner to write
        // every frame. `Inherited` is the no-opinion value; the resolver (later)
        // reasserts `Hidden` while it has a reason.
        if *visibility != Visibility::Inherited {
            *visibility = Visibility::Inherited;
        }
    }
    for fact in clocks.0.iter().filter(|fact| !drawn.contains(&fact.body)) {
        let Some(world) = rooms.of(fact.body) else {
            continue;
        };
        let mut sprite = Sprite::from_color(COLOUR, bar_size(fact.remaining_fraction));
        sprite.custom_size = Some(bar_size(fact.remaining_fraction));
        commands.spawn_session_scoped(
            session_scope,
            (
                BodyClockVisual { body: fact.body },
                sprite,
                Transform::from_translation(bar_translation(&world.0, fact)),
                // Which body this draws, in the shared spelling that consumers such as
                // the portal compositor read.
                PresentationOf(fact.body),
                Name::new("Body clock telegraph"),
            ),
        );
    }
}

/// The bar shrinks from full width to a sliver, never to nothing, so the
/// last frames still read as "almost".
fn bar_size(remaining_fraction: f32) -> Vec2 {
    Vec2::new(
        (FULL_WIDTH * remaining_fraction.clamp(0.0, 1.0)).max(2.0),
        HEIGHT,
    )
}

fn bar_translation(
    world: &ambition_platformer2d_core::World,
    fact: &ambition_sim_view::BodyClockFact,
) -> Vec3 {
    // +Y is down in world space, so "above the head" is -Y.
    ambition_platformer2d_core::config::world_to_bevy(
        world,
        fact.pos - ae::Vec2::new(0.0, fact.half_height + RISE),
        Z,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_sim_view::BodyClockFact;

    const WORLD: ae::Vec2 = ae::Vec2::new(1000.0, 600.0);

    fn app() -> App {
        let mut app = App::new();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
                "body clock",
                WORLD,
                ae::Vec2::new(WORLD.x * 0.5, WORLD.y * 0.5),
                Vec::new(),
            )),
        );
        app.init_resource::<BodyClocksView>();
        app.add_systems(Update, sync_body_clock_visuals);
        app
    }

    fn fact(body: Entity, remaining_fraction: f32) -> BodyClockFact {
        BodyClockFact {
            body,
            pos: ae::Vec2::new(300.0, 200.0),
            half_height: 16.0,
            remaining_fraction,
        }
    }

    fn bars(app: &mut App) -> Vec<(Entity, Entity, f32)> {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &BodyClockVisual, &Sprite)>();
        q.iter(world)
            .map(|(e, bar, sprite)| (e, bar.body, sprite.custom_size.unwrap().x))
            .collect()
    }

    /// The bar is the clock: it exists while the clock does, shrinks with it,
    /// and goes when the clock goes. A bar that never changed would show
    /// "marked" but not "how long".
    #[test]
    fn the_bar_follows_the_clock_and_leaves_with_it() {
        let mut app = app();
        let body = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<BodyClocksView>().0 = vec![fact(body, 1.0)];
        app.update();
        let first = bars(&mut app);
        assert_eq!(first.len(), 1, "one clocked body, one bar");
        assert_eq!(first[0].1, body);
        let full = first[0].2;

        app.world_mut().resource_mut::<BodyClocksView>().0 = vec![fact(body, 0.25)];
        app.update();
        let later = bars(&mut app);
        assert_eq!(later.len(), 1, "the same clock must not grow a second bar");
        assert_eq!(
            later[0].0, first[0].0,
            "the bar is the SAME entity, not a respawn"
        );
        assert!(
            later[0].2 < full * 0.5,
            "the bar did not shrink with the clock: {} against a full {full}",
            later[0].2
        );

        app.world_mut().resource_mut::<BodyClocksView>().0.clear();
        app.update();
        assert!(
            bars(&mut app).is_empty(),
            "the clock is gone and the bar stayed"
        );
    }

    /// Each body's clock is drawn in its body's own live room (view half,
    /// cut V2g). Two live rooms of different sizes, a clocked body in each at
    /// one simulation position: each bar is placed by its body's room and
    /// carries that room's stamp, so V3's band draws it only in the views that
    /// frame that room. Before the cut, a second live room stopped the clock
    /// bars: the system read the sole live room and did not run.
    #[test]
    fn each_clock_is_placed_and_stamped_by_its_body_s_own_live_room() {
        use ambition_platformer2d_shared_tangle::lifecycle::{spawn_live_room, InRoomInstance, LiveRoomInstance};
        let small = ae::Vec2::new(400.0, 300.0);
        let world_of = |size: ae::Vec2| {
            ambition_platformer2d_core::World::new("body clock", size, ae::Vec2::new(size.x * 0.5, size.y * 0.5), Vec::new())
        };
        let mut app = app();
        let second = LiveRoomInstance::ACTIVATION.next();
        spawn_live_room(app.world_mut(), second, ambition_platformer2d_core::RoomGeometry(world_of(small)));
        app.add_systems(
            Update,
            super::super::view_isolation::stamp_presentations_with_their_subject_s_room.after(sync_body_clock_visuals),
        );
        let home = app.world_mut().spawn(InRoomInstance(LiveRoomInstance::ACTIVATION)).id();
        let away = app.world_mut().spawn(InRoomInstance(second)).id();
        app.world_mut().resource_mut::<BodyClocksView>().0 = vec![fact(home, 1.0), fact(away, 1.0)];
        app.update();
        app.update();

        let world = app.world_mut();
        let mut q = world.query::<(&BodyClockVisual, &Transform, Option<&InRoomInstance>)>();
        let mut drawn: Vec<(Entity, Option<u32>, Vec3)> = q
            .iter(world)
            .map(|(bar, transform, stamp)| (bar.body, stamp.map(|stamp| stamp.0.ordinal()), transform.translation))
            .collect();
        drawn.sort_by_key(|(body, ..)| *body);
        let mut expected = vec![
            (home, Some(LiveRoomInstance::ACTIVATION.ordinal()), bar_translation(&world_of(WORLD), &fact(home, 1.0))),
            (away, Some(second.ordinal()), bar_translation(&world_of(small), &fact(away, 1.0))),
        ];
        expected.sort_by_key(|(body, ..)| *body);
        assert_eq!(
            drawn, expected,
            "(body, room, position) of each bar: each must be placed by its body's live room and stamped with it"
        );
    }

    /// The drawable names its body. The portal compositor needs this; a bar
    /// without an owner would be hidden whole or drawn through a pane.
    #[test]
    fn the_bar_says_whose_body_it_draws() {
        let mut app = app();
        let body = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<BodyClocksView>().0 = vec![fact(body, 0.5)];
        app.update();
        let (bar, ..) = bars(&mut app)[0];
        assert_eq!(
            app.world().get::<PresentationOf>(bar).map(|p| p.0),
            Some(body)
        );
        assert!(
            app.world().get::<ChildOf>(bar).is_none(),
            "a parented drawable is not a compositing candidate"
        );
    }
}
