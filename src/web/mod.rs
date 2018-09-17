use actix::{Addr, MailboxError, Recipient};
use actix_web::server::Server;
use actix_web::{server, App, HttpRequest, Json};
use futures::Future;
use messages::{
    AddAnimation, Animation, Node, RequestAnimationList, RequestNodeList, SetNodeAnimation,
};
use service;

pub struct ServerState {
    pub request_node_list: Recipient<RequestNodeList>,
    pub request_animation_list: Recipient<RequestAnimationList>,
    pub add_animation: Recipient<AddAnimation>,
    pub set_node_animation: Recipient<SetNodeAnimation>,
}

pub type Result<T> = Box<Future<Item = ::Result<T>, Error = MailboxError>>;

fn handler_request_node_list(req: HttpRequest<ServerState>) -> Result<Json<Vec<Node>>> {
    Box::new(
        req.state()
            .request_node_list
            .send(RequestNodeList)
            .map(|response| response.map(|r| Json(r.nodes))),
    )
}
fn handler_request_animation_list(req: HttpRequest<ServerState>) -> Result<Json<Vec<Animation>>> {
    Box::new(
        req.state()
            .request_animation_list
            .send(RequestAnimationList)
            .map(|response| response.map(|r| Json(r.animations))),
    )
}

pub fn run(addr: &Addr<service::Service>) -> Addr<Server> {
    let addr = addr.clone();
    server::new(move || {
        let request_node_list = addr.clone().recipient();
        let request_animation_list = addr.clone().recipient();
        let add_animation = addr.clone().recipient();
        let set_node_animation = addr.clone().recipient();
        App::with_state(ServerState {
            request_node_list,
            request_animation_list,
            add_animation,
            set_node_animation,
        }).resource("/api/animations", |r| {
            r.with_async(handler_request_animation_list)
        }).resource("/api/nodes", |r| r.with_async(handler_request_node_list))
    }).bind("0.0.0.0:8001")
    .expect("Could not bind to 0.0.0.0:8001")
    .start()
}
