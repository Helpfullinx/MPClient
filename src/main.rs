mod components;
mod network;

use crate::components::chat::{Chat, chat_window};
use crate::components::hud::Hud;
// use crate::components::lobby::Lobby;
use crate::components::player::{PlayerInfo, player_control, snap_camera_to_player};
use crate::network::net_manage::{
    Communication, TcpConnection, UdpConnection, start_tcp_task, start_udp_task,
};
use crate::network::net_message::{NetworkMessage, TCP};
use crate::network::net_reconciliation::ReconcileBuffer;
use crate::network::net_system::{
    tcp_client_net_receive, tcp_client_net_send, udp_client_net_receive, udp_client_net_send,
};
use crate::network::net_tasks::{handle_tcp_message, handle_udp_message};
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use avian3d::math::Scalar;
use avian3d::PhysicsPlugins;
use avian3d::prelude::{Collider, RigidBody};
use bevy::render::render_resource::TextureViewDimension::Cube;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use crate::components::common::Id;

const LOBBY_ID: u32 = 1;
fn join_lobby(
    mut keyboard_input: EventReader<KeyboardInput>,
    mut connection: ResMut<TcpConnection>,
) {
    for k in keyboard_input.read() {
        if k.state == ButtonState::Released {
            continue;
        };

        match k.key_code {
            KeyCode::KeyJ => {
                if connection.stream.is_some() {
                    connection
                        .output_message
                        .push(NetworkMessage(TCP::Join { lobby_id: Id(LOBBY_ID) }));
                }
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let (udp_send_tx, udp_send_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);
    let (udp_receive_tx, udp_receive_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);
    let (tcp_send_tx, tcp_send_rx) = mpsc::channel::<(Vec<u8>, Arc<TcpStream>)>(1_000);
    let (tcp_receive_tx, tcp_receive_rx) = mpsc::channel::<(Vec<u8>, Arc<TcpStream>)>(1_000);
    
    let remote_addr = SocketAddr::from(([100, 113, 246, 10], 4444));

    start_tcp_task(remote_addr, tcp_send_rx, tcp_receive_tx).await?;
    start_udp_task(remote_addr, udp_send_rx, udp_receive_tx, 1).await?;

    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .add_plugins(DefaultInspectorConfigPlugin)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .add_plugins(ResourceInspectorPlugin::<PlayerInfo>::default())
        .insert_resource(Communication::new(
            udp_send_tx,
            udp_receive_rx,
            tcp_send_tx,
            tcp_receive_rx,
        ))
        .insert_resource(UdpConnection {
            remote_socket: None,
            input_packet_buffer: Default::default(),
            output_message: vec![],
        })
        .insert_resource(TcpConnection {
            stream: None,
            input_packet_buffer: Default::default(),
            output_message: vec![],
        })
        .insert_resource(PlayerInfo {
            current_player_id: Id(0),
            player_inputs: 0,
        })
        .insert_resource(ReconcileBuffer {
            buffer: HashMap::new(),
            sequence_counter: 0,
        })
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .add_systems(Startup, setup)
        .add_systems(Update, (snap_camera_to_player,).chain())
        .add_systems(
            FixedUpdate,
            (
                udp_client_net_receive,
                tcp_client_net_receive,
                handle_udp_message.after(udp_client_net_receive),
                handle_tcp_message.after(tcp_client_net_receive),
                (player_control, udp_client_net_send).chain(),
                ((join_lobby, chat_window), tcp_client_net_send).chain(),
            ),
        )
        .run();

    Ok(())
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(4.0, 0.1, 4.0),
        Mesh3d(meshes.add(Cuboid::new(4.0,0.1,4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    
    commands.spawn((PointLight::default(), Transform::from_xyz(0.0, 0.0, 0.0)));

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

    commands.spawn((
        Chat {
            chat_history: VecDeque::new(),
        },
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.5),
            left: Val::Px(0.5),
            ..default()
        },
    ));
}
