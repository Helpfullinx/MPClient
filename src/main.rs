mod components;
mod network;

use crate::components::player::{
    PlayerInfo, player_control, reconcile_player_position, snap_camera_to_player, update_players,
};
use crate::network::net_manage::{start_tcp_task, start_udp_task, Communication};
use crate::network::net_message::{NetworkMessage, TCP};
use crate::network::net_reconciliation::ReconcileBuffer;
use crate::network::net_system::{NetworkMessages, tcp_client_net_receive, tcp_client_net_send, udp_client_net_receive, udp_client_net_send};
use bevy::prelude::*;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpStream};
use tokio::sync::mpsc;

#[derive(Resource)]
pub struct UdpSocket(pub SocketAddr);

#[derive(Resource)]
pub struct TcpSocket(pub Option<Arc<TcpStream>>); 

#[derive(Resource)]
pub struct Lobby(u128);

#[derive(Component)]
pub struct Hud;

const IPADDR: &str = "127.0.0.1";

// pub fn init_connection(mut player_info: ResMut<PlayerInfo>) {
//     let mut uuid = 0;
// 
//     let handle = tokio::spawn(async move {
//         let addr = SocketAddr::new(IPADDR.parse().unwrap(), 4444);
//         let socket = TcpSocket::new_v4().unwrap();
//         let stream = socket.connect(addr).await.unwrap();
// 
//         stream.ready(Interest::WRITABLE).await.unwrap();
//         let mut encoded_data = Vec::new();
//         encoded_data.push(TCP::Join { lobby_id: 1 });
// 
//         stream
//             .try_write(
//                 bincode::serde::encode_to_vec(encoded_data, bincode::config::standard())
//                     .unwrap()
//                     .as_slice(),
//             )
//             .unwrap();
// 
//         let mut buf = [0; 200];
// 
//         stream.ready(Interest::READABLE).await;
//         stream.try_read(&mut buf);
// 
//         println!("uid: {:x?}", buf);
// 
//         let decoded: (Vec<TCP>, _) =
//             bincode::serde::decode_from_slice(&buf, bincode::config::standard()).unwrap();
// 
//         for m in decoded.0 {
//             match m {
//                 TCP::PlayerId { player_uid } => {
//                     uuid = player_uid;
//                 }
//                 _ => {}
//             }
//         }
//     });
// 
//     player_info.current_player_id = uuid;
// }
//
//

fn join_lobby(
    mut commands: Commands,
) {
    let lobby_id = 0;
    commands.spawn(NetworkMessage(TCP::Join { lobby_id }));
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let l_id = if args.len() > 1 {
        args[1]
            .parse::<u128>()
            .expect("First argument must be an Natural number")
    } else {
        1
    };

    let (udp_send_tx, udp_send_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);
    let (udp_receive_tx, udp_receive_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);
    let (tcp_send_tx, tcp_send_rx) = mpsc::channel::<(Vec<u8>, Arc<TcpStream>)>(1_000);
    let (tcp_receive_tx, tcp_receive_rx) = mpsc::channel::<(Vec<u8>, Arc<TcpStream>)>(1_000);

    start_tcp_task(IPADDR.parse().unwrap(), tcp_send_rx, tcp_receive_tx);
    start_udp_task("0.0.0.0:0", udp_send_rx, udp_receive_tx, 1).await?;

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DefaultInspectorConfigPlugin)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .add_plugins(ResourceInspectorPlugin::<PlayerInfo>::default())
        .insert_resource(UdpSocket(SocketAddr::new(IPADDR.parse().unwrap(), 4444)))
        .insert_resource(TcpSocket(None))
        .insert_resource(Communication::new(
            udp_send_tx,
            udp_receive_rx,
            tcp_send_tx,
            tcp_receive_rx,
        ))
        .insert_resource(PlayerInfo {
            current_player_id: 0,
            player_inputs: 0,
        })
        .insert_resource(NetworkMessages {
            udp_messages: vec![],
            tcp_messages: vec![],
        })
        .insert_resource(Lobby(l_id))
        .insert_resource(ReconcileBuffer {
            buffer: HashMap::new(),
            sequence_counter: 0,
        })
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .add_systems(Startup, (setup, join_lobby))
        .add_systems(
            Update,
            (
                // spawn_players,
                update_players,
                snap_camera_to_player,
            )
                .chain(),
        )
        .add_systems(
            FixedUpdate,
            (
                udp_client_net_receive,
                tcp_client_net_receive,
                (player_control, udp_client_net_send).chain(),
                tcp_client_net_send,
                reconcile_player_position,
            ),
        )
        .run();

    Ok(())
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((PointLight::default(), Transform::from_xyz(0.0, 0.0, 10.0)));

    commands.spawn((
        Hud,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.5),
            right: Val::Px(0.5),
            ..default()
        },
    ));
}
