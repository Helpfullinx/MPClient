use std::collections::HashMap;
use std::fmt::Pointer;
use bevy::prelude::{Commands, Component, Entity, Query, ResMut, Resource};
use bincode::config;
use serde::{Deserialize, Serialize};
use crate::components::player::PlayerBundle;
use crate::network::net_manage::Communication;
use crate::network::net_message::{NetworkMessage, SequenceNumber};
use crate::network::net_message::NetworkMessageType::Sequence;
use crate::network::net_system::NetworkMessages;

pub const BUFFER_SIZE: usize = 1024;

#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct ReconcileObject(pub ReconcileType);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ReconcileType {
    Player { player_bundle: PlayerBundle },
}

#[derive(Resource)]
pub struct ReconcileBuffer{
    pub buffer: HashMap<SequenceNumber,Vec<ReconcileObject>>,
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
    mut message: Vec<NetworkMessage>,
    reconcile_objects: Vec<ReconcileObject>,
    reconcile_buffer: &mut ResMut<ReconcileBuffer>,
) -> Vec<NetworkMessage> {
    let current_sequence = reconcile_buffer.sequence_counter;
    
    if reconcile_buffer.sequence_counter > 1022 {
        reconcile_buffer.sequence_counter = 0;
    } else {
        reconcile_buffer.sequence_counter = current_sequence + 1;
    }

    reconcile_buffer.buffer.insert(current_sequence, reconcile_objects.clone());
    
    message.push(NetworkMessage(Sequence{sequence_number: current_sequence}));
    message
}

pub fn parse_udp_message(
    connection: &mut ResMut<Communication>,
) -> Option<Vec<NetworkMessage>> {
    let mut message = None;
    while !connection.udp_rx.is_empty() {
        match connection.udp_rx.try_recv() {
            Ok((bytes, _)) => {
                let decoded = bincode::serde::decode_from_slice(&bytes, config::standard()).unwrap();
                message = Some(decoded.0)
            }
            Err(_) => {}
        }
    }
    message
}