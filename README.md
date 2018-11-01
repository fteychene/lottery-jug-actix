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


## Build

`cargo build`

## API

### Draw winners 
`GET` -> `/winners?nb=X`

__Results__ : 
 - `200` : 
```json
[
  {
    "first_name": "Francois",
    "last_name": "Teychene"
  },
  {
    "first_name": "Jean-Luc",
    "last_name": "Racine"
  },
  {
    "first_name": "Renard",
    "last_name": "Chenapan"
  }
]
```
 - `400` : Invalid parameter
 - `503` : No live events
 - `500` : Unxepected error

### Record a winner
`POST` -> `/record`

_Body_ : 
```json
{
  "first_name": "Francois",
  "last_name": "Teychene"
}
```

__Results__ : 
 - `200` : 
 ```json
{
    "id": "b3f0182e-b2f4-47a2-9c6f-9ea3a67b588c",
    "first_name": "Francois",
    "last_name": "Teychene",
    "event_id": "52097259305"
}
```
 - `500` : Unexpected error