use anyhow::*;
use image::GenericImageView;
use wgpu::util::DeviceExt;

// CLosest power of two to `v` without exceeding `v`
// E.g., 511 -> 256; 512 -> 512; 513 -> 512
fn pot(v: u32) -> u32 {
    let l = (v as f32).log(2.0).floor() as u32;
    2u32.pow(l)
}

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub view_dimension: wgpu::TextureViewDimension,
}

impl Texture {
    pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
        is_normal_map: bool,
        generate_mipmaps: bool,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;

        let dimensions = img.dimensions();
        let pot_dimensions = (pot(dimensions.0), pot(dimensions.1));

        let img = if generate_mipmaps && dimensions != pot_dimensions {
            img.resize(
                pot_dimensions.0,
                pot_dimensions.1,
                image::imageops::FilterType::CatmullRom,
            )
        } else {
            img
        };

        Self::from_image(
            device,
            queue,
            img,
            Some(label),
            is_normal_map,
            generate_mipmaps,
        )
    }

    fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: image::DynamicImage,
        label: Option<&str>,
        is_normal_map: bool,
        generate_mipmaps: bool,
    ) -> Result<Self> {
        let dimensions = img.dimensions();
        let mip_levels = if generate_mipmaps {
            (((dimensions.0.min(dimensions.1)) as f32).log(2.0).floor() as u32).max(1u32)
        } else {
            1
        };

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: mip_levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if is_normal_map {
                wgpu::TextureFormat::Rgba8Unorm
            } else {
                wgpu::TextureFormat::Rgba8UnormSrgb
            },
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });

        let mut img = img;
        for mip_level in 0..mip_levels {
            if mip_level > 0 {
                img = img.resize_exact(
                    img.dimensions().0 / 2,
                    img.dimensions().1 / 2,
                    image::imageops::FilterType::Triangle,
                );
            }

            let mip_size = img.dimensions();
            let data = img.to_rgba8();

            queue.write_texture(
                wgpu::ImageCopyTexture {
                    aspect: wgpu::TextureAspect::All,
                    texture: &texture,
                    mip_level,
                    origin: wgpu::Origin3d::ZERO,
                },
                &data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: std::num::NonZeroU32::new(4 * mip_size.0),
                    rows_per_image: std::num::NonZeroU32::new(mip_size.1),
                },
                wgpu::Extent3d {
                    width: mip_size.0,
                    height: mip_size.1,
                    depth_or_array_layers: 1,
                },
            );
        }

        let filter_mode = if generate_mipmaps {
            wgpu::FilterMode::Linear
        } else {
            wgpu::FilterMode::Nearest
        };

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: filter_mode,
            min_filter: filter_mode,
            mipmap_filter: filter_mode,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
            view_dimension: wgpu::TextureViewDimension::D2,
        })
    }

    pub fn cubemap_from_dds(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self> {
        let image = ddsfile::Dds::read(&mut std::io::Cursor::new(&bytes))?;
        let size = wgpu::Extent3d {
            width: image.get_width(),
            height: image.get_height(),
            depth_or_array_layers: 6,
        };

        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                size,
                mip_level_count: image.get_num_mipmap_levels(),
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: Some(label),
            },
            &image.data,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(label),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..wgpu::TextureViewDescriptor::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
            view_dimension: wgpu::TextureViewDimension::Cube,
        })
    }

    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            view_dimension: wgpu::TextureViewDimension::D2,
        }
    }

    pub fn create_color_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(Self::COLOR_FORMAT),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            view_dimension: wgpu::TextureViewDimension::D2,
        }
    }
}
