use crate::network::net_message::{NetworkMessage, TCP, UDP, build_udp_message};
use crate::network::net_reconciliation::{
    ReconcileBuffer, ReconcileObject, build_reconcile_object_list,
    sequence_message,
};
use crate::{Communication, TcpSocket, UdpSocket};
use bevy::prelude::{Commands, Entity, Query, Res, ResMut, Resource};
use bincode::config;
use tokio::sync::mpsc::error::{TryRecvError, TrySendError};

#[derive(Resource, Debug)]
pub struct NetworkMessages {
    pub udp_messages: Vec<Vec<NetworkMessage<UDP>>>,
    pub tcp_messages: Vec<Vec<NetworkMessage<TCP>>>,
}

pub fn udp_client_net_receive(
    mut connection: ResMut<Communication>,
    mut net_message: ResMut<NetworkMessages>,
) {
    while !connection.udp_rx.is_empty() {
        match connection.udp_rx.try_recv() {
            Ok((bytes, _)) => {
                let decoded =
                    bincode::serde::decode_from_slice(&bytes, config::standard()).unwrap();
                
                match decoded.0 {
                    Some(m) => {
                        net_message.udp_messages.push(m);
                    }
                    None => {}
                }
            }
            Err(_) => {}
        }
    }
}

pub fn udp_client_net_send(
    comm: ResMut<Communication>,
    server_socket: Res<UdpSocket>,
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

pub fn parse_tcp_message(
    bytes: Vec<u8>
) -> Vec<NetworkMessage<TCP>> {
    bincode::serde::decode_from_slice(&bytes, config::standard()).unwrap().0
}

pub fn tcp_client_net_receive(
    mut commands: Commands,
    mut messages: ResMut<NetworkMessages>,
    mut connection: ResMut<TcpSocket>,
    mut comm: ResMut<Communication>,
) {
    while !comm.tcp_rx.is_empty() {
        match comm.tcp_rx.try_recv() {
            Ok((bytes, stream)) => {
                if let Some(_) = connection.0 {
                    let message = parse_tcp_message(bytes);                    
                    messages.tcp_messages.push(message);     
                } else {
                    connection.0 = Some(stream.clone());
                }
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => break,
        }
    }
}

pub fn tcp_client_net_send(
    comm: ResMut<Communication>,
    mut connection: ResMut<TcpSocket>
) {
    if connection.0.is_none() { return; }

    let message = bincode::serde::encode_to_vec(&c.output_message, config::standard()).unwrap();

    let x = comm.tcp_tx.try_send((message.clone(), c.stream.clone()));

    match x {
        Ok(()) => {
            c.output_message.clear();
        }
        Err(TrySendError::Full(_)) => break,
        Err(TrySendError::Closed(_)) => break,
    }
}
