use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{AccumulatedMouseMotion, MouseMotion};
use bevy::prelude::{ButtonInput, EventReader, KeyCode, Res, ResMut, Vec2};
use crate::components::player::{MovementState, PlayerInfo};

pub fn input_system(
    mouse_input: Res<AccumulatedMouseMotion>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_info: ResMut<PlayerInfo>,
) {
    player_info.player_inputs = 0;
    player_info.player_movement_state.clear();
    
    // Accumulated mouse delta was one frame off
    // Adjusts the offset of the delta by one frame
    if player_info.accumulated_mouse_delta == Vec2::ZERO {
        player_info.accumulated_mouse_delta = player_info.mouse_delta;
    }
    
    player_info.mouse_delta = mouse_input.delta;
    
    player_info.accumulated_mouse_delta += mouse_input.delta;
    
    if keyboard_input.pressed(KeyCode::KeyW) {
        player_info.player_inputs |= 1;
        player_info.player_movement_state.insert(MovementState::Walking);
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        player_info.player_inputs |= 2;
        player_info.player_movement_state.insert(MovementState::Walking);
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        player_info.player_inputs |= 4;
        player_info.player_movement_state.insert(MovementState::Walking);
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        player_info.player_inputs |= 8;
        player_info.player_movement_state.insert(MovementState::Walking);
    }
    if keyboard_input.pressed(KeyCode::Space) {
        player_info.player_inputs |= 16;
        player_info.player_movement_state.insert(MovementState::Jumping);
    }
    if keyboard_input.pressed(KeyCode::ShiftLeft) {
        player_info.player_inputs |= 32;
        player_info.player_movement_state.insert(MovementState::Running);
        player_info.player_movement_state.remove(&MovementState::Walking);
    }
    
    if player_info.player_inputs == 0 {
        player_info.player_movement_state.insert(MovementState::Idle);
    }
}