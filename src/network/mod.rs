use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use bevy::app::{App, Plugin};
use bevy::prelude::{Commands, FixedPostUpdate, FixedPreUpdate, IntoScheduleConfigs, PreStartup, Res};
use bevy_tokio_tasks::{TokioTasksPlugin, TokioTasksRuntime};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use crate::network::net_manage::{start_tcp_task, start_udp_task, Communication, TcpConnection, UdpConnection};
use crate::network::net_reconciliation::ReconcileBuffer;
use crate::network::net_system::{tcp_client_net_receive, tcp_client_net_send, udp_client_net_receive, udp_client_net_send};
use crate::network::net_tasks::{add_ping_message, handle_tcp_message, handle_udp_message};

pub mod net_manage;
pub mod net_message;
pub mod net_reconciliation;
pub mod net_system;
pub mod net_tasks;

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(TokioTasksPlugin::default())
            .insert_resource(UdpConnection::new(None))
            .insert_resource(TcpConnection::new(None))
            .insert_resource(ReconcileBuffer {
                buffer: HashMap::new(),
                sequence_counter: 0,
                miss_predict_counter: 0,
            })
            // .insert_resource(ReconcilePlayerState{
            //     player: Player::default()
            // })
            .add_systems(PreStartup, setup_communications)
            .add_systems(
                FixedPreUpdate,
                (
                    udp_client_net_receive,
                    tcp_client_net_receive,
                    handle_udp_message.after(udp_client_net_receive),
                    handle_tcp_message.after(tcp_client_net_receive),
                    add_ping_message.after(handle_udp_message),
                )
            )
            .add_systems(
                FixedPostUpdate,
                (
                    udp_client_net_send,
                    tcp_client_net_send,
                )
            );
    }
}

fn setup_communications(
    mut commands: Commands,
    runtime: Res<TokioTasksRuntime>
) {
    println!("Setting up communications...");
    let (udp_send_tx, udp_send_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);
    let (udp_receive_tx, udp_receive_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);
    let (tcp_send_tx, tcp_send_rx) = mpsc::channel::<(Vec<u8>, Arc<TcpStream>)>(1_000);
    let (tcp_receive_tx, tcp_receive_rx) = mpsc::channel::<(Vec<u8>, Arc<TcpStream>)>(1_000);
    
    runtime.spawn_background_task(|_| async move {
        println!("starting communication");
        
        let remote_addr = SocketAddr::from(([127, 0, 0, 1], 4444));
        
        start_tcp_task(remote_addr, tcp_send_rx, tcp_receive_tx).await.unwrap();
        start_udp_task(remote_addr, udp_send_rx, udp_receive_tx, 1).await.unwrap();
    });
    
    commands.insert_resource(
        Communication::new(
            udp_send_tx,
            udp_receive_rx,
            tcp_send_tx,
            tcp_receive_rx,
        )
    )
}