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
pub enum CUdpType {
    Sequence {
        sequence_number: SequenceNumber,
    },
    Input {
        keymask: BitMask,
        mouse_delta: Vec2,
        player_id: Id,
    },
    Ping {
        intitiation_time: u32,
        last_rtt: u32,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SUdpType {
    Sequence {
        sequence_number: SequenceNumber,
    },
    Players {
        players: HashMap<Id, Player>,
    },
    Pong {
        initiation_time: u32,
        server_received_time: u32,
    }
}

impl NetworkMessageType for CUdpType {}
impl NetworkMessageType for SUdpType {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CTcpType {
    ChatMessage {
        player_id: Id,
        message: ChatMessage,
    },
    Join {
        lobby_id: Id,
    },
    
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum STcpType {
    PlayerId {
        player_uid: Id,
    },
    Chat {
        messages: Vec<(Id, ChatMessage)>
    },
}

impl NetworkMessageType for CTcpType {}
impl NetworkMessageType for STcpType {}
