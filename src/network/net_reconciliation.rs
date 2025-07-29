use crate::components::player::Player;
use crate::network::net_message::{BitMask, NetworkMessage, SequenceNumber, UDP};
use bevy::prelude::{Commands, Component, Entity, Query, ResMut, Resource};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::network::net_manage::{TcpConnection, UdpConnection};

pub const BUFFER_SIZE: usize = 1024;

#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct ObjectState(pub StateType);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum StateType {
    PlayerState { player: Player },
    InputState { encoded_input: BitMask }
}

#[derive(Resource)]
pub struct ReconcileBuffer {
    pub buffer: HashMap<SequenceNumber, Vec<ObjectState>>,
    pub sequence_counter: SequenceNumber,
}

impl ReconcileBuffer {
    pub fn increment_sequence_num(self: &mut Self) {
        if self.sequence_counter > 1022 {
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

    connection.add_message(NetworkMessage(UDP::Sequence {
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