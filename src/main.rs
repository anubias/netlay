use std::net::{Ipv4Addr, SocketAddr};

use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream, UdpSocket},
};

use crate::config::PortRange;

mod cmdline;
mod config;

#[tokio::main]
async fn main() {
    let args = cmdline::Args::parse();
    let config = config::Config::load_config(&args.config_file);

    if let Some(tcp_forwards) = config.tcp_forwards {
        for forward in &tcp_forwards {
            match forward.port {
                PortRange::Single(port) => forward_tcp_port(forward.addr, port),
                PortRange::Range { begin, end } => {
                    for port in begin..=end {
                        forward_tcp_port(forward.addr, port);
                    }
                }
            }
        }
    }

    if let Some(udp_forwards) = config.udp_forwards {
        for forward in &udp_forwards {
            match forward.port {
                PortRange::Single(port) => forward_udp_port(forward.addr, port),
                PortRange::Range { begin, end } => {
                    for port in begin..=end {
                        forward_udp_port(forward.addr, port);
                    }
                }
            }
        }
    }

    // Prevent main from exiting immediately
    tokio::signal::ctrl_c().await.unwrap();
}

fn forward_tcp_port(addr: Ipv4Addr, port: u16) {
    let local_addr = SocketAddr::from(([0, 0, 0, 0], port));
    let remote_addr = SocketAddr::from((addr, port));

    tokio::spawn(async move {
        let listener = TcpListener::bind(local_addr)
            .await
            .expect("Failed to bind TCP socket");

        loop {
            match listener.accept().await {
                Ok((inbound, _)) => {
                    let remote_addr = remote_addr;
                    connect_and_transfer_tcp_traffic(remote_addr, inbound);
                }
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                }
            }
        }
    });
}

fn connect_and_transfer_tcp_traffic(remote_addr: SocketAddr, mut inbound: TcpStream) {
    tokio::spawn(async move {
        match TcpStream::connect(remote_addr).await {
            Ok(mut outbound) => {
                let _ = copy_bidirectional(&mut inbound, &mut outbound).await;
            }
            Err(e) => {
                eprintln!("Failed to connect to remote: {}", e);
            }
        }
    });
}

fn forward_udp_port(addr: Ipv4Addr, port: u16) {
    let local_addr = SocketAddr::from(([0, 0, 0, 0], port));
    let remote_addr = SocketAddr::from((addr, port)); // same port

    tokio::spawn(async move {
        let socket = UdpSocket::bind(local_addr)
            .await
            .expect("Failed to bind UDP socket");
        println!("UDP listening on {}", local_addr);

        let mut buf = vec![0u8; 65535];

        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, src_addr)) => {
                    // Forward to remote
                    if src_addr != remote_addr {
                        let _ = socket.send_to(&buf[..len], remote_addr).await;
                    } else {
                        // Forward from remote back to original sender
                        let _ = socket.send_to(&buf[..len], src_addr).await;
                    }
                }
                Err(e) => {
                    eprintln!("UDP recv error: {}", e);
                }
            }
        }
    });
}
