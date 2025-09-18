use serde::{Deserialize, Deserializer};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub tcp_forwards: Option<Vec<PortForward>>,
    pub udp_forwards: Option<Vec<PortForward>>,
}

impl Config {
    pub fn load_config(filename: &String) -> Self {
        let contents = std::fs::read_to_string(filename).expect("Failed to read config file");
        toml::from_str(&contents).expect("Failed to parse config file")
    }
}

#[derive(Deserialize, Debug)]
pub struct PortForward {
    pub addr: std::net::Ipv4Addr,
    pub port: PortRange,
}

#[derive(Debug)]
pub enum PortRange {
    Single(u16),
    Range { begin: u16, end: u16 },
}

impl std::fmt::Display for PortRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortRange::Single(port) => write!(f, "{}", port),
            PortRange::Range { begin: start, end } => write!(f, "{}..{}", start, end),
        }
    }
}

impl<'de> Deserialize<'de> for PortRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if let Some((start, end)) = s.split_once("..") {
            let start = start.parse().map_err(serde::de::Error::custom)?;
            let end = end.parse().map_err(serde::de::Error::custom)?;
            Ok(PortRange::Range { begin: start, end })
        } else {
            let single = s.parse().map_err(serde::de::Error::custom)?;
            Ok(PortRange::Single(single))
        }
    }
}
