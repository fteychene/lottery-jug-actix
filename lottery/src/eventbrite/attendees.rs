use reqwest;
use failure::Error;
use std::ops::Range;
use frunk::monoid::combine_all;
use super::model::{Profile, AttendeesResponse};
use super::errors::EventbriteError;
use super::EVENTBRITE_BASE_URL;

/// Traverse a Vec<Result<T, Error>> and combine the values to return a Result<Vec<T>, Error>
///
/// If all values of the vector are Ok then return a Ok containing all the values cloned
/// On the first Err it stop accumulating values and return the matched error
///
/// T should be a Clone type
fn sequence<R>(seq: Vec<Result<R, Error>>) -> Result<Vec<R>, Error>
    where R: Clone {
    let result = seq.into_iter().fold(Ok(vec![]), |result, current|
        result.and_then(|mut vec|
            match current {
                Ok(value) => {
                    vec.push(value.clone());
                    Ok(vec)
                }
                Err(e) => Err(e)
            }));
    result
}

fn attendees_url(event_id: &str, token: &str, page_id: u8) -> String {
    format!("{base_url}/v3/events/{event_id}/attendees/?token={token}&page={page}", base_url = EVENTBRITE_BASE_URL, event_id = event_id, token = token, page = page_id)
}

fn fetch_attendees_page(event_id: &str, token: &str, page: u8) -> Result<AttendeesResponse, Error> {
    let attendees = reqwest::get(&attendees_url(event_id, token, page))?
        .error_for_status()?
        .json()?;
    Ok(attendees)
}

fn fetch_all_attendees<F: Fn(&str, &str, u8) -> Result<AttendeesResponse, Error>>(fetch: F, event_id: &str, token: &str) -> Result<Vec<Profile>, Error> {
    fetch(event_id, token, 0)
        .and_then(|result: AttendeesResponse| {
            let range = Range { start: result.pagination.page_number, end: result.pagination.page_count };
            sequence(range.fold(vec![Ok(result)], |mut result, page| {
                result.push(fetch(event_id, token, page + 1));
                result
            }))
        })
        .map(|results: Vec<AttendeesResponse>| results.into_iter().map(|response| response.attendees.into_iter().map(|attendee| attendee.profile).collect()).collect())
        .map(|results: Vec<Vec<Profile>>| combine_all(&results))
        .map_err(|err| EventbriteError::AttendeesLoadError { event_id: String::from(event_id), cause: err }.into())
}

pub fn load_attendees(event_id: &str, token: &str) -> Result<Vec<Profile>, Error> {
    fetch_all_attendees(fetch_attendees_page, event_id, token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::model::Pagination;

    #[test]
    fn test_attendees_url() {
        assert_eq!(attendees_url("51124390428", "5O5ICDI5I4LUFCAZRSTX", 0), EVENTBRITE_BASE_URL.to_owned() + "/v3/events/51124390428/attendees/?token=5O5ICDI5I4LUFCAZRSTX&page=0");
        assert_eq!(attendees_url("51124390428", "5O5ICDI5I4LUFCAZRSTX", 1), EVENTBRITE_BASE_URL.to_owned() + "/v3/events/51124390428/attendees/?token=5O5ICDI5I4LUFCAZRSTX&page=1");
    }

    #[test]
    fn test_sequence() {
        let actual = sequence(vec![Ok(0), Ok(1), Ok(2)]);
        assert!(actual.is_ok());
        assert_eq!(actual.unwrap(), vec![0, 1, 2]);

        let actual = sequence(vec![Ok(0), Ok(1), Err(EventbriteTestError::TestError { page: 2 }.into())]);
        assert!(actual.is_err());
        assert_eq!(actual.unwrap_err().downcast::<EventbriteTestError>().unwrap(), EventbriteTestError::TestError { page: 2 });

        let actual = sequence(vec![Ok(0), Err(EventbriteTestError::TestError { page: 1 }.into()), Ok(2)]);
        assert!(actual.is_err());
        assert_eq!(actual.unwrap_err().downcast::<EventbriteTestError>().unwrap(), EventbriteTestError::TestError { page: 1 });

        let actual = sequence(vec![Err(EventbriteTestError::TestError { page: 0 }.into()), Ok(1), Ok(2)]);
        assert!(actual.is_err());
        assert_eq!(actual.unwrap_err().downcast::<EventbriteTestError>().unwrap(), EventbriteTestError::TestError { page: 0 });

        let actual = sequence(vec![Err(EventbriteTestError::TestError { page: 0 }.into()), Err(EventbriteTestError::TestError { page: 1 }.into()), Ok(2)]);
        assert!(actual.is_err());
        assert_eq!(actual.unwrap_err().downcast::<EventbriteTestError>().unwrap(), EventbriteTestError::TestError { page: 0 });
    }

    #[test]
    fn test_fetch_all_attendees() {
        use std::io::Error;
        use std::io::ErrorKind;

        // Right case
        let load_function = |_event_id: &str, _token: &str, _page: u8| {
            Ok(AttendeesResponse {
                attendees: Vec::new(),
                pagination: Pagination {
                    object_count: 0,
                    page_count: 1,
                    page_size: 0,
                    page_number: 0,
                },
            })
        };

        let result = fetch_all_attendees(load_function, "51124390428", "5O5ICDI5I4LUFCAZRSTX");
        assert_eq!(result.unwrap().as_slice(), []);

        // Err on first call
        let load_function = |_event_id: &str, _token: &str, _page: u8| {
            Err(Error::new(ErrorKind::ConnectionRefused, "Fake error").into())
        };

        let result = fetch_all_attendees(load_function, "51124390428", "5O5ICDI5I4LUFCAZRSTX");
        assert!(result.is_err());
        let typed_error = result.unwrap_err().downcast::<EventbriteError>().unwrap();
        match typed_error {
            EventbriteError::AttendeesLoadError{event_id: _, cause} => assert_eq!(cause.downcast::<Error>().unwrap().kind(), ErrorKind::ConnectionRefused),
            _ => assert!(false)
        }

        // Err on pagination loading
        let load_function = |_event_id: &str, _token: &str, page: u8| {
            match page {
                0 => Ok(AttendeesResponse {
                    attendees: Vec::new(),
                    pagination: Pagination {
                        object_count: 0,
                        page_count: 2,
                        page_size: 0,
                        page_number: 0,
                    },
                }),
                _ => Err(Error::new(ErrorKind::ConnectionRefused, "Fake error").into())
            }
        };

        let result = fetch_all_attendees(load_function, "51124390428", "5O5ICDI5I4LUFCAZRSTX");
        assert!(result.is_err());
        let typed_error = result.unwrap_err().downcast::<EventbriteError>().unwrap();
        match typed_error {
            EventbriteError::AttendeesLoadError{event_id: _, cause} => assert_eq!(cause.downcast::<Error>().unwrap().kind(), ErrorKind::ConnectionRefused),
            _ => assert!(false)
        }
    }

    #[derive(Debug, Fail, PartialEq)]
    enum EventbriteTestError {
        #[fail(display = "Unexpected Error for tests")]
        TestError {
            page: u8
        }
    }
}