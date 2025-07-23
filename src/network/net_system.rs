use crate::Communication;
use crate::network::net_manage::{Packet, TcpConnection, UdpConnection};
use crate::network::net_reconciliation::{ReconcileBuffer, ObjectState, build_game_state, sequence_message, store_game_state};
use bevy::prelude::{Commands, Entity, Query, ResMut};
use bincode::config;
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::{TryRecvError, TrySendError};

pub fn udp_client_net_receive(
    mut comm: ResMut<Communication>,
    mut connection: ResMut<UdpConnection>,
) {
    while !comm.udp_rx.is_empty() {
        match comm.udp_rx.try_recv() {
            Ok((bytes, addr)) => {
                match connection.remote_socket {
                    Some(_) => {
                        connection.input_packet_buffer.push_back(Packet { bytes });
                    }
                    None => {
                        connection.remote_socket = Some(addr);
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
    mut object_states: Query<(Entity, &ObjectState)>,
    mut reconcile_buffer: ResMut<ReconcileBuffer>,
    mut commands: Commands,
) {
    let game_state = build_game_state(&mut object_states, &mut commands);

    if !connection.output_message.is_empty() {
        sequence_message(
            &mut connection.output_message,
            &reconcile_buffer,
        );
        
        store_game_state(
            game_state,
            &mut reconcile_buffer
        );
        
        reconcile_buffer.increment_sequence_num();
        
        let encoded_message = match bincode::serde::encode_to_vec(&connection.output_message, config::standard()) {
            Ok(m) => m,
            Err(e) => {
                println!("Couldn't encode UDP message: {:?}", e);
                return;
            }
        };

        if let Some(remote_socket) = &connection.remote_socket {
            match comm.udp_tx.try_send(( encoded_message, *remote_socket )) {
                Ok(()) => {
                    connection.output_message.clear();
                }
                Err(TrySendError::Full(_)) => {}
                Err(TrySendError::Closed(_)) => {}
            }
        }
        
    }
}

pub fn tcp_client_net_receive(
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
        let encoded_message = match bincode::serde::encode_to_vec(&connection.output_message, config::standard()) {
            Ok(m) => m,
            Err(e) => {
                println!("Couldn't encode TCP message: {:?}", e);
                return;
            }
        };

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
