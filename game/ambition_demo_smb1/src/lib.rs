//! SMB1-style demo content home.
//!
//! This crate intentionally depends only on the `ambition` facade crate. It is
//! a compile-time oracle for building a second platformer by adding content,
//! rather than editing the engine or app shell.

use ambition::prelude::*;

/// Empty first-cut content plugin for the SMB1 movement demo home.
pub struct Smb1DemoContentPlugin;

impl Plugin for Smb1DemoContentPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Install the SMB1 demo content layer into an engine app.
pub fn add_demo_content(app: &mut App) {
    app.add_plugins(Smb1DemoContentPlugin);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smb1_demo_content_plugin_installs() {
        let mut app = App::new();
        add_demo_content(&mut app);
    }
}
