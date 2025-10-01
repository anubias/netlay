use std::net::{Ipv4Addr, SocketAddr};

use serde::{de::value::StringDeserializer, Deserialize};
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream, UdpSocket},
};

use crate::config::{Config, Relay};

mod cmdline;
mod config;

#[tokio::main]
async fn main() {
    let args = cmdline::Args::parse();
    let config = if let Some(user_rule) = args.relay {
        let deserializer = StringDeserializer::<serde::de::value::Error>::new(user_rule);
        let relay = config::Relay::deserialize(deserializer)
            .expect("Failed to parse relay rule from command line");
        Config {
            relays: Some(vec![relay]),
        }
    } else {
        Config::load_config(&args.config_file)
    };

    if let Some(rules) = config.relays {
        for rule in &rules {
            discriminate_relay(rule);
        }

        // Prevent main from exiting immediately
        tokio::signal::ctrl_c().await.unwrap();
    } else {
        println!("Nothing to do. Quiting...");
    }
}

fn discriminate_relay(relay: &Relay) {
    match relay.protocol {
        config::Protocol::TCP => match relay.port_range {
            config::PortRange::Single(port) => forward_tcp_port(relay.addr, port),
            config::PortRange::Range { begin, end } => {
                for port in begin..=end {
                    forward_tcp_port(relay.addr, port);
                }
            }
        },
        config::Protocol::UDP => match relay.port_range {
            config::PortRange::Single(port) => forward_udp_port(relay.addr, port),
            config::PortRange::Range { begin, end } => {
                for port in begin..=end {
                    forward_udp_port(relay.addr, port);
                }
            }
        },
    }
}

fn forward_tcp_port(addr: Ipv4Addr, port: u16) {
    let local_addr = SocketAddr::from(([0, 0, 0, 0], port));
    let remote_addr = SocketAddr::from((addr, port));

    tokio::spawn(async move {
        let listener = TcpListener::bind(local_addr)
            .await
            .expect(format!("Failed to bind TCP socket {addr}:{port}").as_str());

        loop {
            match listener.accept().await {
                Ok((inbound, _)) => {
                    let remote_addr = remote_addr;
                    connect_and_transfer_tcp_traffic(remote_addr, inbound);
                }
                Err(e) => {
                    eprintln!("Accept error on sockcet {addr}:{port} ({e})");
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
                eprintln!("Failed to connect to remote {remote_addr}: ({e})");
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
            .expect(format!("Failed to bind UDP socket {addr}:{port}").as_str());

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
                    eprintln!("UDP recv error: {e}");
                }
            }
        }
    });
}
