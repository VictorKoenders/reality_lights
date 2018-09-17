use artnet_protocol::PollReply;
use messages::Node;
use std::str;

#[derive(Serialize)]
pub struct Client {
    pub addr: [u8; 4],
    pub last_reply_received: f64,
    pub short_name: String,
    pub long_name: String,
    pub current_animation: Option<(usize, String)>,
}

impl Client {
    pub fn new(reply: &PollReply) -> Client {
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
            addr: reply.address.octets(),
            short_name: str::from_utf8(&reply.short_name[..short_name_index])
                .unwrap()
                .to_owned(),
            long_name: str::from_utf8(&reply.long_name[..long_name_index])
                .unwrap()
                .to_owned(),
            last_reply_received: 0.,
            current_animation: Some((0, String::from("red"))),
        }
    }

    pub fn get_node(&self) -> Node {
        Node {
            ip: self.addr,
            short_name: self.short_name.clone(),
            long_name: self.long_name.clone(),
            current_animation: self
                .current_animation
                .as_ref()
                .map(|(_, name)| name.clone()),
        }
    }
}
