use crate::components::player::Player;
use crate::network::net_message::{BitMask, NetworkMessage, SequenceNumber, CUdpType};
use bevy::prelude::{Commands, Component, Entity, Query, ResMut, Resource, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::network::net_manage::UdpConnection;

pub const BUFFER_SIZE: u16 = 1024;
pub const MISS_PREDICT_LIMIT: u16 = 20;

#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct ObjectState(pub StateType);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum StateType {
    PlayerState { player: Player },
    InputState { encoded_input: BitMask, mouse_delta: Vec2 }
}

#[derive(Resource)]
pub struct ReconcileBuffer {
    pub buffer: HashMap<SequenceNumber, Vec<ObjectState>>,
    pub sequence_counter: SequenceNumber,
    pub miss_predict_counter: u16
}

// #[derive(Resource)]
// pub struct ReconcilePlayerState {
//     pub player: Player
// }

impl ReconcileBuffer {
    pub fn increment_sequence_num(self: &mut Self) {
        if self.sequence_counter >= BUFFER_SIZE - 1 {
            self.sequence_counter = 0;
        } else {
            self.sequence_counter = self.sequence_counter + 1;
        }
    }
}

pub fn build_game_state(
    object_states: &mut Query<(Entity, &ObjectState)>,
    commands: &mut Commands,
) -> Vec<ObjectState> {
    let mut game_state = Vec::new();
    for n in object_states.iter_mut() {
        game_state.push(n.1.clone());
        commands.entity(n.0).despawn();
    }

    game_state
}

pub fn sequence_message(
    connection: &mut UdpConnection,
    reconcile_buffer: &ReconcileBuffer,
) {
    let current_sequence = reconcile_buffer.sequence_counter;

    connection.add_message(NetworkMessage(CUdpType::Sequence {
        sequence_number: current_sequence,
    }));
}

pub fn store_game_state(
    game_state: Vec<ObjectState>,
    reconcile_buffer: &mut ResMut<ReconcileBuffer>,
) {
    let current_sequence = reconcile_buffer.sequence_counter;
    
    reconcile_buffer
        .buffer
        .insert(current_sequence, game_state);
}