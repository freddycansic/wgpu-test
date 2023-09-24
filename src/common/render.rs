use wgpu::util::DeviceExt;

use crate::common::context::WgpuContext;
use crate::common::instance::{ModelInstance, RawInstance};
use crate::common::model::InstancedModel;

pub struct RenderData {
    pub models: Vec<InstancedModel>,
    // instance buffer for models which will only be drawn once e.g the map
    single_instance_buffer: wgpu::Buffer,
}

impl RenderData {
    pub fn new(device: &wgpu::Device) -> Self {
        let instance = ModelInstance::default();
        let instance_raw = RawInstance::from(&instance);
        let single_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("single_instance_buffer"),
            usage: wgpu::BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(&[instance_raw]),
        });

        Self {
            single_instance_buffer,
            models: vec![],
        }
    }

    pub fn single_instance_slice(&self) -> wgpu::BufferSlice {
        self.single_instance_buffer.slice(..)
    }

    pub fn update_instance_buffers(&mut self, wgpu_context: &WgpuContext) {
        for model in self.models.iter_mut() {
            if model.instances.is_none() {
                continue;
            }

            let model_instance_data = model
                .instances
                .as_ref()
                .unwrap()
                .iter()
                .map(RawInstance::from)
                .collect::<Vec<RawInstance>>();

            match &model.instance_buffer {
                Some(instance_buffer) => {
                    let current_instance_buffer_len =
                        instance_buffer.size() / std::mem::size_of::<RawInstance>() as u64;

                    let next_instance_buffer_len = model.instances.as_ref().unwrap().len() as u64;

                    if next_instance_buffer_len > current_instance_buffer_len {
                        model.instance_buffer = Some(wgpu_context.device.create_buffer_init(
                            &wgpu::util::BufferInitDescriptor {
                                label: Some(
                                    format!("{:?}-instance-buffer", model.model.name).as_str(),
                                ),
                                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                                contents: bytemuck::cast_slice(&model_instance_data),
                            },
                        ));

                        log::info!(
                            "Resized instance buffer for model \"{}\" to {} elements from {} elements",
                            model.model.name,
                            current_instance_buffer_len,
                            next_instance_buffer_len
                        )
                    } else {
                        wgpu_context.queue.write_buffer(
                            model.instance_buffer.as_ref().unwrap(),
                            0,
                            bytemuck::cast_slice(&model_instance_data),
                        );
                    }
                }
                None => {
                    model.instance_buffer = Some(wgpu_context.device.create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some(format!("{:?}-instance-buffer", model.model.name).as_str()),
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                            contents: bytemuck::cast_slice(&model_instance_data),
                        },
                    ));

                    log::info!("Created instance buffer for model \"{}\"", model.model.name)
                }
            }
        }
    }
}
