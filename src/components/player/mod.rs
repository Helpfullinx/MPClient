pub mod animation;
pub mod plugin;
mod input;

use crate::components::common::{Id, Vec3};
use crate::components::hud::Hud;
use crate::network::net_manage::UdpConnection;
use crate::network::net_message::{BitMask, NetworkMessage, SequenceNumber, CUdpType};
use crate::network::net_reconciliation::{ReconcileBuffer, ObjectState, MISS_PREDICT_LIMIT, BUFFER_SIZE};
use bevy::asset::{AssetServer, Assets};
use bevy::input::ButtonInput;
use bevy::prelude::{error, info, warn, AnimationGraph, AnimationGraphHandle, AnimationNodeIndex, AnimationPlayer, Camera, Capsule3d, ChildOf, Command, Component, Entity, EventReader, Gizmos, GlobalTransform, Handle, Local, Node, Reflect, Resource, Scene, SceneRoot, Time, Val, Vec2, World};
use bevy::prelude::{
    Camera3d, Commands, KeyCode, Mesh3d, MeshMaterial3d, Query, ReflectResource, Res, ResMut, Text, TextLayout, Transform, With,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::f32::consts::PI;
use std::time::Duration;
use avian3d::prelude::{Collider, CollisionLayers, Friction, LayerMask, LinearVelocity, LockedAxes, Physics, PhysicsSchedule, Position, RigidBody, Rotation, Sleeping};
use bevy::color::palettes::basic::{PURPLE, WHITE};
use bevy::gltf::GltfAssetLabel;
use bevy::input::mouse::{AccumulatedMouseMotion, MouseMotion};
use bevy::math::EulerRot::YXZ;
use bevy::math::Quat;
use bevy::text::{FontSmoothing, TextFont};
use bevy::ui::PositionType;
use bevy::utils::default;
use crate::components::camera::{apply_player_camera_input, CameraInfo};
use crate::components::CollisionLayer;
use crate::components::player::animation::{AnimationState, PlayerAnimationState};
use crate::DefaultFont;
use crate::network::net_reconciliation::StateType::{InputState, PlayerState};

#[derive(Reflect, Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum MovementState {
    Idle,
    Walking,
    Running,
    Jumping,
    Falling,
}

#[derive(Reflect, Resource, Default)]
#[reflect(Resource)]
pub struct PlayerInfo {
    pub current_player_id: Id,
    pub player_inputs: BitMask,
    pub mouse_delta: Vec2,
    pub accumulated_mouse_delta: Vec2,
    pub player_movement_state: HashSet<MovementState>,
}

#[derive(Component)]
pub struct PlayerMarker;

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq)]
pub struct Player {
    pub position: Vec3,
    pub linear_velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub animation_state: AnimationState
}

pub struct ResimulatePlayer {
    pub received_sequence_number: SequenceNumber,
    pub object_states: Vec<ObjectState>,
}

impl Player {
    pub fn new(position: Vec3, linear_velocity: Vec3, yaw: f32, pitch: f32, animation_state: AnimationState) -> Self {
        Self {
            position,
            linear_velocity,
            yaw,
            pitch,
            animation_state
        }
    }
}

impl ResimulatePlayer {
    fn rollback_player(&self, world: &mut World) {
        let rollback_player_state = {
            let mut reconcile_buffer = world.resource_mut::<ReconcileBuffer>();

            // Save frame state to buffer
            reconcile_buffer
                .buffer
                .insert(self.received_sequence_number, self.object_states.clone());
            
            self.object_states
                .iter()
                .find_map(|object_state| {
                        match object_state.0 {
                            PlayerState { player } => Some((player.position, player.linear_velocity, player.yaw, player.pitch)),
                            _ => None
                        }
                    }
                )
        };

        // Set transform to match historical frame state
        let mut player = world.query_filtered::<(&mut Position, &mut LinearVelocity, &mut CameraInfo), With<PlayerMarker>>();
        if let Some(mut p) = player.single_mut(world).ok() {
            if let Some(pv) = rollback_player_state {
                info!("Rollback: yaw {:?}, pitch {:?}", pv.2, pv.3);

                p.0.x = pv.0.x;
                p.0.y = pv.0.y;
                p.0.z = pv.0.z;
                p.1.x = pv.1.x;
                p.1.y = pv.1.y;
                p.1.z = pv.1.z;
                p.2.yaw = pv.2;
                p.2.pitch = pv.3;
            }
        }
    }

    fn resimulate_player(&self, world: &mut World) {
        for i in self.received_sequence_number + 1.. {
            // Extract input for this tick
            let frame_input = {
                let reconcile_buffer = world.resource_mut::<ReconcileBuffer>();

                if !reconcile_buffer.seq_is_newer(i) {
                    break;
                }

                reconcile_buffer
                    .buffer
                    .get(&i)
                    .and_then(|frame_state| {
                        frame_state.iter().find_map(|object_state| match object_state.0 {
                            InputState { encoded_input , mouse_delta} => {
                                info!("Input Found");
                                Some((encoded_input, mouse_delta))
                            },
                            _ => None,
                        })
                    })
            };

            if frame_input.is_none() {
                warn!("No input for frame {:?}", i);
            }

            // Apply input
            if let Some(fi) = frame_input {
                if let Some(mut player) = world
                    .query_filtered::<(&mut LinearVelocity, &mut Rotation, &mut CameraInfo), With<PlayerMarker>>()
                    .single_mut(world)
                    .ok()
                {
                    if fi.0 != 0 {
                        apply_player_movement_input(fi.0, &mut player.0, &mut player.1, &player.2.yaw);
                    }
                    apply_player_camera_input(fi.1, &mut player.2);

                    info!("Sequence {:?}: yaw {:?}, pitch {:?}, mouse_delta {:?}", i, player.2.yaw, player.2.pitch, fi.1);
                }
            }

            // Run the physics schedule
            world.resource_mut::<Time<Physics>>().advance_by(Duration::from_secs_f64(1.0 / 60.0));
            world.run_schedule(PhysicsSchedule);

            let new_player_info = {
                world
                    .query_filtered::<(&Position, &LinearVelocity, &CameraInfo, &PlayerAnimationState), With<PlayerMarker>>()
                    .single(world)
                    .ok()
                    .and_then(|p| Some((p.0.0, p.1.0, p.2.yaw, p.2.pitch, p.3.0)))
            };

            // Save updated player state
            let mut reconcile_buffer = world.resource_mut::<ReconcileBuffer>();

            let index = if i == BUFFER_SIZE - 1 {
                0
            } else {
                i + 1
            };
            
            let fs = reconcile_buffer.buffer.get_mut(&index);
            if fs.is_none() {
                info!("Couldn't find frame state for sequence {:?}", index);
            }

            if let Some(frame_state) = fs {
                for object_state in frame_state.iter_mut() {
                    match &mut object_state.0 {
                        PlayerState { player } => {
                            if let Some(p) = new_player_info {
                                info!("Set state {:?}: yaw {:?}, pitch {:?}", index, p.2, p.3);

                                *player = Player::new(
                                    Vec3::new(p.0.x, p.0.y, p.0.z),
                                    Vec3::new(p.1.x, p.1.y, p.1.z),
                                    p.2,
                                    p.3,
                                    p.4
                                )
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn set_updated_player_state(&self, world: &mut World) {
        let new_current_data = {
            let reconcile_buffer = world.resource::<ReconcileBuffer>();

            let index = if reconcile_buffer.sequence_counter == BUFFER_SIZE - 1 {
                0
            } else {
                reconcile_buffer.sequence_counter + 1
            };

            info!("Updated state sequence {:?}", index);

            reconcile_buffer
                .buffer
                .get(&(index))
                .and_then(|frame_state| {
                    frame_state.iter().find_map(|object_state| {
                        match object_state.0 {
                            PlayerState { player: player_state } => {
                                Some((player_state.position, player_state.linear_velocity, player_state.yaw, player_state.pitch))
                            }
                            _ => None
                        }
                    })
                })
        };

        if new_current_data.is_none() {
            error!("No updated player state found!");
        }

        if let Some(ncd) = new_current_data {
            if let Some(mut p) = world
                .query_filtered::<(&mut Position, &mut LinearVelocity, &mut CameraInfo), With<PlayerMarker>>()
                .single_mut(world)
                .ok()
            {
                info!("Updated state: yaw {:?}, pitch {:?}", ncd.2, ncd.3);
                p.0.x = ncd.0.x;
                p.0.y = ncd.0.y;
                p.0.z = ncd.0.z;
                p.1.x = ncd.1.x;
                p.1.y = ncd.1.y;
                p.1.z = ncd.1.z;
                p.2.yaw = ncd.2;
                p.2.pitch = ncd.3;
            }
        }
    }
}

impl Command for ResimulatePlayer {
    fn apply(self, world: &mut World) -> () {
        warn!("RESIMULATING");
        self.rollback_player(world);

        self.resimulate_player(world);

        self.set_updated_player_state(world);
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

const WALK_SPEED: f32 = 1.5;
const RUN_SPEED: f32 = 5.0;

fn apply_player_movement_input(
    encoded_input: BitMask,
    linear_velocity: &mut LinearVelocity,
    rotation: &mut Rotation,
    yaw: &f32,
) {
    let mut vector = bevy::math::Vec3::ZERO;

    if encoded_input & 1 > 0 {
        vector.z -= 1.0;
    }
    if encoded_input & 2 > 0 {
        vector.z += 1.0;
    }
    if encoded_input & 4 > 0 {
        vector.x += 1.0;
    }
    if encoded_input & 8 > 0 {
        vector.x -= 1.0;
    }
    if encoded_input & 16 > 0 {
        linear_velocity.y += 1.0;
    }

    let normalized_rotated_velocity = Quat::from_euler(YXZ, *yaw, 0.0, 0.0).mul_vec3(vector.normalize_or_zero());

    // println!("normalized_velocity: {:?}", normalized_velocity);

    linear_velocity.x = normalized_rotated_velocity.x * WALK_SPEED;
    linear_velocity.z = normalized_rotated_velocity.z * WALK_SPEED;
    rotation.0 = Quat::from_euler(YXZ, *yaw, 0.0, 0.0);
}

pub fn player_controller(
    mut player_info: ResMut<PlayerInfo>,
    mut players: Query<(&Id, &Transform, &mut LinearVelocity, &mut Rotation, &mut CameraInfo, &mut PlayerAnimationState), With<PlayerMarker>>,
    mut hud: Query<&mut Text, With<Hud>>,
    mut connection: ResMut<UdpConnection>,
    reconcile_buffer: Res<ReconcileBuffer>,
    mut commands: Commands,
) {
    if connection.remote_socket.is_some() {
        for (id, transform, mut linear_velo, mut rotation, mut camera_info, mut player_anim_state) in players.iter_mut() {
            if player_info.current_player_id == *id {
                if player_info.player_inputs != 0 {
                    player_anim_state.0 = AnimationState::Walking;
                    apply_player_movement_input(player_info.player_inputs, &mut linear_velo, &mut rotation, &camera_info.yaw);
                } else {
                    player_anim_state.0 = AnimationState::Idle;
                }

                if let Some(mut h) = hud.single_mut().ok() {
                    h.clear();
                    h.push_str(&format!(
                        "x: {:?}\ny: {:?}\nz: {:?}\nping: {:?}\n{:?}",
                        transform.translation.x, transform.translation.y, transform.translation.z, connection.ping, player_info.current_player_id
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
                
                commands.spawn(ObjectState(PlayerState { player: Player::new(position, lv, camera_info.yaw, camera_info.pitch, player_anim_state.0) }));
                commands.spawn(ObjectState(InputState { encoded_input: player_info.player_inputs, mouse_delta: player_info.accumulated_mouse_delta - player_info.mouse_delta }));
            }
        }

        connection.add_message(NetworkMessage(CUdpType::Input {
            keymask: player_info.player_inputs,
            mouse_delta: player_info.accumulated_mouse_delta - player_info.mouse_delta,
            player_id: player_info.current_player_id,
        }));

        player_info.accumulated_mouse_delta = Vec2::ZERO;
    }
}

pub fn reconcile_player(
    commands: &mut Commands,
    gizmos: &mut Gizmos,
    message_seq_num: SequenceNumber,
    server_players: &HashMap<Id, Player>,
    client_players: &mut Query<(&mut Transform, &Id, Entity, &CameraInfo, &mut PlayerAnimationState), With<PlayerMarker>>,
    player_info: &Res<PlayerInfo>,
    reconcile_buffer: &mut ReconcileBuffer,
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
        
        for (t, id, _, _, _) in client_players.iter() {
            if player_info.current_player_id == *id
                && server_player_state.is_some()
                && client_player_state.is_some()
            {
                let sps = *server_player_state.unwrap();
                let cps = client_player_state.unwrap();

                gizmos.cuboid(
                    Transform::from_xyz(sps.position.x, sps.position.y, sps.position.z)
                        .with_scale(bevy::math::Vec3::splat(1.1))
                        .with_rotation(Quat::from_euler(YXZ, sps.yaw,0.0,0.0)),
                    WHITE
                );
                
                gizmos.cuboid(
                    Transform::from_xyz(cps.position.x, cps.position.y, cps.position.z).with_rotation(Quat::from_euler(YXZ, cps.yaw,0.0,0.0)),
                    PURPLE
                );

                if !sps.eq(&cps) {
                    if reconcile_buffer.miss_predict_counter >= MISS_PREDICT_LIMIT - 1 {
                        warn!("RECONCILED");
                        info!("current sequence: {:?}, recieved sequence: {:?}", reconcile_buffer.sequence_counter, message_seq_num);
                        info!("client: {:?}, server: {:?}", (cps.yaw, cps.pitch), (sps.yaw, sps.pitch));
                        

                        let mut new_frame_state = reconcile_objects.clone();
                        for object_state in new_frame_state.iter_mut() {
                            match &mut object_state.0 {
                                PlayerState { player } => {
                                    *player = Player::new(sps.position, sps.linear_velocity, sps.yaw, sps.pitch, sps.animation_state);
                                }
                                _ => {}
                            }
                        }

                        commands.queue(ResimulatePlayer{ received_sequence_number: message_seq_num, object_states: new_frame_state });
                        reconcile_buffer.miss_predict_counter = 0;
                    } else {
                        reconcile_buffer.miss_predict_counter += 1;
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
    default_font: &Res<DefaultFont>,
    asset_server: &Res<AssetServer>,
    animation_graphs: &mut Assets<AnimationGraph>,
    // meshes: &mut ResMut<Assets<Mesh>>,
    // materials: &mut ResMut<Assets<StandardMaterial>>,
    server_players: &HashMap<Id, Player>,
    client_players: &mut Query<(&mut Transform, &Id, Entity, &CameraInfo, &mut PlayerAnimationState), With<PlayerMarker>>,
    info: &Res<PlayerInfo>,
) {
    let mut existing_players = HashSet::new();

    for (mut transform, id, entity, _, mut anim_state) in client_players.iter_mut() {
        existing_players.insert(id);

        let player = match server_players.get(id) {
            Some(p) => p,
            None => continue,
        };

        if *id != info.current_player_id {
            commands.entity(entity).remove::<LinearVelocity>();
            commands.entity(entity).remove::<RigidBody>();
            commands.entity(entity).remove::<LockedAxes>();
            commands.entity(entity).remove::<Friction>();
            commands.entity(entity).remove::<Sleeping>();
            commands.entity(entity).remove::<CollisionLayers>();

            commands.entity(entity).insert(CollisionLayers::new(CollisionLayer::Enemy, [LayerMask::ALL]));

            transform.translation.x = player.position.x;
            transform.translation.y = player.position.y;
            transform.translation.z = player.position.z;
            transform.rotation = Quat::from_euler(YXZ, player.yaw, 0.0, 0.0);
            anim_state.0 = player.animation_state;
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
                CollisionLayers::new(CollisionLayer::Player, [LayerMask::ALL]),
                // Mesh3d(meshes.add(Capsule3d::new(0.5, 1.0))),
                // MeshMaterial3d(materials.add(StandardMaterial::from(Color::WHITE))),
                Transform::default().with_scale(bevy::math::Vec3::splat(1.0)),
                CameraInfo{ yaw: p.1.yaw, pitch: p.1.pitch },
                PlayerAnimationState(AnimationState::Idle),
                *p.0,
                PlayerMarker
            )).with_children( |parent| {
                parent.spawn((
                    SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("meshes\\player.glb"))),
                    Transform::from_xyz(0.0, -1.0, 0.0).with_rotation(Quat::from_euler(YXZ, PI, 0.0, 0.0)),
                ));
            }).id();

            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                PlayerLabel(player)
            )).with_children(|parent| {
                parent.spawn((
                    Text::new(p.0.0.to_string()),
                    TextFont{
                        font: default_font.0.clone(),
                        font_size: 20.0,
                        line_height: Default::default(),
                        font_smoothing: FontSmoothing::None,
                    },
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
    mut labels: Query<(Entity, &mut Node, &PlayerLabel)>,
    players: Query<&GlobalTransform>,
    camera3d: Query<(&mut Camera, &GlobalTransform), With<Camera3d>>,
    mut commands: Commands
) {
    for (entity, mut node, label) in &mut labels {
        if let Some(world_position) = players.get(label.0).ok() {
            let pos = world_position.translation() + bevy::math::Vec3::Y;
            
            let (camera, camera_transform) = camera3d.single().unwrap();

            let viewport_position = match camera.world_to_viewport(camera_transform, pos) {
                Ok(v) => v,
                Err(e) => { /*println!("{:?}", e);*/ continue; },
            };

            node.top = Val::Px(viewport_position.y);
            node.left = Val::Px(viewport_position.x);
        } else {
            commands.entity(entity).despawn();
        }
    }
}