use once_cell::sync::Lazy;
use std::sync::RwLock;

use cgmath as cg;
use winit::{dpi::PhysicalPosition, event::*};
use winit_input_helper::WinitInputHelper;

#[derive(Clone, Copy, PartialEq)]
pub enum CursorState {
    Visible,
    Hidden,
}

static CURSOR_STATE: RwLock<CursorState> = RwLock::new(CursorState::Hidden);

const NUM_KEYS: usize = 162;
static KEY_STATES: RwLock<[ElementState; NUM_KEYS]> =
    RwLock::new([ElementState::Released; NUM_KEYS]);

static WINIT_INPUT_HELPER: Lazy<RwLock<WinitInputHelper>> =
    Lazy::new(|| RwLock::new(WinitInputHelper::new()));

#[derive(Debug)]
struct MouseMovement {
    current_position: cg::Point2<f32>,
    window_center: cg::Point2<f32>,
}

static MOUSE_MOVEMENT: RwLock<MouseMovement> = RwLock::new(MouseMovement {
    current_position: cg::Point2::new(0.0, 0.0),
    window_center: cg::Point2::new(0.0, 0.0),
});

pub fn key_pressed(key: VirtualKeyCode) -> bool {
    WINIT_INPUT_HELPER.read().unwrap().key_pressed(key)
}

pub fn key_released(key: VirtualKeyCode) -> bool {
    WINIT_INPUT_HELPER.read().unwrap().key_released(key)
}

pub fn key_held(key: VirtualKeyCode) -> bool {
    WINIT_INPUT_HELPER.read().unwrap().key_held(key)
}

pub fn key_down(key: VirtualKeyCode) -> bool {
    KEY_STATES.read().unwrap()[key as usize] == ElementState::Pressed
}

pub fn key_up(key: VirtualKeyCode) -> bool {
    KEY_STATES.read().unwrap()[key as usize] == ElementState::Released
}

pub fn mouse_diff() -> cg::Vector2<f32> {
    MOUSE_MOVEMENT.read().unwrap().current_position - MOUSE_MOVEMENT.read().unwrap().window_center
}

pub fn update_input_state(event: &Event<crate::gui::GuiEvent>, window: &winit::window::Window) {
    WINIT_INPUT_HELPER.write().unwrap().update(event);

    if let Event::WindowEvent { ref event, .. } = event {
        update_key_state(event);

        // TODO TEMP, move all keybinds into another file
        if key_released(VirtualKeyCode::G) {
            match cursor_state() {
                CursorState::Hidden => *CURSOR_STATE.write().unwrap() = CursorState::Visible,
                CursorState::Visible => {
                    let window_center = MOUSE_MOVEMENT.read().unwrap().window_center;
                    // force mouse position back to center of screen, hacky
                    *CURSOR_STATE.write().unwrap() = CursorState::Hidden;
                    update_mouse_diff(window_center, window);
                    return;
                }
            }
        }

        if let WindowEvent::CursorMoved { position, .. } = event {
            update_mouse_diff(
                cg::Point2::new(position.x as f32, position.y as f32),
                window,
            );
        }
    }
}

pub fn cursor_state() -> CursorState {
    *CURSOR_STATE.read().unwrap()
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

fn update_mouse_diff(current_position: cg::Point2<f32>, window: &winit::window::Window) {
    MOUSE_MOVEMENT.write().unwrap().current_position = current_position;

    let size = window.inner_size();
    MOUSE_MOVEMENT.write().unwrap().window_center =
        cg::Point2::new((size.width / 2) as f32, (size.height / 2) as f32);

    if cursor_state() == CursorState::Hidden {
        window
            .set_cursor_position(PhysicalPosition::new(
                MOUSE_MOVEMENT.read().unwrap().window_center.x,
                MOUSE_MOVEMENT.read().unwrap().window_center.y,
            ))
            .unwrap();
    }
}
