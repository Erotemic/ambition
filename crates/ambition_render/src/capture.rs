//! Generic offscreen capture plumbing for composed apps. This module owns the
//! render target, camera adoption, readback, file write, and exit. The caller
//! owns scene-readiness policy and must not request capture before its required
//! presentation state exists.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::render::view::Msaa;

use ambition_platformer2d_shared_tangle::camera_layers::{FrontHudCamera, MainCamera};

/// Where the picture goes and how big it is. Everything a capture needs to know
/// that has nothing to do with which game is being captured.
#[derive(Resource, Clone, Debug)]
pub struct CaptureSettings {
    pub output: PathBuf,
    pub size: UVec2,
    /// Draw the HUD/UI camera into the shot as well as the world camera.
    pub include_ui: bool,
}

/// The render target the cameras were pointed at, once [`setup_capture_target`]
/// has built it. Absent until then, which is a caller's cue that there is
/// nothing to read back yet.
#[derive(Resource, Debug)]
pub struct CaptureTarget {
    pub image: Handle<Image>,
    /// How many cameras have been pointed at it. Zero means nothing is drawing
    /// into this texture, and a caller that shoots anyway writes a transparent
    /// PNG and calls it a success.
    pub adopted: u32,
}

/// How far along the capture is. A caller owns the policy; this is the record.
#[derive(Resource, Debug, Default)]
pub struct CaptureProgress {
    /// A readback has been asked for — do not ask twice.
    pub requested: bool,
    /// The readback arrived and the file was written (or failed).
    pub completed: bool,
    /// Something went wrong; no image was written, and the exit code says so.
    pub failed: bool,
}

/// Build the offscreen texture. Camera adoption is a separate step and may
/// happen after cameras are created.
pub fn setup_capture_target(
    mut commands: Commands,
    settings: Res<CaptureSettings>,
    mut images: ResMut<Assets<Image>>,
) {
    if let Some(parent) = settings
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        if let Err(error) = std::fs::create_dir_all(parent) {
            eprintln!(
                "capture: failed to create output directory '{}': {error}",
                parent.display()
            );
            commands.write_message(AppExit::from_code(2));
            return;
        }
    }

    let mut capture_image = Image::new_target_texture(
        settings.size.x.max(1),
        settings.size.y.max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    capture_image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let image = images.add(capture_image);
    commands.insert_resource(CaptureTarget { image, adopted: 0 });
}

/// Point every camera that exists at the capture target, every frame.
///
/// The PNG is transparent and the tool reports success. That is exactly what Mary-O's first capture
/// produced, and it took reading the pixel values to tell it apart from "the scene is white".
///
///  WHEN a camera appears is composition-specific and therefore not knowable
/// here — which is the same reason readiness belongs to the caller. So this
/// runs every frame and counts what it has adopted; a caller shoots only once
/// [`CaptureTarget::adopted`] is non-zero.
pub fn adopt_cameras_into_capture_target(
    mut commands: Commands,
    settings: Res<CaptureSettings>,
    target: Option<ResMut<CaptureTarget>>,
    mut main_cameras: Query<(Entity, &mut Camera), (With<MainCamera>, Without<CaptureAdopted>)>,
    mut hud_cameras: Query<
        (Entity, &mut Camera),
        (
            With<FrontHudCamera>,
            Without<MainCamera>,
            Without<CaptureAdopted>,
        ),
    >,
) {
    let Some(mut target) = target else {
        return;
    };
    let render_target = bevy::camera::RenderTarget::Image(bevy::camera::ImageRenderTarget::from(
        target.image.clone(),
    ));
    for (entity, mut camera) in &mut main_cameras {
        camera.is_active = true;
        commands
            .entity(entity)
            .insert((render_target.clone(), Msaa::Off, CaptureAdopted));
        target.adopted += 1;
    }
    //  the HUD camera is pointed at the target only when it is WANTED. Leaving
    // it drawing into the same texture is how a "world only" capture grew a
    // health bar. It is still MARKED either way, so an unwanted HUD camera is
    // not revisited every frame forever.
    for (entity, mut camera) in &mut hud_cameras {
        camera.is_active = settings.include_ui;
        if settings.include_ui {
            commands
                .entity(entity)
                .insert((render_target.clone(), Msaa::Off));
        }
        commands.entity(entity).insert(CaptureAdopted);
    }
}

/// This camera has already been pointed at the capture target.
#[derive(Component)]
pub struct CaptureAdopted;

/// Ask for the readback that becomes the PNG. Idempotent: a second call while
/// one is in flight does nothing.
///
/// The caller decides WHEN — see this module's header for why that cannot live
/// here.
pub fn request_capture(
    commands: &mut Commands,
    target: &CaptureTarget,
    progress: &mut CaptureProgress,
) {
    if progress.requested || progress.completed {
        return;
    }
    commands
        .spawn(Readback::texture(target.image.clone()))
        .observe(save_readback_to_disk);
    progress.requested = true;
}

/// Copy the GPU readback into a PNG on disk.
///
///  the row padding is not optional. wgpu pads every row to a 256-byte boundary, so the
/// buffer is wider than the image for any width that is not a multiple of 64 pixels.
fn save_readback_to_disk(
    event: On<ReadbackComplete>,
    mut commands: Commands,
    settings: Res<CaptureSettings>,
    mut progress: ResMut<CaptureProgress>,
) {
    commands.entity(event.entity).despawn();
    let width = settings.size.x.max(1);
    let height = settings.size.y.max(1);
    let row_bytes = width as usize * 4;
    let padded_row_bytes = row_bytes.div_ceil(256) * 256;
    let expected = padded_row_bytes * height as usize;
    if event.data.len() < expected {
        eprintln!(
            "capture: readback returned {} bytes, expected at least {expected}",
            event.data.len()
        );
        progress.failed = true;
        progress.completed = true;
        commands.write_message(AppExit::from_code(1));
        return;
    }

    let mut pixels = vec![0u8; row_bytes * height as usize];
    for y in 0..height as usize {
        let src = y * padded_row_bytes;
        let dst = y * row_bytes;
        pixels[dst..dst + row_bytes].copy_from_slice(&event.data[src..src + row_bytes]);
    }

    let Some(image) = image::RgbaImage::from_raw(width, height, pixels) else {
        eprintln!("capture: failed to build PNG buffer");
        progress.failed = true;
        progress.completed = true;
        commands.write_message(AppExit::from_code(1));
        return;
    };
    if let Err(error) = image.save(&settings.output) {
        eprintln!(
            "capture: failed to save '{}': {error}",
            settings.output.display()
        );
        progress.failed = true;
        progress.completed = true;
        commands.write_message(AppExit::from_code(1));
        return;
    }
    progress.completed = true;
}

/// Exit successfully once the file is on disk.
///
///  a FAILED capture must not take this branch. It has already written its
/// own non-zero exit; announcing success afterwards would report a picture that
/// does not exist, which is worse than any crash.
pub fn finish_after_capture(
    mut commands: Commands,
    settings: Res<CaptureSettings>,
    progress: Res<CaptureProgress>,
) {
    if !progress.completed || progress.failed {
        return;
    }
    println!(
        "capture: wrote {} ({}x{} px)",
        settings.output.display(),
        settings.size.x,
        settings.size.y,
    );
    commands.write_message(AppExit::Success);
}
