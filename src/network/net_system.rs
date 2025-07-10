use crate::Communication;
use crate::network::net_manage::{Packet, TcpConnection, UdpConnection};
use crate::network::net_reconciliation::{
    ReconcileBuffer, ReconcileObject, build_reconcile_object_list, sequence_message,
};
use bevy::prelude::{Commands, Entity, Query, ResMut};
use bincode::config;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::{TryRecvError, TrySendError};

pub fn udp_client_net_receive(
    mut comm: ResMut<Communication>,
    mut connection: ResMut<UdpConnection>,
    command: Commands,
) {
    while !comm.udp_rx.is_empty() {
        match comm.udp_rx.try_recv() {
            Ok((bytes, addr)) => {
                // println!("Received UDP packet from {}", addr);
                match connection.ip_addrs {
                    Some(_) => {
                        connection.input_packet_buffer.push_back(Packet { bytes });
                    }
                    None => {
                        connection.ip_addrs = Some(addr);
                    }
                }
            }
            Err(_) => {}
        }
    }
}

pub fn udp_client_net_send(
    comm: ResMut<Communication>,
    mut connection: ResMut<UdpConnection>,
    mut reconcile_objects: Query<(Entity, &ReconcileObject)>,
    mut message_buffer: ResMut<ReconcileBuffer>,
    mut commands: Commands,
) {
    // Takes in all NetworkMessage that have been added to ECS and builds Network
    // let net_message = build_udp_message(&mut messages, &mut commands);
    let reconciled_objects = build_reconcile_object_list(&mut reconcile_objects, &mut commands);

    if !connection.output_message.is_empty() {
        sequence_message(
            &mut connection.output_message,
            reconciled_objects,
            &mut message_buffer,
        );
        let message =
            bincode::serde::encode_to_vec(&connection.output_message, config::standard())
                .unwrap();

        match comm.udp_tx.try_send((
            message,
            SocketAddr::from(([127, 0, 0, 1], 4444)), /*c.ip_addrs*/
        )) {
            Ok(()) => {
                connection.output_message.clear();
            }
            Err(TrySendError::Full(_)) => {}
            Err(TrySendError::Closed(_)) => {}
        }
    }
}

pub fn tcp_client_net_receive(
    commands: Commands,
    mut connection: ResMut<TcpConnection>,
    mut comm: ResMut<Communication>,
) {
    while !comm.tcp_rx.is_empty() {
        match comm.tcp_rx.try_recv() {
            Ok((bytes, stream)) => match connection.stream {
                Some(_) => {
                    connection.input_packet_buffer.push_back(Packet { bytes });
                }
                None => {
                    connection.stream = Some(stream);
                }
            },
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => break,
        }
    }
}

pub fn tcp_client_net_send(comm: ResMut<Communication>, mut connection: ResMut<TcpConnection>) {
    if !connection.output_message.is_empty() {
        let encoded_message =
            bincode::serde::encode_to_vec(&connection.output_message, config::standard())
                .unwrap();

        if let Some(s) = &connection.stream {
            match comm.tcp_tx.try_send((encoded_message, s.clone())) {
                Ok(()) => {
                    connection.output_message.clear();
                }
                Err(TrySendError::Full(_)) => return,
                Err(TrySendError::Closed(_)) => return,
            };
        }
    }
}

fn same_stream(a: &TcpStream, b: &TcpStream) -> bool {
    a.peer_addr().ok() == b.peer_addr().ok() && a.local_addr().ok() == b.local_addr().ok()
}
