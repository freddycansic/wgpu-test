use crate::BufferContents;
use cg::prelude::*;
use cgmath as cg;

#[derive(Clone)]
pub struct ModelInstance {
    pub position: cg::Vector3<f32>,
    pub rotation: cg::Quaternion<f32>,
}

impl Default for ModelInstance {
    fn default() -> Self {
        Self {
            position: cg::Vector3::zero(),
            rotation: cg::Quaternion::zero(),
        }
    }
}

#[derive(Default)]
pub struct Instances {
    pub model_instances: Vec<ModelInstance>,
    pub instance_buffer: Option<wgpu::Buffer>,
}

#[repr(C)]
#[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw([[f32; 4]; 4]);

impl InstanceRaw {
    const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4];
}

impl BufferContents for InstanceRaw {
    fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::INSTANCE_ATTRIBUTES,
        }
    }
}

impl From<&ModelInstance> for InstanceRaw {
    fn from(instance: &ModelInstance) -> Self {
        InstanceRaw(
            (cg::Matrix4::from_translation(instance.position)
                * cg::Matrix4::from(instance.rotation))
            .into(),
        )
    }
}
