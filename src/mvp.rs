use std::time::Duration;

use glam::Vec3Swizzles;
use winit::event::{ElementState, MouseScrollDelta, VirtualKeyCode};

pub struct Projection {
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Projection {
    pub fn new(width: u32, height: u32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.to_radians(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> glam::Mat4 {
        glam::Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

pub struct Camera {
    pub position: glam::Vec3,

    speed: f32,       // 速度
    sensitivity: f32, // 灵敏度
}

impl Camera {
    pub fn new(position: impl Into<glam::Vec3>, speed: f32, sensitivity: f32) -> Self {
        Self {
            position: position.into(),
            speed,
            sensitivity,
        }
    }

    pub fn calc_matrix(&self) -> glam::Mat4 {
        const FRONT: glam::Vec3 = glam::vec3(0.0, 0.0, -1.0);

        glam::Mat4::look_at_rh(self.position, FRONT + self.position, glam::Vec3::Y)
    }
}

#[derive(Debug, Default)]
pub struct CameraController {
    amount_left: bool,     // a
    amount_right: bool,    // d
    amount_forward: bool,  // w
    amount_backward: bool, // s
    scale: f32,
}

impl CameraController {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let state = state == ElementState::Pressed;
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.amount_forward = state;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.amount_backward = state;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.amount_left = state;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.amount_right = state;
                true
            }
            _ => false,
        }
    }

    pub fn process_wheel(&mut self, delta: MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(_, y) => self.scale -= y,
            MouseScrollDelta::PixelDelta(delta) => self.scale -= delta.y as f32,
        }
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        let speed = camera.speed * dt;

        const FRONTS: glam::Vec3 = glam::vec3(-1.0, 0.0, 1.0);

        if self.amount_forward {
            camera.position += FRONTS.yzy() * speed;
        }
        if self.amount_backward {
            camera.position += FRONTS.yxy() * speed;
        }
        if self.amount_left {
            camera.position += FRONTS.xyy() * speed;
        }
        if self.amount_right {
            camera.position += FRONTS.zyy() * speed;
        }

        camera.position.z += self.scale * camera.sensitivity * dt;
        camera.position.z = camera.position.z.clamp(0.1, 10.0);

        self.scale = 0.0;
    }
}
