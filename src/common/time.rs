pub struct Time {
    pub start: instant::Instant,
    pub current: instant::Duration,
    pub delta: instant::Duration,
    pub fps: f32,
}

impl Time {
    pub fn start_frame(&mut self) {
        self.current = self.start.elapsed();
        self.fps = 1.0 / self.delta.as_secs_f32();
    }

    pub fn end_frame(&mut self) {
        self.delta = self.start.elapsed() - self.current;
    }
}
