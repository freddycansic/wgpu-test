use cgmath as cg;
use cg::prelude::*;
use winit::event::VirtualKeyCode;

use crate::input::{input, ContinuousKeyPresses};

// opengl NDC has z dimension from -1 to 1, wgpu has it from 0 to 1
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

pub struct Camera {
    pub position: cg::Point3<f32>,
    pub direction: cg::Vector3<f32>,
    pub yaw: f32,
    pub pitch: f32,
    pub up: cg::Vector3<f32>,
    pub aspect_ratio: f32,
    pub fov: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> cg::Matrix4<f32> {
        let view = cg::Matrix4::look_at_rh(self.position, self.position + self.direction, self.up);
        let projection = cg::perspective(
            cg::Deg(self.fov),
            self.aspect_ratio,
            self.z_near,
            self.z_far,
        );

        OPENGL_TO_WGPU_MATRIX * projection * view
    }

    pub fn update_position(&mut self, delta: instant::Duration) {
        let forward = self.direction.normalize();
        let right = forward.cross(self.up).normalize();
        let speed = 10.0;

        if input().read().unwrap().key_down(VirtualKeyCode::W) {
            self.position += forward * speed * delta.as_secs_f32();
        }

        if input().read().unwrap().key_down(VirtualKeyCode::S) {
            self.position -= forward * speed * delta.as_secs_f32();
        }

        if input().read().unwrap().key_down(VirtualKeyCode::A) {
            self.position -= right * speed * delta.as_secs_f32();
        }

        if input().read().unwrap().key_down(VirtualKeyCode::D) {
            self.position += right * speed * delta.as_secs_f32();
        }
    }

    pub fn update_direction(&mut self, delta: instant::Duration) {
        let mouse_diff = input().read().unwrap().mouse_diff();

        if mouse_diff == (0.0, 0.0) {
            return;
        }

        let sensitivity = 1.5;

        self.yaw += mouse_diff.0 * sensitivity * delta.as_secs_f32();
        self.pitch += mouse_diff.1 * sensitivity * delta.as_secs_f32();

        let offset = 0.01;
        let pi = std::f32::consts::PI;
        self.pitch = self.pitch.clamp(-pi / 2.0 + offset, pi / 2.0 - offset);

        self.direction = cg::Vector3 {
            x: self.yaw.cos() * self.pitch.cos(),
            y: -self.pitch.sin(),
            z: self.yaw.sin() * self.pitch.cos(),
        }
        .normalize();
    }
}