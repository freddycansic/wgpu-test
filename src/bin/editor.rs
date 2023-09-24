use shooter_game::common;

use cg::prelude::*;
use cgmath as cg;

use color_eyre::Result;
use fern::colors::Color;
use shooter_game::common::context::RenderingContext;
use winit::{event::*, event_loop::ControlFlow, window::Window, window::WindowBuilder};

use common::model::{BufferContents, DrawModels};
use common::*;

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

struct Editor {
    render_pipline: wgpu::RenderPipeline,
    view_projection_bind_group: wgpu::BindGroup,
    time: time::Time,
    render_data: render::RenderData,
    render_pipeline: wgpu::RenderPipeline,
}

impl Application for Editor {
    fn render(&self, render_pass: &mut wgpu::RenderPass, context: &RenderingContext) -> Result<()> {
        render_pass.set_bind_group(1, &self.view_projection_bind_group, &[]);
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.draw_models(&self.render_data);

        self.gui.render(
            &mut encoder,
            &view,
            &self.context,
            &self.time,
            &self.render_data,
        )?;

        Ok(())
    }
}

pub fn main() -> Result<()> {
    let log_colors = fern::colors::ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Blue);

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                chrono::Local::now().format("%H:%M:%S").to_string(),
                log_colors.color(record.level()),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Error)
        .level_for("shooter_game", log::LevelFilter::Trace)
        .chain(std::io::stdout())
        .apply()?;

    color_eyre::install()?;

    std::env::set_current_dir(std::path::Path::new("assets")).unwrap();
    let working_directory = std::env::current_dir().unwrap();
    log::info!("Working directory \"{}\"", working_directory.display());

    let event_loop =
        winit::event_loop::EventLoopBuilder::<gui::GuiEvent>::with_user_event().build();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = pollster::block_on(ApplicationHandler::new(window, application));

    event_loop.run(move |event, _, control_flow| {
        if input::cursor_state() == input::CursorState::Visible {
            state.gui.handle_event(&event);
        }

        input::update_input_state(&event, &state.context.window);

        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.context.window.id() => {
                state.process_window_event(event, control_flow)
            }
            Event::RedrawRequested(window_id) if window_id == state.context.window.id() => {
                state.time.start_frame();

                state.update();
                match state.render() {
                    Ok(_) => {}
                    // reconfigure the surface if lost
                    Err(err) => {
                        if err
                            .downcast_ref::<wgpu::SurfaceError>()
                            .is_some_and(|err| *err == wgpu::SurfaceError::Lost)
                        {
                            state.context.resize(state.context.size)
                        } else {
                            eprintln!("{:?}", err);
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                }

                state.time.end_frame();
            }
            Event::MainEventsCleared | Event::UserEvent(gui::GuiEvent::RequestRedraw) => {
                state.context.window.request_redraw();
            }
            _ => {}
        }
    });
}
