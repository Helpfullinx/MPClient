use crate::components::chat::{Chat, add_chat_message};
use crate::components::common::Id;
use crate::components::player::{
    PlayerInfo, reconcile_player_position, set_player_id, update_players,
};
use crate::network::net_manage::{TcpConnection, UdpConnection};
use crate::network::net_message::{TCP, UDP};
use crate::network::net_reconciliation::ReconcileBuffer;
use bevy::asset::Assets;
use bevy::pbr::StandardMaterial;
use bevy::prelude::{Commands, Mesh, Query, Res, ResMut, Transform};
use bincode::config;

pub fn handle_udp_message(
    mut connection: ResMut<UdpConnection>,
    mut client_players: Query<(&mut Transform, &Id)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_info: Res<PlayerInfo>,
    reconcile_buffer: Res<ReconcileBuffer>,
) {
    while let Some(p) = connection.input_packet_buffer.pop_front() {
        let decoded_message: (Vec<UDP>, usize) = match bincode::serde::decode_from_slice(&p.bytes, config::standard()) {
            Ok(m) => m,
            Err(e) => {
                println!("Couldn't decode UDP message: {:?}", e);
                continue;
            }
        };

        let mut seq_num = None;

        for m in decoded_message.0.iter() {
            match m {
                UDP::Sequence { sequence_number } => {
                    seq_num = Some(sequence_number);
                }
                _ => {}
            }
        }

        if seq_num.is_none() {
            continue;
        };

        for m in decoded_message.0.iter() {
            match m {
                UDP::Players { players } => {
                    reconcile_player_position(
                        *seq_num.unwrap(),
                        &players,
                        &mut client_players,
                        &player_info,
                        &reconcile_buffer,
                    );
                    update_players(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &players,
                        &mut client_players,
                        &player_info,
                    );
                }
                _ => {}
            }
        }
    }
}

pub fn handle_tcp_message(
    mut player_info: ResMut<PlayerInfo>,
    mut chat: Query<&mut Chat>,
    mut connection: ResMut<TcpConnection>,
) {
    while let Some(p) = connection.input_packet_buffer.pop_front() {
        let mut decoded_message: (Vec<TCP>, usize) = match bincode::serde::decode_from_slice(&p.bytes, config::standard()) {
            Ok(m) => m,
            Err(e) => {
                println!("Couldn't decode TCP message: {:?}", e);
                continue;
            }
        };
        
        for m in decoded_message.0.iter_mut() {
            match m {
                TCP::ChatMessage { player_id, message } => {}
                TCP::Chat { messages } => {
                    add_chat_message(messages, &mut chat);
                }
                TCP::Join { .. } => {}
                TCP::PlayerId { player_uid } => {
                    set_player_id(&mut player_info, *player_uid);
                }
            }
        }
    }
}
