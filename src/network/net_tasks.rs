use avian3d::prelude::{AngularVelocity, LinearVelocity, Sleeping};
use crate::components::chat::{Chat, add_chat_message};
use crate::components::common::Id;
use crate::components::player::{PlayerInfo, reconcile_player, set_player_id, update_players, PlayerMarker};
use crate::network::net_manage::{TcpConnection, UdpConnection};
use crate::network::net_message::{TCP, UDP};
use crate::network::net_reconciliation::ReconcileBuffer;
use bevy::asset::Assets;
use bevy::log::Level;
use bevy::log::tracing::span;
use bevy::pbr::StandardMaterial;
use bevy::prelude::{Commands, Entity, EventWriter, Gizmos, Mesh, Query, Res, ResMut, Transform, With, World};
use bincode::config;

pub fn handle_udp_message(
    mut gizmos: Gizmos,
    mut connection: ResMut<UdpConnection>,
    mut client_players: Query<(&mut Transform, &Id, Entity), With<PlayerMarker>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_info: Res<PlayerInfo>,
    reconcile_buffer: ResMut<ReconcileBuffer>,
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
        }
        
        for m in decoded_message.0.iter() {
            match m {
                UDP::Players { players } => {
                    reconcile_player(
                        &mut commands,
                        &mut gizmos,
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
    mut reconcile_buffer: ResMut<ReconcileBuffer>
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
                    set_player_id(&mut player_info, *player_uid, &mut reconcile_buffer);
                }
            }
        }
    }
}
