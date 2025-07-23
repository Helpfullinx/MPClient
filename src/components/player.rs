use crate::components::common::{Id, Vec3};
use crate::components::hud::Hud;
use crate::network::net_manage::UdpConnection;
use crate::network::net_message::{BitMask, NetworkMessage, SequenceNumber, UDP};
use crate::network::net_reconciliation::{ReconcileBuffer, ObjectState, StateType};
use bevy::asset::Assets;
use bevy::color::Color;
use bevy::input::ButtonInput;
use bevy::pbr::StandardMaterial;
use bevy::prelude::{Component, Cuboid, Entity, Event, EventReader, EventWriter, Gizmos, QueryState, Reflect, Resource, Time, World};
use bevy::prelude::{
    Camera3d, Commands, KeyCode, Mesh, Mesh3d, MeshMaterial3d, Query, ReflectResource, Res, ResMut, Text,
    Text2d, TextLayout, Transform, With, Without,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::time::Duration;
use avian3d::prelude::{Collider, LinearVelocity, LockedAxes, Physics, PhysicsSchedule, RigidBody};
use bevy::color::palettes::css::PURPLE;
use bevy::ecs::system::SystemState;
use crate::network::net_reconciliation::StateType::{InputState, PlayerState};

#[derive(Event)]
#[derive(Clone)]
pub struct ResimulateEvent {
    pub received_sequence_number: SequenceNumber,
    pub object_states: Vec<ObjectState>,
}

#[derive(Reflect, Resource, Default)]
#[reflect(Resource)]
pub struct PlayerInfo {
    pub current_player_id: Id,
    pub player_inputs: u8,
}

#[derive(Component, Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq)]
pub struct Player {
    pub position: Vec3,
    pub linear_velocity: Vec3
}

impl Player {
    pub fn new(position: Vec3, linear_velocity: Vec3) -> Self {
        Self {
            position,
            linear_velocity
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

pub fn snap_camera_to_player(
    player_info: ResMut<PlayerInfo>,
    players: Query<(&Transform, &Id), Without<Camera3d>>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
) {
    let mut cam = camera.single_mut().unwrap();
    for player in players.iter() {
        if *player.1 == player_info.current_player_id {
            cam.translation.x = player.0.translation.x + 10.0;
            cam.translation.y = player.0.translation.y + 10.0;
            cam.translation.z = player.0.translation.z + 10.0;
        }
    }
}

fn apply_player_input(
    encoded_input: BitMask,
    transform: &mut Transform,
) {
    let move_speed = 0.1;

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
}

pub fn player_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_info: ResMut<PlayerInfo>,
    mut players: Query<(&mut Transform, &Id, &LinearVelocity)>,
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

        for (mut transform, id, linear_velo) in players.iter_mut() {
            if player_id == *id {
                apply_player_input(encoded_input, &mut transform);

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

        connection.output_message.push(NetworkMessage(UDP::Input {
            keymask: encoded_input,
            player_id,
        }));
    }
}

pub fn reconcile_player(
    event_writer: &mut EventWriter<ResimulateEvent>,
    gizmos: &mut Gizmos,
    message_seq_num: SequenceNumber,
    server_players: &HashMap<Id, Player>,
    client_players: &mut Query<(&mut Transform, &mut LinearVelocity, &Id, Entity)>,
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


        for client_player in client_players.iter() {
            if player_info.current_player_id == *client_player.2
                && server_player_state.is_some()
                && client_player_state.is_some()
            {
                let sps = *server_player_state.unwrap();
                let cps = client_player_state.unwrap();

                // gizmos.cuboid(
                //     Transform::from_xyz(sps.position.x, sps.position.y, sps.position.z).with_scale(bevy::math::Vec3::splat(1.5)),
                //     Color::WHITE
                // );
                //
                // gizmos.cuboid(
                //     Transform::from_xyz(cps.position.x, cps.position.y, cps.position.z),
                //     PURPLE
                // );

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

                    event_writer.write(ResimulateEvent { received_sequence_number: message_seq_num, object_states: new_frame_state });
                }
            }
        }
    }
}

//get player
//initialize player trans

//recalc input
//physics
//step

// pub fn resimulate_player(
//     world: &mut World,
//     params: &mut SystemState<(
//         EventReader<ResimulateEvent>,
//         Query<(&mut Transform, &Id, &mut LinearVelocity)>,
//         Res<PlayerInfo>,
//         ResMut<ReconcileBuffer>,
//         ResMut<Time<Physics>>,
//     )>
// ) {
//     {
//         let (
//             mut events,
//             mut players,
//             player_info,
//             mut reconcile_buffer,
//             mut physics
//         ) = params.get_mut(world);
//
//         for ev in events.read() {
//             println!("Resimulating");
//
//             let mut client_player: Vec<_> = players.iter_mut().filter(|(_, id, _)| { player_info.current_player_id == **id }).collect();
//
//             reconcile_buffer.buffer.insert(ev.received_sequence_number, ev.object_states.clone());
//
//             println!("Client players: {:?}", client_player.len());
//             if let Some(p) = client_player.get_mut(0) {
//                 if let Some(frame_state) = reconcile_buffer.buffer.get_mut(&ev.received_sequence_number) {
//                     for object_state in frame_state.iter() {
//                         match object_state.0 {
//                             PlayerState {player} => {
//                                 p.0.translation.x = player.position.x;
//                                 p.0.translation.y = player.position.y;
//                                 p.0.translation.z = player.position.z;
//                             }
//                             _ => {}
//                         }
//                     }
//                 }
//             }
//
//             for i in ev.received_sequence_number+1..reconcile_buffer.sequence_counter {
//                 if let Some(frame_state) = reconcile_buffer.buffer.get_mut(&i) {
//                     let mut frame_input = None;
//                     for object_state in frame_state.iter() {
//                         match object_state.0 {
//                             InputState { encoded_input } => {
//                                 frame_input = Some(encoded_input);
//                             }
//                             _ => {}
//                         }
//                     }
//
//                     if let Some(fi) = frame_input {
//                         for p in client_player.iter_mut() {
//                             apply_player_input(fi, &mut p.0);
//                         }
//                     }
//
//                     physics.advance_by(Duration::from_secs_f64(1.0 / 60.0));
//
//                     for object_state in frame_state.iter_mut() {
//                         match &mut object_state.0 {
//                             PlayerState { player } => {
//                                 if let Some(p) = client_player.get(0) {
//                                     *player = Player::new(Vec3::new(p.0.translation.x,p.0.translation.y, p.0.translation.z), Vec3::new(p.2.x,p.2.y,p.2.z));
//                                 }
//                             }
//                             _ => {}
//                         }
//                     }
//                 }
//
//                 world.
//
//                 world.run_schedule(PhysicsSchedule);
//             }
//         }
//     }
// }


pub fn resimulate_player(
    world: &mut World,
    params: &mut SystemState<(
        EventReader<ResimulateEvent>,
        Query<(&mut Transform, &Id, &mut LinearVelocity)>,
        Res<PlayerInfo>,
        ResMut<ReconcileBuffer>,
        ResMut<Time<Physics>>,
    )>,
) {
    // Phase 1: Read and cache all events so we can drop the borrow early
    let event_data: Vec<ResimulateEvent> = {
        let (mut events, _, _, _, _) = params.get_mut(world);
        events.read().cloned().collect()
    };

    if !event_data.is_empty() {
        println!("Resimulating");
        println!("{:?}", event_data.len());
    }

    for ev in event_data {
        // Phase 2: Get what you need for the current event
        {
            let (_, mut players, player_info, mut reconcile_buffer, _) = params.get_mut(world);

            // Save frame state to buffer
            reconcile_buffer
                .buffer
                .insert(ev.received_sequence_number, ev.object_states);

            // Get client player
            let mut client_player: Vec<_> = players
                .iter_mut()
                .filter(|(_, id, _)| player_info.current_player_id == **id)
                .collect();

            // Set transform to match historical frame state
            if let Some(p) = client_player.get_mut(0) {
                if let Some(frame_state) =
                    reconcile_buffer.buffer.get_mut(&ev.received_sequence_number)
                {
                    for object_state in frame_state.iter() {
                        match object_state.0 {
                            PlayerState { player } => {
                                p.0.translation.x = player.position.x;
                                p.0.translation.y = player.position.y;
                                p.0.translation.z = player.position.z;
                                p.2.x = player.linear_velocity.x;
                                p.2.y = player.linear_velocity.y;
                                p.2.z = player.linear_velocity.z;
                            }
                            _ => {}
                        }
                    }
                }
            }
        } // <-- All borrows on `world` are dropped here automatically

        let sequence_counter = {
            let (_, _, _, reconcile_buffer, _) = params.get_mut(world);
            println!("Resimulate length: {:?}..{:?}", ev.received_sequence_number, reconcile_buffer.sequence_counter);
            reconcile_buffer.sequence_counter
        };

        // Phase 3: Resimulate and run physics for each tick
        for i in ev.received_sequence_number.. {
            if i >= sequence_counter {
                break;
            }

            // Extract input for this tick
            let frame_input = {
                let (_, _, _, reconcile_buffer, _) = params.get_mut(world);
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
                let (_, mut players, player_info, _, _) = params.get_mut(world);
                for mut p in players.iter_mut() {
                    if player_info.current_player_id == *p.1 {
                        apply_player_input(fi, &mut p.0);
                    }
                }
            }

            // Advance physics tick
            let (_, _, _, _, mut physics) = params.get_mut(world);
            physics.advance_by(Duration::from_secs_f64(1.0 / 60.0));


            // Run the physics schedule
            world.run_schedule(PhysicsSchedule);

            // Save updated player state
            {
                let (_, players, player_info, mut reconcile_buffer, _) = params.get_mut(world);
                let client_player: Vec<_> = players
                    .iter()
                    .filter(|(_, id, _)| player_info.current_player_id == **id)
                    .collect();

                if let Some(p) = client_player.get(0) {
                    if let Some(frame_state) = reconcile_buffer.buffer.get_mut(&i) {
                        for object_state in frame_state.iter_mut() {
                            match &mut object_state.0 {
                                PlayerState { player } => {
                                    let new_player_state = Player::new(
                                        Vec3::new(p.0.translation.x, p.0.translation.y, p.0.translation.z),
                                        Vec3::new(p.2.x, p.2.y, p.2.z),
                                    );

                                    println!("{:?} - New player state: {:?}", i, new_player_state);

                                    *player = new_player_state;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        {
            let (_, mut players, player_info,reconcile_buffer,_) = params.get_mut(world);

            let index;
            if reconcile_buffer.sequence_counter != 0 {
                index = reconcile_buffer.sequence_counter - 1;
            } else {
                index = 1023;
            }

            if let Some(state) = reconcile_buffer.buffer.get(&(index)) {
                for object_state in state.iter() {
                    match object_state.0 {
                        PlayerState { player: player_state } => {
                            for mut p in players.iter_mut() {
                                if *p.1 == player_info.current_player_id {
                                    println!("{:?} - {:?}", sequence_counter, player_state);
                                    p.0.translation.x = player_state.position.x;
                                    p.0.translation.y = player_state.position.y;
                                    p.0.translation.z = player_state.position.z;
                                    p.2.x = player_state.linear_velocity.x;
                                    p.2.y = player_state.linear_velocity.y;
                                    p.2.z = player_state.linear_velocity.z;
                                }
                            }
                        }
                        _ => {}
                    }
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
    client_players: &mut Query<(&mut Transform, &mut LinearVelocity, &Id, Entity)>,
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
            player.0.translation.y = pos.y;
            player.0.translation.z = pos.z;
        }

        // if *player.2 != info.current_player_id {
        //     commands.entity(player.3).remove::<LinearVelocity>();
        //     commands.entity(player.3).remove::<Collider>();
        //     commands.entity(player.3).remove::<RigidBody>();
        //     commands.entity(player.3).remove::<LockedAxes>();
        // }
    }

    // Spawns players if they do not exist
    for p in server_players.iter() {
        if !existing_players.contains(p.0) {
            commands.spawn((
                RigidBody::Dynamic,
                Collider::cuboid(1.0,1.0,1.0),
                LinearVelocity::default(),
                LockedAxes::new().lock_rotation_x().lock_rotation_y().lock_rotation_z(),
                Mesh3d(meshes.add(Mesh::from(Cuboid::new(1.0,1.0,1.0)))),
                MeshMaterial3d(materials.add(StandardMaterial::from(Color::WHITE))),
                Transform::from_xyz(p.1.position.x, p.1.position.y, p.1.position.z).with_scale(bevy::math::Vec3::splat(1.0)),
                *p.0
            ));
        }
    }
}