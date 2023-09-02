use std::sync::RwLock;
use winit::event::*;
use winit_input_helper::WinitInputHelper;

const NUM_KEYS: usize = 162;
static KEY_STATES: RwLock<[ElementState; NUM_KEYS]> =
    RwLock::new([ElementState::Released; NUM_KEYS]);

lazy_static::lazy_static! {
    static ref WINIT_INPUT_HELPER: RwLock<WinitInputHelper> = RwLock::new(WinitInputHelper::new());
}

pub fn input() -> &'static RwLock<WinitInputHelper> {
    &WINIT_INPUT_HELPER
}

pub fn update_input_state(event: &Event<()>) {
    WINIT_INPUT_HELPER.write().unwrap().update(event);

    if let Event::WindowEvent { ref event, .. } = event {
        update_key_state(event)
    }
}

fn update_key_state(event: &WindowEvent) {
    if let WindowEvent::KeyboardInput {
        input:
            KeyboardInput {
                state,
                virtual_keycode: Some(virtual_keycode),
                ..
            },
        ..
    } = *event
    {
        KEY_STATES.write().unwrap()[virtual_keycode as usize] = state;
    }
}

pub trait ContinuousKeyPresses {
    fn key_down(&self, key: VirtualKeyCode) -> bool;
    fn key_up(&self, key: VirtualKeyCode) -> bool;
}

impl ContinuousKeyPresses for WinitInputHelper {
    /// First press until release
    fn key_down(&self, key: VirtualKeyCode) -> bool {
        KEY_STATES.read().unwrap()[key as usize] == ElementState::Pressed
    }
    
    /// First release until press
    fn key_up(&self, key: VirtualKeyCode) -> bool {
        KEY_STATES.read().unwrap()[key as usize] == ElementState::Released
    }
}