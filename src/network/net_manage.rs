use crate::network::net_message::{NetworkMessage, TCP, UDP};
use bevy::prelude::{Component, Resource};
use std::collections::VecDeque;
use std::io::Error;
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

#[derive(Resource, Debug)]
pub struct UdpConnection {
    pub remote_socket: Option<SocketAddr>,
    pub input_packet_buffer: VecDeque<Packet>,
    pub output_message: Vec<NetworkMessage<UDP>>,
}

#[derive(Resource, Debug)]
pub struct TcpConnection {
    pub stream: Option<Arc<TcpStream>>,
    pub input_packet_buffer: VecDeque<Packet>,
    pub output_message: Vec<NetworkMessage<TCP>>,
}

#[derive(Component, Debug)]
pub struct Packet {
    pub bytes: Vec<u8>,
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
    remote_addr: SocketAddr,
    mut outbound: Receiver<(Vec<u8>, SocketAddr)>,
    inbound: Sender<(Vec<u8>, SocketAddr)>,
    pool_size: usize,
) -> Result<(), Error> {
    let socket = Arc::new(UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).await?);

    println!("Socket bound on {:?}", socket.local_addr()?);

    let _ = inbound.send((vec![], remote_addr)).await;

    for _ in 0..pool_size {
        let inbound_tx = inbound.clone();
        let s = socket.clone();

        tokio::spawn(async move {
            let mut buf = vec![0u8; 1024];
            loop {
                match s.clone().recv_from(&mut buf).await {
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
            match socket.clone().send_to(&bytes, &addr).await {
                Ok(_) => {}
                Err(e) => println!("send error: {}", e),
            }
        }
    });

    Ok(())
}

pub async fn start_tcp_task(
    remote_addr: SocketAddr,
    mut outbound: Receiver<(Vec<u8>, Arc<TcpStream>)>,
    inbound: Sender<(Vec<u8>, Arc<TcpStream>)>,
) -> Result<(), Error> {
    let socket = TcpSocket::new_v4()?;

    let inbound_accept = inbound.clone();
    // Task responsible for accepting new TCP connections
    tokio::spawn(async move {
        // Accept first connection in queue
        match socket.connect(remote_addr).await {
            Ok(stream) => {
                println!("Connected to server via TCP: {:?}", remote_addr.ip().to_string());

                // TODO: Apparently this can create false positives and what it reads because of that may be empty, therefore we have to check that
                // Get the ready-ness value for the stream
                let stream = Arc::new(stream);

                // Save stream
                let _ = inbound_accept.send((vec![], stream.clone())).await;

                // Spawn a task dedicated to continuously reading from this client
                let inbound_task = inbound_accept.clone();
                let stream_task = stream.clone();
                let mut read_buf = vec![0u8; 1024];
                loop {
                    let ready = stream_task.ready(Interest::READABLE).await.unwrap();
                    if ready.is_readable() {
                        match stream_task.try_read(&mut read_buf) {
                            Ok(0) => { break }
                            Ok(len) => {
                                let _ = inbound_task
                                    .send((read_buf[..len].to_vec(), stream_task.clone()))
                                    .await;
                            }
                            Err(e) => {
                                println!("Couldn't read: {:?}", e);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                println!("Couldn't connect to remote server")
            }
        }
    });

    // Task responsible for sending queued TCP messages
    tokio::spawn(async move {
        while let Some((bytes, stream)) = outbound.recv().await {
            let ready = stream.ready(Interest::WRITABLE).await.unwrap();

            if ready.is_writable() {
                match stream.try_write(&*bytes) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Couldn't write: {:?}", e)
                    }
                };
            }
        }
    });

    Ok(())
}
