//! A consumer with no background bindings has no inherited background art.

#![cfg(feature = "visible")]

use ambition_platformer2d::view::ParallaxLayerVisual;

#[test]
fn an_independent_game_has_no_implicit_shared_backdrop() {
    let mut app = outlander::stepped_windowed_app();
    outlander::run_outlander_walkthrough(&mut app).expect("the consumer's session activates");
    for _ in 0..60 {
        app.update();
    }
    let mut layers = app.world_mut().query_filtered::<(), bevy::prelude::With<ParallaxLayerVisual>>();
    assert_eq!(layers.iter(app.world()).count(), 0);
}
