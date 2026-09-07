//! A clock a body is carrying, drawn where the player is already looking.
//!
//! ⭐ THE RENDER HALF OF `BodyClocksView`. The sim publishes "this body has this
//! much of a countdown left"; this draws a bar above the body's head that
//! shrinks with it. It knows nothing about WHY the body has a clock — the
//! delayed mark is the first customer and a poison or a fuse would be the next,
//! and neither would touch this file.
//!
//! ⛔ ONE PERSISTENT DRAWABLE PER CLOCKED BODY, not a clear-and-respawn every
//! frame like the recall beacon. A drawable that names its body
//! (`PresentationOf`) is what the portal compositor classifies and clips, and
//! that bookkeeping (`PortalDependantHidden`, the compositing candidate) rides
//! the drawable's entity across frames; an entity that is new every frame would
//! be claimed and discarded before the claim could act.
//!
//! ⭐ A PLAIN COLOUR SPRITE, DELIBERATELY. An unparented `Sprite` with a
//! `custom_size` is the population the portal publisher evaluates and the
//! far-side compositor rebuilds, so this telegraph is portal-correct by
//! construction rather than by a later workaround. Bevy inserts a 1x1 white
//! image under the default handle, which is what `sprite_frame_basis` needs.

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

/// Width of a full clock, in world px. About a body's width, so the read is
/// "a bar the size of the fighter" rather than a HUD element.
const FULL_WIDTH: f32 = 28.0;
const HEIGHT: f32 = 4.0;
/// Gap between the top of the body and the bar.
const RISE: f32 = 10.0;
/// Above the fighters, below the panes the portal band pins at `WORLD_Z_DUMMY`
/// and above: the compositor decides what a pane hides, not z.
const Z: f32 = 9.5;
/// A warning colour, so the read is "something is about to happen to you".
const COLOUR: Color = Color::srgb(1.0, 0.55, 0.1);

/// Keep one bar per clocked body, sized to what is left on the clock, and drop
/// the bar when the clock is gone.
pub fn sync_body_clock_visuals(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    clocks: Res<BodyClocksView>,
    mut bars: Query<(Entity, &BodyClockVisual, &mut Sprite, &mut Transform)>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    let mut drawn: Vec<Entity> = Vec::with_capacity(clocks.0.len());
    for (entity, bar, mut sprite, mut transform) in &mut bars {
        let Some(fact) = clocks.0.iter().find(|fact| fact.body == bar.body) else {
            // The clock ran out or the body left: the bar goes with it.
            commands.entity(entity).despawn();
            continue;
        };
        drawn.push(bar.body);
        sprite.custom_size = Some(bar_size(fact.remaining_fraction));
        transform.translation = bar_translation(&world.0, fact);
    }
    for fact in clocks.0.iter().filter(|fact| !drawn.contains(&fact.body)) {
        let mut sprite = Sprite::from_color(COLOUR, bar_size(fact.remaining_fraction));
        sprite.custom_size = Some(bar_size(fact.remaining_fraction));
        commands.spawn_session_scoped(
            session_scope,
            (
                BodyClockVisual { body: fact.body },
                sprite,
                Transform::from_translation(bar_translation(&world.0, fact)),
                // ⭐ WHOSE BODY THIS DRAWS, in the one spelling every consumer
                // asks for — the portal compositor among them.
                PresentationOf(fact.body),
                Name::new("Body clock telegraph"),
            ),
        );
    }
}

/// The bar shrinks from the full width to nothing; never below a sliver, so the
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
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
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

    /// ⭐ THE BAR IS THE CLOCK: it exists while the clock does, shrinks with it,
    /// and goes when the clock goes. A telegraph that appeared and never
    /// changed would say "marked" and not "how long".
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

    /// ⛔ THE DRAWABLE NAMES ITS BODY, which is the whole reason it is a
    /// persistent unparented sprite: that is what the portal compositor asks
    /// for, and a bar that did not say whose it was would be hidden wholesale
    /// or drawn through a pane.
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
