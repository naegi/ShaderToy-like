use super::camera::*;
use std::borrow::Cow;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-1., 1.],
    },
    Vertex {
        position: [-1., -1.],
    },
    Vertex {
        position: [1., -1.],
    },
    Vertex {
        position: [1., -1.],
    },
    Vertex { position: [1., 1.] },
    Vertex {
        position: [-1., 1.],
    },
];

impl Vertex {
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

struct AppInner {
    event_loop: Option<winit::event_loop::EventLoop<()>>,
    window: winit::window::Window,
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    swapchain_format: wgpu::TextureFormat,
    render_pipeline_layout: wgpu::PipelineLayout,
    multisampled_framebuffer: wgpu::TextureView,
    sample_count: u32,
}

pub struct App {
    inner: AppInner,
    shader: wgpu::ShaderModule,
    vertex_buffer: wgpu::Buffer,
    camera: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,
}

impl App {
    pub fn new_block() -> Self {
        pollster::block_on(Self::new())
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn new() -> Self {
        Self::new_common().await
    }
    #[cfg(target_arch = "wasm32")]
    pub async fn new() -> Self {
        let mut this = Self::new_common().await;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(this.window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
        this
    }

    async fn new_common() -> Self {
        let sample_count = 4;
        let event_loop = winit::event_loop::EventLoop::new();
        let window = winit::window::Window::new(&event_loop).unwrap();

        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::Backends::all());

        #[cfg(target_os = "macos")]
        compile_error!("This should be in main thread.");
        // SAFETY: (cf. https://docs.rs/wgpu/latest/wgpu/struct.Instance.html#method.create_surface)
        // We are not on MacOS, thus, it's OK + the window is valid
        let surface = unsafe { instance.create_surface(&window) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    // Make sure we use the texture resolution limits from the adapte,
                    // so we can support images the size of the swapchain.
                    limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .expect("Failed to create device");
        device.on_uncaptured_error(catch_all_error);

        let swapchain_format = surface.get_supported_formats(&adapter)[0];

        let camera = CameraUniform::new(size.width as f32 / size.height as f32);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let render_pipeline = generate_render_pipeline(
            &shader,
            swapchain_format,
            &render_pipeline_layout,
            sample_count,
            |render_pipeline_descriptor| device.create_render_pipeline(&render_pipeline_descriptor),
        );

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
        };

        surface.configure(&device, &config);

        let multisampled_framebuffer =
            create_multisampled_framebuffer(&device, &config, sample_count);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            inner: AppInner {
                window,
                event_loop: Some(event_loop),
                instance,
                render_pipeline,
                device,
                queue,
                config,
                surface,
                swapchain_format,
                render_pipeline_layout,
                multisampled_framebuffer,
                sample_count,
            },
            shader,
            vertex_buffer,
            camera,
            camera_buffer,
            camera_bind_group,
            camera_controller: CameraController::new(),
        }
    }

    pub fn run(mut self) {
        let mut last = std::time::Instant::now();
        let event_loop = std::mem::take(&mut self.inner.event_loop).unwrap();

        event_loop.run(move |event, _, control_flow| {
            use winit::event::Event;
            use winit::event::KeyboardInput;
            use winit::event::VirtualKeyCode;
            use winit::event::WindowEvent;
            control_flow.set_wait(); // Bc Mandelbrot isn't animated

            let dt = last.elapsed();
            //println!("Elapsed since last frame: {}ns", dt.as_nanos());
            last = std::time::Instant::now();

            if self.camera_controller.update_camera(dt, &mut self.camera) {
                self.inner.window.request_redraw();
                self.inner.queue.write_buffer(
                    &self.camera_buffer,
                    0,
                    bytemuck::cast_slice(&[self.camera]),
                );
                control_flow.set_poll();
            }

            match event {
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    // On minimize?
                    if size.width == 0 || size.height == 0 {
                        return;
                    }

                    self.inner.config.width = size.width;
                    self.inner.config.height = size.height;
                    self.camera_controller
                        .update_aspect_ratio(size.width as f32 / size.height as f32);
                    self.inner
                        .surface
                        .configure(&self.inner.device, &self.inner.config);
                    self.inner.multisampled_framebuffer = create_multisampled_framebuffer(
                        &self.inner.device,
                        &self.inner.config,
                        self.inner.sample_count,
                    );
                    self.inner.window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    self.draw();
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => control_flow.set_exit(),
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: winit::event::ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::R),
                                    ..
                                },
                            ..
                        },
                    ..
                } => {
                    log::info!("Reloading Shader");
                    self.reload_shader();
                    self.inner.window.request_redraw();
                }
                Event::WindowEvent { event, .. } => {
                    self.camera_controller.process_events(&event);
                }
                _ => {}
            }
        });
    }

    fn draw(&self) {
        let frame = self
            .inner
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .inner
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.inner.multisampled_framebuffer,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.inner.render_pipeline);
            rpass.set_bind_group(0, &self.camera_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.draw(0..(VERTICES.len() as u32), 0..1);
        }

        self.inner.queue.submit(Some(encoder.finish()));
        frame.present();
    }

    fn reload_shader(&mut self) {
        use std::io::Read;
        let mut data = String::new();
        std::fs::File::open("src/shader.wgsl")
            .expect("Can't open `shader.wgsl`")
            .read_to_string(&mut data)
            .unwrap();

        let shader = self
            .inner
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Owned(data)),
            });

        generate_render_pipeline(
            &shader,
            self.inner.swapchain_format,
            &self.inner.render_pipeline_layout,
            self.inner.sample_count,
            |render_pipeline_descriptor| {
                let render_pipeline = self
                    .inner
                    .device
                    .create_render_pipeline(&render_pipeline_descriptor);
                self.inner.render_pipeline = render_pipeline;
            },
        );
    }
}

fn create_multisampled_framebuffer(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    sample_count: u32,
) -> wgpu::TextureView {
    let multisampled_texture_extent = wgpu::Extent3d {
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        size: multisampled_texture_extent,
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
    };

    device
        .create_texture(multisampled_frame_descriptor)
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn generate_render_pipeline<'a, T, F: FnOnce(wgpu::RenderPipelineDescriptor) -> T>(
    shader: &'a wgpu::ShaderModule,
    swapchain_format: wgpu::TextureFormat,
    render_pipeline_layout: &'a wgpu::PipelineLayout,
    sample_count: u32,
    f: F,
) -> T {
    let targets = &[Some(swapchain_format.into())];
    let render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
        label: Some("MainRenderPipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[Vertex::layout()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets,
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        multiview: None,
    };

    f(render_pipeline_descriptor)
}

fn catch_all_error(err: wgpu::Error) {
    log::error!("{}", err);
}
