//! Cameras that layer into one window must share one main texture.
//!
//! Bevy renders each camera into an intermediate main texture and blits it to
//! the camera's target. Cameras with the same (target, main-texture usages,
//! main-texture format, MSAA) share one main texture, which is how a camera
//! that does not clear (`ClearColorConfig::None`: the HUD, the cube menu, a
//! scrim) draws OVER the cameras before it. A camera whose key differs gets a
//! texture of its own: it never sees what the earlier cameras drew, and its
//! never-cleared texture is written over theirs.
//!
//! ⛔ TWICE, from two different properties. The cube menu's camera drew nothing
//! of the world until it was pinned to the gameplay camera's MSAA
//! (`Msaa::Sample4`); and on 2026-10-05 a gameplay camera put in
//! `CompositingSpace::Srgb` (main texture `Rgba8Unorm`, not the window's
//! `Rgba8UnormSrgb`) left every game a black stage under ghosting menus.
//! `capture_scene` renders into an image and saw neither.
//!
//! [`incompatible_window_stacks`] walks each window's cameras in order and
//! reports every camera that layers onto the ones before it with a different
//! key; [`report_incompatible_window_stacks`] logs each, naming both cameras.
//! Cameras that own their target (an image: the impostor atlas, a portal
//! capture) are not in a window's stack.

use bevy::camera::{CameraMainTextureUsages, ClearColorConfig, CompositingSpace, Hdr, RenderTarget};
use bevy::prelude::*;
use bevy::render::view::Msaa;
use bevy::window::{PrimaryWindow, WindowRef};

/// The format class Bevy derives a camera's main texture from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MainTextureFormat {
    /// `Hdr`: `Rgba16Float`.
    Hdr,
    /// `CompositingSpace::Srgb` (LDR): `Rgba8Unorm`, sRGB values stored raw.
    SrgbValues,
    /// The target's own format (a window: `Rgba8UnormSrgb` / `Bgra8UnormSrgb`).
    TargetFormat,
}

/// What Bevy keys a camera's main texture by, as far as the main world knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainTextureKey {
    pub usages: u32,
    pub format: MainTextureFormat,
    pub msaa: u32,
}

/// One camera's part in a window's stack.
#[derive(Debug, Clone)]
pub struct WindowCamera {
    pub name: String,
    pub window: Option<Entity>,
    pub order: isize,
    pub clears: bool,
    pub key: MainTextureKey,
    pub compositing: Option<CompositingSpace>,
    pub hdr: bool,
}

/// A camera that draws over earlier cameras of its window with another main
/// texture.
#[derive(Debug, Clone)]
pub struct Incompatibility {
    pub below: WindowCamera,
    pub above: WindowCamera,
}

impl std::fmt::Display for Incompatibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let describe = |camera: &WindowCamera| {
            format!(
                "  {} (order {}, {}):\n    usages = {:#x}\n    format = {:?}\n    msaa = {}\n    compositing = {:?}\n    hdr = {}",
                camera.name,
                camera.order,
                if camera.clears { "clears" } else { "draws over the cameras before it" },
                camera.key.usages,
                camera.key.format,
                camera.key.msaa,
                camera.compositing,
                camera.hdr,
            )
        };
        write!(
            f,
            "direct-window camera stack is incompatible: these cameras are expected to accumulate into one \
             main texture of the same window, and Bevy will give them two (the later one's never-cleared \
             texture is written over the earlier one's picture).\n{}\n{}",
            describe(&self.below),
            describe(&self.above)
        )
    }
}

/// Every camera that draws over the earlier cameras of its window with a
/// different main-texture key, paired with the camera it draws over. A
/// clearing camera starts its window's stack afresh; inactive cameras draw
/// nothing and are left out by the caller.
pub fn incompatible_window_stacks(mut cameras: Vec<WindowCamera>) -> Vec<Incompatibility> {
    cameras.sort_by(|a, b| a.window.cmp(&b.window).then(a.order.cmp(&b.order)));
    let mut found = Vec::new();
    let mut below: Option<&WindowCamera> = None;
    for camera in &cameras {
        let same_window = below.is_some_and(|below| below.window == camera.window);
        if !same_window {
            below = Some(camera);
            continue;
        }
        let base = below.expect("checked");
        if !camera.clears && camera.key != base.key {
            found.push(Incompatibility {
                below: base.clone(),
                above: camera.clone(),
            });
        }
        if camera.clears {
            below = Some(camera);
        }
    }
    found
}

type CameraFacts = (
    Entity,
    &'static Camera,
    &'static RenderTarget,
    Option<&'static Msaa>,
    Option<&'static CompositingSpace>,
    Has<Hdr>,
    Option<&'static CameraMainTextureUsages>,
    Option<&'static Name>,
);

/// The active cameras that draw into a window, as [`incompatible_window_stacks`]
/// reads them.
pub fn window_cameras(world: &mut World) -> Vec<WindowCamera> {
    let primary = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .iter(world)
        .next();
    let mut cameras = world.query::<CameraFacts>();
    cameras
        .iter(world)
        .filter(|(_, camera, ..)| camera.is_active)
        .filter_map(|(entity, camera, target, msaa, compositing, hdr, usages, name)| {
            let RenderTarget::Window(window) = target else {
                return None;
            };
            // `Primary` and the primary window's entity are one window; with no
            // window open (a headless test), `Primary` is still one target.
            let window = match window {
                WindowRef::Primary => primary,
                WindowRef::Entity(entity) => Some(*entity),
            };
            let format = if hdr {
                MainTextureFormat::Hdr
            } else if compositing == Some(&CompositingSpace::Srgb) {
                MainTextureFormat::SrgbValues
            } else {
                MainTextureFormat::TargetFormat
            };
            Some(WindowCamera {
                name: name.map_or_else(|| format!("{entity}"), |name| format!("{name} ({entity})")),
                window,
                order: camera.order,
                clears: !matches!(camera.clear_color, ClearColorConfig::None),
                key: MainTextureKey {
                    usages: usages.copied().unwrap_or_default().0.bits(),
                    format,
                    msaa: msaa.copied().unwrap_or_default().samples(),
                },
                compositing: compositing.copied(),
                hdr,
            })
        })
        .collect()
}

/// Log every incompatible window stack once per change in the camera set.
pub fn report_incompatible_window_stacks(world: &mut World, mut last: Local<Vec<String>>) {
    let found: Vec<String> = incompatible_window_stacks(window_cameras(world))
        .iter()
        .map(ToString::to_string)
        .collect();
    if *last != found {
        for message in &found {
            error!("{message}");
        }
        *last = found;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera(name: &str, order: isize, clears: bool, format: MainTextureFormat, msaa: u32) -> WindowCamera {
        WindowCamera {
            name: name.to_owned(),
            window: None,
            order,
            clears,
            key: MainTextureKey { usages: 1, format, msaa },
            compositing: None,
            hdr: false,
        }
    }

    /// The two failures this exists for, and the stacks that are fine.
    #[test]
    fn a_camera_drawing_over_another_with_another_main_texture_is_reported() {
        use MainTextureFormat::*;
        // The 2026-10-05 black stage: a gamma gameplay camera under the HUD.
        let found = incompatible_window_stacks(vec![
            camera("world", 0, true, SrgbValues, 4),
            camera("hud", 9, false, TargetFormat, 4),
        ]);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].below.name.as_str(), found[0].above.name.as_str()), ("world", "hud"));
        // The cube menu at another MSAA.
        assert_eq!(
            incompatible_window_stacks(vec![camera("world", 0, true, TargetFormat, 4), camera("cube", 8, false, TargetFormat, 1)]).len(),
            1
        );
        // Matching keys layer fine; a clearing camera starts afresh; another
        // window is another stack.
        assert!(incompatible_window_stacks(vec![
            camera("world", 0, true, TargetFormat, 4),
            camera("cube", 8, false, TargetFormat, 4),
            camera("hud", 9, false, TargetFormat, 4),
        ])
        .is_empty());
        assert!(incompatible_window_stacks(vec![camera("world", 0, true, SrgbValues, 4), camera("menu", 5, true, TargetFormat, 1)]).is_empty());
        let mut other = camera("other window", 1, false, TargetFormat, 1);
        other.window = Some(Entity::from_raw_u32(7).unwrap());
        assert!(incompatible_window_stacks(vec![camera("world", 0, true, SrgbValues, 4), other]).is_empty());
    }
}
