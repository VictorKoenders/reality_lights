use actix::{
    Actor, ActorContext, ArbiterService, AsyncContext, Context, Handler, Message, StreamHandler,
    Supervised,
};
use animation_handler::AnimationHandler;
use artnet::{Client, Codec};
use artnet_protocol::{ArtCommand, Output};
use failure::Error;
use futures::{sink::Wait, Sink, Stream};
use messages::{
    AddAnimation, RequestAnimationList, RequestNodeList, ResponseAnimationList, ResponseNodeList,
    SendMessage, SetNodeAnimation,
};
use std::collections::HashMap;
use std::mem;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket as NetSocket};
use std::time::Duration;
use time;
use tokio::prelude::stream::SplitSink;
use tokio_reactor::Handle;
use tokio_udp::{UdpFramed, UdpSocket};
use Result;

#[derive(Default)]
pub struct Service {
    sink: Option<Wait<SplitSink<UdpFramed<Codec>>>>,
    clients: HashMap<Ipv4Addr, Client>,
    animations: AnimationHandler,
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
        let addr = match addr {
            SocketAddr::V4(v4) => *v4.ip(),
            _ => unreachable!(),
        };
        if !self.clients.contains_key(&addr) {
            if let ArtCommand::PollReply(reply) = &command {
                self.clients.insert(addr, Client::new(reply));
            } else {
                return;
            }
        }

        let client = self.clients.get_mut(&addr).unwrap();
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

        Self::add_stream(stream, ctx);
        self.sink = Some(sink.wait());

        self.broadcast(ctx);
        ctx.run_interval(Duration::from_secs(1), Self::tick);
        ctx.run_interval(Duration::from_millis(100), Self::render);

        Ok(())
    }
    fn tick(&mut self, context: &mut Context<Self>) {
        let remove_time = time::precise_time_s() - 30.;
        self.clients
            .retain(|_, v| v.last_reply_received > remove_time);
        self.broadcast(context);
    }

    fn render(&mut self, _: &mut Context<Self>) {
        for client in self.clients.values_mut() {
            if let Some((index, name)) = &mut client.current_animation {
                if let Some(animation) = self.animations.animations.iter().find(|a| &a.name == name)
                {
                    println!(
                        "Sending animation {} ({}) to {:?}",
                        name, index, client.addr
                    );
                    let index = *index % animation.frames.len();
                    let frame = animation.frames[index];

                    let bytes: [u8; 7 * 3 * 22] = unsafe { mem::transmute(frame) };

                    let message = Output {
                        data: bytes[12..].to_vec(),
                        length: 450,
                        ..Output::default()
                    };
                    assert_eq!(message.length as usize, message.data.len());
                    self.sink
                        .as_mut()
                        .unwrap()
                        .send((
                            ArtCommand::Output(message),
                            SocketAddr::V4(SocketAddrV4::new(
                                Ipv4Addr::new(
                                    client.addr[0],
                                    client.addr[1],
                                    client.addr[2],
                                    client.addr[3],
                                ),
                                6454,
                            )),
                        )).unwrap();
                }
                *index += 1;
            }
        }
    }

    fn broadcast(&mut self, _: &mut Context<Self>) {
        if self.sink.is_none() {
            panic!("Sink is empty");
        }
        let broadcast_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)), 6454);
        self.sink
            .as_mut()
            .unwrap()
            .send((ArtCommand::Poll(Default::default()), broadcast_addr))
            .unwrap();
    }
}

impl Handler<SendMessage> for Service {
    type Result = ();

    fn handle(&mut self, message: SendMessage, _: &mut Context<Self>) {
        if self.sink.is_none() {
            panic!("Sink is empty");
        }
        self.sink
            .as_mut()
            .unwrap()
            .send((message.message, (message.address, 6454).into()))
            .unwrap();
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
        _animation: SetNodeAnimation,
        _context: &mut Self::Context,
    ) -> Self::Result {
    }
}
