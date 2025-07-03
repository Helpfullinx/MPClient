use crate::network::net_message::{NetworkMessageType, TCP};
use bevy::prelude::{Component, Resource};
use bincode::config;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io;
use tokio::io::Interest;
use tokio::net::{TcpSocket, TcpStream, UdpSocket};
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Resource)]
pub struct Communication {
    pub udp_tx: Sender<(Vec<u8>, SocketAddr)>,
    pub udp_rx: Receiver<(Vec<u8>, SocketAddr)>,
    pub tcp_tx: Sender<(Vec<u8>, Arc<TcpStream>)>,
    pub tcp_rx: Receiver<(Vec<u8>, Arc<TcpStream>)>,
}

#[derive(Component)]
pub struct UdpPacket {
    pub bytes: Vec<u8>,
    pub addr: SocketAddr,
}

#[derive(Component)]
pub struct TcpPacket {
    pub bytes: Vec<u8>,
    pub tcp_stream: Arc<TcpStream>,
}

impl Communication {
    pub fn new(
        udp_tx: Sender<(Vec<u8>, SocketAddr)>,
        udp_rx: Receiver<(Vec<u8>, SocketAddr)>,
        tcp_tx: Sender<(Vec<u8>, Arc<TcpStream>)>,
        tcp_rx: Receiver<(Vec<u8>, Arc<TcpStream>)>,
    ) -> Self {
        Self {
            udp_tx,
            udp_rx,
            tcp_tx,
            tcp_rx,
        }
    }
}

pub async fn start_udp_task(
    bind_addr: &str,
    mut outbound: Receiver<(Vec<u8>, SocketAddr)>,
    inbound: Sender<(Vec<u8>, SocketAddr)>,
    pool_size: usize,
) -> Result<(), io::Error> {
    let socket = Arc::new(UdpSocket::bind(bind_addr).await?);
    let send_sock = socket.clone();

    println!("Socket bound on {:?}", socket.local_addr()?);

    for _ in 0..pool_size {
        let recv_sock = socket.clone();
        let inbound_tx = inbound.clone();

        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                match recv_sock.recv_from(&mut buf).await {
                    Ok((len, addr)) => {
                        let _ = inbound_tx.send((buf[..len].to_vec(), addr)).await;
                    }
                    Err(e) => {
                        eprintln!("recv error: {e}, continuing...");
                    }
                }
            }
        });
    }

    tokio::spawn(async move {
        while let Some((bytes, addr)) = outbound.recv().await {
            match send_sock.send_to(&bytes, &addr).await {
                Ok(_) => {}
                Err(e) => println!("send error: {}", e),
            }
        }
    });

    Ok(())
}

pub async fn init_connection(addr: SocketAddr, lobby_id: u128) -> Result<u128, io::Error> {
    let socket = TcpSocket::new_v4()?;
    let stream = socket.connect(addr).await?;

    stream.ready(Interest::WRITABLE).await?;
    let mut encoded_data = Vec::new();
    encoded_data.push(TCP::Join { lobby_id });
    stream.try_write(
        bincode::serde::encode_to_vec(encoded_data, config::standard())
            .unwrap()
            .as_slice(),
    )?;

    let mut buf = [0; 200];

    stream.ready(Interest::READABLE).await?;
    stream.try_read(&mut buf)?;

    println!("uid: {:x?}", buf);
    let mut uuid = 0;

    let decoded: (Vec<TCP>, _) =
        bincode::serde::decode_from_slice(&buf, config::standard()).unwrap();

    println!("{:?}", decoded);

    for m in decoded.0 {
        match m {
            TCP::PlayerId { player_uid } => {
                uuid = player_uid;
            }
            _ => {}
        }
    }

    Ok(uuid)
}

pub async fn connect_to_server(socket: &UdpSocket, addr: SocketAddr) -> Result<(), io::Error> {
    match socket.connect(addr).await {
        Ok(_) => {
            println!("connected to server");
            Ok(())
        }
        Err(_) => retry_connection(socket, addr, 5).await,
    }
}

pub async fn retry_connection(
    socket: &UdpSocket,
    addr: SocketAddr,
    retry_count: u8,
) -> Result<(), io::Error> {
    for _ in 0..retry_count {
        match socket.connect(addr).await {
            Ok(_) => {
                println!("connected to server");
                return Ok(());
            }
            Err(_) => {
                println!("failed to connect to server, retrying...");
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::Other,
        "failed to connect to server",
    ))
}
