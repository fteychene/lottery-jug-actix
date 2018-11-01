#![allow(proc_macro_derive_resolution_fallback)] // Diesel compilation warning, should be fixed in diesel 1.4 TODO update diesel to 1.4 when released and remove this configuration

extern crate actix;
extern crate actix_web;
extern crate tokio;
extern crate jug_actix_lottery;
#[macro_use]
extern crate failure_derive;
extern crate failure;
extern crate core;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate diesel;
extern crate uuid;
extern crate r2d2;

mod attendees;
mod errors;
mod record;

use actix::prelude::{System, Arbiter, Addr, Actor};
use attendees::actor::LotteryCache;
use attendees::message::{GetAttendees, GetEvent};
use attendees::cache_loop::cache_update_interval;
use errors::LotteryError;
use tokio::prelude::future;
use tokio::prelude::future::Future;

use actix_web::{App, HttpResponse, FutureResponse, State, AsyncResponder, Query, Path, Json};
use actix_web::{http, error, middleware};
use actix_web::server::HttpServer;
use std::env;

use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use record::db::{DbExecutor, CreateWinner};
use actix::SyncArbiter;

struct WebState {
    cache: Addr<LotteryCache>,
    db: Addr<DbExecutor>,
}

impl error::ResponseError for LotteryError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            LotteryError::InvalidParameter => HttpResponse::new(http::StatusCode::BAD_REQUEST),
            LotteryError::NoEventAvailable => HttpResponse::with_body(http::StatusCode::SERVICE_UNAVAILABLE, "No event available on eventbrite"),
            LotteryError::DrawError { cause: ref e } => HttpResponse::with_body(http::StatusCode::BAD_REQUEST, format!("{}", e)),
            LotteryError::UnexpectedError { cause: ref e } => HttpResponse::with_body(http::StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
        }
    }
}

#[derive(Deserialize)]
struct WinnerQuery {
    nb: i8
}

fn winner_handler((state, query): (State<WebState>, Query<WinnerQuery>)) -> FutureResponse<HttpResponse, LotteryError> {
    match query.nb {
        nb if nb < 0 => Box::new(future::err(LotteryError::InvalidParameter)),
        _ => state.cache.send(GetAttendees { nb: query.nb })
            .map_err(|error| LotteryError::UnexpectedError { cause: error.into() })
            .and_then(|result| result)
            .and_then(|res| Ok(HttpResponse::Ok().json(res)))
            .responder()
    }
}

/// Async request handler
fn record_winner_handler(
    (winner, state): (Json<CreateWinner>, State<WebState>),
) -> FutureResponse<HttpResponse> {
    state.cache.send(GetEvent{})
        .and_then(move |event| {
            let mut  winner = winner.into_inner();
            winner.event_id = event.map(|event| event.id).ok();
            state.db.send(winner)
        })
        .from_err()
        .and_then(|res| match res {
            Ok(user) => Ok(HttpResponse::Ok().json(user)),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}

fn main() {
    env_logger::init();
    let organizer = env::var("ORGANIZER_TOKEN").expect("ORGANIZER_TOKEN is mandatory");
    let token = env::var("EVENTBRITE_TOKEN").expect("EVENTBRITE_TOKEN is mandatory");

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL env var is mandatory");

    info!("Starting lottery ! ");
    let system = System::new("lottery");

    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let db_addr = SyncArbiter::start(3, move || DbExecutor(pool.clone()));

//    Arbiter::spawn_fn(move || {
//        db_addr
//            .send(CreateWinner {
//                first_name: "Francois".to_owned(),
//                last_name: "Teychene".to_owned(),
//            })
//            .map_err(|err| eprintln!("Error : {}", err))
//            .map(|res| match res {
//                Ok(user) => println!("{:?}", user),
//                Err(err) => eprintln!("Error : {}", err),
//            })
//    });

    let addr = LotteryCache::default().start();
    Arbiter::spawn(cache_update_interval(10, addr.clone(), token.clone(), organizer.clone()));

    let http_bind = env::var("HTTP_BIND").unwrap_or("0.0.0.0".to_string());
    let http_port = env::var("HTTP_PORT").unwrap_or("8088".to_string());
    let addr_cloned = addr.clone();
    HttpServer::new(move ||
        App::with_state(WebState { cache: addr_cloned.clone(), db: db_addr.clone() })
            .middleware(middleware::Logger::default())
            .resource("/winners", |r| r.method(http::Method::GET).with(winner_handler))
            .resource("/record", |r| r.method(http::Method::POST).with(record_winner_handler)))
        .bind(format!("{}:{}", http_bind, http_port))
        .unwrap()
        .start();

    system.run();
}