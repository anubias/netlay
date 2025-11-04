use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    after_help = r#"If no arguments are given, the configuration file at /etc/netlay/netlay.conf will be used.

RELAY_URL format:
  <tcp|udp>://<IPv4_address>:<port_range>

where
  <tcp|udp>       - socket type
  <IPv4_address>  - destination address to forward traffic to
  <port_range>    - port numbers range (or single port value) to listen on and forward traffic to

Examples:
  Relay traffic according to a custom configuration file
    netlay --config my_file.conf

  Relay TCP traffic on port 80 to 192.168.100.200
    netlay --relay tcp://192.168.100.200:80"

  Relay UDP traffic on all ports between 1000 and 1010, to 192.168.100.200
    netlay --relay udp://192.168.100.200:1000..1010"#
)]
pub struct Args {
    /// Path to the configuration file
    #[arg(default_value = "/etc/netlay/netlay.conf", short, long)]
    pub config_file: String,

    /// Run as a background daemon, logs will be written to `/var/log/netlay/netlay.log`
    #[arg(default_value_t = false, short, long)]
    pub daemon_mode: bool,

    /// Only relay traffic according to this rule, bypassing the config file
    #[arg(short, long, value_name = "RELAY_URL")]
    pub relay: Option<String>,
}

impl Args {
    pub fn parse() -> Self {
        clap::Parser::parse()
    }
}
