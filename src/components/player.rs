use crate::components::common::{Id, Vec3};
use crate::components::hud::Hud;
use crate::network::net_manage::UdpConnection;
use crate::network::net_message::{BitMask, NetworkMessage, SequenceNumber, UDP};
use crate::network::net_reconciliation::{ReconcileBuffer, ObjectState, StateType};
use bevy::asset::Assets;
use bevy::color::Color;
use bevy::input::ButtonInput;
use bevy::pbr::StandardMaterial;
use bevy::prelude::{Camera, Capsule3d, Command, Component, Cuboid, Entity, Event, EventReader, EventWriter, Gizmos, GlobalTransform, Node, QueryState, Reflect, Resource, Single, Time, Val, World};
use bevy::prelude::{
    Camera3d, Commands, KeyCode, Mesh, Mesh3d, MeshMaterial3d, Query, ReflectResource, Res, ResMut, Text,
    Text2d, TextLayout, Transform, With, Without,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::time::Duration;
use avian3d::prelude::{Collider, Friction, LinearVelocity, LockedAxes, Physics, PhysicsSchedule, Position, RigidBody, Rotation, Sleeping};
use bevy::color::palettes::css::PURPLE;
use bevy::ecs::system::SystemState;
use bevy::ui::PositionType;
use bevy::utils::default;
use crate::network::net_reconciliation::StateType::{InputState, PlayerState};

#[derive(Reflect, Resource, Default)]
#[reflect(Resource)]
pub struct PlayerInfo {
    pub current_player_id: Id,
    pub player_inputs: BitMask,
}

#[derive(Component)]
pub struct PlayerMarker;

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq)]
pub struct Player {
    pub position: Vec3,
    pub linear_velocity: Vec3
}

pub struct ResimulatePlayer {
    pub received_sequence_number: SequenceNumber,
    pub object_states: Vec<ObjectState>,
}

impl Player {
    pub fn new(position: Vec3, linear_velocity: Vec3) -> Self {
        Self {
            position,
            linear_velocity
        }
    }
}

impl Command for ResimulatePlayer {
    fn apply(self, world: &mut World) -> () {
        let pos_and_velo = {
            let mut reconcile_buffer = world.resource_mut::<ReconcileBuffer>();

            // Save frame state to buffer
            reconcile_buffer
                .buffer
                .insert(self.received_sequence_number, self.object_states);

            reconcile_buffer
                .buffer
                .get(&self.received_sequence_number)
                .and_then(|frame_state| {
                    frame_state.iter().find_map(|object_state| {
                        match object_state.0 {
                            PlayerState { player } => Some((player.position, player.linear_velocity)),
                            _ => None
                        }
                    })
                })
        };
        
        // Set transform to match historical frame state
        let mut player = world.query_filtered::<(&mut Position, &mut LinearVelocity), With<PlayerMarker>>();
        if let Some(mut p) = player.single_mut(world).ok() {
            if let Some(pv) = pos_and_velo {
                p.0.x = pv.0.x;
                p.0.y = pv.0.y;
                p.0.z = pv.0.z;
                p.1.x = pv.1.x;
                p.1.y = pv.1.y;
                p.1.z = pv.1.z;
            }
        }

        // println!("Resimulate length: {:?}..{:?}", ev.received_sequence_number, reconcile_buffer.sequence_counter);
        
        // Run physics calculations and input for each tick from received up to current tick
        for i in self.received_sequence_number.. {
            // Extract input for this tick
            let frame_input = {
                let reconcile_buffer = world.resource_mut::<ReconcileBuffer>();

                if i >= reconcile_buffer.sequence_counter {
                    break;
                }

                reconcile_buffer
                    .buffer
                    .get(&i)
                    .and_then(|frame_state| {
                        frame_state.iter().find_map(|object_state| match object_state.0 {
                            InputState { encoded_input } => Some(encoded_input),
                            _ => None,
                        })
                    })
            };

            // Apply input
            if let Some(fi) = frame_input {
                let mut player = world.query_filtered::<&mut LinearVelocity, With<PlayerMarker>>();
                if let Some(mut linear_velo) = player.single_mut(world).ok() {
                    if fi != 0 {
                        apply_player_input(fi, &mut linear_velo);
                    }
                }
            }

            // Run the physics schedule
            world.resource_mut::<Time<Physics>>().advance_by(Duration::from_secs_f64(1.0 / 60.0));
            world.run_schedule(PhysicsSchedule);


            let new_player_data = {
                let mut player = world.query_filtered::<(&Position, &LinearVelocity), With<PlayerMarker>>();

                player.single(world).ok().and_then(|p| Some((p.0.0, p.1.0)))
            };

            // Save updated player state
            let mut reconcile_buffer = world.resource_mut::<ReconcileBuffer>();
            if let Some(frame_state) = reconcile_buffer.buffer.get_mut(&i) {
                for object_state in frame_state.iter_mut() {
                    match &mut object_state.0 {
                        PlayerState { player } => {
                            if let Some(new_player_data) = new_player_data {
                                *player = Player::new(
                                    Vec3::new(new_player_data.0.x, new_player_data.0.y, new_player_data.0.z),
                                    Vec3::new(new_player_data.1.x, new_player_data.1.y, new_player_data.1.z),
                                )
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Set the character's position to the updated current position
        {
            let new_current_data = {
                let reconcile_buffer = world.resource::<ReconcileBuffer>();
                let index = {
                    if reconcile_buffer.sequence_counter != 0 {
                        reconcile_buffer.sequence_counter - 1
                    } else {
                        1023
                    }
                };

                reconcile_buffer
                    .buffer
                    .get(&(index))
                    .and_then(|frame_state| {
                        frame_state.iter().find_map(|object_state| {
                            match object_state.0 {
                                PlayerState { player: player_state } => {
                                    Some((player_state.position, player_state.linear_velocity))
                                }
                                _ => None
                            }
                        })
                    })
            };

            if let Some(new_current_data) = new_current_data {
                if let Some(mut p) = world.query_filtered::<(&mut Position, &mut LinearVelocity), With<PlayerMarker>>().single_mut(world).ok() {
                    p.0.x = new_current_data.0.x;
                    p.0.y = new_current_data.0.y;
                    p.0.z = new_current_data.0.z;
                    p.1.x = new_current_data.1.x;
                    p.1.y = new_current_data.1.y;
                    p.1.z = new_current_data.1.z;
                }
            }
        }
    }
}

pub fn set_player_id(
    player_info: &mut ResMut<PlayerInfo>,
    player_id: Id,
    reconcile_buffer: &mut ReconcileBuffer
) {
    player_info.current_player_id = player_id;
    reconcile_buffer.buffer.clear()
}

const MOVE_SPEED: f32 = 5.0;

fn apply_player_input(
    encoded_input: BitMask,
    linear_velocity: &mut LinearVelocity,
) {
    println!("linear velo {:?}", linear_velocity);

    let mut vector = bevy::math::Vec3::ZERO;
    
    if encoded_input & 1 > 0 {
        vector.x -= 1.0;
    }
    if encoded_input & 2 > 0 {
        vector.x += 1.0;
    }
    if encoded_input & 4 > 0 {
        vector.z -= 1.0;
    }
    if encoded_input & 8 > 0 {
        vector.z += 1.0;
    }
    if encoded_input & 16 > 0 {
        linear_velocity.y += 1.0;
    }
    
    let normalized_velocity = vector.normalize_or_zero();
    
    // println!("normalized_velocity: {:?}", normalized_velocity);
    
    linear_velocity.x = normalized_velocity.x * MOVE_SPEED;
    linear_velocity.z = normalized_velocity.z * MOVE_SPEED;
}

pub fn player_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_info: ResMut<PlayerInfo>,
    mut players: Query<(&Id, &Transform, &mut LinearVelocity), With<PlayerMarker>>,
    mut hud: Query<&mut Text, With<Hud>>,
    mut connection: ResMut<UdpConnection>,
    mut commands: Commands,
) {
    if connection.remote_socket.is_some() {
        let mut encoded_input: BitMask = 0u16;

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
        if keyboard_input.pressed(KeyCode::Space) { 
            encoded_input |= 16;
        }

        let player_id = player_info.current_player_id;

        for (id, transform, mut linear_velo) in players.iter_mut() {
            if player_id == *id {
                if encoded_input != 0 {
                    apply_player_input(encoded_input, &mut linear_velo);
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

                let lv = Vec3::new(
                    linear_velo.x,
                    linear_velo.y,
                    linear_velo.z,
                );
                
                commands.spawn(ObjectState(PlayerState { player: Player::new(position, lv) }));
                commands.spawn(ObjectState(InputState { encoded_input }));
            }
        }

        player_info.player_inputs = encoded_input;

        connection.add_message(NetworkMessage(UDP::Input {
            keymask: encoded_input,
            player_id,
        }));
    }
}

pub fn reconcile_player(
    commands: &mut Commands,
    gizmos: &mut Gizmos,
    message_seq_num: SequenceNumber,
    server_players: &HashMap<Id, Player>,
    client_players: &mut Query<(&mut Transform, &Id, Entity), With<PlayerMarker>>,
    player_info: &Res<PlayerInfo>,
    reconcile_buffer: &ReconcileBuffer,
) {
    let server_player_state = server_players.get(&player_info.current_player_id);

    let mut client_player_state = None;
    
    if let Some(reconcile_objects) = reconcile_buffer.buffer.get(&message_seq_num) {
        for r in reconcile_objects {
            match r.0 {
                PlayerState { player } => {
                    client_player_state = Some(player);
                },
                _ => {}
            }
        }


        for (_, id, _) in client_players.iter() {
            if player_info.current_player_id == *id
                && server_player_state.is_some()
                && client_player_state.is_some()
            {
                let sps = *server_player_state.unwrap();
                let cps = client_player_state.unwrap();

                gizmos.cuboid(
                    Transform::from_xyz(sps.position.x, sps.position.y, sps.position.z).with_scale(bevy::math::Vec3::splat(1.1)),
                    Color::WHITE
                );
                
                gizmos.cuboid(
                    Transform::from_xyz(cps.position.x, cps.position.y, cps.position.z),
                    PURPLE
                );

                if !sps.position.eq(&cps.position) {
                    // println!("sequence: {:?}", message_seq_num);
                    println!("client: {:?}, server: {:?}", cps, sps);
                    // println!("Reconciled");

                    let mut new_frame_state = reconcile_objects.clone();
                    for object_state in new_frame_state.iter_mut() {
                        match &mut object_state.0 {
                            PlayerState { player } => {
                                *player = Player::new(sps.position, sps.linear_velocity);
                            }
                            InputState { .. } => {}
                        }
                    }

                    commands.queue(ResimulatePlayer{received_sequence_number: message_seq_num, object_states: new_frame_state});
                }
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
    client_players: &mut Query<(&mut Transform, &Id, Entity), With<PlayerMarker>>,
    info: &Res<PlayerInfo>,
) {
    let mut existing_players = HashSet::new();
    
    for (mut transform, id, entity) in client_players.iter_mut() {
        existing_players.insert(id);

        let pos = match server_players.get(id) {
            Some(p) => p.position,
            None => continue,
        };

        if *id != info.current_player_id {
            commands.entity(entity).remove::<LinearVelocity>();
            commands.entity(entity).remove::<Collider>();
            commands.entity(entity).remove::<RigidBody>();
            commands.entity(entity).remove::<LockedAxes>();
            commands.entity(entity).remove::<Friction>();
            commands.entity(entity).remove::<Sleeping>();
            
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
            transform.translation.z = pos.z;
        }
    }

    // Spawns players if they do not exist
    for p in server_players.iter() {
        if !existing_players.contains(p.0) {
            println!("{:?}", p.1.position);
            let player = commands.spawn((
                RigidBody::Dynamic,
                Collider::capsule(0.5, 1.0),
                Friction::new(1.0),
                LockedAxes::new().lock_rotation_x().lock_rotation_y().lock_rotation_z(),
                Position::from_xyz(p.1.position.x, p.1.position.y, p.1.position.z),
                Mesh3d(meshes.add(Capsule3d::new(0.5, 1.0))),
                MeshMaterial3d(materials.add(StandardMaterial::from(Color::WHITE))),
                Transform::default().with_scale(bevy::math::Vec3::splat(1.0)),
                *p.0,
                PlayerMarker
            )).id();
            
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                PlayerLabel(player)
            )).with_children(|parent| {
                parent.spawn((
                    Text::new(p.0.0.to_string()),
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: Val::ZERO,
                        ..default()
                    },
                    TextLayout::default().with_no_wrap(),
                ));
            });
        }
    }
}

#[derive(Component)]
pub struct PlayerLabel(Entity);
pub fn update_label_pos(
    mut labels: Query<(&mut Node, &PlayerLabel)>,
    players: Query<&GlobalTransform>,
    camera3d: Query<(&mut Camera, &GlobalTransform), With<Camera3d>>,
) {
    for (mut node, label) in &mut labels {
        let world_position = players.get(label.0).unwrap().translation() + bevy::math::Vec3::Y;

        let (camera, camera_transform) = camera3d.single().unwrap();
        
        let viewport_position = match camera.world_to_viewport(camera_transform, world_position) {
            Ok(v) => v,
            Err(e) => { println!("{:?}", e); continue; },
        };

        node.top = Val::Px(viewport_position.y);
        node.left = Val::Px(viewport_position.x);
    }
}