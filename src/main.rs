mod components;
mod network;
mod test;

use crate::components::chat::{Chat, chat_window};
use crate::components::hud::Hud;
use crate::components::player::{PlayerInfo, player_control, PlayerMarker, update_label_pos};
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
use avian3d::prelude::{Collider, Friction, LinearVelocity, Physics, PhysicsDebugPlugin, PhysicsDiagnosticsPlugin, PhysicsDiagnosticsUiPlugin, PhysicsSet, PhysicsTime, RigidBody, Sleeping};
use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::log::LogPlugin;
use bevy::render::render_resource::TextureViewDimension::Cube;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use crate::components::camera::camera_controller;
use crate::components::common::Id;
use crate::network::NetworkPlugin;

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
                    connection.add_message(NetworkMessage(TCP::Join { lobby_id: Id(LOBBY_ID) }));
                }
            }
            _ => {}
        }
    }
}

fn main() -> io::Result<()> {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default().with_length_unit(10.0)))
        .add_plugins(DefaultInspectorConfigPlugin)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .add_plugins(FpsOverlayPlugin::default())
        .add_plugins(PhysicsDebugPlugin::default())
        .add_plugins(ResourceInspectorPlugin::<PlayerInfo>::default())
        .add_plugins(NetworkPlugin)
        .insert_resource(PlayerInfo {
            current_player_id: Id(0),
            player_inputs: 0,
        })
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(Time::<Physics>::default().with_relative_speed(1.0))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                camera_controller,
                update_label_pos
            )
        )
        .add_systems(
            FixedUpdate,
            (
                player_control,
                join_lobby,
                chat_window,
                // debug_player_sleeping
                // linear_is_changed
            )
        )
        .run();

    Ok(())
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Main Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(10.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground Plane
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(40.0, 0.5, 40.0),
        Mesh3d(meshes.add(Cuboid::new(40.0,0.5,40.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    //Light Source
    commands.spawn((PointLight::default(), Transform::from_xyz(0.0, 10.0, 0.0)));

    //Position and ID Hud
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

    // Chat Window
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

fn linear_is_changed(
    id: Query<&Id, Changed<LinearVelocity>>,
) {
    for id in id.iter() {
        println!("player linear velo changed: {:?}", id);
    }
}

fn debug_player_sleeping(
    sleeping_players: Query<(&LinearVelocity, &PlayerMarker), With<Sleeping>>,
    nonsleeping_players: Query<(&LinearVelocity, &PlayerMarker), Without<Sleeping>>,
) {
    for p in sleeping_players.iter() {
        println!("Sleeping: {:?}", p.0);
    }
    
    for p in nonsleeping_players.iter() {
        println!("NonSleeping: {:?}", p.0);
    }
}
