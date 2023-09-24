use color_eyre::eyre::Result;
use egui_winit_platform::PlatformDescriptor;

use crate::common::context;
use crate::common::context::RenderingContext;
use crate::common::input;

#[derive(Debug)]
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

pub struct GuiState {}

pub struct Gui {
    pub render_pass: egui_wgpu_backend::RenderPass,
    pub platform: egui_winit_platform::Platform,
    pub state: GuiState,
}

impl Gui {
    pub fn new(context: &RenderingContext) -> Result<Self> {
        let platform = egui_winit_platform::Platform::new(PlatformDescriptor {
            physical_width: context.wgpu.config.width,
            physical_height: context.wgpu.config.height,
            scale_factor: context.window.scale_factor(),
            font_definitions: egui::FontDefinitions::default(),
            style: egui::Style::default(),
        });

        let render_pass =
            egui_wgpu_backend::RenderPass::new(&context.wgpu.device, context.wgpu.config.format, 1);

        let state = GuiState {};

        Ok(Self {
            render_pass,
            platform,
            state,
        })
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        context: &context::RenderingContext,
        time: &crate::common::time::Time,
        render_data: &crate::common::render::RenderData,
    ) -> Result<()> {
        self.platform.begin_frame();

        self.show(time, render_data);

        if input::cursor_state() == input::CursorState::Hidden {
            self.platform
                .context()
                .set_cursor_icon(egui::CursorIcon::None);
        }

        let full_output = self.platform.end_frame(Some(&context.window));

        let paint_jobs = self.platform.context().tessellate(full_output.shapes);

        let screen_descriptor = egui_wgpu_backend::ScreenDescriptor {
            physical_width: context.wgpu.config.width,
            physical_height: context.wgpu.config.height,
            scale_factor: context.window.scale_factor() as f32,
        };

        let textures_delta = full_output.textures_delta;

        self.render_pass.add_textures(
            &context.wgpu.device,
            &context.wgpu.queue,
            &textures_delta,
        )?;

        self.render_pass.update_buffers(
            &context.wgpu.device,
            &context.wgpu.queue,
            &paint_jobs,
            &screen_descriptor,
        );
        self.render_pass
            .execute(encoder, view, &paint_jobs, &screen_descriptor, None)?;

        self.render_pass.remove_textures(textures_delta)?;

        Ok(())
    }

    pub fn handle_event<T>(&mut self, winit_event: &winit::event::Event<T>) {
        self.platform.handle_event(winit_event)
    }

    pub fn show(
        &mut self,
        time: &crate::common::time::Time,
        render_data: &crate::common::render::RenderData,
    ) {
        egui::Window::new("performance-window")
            .title_bar(false)
            .show(&self.platform.context(), |ui| {
                let color = match time.fps {
                    fps if fps < 60.0 => egui::Color32::RED,
                    fps if fps < 144.0 => egui::Color32::YELLOW,
                    _ => egui::Color32::WHITE,
                };

                let text = format!(
                    "{:.1} FPS\n{:.2} ms",
                    time.fps,
                    time.delta.as_micros() as f32 / 1000.0
                );
                ui.colored_label(color, text);
            });

        egui::Window::new("scene-viewer").show(&self.platform.context(), |ui| {
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for model in render_data.models.iter() {
                        ui.label(format!(
                            "{} | {}",
                            model.model.name.as_str(),
                            model
                                .instances
                                .as_ref()
                                .map_or(1, |instances| instances.len())
                        ));
                    }
                });
        });
    }
}
