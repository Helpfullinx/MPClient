use crate::components::common::{Id, Position};
use crate::network::net_message::{NetworkMessage, UDP};
use crate::network::net_reconciliation::ReconcileType::Player;
use crate::network::net_reconciliation::{ReconcileBuffer, ReconcileObject, ReconcileType};
use crate::{Hud, NetworkMessages};
use bevy::asset::Assets;
use bevy::color::Color;
use bevy::color::palettes::basic::RED;
use bevy::input::ButtonInput;
use bevy::math::{Vec3, VectorSpace};
use bevy::pbr::StandardMaterial;
use bevy::prelude::{Bundle, Reflect, Resource};
use bevy::prelude::{
    Camera2d, Camera3d, ColorMaterial, Commands, GlobalTransform, JustifyText, KeyCode, Mesh,
    Mesh2d, Mesh3d, MeshMaterial2d, MeshMaterial3d, Query, Rectangle, ReflectResource, Res, ResMut,
    Sphere, Text, Text2d, TextLayout, Transform, With, Without,
};
use bevy::render::render_resource::TextureViewDimension::Cube;
use bevy_inspector_egui::egui::lerp;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ops::Add;

#[derive(Reflect, Resource, Default)]
#[reflect(Resource)]
pub struct PlayerInfo {
    pub current_player_id: u128,
    pub player_inputs: u8,
}

#[derive(Bundle, Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq)]
pub struct PlayerBundle {
    pub position: Position,
}

impl PlayerBundle {
    pub fn new(position: Position) -> Self {
        Self { position }
    }
}

pub fn snap_camera_to_player(
    player_info: ResMut<PlayerInfo>,
    players: Query<(&Transform, &Id), Without<Camera3d>>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
) {
    let mut cam = camera.single_mut().unwrap();
    for player in players.iter() {
        if player.1.0 == player_info.current_player_id {
            cam.translation.x = player.0.translation.x;
            cam.translation.y = player.0.translation.y;
        }
    }
}

pub fn player_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_info: ResMut<PlayerInfo>,
    mut players: Query<(&mut Transform, &Id)>,
    mut hud: Query<&mut Text, With<Hud>>,
    mut commands: Commands,
) {
    let mut encoded_input = 0u8;

    if keyboard_input.pressed(KeyCode::ArrowUp) {
        encoded_input |= 1;
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) {
        encoded_input |= 2;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) {
        encoded_input |= 4;
    }
    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        encoded_input |= 8;
    }

    let player_id = player_info.current_player_id;
    let move_speed = 0.1;

    for (mut transform, id) in players.iter_mut() {
        if player_id == id.0 {
            if encoded_input & 1 > 0 {
                transform.translation.y += move_speed;
            }
            if encoded_input & 2 > 0 {
                transform.translation.y -= move_speed;
            }
            if encoded_input & 4 > 0 {
                transform.translation.x += move_speed;
            }
            if encoded_input & 8 > 0 {
                transform.translation.x -= move_speed;
            }

            let mut h = hud.single_mut().unwrap();
            h.clear();
            h.push_str(&format!(
                "x: {:?}\ny: {:?}\n{:?}",
                transform.translation.x, transform.translation.y, player_id
            ));

            commands.spawn(ReconcileObject(Player {
                player_bundle: PlayerBundle::new(Position::new(
                    transform.translation.x,
                    transform.translation.y,
                )),
            }));
        }
    }

    player_info.player_inputs = encoded_input;

    commands.spawn(NetworkMessage(UDP::Input {
        keymask: encoded_input,
        player_id: player_id,
    }));
}

pub fn reconcile_player_position(
    net_messages: ResMut<NetworkMessages>,
    mut players: Query<(&mut Transform, &Id)>,
    player_info: ResMut<PlayerInfo>,
    reconcile_buffer: Res<ReconcileBuffer>,
) {
    let mut server_player = None;
    let mut seq_num = None;
    let mut count = 0;
    for m in &net_messages.udp_message {
        match &m.0 {
            UDP::Players { players } => {
                count += 1;
                server_player = players.get(&player_info.current_player_id);
            }
            UDP::Sequence { sequence_number } => seq_num = Some(sequence_number),
            _ => {}
        }
    }

    if seq_num.is_none() {
        return;
    };

    if count == 2 {
        println!("{:?}", net_messages);
    }

    let mut client_player = None;
    match reconcile_buffer.buffer.get(seq_num.unwrap()) {
        Some(reconcile_objects) => {
            for r in reconcile_objects {
                match r.0 {
                    Player { player_bundle } => {
                        client_player = Some(player_bundle);
                    }
                }
            }
        }
        None => {}
    }

    for (mut transform, id) in players.iter_mut() {
        if player_info.current_player_id == id.0
            && server_player.is_some()
            && client_player.is_some()
        {
            let server_pos = (*server_player.unwrap()).position;
            let client_pos = client_player.unwrap().position;

            if server_pos != client_pos {
                transform.translation.x = server_pos.x;
                transform.translation.y = server_pos.y;
                println!("sequence: {:?}", seq_num.unwrap());
                println!("client: {:?}, server: {:?}", client_player, server_player);
                println!("Reconciled");
            }
        }
    }
}

pub fn spawn_players(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut net_message: ResMut<NetworkMessages>,
) {
    let res = &mut net_message.udp_message;
    for m in res {
        match &m.0 {
            UDP::Spawn { player_uid } => {
                println!("Spawning player {:?}", player_uid);

                let mesh = Mesh::from(Sphere::default());
                for p in player_uid {
                    commands.spawn((
                        Mesh3d(meshes.add(mesh.clone())),
                        MeshMaterial3d(materials.add(StandardMaterial::from(Color::WHITE))),
                        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(128.)),
                        Id(p.0),
                    ));
                }
            }
            _ => {}
        }
    }
}

pub fn update_players(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut players: Query<(&mut Transform, &Id)>,
    info: Res<PlayerInfo>,
    mut net_message: ResMut<NetworkMessages>,
) {
    for m in net_message.udp_message.iter_mut() {
        match &m.0 {
            UDP::Players {
                players: updated_players,
            } => {
                let mut existing_players = HashSet::new();
                for mut player in players.iter_mut() {
                    existing_players.insert(player.1.0);

                    let pos = match updated_players.get(&player.1.0) {
                        Some(p) => p.position,
                        None => continue,
                    };

                    if player.1.0 != info.current_player_id {
                        player.0.translation.x = pos.x;
                        player.0.translation.y = pos.y;
                    }
                }

                // Spawns players if they do not exist
                let mesh = Mesh::from(Sphere::default());
                for p in updated_players.iter() {
                    if !existing_players.contains(p.0) {
                        let parent = commands
                            .spawn((
                                Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(1.0)),
                                GlobalTransform::default(),
                                Id(*p.0),
                            ))
                            .id();

                        // Spawn the mesh as a child and it will inherit the scaling
                        commands.entity(parent).with_children(|parent| {
                            parent.spawn((
                                Mesh3d(meshes.add(mesh.clone())),
                                MeshMaterial3d(materials.add(StandardMaterial::from(Color::WHITE))),
                                Transform::default().with_scale(Vec3::splat(1.0)),
                                GlobalTransform::default(),
                            ));

                            // Spawn the text as a sibling with no scale
                            parent.spawn((
                                Text2d::new(&*p.0.to_string()),
                                TextLayout::new_with_justify(JustifyText::Center),
                                Transform::from_xyz(0.0, 64.0, 0.0).with_scale(Vec3::splat(0.5)), // unscaled
                                GlobalTransform::default(),
                            ));
                        });
                    }
                }
            }
            _ => {}
        };
    }
}
