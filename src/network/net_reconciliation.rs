use std::collections::HashMap;
use bevy::prelude::{ResMut, Resource};
use crate::network::net_message::{NetworkMessage, SequenceNumber};

pub const BUFFER_SIZE: usize = 1024;

#[derive(Resource)]
pub struct MessageBuffer{
    pub buffer: HashMap<SequenceNumber,Vec<NetworkMessage>>,
    pub sequence_counter: SequenceNumber,
}

pub fn reconcile() {
    
}

pub fn sequence_message(message: Vec<NetworkMessage>, message_buffer: &mut ResMut<MessageBuffer>) -> (SequenceNumber, Vec<NetworkMessage>) {
    let current_sequence = message_buffer.sequence_counter;
    if message_buffer.sequence_counter > 1022 {
        message_buffer.sequence_counter = 0;
    } else {
        message_buffer.sequence_counter = current_sequence + 1;
    }
    
    message_buffer.buffer.insert(current_sequence, message.clone());
    
    (current_sequence, message)
}