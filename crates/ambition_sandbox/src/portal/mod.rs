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

    use ambition_platformer_runtime::world_query::SolidWorldQuery;
    use ambition_portal_presentation::{
        PortalDebugOverlay, PortalGunArt, PortalSceneBody, PortalViewer, PortalWorldFrame,
    };

    use crate::abilities::traversal::possession::PossessionState;
    use crate::features::FeatureAabb;
    use crate::platformer_runtime::body::BodyKinematics;
    use crate::player::{PlayerEntity, PrimaryPlayer};
    use crate::platformer_runtime::lifecycle::PlayerVisual;
    use crate::GameWorld;

    /// Bridge the controlled character + the collision world → the crate-owned
    /// [`PortalViewer`] seam, so each portal window is the wedge that character
    /// can actually see through the aperture. The eye is the possessed actor's
    /// body when possessing (so the view follows the body you're driving), else
    /// the primary player's; `occluders` is a snapshot of the world's solid
    /// blocks for the line-of-sight test. Absent player/possessed body ⇒
    /// `present = false`, and the renderer falls back to the static window.
    pub fn sync_portal_viewer(
        world: Res<GameWorld>,
        possession: Res<PossessionState>,
        feature_aabbs: Query<&FeatureAabb>,
        player: Query<&BodyKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>,
        viewer: Option<ResMut<PortalViewer>>,
    ) {
        let Some(mut viewer) = viewer else {
            return;
        };
        let body = possession
            .possessed
            .and_then(|e| feature_aabbs.get(e).ok().map(|a| (a.center, a.half_size)))
            .or_else(|| player.single().ok().map(|k| (k.pos, k.size * 0.5)));
        match body {
            Some((eye, half_size)) => {
                viewer.present = true;
                viewer.eye = eye;
                viewer.half_size = half_size;
                viewer.occluders.clear();
                world
                    .0
                    .for_each_solid_aabb(false, &mut |aabb| viewer.occluders.push(aabb));
            }
            None => viewer.present = false,
        }
    }

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

    /// Drive the portal debug overlay's host-side F1 gate from the standard
    /// `SandboxDevState.debug` flag, so the portal gizmos stay quiet unless the
    /// global debug overlay is on.
    pub fn sync_portal_debug_overlay_to_f1(
        dev_state: Res<crate::SandboxDevState>,
        debug: Option<ResMut<PortalDebugOverlay>>,
    ) {
        if let Some(mut debug) = debug {
            debug.enabled = dev_state.debug;
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

    /// Dev: `F10` flips the game-wide portal map CONVENTION live, to A/B the
    /// feel — reflection (det −1, default: tangent preserved, facing/thin-wall
    /// pairs vertically flip) vs rotation (det +1: facing/thin-wall pairs are a
    /// clean straight-through "door", floor↔floor reverses horizontal). Affects
    /// transit, the view cones, the body copy, and collision pieces together.
    pub fn portal_convention_toggle_system(
        keys: Res<ButtonInput<KeyCode>>,
        tuning: Option<ResMut<ambition_portal::PortalTuning>>,
    ) {
        if !keys.just_pressed(KeyCode::F10) {
            return;
        }
        let next = !ambition_portal::portal_map_rotation();
        if let Some(mut tuning) = tuning {
            tuning.convention = ambition_portal::PortalConvention::from_rotation(next);
        }
        ambition_portal::set_portal_map_rotation(next);
        bevy::log::info!(
            target: "ambition::portal",
            "portal map convention = {}",
            if next { "rotation (det +1)" } else { "reflection (det -1)" }
        );
    }
}

#[cfg(feature = "portal_render")]
pub use host_adapter::{
    load_portal_gun_art, portal_convention_toggle_system, portal_dev_toggle_system,
    sync_portal_debug_overlay_to_f1, sync_portal_viewer, sync_portal_world_frame,
    tag_portal_scene_bodies,
};
