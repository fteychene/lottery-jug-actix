#[derive(Deserialize, Debug, Clone)]
pub struct Pagination {
    pub object_count: u8,
    pub page_count: u8,
    pub page_size: u8,
    pub page_number: u8
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Profile {
    pub first_name: String,
    pub last_name: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Attende {
    pub profile: Profile
}

#[derive(Deserialize, Debug, Clone)]
pub struct AttendeesResponse {
    pub attendees: Vec<Attende>,
    pub pagination: Pagination
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Event {
    pub id: String
}

#[derive(Deserialize, Debug)]
pub struct EventsResponse {
    pub events: Vec<Event>
}