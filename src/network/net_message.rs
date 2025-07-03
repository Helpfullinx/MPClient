use crate::components::common::{Id, Position};
use crate::components::entity::Entity;
use crate::components::player::PlayerBundle;
use bevy::ecs::entity;
use bevy::prelude::{Commands, Component, Query};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub trait NetworkMessageType {}

#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct NetworkMessage<T: NetworkMessageType>(pub T);

pub type SequenceNumber = u32;
pub type BitMask = u8;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum UDP {
    Sequence {
        sequence_number: SequenceNumber,
    },
    Spawn {
        player_uid: Vec<Id>,
    },
    Players {
        players: HashMap<u128, PlayerBundle>,
    },
    Entities {
        entities: Vec<(Entity, Position)>,
    },
    Input {
        keymask: BitMask,
        player_id: u128,
    },
}

impl NetworkMessageType for UDP {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TCP {
    TextMessage { message: String },
    Join { lobby_id: u128 },
    PlayerId { player_uid: u128 },
}

impl NetworkMessageType for TCP {}

#[inline]
pub fn build_udp_message(
    messages: &mut Query<(entity::Entity, &NetworkMessage<UDP>)>,
    commands: &mut Commands,
) -> Vec<NetworkMessage<UDP>> {
    Vec::from_iter(messages.iter_mut().map(|x| {
        commands.entity(x.0).despawn();
        x.1.clone()
    }))
}
