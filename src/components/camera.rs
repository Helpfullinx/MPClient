use avian3d::prelude::{Position, Rotation};
use bevy::input::mouse::{accumulate_mouse_motion_system, MouseMotion, MouseWheel};
use bevy::prelude::{Camera3d, Changed, Component, EventReader, Fixed, Local, Quat, Query, Res, ResMut, Resource, Single, Time, Transform, Vec2, Vec3, With, Without};
use bevy::prelude::EulerRot::YXZ;
use crate::components::common::Id;
use crate::components::player::{PlayerInfo, PlayerMarker};

// pub fn camera_controller(
//     player_info: ResMut<PlayerInfo>,
//     players: Query<(&Transform, &Id), Without<Camera3d>>,
//     mut camera: Query<&mut Transform, With<Camera3d>>,
//     mut mouse_event: EventReader<MouseMotion>,
// ) {
//     let mut cam = camera.single_mut().unwrap();
//     for player in players.iter() {
//         if *player.1 == player_info.current_player_id {
//             cam.translation = player.0.translation + 10.0;
//         }
//     }
//
//     // for ev in mouse_event.read() {
//     //     println!("{:?}", ev.delta);
//     // }
// }

#[derive(Component, Debug)]
pub struct CameraInfo {
    pub yaw: f32,
    pub pitch: f32,
}

pub fn apply_player_camera_input (
    mouse_delta: Vec2,
    camera_info: &mut CameraInfo,
) {
    camera_info.yaw += -1.0 * LOOK_SENSITIVITY.0 * mouse_delta.x * 0.005;
    camera_info.pitch += 1.0 * LOOK_SENSITIVITY.1 * mouse_delta.y * 0.005;

    camera_info.pitch = camera_info.pitch.clamp(-90.0f32.to_radians(), 90.0f32.to_radians());
}

const LOOK_SENSITIVITY: (f32, f32) = (1.0, 1.0);
const CAM_SPACE: f32 = 10.0;

pub(crate) fn camera_controller(
    mut camera: Query<&mut Transform, (With<Camera3d>, Without<PlayerMarker>)>,
    player: Query<(&Id, &Position, &CameraInfo), (With<PlayerMarker>, Without<Camera3d>)>,
    mut mouse_wheel: EventReader<MouseWheel>,
    player_info: Res<PlayerInfo>,
    mut zoom: Local<f32>
) {
    for ev in mouse_wheel.read() {
        *zoom -= ev.y;
        *zoom = zoom.clamp(0.0, 10.0);
    }
    
    for player in player.iter() {
        if *player.0 == player_info.current_player_id {
            
            // println!("camera info: {:?}", player.2);
            
            for mut cam in camera.iter_mut() {
                cam.rotation = Quat::from_euler(YXZ, player.2.yaw, -player.2.pitch, 0.0);

                if CAM_SPACE == 0. {
                    cam.translation = player.1.0 + Vec3::new(0.0, 0.0, *zoom); // 0.0, 0.5, 2.0
                } else {
                    cam.translation = player.1.0 + cam.rotation * Vec3::new(0.0, 0.0, *zoom); // 0.0, 0.5, 2.0
                }
            }
        }
    }
}
