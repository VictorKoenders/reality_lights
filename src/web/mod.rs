use crate::config::Config;
use crate::messages::{AddAnimation, RequestAnimationList, RequestNodeList, SetNodeAnimation, SetNodeColor};
use crate::service;
use actix::{Addr, Recipient};
use actix_files::NamedFile;
use actix_http::http;
use actix_multipart::Multipart;
use actix_web::dev::Server;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use failure::Error;
use futures::{future, Future, Stream};
use serde::Serialize;
use serde_json;

pub struct ServerState {
    pub request_node_list: Recipient<RequestNodeList>,
    pub request_animation_list: Recipient<RequestAnimationList>,
    pub add_animation: Recipient<AddAnimation>,
    pub set_node_animation: Recipient<SetNodeAnimation>,
    pub set_node_color: Recipient<SetNodeColor>,
}

impl ServerState {
    pub fn new(addr: &Addr<service::Service>) -> Self {
        let request_node_list = addr.clone().recipient();
        let request_animation_list = addr.clone().recipient();
        let add_animation = addr.clone().recipient();
        let set_node_animation = addr.clone().recipient();
        let set_node_color = addr.clone().recipient();
        ServerState {
            request_node_list,
            request_animation_list,
            add_animation,
            set_node_animation,
            set_node_color,
        }
    }
}
type Response = Box<dyn Future<Item = HttpResponse, Error = Error>>;

fn index(_req: HttpRequest) -> ::std::result::Result<NamedFile, Error> {
    Ok(NamedFile::open("static/index.html")?)
}

fn json(t: impl Serialize) -> HttpResponse {
    HttpResponse::Ok().body(serde_json::to_string_pretty(&t).expect("Could not serialize"))
}
fn str(str: String) -> HttpResponse {
    HttpResponse::Ok().body(str)
}

fn err(e: &Error) -> HttpResponse {
    eprintln!("{:?}", e);
    HttpResponse::BadRequest().body(e.to_string())
}

fn handler_request_node_list(req: HttpRequest) -> Response {
    Box::new(
        req.app_data::<ServerState>()
            .unwrap()
            .request_node_list
            .send(RequestNodeList)
            .map(|response| match response {
                Ok(r) => json(r.nodes),
                Err(e) => err(&e),
            })
            .or_else(|e| Ok(err(&e.into()))),
    )
}
fn handler_request_animation_list(req: HttpRequest) -> Response {
    Box::new(
        req.app_data::<ServerState>()
            .unwrap()
            .request_animation_list
            .send(RequestAnimationList)
            .map(|response| match response {
                Ok(r) => json(r.animations),
                Err(e) => err(&e),
            })
            .or_else(|e| Ok(err(&e.into()))),
    )
}

fn handler_set_node_animation(
    (req, param): (HttpRequest, web::Path<(String, String)>),
) -> Response {
    let ip = param.0.clone();
    let animation_name = param.1.clone();
    Box::new(
        req.app_data::<ServerState>()
            .unwrap()
            .set_node_animation
            .send(SetNodeAnimation { ip, animation_name })
            .map(|v| match v {
                Ok(_) => str(String::from("ok")),
                Err(e) => err(&e),
            })
            .or_else(|e| Ok(err(&e.into()))),
    )
}


fn handler_set_node_color(
    (req, param): (HttpRequest, web::Path<(String, String)>),
) -> Response {
    let ip = param.0.clone();
    let color_name = param.1.clone();
    Box::new(
        req.app_data::<ServerState>()
            .unwrap()
            .set_node_color
            .send(SetNodeColor { ip, color_name })
            .map(|v| match v {
                Ok(_) => str(String::from("ok")),
                Err(e) => err(&e),
            })
            .or_else(|e| Ok(err(&e.into()))),
    )
}

#[derive(Debug)]
enum UploadItem {
    Form { name: String, value: String },
    File { data: Vec<u8> },
}

impl UploadItem {
    pub fn get_formdata_name(&self) -> Option<&str> {
        match self {
            UploadItem::Form { value, .. } => Some(value),
            _ => None,
        }
    }

    pub fn get_file_data(&self) -> Option<&[u8]> {
        match self {
            UploadItem::File { data } => Some(&data),
            _ => None,
        }
    }
}

fn map_multipart_field(
    field: actix_multipart::Field,
) -> Box<dyn Future<Item = Option<UploadItem>, Error = Error>> {
    match field.content_disposition() {
        None => Box::new(future::ok(None)),
        Some(http::header::ContentDisposition {
            disposition: http::header::DispositionType::FormData,
            parameters,
        }) => {
            let item = if let Some(http::header::DispositionParam::Filename(_)) = parameters.get(1)
            {
                UploadItem::File { data: Vec::new() }
            } else if let Some(http::header::DispositionParam::Name(name)) = parameters.get(0) {
                UploadItem::Form {
                    name: name.clone(),
                    value: String::new(),
                }
            } else {
                return Box::new(futures::future::ok(None));
            };
            Box::new(
                field
                    .map_err(|e| format_err!("Multipart field error: {:?}", e))
                    .fold(item, |mut item, chunk| {
                        match &mut item {
                            UploadItem::File { data } => data.extend_from_slice(&chunk),
                            UploadItem::Form { value, .. } => {
                                if let Ok(val) = std::str::from_utf8(&chunk) {
                                    *value += val;
                                }
                            }
                        }
                        Ok::<_, Error>(item)
                    })
                    .map(Some),
            )
        }
        x => {
            eprintln!("Unknown multipart item: {:?}", x);
            Box::new(futures::future::ok(None))
        }
    }
}

fn handler_add_animation(
    (req, multipart, _animation_name): (HttpRequest, Multipart, web::Path<String>),
) -> Response {
    Box::new(
        multipart
            .map_err(|e| format_err!("Multipart error: {:?}", e))
            .and_then(map_multipart_field)
            .filter_map(|e| e)
            .collect()
            .map(move |params| {
                let name = params.iter().find_map(|f| f.get_formdata_name());
                let data = params.iter().find_map(|f| f.get_file_data());

                if let (Some(name), Some(data)) = (name, data) {
                    future::Either::A(
                        req.app_data::<ServerState>()
                            .unwrap()
                            .add_animation
                            .send(AddAnimation {
                                name: name.to_owned(),
                                bytes: data.to_vec(),
                            })
                            .map(|e| match e {
                                Ok(_) => str(String::from("ok")),
                                Err(e) => err(&e),
                            })
                            .or_else(|e| future::ok(err(&e.into()))),
                    )
                } else {
                    future::Either::B(future::ok(str("Invalid form data".to_owned())))
                }
            })
            .and_then(|e| e),
    )
}

pub fn run(addr: &Addr<service::Service>) -> Server {
    let config = Config::from_file("config.json").expect("Could not load config");
    let addr = addr.clone();
    let result = HttpServer::new(move || {
        App::new()
            .data(ServerState::new(&addr))
            .service(web::resource("/").to(index))
            .service(web::resource("/api/nodes").to(handler_request_node_list))
            .service(web::resource("/api/animations").to(handler_request_animation_list))
            .service(
                web::resource("/api/set_animation/{ip:[\\w\\.]+}/{animation}")
                    .to(handler_set_node_animation),
            )
            .service(
                web::resource("/api/set_color/{ip:[\\w\\.]+}/{color}").to(handler_set_node_color),
            )
            .service(web::resource("/api/animation/{name}").to(handler_add_animation))
    })
    .bind(config.web_endpoint)
    .expect("Could not bind web API")
    .start();

    println!("Server running on {}", config.web_endpoint);
    result
}
