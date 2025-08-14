use std::ops::Neg;
use avian3d::math::Quaternion;
use avian3d::prelude::{LayerMask, SpatialQueryFilter, SpatialQueryPipeline};
use bevy::color::palettes::css::{BLUE, YELLOW};
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseButtonInput;
use bevy::math::{Isometry3d, Vec3};
use bevy::prelude::{Camera3d, Commands, Component, Dir3, Entity, EulerRot, EventReader, Gizmo, Gizmos, KeyCode, MouseButton, Query, Res, Single, Transform, With};
use crate::components::camera::CameraInfo;
use crate::components::CollisionLayer;

#[derive(Component)]
pub struct Weapon {
    pub damage: u32,
    pub range: f32,
}

impl Weapon {
    pub fn fire(&self) {

    }
}

// pub fn weapon_equip (
//     mut commands: Commands,
//     mut key_input: EventReader<KeyboardInput>
// ) {
//     for key_in in key_input.read() {
//         match key_in.key_code {
//             KeyCode::Digit1 => {
//
//                 println!("Weapon equipped!");
//             }
//             _ => {}
//         }
//     }
// }

pub fn weapon_controller(
    weapon: Single<&mut Weapon>,
    spatial_query: Res<SpatialQueryPipeline>,
    mut mouse_input: EventReader<MouseButtonInput>,
    camera_transform: Single<&Transform, With<Camera3d>>,
    mut gizmos: Gizmos
) {
    for mouse_in in mouse_input.read() {
        if mouse_in.state == ButtonState::Pressed && mouse_in.button == MouseButton::Left {
            if let Some(hit) = spatial_query.cast_ray(camera_transform.translation, camera_transform.forward(), weapon.range, false, &SpatialQueryFilter::from_mask(!LayerMask::from(CollisionLayer::Player))) {
                println!("Hit: {:?}", hit);
                gizmos.sphere(Isometry3d::new(camera_transform.translation + (*camera_transform.forward() * hit.distance), Quaternion::default()), 0.25, YELLOW);
            }
        }
    }
}