use crate::components::player::PlayerInfo;
use crate::network::net_manage::TcpConnection;
use crate::network::net_message::{NetworkMessage, TCP};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::{ButtonInput, ButtonState};
use bevy::prelude::{Component, EventReader, KeyCode, Local, Query, Res, ResMut, Text, With};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Component)]
pub struct Chat {
    pub chat_history: VecDeque<(u128, ChatMessage)>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub message: String,
}

pub fn chat_window(
    player_info: Res<PlayerInfo>,
    mut connection: ResMut<TcpConnection>,
    mut keyboard_input: EventReader<KeyboardInput>,
    mut message_buffer: Local<String>,
    mut is_active: Local<bool>,
    mut chat: Query<(&mut Text, &mut Chat), With<Chat>>,
) {
    for k in keyboard_input.read() {
        if k.state == ButtonState::Released {
            continue;
        }

        if *is_active {
            match &k.logical_key {
                Key::Backspace => {
                    message_buffer.pop();
                }
                Key::Enter => {
                    if connection.stream.is_some() {
                        connection
                            .output_message
                            .push(NetworkMessage(TCP::ChatMessage {
                                player_id: player_info.current_player_id,
                                message: ChatMessage {
                                    message: message_buffer.clone(),
                                },
                            }))
                    }
                    message_buffer.clear()
                }
                Key::Character(c) => {
                    message_buffer.push_str(c);
                }
                Key::Space => {
                    message_buffer.push_str(" ");
                }
                _ => {}
            }
        }

        match k.key_code {
            KeyCode::KeyT => {
                *is_active = true;
            }
            KeyCode::Escape => {
                *is_active = false;
            }
            _ => {}
        }

        println!("{:?}", message_buffer);
    }

    // Updates chat window
    if let Some(mut chat) = chat.single_mut().ok() {
        chat.0.0.clear();
        for c in chat.1.chat_history.iter_mut() {
            chat.0.0.push_str(&format!(
                "{:?}: {:?}\n",
                c.0.to_string(),
                c.1.message.to_string()
            ));
        }
        if *is_active {
            chat.0
                .0
                .push_str(&format!("{:?}\n", message_buffer.to_string()));
        }
    }
}

const CHAT_HISTORY_LEN: usize = 10;

pub fn add_chat_message(messages: &mut Vec<(u128, ChatMessage)>, chat: &mut Query<&mut Chat>) {
    if let Some(mut chat) = chat.single_mut().ok() {
        chat.chat_history.clear();
        while !messages.is_empty() {
            if chat.chat_history.len() > CHAT_HISTORY_LEN {
                chat.chat_history.pop_back();
            }
            let message = messages.pop().unwrap();
            chat.chat_history.push_front(message);
        }
    }
}
