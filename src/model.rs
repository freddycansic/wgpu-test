use std::hash::{Hash, Hasher};
use std::sync::Arc;

use color_eyre::Result;
use rustc_hash::FxHashMap;
use wgpu::util::DeviceExt;

use crate::instance::{InstanceRaw, Instances};
use crate::texture::Texture;

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

pub struct Model {
    pub name: Arc<String>,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub struct Material {
    pub name: Arc<String>,
    pub diffuse_texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: Arc<String>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub indices_count: u32,
    pub material_index: usize,
}

impl Model {
    pub fn load(
        path: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> Result<Self> {
        let (meshes, model_materials) = tobj::load_obj(
            path,
            &tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ..Default::default()
            },
        )?;

        log::info!("Loaded model \"{}\"", path);

        let materials = model_materials?
            .into_iter()
            .map(|material| {
                let diffuse_texture_path = match material.diffuse_texture {
                    Some(texture_path) => texture_path,
                    None => {
                        log::warn!(
                            "No diffuse texture found for model \"{}\", loading default texture.",
                            path
                        );
                        "default.png".to_string()
                    }
                };

                let diffuse_texture = Texture::from_path(
                    &diffuse_texture_path,
                    device,
                    queue,
                    Some(diffuse_texture_path.as_str()),
                )?;

                log::info!("Loaded diffuse texture \"{}\"", diffuse_texture_path);

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
                    label: Some(format!("{:?}-diffuse-bind-group", path).as_str()),
                });

                Ok(Material {
                    name: Arc::new(material.name),
                    diffuse_texture,
                    bind_group,
                })
            })
            .collect::<Result<Vec<Material>>>()?;

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

                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?}-vertex-buffer", mesh.name)),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?}-index-buffer", mesh.name)),
                    contents: bytemuck::cast_slice(&mesh.mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                Mesh {
                    name: Arc::new(mesh.name),
                    vertex_buffer,
                    index_buffer,
                    indices_count: mesh.mesh.indices.len() as u32,
                    material_index: mesh.mesh.material_id.unwrap_or(0),
                }
            })
            .collect::<Vec<Mesh>>();

        Ok(Self {
            name: Arc::new(path.to_string()),
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
    fn draw_models(
        &mut self,
        models: &'a mut FxHashMap<Arc<Model>, Instances>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
}

impl<'a, 'b> DrawModels<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    // compile all stuff and draw
    fn draw_models(
        &mut self,
        models: &'b mut FxHashMap<Arc<Model>, Instances>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        for (model, instances) in models.iter_mut() {
            let model_instance_data = instances
                .model_instances
                .iter()
                .map(InstanceRaw::from)
                .collect::<Vec<InstanceRaw>>();

            match &instances.instance_buffer {
                Some(instance_buffer) => {
                    let current_instance_buffer_len =
                        instance_buffer.size() / std::mem::size_of::<InstanceRaw>() as u64;

                    let next_instance_buffer_len = instances.model_instances.len() as u64;

                    if next_instance_buffer_len > current_instance_buffer_len {
                        instances.instance_buffer = Some(device.create_buffer_init(
                            &wgpu::util::BufferInitDescriptor {
                                label: Some(format!("{:?}-instance-buffer", model.name).as_str()),
                                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                                contents: bytemuck::cast_slice(&model_instance_data),
                            },
                        ));

                        log::info!("Resized instance buffer for model \"{}\" to {} elements from {} elements", model.name, current_instance_buffer_len, next_instance_buffer_len)
                    } else {
                        queue.write_buffer(
                            instances.instance_buffer.as_ref().unwrap(),
                            0,
                            bytemuck::cast_slice(&model_instance_data),
                        );
                    }
                }
                None => {
                    instances.instance_buffer = Some(device.create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some(format!("{:?}-instance-buffer", model.name).as_str()),
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                            contents: bytemuck::cast_slice(&model_instance_data),
                        },
                    ));

                    log::info!("Created instance buffer for model \"{}\"", model.name)
                }
            }

            // if let None = instances.instance_buffer {
            //     instances.instance_buffer = Some(device.create_buffer_init(
            //         &wgpu::util::BufferInitDescriptor {
            //             label: Some(format!("{:?}-instance-buffer", model.name).as_str()),
            //             usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            //             contents: bytemuck::cast_slice(&model_instance_data),
            //         },
            //     ));

            // } else {
            //
            // }

            self.set_vertex_buffer(1, instances.instance_buffer.as_ref().unwrap().slice(..));

            for mesh in model.meshes.iter() {
                self.set_bind_group(0, &model.materials[mesh.material_index].bind_group, &[]);
                self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                self.draw_indexed(
                    0..mesh.indices_count,
                    0,
                    0..instances.model_instances.len() as u32,
                )
            }

            // instances.model_instances.clear()
        }
    }
}
