# Jug Lottery (v3)

Rust implementation of Montpellier Jug lottery (third version).  
This implementation is a project to demonstrate the utilisation of several crates :
 - [Actix-web](https://github.com/actix/actix-web)
 - [Actix](https://github.com/actix/actix)
 - [Failure](https://github.com/rust-lang-nursery/failure)
 - [Serde](https://github.com/serde-rs/serde)
 - [Reqwest](https://github.com/seanmonstar/reqwest)
 - [Diesel](https://github.com/diesel-rs/diesel)
 
 
 This project define 2 projects :
  - [lottery](lottery) : Define the application logic (eventbrite call and draw logic)
  - [bin](bin) : Define the actor model and create application library.
  
