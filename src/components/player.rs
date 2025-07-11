use crate::components::common::{Id, Vec3};
use crate::components::hud::Hud;
use crate::network::net_manage::UdpConnection;
use crate::network::net_message::{NetworkMessage, SequenceNumber, UDP};
use crate::network::net_reconciliation::{ReconcileBuffer, ReconcileObject};
use bevy::asset::Assets;
use bevy::color::Color;
use bevy::input::ButtonInput;
use bevy::pbr::StandardMaterial;
use bevy::prelude::{Component, Reflect, Resource};
use bevy::prelude::{
    Camera3d, Commands, JustifyText, KeyCode, Mesh, Mesh3d, MeshMaterial3d, Query, ReflectResource, Res, ResMut, Sphere, Text,
    Text2d, TextLayout, Transform, With, Without,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use avian3d::math::Vector;
use avian3d::prelude::{AngularVelocity, Collider, RigidBody};
use crate::network::net_reconciliation::ReconcileType::PlayerObject;

#[derive(Reflect, Resource, Default)]
#[reflect(Resource)]
pub struct PlayerInfo {
    pub current_player_id: Id,
    pub player_inputs: u8,
}

#[derive(Component, Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq)]
pub struct Player {
    pub position: Vec3,
    pub angular_velocity: Vec3
}

impl Player {
    pub fn new(position: Vec3, angular_velocity: Vec3) -> Self {
        Self {
            position,
            angular_velocity
        }
    }
}

pub fn set_player_id(player_info: &mut ResMut<PlayerInfo>, player_id: Id) {
    player_info.current_player_id = player_id;
}

pub fn snap_camera_to_player(
    player_info: ResMut<PlayerInfo>,
    players: Query<(&Transform, &Id), Without<Camera3d>>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
) {
    let mut cam = camera.single_mut().unwrap();
    for player in players.iter() {
        if *player.1 == player_info.current_player_id {
            cam.translation.x = player.0.translation.x;
            cam.translation.z = player.0.translation.z;
        }
    }
}

pub fn player_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_info: ResMut<PlayerInfo>,
    mut players: Query<(&mut Transform, &Id)>,
    mut hud: Query<&mut Text, With<Hud>>,
    mut connection: ResMut<UdpConnection>,
    mut commands: Commands,
) {
    if connection.remote_socket.is_some() {
        let mut encoded_input = 0u8;

        if keyboard_input.pressed(KeyCode::KeyW) {
            encoded_input |= 1;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            encoded_input |= 2;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            encoded_input |= 4;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            encoded_input |= 8;
        }

        let player_id = player_info.current_player_id;
        let move_speed = 0.1;

        for (mut transform, id) in players.iter_mut() {
            if player_id == *id {
                if encoded_input & 1 > 0 {
                    transform.translation.x -= move_speed;
                }
                if encoded_input & 2 > 0 {
                    transform.translation.x += move_speed;
                }
                if encoded_input & 4 > 0 {
                    transform.translation.z -= move_speed;
                }
                if encoded_input & 8 > 0 {
                    transform.translation.z += move_speed;
                }

                if let Some(mut h) = hud.single_mut().ok() {
                    h.clear();
                    h.push_str(&format!(
                        "x: {:?}\ny: {:?}\nz: {:?}\n{:?}",
                        transform.translation.x, transform.translation.y, transform.translation.z, player_id
                    ));
                }

                let position = Vec3::new(
                    transform.translation.x,
                    transform.translation.y,
                    transform.translation.z,
                );
                
                commands.spawn(ReconcileObject(PlayerObject { player: Player::new(position, Vec3::new(0.0,0.0,0.0)) }));
            }
        }

        player_info.player_inputs = encoded_input;

        connection.output_message.push(NetworkMessage(UDP::Input {
            keymask: encoded_input,
            player_id,
        }));
    }
}

pub fn reconcile_player(
    message_seq_num: SequenceNumber,
    server_players: &HashMap<Id, Player>,
    client_players: &mut Query<(&mut Transform, &mut AngularVelocity, &Id)>,
    player_info: &Res<PlayerInfo>,
    reconcile_buffer: &Res<ReconcileBuffer>,
) {
    let server_player = server_players.get(&player_info.current_player_id);

    let mut client_player = None;
    if let Some(reconcile_objects) = reconcile_buffer.buffer.get(&message_seq_num) {
        for r in reconcile_objects {
            match r.0 {
                PlayerObject { player } => {
                    client_player = Some(player);
                }
            }
        }
    }

    for (mut transform, mut angular_velo, id) in client_players.iter_mut() {
        if player_info.current_player_id == *id
            && server_player.is_some()
            && client_player.is_some()
        {
            let server_player = *server_player.unwrap();
            let client_player = client_player.unwrap();

            if server_player != client_player {
                transform.translation.x = server_player.position.x;
                transform.translation.y = server_player.position.y;
                transform.translation.z = server_player.position.z;
                angular_velo.x = server_player.angular_velocity.x;
                angular_velo.y = server_player.angular_velocity.y;
                angular_velo.z = server_player.angular_velocity.z;

                println!("sequence: {:?}", message_seq_num);
                println!("client: {:?}, server: {:?}", client_player, server_player);
                println!("Reconciled");
            }
        }
    }
}

// pub fn spawn_players(
//     mut commands: Commands,
//     mut meshes: ResMut<Assets<Mesh>>,
//     mut materials: ResMut<Assets<StandardMaterial>>,
//     mut net_message: ResMut<NetworkMessages>,
// ) {
//     let res = &mut net_message.udp_messages;
//     for m in res {
//         match &m.0 {
//             UDP::Spawn { player_uid } => {
//                 println!("Spawning player {:?}", player_uid);
//
//                 let mesh = Mesh::from(Sphere::default());
//                 for p in player_uid {
//                     commands.spawn((
//                         Mesh3d(meshes.add(mesh.clone())),
//                         MeshMaterial3d(materials.add(StandardMaterial::from(Color::WHITE))),
//                         Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(128.)),
//                         Id(p.0),
//                     ));
//                 }
//             }
//             _ => {}
//         }
//     }
// }

pub fn update_players(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    server_players: &HashMap<Id, Player>,
    client_players: &mut Query<(&mut Transform, &mut AngularVelocity, &Id)>,
    info: &Res<PlayerInfo>,
) {
    let mut existing_players = HashSet::new();
    for mut player in client_players.iter_mut() {
        existing_players.insert(player.2);

        let pos = match server_players.get(player.2) {
            Some(p) => p.position,
            None => continue,
        };

        if *player.2 != info.current_player_id {
            player.0.translation.x = pos.x;
            player.0.translation.y = pos.z;
        }
    }

    // Spawns players if they do not exist
    for p in server_players.iter() {
        if !existing_players.contains(p.0) {
            commands.spawn((
                RigidBody::Dynamic,
                Collider::sphere(1.0),
                AngularVelocity(Vector::new(0.0, 0.0, 0.0)),
                Mesh3d(meshes.add(Mesh::from(Sphere::default()))),
                MeshMaterial3d(materials.add(StandardMaterial::from(Color::WHITE))),
                Transform::from_xyz(0.0,10.0, 0.0).with_scale(bevy::math::Vec3::splat(1.0)),
                *p.0
            ));
        }
    }
}
