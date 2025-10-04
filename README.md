# *NET*work re*LAY* (`netlay`)

A command-line Linux utility for asynchronously relaying TCP and UDP sockets between machines. Useful for bridging traffic across networks that are not directly routed together — ideal for development, maintenance, or debugging sessions.

## Features

- **Bidirectional TCP/UDP forwarding** between local and remote endpoints
- **Configurable via TOML file** or command-line arguments
- **Supports port ranges** (e.g., `8000..8010`) and single ports
- **Asynchronous I/O** for efficient, scalable performance using [Tokio](https://tokio.rs/)
- **Minimal dependencies**, easy to deploy

## Usage

```sh
netlay [OPTIONS]
```

### Options

| Option                              | Description                                                         |
|-------------------------------------|---------------------------------------------------------------------|
| `-c`, `--config-file <CONFIG_FILE>` | Path to the configuration file (default: `/etc/netlay/netlay.conf`) |
| `-r`, `--relay <RELAY_URL>`         | Relay traffic according to this rule, bypassing the config file     |
| `-h`, `--help`                      | Print help information                                              |

### RELAY_URL syntax

```text
<tcp|udp>://<IPv4_address>:<port_range>
```

- `<tcp|udp>`: Socket type (TCP or UDP)
- `<IPv4_address>`: Destination address to forward traffic to
- `<port_range>`: Port number or range (e.g., `8080` or `8000..8010`)

### Examples

Relay traffic according to a custom configuration file:

```sh
netlay --config-file my_file.conf
```

Relay TCP traffic on port 80 to `192.168.100.200`:

```sh
netlay --relay tcp://192.168.100.200:80
```

Relay UDP traffic on all ports between 1000 and 1010 to `192.168.100.200`:

```sh
netlay --relay udp://192.168.100.200:1000..1010
```

## Configuration file example (`netlay.conf`)

The configuration file(s) need to follow the TOML syntax. An example is depicted below:

```toml
relays = [
    "tcp://192.168.1.100:8080",
    "udp://192.168.1.101:5353",
    "tcp://10.0.0.2:8000..8010"
]
```

## How it works

What makes this utility stand out is the use of asynchronous processing (via [Tokio](https://tokio.rs/)) to efficiently relay traffic. Unlike synchronous tools that require a thread per connection, `netlay` leverages a thread pool sized to available CPU cores, enabling high performance and scalability for many simultaneous connections.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contributing

Contributions, bug reports, and feature requests are welcome! Please open an issue or submit a pull request.

## Author

Aurelian Pop
