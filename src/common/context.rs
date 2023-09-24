pub struct RenderingContext {
    pub wgpu: WgpuContext,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub window: winit::window::Window,
}

pub struct WgpuContext {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub depth_texture: crate::common::texture::Texture,
}

impl RenderingContext {
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        assert_ne!(new_size.width, 0);
        assert_ne!(new_size.height, 0);

        self.size = new_size;
        self.wgpu.config.width = new_size.width;
        self.wgpu.config.height = new_size.height;
        self.wgpu
            .surface
            .configure(&self.wgpu.device, &self.wgpu.config);

        self.wgpu.depth_texture = crate::common::texture::Texture::create_depth_texture(
            &self.wgpu.device,
            &self.wgpu.config,
            Some("depth-texture"),
        );
    }
}

impl RenderingContext {
    pub async fn new(window: winit::window::Window) -> Self {
        let size = window.inner_size();

        // instance = establish backend to create surfaces and adapters
        // all backends = vulkan, dx12, metal
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all() & !wgpu::Backends::BROWSER_WEBGPU,
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
                    limits: wgpu::Limits::default(),
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

        let depth_texture = crate::common::texture::Texture::create_depth_texture(
            &device,
            &config,
            Some("depth-texture"),
        );

        Self {
            wgpu: WgpuContext {
                surface,
                device,
                queue,
                config,
                depth_texture,
            },
            size,
            window,
        }
    }
}
