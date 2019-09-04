#![cfg_attr(not(debug_assertions), allow(warnings))]

#[macro_use]
extern crate failure;

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
    system.run().expect("Actix system crashed");
}
