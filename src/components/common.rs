use bevy::prelude::Component;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Id(pub u128);

#[derive(
    Component, Encode, Decode, Serialize, Deserialize, Copy, Clone, Debug, Default, PartialEq,
)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}
