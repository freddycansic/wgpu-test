pub struct Time {
    pub start: instant::Instant,
    pub current: instant::Duration,
    pub delta: instant::Duration,
    pub fps: f32,
}
