use crate::network::net_message::{NetworkMessage, TCP, UDP, build_udp_message};
use crate::network::net_reconciliation::{
    ReconcileBuffer, ReconcileObject, build_reconcile_object_list, parse_udp_message,
    sequence_message,
};
use crate::{Communication, ServerSocket};
use bevy::prelude::{Commands, Entity, Query, Res, ResMut, Resource};
use bincode::config;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::error::TrySendError;

#[derive(Resource, Debug)]
pub struct NetworkMessages {
    pub udp_message: Vec<NetworkMessage<UDP>>,
    pub tcp_message: Vec<NetworkMessage<TCP>>,
}

pub fn udp_client_net_recieve(
    mut connection: ResMut<Communication>,
    mut net_message: ResMut<NetworkMessages>,
) {
    let message = parse_udp_message(&mut connection);

    match message {
        Some(m) => {
            net_message.udp_message = m;
        }
        None => {}
    }
}

pub fn udp_client_net_send(
    comm: ResMut<Communication>,
    server_socket: Res<ServerSocket>,
    mut messages: Query<(Entity, &NetworkMessage<UDP>)>,
    mut reconcile_objects: Query<(Entity, &ReconcileObject)>,
    mut message_buffer: ResMut<ReconcileBuffer>,
    mut commands: Commands,
) {
    // Takes in all NetworkMessage that have been added to ECS and builds Network
    let net_message = build_udp_message(&mut messages, &mut commands);
    let reconciled_objects = build_reconcile_object_list(&mut reconcile_objects, &mut commands);
    let sm = sequence_message(net_message, reconciled_objects, &mut message_buffer);

    let message = bincode::serde::encode_to_vec(sm, config::standard()).unwrap();

    match comm.udp_tx.try_send((message, server_socket.0)) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {}
        Err(TrySendError::Closed(_)) => {}
    }
}
