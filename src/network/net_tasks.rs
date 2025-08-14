use std::time::SystemTime;
use crate::components::chat::{Chat, add_chat_message};
use crate::components::common::Id;
use crate::components::player::{PlayerInfo, reconcile_player, set_player_id, update_players, PlayerMarker, PlayerAnimationState};
use crate::network::net_manage::{TcpConnection, UdpConnection};
use crate::network::net_message::{NetworkMessage, STcpType, SUdpType};
use crate::network::net_reconciliation::ReconcileBuffer;
use bevy::asset::{AssetServer, Assets};
use bevy::pbr::StandardMaterial;
use bevy::prelude::{AnimationGraph, Commands, Entity, Gizmos, Mesh, Query, Res, ResMut, Transform, With};
use bincode::config;
use crate::components::camera::CameraInfo;
use crate::DefaultFont;
use crate::network::net_message::CUdpType::Ping;

pub fn handle_udp_message(
    mut gizmos: Gizmos,
    mut connection: ResMut<UdpConnection>,
    mut client_players: Query<(&mut Transform, &Id, Entity, &CameraInfo, &mut PlayerAnimationState), With<PlayerMarker>>,
    mut commands: Commands,
    mut asset_server: Res<AssetServer>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut reconcile_buffer: ResMut<ReconcileBuffer>,
    default_font: Res<DefaultFont>,
    player_info: Res<PlayerInfo>,
) {
    while let Some(p) = connection.input_packet_buffer.pop_front() {
        let decoded_message: (Vec<SUdpType>, usize) = match bincode::serde::decode_from_slice(&p.bytes, config::standard()) {
            Ok(m) => m,
            Err(e) => {
                println!("Couldn't decode UDP message: {:?}", e);
                continue;
            }
        };

        let mut seq_num = None;

        for m in decoded_message.0.iter() {
            match m {
                SUdpType::Sequence { sequence_number } => {
                    seq_num = Some(sequence_number);
                }
                _ => {}
            }
        }

        if seq_num.is_none() {
            println!("No sequence number given");
            continue;
        }
        
        for m in decoded_message.0.iter() {
            match m {
                SUdpType::Players { players } => {
                    reconcile_player(
                        &mut commands,
                        &mut gizmos,
                        *seq_num.unwrap(),
                        &players,
                        &mut client_players,
                        &player_info,
                        &mut reconcile_buffer,
                    );
                    update_players(
                        &mut commands,
                        &default_font,
                        &asset_server,
                        &mut animation_graphs,
                        // &mut meshes,
                        // &mut materials,
                        &players,
                        &mut client_players,
                        &player_info,
                    );
                },
                SUdpType::Pong { initiation_time, server_received_time } => {
                    let time_now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u32;
                    let rtt = time_now - *initiation_time;
                    
                    connection.ping = rtt;
                }
                SUdpType::Sequence { .. } => {}
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
        let mut decoded_message: (Vec<STcpType>, usize) = match bincode::serde::decode_from_slice(&p.bytes, config::standard()) {
            Ok(m) => m,
            Err(e) => {
                println!("Couldn't decode TCP message: {:?}", e);
                continue;
            }
        };
        
        for m in decoded_message.0.iter_mut() {
            match m {
                STcpType::Chat { messages } => {
                    add_chat_message(messages, &mut chat);
                },
                STcpType::PlayerId { player_uid } => {
                    set_player_id(&mut player_info, *player_uid, &mut reconcile_buffer);
                }
            }
        }
    }
}

pub fn add_ping_message(
    mut connection: ResMut<UdpConnection>
) {
    let time_now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u32;
    let last_rtt = connection.ping;
    connection.add_message(NetworkMessage(Ping{ intitiation_time: time_now, last_rtt }))
}
