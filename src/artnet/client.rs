use artnet_protocol::PollReply;
use messages::Node;
use std::net::SocketAddr;
use std::str;

pub struct Client {
    pub socket_address: SocketAddr,
    pub addr: [u8; 4],
    pub addr_string: String,
    pub last_reply_received: f64,
    pub short_name: String,
    pub long_name: String,
    pub current_animation: String,
    pub current_animation_frame: usize,
}

impl Client {
    pub fn new(socket_address: SocketAddr, reply: &PollReply) -> Client {
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
        Client {
            socket_address,
            addr: reply.address.octets(),
            addr_string: format!("{}", reply.address),
            short_name: str::from_utf8(&reply.short_name[..short_name_index])
                .expect("Could not decode torch short_name")
                .to_owned(),
            long_name: str::from_utf8(&reply.long_name[..long_name_index])
                .expect("Could not decode torch long_name")
                .to_owned(),
            last_reply_received: 0.,
            current_animation: String::from("green"),
            current_animation_frame: 0,
        }
    }

    pub fn get_node(&self) -> Node {
        Node {
            ip: self.addr_string.clone(),
            short_name: self.short_name.clone(),
            long_name: self.long_name.clone(),
            current_animation: self.current_animation.clone(),
        }
    }
}
