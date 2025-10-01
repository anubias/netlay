use std::str::FromStr;

use serde::{Deserialize, Deserializer};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub relays: Option<Vec<Relay>>,
}

impl Config {
    pub fn load_config(filename: &String) -> Self {
        println!("Loading config from {} ...", filename);
        let contents = std::fs::read_to_string(filename)
            .expect(format!("Failed to read config file {filename}").as_str());
        toml::from_str(&contents).expect(format!("Failed to parse config file {filename}").as_str())
    }
}

#[derive(Debug)]
pub struct Relay {
    pub protocol: Protocol,
    pub addr: std::net::Ipv4Addr,
    pub port_range: PortRange,
}

impl<'de> Deserialize<'de> for Relay {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if let Some((protocol, target)) = s.split_once("://") {
            let protocol = protocol.parse().map_err(serde::de::Error::custom)?;
            if let Some((addr, port)) = target.rsplit_once(':') {
                let addr = addr.parse().map_err(serde::de::Error::custom)?;
                let port_range = port.parse().map_err(serde::de::Error::custom)?;
                return Ok(Relay {
                    protocol,
                    addr,
                    port_range,
                });
            } else {
                return Err(serde::de::Error::custom(format!(
                    "Invalid target format: {target}"
                )));
            }
        } else {
            Err(serde::de::Error::custom(format!(
                "Invalid relay rule format {s}"
            )))
        }
    }
}

#[derive(Debug)]
pub enum Protocol {
    TCP,
    UDP,
}

impl FromStr for Protocol {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tcp" => Ok(Protocol::TCP),
            "udp" => Ok(Protocol::UDP),
            _ => Err(format!("Invalid protocol: {s}")),
        }
    }
}

#[derive(Debug)]
pub enum PortRange {
    Single(u16),
    Range { begin: u16, end: u16 },
}

impl std::fmt::Display for PortRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortRange::Single(port) => write!(f, "{port}"),
            PortRange::Range { begin: start, end } => write!(f, "{start}..{end}"),
        }
    }
}

impl FromStr for PortRange {
    type Err = String;

    fn from_str(port: &str) -> Result<Self, Self::Err> {
        if let Some((begin, end)) = port.split_once("..") {
            let begin = begin
                .parse()
                .map_err(|_| format!("Invalid port range beginning {begin}"))?;
            let end = end
                .parse()
                .map_err(|_| format!("Invalid port range ending {end}"))?;
            if begin < end {
                Ok(PortRange::Range { begin, end })
            } else {
                Err(format!(
                    "Beginning of port range ({begin}) must be lower than the ending of the port range ({end})"
                ))
            }
        } else {
            let single = port.parse().map_err(|_| format!("Invalid port {port}"))?;
            Ok(PortRange::Single(single))
        }
    }
}
