use cgmath::prelude::*;
use std::io::{BufReader, Cursor};
use wgpu::util::DeviceExt;

use super::{model, texture, util::*};

/////////////////////////////////////////

pub fn load_string_sync(file_name: &str) -> anyhow::Result<String> {
    pollster::block_on(load_string(file_name))
}

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    let path = std::path::Path::new(env!("OUT_DIR"))
        .join("res")
        .join(file_name);
    let txt = std::fs::read_to_string(path)?;
    Ok(txt)
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    let path = std::path::Path::new(env!("OUT_DIR"))
        .join("res")
        .join(file_name);
    let data = std::fs::read(path)?;
    Ok(data)
}

pub fn load_texture_sync(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    is_normal_map: bool,
    generate_mipmaps: bool,
) -> anyhow::Result<texture::Texture> {
    pollster::block_on(load_texture(
        file_name,
        device,
        queue,
        is_normal_map,
        generate_mipmaps,
    ))
}

pub async fn load_texture(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    is_normal_map: bool,
    generate_mipmaps: bool,
) -> anyhow::Result<texture::Texture> {
    let data = load_binary(file_name).await?;
    texture::Texture::from_bytes(
        device,
        queue,
        &data,
        file_name,
        is_normal_map,
        generate_mipmaps,
    )
}

pub fn load_model_sync(
    file_name: &str,
    material_name: Option<&str>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    instances: &[model::Instance],
    generate_mipmaps: bool,
) -> anyhow::Result<model::Model> {
    pollster::block_on(load_model(
        file_name,
        material_name,
        device,
        queue,
        instances,
        generate_mipmaps,
    ))
}

pub async fn load_model(
    file_name: &str,
    material_name: Option<&str>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    instances: &[model::Instance],
    generate_mipmaps: bool,
) -> anyhow::Result<model::Model> {
    let obj_text = load_string(file_name).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, obj_materials) = tobj::load_obj_buf_async(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let material_name = material_name.unwrap_or(&p);
            let mat_text = load_string(material_name).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    )
    .await?;

    let mut materials = Vec::new();
    for m in obj_materials? {
        let ambient = Vec4::new(m.ambient[0], m.ambient[1], m.ambient[2], 1.0);
        let diffuse = Vec4::new(m.diffuse[0], m.diffuse[1], m.diffuse[2], 1.0);
        let specular = Vec4::new(m.specular[0], m.specular[1], m.specular[2], 1.0);

        let diffuse_texture =
            load_texture(&m.diffuse_texture, device, queue, false, generate_mipmaps)
                .await
                .ok();
        let normal_texture = load_texture(&m.normal_texture, device, queue, true, generate_mipmaps)
            .await
            .ok();
        let shininess_texture =
            load_texture(&m.shininess_texture, device, queue, false, generate_mipmaps)
                .await
                .ok();

        materials.push(model::Material::new(
            device,
            model::MaterialProperties {
                name: &m.name,
                ambient,
                diffuse,
                specular,
                shininess: m.shininess,
                diffuse_texture,
                normal_texture,
                shininess_texture,
            },
        ));
    }

    let meshes = models
        .into_iter()
        .map(|m| {
            let mut vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| model::ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                    tangent: [0.0; 3],
                    bitangent: [0.0; 3],
                })
                .collect::<Vec<_>>();

            let indices = &m.mesh.indices;
            let mut triangles_included = (0..vertices.len()).collect::<Vec<_>>();

            // compute tangent and bitangent
            for c in indices.chunks(3) {
                let v0 = vertices[c[0] as usize];
                let v1 = vertices[c[1] as usize];
                let v2 = vertices[c[2] as usize];

                let pos0: Vec3 = v0.position.into();
                let pos1: Vec3 = v1.position.into();
                let pos2: Vec3 = v2.position.into();

                let uv0: Vec2 = v0.tex_coords.into();
                let uv1: Vec2 = v1.tex_coords.into();
                let uv2: Vec2 = v2.tex_coords.into();

                let delta_pos1 = pos1 - pos0;
                let delta_pos2 = pos2 - pos0;
                let delta_uv1 = uv1 - uv0;
                let delta_uv2 = uv2 - uv0;

                let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
                let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
                let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * -r;

                vertices[c[0] as usize].tangent =
                    (tangent + Vec3::from(vertices[c[0] as usize].tangent)).into();
                vertices[c[1] as usize].tangent =
                    (tangent + Vec3::from(vertices[c[1] as usize].tangent)).into();
                vertices[c[2] as usize].tangent =
                    (tangent + Vec3::from(vertices[c[2] as usize].tangent)).into();
                vertices[c[0] as usize].bitangent =
                    (bitangent + Vec3::from(vertices[c[0] as usize].bitangent)).into();
                vertices[c[1] as usize].bitangent =
                    (bitangent + Vec3::from(vertices[c[1] as usize].bitangent)).into();
                vertices[c[2] as usize].bitangent =
                    (bitangent + Vec3::from(vertices[c[2] as usize].bitangent)).into();

                // Used to average the tangents/bitangents
                triangles_included[c[0] as usize] += 1;
                triangles_included[c[1] as usize] += 1;
                triangles_included[c[2] as usize] += 1;
            }

            for (i, n) in triangles_included.into_iter().enumerate() {
                let denom = 1.0 / n as f32;
                let mut v = &mut vertices[i];
                v.tangent = (Vec3::from(v.tangent) * denom).normalize().into();
                v.bitangent = (Vec3::from(v.bitangent) * denom).normalize().into();
            }

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            model::Mesh {
                name: file_name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            }
        })
        .collect::<Vec<_>>();

    Ok(model::Model::new(device, meshes, materials, instances))
}
