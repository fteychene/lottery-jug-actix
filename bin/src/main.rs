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

use actix_web::{App, HttpResponse, http::Method, FutureResponse, State, AsyncResponder, Query};
use actix_web::{http, error};
use tokio::prelude::future::Future;
use tokio::prelude::Stream;
use actix::{prelude::*, Actor, Context, Message, Handler};
use jug_actix_lottery::eventbrite::attendees::load_attendees;
use jug_actix_lottery::eventbrite::events::get_current_event;
use jug_actix_lottery::eventbrite::model::Profile;
use jug_actix_lottery::eventbrite::errors::EventbriteError;
use jug_actix_lottery::lottery::draw;
use core::time;
use actix_web::server::HttpServer;
use tokio::prelude::future;
use tokio::timer::Interval;
use std::time::Instant;
use std::env;
use actix::dev::{MessageResponse, ResponseChannel};


struct AttendeesActor {
    attendees: Option<Vec<Profile>>
}

impl Actor for AttendeesActor {
    type Context = Context<Self>;
}

impl Default for AttendeesActor {
    fn default() -> Self {
        AttendeesActor { attendees: None }
    }
}

struct UpdateAttendees {
    organizer: String,
    token: String,
}

enum UpdateAttendeesResponse {
    Updated,
    NoEventAvailable,
    EventbriteError {
        error: EventbriteError
    },
    UnexpectedError {
        error: failure::Error
    },
}

impl<A, M> MessageResponse<A, M> for UpdateAttendeesResponse
    where
        A: Actor,
        M: Message<Result=UpdateAttendeesResponse>,
{
    fn handle<R: ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}

impl Message for UpdateAttendees {
    type Result = UpdateAttendeesResponse;
}

impl Handler<UpdateAttendees> for AttendeesActor {
    type Result = UpdateAttendeesResponse;

    fn handle(&mut self, msg: UpdateAttendees, _ctx: &mut Context<Self>) -> Self::Result {
        match get_current_event(&msg.organizer, &msg.token)
            .and_then(|event| load_attendees(&event.id, &msg.token)) {
            Ok(attendees) => {
                self.attendees = Some(attendees);
                UpdateAttendeesResponse::Updated
            }
            Err(e) => {
                self.attendees = None;
                match e.downcast::<EventbriteError>() {
                    Ok(error) => match error{
                        EventbriteError::NoEventAvailable => UpdateAttendeesResponse::NoEventAvailable,
                        other_eventbrite_error => UpdateAttendeesResponse::EventbriteError { error: other_eventbrite_error }
                    },
                    Err(error) => UpdateAttendeesResponse::UnexpectedError { error: error }
                }
            }
        }
    }
}

struct GetAttendees {
    nb: i8
}

impl Message for GetAttendees {
    type Result = Result<Vec<Profile>, WinnerError>;
}

impl Handler<GetAttendees> for AttendeesActor {
    type Result = Result<Vec<Profile>, WinnerError>;

    fn handle(&mut self, msg: GetAttendees, _ctx: &mut Context<Self>) -> Self::Result {
        self.attendees.as_ref()
            .ok_or(WinnerError::NoEventAvailable)
            .and_then(|ref attendees| draw(msg.nb, attendees).map_err(|error| WinnerError::DrawError { cause: error }))
            .map(|attendees| attendees.into_iter().map(|r| r.clone()).collect())
    }
}

struct WebState {
    attendees: Addr<AttendeesActor>
}

// WINNER handler

#[derive(Fail, Debug)]
enum WinnerError {
    #[fail(display = "Invalid parameter")]
    InvalidParameter,
    #[fail(display = "No event available")]
    NoEventAvailable,
    #[fail(display = "Error during attendees draw")]
    DrawError { cause: failure::Error },
    #[fail(display = "Unexpected error")]
    UnexpectedError { cause: failure::Error },
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

fn main() {
    env_logger::init();
    let organizer = env::var("ORGANIZER_TOKEN").expect("ORGANIZER_TOKEN is mandatory");
    let token = env::var("EVENTBRITE_TOKEN").expect("EVENTBRITE_TOKEN is mandatory");

    info!("Starting lottery ! ")
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