use winit::event::*;

pub fn event_on_key(event: &WindowEvent, key: VirtualKeyCode, key_state: ElementState) -> bool {
    match *event {
        WindowEvent::KeyboardInput {
            input:
                KeyboardInput {
                    state,
                    virtual_keycode,
                    ..
                },
            ..
        } => {
            if let Some(virtual_keycode) = virtual_keycode {
                state == key_state && key == virtual_keycode
            } else {
                false
            }
        }
        _ => false,
    }
}
