#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    offset: [f32; 2],
    zoom: f32,
    aspect_ratio: f32,
}

impl CameraUniform {
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            zoom: 1.0,
            offset: [0., 0.],
            aspect_ratio,
        }
    }
}

#[derive(Debug)]
pub struct CameraController {
    is_up_pressed: bool,
    is_down_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    zoom_in: bool,
    zoom_out: bool,
    aspect_ratio: Option<f32>,
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            is_up_pressed: false,
            is_down_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            zoom_in: false,
            zoom_out: false,
            aspect_ratio: None,
        }
    }
    pub fn process_events(&mut self, event: &winit::event::WindowEvent) -> bool {
        use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::W | VirtualKeyCode::Up => {
                        self.is_up_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::A | VirtualKeyCode::Left => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::S | VirtualKeyCode::Down => {
                        self.is_down_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::D | VirtualKeyCode::Right => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::Minus | VirtualKeyCode::NumpadSubtract => {
                        self.zoom_out = is_pressed;
                        true
                    }
                    VirtualKeyCode::Equals | VirtualKeyCode::NumpadAdd => {
                        self.zoom_in = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = Some(aspect_ratio);
    }

    pub fn update_camera(&mut self, dt: std::time::Duration, camera: &mut CameraUniform) -> bool {
        let dt = dt.as_secs_f32();
        let speed = 1.0;
        let amount = dt * speed * camera.zoom;
        let zoom_amount = 1.7_f32.powf(dt);
        let mut updated = false;
        if let Some(r) = self.aspect_ratio {
            camera.aspect_ratio = r;
            self.aspect_ratio = None;
            updated = true;
        }
        if self.is_up_pressed {
            camera.offset[1] -= amount;
            updated = true;
        }
        if self.is_down_pressed {
            camera.offset[1] += amount;
            updated = true;
        }
        if self.is_left_pressed {
            camera.offset[0] += amount;
            updated = true;
        }
        if self.is_right_pressed {
            camera.offset[0] -= amount;
            updated = true;
        }
        if self.zoom_out {
            camera.zoom *= zoom_amount;
            updated = true
        }
        if self.zoom_in {
            camera.zoom /= zoom_amount;
            updated = true
        }
        updated
    }
}
