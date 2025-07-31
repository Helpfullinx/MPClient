use crate::components::chat::ChatMessage;
use crate::components::common::Id;
use crate::components::player::Player;
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use bevy::math::Vec2;

pub trait NetworkMessageType {}

#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct NetworkMessage<T: NetworkMessageType>(pub T);

pub type SequenceNumber = u16;
pub type BitMask = u16;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum UDP {
    Sequence {
        sequence_number: SequenceNumber,
    },
    Players {
        players: HashMap<Id, Player>,
    },
    Input {
        keymask: BitMask,
        mouse_delta: Vec2,
        player_id: Id,
    },
}

impl NetworkMessageType for UDP {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TCP {
    ChatMessage {
        player_id: Id,
        message: ChatMessage,
    },
    Chat {
        messages: Vec<(Id, ChatMessage)>
    },
    Join {
        lobby_id: Id,
    },
    PlayerId {
        player_uid: Id,
    },
}

impl NetworkMessageType for TCP {}
