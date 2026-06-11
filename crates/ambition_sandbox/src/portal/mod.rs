//! Portal mechanic facade + the Ambition host adapter for portal presentation.
//!
//! The portal **mechanic** (the portal gun place/replace/channel, the one
//! generic aperture transit over `PortalBody` + `PortalPolicy`, placement +
//! transit math, carve publishing, pieces geometry, lifecycle, the pure shot
//! helper over `SolidWorldQuery`, the portal events + `PortalPlugin`) lives in
//! the standalone, content-free crate
//! [`ambition_portal`](https://docs.rs/ambition_portal) (Stage 19; ADR 0019),
//! and the portal **visuals** (placed-portal quads + labels, the held / pickup
//! gun sprite, mid-transit body pieces, the disorientation indicator, the
//! through-portal view cones) live in its reusable default renderer
//! [`ambition_portal_presentation`]. This module is a thin FACADE that
//! re-exports both so every inbound `crate::portal::…` path keeps resolving
//! with zero churn, plus the Ambition-specific glue that is NOT reusable:
//!
//! - the **presentation host adapter** (render-gated): sync the crate-owned
//!   [`PortalWorldFrame`] from [`GameWorld`], tag [`PortalSceneBody`] on the
//!   player's visual entity, and load [`PortalGunArt`] from the Ambition asset
//!   paths. The presentation crate never names a host type; these three
//!   systems are the entire bridge.
//! - the `F7` dev off-switch (raw keyboard = host input concern).
//!
//! The Ambition adapters that bridge the MECHANIC's seams to game concepts
//! (input → fire intent, carve → collision overlay, room-reset → clear, sfx,
//! player input / ability shaping, identity → policy tagging) live in
//! [`crate::ambition_content::portal`]; the portal integration tests live in
//! `ambition_content::portal::tests`.

// The whole reusable mechanic, surfaced at the historic `crate::portal::…` paths.
pub use ambition_portal::*;

// The whole reusable default renderer, surfaced at the same historic paths
// (`crate::portal::sync_portal_visuals`, `crate::portal::PortalAimHint`, …).
// Render only — exclusively behind `portal_render`, so the portal *simulation*
// builds without any render-facing systems or components.
#[cfg(feature = "portal_render")]
pub use ambition_portal_presentation::*;

#[cfg(feature = "portal_render")]
mod host_adapter {
    use bevy::prelude::*;

    use ambition_portal_presentation::{
        PortalGunArt, PortalSceneBody, PortalViewConeConfig, PortalWorldFrame,
    };

    use crate::presentation::rendering::PlayerVisual;
    use crate::GameWorld;

    /// Bridge [`GameWorld`] → the crate-owned [`PortalWorldFrame`] seam: the
    /// presentation crate only ever needs the world's size for its centered
    /// y-flip render transform, so the host copies that one field each frame
    /// (room transitions resize the world).
    pub fn sync_portal_world_frame(world: Res<GameWorld>, mut frame: ResMut<PortalWorldFrame>) {
        if frame.size != world.0.size {
            frame.size = world.0.size;
        }
    }

    /// Bridge the host's player-visual marker → the crate-owned
    /// [`PortalSceneBody`] seam, so the mid-transit body-piece decomposition
    /// draws the player's sprite without the presentation crate naming
    /// [`PlayerVisual`].
    pub fn tag_portal_scene_bodies(
        mut commands: Commands,
        untagged: Query<Entity, (With<PlayerVisual>, Without<PortalSceneBody>)>,
    ) {
        for entity in &untagged {
            commands.entity(entity).insert(PortalSceneBody);
        }
    }

    /// Load the portal-gun mode sprites at startup into the crate-owned
    /// [`PortalGunArt`] seam — asset PATHS are Ambition content, so loading
    /// stays host-side.
    pub fn load_portal_gun_art(mut commands: Commands, assets: Res<AssetServer>) {
        commands.insert_resource(PortalGunArt {
            blue: assets.load("sprites/props/portal_gun_blue.png"),
            orange: assets.load("sprites/props/portal_gun_orange.png"),
        });
    }

    /// Drive the portal view-cone debug outline (each portal's exit sample
    /// zone + entry window trapezoid) off the standard `F1` debug overlay,
    /// rather than a key of its own — so it shows exactly when the rest of the
    /// F1 debug drawing is on. Mirrors the `SandboxDevState.debug` flag into
    /// the crate-owned config each frame.
    pub fn sync_portal_view_debug_to_f1(
        dev_state: Res<crate::SandboxDevState>,
        config: Option<ResMut<PortalViewConeConfig>>,
    ) {
        if let Some(mut config) = config {
            if config.debug_outline != dev_state.debug {
                config.debug_outline = dev_state.debug;
            }
        }
    }

    /// Dev off-switch: `F7` toggles the portal gun active/inactive so the
    /// always-on slice gun doesn't fire portals on every Attack while testing
    /// other sandbox mechanics. (Visible build only.) Final gating is via
    /// held-item equip; this is a developer convenience until then.
    ///
    /// This reads raw keyboard input (a host input / dev concern), so it lives
    /// host-side rather than in a portal crate — it just flips
    /// `PortalGun.active` the way the crate's message-driven toggle would.
    pub fn portal_dev_toggle_system(
        keys: Res<ButtonInput<KeyCode>>,
        mut guns: Query<&mut ambition_portal::PortalGun>,
    ) {
        if !keys.just_pressed(KeyCode::F7) {
            return;
        }
        for mut gun in &mut guns {
            gun.active = !gun.active;
            bevy::log::info!(target: "ambition::portal", "portal gun active = {}", gun.active);
        }
    }
}

#[cfg(feature = "portal_render")]
pub use host_adapter::{
    load_portal_gun_art, portal_dev_toggle_system, sync_portal_view_debug_to_f1,
    sync_portal_world_frame, tag_portal_scene_bodies,
};
