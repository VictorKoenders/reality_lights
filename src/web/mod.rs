use actix::{Addr, Recipient};
use actix_web::dev::Payload;
use actix_web::fs::NamedFile;
use actix_web::http::header::{ContentDisposition, DispositionParam};
use actix_web::http::Method;
use actix_web::multipart::MultipartItem;
use actix_web::server::Server;
use actix_web::{server, App, FromRequest, HttpMessage, HttpRequest, Json, Path};
use bytes::Bytes;
use failure::Error;
use futures::{future, Future, Stream};
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

impl ServerState {
    pub fn new(addr: &Addr<service::Service>) -> Self {
        let request_node_list = addr.clone().recipient();
        let request_animation_list = addr.clone().recipient();
        let add_animation = addr.clone().recipient();
        let set_node_animation = addr.clone().recipient();
        ServerState {
            request_node_list,
            request_animation_list,
            add_animation,
            set_node_animation,
        }
    }
}

pub type Result<T> = Box<Future<Item = ::Result<T>, Error = Error>>;

fn index(_req: &HttpRequest<ServerState>) -> ::std::result::Result<NamedFile, Error> {
    Ok(NamedFile::open("static/index.html")?)
}

fn handler_request_node_list(req: &HttpRequest<ServerState>) -> Result<Json<Vec<Node>>> {
    Box::new(
        req.state()
            .request_node_list
            .send(RequestNodeList)
            .map(|response| response.map(|r| Json(r.nodes)))
            .map_err(Into::into),
    )
}
fn handler_request_animation_list(req: &HttpRequest<ServerState>) -> Result<Json<Vec<Animation>>> {
    Box::new(
        req.state()
            .request_animation_list
            .send(RequestAnimationList)
            .map(|response| response.map(|r| Json(r.animations)))
            .map_err(Into::into),
    )
}

fn handler_set_node_animation(req: &HttpRequest<ServerState>) -> Result<&'static str> {
    let (ip, animation_name) = match Path::<(String, String)>::extract(req) {
        Ok(p) => p.into_inner(),
        Err(e) => {
            return Box::new(future::err(format_err!("Could not get params: {:?}", e)));
        }
    };
    println!("Setting {:?} to {:?}", ip, animation_name);
    Box::new(
        req.state()
            .set_node_animation
            .send(SetNodeAnimation { ip, animation_name })
            .map(|v| v.map(|_| "ok"))
            .map_err(Into::into),
    )
}

pub fn handle_multipart_item(
    item: MultipartItem<Payload>,
) -> Box<Stream<Item = (Option<ContentDisposition>, Bytes), Error = Error>> {
    match item {
        MultipartItem::Field(field) => {
            let content_disposition = field.content_disposition();
            Box::new(
                field
                    .map(move |f| (content_disposition.clone(), f))
                    .map_err(Error::from),
            )
        }
        MultipartItem::Nested(mp) => {
            println!("Found nested");
            Box::new(mp.map_err(Error::from).map(handle_multipart_item).flatten())
        }
    }
}

fn handler_add_animation(req: &HttpRequest<ServerState>) -> Result<String> {
    let req = req.clone();
    let animation_name = match Path::<String>::extract(&req) {
        Ok(p) => p.into_inner(),
        Err(e) => {
            return Box::new(future::err(format_err!("Could not get params: {:?}", e)));
        }
    };
    Box::new(Box::new(
        req.multipart()
            .map_err(Error::from)
            .map(handle_multipart_item)
            .flatten()
            .filter_map(|(disposition, bytes)| {
                let disposition = match disposition {
                    Some(d) => d,
                    None => return None,
                };
                for param in disposition.parameters {
                    if let DispositionParam::Filename(_name) = param {
                        return Some(bytes);
                    }
                }
                None
            }).collect()
            .and_then(move |file| {
                let file = file
                    .iter()
                    .flat_map(|b| b.as_ref())
                    .cloned()
                    .collect::<Vec<u8>>();
                req.state()
                    .add_animation
                    .send(AddAnimation {
                        name: animation_name,
                        bytes: file,
                    }).map(|e| e.map(|_| String::from("ok")))
                    .map_err(Error::from)
            }).map_err(|e| {
                println!("failed: {}", e);
                e
            }),
    ))
}

pub fn run(addr: &Addr<service::Service>) -> Addr<Server> {
    let addr = addr.clone();
    server::new(move || {
        App::with_state(ServerState::new(&addr))
            .resource("/", |r| r.method(Method::GET).f(index))
            .resource("/api/nodes", |r| r.route().a(handler_request_node_list))
            .resource("/api/animations", |r| {
                r.route().a(handler_request_animation_list)
            }).resource("/api/set_animation/{ip:[\\w\\.]+}/{animation}", |r| {
                r.route().a(handler_set_node_animation)
            }).resource("/api/animation/{name}", |r| {
                r.route().a(handler_add_animation)
            })
    }).bind("0.0.0.0:8001")
    .expect("Could not bind to 0.0.0.0:8001")
    .start()
}
