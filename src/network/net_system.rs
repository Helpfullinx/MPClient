use std::net::SocketAddr;
use bevy::prelude::{Commands, Entity, Query, Res, ResMut, Resource};
use bincode::config;
use tokio::sync::mpsc::error::TrySendError;
use crate::{Communication, ServerSocket};
use crate::network::net_message::{build_udp_message, NetworkMessage, SequenceNumber};
use crate::network::net_reconciliation::{sequence_message, MessageBuffer};

#[derive(Resource)]
pub struct NetworkMessages(pub (SequenceNumber, Vec<NetworkMessage>));

pub fn udp_client_net_send(
    comm: ResMut<Communication>,
    server_socket: Res<ServerSocket>,
    mut messages: Query<(Entity, &NetworkMessage)>,
    mut message_buffer: ResMut<MessageBuffer>,
    mut commands: Commands
) {
    // Takes in all NetworkMessage that have been added to ECS and builds Network
    let net_message = build_udp_message(&mut messages, &mut commands);
    let sm = sequence_message(net_message, &mut message_buffer);
    
    let message = bincode::serde::encode_to_vec(sm, config::standard()).unwrap();
    
    match comm.udp_tx.try_send((message, server_socket.0)) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {},
        Err(TrySendError::Closed(_)) => {}
    }
}

pub fn parse_udp_message(
    mut connection: ResMut<Communication>,
    mut net_message: ResMut<NetworkMessages>
) {
    while !connection.udp_rx.is_empty() {
        match connection.udp_rx.try_recv() {
            Ok((bytes, _)) => {
                let decoded = bincode::serde::decode_from_slice(&bytes, config::standard()).unwrap();
                net_message.0 = decoded.0;
            }
            Err(_) => {}
        }
    }
}