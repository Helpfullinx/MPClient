use bevy::prelude::{Component, Resource};
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

pub async fn start_tcp_task(
    bind_addr: SocketAddr,
    mut outbound: Receiver<(Vec<u8>, Arc<TcpStream>)>,
    inbound: Sender<(Vec<u8>, Arc<TcpStream>)>,
) -> Result<(), Error> {
    let socket = TcpSocket::new_v4()?;

    let inbound_accept = inbound.clone();
    // Task responsible for accepting new TCP connections
    tokio::spawn(async move {
        // Accept first connection in queue
        match socket.connect(bind_addr).await {
            Ok(stream) => {
                println!("Connected to server");

                // TODO: Apparently this can create false positives and what it reads because of that may be empty, therefore we have to check that
                // Get the ready-ness value for the stream
                let stream = Arc::new(stream);
                
                // Save stream
                inbound_accept.send((vec![], stream.clone()));

                // Spawn a task dedicated to continuously reading from this client
                let inbound_task = inbound_accept.clone();
                let stream_task = stream.clone();
                let mut read_buf = vec![0u8; 2048];
                loop {
                    let ready = stream_task.ready(Interest::READABLE).await.unwrap();
                    if ready.is_readable() {
                        match stream_task.try_read(&mut read_buf) {
                            Ok(0) => break, // connection closed
                            Ok(len) => {
                                let _ = inbound_task
                                    .send((read_buf[..len].to_vec(), stream_task.clone()))
                                    .await;
                            }
                            Err(e) => {
                                println!("Couldn't read: {:?}", e);
                                break;
                            }
                        }
                    }
                }
            }
            Err(_) => todo!(),
        }
    });

    // Task responsible for sending queued TCP messages
    tokio::spawn(async move {
        while let Some(bytes) = outbound.recv().await {
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
