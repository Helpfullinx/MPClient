use avian3d::prelude::PhysicsLayer;

pub mod chat;
pub mod common;
pub mod hud;
pub mod lobby;
pub mod player;
pub mod camera;
pub mod weapon;

#[derive(PhysicsLayer, Default, Debug, Copy, Clone)]
pub enum CollisionLayer {
    #[default]
    Ground,
    Player,
    Enemy,
}