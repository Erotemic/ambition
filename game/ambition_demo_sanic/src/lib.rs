//! Sanic-style demo content home.
//!
//! This crate intentionally depends only on the `ambition` facade crate. It is
//! a compile-time oracle: new demo/game content should start here, not by
//! copying `game/ambition_app`'s direct dependency wall.

use ambition::prelude::*;

/// Empty first-cut content plugin for the Sanic movement demo home.
pub struct SanicDemoContentPlugin;

impl Plugin for SanicDemoContentPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Install the Sanic demo content layer into an engine app.
pub fn add_demo_content(app: &mut App) {
    app.add_plugins(SanicDemoContentPlugin);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanic_demo_content_plugin_installs() {
        let mut app = App::new();
        add_demo_content(&mut app);
    }
}
