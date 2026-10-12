//! In the drawn demo, the visual of each void body carries its aura (review
//! A5, R6). The aura is keyed by the live room and the id of the body, so the
//! visual and the body must carry the same live room stamp; this measures
//! that on the shipped spawn roads.
#![cfg(feature = "visible")]

use bevy::prelude::*;

use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::render::rendering::FeatureVisual;
use ambition_platformer2d::render::void_field::VoidAura;

/// ⭐ Each void body of 1-3 has a visual with an aura. A headless drawn demo
/// has no render app, so `void_look::install` adds nothing: the system is
/// added here, as `install` adds it.
#[test]
fn each_void_body_of_the_drawn_demo_has_a_visual_with_its_aura() {
    let mut app = ambition_demo_mary_o_app::build_windowed_demo_app(ambition_demo_mary_o_app::RenderMode::Headless);
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom("mary_o_1_3".to_string()));
    app.add_systems(Update, ambition_demo_mary_o::void_look::reconcile_void_auras);
    for _ in 0..300 {
        app.update();
    }
    let world = app.world_mut();
    let void_ids: Vec<String> = world
        .query::<(&FeatureId, &ActorConfig)>()
        .iter(world)
        .filter(|(_, config)| format!("{:?}", config.brain).contains("void"))
        .map(|(id, _)| id.as_str().to_owned())
        .collect();
    assert_eq!(void_ids.len(), 2, "the premise: 1-3 has its two Void Stars");
    let with_aura: Vec<String> = world
        .query::<(&FeatureVisual, Option<&VoidAura>)>()
        .iter(world)
        .filter(|(visual, aura)| void_ids.contains(&visual.id) && aura.is_some())
        .map(|(visual, _)| visual.id.clone())
        .collect();
    assert_eq!(with_aura.len(), void_ids.len(), "each void body's visual has its aura: {with_aura:?} of {void_ids:?}");
}
