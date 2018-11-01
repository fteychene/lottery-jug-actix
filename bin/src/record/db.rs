//! Db executor actor
use actix::prelude::*;
use actix_web::*;
use diesel;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use uuid;

use super::models;
use super::schema;

/// This is db executor actor. We are going to run 3 of them in parallel.
pub struct DbExecutor(pub Pool<ConnectionManager<SqliteConnection>>);

/// This is only message that this actor can handle, but it is easy to extend
/// number of messages.
#[derive(Serialize, Deserialize)]
pub struct CreateWinner {
    pub first_name: String,
    pub last_name: String,
    pub event_id: Option<String>
}

impl Message for CreateWinner {
    type Result = Result<models::Winner, Error>;
}

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

impl Handler<CreateWinner> for DbExecutor {
    type Result = Result<super::models::Winner, Error>;

    fn handle(&mut self, msg: CreateWinner, _: &mut Self::Context) -> Self::Result {
        use self::schema::winners::dsl::*;



        let uuid = format!("{}", uuid::Uuid::new_v4());
        let new_user = models::NewWinner {
            id: &uuid,
            first_name: &msg.first_name,
            last_name: &msg.last_name,
            event_id: &msg.event_id.unwrap_or("Unknown".to_owned()),
        };

        let conn: &SqliteConnection = &self.0.get().unwrap();

        diesel::insert_into(winners)
            .values(&new_user)
            .execute(conn)
            .map_err(|err| { eprintln!("{:?}", err); error::ErrorInternalServerError("Error inserting person") })?;

        let mut items = winners
            .filter(id.eq(&uuid))
            .load::<models::Winner>(conn)
            .map_err(|_| error::ErrorInternalServerError("Error loading person"))?;

        Ok(items.pop().unwrap())
    }
}
