use actix::fut::wrap_future;
use actix::{
    Actor, ActorContext, ArbiterService, AsyncContext, Context, Handler, Message, StreamHandler,
    Supervised,
};
use animation_handler::AnimationHandler;
use artnet::{Client, Codec};
use artnet_protocol::{ArtCommand, Output};
use failure::Error;
use futures::sync::mpsc::{channel, Sender};
use futures::{Future, Sink, Stream};
use messages::{
    AddAnimation, RequestAnimationList, RequestNodeList, ResponseAnimationList, ResponseNodeList,
    SendMessage, SetNodeAnimation,
};
use std::collections::HashMap;
use std::mem;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket as NetSocket};
use std::time::Duration;
use time;
use tokio_reactor::Handle;
use tokio_udp::{UdpFramed, UdpSocket};
use Result;

pub struct Service {
    udp_sender: Sender<(ArtCommand, SocketAddr)>,
    clients: HashMap<SocketAddr, Client>,
    animations: AnimationHandler,
}

impl Default for Service {
    fn default() -> Service {
        Service {
            udp_sender: channel(0).0,
            clients: HashMap::new(),
            animations: AnimationHandler::new().expect("Cannot load animation handler"),
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
                self.clients.insert(addr, Client::new(addr, reply));
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
        let socket = NetSocket::bind((Ipv4Addr::new(0, 0, 0, 0), 6454))?;
        socket.set_broadcast(true)?;
        let framed = UdpFramed::new(
            UdpSocket::from_std(socket, &Handle::default())?,
            Codec::default(),
        );

        let (sink, stream) = framed.split();
        let (sender, receiver) = channel(100);

        Self::add_stream(stream, ctx);
        self.udp_sender = sender;
        let sink_future: Box<Future<Item = (), Error = ()>> = Box::new(
            sink.sink_map_err(|e| {
                panic!("Could not send_all: {:?}", e);
            }).send_all(receiver.map_err(|e| {
                panic!("Could not receive data from internal receiver: {:?}", e);
            })).map(|_| ()),
        );

        ctx.spawn(wrap_future(sink_future));

        self.tick(ctx);
        ctx.run_interval(Duration::from_secs(1), Self::tick);
        ctx.run_interval(Duration::from_millis(100), Self::render);

        Ok(())
    }
    fn tick(&mut self, _context: &mut Context<Self>) {
        let remove_time = time::precise_time_s() - 30.;
        self.clients
            .retain(|_, v| v.last_reply_received > remove_time);
        let broadcast_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)), 6454);
        self.udp_sender
            .try_send((ArtCommand::Poll(Default::default()), broadcast_addr))
            .expect("Can not broadcast");
    }

    fn render(&mut self, _: &mut Context<Self>) {
        for (addr, client) in &mut self.clients {
            if let Some(animation) = self
                .animations
                .animations
                .iter()
                .find(|a| a.name == client.current_animation)
            {
                let frame = animation.frames[client.current_animation_frame];

                let bytes: [u8; 7 * 3 * 22] = unsafe { mem::transmute(frame) };

                let message = Output {
                    data: bytes[12..].to_vec(),
                    length: 450,
                    ..Output::default()
                };
                assert_eq!(message.length as usize, message.data.len());
                self.udp_sender
                    .try_send((ArtCommand::Output(message), *addr))
                    .expect("Can not send animation");
                client.current_animation_frame =
                    (client.current_animation_frame + 1) % animation.frames.len();
            } else {
                client.current_animation_frame = 0;
            }
        }
    }
}

impl Handler<SendMessage> for Service {
    type Result = ();

    fn handle(&mut self, message: SendMessage, _: &mut Context<Self>) {
        self.udp_sender
            .try_send((message.message, (message.address, 6454).into()))
            .expect("Could not send SendMessage");
    }
}

impl Handler<AddAnimation> for Service {
    type Result = <AddAnimation as Message>::Result;

    fn handle(&mut self, _animation: AddAnimation, _context: &mut Self::Context) -> Self::Result {}
}

impl Handler<RequestAnimationList> for Service {
    type Result = <RequestAnimationList as Message>::Result;

    fn handle(
        &mut self,
        _animation: RequestAnimationList,
        _context: &mut Self::Context,
    ) -> Self::Result {
        let result = ResponseAnimationList {
            animations: self.animations.animations.clone(),
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
        for client in self.clients.values_mut() {
            if client.addr_string == animation.ip {
                client.current_animation = animation.animation_name;
                client.current_animation_frame = 0;
                return Ok(());
            }
        }
        bail!("Client not found")
    }
}
