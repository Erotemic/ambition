//! Prepare repeating RGBA textures for filtered minification.

use bevy::{
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{TextureDimension, TextureFormat},
};

/// Build a complete mip chain. Average sRGB colors in linear space.
pub(crate) fn prepare_tiled_rgba(image: &mut Image) {
    let descriptor = &image.texture_descriptor;
    assert_eq!(descriptor.dimension, TextureDimension::D2);
    assert_eq!(descriptor.size.depth_or_array_layers, 1);
    assert!(matches!(
        descriptor.format,
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb
    ));
    assert_eq!(descriptor.mip_level_count, 1);
    let srgb = descriptor.format == TextureFormat::Rgba8UnormSrgb;
    let mut width = descriptor.size.width;
    let mut height = descriptor.size.height;
    let pixels = image
        .data
        .as_mut()
        .expect("texture preparation needs CPU pixels");
    assert_eq!(pixels.len(), (width * height * 4) as usize);
    let linear = std::array::from_fn::<_, 256, _>(|n| {
        let value = n as f32 / 255.0;
        if srgb {
            LinearRgba::from(Srgba::new(value, value, value, 1.0)).red
        } else {
            value
        }
    });
    let mut offset = 0;
    while width > 1 || height > 1 {
        let next_width = (width / 2).max(1);
        let next_height = (height / 2).max(1);
        let next_offset = pixels.len();
        for y in 0..next_height {
            for x in 0..next_width {
                let x0 = x as f32 * width as f32 / next_width as f32;
                let x1 = (x + 1) as f32 * width as f32 / next_width as f32;
                let y0 = y as f32 * height as f32 / next_height as f32;
                let y1 = (y + 1) as f32 * height as f32 / next_height as f32;
                let mut sum = Vec4::ZERO;
                let mut count = 0.0;
                for sy in y0.floor() as u32..y1.ceil() as u32 {
                    for sx in x0.floor() as u32..x1.ceil() as u32 {
                        let weight = ((sx + 1) as f32).min(x1) - (sx as f32).max(x0);
                        let weight = weight * (((sy + 1) as f32).min(y1) - (sy as f32).max(y0));
                        let i = offset + ((sy * width + sx) * 4) as usize;
                        sum += Vec4::new(
                            linear[pixels[i] as usize],
                            linear[pixels[i + 1] as usize],
                            linear[pixels[i + 2] as usize],
                            pixels[i + 3] as f32 / 255.0,
                        ) * weight;
                        count += weight;
                    }
                }
                let colour = sum / count;
                let encoded = if srgb {
                    let value =
                        Srgba::from(LinearRgba::new(colour.x, colour.y, colour.z, colour.w));
                    Vec4::new(value.red, value.green, value.blue, value.alpha)
                } else {
                    colour
                };
                pixels.extend(
                    encoded
                        .to_array()
                        .map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8),
                );
            }
        }
        offset = next_offset;
        width = next_width;
        height = next_height;
        image.texture_descriptor.mip_level_count += 1;
    }
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{asset::RenderAssetUsages, render::render_resource::Extent3d};

    #[test]
    fn minified_srgb_detail_keeps_its_linear_light_energy() {
        let mut image = Image::new(
            Extent3d {
                width: 2,
                height: 2,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![
                0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255,
            ],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        prepare_tiled_rgba(&mut image);
        assert_eq!(image.texture_descriptor.mip_level_count, 2);
        assert_eq!(&image.data.as_ref().unwrap()[16..], &[188, 188, 188, 255]);
    }

    #[test]
    fn a_narrow_texture_reaches_one_texel_without_losing_its_last_column() {
        let mut image = Image::new(
            Extent3d {
                width: 3,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        prepare_tiled_rgba(&mut image);
        assert_eq!(&image.data.as_ref().unwrap()[12..], &[85, 85, 85, 255]);
    }
}
