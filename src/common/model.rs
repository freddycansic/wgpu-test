use std::hash::{Hash, Hasher};
use std::rc::Rc;

use color_eyre::Result;
use wgpu::util::DeviceExt;

use crate::common::context::WgpuContext;
use crate::common::instance::ModelInstance;
use crate::common::render::RenderData;
use crate::common::texture::{Texture, TextureAtlas};

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
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::MODEL_VERTEX_ATTRIBUTES,
        }
    }
}

// models are only going to be loaded once ever anyway so no Rc for model
pub struct InstancedModel {
    pub model: Model,
    pub instances: Option<Vec<ModelInstance>>,
    pub instance_buffer: Option<wgpu::Buffer>,
}

impl From<Model> for InstancedModel {
    fn from(model: Model) -> Self {
        Self {
            model,
            instances: None,
            instance_buffer: None,
        }
    }
}

pub struct Model {
    pub name: Rc<String>,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub struct Material {
    pub name: Rc<String>,
    // the same texture can be used in multiple materials to avoid loading the same image multiple times hence Rc
    pub diffuse: Rc<Texture>,
    pub normal: Option<Rc<Texture>>,
    pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: Rc<String>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub indices_count: u32,
    pub material_index: usize,
}

impl Model {
    pub fn load(
        path: String,
        texture_atlas: &mut TextureAtlas,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        wgpu_context: &WgpuContext,
    ) -> Result<Self> {
        let (meshes, model_materials) = tobj::load_obj(
            path.clone(),
            &tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ..Default::default()
            },
        )?;

        log::info!("Loaded model \"{}\"", path);

        let model_materials = model_materials?;

        let materials = if model_materials.is_empty() {
            let default_texture_path: Rc<String> = Rc::new("default.png".to_string());
            let default_texture = texture_atlas.get(
                default_texture_path,
                &wgpu_context.device,
                &wgpu_context.queue,
            )?;

            let bind_group = wgpu_context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &texture_bind_group_layout,
                    label: Some("default-texture-bind-group"),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&default_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&default_texture.sampler),
                        },
                    ],
                });

            vec![Material {
                diffuse: default_texture,
                normal: None,
                bind_group,
                name: Rc::new("default-material".to_string()),
            }]
        } else {
            model_materials
                .into_iter()
                .map(|material| {
                    let diffuse_texture_path = match material.diffuse_texture {
                        Some(texture_path) => texture_path,
                        None => {
                            log::warn!(
                                "No diffuse texture found for model \"{}\", using default texture.",
                                path
                            );
                            "default.png".to_string()
                        }
                    };

                    let diffuse_texture = texture_atlas.get(
                        Rc::new(diffuse_texture_path),
                        &wgpu_context.device,
                        &wgpu_context.queue,
                    )?;

                    let bind_group =
                        wgpu_context
                            .device
                            .create_bind_group(&wgpu::BindGroupDescriptor {
                                layout: &texture_bind_group_layout,
                                label: Some("default-texture-bind-group"),
                                entries: &[
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: wgpu::BindingResource::TextureView(
                                            &diffuse_texture.view,
                                        ),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 1,
                                        resource: wgpu::BindingResource::Sampler(
                                            &diffuse_texture.sampler,
                                        ),
                                    },
                                ],
                            });

                    Ok(Material {
                        name: Rc::new(material.name),
                        diffuse: diffuse_texture,
                        normal: None,
                        bind_group,
                    })
                })
                .collect::<Result<Vec<Material>>>()?
        };

        let meshes = meshes
            .into_iter()
            .map(|mesh| {
                let positions_chunks = mesh.mesh.positions.chunks(3);
                let texcoords_chunks = mesh.mesh.texcoords.chunks(2);
                let normals_chunks = mesh.mesh.normals.chunks(3);

                let vertices = positions_chunks
                    .zip(texcoords_chunks)
                    .zip(normals_chunks)
                    .map(|((pos, tex), norm)| ModelVertex {
                        position: [pos[0], pos[1], pos[2]],
                        texture_coords: [tex[0], tex[1]],
                        normal: [norm[0], norm[1], norm[2]],
                    })
                    .collect::<Vec<ModelVertex>>();

                let vertex_buffer =
                    wgpu_context
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&format!("{:?}-vertex-buffer", mesh.name)),
                            contents: bytemuck::cast_slice(&vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                let index_buffer =
                    wgpu_context
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&format!("{:?}-index-buffer", mesh.name)),
                            contents: bytemuck::cast_slice(&mesh.mesh.indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });

                Mesh {
                    name: Rc::new(mesh.name),
                    vertex_buffer,
                    index_buffer,
                    indices_count: mesh.mesh.indices.len() as u32,
                    material_index: mesh.mesh.material_id.unwrap_or(0),
                }
            })
            .collect::<Vec<Mesh>>();

        Ok(Self {
            name: Rc::new(path.to_string()),
            materials,
            meshes,
        })
    }
}

impl Hash for Model {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Model {}

pub trait DrawModels<'a> {
    fn draw_models(&mut self, render_data: &'a RenderData);
}

impl<'a, 'b> DrawModels<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    // compile all stuff and draw
    fn draw_models(&mut self, render_data: &'b RenderData) {
        for instanced_model in render_data.models.iter() {
            // todo, if running low on gpu resources i could make it so that the instanced models are sorted by Option<Instance> so that the single instance buffer is only bound once for all of the single render models. for now this is good
            match &instanced_model.instance_buffer {
                Some(instance_buffer) => self.set_vertex_buffer(1, instance_buffer.slice(..)),
                None => self.set_vertex_buffer(1, render_data.single_instance_slice()),
            }

            for mesh in instanced_model.model.meshes.iter() {
                self.set_bind_group(
                    0,
                    &instanced_model.model.materials[mesh.material_index].bind_group,
                    &[],
                );
                self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                match &instanced_model.instances {
                    Some(instances) => {
                        self.draw_indexed(0..mesh.indices_count, 0, 0..instances.len() as u32)
                    }
                    None => self.draw_indexed(0..mesh.indices_count, 0, 0..1),
                }
            }
        }
    }
}
