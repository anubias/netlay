use std::net::{Ipv4Addr, SocketAddr};

use chrono::Local;
use fern::Dispatch;
use log::{error, info, LevelFilter};
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
    const LOG_FILE_NAME: &str = "/var/log/netlay/netlay.log";

    let log_file_res = fern::log_file(LOG_FILE_NAME);
    let mut log_dispatch = Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} -- {}: {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Info)
        .chain(std::io::stdout());

    match log_file_res {
        Ok(log_file) => log_dispatch = log_dispatch.chain(log_file),
        Err(e) => {
            error!(
                "Failed to open log file {LOG_FILE_NAME}, logs will only go to the console: {e}"
            );
        }
    }

    if let Err(e) = log_dispatch.apply() {
        eprintln!("Failed to initialize logging: {e}");
        return;
    }

    let args = cmdline::Args::parse();
    let config = if let Some(user_rule) = args.relay {
        let deserializer = StringDeserializer::<serde::de::value::Error>::new(user_rule);
        match config::Relay::deserialize(deserializer) {
            Ok(relay) => Config {
                relays: Some(vec![relay]),
            },
            Err(e) => {
                error!("Failed to parse relay rule from the command line: {e}");
                return;
            }
        }
    } else {
        info!("Loading config from {} ...", &args.config_file);
        Config::load_config(&args.config_file)
    };

    if let Some(rules) = config.relays {
        for rule in &rules {
            discriminate_relay(rule);
        }

        // Prevent main from exiting immediately
        tokio::signal::ctrl_c().await.unwrap();
    } else {
        info!("Nothing to do. Quiting...");
    }
}

fn discriminate_relay(relay: &Relay) {
    match relay.protocol {
        config::Protocol::TCP => match relay.port_range {
            config::PortRange::Single(port) => relay_tcp_port(relay.addr, port),
            config::PortRange::Range { begin, end } => {
                for port in begin..=end {
                    relay_tcp_port(relay.addr, port);
                }
            }
        },
        config::Protocol::UDP => match relay.port_range {
            config::PortRange::Single(port) => relay_udp_port(relay.addr, port),
            config::PortRange::Range { begin, end } => {
                for port in begin..=end {
                    relay_udp_port(relay.addr, port);
                }
            }
        },
    }
}

fn relay_tcp_port(addr: Ipv4Addr, port: u16) {
    let local_addr = SocketAddr::from(([0, 0, 0, 0], port));
    let remote_addr = SocketAddr::from((addr, port));

    tokio::spawn(async move {
        let listener = match TcpListener::bind(local_addr).await {
            Ok(l) => l,
            Err(e) => {
                error!("Failed to bind TCP socket {addr}:{port} {e}");
                return;
            }
        };

        info!("Listening on TCP port {port} ...");

        loop {
            match listener.accept().await {
                Ok((inbound, _)) => {
                    info!(
                        "Accepted TCP connection from {}",
                        inbound
                            .peer_addr()
                            .unwrap_or(SocketAddr::from(([0, 0, 0, 0], 0)))
                    );
                    let remote_addr = remote_addr;
                    //Start a new task to handle this connection independently while waiting for another one.
                    connect_and_transfer_tcp_traffic(remote_addr, inbound);
                }
                Err(e) => {
                    error!("Accept error on sockcet {addr}:{port} {e}");
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
                error!("Failed to connect to remote {remote_addr} {e}");
            }
        }
    });
}

fn relay_udp_port(addr: Ipv4Addr, port: u16) {
    let local_addr = SocketAddr::from(([0, 0, 0, 0], port));
    let remote_addr = SocketAddr::from((addr, port)); // same port

    tokio::spawn(async move {
        let socket = match UdpSocket::bind(local_addr).await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to bind UDP socket {addr}:{port} {e}");
                return;
            }
        };

        info!("Listening on UDP port {port} ...");

        let mut buf = vec![0u8; 8192];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, src_addr)) => {
                    // Forward to remote
                    if src_addr != remote_addr {
                        // Send any received packet not originating from the remote address to the remote address
                        let _ = socket.send_to(&buf[..len], remote_addr).await;
                    } else {
                        // Forward any received packet from the remote address back to the original sender
                        let _ = socket.send_to(&buf[..len], src_addr).await;
                    }
                }
                Err(e) => {
                    error!("UDP recv error: {e}");
                }
            }
        }
    });
}
