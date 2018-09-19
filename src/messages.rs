use actix::Message;
use artnet_protocol::ArtCommand;
use bytes::Bytes;
use std::net::Ipv4Addr;
use Result;

pub struct SendMessage {
    pub address: Ipv4Addr,
    pub message: ArtCommand,
}

impl Message for SendMessage {
    type Result = ();
}

#[derive(Debug)]
pub struct RequestNodeList;

impl Message for RequestNodeList {
    type Result = Result<ResponseNodeList>;
}

#[derive(Debug)]
pub struct ResponseNodeList {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Serialize)]
pub struct Node {
    pub ip: String,
    pub short_name: String,
    pub long_name: String,
    pub current_animation: String,
}

#[derive(Debug)]
pub struct RequestAnimationList;

impl Message for RequestAnimationList {
    type Result = Result<ResponseAnimationList>;
}

#[derive(Debug)]
pub struct ResponseAnimationList {
    pub animations: Vec<Animation>,
}

#[derive(Debug)]
pub struct AddAnimation {
    name: String,
    bytes: Bytes,
}

impl Message for AddAnimation {
    type Result = ();
}

#[derive(Default, Clone, Debug, Serialize)]
pub struct Animation {
    pub name: String,
    #[serde(skip_serializing)]
    pub frames: Vec<AnimationFrame>,
    pub fps: u16,
}

pub type AnimationFrame = [[(u8, u8, u8); 7]; 22];

#[derive(Debug)]
pub struct SetNodeAnimation {
    pub ip: String,
    pub animation_name: String,
}

impl Message for SetNodeAnimation {
    type Result = Result<()>;
}
