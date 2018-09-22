use serde_json;
use std::fs::File;
use std::net::SocketAddr;
use Result;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub web_endpoint: SocketAddr,
    pub torch_endpoints: Vec<SocketAddr>,
    pub broadcasts: Vec<SocketAddr>,
    pub torch_mappings: Vec<TorchMap>,
}

impl Config {
    pub fn from_file(file: &str) -> Result<Config> {
        let mut file = File::open(file)?;
        let config: Config = serde_json::from_reader(&mut file)?;
        Ok(config)
    }
}

#[derive(Deserialize, Debug)]
pub struct TorchMap {
    pub ip: SocketAddr,
    pub color: Color,
    pub side: Side,
    pub row: u8,
}

#[derive(Deserialize, Debug)]
pub enum Color {
    Blue,
    Red,
    Green,
    Purple,
}

#[derive(Deserialize, Debug)]
pub enum Side {
    Middle,
    Outside,
}
