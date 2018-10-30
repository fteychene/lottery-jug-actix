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

mod attendees;
mod errors;
mod cache_loop;

use actix::prelude::{System, Arbiter, Addr, Actor};
use attendees::actor::AttendeesActor;
use attendees::message::{GetAttendees};
use errors::WinnerError;
use tokio::prelude::future;
use tokio::prelude::future::Future;

use actix_web::{App, HttpResponse, http::Method, FutureResponse, State, AsyncResponder, Query};
use actix_web::{http, error};
use actix_web::server::HttpServer;
use std::env;

struct WebState {
    attendees: Addr<AttendeesActor>
}

impl error::ResponseError for WinnerError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            WinnerError::InvalidParameter => HttpResponse::new(http::StatusCode::BAD_REQUEST),
            WinnerError::NoEventAvailable => HttpResponse::with_body(http::StatusCode::SERVICE_UNAVAILABLE, "No event available on eventbrite"),
            WinnerError::DrawError { cause: ref e } => HttpResponse::with_body(http::StatusCode::BAD_REQUEST, format!("{}", e)),
            WinnerError::UnexpectedError { cause: ref e } => HttpResponse::with_body(http::StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
        }
    }
}

#[derive(Deserialize)]
struct WinnerQuery {
    nb: i8
}

fn winner_handler((state, query): (State<WebState>, Query<WinnerQuery>)) -> FutureResponse<HttpResponse, WinnerError> {
    match query.nb {
        nb if nb < 0 => Box::new(future::err(WinnerError::InvalidParameter)),
        _ => state.attendees.send(GetAttendees { nb: query.nb })
            .map_err(|error| WinnerError::UnexpectedError { cause: error.into() })
            .and_then(|result| result)
            .and_then(|res| Ok(HttpResponse::Ok().json(res)))
            .responder()
    }
}

fn main() {
    env_logger::init();
    let organizer = env::var("ORGANIZER_TOKEN").expect("ORGANIZER_TOKEN is mandatory");
    let token = env::var("EVENTBRITE_TOKEN").expect("EVENTBRITE_TOKEN is mandatory");

    info!("Starting lottery ! ");
    let system = System::new("lottery");

    let addr = AttendeesActor::default().start();
    Arbiter::spawn(cache_loop::cache_update_interval(10, addr.clone(), token.clone(), organizer.clone()));

    let addr_cloned = addr.clone();
    HttpServer::new(move ||
        App::with_state(WebState { attendees: addr_cloned.clone() }).resource("/winners", |r| r.method(Method::GET).with(winner_handler)))
        .bind("127.0.0.1:8088")
        .unwrap()
        .start();

    system.run();
}