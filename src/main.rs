use std::{
    env,
    fs::OpenOptions,
    net::{Ipv4Addr, SocketAddr},
    os::unix::io::AsRawFd,
    process::exit,
};

use chrono::Local;
use fern::Dispatch;
use log::{debug, error, info, trace, warn, LevelFilter};
use serde::{de::value::StringDeserializer, Deserialize};
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream, UdpSocket},
};

use crate::config::{Config, Relay};

mod cmdline;
mod config;

fn main() -> std::io::Result<()> {
    let args = cmdline::Args::parse();
    init_logging(args.daemon_mode)?;

    if args.daemon_mode {
        daemonize()?;
    }
    async_main(args);

    Ok(())
}

#[tokio::main]
async fn async_main(args: cmdline::Args) {
    let config = if let Some(relay_rule) = args.relay {
        let deserializer = StringDeserializer::<serde::de::value::Error>::new(relay_rule);
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

fn init_logging(daemon_mode: bool) -> std::io::Result<()> {
    let mut log_dispatch = Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} -- {}: {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Info);

    if daemon_mode {
        const LOG_FILE_NAME: &str = "/var/log/netlay/netlay.log";

        let log_file_res = fern::log_file(LOG_FILE_NAME);
        log_dispatch = match log_file_res {
            Ok(log_file) => log_dispatch.chain(log_file),
            Err(e) => {
                eprintln!("Failed to open log file {LOG_FILE_NAME}: {e}");
                return std::io::Result::Err(e);
            }
        };
    } else {
        log_dispatch = log_dispatch.chain(std::io::stdout());
    }

    if let Err(e) = log_dispatch.apply() {
        eprintln!("Failed to initialize logging: {e}");
        return std::io::Result::Err(std::io::Error::other("Failed to initialize logging"));
    }

    Ok(())
}

fn daemonize() -> std::io::Result<()> {
    // Double-fork daemonization
    unsafe {
        match libc::fork() {
            -1 => return Err(std::io::Error::last_os_error()),
            0 => {}                 //child continues
            _parent_pid => exit(0), // parent exits
        }

        // Create a new session and become session leader
        if libc::setsid() == -1 {
            eprintln!("Failed to create new session for daemon process");
            return Err(std::io::Error::last_os_error());
        }

        // Second fork to prevent reacquiring a controlling terminal
        match libc::fork() {
            -1 => return Err(std::io::Error::last_os_error()),
            0 => {}                 // grandchild continues
            _parent_pid => exit(0), // intermediate exits
        }
    }

    // reset file mode creation mask
    unsafe { libc::umask(0) };

    // change working dir to root
    env::set_current_dir("/")?;

    // redirect stdio to /dev/null (reuse existing helper)
    redirect_stdio_to_devnull()?;

    Ok(())
}

fn redirect_stdio_to_devnull() -> std::io::Result<()> {
    // Open /dev/null for read+write
    let devnull = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")?;
    let fd = devnull.as_raw_fd();

    // Duplicate /dev/null onto stdin/stdout/stderr
    unsafe {
        if libc::dup2(fd, libc::STDIN_FILENO) == -1 {
            return Err(std::io::Error::last_os_error());
        }
        if libc::dup2(fd, libc::STDOUT_FILENO) == -1 {
            return Err(std::io::Error::last_os_error());
        }
        if libc::dup2(fd, libc::STDERR_FILENO) == -1 {
            return Err(std::io::Error::last_os_error());
        }
    }

    // devnull is dropped here; the duplicated fds remain valid
    Ok(())
}

fn discriminate_relay(relay: &Relay) {
    match relay.protocol {
        config::Protocol::Tcp => match relay.port_range {
            config::PortRange::Single(port) => relay_tcp_port(relay.addr, port),
            config::PortRange::Range { begin, end } => {
                for port in begin..=end {
                    relay_tcp_port(relay.addr, port);
                }
            }
        },
        config::Protocol::Udp => match relay.port_range {
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
                error!("Failed to bind TCP socket {local_addr} {e}");
                return;
            }
        };

        info!("Listening on local TCP port {port} ...");

        loop {
            match listener.accept().await {
                Ok((inbound, _)) => {
                    info!(
                        "Accepted TCP connection from {} on local port {port}",
                        inbound.peer_addr().unwrap_or(remote_addr)
                    );
                    //Start a new task to handle this connection independently while waiting for another one.
                    connect_and_transfer_tcp_traffic(remote_addr, inbound);
                }
                Err(e) => {
                    error!("Accept error on socket {remote_addr} {e}");
                }
            }
        }
    });
}

fn connect_and_transfer_tcp_traffic(remote_addr: SocketAddr, mut inbound: TcpStream) {
    tokio::spawn(async move {
        match TcpStream::connect(remote_addr).await {
            Ok(mut outbound) => {
                let res = copy_bidirectional(&mut inbound, &mut outbound).await;
                match res {
                    Ok((a_to_b, b_to_a)) => {
                        trace!("TCP connection to {remote_addr} closed.");
                        trace!("Transferred {a_to_b} total bytes from inbound and {b_to_a} total bytes from outbound.");
                    }
                    Err(e) => {
                        error!("TCP data transfer error with {}: {e}", remote_addr);
                    }
                }
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
                error!("Failed to bind UDP socket {local_addr} {e}");
                return;
            }
        };

        info!("Listening on local UDP port {port} ...");

        const BUFFER_SIZE: usize = 65536; // Using maximum UDP packet size, to avoid loss of data
        let mut buf = vec![0u8; BUFFER_SIZE];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((recv_len, src_addr)) => {
                    debug!("Received UDP packet ({recv_len} bytes) from {src_addr} on local port {port}");
                    if recv_len == BUFFER_SIZE {
                        warn!("Received UDP packet may be truncated (UDP packet size equals to internal buffer size)");
                    }

                    // Forward to remote
                    if src_addr != remote_addr {
                        // Send any received packet not originating from the remote address to the remote address
                        let res = socket.send_to(&buf[..recv_len], remote_addr).await;
                        match res {
                            Ok(send_len) => trace!("Forwarded UDP packet ({send_len} bytes) from {src_addr} to {remote_addr}"),
                            Err(e) => error!("UDP send error to {remote_addr}: {e}"),
                        }
                    } else {
                        // Forward any received packet from the remote address back to the original sender
                        let res = socket.send_to(&buf[..recv_len], src_addr).await;
                        match res {
                            Ok(send_len) => trace!("Forwarded UDP packet ({send_len} bytes) from {remote_addr} to {src_addr}"),
                            Err(e) => error!("UDP send error to {src_addr}: {e}"),
                        }
                    }
                }
                Err(e) => {
                    error!("UDP recv error from {remote_addr} {e}");
                }
            }
        }
    });
}
