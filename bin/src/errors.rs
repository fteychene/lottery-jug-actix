use failure::Error;

#[derive(Fail, Debug)]
pub enum WinnerError {
    #[fail(display = "Invalid parameter")]
    InvalidParameter,
    #[fail(display = "No event available")]
    NoEventAvailable,
    #[fail(display = "Error during attendees draw")]
    DrawError { cause: Error },
    #[fail(display = "Unexpected error")]
    UnexpectedError { cause: Error },
}