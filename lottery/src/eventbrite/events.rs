use reqwest;
use failure::Error;
use super::errors::EventbriteError;
use super::model::{Event, EventsResponse};
use super::EVENTBRITE_BASE_URL;


fn events_url(organizer: &str, token: &str) -> String {
    format!("{base_url}/v3/events/search/?sort_by=date&organizer.id={organizer}&token={token}", base_url = EVENTBRITE_BASE_URL, organizer = organizer, token = token)
}

fn load_events(organizer: &str, token: &str) -> Result<EventsResponse, Error> {
    let events = reqwest::get(&events_url(organizer, token))?
        .error_for_status()?
        .json()?;
    Ok(events)
}

fn first_event(events: EventsResponse) -> Result<Event, Error> {
    events.events.first()
        .map(|reference| reference.clone())
        .ok_or(EventbriteError::NoEventAvailable.into())
}

fn fetch_first_event<F: Fn(&str, &str) -> Result<EventsResponse, Error>>(fetch: F, organizer: &str, token: &str) -> Result<Event, Error> {
    fetch(organizer, token).and_then(first_event)
}

pub fn get_current_event(organizer: &str, token: &str) -> Result<Event, Error> {
    fetch_first_event(load_events, organizer, token)
}


#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_events_url() {
        assert_eq!(events_url("412451CDS", "5O5ICDI5I4LUFCAZRSTX"), EVENTBRITE_BASE_URL.to_owned() + "/v3/events/search/?sort_by=date&organizer.id=412451CDS&token=5O5ICDI5I4LUFCAZRSTX");
    }

    #[test]
    fn test_first_event() {
        let response = EventsResponse{ events: vec![Event{id: "51124390428".to_string()}]};
        let actual = first_event(response);
        assert!(actual.is_ok());
        assert_eq!(actual.unwrap(), Event{id: "51124390428".to_string()});

        let response = EventsResponse{ events: vec![]};
        let actual = first_event(response);
        assert!(actual.is_err());
        matches!(actual.unwrap_err().downcast::<EventbriteError>(), Ok(EventbriteError::NoEventAvailable));

        let response = EventsResponse{ events: vec![Event{id: "51124390432".to_string()}, Event{id: "51124390428".to_string()}]};
        let actual = first_event(response);
        assert!(actual.is_ok());
        assert_eq!(actual.unwrap(), Event{id: "51124390432".to_string()});
    }

    #[test]
    fn test_fetch_first_event() {
        use std::io::Error;
        use std::io::ErrorKind;

        let fetch = |_organizer: &str, _token: &str| {
            Ok(EventsResponse{events: vec![Event{id: "51124390428".to_string()}]})
        };
        let actual = fetch_first_event(fetch, "412451CDS", "5O5ICDI5I4LUFCAZRSTX");
        assert!(actual.is_ok());
        assert_eq!(actual.unwrap(), Event{id: "51124390428".to_string()});

        let fetch = |_organizer: &str, _token: &str| {
            Ok(EventsResponse{events: vec![]})
        };
        let actual = fetch_first_event(fetch, "412451CDS", "5O5ICDI5I4LUFCAZRSTX");
        assert!(actual.is_err());
        matches!(actual.unwrap_err().downcast::<EventbriteError>(), Ok(EventbriteError::NoEventAvailable));

        let fetch = |_organizer: &str, _token: &str| {
            Err(Error::new(ErrorKind::ConnectionRefused, "Fake error").into())
        };
        let actual = fetch_first_event(fetch, "412451CDS", "5O5ICDI5I4LUFCAZRSTX");
        assert!(actual.is_err());
        matches!(actual.unwrap_err().downcast::<Error>(), Ok(ref e) if e.kind() == ErrorKind::ConnectionRefused);

        let fetch = |_organizer: &str, _token: &str| {
            Ok(EventsResponse{events: vec![Event{id: "51124390432".to_string()}, Event{id: "51124390428".to_string()}]})
        };
        let actual = fetch_first_event(fetch, "412451CDS", "5O5ICDI5I4LUFCAZRSTX");
        assert!(actual.is_ok());
        assert_eq!(actual.unwrap(), Event{id: "51124390432".to_string()});
    }

}