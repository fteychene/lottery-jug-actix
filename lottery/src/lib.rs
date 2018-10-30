extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate reqwest;
extern crate frunk;
#[cfg(test)] #[macro_use] extern crate matches;
extern crate rand;

pub mod eventbrite;
pub mod lottery;