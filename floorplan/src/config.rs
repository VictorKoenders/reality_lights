use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub mapping: HashMap<IpAddr, Torch>,
    pub areas: Vec<Area>,
}

impl Config {
    pub fn load() -> Result<Config, failure::Error> {
        let mut file = std::fs::File::open("config.json")?;
        let result = serde_json::from_reader(&mut file)?;
        Ok(result)
    }

    pub fn get_ip(&self, color: &str, row: u8, side: &str) -> Option<IpAddr> {
        let color = color.parse().ok()?;
        let side: Side = side.parse().ok()?;
        dbg!(&color);
        dbg!(&side);
        dbg!(row);

        for (ip, torch) in &self.mapping {
            if torch.color == color && torch.side == side && torch.row == row {
                return Some(*ip);
            }
        }

        None
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Area {
    pub color: Color,
    pub rows: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Torch {
    pub color: Color,
    pub side: Side,
    pub row: u8,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, enum_utils::FromStr)]
#[enumeration(case_insensitive)]
#[serde(rename_all = "lowercase")]
pub enum Color {
    Red,
    Green,
    Blue,
    Purple,
    Yellow,
    White,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, enum_utils::FromStr)]
#[enumeration(case_insensitive)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Right,
}
