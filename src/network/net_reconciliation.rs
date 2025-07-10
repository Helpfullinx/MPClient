use crate::components::player::PlayerBundle;
use crate::network::net_manage::Communication;
use crate::network::net_message::{NetworkMessage, SequenceNumber, UDP};
use bevy::prelude::{Commands, Component, Entity, Query, ResMut, Resource};
use bincode::config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Pointer;

pub const BUFFER_SIZE: usize = 1024;

#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct ReconcileObject(pub ReconcileType);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ReconcileType {
    Player { player_bundle: PlayerBundle },
}

#[derive(Resource)]
pub struct ReconcileBuffer {
    pub buffer: HashMap<SequenceNumber, Vec<ReconcileObject>>,
    pub sequence_counter: SequenceNumber,
}

pub fn build_reconcile_object_list(
    reconcile_objects: &mut Query<(Entity, &ReconcileObject)>,
    commands: &mut Commands,
) -> Vec<ReconcileObject> {
    let mut reconcile_list = Vec::new();
    for n in reconcile_objects.iter_mut() {
        reconcile_list.push(n.1.clone());
        commands.entity(n.0).despawn();
    }

    reconcile_list
}

pub fn sequence_message(
    message: &mut Vec<NetworkMessage<UDP>>,
    reconcile_objects: Vec<ReconcileObject>,
    reconcile_buffer: &mut ResMut<ReconcileBuffer>,
) {
    let current_sequence = reconcile_buffer.sequence_counter;

    if reconcile_buffer.sequence_counter > 1022 {
        reconcile_buffer.sequence_counter = 0;
    } else {
        reconcile_buffer.sequence_counter = current_sequence + 1;
    }

    reconcile_buffer
        .buffer
        .insert(current_sequence, reconcile_objects.clone());

    message.push(NetworkMessage(UDP::Sequence {
        sequence_number: current_sequence,
    }));
}
