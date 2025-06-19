use std::collections::HashSet;
use bevy::asset::Assets;
use bevy::color::Color;
use bevy::color::palettes::basic::RED;
use bevy::input::ButtonInput;
use bevy::math::{Vec3, VectorSpace};
use bevy::prelude::{ColorMaterial, Commands, GlobalTransform, JustifyText, KeyCode, Mesh, Mesh2d, MeshMaterial2d, Query, Rectangle, ReflectResource, Res, ResMut, Text2d, TextLayout, Transform};
use bevy::prelude::{Bundle, Reflect, Resource};
use serde::{Deserialize, Serialize};
use crate::components::common::{Id, Position};
use crate::network::net_message::{NetworkMessage, NetworkMessageType};
use crate::network::net_message::NetworkMessageType::Input;
use crate::NetworkMessages;

#[derive(Reflect, Resource, Default)]
#[reflect(Resource)]
pub struct PlayerInfo {
    pub current_player_id: u128,
    pub player_inputs: u8
}

#[derive(Bundle, Serialize, Deserialize, Debug, Default, Copy, Clone)]
pub struct PlayerBundle{
    pub position: Position,
}

impl PlayerBundle{
    pub fn new(position: Position) -> Self{
        Self {
            position,
        }
    }
}

pub fn player_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_info: ResMut<PlayerInfo>,
    mut players: Query<(&mut Transform, &Id)>,
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
    let move_speed = 3.0;

    for (mut transform, id) in players.iter_mut() {
        if player_id == id.0 {
            if encoded_input & 1 > 0 { transform.translation.y += move_speed; }
            if encoded_input & 2 > 0 { transform.translation.y -= move_speed; }
            if encoded_input & 4 > 0 { transform.translation.x += move_speed; }
            if encoded_input & 8 > 0 { transform.translation.x -= move_speed; }
        }
    }
    
    player_info.player_inputs = encoded_input;
    
    commands.spawn(NetworkMessage(Input { keymask: encoded_input, player_uid: player_id }));
}

pub fn spawn_players (
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut net_message: ResMut<NetworkMessages>,
) {

    let res = &mut net_message.0.1;
    for m in res {
        match &m.0 {
            NetworkMessageType::Spawn { player_uid} => {
                println!("Spawning player {:?}", player_uid);

                let mesh = Mesh::from(Rectangle::default());
                for p in player_uid {
                    commands.spawn(( Mesh2d(meshes.add(mesh.clone())),
                                     MeshMaterial2d(materials.add(Color::from(RED))),
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
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut players: Query<(&mut Transform, &Id)>,
    info: Res<PlayerInfo>,
    mut net_message: ResMut<NetworkMessages>,
) {
    for m in net_message.0.1.iter_mut() {
        match &m.0 {
            NetworkMessageType::Players { players: updated_players } => {

                let mut existing_players = HashSet::new();
                for mut player in players.iter_mut() {
                    existing_players.insert(player.1.0);

                    let pos = match updated_players.get(&player.1.0){
                        Some(p) => p.position,
                        None => continue,
                    };

                    if player.1.0 != info.current_player_id {
                        player.0.translation.x = VectorSpace::lerp(player.0.translation.x, pos.x, 0.1);
                        player.0.translation.y = VectorSpace::lerp(player.0.translation.y, pos.y, 0.1);
                    }
                }
                
                // Spawns players if they do not exist
                let mesh = Mesh::from(Rectangle::default());
                for p in updated_players.iter() {
                    if !existing_players.contains(p.0) {
                        let parent = commands.spawn((
                            Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(1.0)),
                            GlobalTransform::default(),
                            Id(*p.0),
                        )).id();

                        // Spawn the mesh as a child and it will inherit the scaling
                        commands.entity(parent).with_children(|parent| {
                            parent.spawn((
                                Mesh2d(meshes.add(mesh.clone())),
                                MeshMaterial2d(materials.add(Color::from(RED))),
                                Transform::default().with_scale(Vec3::splat(128.0)),
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