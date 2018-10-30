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

use actix::prelude::{System, Arbiter, Addr, Actor};
use attendees::actor::AttendeesActor;
use attendees::message::{GetAttendees, UpdateAttendees, UpdateAttendeesResponse};
use errors::WinnerError;

use actix_web::{App, HttpResponse, http::Method, FutureResponse, State, AsyncResponder, Query};
use actix_web::{http, error};
use tokio::prelude::future::Future;
use tokio::prelude::Stream;
use core::time;
use actix_web::server::HttpServer;
use tokio::prelude::future;
use tokio::timer::Interval;
use std::time::Instant;
use std::env;

fn cache_update_interval(duration: u64, addr: Addr<AttendeesActor>, token: String, organizer: String) -> impl Future<Item=(), Error=()> + 'static {
    Interval::new(Instant::now(), time::Duration::from_secs(duration))
        .then(move |_instant| addr.send(UpdateAttendees { token: token.clone(), organizer: organizer.clone() })
            .map_err(|err| eprintln!("Error on sending update message : {}", err)))
        .for_each(move |res| {
            match res {
                UpdateAttendeesResponse::Updated => info!("Attendees cache updated"),
                UpdateAttendeesResponse::NoEventAvailable => info!("No event available on eventbrite"),
                UpdateAttendeesResponse::EventbriteError {error :ref e} => info!("Error on eventbrite : {}", e),
                UpdateAttendeesResponse::UnexpectedError {error: ref e} => error!("Unexpected error on update attendees \n{:?}", e)
            };
            Ok(())
        })
}

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
    Arbiter::spawn(cache_update_interval(10, addr.clone(), token.clone(), organizer.clone()));

    let addr_cloned = addr.clone();
    HttpServer::new(move ||
        App::with_state(WebState { attendees: addr_cloned.clone() }).resource("/winners", |r| r.method(Method::GET).with(winner_handler)))
        .bind("127.0.0.1:8088")
        .unwrap()
        .start();

    system.run();
}