# *NET*work re*LAY* (`netlay`)

A command-line Linux utility for asynchronously relaying TCP and UDP sockets between machines. Useful for bridging traffic across networks that are not directly routed together — ideal for development, maintenance, or debugging sessions.

## Features

- **Bidirectional TCP/UDP forwarding** between local and remote endpoints
- **Asynchronous I/O** for efficiency and scalability
- **Configurable via TOML file** or command-line arguments
- **Supports port ranges** for convenience (e.g. `8000..8010`)
- **Systemd** integrated

What makes this utility stand out is the use of asynchronous processing to efficiently relay network traffic. Unlike synchronous tools that require one thread per connection, `netlay` leverages a thread pool sized to the number of available CPU cores, enabling high performance and scalability for many simultaneous connections.

## Installation

Here is a step by step guideline on how to install and run the service.

### Alternative A (recommended)

Add this repository to your trusted `apt` repositories list. This will ensure that you will automatically take any updates, as they are released.

```sh
# This will create a netlay.gpg file under /etc/apt/keyrings
curl -fsSL https://anubias.github.io/netlay-apt/public.key | sudo gpg --dearmor -o /etc/apt/keyrings/netlay-apt.gpg

# This will create a `netlay-apt.list` file under `/etc/apt/sources.list.d`
echo "deb [signed-by=/etc/apt/keyrings/netlay-apt.gpg] https://anubias.github.io/netlay-apt stable main" \
| sudo tee /etc/apt/sources.list.d/netlay-apt.list

# Update apt package list, to allow apt to discover this package
sudo apt update

# Install and verify the installation  
sudo apt install netlay
systemctl status netlay.service
```

### Alternative B

Manually download and install the deb package. However, this will not automatically give you the new updates. You will have to manually manage that process yourself.

```sh
# The name (version) of the file may have changed meanwhile
curl -O https://anubias.github.io/netlay-apt/pool/main/n/netlay/netlay_0.1.0-1_amd64.deb

# You can now install the downloaded debian file directly
sudo apt install ./netlay_0.1.0-1_amd64.deb
```

### Configure, enable, and start the service

Now that the netlay is installed, let's get started.

```sh
# First, edit the configuration file according to your needs
nano /etc/netlay/netlay.conf

# Enable and start the service
sudo systemctl enable --now netlay.service
sudo systemctl start netlay.service

# Optionally, check the service status
systemctl status netlay.service
```

Netlay should now be up and running.

## Usage

Once installed, the best way to use netlay is via `systemctl` command interface. However, you can also interact directly with netlay by invoking the command in your terminal.

### Synopsis

```sh
netlay [OPTIONS]
```

#### Options

| Option                              | Description                                                         |
|-------------------------------------|---------------------------------------------------------------------|
| `-c`, `--config-file <CONFIG_FILE>` | Path to the configuration file (default: `/etc/netlay/netlay.conf`) |
| `-r`, `--relay <RELAY_URL>`         | Relay traffic according to this rule, bypassing the config file     |
| `-h`, `--help`                      | Print help information                                              |

### Configuration file example (`netlay.conf`)

The main usecase for netlay would be to run as a background service, with the default configuration file. The configuration file(s) need to follow the TOML syntax. An example is depicted below:

```toml
relays = [
    "tcp://192.168.1.100:8080",
    "udp://192.168.1.101:5353",
    "tcp://10.0.0.2:8000..8010"
]
```

#### RELAY_URL syntax

As you can see from the provided configuration example, a relay URL line must follow this syntax:

```text
<tcp|udp>://<IPv4_address>:<port_range>
```

where:

- `<tcp|udp>`: Socket type (TCP or UDP)
- `<IPv4_address>`: Destination address to forward traffic to
- `<port_range>`: Port number or range (e.g., `8080` or `8000..8010`)

## License

This project is licensed under the MIT License terms.

## Contributing

Contributions, bug reports, and feature requests are welcome! Please open an issue or submit a pull request.

## Author

Aurelian Pop
