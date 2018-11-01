use jug_actix_lottery::eventbrite::errors::EventbriteError;

pub struct UpdateAttendees {
    pub organizer: String,
    pub token: String,
}

pub enum UpdateAttendeesResponse {
    Updated,
    NoEventAvailable,
    EventbriteError {
        error: EventbriteError
    },
    UnexpectedError {
        error: failure::Error
    },
}

pub struct GetAttendees {
    pub nb: i8
}

pub struct GetEvent {}