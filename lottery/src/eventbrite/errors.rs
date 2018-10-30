use failure::Error;

#[derive(Debug, Fail)]
pub enum EventbriteError {
    #[fail(display = "error while loading attendees for event {}", event_id)]
    AttendeesLoadError {
        event_id: String,
        #[cause] cause: Error
    },
    #[fail(display = "No event available on eventbrite")]
    NoEventAvailable
}
