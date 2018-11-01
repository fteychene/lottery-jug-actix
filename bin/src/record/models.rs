use super::schema::winners;

#[derive(Serialize, Queryable, Debug)]
pub struct Winner {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub event_id: String
}

#[derive(Insertable)]
#[table_name = "winners"]
pub struct NewWinner<'a> {
    pub id: &'a str,
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub event_id: &'a str
}
