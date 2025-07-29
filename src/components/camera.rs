use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::{Camera3d, EventReader, Local, Quat, Query, Res, ResMut, Time, Transform, Vec2, Vec3, With, Without};
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

const LOOK_SENSITIVITY: (f32, f32) = (1.0, 1.0);
const CAM_SPACE: f32 = 10.0;

pub(crate) fn camera_controller(
    time: Res<Time>,
    mut camera: Query<&mut Transform, (With<Camera3d>, Without<PlayerMarker>)>,
    mut player: Query<(&Id,&Transform), (With<PlayerMarker>, Without<Camera3d>)>,
    mut input: EventReader<MouseMotion>,
    mut mouse_wheel: EventReader<MouseWheel>,
    player_info: Res<PlayerInfo>,
    mut camera_pos: Local<Vec2>,
    mut zoom: Local<f32>
) {
    for ev in mouse_wheel.read() {
        *zoom -= ev.y;
        *zoom = zoom.clamp(0.0, 10.0);
    }
    
    for player in player.iter_mut() {
        if *player.0 == player_info.current_player_id {
            for ev in input.read() {
                *camera_pos += Vec2::new(-1.0 * LOOK_SENSITIVITY.0, 1.0 * LOOK_SENSITIVITY.1) * ev.delta * time.delta_secs();

                camera_pos.y = camera_pos.y.clamp(-90.0f32.to_radians(), 90.0f32.to_radians());
            }

            for mut cam in camera.iter_mut() {
                cam.rotation = Quat::from_euler(YXZ, camera_pos.x, -camera_pos.y, 0.0);

                if CAM_SPACE == 0. {
                    cam.translation = player.1.translation + Vec3::new(0.0, 0.5, *zoom); // 0.0, 0.5, 2.0
                } else {
                    cam.translation = player.1.translation + cam.rotation * Vec3::new(0.0, 0.5, *zoom); // 0.0, 0.5, 2.0
                }
            }
        }
    }
}
