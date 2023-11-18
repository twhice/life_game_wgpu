use std::time::Duration;

use glam::Vec3Swizzles;
use winit::event::{ElementState, MouseScrollDelta};
use winit::keyboard::KeyCode;

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

#[derive(Debug)]
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

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let state = state == ElementState::Pressed;
        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.amount_forward = state;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.amount_backward = state;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.amount_left = state;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.amount_right = state;
                true
            }
            _ => false,
        }
    }

    pub fn process_wheel(&mut self, delta: MouseScrollDelta, dt: Duration) {
        let delta = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(delta) => delta.y as f32,
        };
        self.scale -= dt.as_secs_f32() * 1000.0 * delta;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        let speed = camera.speed * dt * self.scale;

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

        camera.position.z = self.scale * camera.sensitivity;
        camera.position.z = camera.position.z.clamp(0.1, 10.0);

        // self.scale = 0.0;
    }
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            amount_left: Default::default(),
            amount_right: Default::default(),
            amount_forward: Default::default(),
            amount_backward: Default::default(),
            scale: 1.0,
        }
    }
}
