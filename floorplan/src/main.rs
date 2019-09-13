#[macro_use]
extern crate warp;

mod config;

use crate::config::*;
use futures::future::{err, Either, Future};
use parking_lot::RwLock;
use std::sync::Arc;
use warp::Filter;

fn main() {
    let config = Arc::new(RwLock::new(
        Config::load().expect("Could not reload config"),
    ));
    let index = warp::path::end().and(warp::fs::file("static/index.html"));

    let api_mapping = path!("api" / "mapping");
    let api_mapping_get = {
        let config = config.clone();
        api_mapping.and(warp::path::end()).and(warp::get2()).map(
            move || match serde_json::to_string(&*config.read()) {
                Ok(c) => c,
                Err(e) => format!("Could not load config: {:?}", e),
            },
        )
    };

    let api_mapping_post = {
        let config = config.clone();
        api_mapping
            .and(warp::post2())
            .and(warp::body::json())
            .map(move |new_config: Config| {
                println!("{:?}", new_config);
                *config.write() = new_config;
                format!("POST mapping")
            })
    };

    let api_animations = proxy(
        path!("api" / "animations"),
        "http://localhost:6454/api/animations",
    );

    let api_nodes = proxy(path!("api" / "nodes"), "http://localhost:6454/api/nodes");

    let api_animation_put = api_animation_put(config.clone(), path!("api" / "set"));
    warp::serve(
        index
            .or(api_mapping_get)
            .or(api_mapping_post)
            .or(api_animations)
            .or(api_animation_put)
            .or(api_nodes),
    )
    .run(([0, 0, 0, 0], 6464));
}

fn api_animation_put(
    config: Arc<RwLock<Config>>,
    path: impl Filter<Extract = (), Error = warp::Rejection> + Clone,
) -> impl Filter<Extract = (String,), Error = warp::Rejection> + Clone {
    let client = reqwest::r#async::Client::new();
    path.and(warp::path::param())
        .and(warp::path::param())
        .and(warp::path::param())
        .and(warp::path::path("a"))
        .and(warp::path::param())
        .and(warp::get2())
        .and_then(
            move |color: String, row: u8, side: String, animation: String| {
                let fut: Either<_, _> = match config.read().get_ip(&color, row, &side) {
                    Some(ip) => Either::A(
                        client
                            .get(&format!(
                                "http://localhost:6454/api/set_animation/{}/{}",
                                ip, animation
                            ))
                            .send()
                            .and_then(|mut res| res.text())
                            .map_err(warp::reject::custom),
                    ),
                    None => Either::B(err(warp::reject::custom("Torch is not mapped to an IP"))),
                };
                fut.map(|f: String| f)
            },
        )
        .map(|text| text)
}

fn proxy(
    from: impl Filter<Extract = (), Error = warp::Rejection> + Clone,
    to: &'static str,
) -> impl Filter<Extract = (String,), Error = warp::Rejection> + Clone {
    let client = reqwest::r#async::Client::new();
    from.and_then(move || {
        client
            .get(to)
            .send()
            .and_then(|mut res| res.text())
            .map_err(warp::reject::custom)
    })
    .map(|text| text)
}
