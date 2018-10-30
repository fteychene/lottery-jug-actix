use jug_actix_lottery::eventbrite::model::Profile;
use jug_actix_lottery::eventbrite::errors::EventbriteError;
use jug_actix_lottery::eventbrite::attendees::load_attendees;
use jug_actix_lottery::eventbrite::events::get_current_event;
use jug_actix_lottery::lottery::draw;
use actix::{Actor, Context, Message, Handler};
use actix::dev::{MessageResponse, ResponseChannel};
use super::message::{GetAttendees, UpdateAttendeesResponse, UpdateAttendees};
use errors::WinnerError;

pub struct AttendeesActor {
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

impl Message for UpdateAttendees {
    type Result = UpdateAttendeesResponse;
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