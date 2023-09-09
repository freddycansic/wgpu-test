use std::sync::RwLock;

use color_eyre::eyre::{eyre, Result};
use egui_winit_platform::PlatformDescriptor;
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;

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
    pub render_pass: OnceCell<egui_wgpu_backend::RenderPass>,
    pub platform: OnceCell<egui_winit_platform::Platform>,
    pub state: GuiState,
}

static GUI: Lazy<RwLock<Gui>> = Lazy::new(|| RwLock::new(Gui::new()));

pub fn gui() -> &'static RwLock<Gui> {
    &GUI
}

impl Gui {
    pub fn new() -> Self {
        Self {
            render_pass: OnceCell::new(),
            platform: OnceCell::new(),
            state: GuiState {},
        }
    }

    pub fn create(
        &mut self,
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        window: &winit::window::Window,
    ) -> Result<()> {
        let platform = egui_winit_platform::Platform::new(PlatformDescriptor {
            physical_width: surface_config.width,
            physical_height: surface_config.height,
            scale_factor: window.scale_factor(),
            font_definitions: egui::FontDefinitions::default(),
            style: egui::Style::default(),
        });

        let render_pass = egui_wgpu_backend::RenderPass::new(device, surface_config.format, 1);

        self.platform
            .set(platform)
            .map_err(|_| eyre!("OnceCell: Could not create Gui, platform already set."))?;
        self.render_pass
            .set(render_pass)
            .map_err(|_| eyre!("OnceCell: Could not create Gui, render_pass already set."))?;

        Ok(())
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        state: &mut crate::State,
    ) -> Result<()> {
        self.platform.get_mut().unwrap().begin_frame();

        self.show(state);

        if !state.cursor_visible {
            self.platform
                .get()
                .unwrap()
                .context()
                .set_cursor_icon(egui::CursorIcon::None);
        }

        let full_output = self
            .platform
            .get_mut()
            .unwrap()
            .end_frame(Some(&state.window));

        let paint_jobs = self
            .platform
            .get()
            .unwrap()
            .context()
            .tessellate(full_output.shapes);

        let screen_descriptor = egui_wgpu_backend::ScreenDescriptor {
            physical_width: state.config.width,
            physical_height: state.config.height,
            scale_factor: state.window.scale_factor() as f32,
        };

        let textures_delta = full_output.textures_delta;

        self.render_pass.get_mut().unwrap().add_textures(
            &state.device,
            &state.queue,
            &textures_delta,
        )?;

        self.render_pass.get_mut().unwrap().update_buffers(
            &state.device,
            &state.queue,
            &paint_jobs,
            &screen_descriptor,
        );
        self.render_pass.get().unwrap().execute(
            encoder,
            view,
            &paint_jobs,
            &screen_descriptor,
            None,
        )?;

        self.render_pass
            .get_mut()
            .unwrap()
            .remove_textures(textures_delta)?;

        Ok(())
    }

    pub fn handle_event<T>(&mut self, winit_event: &winit::event::Event<T>) {
        self.platform.get_mut().unwrap().handle_event(winit_event)
    }

    pub fn show(&mut self, state: &mut crate::State) {
        egui::Window::new("performance-window")
            .title_bar(false)
            .show(&self.platform.get().unwrap().context(), |ui| {
                let color = match state.fps {
                    fps if fps < 60.0 => egui::Color32::RED,
                    fps if fps < 144.0 => egui::Color32::YELLOW,
                    _ => egui::Color32::WHITE,
                };

                let text = format!(
                    "{:.1} FPS\n{:.2} ms",
                    state.fps,
                    state.time.delta.as_micros() as f32 / 1000.0
                );
                ui.colored_label(color, text);
            });
    }
}
