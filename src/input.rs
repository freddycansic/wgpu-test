use std::sync::RwLock;
use winit::event::*;

const NUM_KEYS: usize = 162;
pub static KEY_STATES: RwLock<[ElementState; NUM_KEYS]> =
    RwLock::new([ElementState::Released; NUM_KEYS]);

pub fn update_key_state(event: &WindowEvent) {
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
        KEY_STATES.write().unwrap()[virtual_keycode as usize] = state
    }
}

pub fn is_key(key: VirtualKeyCode, key_state: ElementState) -> bool {
    KEY_STATES.read().unwrap()[key as usize] == key_state
}
