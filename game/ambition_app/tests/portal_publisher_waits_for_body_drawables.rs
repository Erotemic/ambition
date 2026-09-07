//! The portal publisher runs after every body-owned drawable writer, in the
//! SHIPPED schedule -- asked of the schedule graph, not of the source.
//!
//! ⛔⛔ A SET IS A DECLARATION; THE EDGE IS THE MECHANISM. `BodyOwnedDrawableSync`
//! exists so the portal publisher classifies this frame's clock bar and flash
//! silhouette rather than last frame's or none. A writer that forgot to join
//! the set, or a publisher that dropped its `.after`, would leave the set
//! declared and the mechanism gone. The crate-level bridge tests wire the two
//! themselves, so only the real `Update` schedule can witness the wiring.

use bevy::ecs::schedule::{NodeId, ScheduleGraph, Schedules, SystemSet};
use bevy::prelude::*;

fn set_key<S: SystemSet + Copy + std::fmt::Debug>(graph: &ScheduleGraph, set: S) -> NodeId {
    NodeId::Set(
        graph
            .system_sets
            .get_key(set.intern())
            .unwrap_or_else(|| panic!("{set:?} must be a registered SystemSet")),
    )
}

/// The registered system whose TYPE is `f`'s. ⛔ BY TYPE, NOT BY NAME: system
/// names ride `bevy_utils/debug`, which only an optional GUI crate turns on, so
/// in this build every name is empty and a name lookup finds nothing.
fn system_key<M>(graph: &ScheduleGraph, f: impl IntoSystem<(), (), M>, what: &str) -> NodeId {
    let wanted = IntoSystem::into_system(f).system_type();
    let mut found = graph
        .systems
        .iter()
        .filter(|(_, system, _)| system.system_type() == wanted)
        .map(|(key, _, _)| NodeId::System(key));
    let key = found.next().unwrap_or_else(|| {
        let names: Vec<String> = graph
            .systems
            .iter()
            .take(6)
            .map(|(_, system, _)| format!("{:?}/{:?}", system.name().as_string(), system.system_type()))
            .collect();
        panic!(
            "`{what}` is not registered in Update ({} systems; wanted {wanted:?}; e.g. {names:?})",
            graph.systems.len()
        )
    });
    assert!(
        found.next().is_none(),
        "`{what}` is registered more than once in Update"
    );
    key
}

#[test]
fn the_portal_publisher_waits_for_every_body_owned_drawable_writer() {
    use ambition_platformer2d::render::rendering;
    use ambition_platformer2d::render::rendering::BodyOwnedDrawableSync;

    // ⚠ NOT UPDATED. Running the schedule moves its systems out of the graph
    // into the executor, and the graph then answers "no systems" -- 499 counted
    // and none iterable, which is how the first draft of this failed.
    let app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules
        .get(Update)
        .expect("the Update schedule exists")
        .graph();
    let set = set_key(graph, BodyOwnedDrawableSync);

    // The writers first: every one is a member of the set.
    let writers: [(NodeId, &str); 4] = [
        (
            system_key(
                graph,
                rendering::body_clock::sync_body_clock_visuals,
                "body clock",
            ),
            "body_clock::sync_body_clock_visuals",
        ),
        (
            system_key(
                graph,
                rendering::hit_flash::sync_hit_flash_overlays,
                "hit flash",
            ),
            "hit_flash::sync_hit_flash_overlays",
        ),
        (
            system_key(
                graph,
                rendering::morph_ball::sync_morph_ball_visual,
                "morph ball",
            ),
            "morph_ball::sync_morph_ball_visual",
        ),
        (
            system_key(graph, rendering::tether::sync_tether_visuals, "tether"),
            "tether::sync_tether_visuals",
        ),
    ];
    for (system, writer) in writers {
        assert!(
            graph.hierarchy().graph().contains_edge(set, system),
            "`{writer}` writes a body-owned drawable and is not in \
             `BodyOwnedDrawableSync`, so the publisher does not wait for it"
        );
    }

    // And the publisher runs after the set.
    let publisher = system_key(
        graph,
        rendering::portal_compositing::publish_portal_compositing_candidates,
        "publish_portal_compositing_candidates",
    );
    assert!(
        graph.dependency().graph().contains_edge(set, publisher),
        "the portal publisher has no edge after `BodyOwnedDrawableSync`: a clock \
         bar or flash silhouette written this frame is classified late or never"
    );
}
