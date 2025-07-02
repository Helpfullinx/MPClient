mod components;
mod network;

use std::collections::HashMap;
use bevy::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use bevy_inspector_egui::quick::{ResourceInspectorPlugin};
use tokio::io;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use crate::components::player::{PlayerInfo, player_control, update_players, reconcile_player_position};
use crate::network::net_manage::{init_connection, start_udp_task, Communication};
use crate::network::net_reconciliation::ReconcileBuffer;
use crate::network::net_system::{udp_client_net_recieve, udp_client_net_send, NetworkMessages};

#[derive(Resource)]
pub struct ServerSocket(pub SocketAddr);

#[derive(Resource)]
pub struct Lobby(u128);

#[derive(Component)]
pub struct UuidText;

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let l_id = if args.len() > 1 {
        args[1].parse::<u128>().expect("First argument must be an Natural number")
    } else {
        1
    };

    let player_uid = init_connection(SocketAddr::new("100.113.246.10".parse().unwrap(), 4444), l_id).await?;

    let (udp_send_tx, udp_send_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);
    let (udp_receive_tx, udp_receive_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);
    let (tcp_send_tx, tcp_send_rx) = mpsc::channel::<(Vec<u8>, Arc<TcpStream>)>(1_000);
    let (tcp_receive_tx, tcp_receive_rx) = mpsc::channel::<(Vec<u8>, Arc<TcpStream>)>(1_000);
    
    start_udp_task("0.0.0.0:0", udp_send_rx, udp_receive_tx, 1).await?;
    
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DefaultInspectorConfigPlugin)
        .add_plugins(EguiPlugin {enable_multipass_for_primary_context: true})
        .add_plugins(ResourceInspectorPlugin::<PlayerInfo>::default())
        .insert_resource(ServerSocket(SocketAddr::new("100.113.246.10".parse().unwrap(), 4444)))
        .insert_resource(Communication::new(udp_send_tx, udp_receive_rx, tcp_send_tx, tcp_receive_rx))
        .insert_resource(PlayerInfo { current_player_id: player_uid , player_inputs: 0 })
        .insert_resource(NetworkMessages{ message: vec![] })
        .insert_resource(Lobby(l_id))
        .insert_resource(ReconcileBuffer{ buffer: HashMap::new(), sequence_counter: 0})
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .add_systems(Startup, setup)
        .add_systems(Update, (
            // spawn_players,
            update_players
        ).chain())
        .add_systems(FixedUpdate, (
            udp_client_net_recieve,
            (player_control, udp_client_net_send).chain(),
            reconcile_player_position
        ))
        .run();
    
    Ok(())
}

fn setup(
    mut commands: Commands,
    player_info: Res<PlayerInfo>,
) {
    commands.spawn(Camera2d);

    commands.spawn((
        UuidText,
        Text::new(player_info.current_player_id.to_string()),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.5),
            right: Val::Px(0.5),
            ..default()
        }
    ));
}