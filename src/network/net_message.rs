use std::collections::HashMap;
use std::net::SocketAddr;
use bevy::ecs::entity;
use bevy::prelude::{Commands, Component, Query, Res, Resource};
use bincode::config;
use serde::{Deserialize, Serialize};
use crate::components::common::{Id, Position};
use crate::components::entity::Entity;
use crate::components::player::{PlayerInfo, PlayerBundle};
use crate::network::net_manage::Communication;
use crate::network::net_message::NetworkMessageType::Input;
// #[derive(Component, Serialize, Deserialize, Clone)]
// pub struct NetworkMessage(pub NetworkMessageType);

#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct NetworkMessage(pub NetworkMessageType);

type BitMask = u8;
pub type SequenceNumber = u32;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NetworkMessageType {
    Sequence { sequence_number: u32 },
    Spawn { player_uid: Vec<Id> },
    Players { players: HashMap<u128, PlayerBundle> },
    Entities { entities: Vec<(Entity, Position)> },
    Input { keymask: BitMask, player_uid: u128 },
    Join { lobby_id: u128 },
    PlayerId { player_uid: u128 },
}

pub fn build_udp_message(
    messages: &mut Query<(entity::Entity, &NetworkMessage)>,
    commands: &mut Commands
) -> Vec<NetworkMessage> {
    let mut net_message = Vec::new();
    for n in messages.iter_mut() {
        net_message.push(n.1.clone());
        commands.entity(n.0).despawn();
    }

    net_message
}