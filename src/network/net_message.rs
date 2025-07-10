use crate::components::chat::ChatMessage;
use crate::components::common::{Id, Position};
use crate::components::entity::Entity;
use crate::components::player::PlayerBundle;
use bevy::prelude::Component;
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
    ChatMessage {
        player_id: u128,
        message: ChatMessage,
    },
    Chat {
        messages: Vec<(u128, ChatMessage)>,
    },
    Join {
        lobby_id: u128,
    },
    PlayerId {
        player_uid: u128,
    },
}

impl NetworkMessageType for TCP {}
