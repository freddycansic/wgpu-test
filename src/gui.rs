use anyhow::Result;
use egui_winit_platform::PlatformDescriptor;

pub enum GuiEvent {
    RequestRedraw,
}

// egui hook into winit so that it can request redraws from separate threads
struct RepaintSignal(std::sync::Mutex<winit::event_loop::EventLoopProxy<GuiEvent>>);

impl epi::backend::RepaintSignal for RepaintSignal {
    fn request_repaint(&self) {
        self.0
            .lock()
            .unwrap()
            .send_event(GuiEvent::RequestRedraw)
            .ok();
    }
}

pub struct Gui {
    pub render_pass: egui_wgpu_backend::RenderPass,
    pub platform: egui_winit_platform::Platform,
    pub demo_app: egui_demo_lib::DemoWindows,
}

impl Gui {
    pub fn new(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        window: &winit::window::Window,
    ) -> Self {
        let platform = egui_winit_platform::Platform::new(PlatformDescriptor {
            physical_width: surface_config.width,
            physical_height: surface_config.height,
            scale_factor: window.scale_factor(),
            font_definitions: egui::FontDefinitions::default(),
            style: egui::Style::default(),
        });

        let render_pass = egui_wgpu_backend::RenderPass::new(device, surface_config.format, 1);

        let demo_app = egui_demo_lib::DemoWindows::default();

        Self {
            platform,
            render_pass,
            demo_app,
        }
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        config: &wgpu::SurfaceConfiguration,
        window: &winit::window::Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<()> {
        self.platform.begin_frame();

        self.show();

        let full_output = self.platform.end_frame(Some(window));
        let paint_jobs = self.platform.context().tessellate(full_output.shapes);

        let screen_descriptor = egui_wgpu_backend::ScreenDescriptor {
            physical_width: config.width,
            physical_height: config.height,
            scale_factor: window.scale_factor() as f32,
        };

        let textures_delta = full_output.textures_delta;

        self.render_pass
            .add_textures(device, queue, &textures_delta)?;

        self.render_pass
            .update_buffers(device, queue, &paint_jobs, &screen_descriptor);
        self.render_pass
            .execute(encoder, &view, &paint_jobs, &screen_descriptor, None)?;

        self.render_pass.remove_textures(textures_delta)?;

        Ok(())
    }

    pub fn show(&mut self) {
        egui::SidePanel::left("Hello")
            .default_width(100.0)
            .show(&self.platform.context(), |ui| {
                todo!("match deltatime and go red if fps < 60")
                // ui.label(egui::RichText::text(&self))
            });
    }
}
