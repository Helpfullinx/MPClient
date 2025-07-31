use approx::ulps_eq;
use bevy::prelude::{Component, Reflect};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Reflect, Component, Serialize, Deserialize, Clone, Copy, Debug, Hash, PartialEq, Default)]
#[derive(Eq)]
pub struct Id(pub u32);

#[derive(
    Component, Encode, Decode, Serialize, Deserialize, Copy, Clone, Debug, Default,
)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

impl PartialEq<Self> for Vec3 {
    fn eq(&self, other: &Self) -> bool {
        ulps_eq!(self.x, other.x, max_ulps = 1000) && ulps_eq!(self.y, other.y, max_ulps = 1000) && ulps_eq!(self.z, other.z, max_ulps = 1000)
    }
}