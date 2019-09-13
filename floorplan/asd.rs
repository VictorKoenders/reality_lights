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
        None => Either::B(err(Err(warp::reject::custom(
            "Torch is not mapped to an IP",
        )))),
    };
    fut.map(|f: String| f)
},

    Checking floorplan v0.1.0 (/home/trangar/development/rust/reality_lights/floorplan)
error[E0599]: no method named `map` found for type `futures::future::either::Either<futures::future::map_err::MapErr<futures::future::and_then::AndThen<impl warp::Future, impl warp::Future, [closure@src/main.rs:81:39: 81:59]>, fn(reqwest::error::Error) -> warp::Rejection {warp::reject::custom::<reqwest::error::Error>}>, futures::future::result_::FutureResult<_, std::result::Result<_, warp::Rejection>>>` in the current scope
  --> src/main.rs:88:21
   |
88 |                 fut.map(|f: String| f)
   |                     ^^^
   |
   = note: the method `map` exists but the following trait bounds were not satisfied:
           `&futures::future::either::Either<futures::future::map_err::MapErr<futures::future::and_then::AndThen<impl warp::Future, impl warp::Future, [closure@src/main.rs:81:39: 81:59]>, fn(reqwest::error::Error) -> warp::Rejection {warp::reject::custom::<reqwest::error::Error>}>, futures::future::result_::FutureResult<_, std::result::Result<_, warp::Rejection>>> : warp::Filter`
           `&mut futures::future::either::Either<futures::future::map_err::MapErr<futures::future::and_then::AndThen<impl warp::Future, impl warp::Future, [closure@src/main.rs:81:39: 81:59]>, fn(reqwest::error::Error) -> warp::Rejection {warp::reject::custom::<reqwest::error::Error>}>, futures::future::result_::FutureResult<_, std::result::Result<_, warp::Rejection>>> : std::iter::Iterator`
           `&mut futures::future::either::Either<futures::future::map_err::MapErr<futures::future::and_then::AndThen<impl warp::Future, impl warp::Future, [closure@src/main.rs:81:39: 81:59]>, fn(reqwest::error::Error) -> warp::Rejection {warp::reject::custom::<reqwest::error::Error>}>, futures::future::result_::FutureResult<_, std::result::Result<_, warp::Rejection>>> : warp::Filter`
           `&mut futures::future::either::Either<futures::future::map_err::MapErr<futures::future::and_then::AndThen<impl warp::Future, impl warp::Future, [closure@src/main.rs:81:39: 81:59]>, fn(reqwest::error::Error) -> warp::Rejection {warp::reject::custom::<reqwest::error::Error>}>, futures::future::result_::FutureResult<_, std::result::Result<_, warp::Rejection>>> : warp::Future`
           `futures::future::either::Either<futures::future::map_err::MapErr<futures::future::and_then::AndThen<impl warp::Future, impl warp::Future, [closure@src/main.rs:81:39: 81:59]>, fn(reqwest::error::Error) -> warp::Rejection {warp::reject::custom::<reqwest::error::Error>}>, futures::future::result_::FutureResult<_, std::result::Result<_, warp::Rejection>>> : warp::Filter`
           `futures::future::either::Either<futures::future::map_err::MapErr<futures::future::and_then::AndThen<impl warp::Future, impl warp::Future, [closure@src/main.rs:81:39: 81:59]>, fn(reqwest::error::Error) -> warp::Rejection {warp::reject::custom::<reqwest::error::Error>}>, futures::future::result_::FutureResult<_, std::result::Result<_, warp::Rejection>>> : warp::Future`

error: aborting due to previous error

For more information about this error, try `rustc --explain E0599`.
error: Could not compile `floorplan`.

To learn more, run the command again with --verbose.
