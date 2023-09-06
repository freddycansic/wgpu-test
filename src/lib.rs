mod gui;
mod input;
mod texture;
mod camera;

use cg::prelude::*;
use cgmath as cg;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use anyhow::Result;
use wgpu::util::DeviceExt;
use winit::{event::*, event_loop::ControlFlow, window::Window, window::WindowBuilder};

use texture::Texture;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct MatrixUniform([[f32; 4]; 4]);

trait ToUniform<T: bytemuck::Pod + bytemuck::Zeroable> {
    fn to_uniform(self) -> T;
}

impl ToUniform<MatrixUniform> for cg::Matrix4<f32> {
    fn to_uniform(self) -> MatrixUniform {
        MatrixUniform(self.into())
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    // (0, 0) is top left, (1, 1) is bottom right
    texture_coords: [f32; 2],
}

impl Vertex {
    const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::VERTEX_ATTRIBUTES,
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        texture_coords: [0.4131759, 0.00759614],
    },
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        texture_coords: [0.0048659444, 0.43041354],
    },
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        texture_coords: [0.28081453, 0.949397],
    },
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        texture_coords: [0.85967, 0.84732914],
    },
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        texture_coords: [0.9414737, 0.2652641],
    },
];

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

const NUM_INSTANCES_PER_ROW: u32 = 10;

#[derive(Clone)]
struct Instance {
    position: cg::Vector3<f32>,
    rotation: cg::Quaternion<f32>,
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            position: cg::Vector3::zero(),
            rotation: cg::Quaternion::zero(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw([[f32; 4]; 4]);

impl InstanceRaw {
    const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::INSTANCE_ATTRIBUTES,
        }
    }
}

impl From<&Instance> for InstanceRaw {
    fn from(instance: &Instance) -> Self {
        InstanceRaw(
            (cg::Matrix4::from_translation(instance.position)
                * cg::Matrix4::from(instance.rotation))
            .into(),
        )
    }
}

struct Time {
    start: instant::Instant,
    current: instant::Duration,
    delta: instant::Duration,
}

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    texture_bind_group: wgpu::BindGroup,
    depth_texture: Texture,
    camera: camera::Camera,
    view_projection_buffer: wgpu::Buffer,
    view_projection_bind_group: wgpu::BindGroup,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    time: Time,
    fps: f32,
}

impl State {
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        // instance = establish backend to create surfaces and adapters
        // all backends = vulkan, dx12, metal + web
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        // the part of the window that we draw to
        let surface = unsafe { instance.create_surface(&window).unwrap() };

        // handle to physical gpu
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    // disable some features if building for web
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);

        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(surface_capabilities.formats[0]);

        // define how the surface will create its SurfaceTextures
        let config = wgpu::SurfaceConfiguration {
            // SurfaceTextures will be used to write to the screen
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // how the textures will be stored on the gpu
            format: surface_format,
            width: size.width,
            height: size.height,
            // present_mode: wgpu::PresentMode::AutoVsync,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let diffuse_texture = Texture::from_bytes(
            include_bytes!("../assets/happy-tree.png"),
            &device,
            &queue,
            Some("passport-photo.jpg"),
        )
        .unwrap();

        let depth_texture = Texture::create_depth_texture(&device, &config, Some("depth-texture"));

        // bind group describes a set of resources and how they can be accessed by the shaders
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            // IDK
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // WHAT?
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("Texture Bind Group Layout"),
            });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Diffuse Bind Group"),
            layout: &texture_bind_group_layout,
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
        });

        let camera = camera::Camera {
            position: (0.0, 2.0, 2.0).into(),
            direction: -cg::Vector3::unit_z(),
            pitch: 0.0,
            yaw: 0.0,
            up: cg::Vector3::unit_y(),
            aspect_ratio: config.width as f32 / config.height as f32,
            fov: 45.0,
            z_near: 0.1,
            z_far: 100.0,
        };

        let view_projection_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("View Projection Buffer"),
            contents: bytemuck::cast_slice(&[camera.build_view_projection_matrix().to_uniform()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let view_projection_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("View Projection Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let view_projection_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("View Projection Bind Group"),
            layout: &view_projection_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: view_projection_buffer.as_entire_binding(),
            }],
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        // let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &view_projection_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                // type of vertices to pass to the vertex shader
                buffers: &[Vertex::buffer_layout(), InstanceRaw::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                // same color output state as the surface's
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    // replace old data in texture with new data e.g new frame
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                // triangles are considered forward facing if their vertices are in a counter clockwise order
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                // wgpu::PolygonMode::Line = wireframe
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                // antialiasing stuff
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            mapped_at_creation: false,
            size: std::mem::size_of::<[InstanceRaw; NUM_INSTANCES_PER_ROW.pow(2) as usize]>()
                as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            texture_bind_group,
            depth_texture,
            camera,
            view_projection_buffer,
            view_projection_bind_group,
            instances: vec![Instance::default(); NUM_INSTANCES_PER_ROW.pow(2) as usize],
            instance_buffer,
            time: Time {
                start: instant::Instant::now(),
                current: instant::Duration::default(),
                delta: instant::Duration::default(),
            },
            fps: 0.0,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        assert_ne!(new_size.width, 0);
        assert_ne!(new_size.height, 0);

        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);

        self.depth_texture = Texture::create_depth_texture(&self.device, &self.config, Some("depth-texture"));
    }

    // called per event
    fn process_window_event(&mut self, _event: &WindowEvent) {
        self.camera.update_direction(self.time.delta);
    }

    // called per frame
    fn update(&mut self) {
        // self.gui
        //     .platform
        //     .update_time(self.time.current.as_secs_f64());

        self.queue.write_buffer(
            &self.view_projection_buffer,
            0,
            bytemuck::cast_slice(&[self.camera.build_view_projection_matrix().to_uniform()]),
        );

        self.instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|x| {
                let num_instances = NUM_INSTANCES_PER_ROW as f32;
                let pi = std::f32::consts::PI;
                let angle = x as f32 / num_instances * pi * 2.0; // between 0 and 2pi
                let angle = (angle + self.time.current.as_secs_f32()) % (pi * 2.0); // shift period by time
                let y = angle.sin() * 2.0;

                (0..NUM_INSTANCES_PER_ROW).map(move |z| Instance {
                    position: cg::Vector3 {
                        x: x as f32,
                        y,
                        z: z as f32,
                    },
                    rotation: cg::Quaternion::from_angle_x(cg::Deg(0.0)),
                })
            })
            .collect();

        let instance_data = self
            .instances
            .iter()
            .map(InstanceRaw::from)
            .collect::<Vec<InstanceRaw>>();

        self.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&instance_data),
        );

        self.camera.update_position(self.time.delta);
    }

    fn render(&mut self) -> Result<()> {
        // wait for surface to provide a new SurfaceTexture to write on
        let output = self.surface.get_current_texture()?;

        // view describes a texture
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // creates and encodes commands to send to the gpu
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // begin_render_pass returns a render pass with the same lifetime as the encoder, since the encoder is borrowed mutably for this function it cannot be borrowed later on as immutable unless the render pass is dropped and the reference dropped, hence the limiting scope
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                // where to draw color to
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    // texture to recieve output, same as view unless using multisampling
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true
                    }),
                    stencil_ops: None
                }),
            });

            render_pass.set_pipeline(&self.render_pipeline);
            // uniforms
            render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            render_pass.set_bind_group(1, &self.view_projection_bind_group, &[]);
            // rendering
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..self.instances.len() as u32);
        }

        gui::gui()
            .write()
            .unwrap()
            .render(&mut encoder, &view, self)?;

        // submit to render queue
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop =
        winit::event_loop::EventLoopBuilder::<gui::GuiEvent>::with_user_event().build();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        // wasm-example = name of element in HTML to show the window
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dest = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dest.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("couldn't append canvas to document body");
    }

    let mut state = State::new(window).await;
    gui::gui()
        .write()
        .unwrap()
        .create(&state.device, &state.config, &state.window)
        .unwrap();

    event_loop.run(move |event, _, control_flow| {
        gui::gui().write().unwrap().handle_event(&event);
        input::update_input_state(&event);

        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => {
                state.process_window_event(event);

                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(window_id) if window_id == state.window().id() => {
                state.time.current = state.time.start.elapsed();
                state.fps = 1.0 / state.time.delta.as_secs_f32();

                state.update();
                match state.render() {
                    Ok(_) => {}
                    // reconfigure the surface if lost
                    Err(err) if err.downcast_ref::<wgpu::SurfaceError>().is_some() => {
                        let surface_error = err.downcast_ref::<wgpu::SurfaceError>().unwrap();
                        match surface_error {
                            wgpu::SurfaceError::Lost => state.resize(state.size),
                            _ => {
                                eprintln!("{:?}", surface_error);
                                *control_flow = ControlFlow::Exit;
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("{:?}", err);
                        *control_flow = ControlFlow::Exit
                    }
                }

                state.time.delta = state.time.start.elapsed() - state.time.current;
            }
            Event::MainEventsCleared | Event::UserEvent(gui::GuiEvent::RequestRedraw) => {
                state.window().request_redraw();
            }
            _ => {}
        }
    });
}
