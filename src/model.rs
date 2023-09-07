use std::{io::{BufReader, Cursor}, ops::Range};

use anyhow::Result;
use wgpu::util::DeviceExt;

use crate::{resources, texture::Texture};

pub trait BufferContents {
    fn buffer_layout() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub texture_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl ModelVertex {
    const MODEL_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3];
}

impl BufferContents for ModelVertex {
    fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::MODEL_VERTEX_ATTRIBUTES,
        }
    }
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub indices_count: u32,
    pub material: usize,
}

impl Model {
    pub fn load(
        path: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> Result<Self> {
        let (models, model_materials) = tobj::load_obj(
            path,
            &tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ..Default::default()
            },
        )?;

        let materials = model_materials?
            .into_iter()
            .map(|material| {
                let diffuse_texture_path = material.diffuse_texture.ok_or_else(|| anyhow::anyhow!("No diffuse texture found"))?;

                let diffuse_texture = Texture::from_path(
                    &diffuse_texture_path,
                    device,
                    queue,
                    Some(diffuse_texture_path.as_str()),
                )?;

                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                        },
                    ],
                    label: None,
                });

                Ok(Material {
                    name: material.name,
                    diffuse_texture,
                    bind_group,
                })
            })
            .collect::<Result<Vec<Material>>>()?;

            let meshes = models
            .into_iter()
            .map(|model| {
                let vertices = (0..model.mesh.positions.len() / 3)
                    .map(|i| ModelVertex {
                        position: [
                            model.mesh.positions[i * 3],
                            model.mesh.positions[i * 3 + 1],
                            model.mesh.positions[i * 3 + 2],
                        ],
                        texture_coords: [model.mesh.texcoords[i * 2], model.mesh.texcoords[i * 2 + 1]],
                        normal: [
                            model.mesh.normals[i * 3],
                            model.mesh.normals[i * 3 + 1],
                            model.mesh.normals[i * 3 + 2],
                        ],
                    })
                    .collect::<Vec<_>>();
    
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer", path)),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Index Buffer", path)),
                    contents: bytemuck::cast_slice(&model.mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
    
                Mesh {
                    name: path.to_string(),
                    vertex_buffer,
                    index_buffer,
                    indices_count: model.mesh.indices.len() as u32,
                    material: model.mesh.material_id.unwrap_or(0),
                }
            })
            .collect::<Vec<Mesh>>();

        Ok(Self { materials, meshes })
    }
}

pub trait DrawModel<'a> {
    fn draw_mesh(&mut self, mesh: &'a Mesh);
    fn draw_mesh_instanced(&mut self, mesh: &'a Mesh, instances: Range<u32>);
}

impl <'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
// b lives at least as long as a
where 'b: 'a 
{
    fn draw_mesh(&mut self, mesh: &'b Mesh) {
        self.draw_mesh_instanced(mesh, 0..1)
    }

    fn draw_mesh_instanced(&mut self, mesh: &'b Mesh, instances: Range<u32>) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(0..mesh.indices_count, 0, instances);
    }
}