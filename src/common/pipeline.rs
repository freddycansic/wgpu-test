pub fn create_pipeline(
    device: &wgpu::Device,
    surface_config: &wgpu::SurfaceConfiguration,
    layout: &wgpu::PipelineLayout,
    shader_description: wgpu::ShaderModuleDescriptor,
    buffers: &[wgpu::VertexBufferLayout],
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(shader_description);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("instanced_render_pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            // type of vertices to pass to the vertex shader
            buffers,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            // same color output state as the surface's
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_config.format,
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
            format: crate::common::texture::Texture::DEPTH_FORMAT,
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
    })
}
