use crate::messages::Node;
use crate::Result;
use artnet_protocol::PollReply;
use failure::ResultExt;
use serde::Serialize;
use std::net::SocketAddr;
use std::str;

pub struct Client {
    pub socket_address: SocketAddr,
    pub addr: [u8; 4],
    pub addr_string: String,
    pub last_reply_received: f64,
    pub short_name: String,
    pub long_name: String,
    pub current: RenderMode,
    pub millis_since_last_frame: usize,
    pub current_animation_frame: usize,
}

#[derive(Debug, Clone, Serialize)]
pub enum RenderMode {
    Color(u8, u8, u8),
    Animation(String),
}

impl Client {
    pub fn new(socket_address: SocketAddr, reply: &PollReply) -> Result<Client> {
        let short_name_index = reply
            .short_name
            .iter()
            .position(|b| *b == 0)
            .unwrap_or_else(|| reply.short_name.len());
        let long_name_index = reply
            .long_name
            .iter()
            .position(|b| *b == 0)
            .unwrap_or_else(|| reply.long_name.len());

        let short_name = str::from_utf8(&reply.short_name[..short_name_index])
            .context("Could not get short_name")?
            .to_owned();
        let long_name = str::from_utf8(&reply.long_name[..long_name_index])
            .context("Could not get long_name")?
            .to_owned();

        Ok(Client {
            socket_address,
            addr: reply.address.octets(),
            addr_string: format!("{}", reply.address),
            short_name,
            long_name,
            last_reply_received: 0.,
            current: RenderMode::Color(0, 100, 0),
            millis_since_last_frame: 0,
            current_animation_frame: 0,
        })
    }

    pub fn get_node(&self) -> Node {
        Node {
            ip: self.addr_string.clone(),
            short_name: self.short_name.clone(),
            long_name: self.long_name.clone(),
            current: self.current.clone(),
        }
    }
}
