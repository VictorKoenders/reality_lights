use crate::animation_handler::AnimationHandler;
use crate::artnet::{Client, Codec, RenderMode};
use crate::config::Config;
use crate::messages::{
    AddAnimation, RequestAnimationList, RequestNodeList, ResponseAnimationList, ResponseNodeList,
    SetNodeAnimation, SetNodeColor,
};
use crate::Result;
use actix::fut::wrap_future;
use actix::{
    Actor, ActorContext, ArbiterService, AsyncContext, Context, Handler, Message, StreamHandler,
    Supervised,
};
use artnet_protocol::{ArtCommand, Output};
use failure::{Error, ResultExt};
use futures::sync::mpsc::{channel, Sender};
use futures::{Future, Sink, Stream};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read as IoRead, Write as IoWrite};
use std::net::{SocketAddr, UdpSocket as NetSocket};
use std::time::Duration;
use std::{fs, mem};
use time;
use tokio_reactor::Handle;
use tokio_udp::{UdpFramed, UdpSocket};
use zip::ZipArchive;

pub struct Service {
    config: Config,
    clients: HashMap<SocketAddr, Client>,
    animations: AnimationHandler,
    udp_sender: Sender<(ArtCommand, SocketAddr)>,
}

impl Default for Service {
    fn default() -> Service {
        Service {
            config: Config::from_file("config.json").expect("Could not load config"),
            clients: HashMap::new(),
            animations: AnimationHandler::new().expect("Cannot load animation handler"),
            udp_sender: channel(0).0,
        }
    }
}

impl Actor for Service {
    type Context = Context<Self>;
}

impl Supervised for Service {}

impl ArbiterService for Service {
    fn service_started(&mut self, ctx: &mut Context<Self>) {
        if let Err(e) = self.init(ctx) {
            println!("Could not start arnet::service: {:?}", e);
            ctx.stop();
        }
    }
}

impl StreamHandler<(ArtCommand, SocketAddr), Error> for Service {
    fn handle(&mut self, (command, addr): (ArtCommand, SocketAddr), _ctx: &mut Context<Self>) {
        if !self.clients.contains_key(&addr) {
            if let ArtCommand::PollReply(reply) = &command {
                let client = match Client::new(addr, reply) {
                    Ok(c) => c,
                    Err(e) => {
                        println!("Could not accept client: {:?}", e);
                        return;
                    }
                };
                self.clients.insert(addr, client);
            } else {
                return;
            }
        }

        let client = self.clients.get_mut(&addr).expect("Unreachable");
        client.last_reply_received = time::precise_time_s();
    }
}

impl Service {
    fn init(&mut self, ctx: &mut Context<Self>) -> Result<()> {
        println!("Binding listening address");
        let socket = NetSocket::bind("0.0.0.0:6454")?;

        #[cfg(unix)]
        unsafe {
            use std::os::unix::io::AsRawFd;
            let optval: libc::c_int = 1;
            let ret = libc::setsockopt(
                socket.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_REUSEPORT,
                &optval as *const _ as *const libc::c_void,
                mem::size_of_val(&optval) as libc::socklen_t,
            );
            if ret != 0 {
                panic!("setsockopt failed");
            }
        }
        socket.set_broadcast(true)?;

        let framed = UdpFramed::new(
            UdpSocket::from_std(socket, &Handle::default())?,
            Codec::default(),
        );

        let (sender, receiver) = channel(100);
        let (sink, stream) = framed.split();
        let sink_future: Box<dyn Future<Item = (), Error = ()>> = Box::new(
            sink.sink_map_err(move |e| {
                panic!("Could not send_all {:?}", e);
            })
            .send_all(receiver.map_err(|e| {
                panic!("Could not receive data from internal receiver: {:?}", e);
            }))
            .map(|_| ()),
        );
        Self::add_stream(stream, ctx);
        ctx.spawn(wrap_future(sink_future));
        self.udp_sender = sender;

        self.tick(ctx);
        ctx.run_interval(Duration::from_secs(1), Self::tick);
        ctx.run_interval(Duration::from_millis(33), Self::render);

        Ok(())
    }
    fn tick(&mut self, _context: &mut Context<Self>) {
        for ip in &mut self.config.broadcasts {
            if let Err(e) = self
                .udp_sender
                .try_send((ArtCommand::Poll(Default::default()), *ip))
            {
                println!("Can not broadcast: {:?}", e);
            }
        }
    }

    fn render(&mut self, _: &mut Context<Self>) {
        for (addr, client) in &mut self.clients {
            let (bytes, fps, frame_count): (Vec<u8>, u8, usize) = match &client.current {
                RenderMode::Color(r, g, b) => (
                    std::iter::repeat([r, g, b].iter())
                        .take(7 * 22)
                        .flatten()
                        .map(|c| **c)
                        .collect(),
                    1,
                    1,
                ),
                RenderMode::Animation(animation_name) => {
                    match self.animations.animations.get(animation_name.as_str()) {
                        Some(anim) => {
                            let frame = anim.frames[client.current_animation_frame];
                            let bytes = frame
                                .iter()
                                .flatten()
                                .flat_map(|(r, g, b)| vec![r, g, b])
                                .copied()
                                .collect();
                            (bytes, anim.fps, anim.frames.len())
                        }
                        None => {
                            client.current_animation_frame = 0;
                            continue;
                        }
                    }
                }
            };
            client.millis_since_last_frame += 33;
            let millis_per_frame = 1000 / usize::from(fps);
            if client.millis_since_last_frame >= millis_per_frame {
                client.millis_since_last_frame -= millis_per_frame;
            } else {
                continue;
            }
            let message = Output {
                data: bytes[12..].to_vec(),
                length: 450,
                ..Output::default()
            };
            assert_eq!(message.length as usize, message.data.len());
            if let Err(e) = self
                .udp_sender
                .try_send((ArtCommand::Output(message), *addr))
            {
                println!("Can not send animation: {:?}", e);
                client.current_animation_frame = 0;
                continue;
            }
            client.current_animation_frame = (client.current_animation_frame + 1) % frame_count;
        }
    }
}

impl Handler<AddAnimation> for Service {
    type Result = <AddAnimation as Message>::Result;

    fn handle(&mut self, animation: AddAnimation, _context: &mut Self::Context) -> Self::Result {
        let name = animation.name;
        println!("Loading {:?} ({} bytes)", name, animation.bytes.len());
        let mut zip = ZipArchive::new(Cursor::new(animation.bytes))?;
        let mut map = HashMap::new();
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            map.insert(file.name().to_owned(), buffer);
        }

        self.animations.load(&name, &map)?;

        let _ = fs::create_dir(&format!("animations/{}", name));
        for (file_name, contents) in map {
            let file_name = format!("animations/{}/{}", name, file_name);
            let mut file = File::create(&file_name)?;
            file.write_all(&contents)?;
        }

        Ok(())
    }
}

impl Handler<RequestAnimationList> for Service {
    type Result = <RequestAnimationList as Message>::Result;

    fn handle(
        &mut self,
        _animation: RequestAnimationList,
        _context: &mut Self::Context,
    ) -> Self::Result {
        let result = ResponseAnimationList {
            animations: self.animations.animations.values().cloned().collect(),
        };
        Ok(result)
    }
}

impl Handler<RequestNodeList> for Service {
    type Result = <RequestNodeList as Message>::Result;

    fn handle(
        &mut self,
        _animation: RequestNodeList,
        _context: &mut Self::Context,
    ) -> Self::Result {
        let result = ResponseNodeList {
            nodes: self.clients.values().map(Client::get_node).collect(),
        };
        Ok(result)
    }
}

impl Handler<SetNodeAnimation> for Service {
    type Result = <SetNodeAnimation as Message>::Result;

    fn handle(
        &mut self,
        animation: SetNodeAnimation,
        _context: &mut Self::Context,
    ) -> Self::Result {
        if self
            .animations
            .animations
            .get(&animation.animation_name)
            .is_none()
        {
            bail!("Animation not found");
        }
        for client in self.clients.values_mut() {
            if client.addr_string == animation.ip {
                client.current = RenderMode::Animation(animation.animation_name);
                client.current_animation_frame = 0;
                client.millis_since_last_frame = 1000;
                return Ok(());
            }
        }
        bail!("Torch with ip {} not found", animation.ip)
    }
}

impl Handler<SetNodeColor> for Service {
    type Result = <SetNodeColor as Message>::Result;

    fn handle(&mut self, color: SetNodeColor, _context: &mut Self::Context) -> Self::Result {
        if color.color_name.len() != 6 {
            bail!("Color invalid, should be 6-character hexadecimal");
        }
        let r: u8 = u8::from_str_radix(&color.color_name[0..2], 16)
            .context("Color invalid, should be 6-character hexadecimal")?;
        let g: u8 = u8::from_str_radix(&color.color_name[2..4], 16)
            .context("Color invalid, should be 6-character hexadecimal")?;
        let b: u8 = u8::from_str_radix(&color.color_name[4..6], 16)
            .context("Color invalid, should be 6-character hexadecimal")?;
        for client in self.clients.values_mut() {
            if client.addr_string == color.ip {
                client.current = RenderMode::Color(r, g, b);
                client.current_animation_frame = 0;
                client.millis_since_last_frame = 1000;
                return Ok(());
            }
        }
        bail!("Torch with ip {} not found", color.ip)
    }
}
