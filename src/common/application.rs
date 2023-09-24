use super::context::RenderingContext;
use color_eyre::Result;

pub trait Application {
    fn new(context: &RenderingContext) -> Self;
    fn render(&self, render_pass: &mut wgpu::RenderPass) -> Result<()>;
}
