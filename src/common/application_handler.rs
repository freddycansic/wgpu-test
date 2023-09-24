use crate::common::*;

pub struct ApplicationHandler<App: application::Application> {
    app: App,
    context: context::RenderingContext,
    render_pipeline: wgpu::RenderPipeline,
    camera: camera::Camera,
    view_projection_buffer: wgpu::Buffer,
    view_projection_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_atlas: texture::TextureAtlas,
    time: time::Time,
    render_data: render::RenderData,
    gui: gui::Gui,
}

impl<App> ApplicationHandler<App> {
    async fn new(window: winit::window::Window) -> Self {
        let context = context::RenderingContext::new(window).await;

        // bind group describes a set of resources and how they can be accessed by the shaders
        let texture_bind_group_layout =
            context
                .wgpu
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    label: Some("texture_bind_group_layout"),
                });

        let view_projection_buffer = context.wgpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("view_projection_buffer"),
            size: std::mem::size_of::<MatrixUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let view_projection_bind_group_layout =
            context
                .wgpu
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("view_projection_bind_group_layout"),
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

        let view_projection_bind_group =
            context
                .wgpu
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("view_projection_bind_group"),
                    layout: &view_projection_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: view_projection_buffer.as_entire_binding(),
                    }],
                });

        let render_pipeline_layout =
            context
                .wgpu
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("render_pipeline_layout"),
                    bind_group_layouts: &[
                        &texture_bind_group_layout,
                        &view_projection_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let render_pipeline = pipeline::create_pipeline(
            &context.wgpu.device,
            &context.wgpu.config,
            &render_pipeline_layout,
            // TODO HELP!
            wgpu::include_wgsl!("../../assets/shader.wgsl"),
            &[
                model::ModelVertex::buffer_layout(),
                instance::RawInstance::buffer_layout(),
            ],
        );

        let camera = camera::Camera {
            position: (0.0, 2.0, 2.0).into(),
            direction: -cg::Vector3::unit_z(),
            pitch: 0.0,
            yaw: 0.0,
            up: cg::Vector3::unit_y(),
            aspect_ratio: context.size.width as f32 / context.size.height as f32,
            fov: 45.0,
            z_near: 0.1,
            z_far: 100.0,
        };

        let gui = gui::Gui::new(&context).unwrap();

        let mut texture_atlas = texture::TextureAtlas::new();

        let cube = model::Model::load(
            "cube.obj".to_string(),
            &mut texture_atlas,
            &texture_bind_group_layout,
            &context.wgpu,
        )
        .unwrap();

        let map = model::Model::load(
            "map.obj".to_string(),
            &mut texture_atlas,
            &texture_bind_group_layout,
            &context.wgpu,
        )
        .unwrap();

        let mut render_data = render::RenderData::new(&context.wgpu.device);

        let models = vec![map.into(), cube.into()];
        render_data.models = models;

        let application = App::new(&context);

        Self {
            context,
            render_pipeline,
            camera,
            view_projection_buffer,
            view_projection_bind_group,
            texture_bind_group_layout,
            time: time::Time {
                start: instant::Instant::now(),
                current: instant::Duration::default(),
                delta: instant::Duration::default(),
                fps: 0.0,
            },
            texture_atlas,
            render_data,
            gui,
            application,
        }
    }

    // called per event
    fn process_window_event(&mut self, event: &WindowEvent, control_flow: &mut ControlFlow) {
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        ApplicationHandler: ElementApplicationHandler::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                ..
            } => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(physical_size) => {
                self.context.resize(*physical_size);
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                self.context.resize(**new_inner_size);
            }
            WindowEvent::CursorMoved { .. } => {
                if input::cursor_ApplicationHandler() == input::CursorApplicationHandler::Hidden {
                    self.camera.update_direction(self.time.delta);
                }
            }
            _ => {}
        }
    }

    // called per frame
    fn update(&mut self) {
        self.context.wgpu.queue.write_buffer(
            &self.view_projection_buffer,
            0,
            bytemuck::cast_slice(&[self.camera.build_view_projection_matrix().to_uniform()]),
        );

        self.render_data.models[1].instances = Some(
            // TODO magic num
            (0..20)
                .flat_map(|x| {
                    // TODO
                    let num_instances_per_column = 20 as f32;
                    let pi = std::f32::consts::PI;
                    let angle = x as f32 / num_instances_per_column * pi * 2.0; // between 0 and 2pi
                    let angle = (angle + self.time.current.as_secs_f32()) % (pi * 2.0); // shift period by time
                    let y = angle.sin() * 2.0;

                    // TODO
                    (0..20).map(move |z| instance::ModelInstance {
                        position: cg::Vector3 {
                            x: x as f32 * 2.5,
                            y: y * 3.0,
                            z: z as f32 * 2.5,
                        },
                        rotation: cg::Quaternion::from_angle_x(cg::Deg(0.0)),
                    })
                })
                .collect(),
        );

        self.render_data.update_instance_buffers(&self.context.wgpu);
        self.camera.update_position(self.time.delta);
    }

    fn render(&mut self) -> Result<()> {
        // wait for surface to provide a new SurfaceTexture to write on
        let output = self.context.wgpu.surface.get_current_texture()?;

        // view describes a texture
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // creates and encodes commands to send to the gpu
        let mut encoder =
            self.context
                .wgpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("render_encoder"),
                });

        // begin_render_pass returns a render pass with the same lifetime as the encoder, since the encoder is borrowed mutably for this function it cannot be borrowed later on as immutable unless the render pass is dropped and the reference dropped, hence the limiting scope
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
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
                    view: &self.context.wgpu.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
        }

        // submit to render queue
        self.context
            .wgpu
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
