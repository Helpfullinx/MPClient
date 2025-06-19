use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct Entity{
    pub id: u8,
}

impl Entity{
    pub fn new(id: u8) -> Entity{
        Self {
            id
        }
    }
}