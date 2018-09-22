extern crate actix;
extern crate actix_web;
extern crate artnet_protocol;
extern crate bytes;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate serde;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_io;
extern crate tokio_reactor;
extern crate tokio_tcp;
extern crate tokio_udp;
#[macro_use]
extern crate serde_derive;
extern crate image;
extern crate serde_json;
extern crate time;
extern crate zip;

pub type Result<T> = std::result::Result<T, failure::Error>;

mod animation_handler;
mod artnet;
mod config;
mod messages;
mod service;
mod web;

use actix::{ArbiterService, System};

fn main() {
    let system = System::new("TR");
    let artnet = service::Service::start_service();
    let _addr = web::run(&artnet);
    system.run();
}
