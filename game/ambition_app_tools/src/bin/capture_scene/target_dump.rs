//! GPU target readback for render-path diagnosis.

use super::*;

pub(super) fn prepare_readback(config: Res<SceneCaptureConfig>, mut images: ResMut<Assets<Image>>) {
    if config.dump_targets_at.is_none() { return; }
    use bevy::render::render_resource::TextureUsages;
    let fresh: Vec<_> = images.iter().filter_map(|(id,image)| {
        let usage = image.texture_descriptor.usage;
        (usage.contains(TextureUsages::RENDER_ATTACHMENT) && !usage.contains(TextureUsages::COPY_SRC)).then_some(id)
    }).collect();
    for id in fresh {
        images.get_mut(id).unwrap().texture_descriptor.usage |= TextureUsages::COPY_SRC;
    }
}

pub(super) fn dump_targets(
    config: Res<SceneCaptureConfig>,
    tick: Option<Res<ambition_platformer2d::time::SimTick>>,
    cameras: Query<(Entity, Option<&Name>, &bevy::camera::RenderTarget), With<Camera>>,
    images: Res<Assets<Image>>,
    mut done: Local<bool>,
    mut commands: Commands,
) {
    if *done || !config.dump_targets_at.is_some_and(|at| tick.as_deref().is_some_and(|tick| tick.get() == at)) { return; }
    *done = true;
    let folder = config.output.parent().unwrap_or_else(|| std::path::Path::new(".")).join("targets");
    if let Err(error) = std::fs::create_dir_all(&folder) {
        eprintln!("capture_scene: target dump directory: {error}");
        commands.write_message(AppExit::from_code(1));
        return;
    }
    let mut cameras: Vec<_> = cameras.iter().collect();
    cameras.sort_by_key(|(entity,_,_)| *entity);
    let mut seen = Vec::new();
    let mut manifest = Vec::new();
    for (camera, name, target) in cameras {
        let bevy::camera::RenderTarget::Image(target) = target else { continue; };
        if seen.contains(&target.handle.id()) { continue; }
        let Some(image) = images.get(&target.handle) else { continue; };
        use bevy::render::render_resource::TextureFormat;
        if !matches!(image.texture_descriptor.format, TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb) { continue; }
        let (width,height) = (image.texture_descriptor.size.width,image.texture_descriptor.size.height);
        let path = folder.join(format!("target-{}.png",seen.len()));
        manifest.push(serde_json::json!({"camera":format!("{camera:?}"),"name":name.map(Name::as_str),
            "target":format!("{:?}",target.handle),"size":[width,height],"file":path.file_name().unwrap().to_string_lossy()}));
        seen.push(target.handle.id());
        commands.spawn(Readback::texture(target.handle.clone())).observe(move |event: On<ReadbackComplete>, mut commands: Commands| {
            commands.entity(event.entity).despawn();
            let row = width as usize * 4;
            let stride = row.div_ceil(256)*256;
            let result = if event.data.len() < stride * height as usize {
                Err("short target readback".to_string())
            } else {
                let pixels: Vec<_> = event.data.chunks(stride).take(height as usize)
                    .flat_map(|line| line[..row].iter().copied()).collect();
                image::save_buffer(&path,&pixels,width,height,image::ColorType::Rgba8).map_err(|error|error.to_string())
            };
            if let Err(error) = result {
                eprintln!("capture_scene: target dump {}: {error}",path.display());
                commands.write_message(AppExit::from_code(1));
            }
        });
    }
    if let Err(error) = std::fs::write(folder.join("manifest.json"),serde_json::to_vec_pretty(&manifest).unwrap()) {
        eprintln!("capture_scene: target dump manifest: {error}");
        commands.write_message(AppExit::from_code(1));
    }
}
