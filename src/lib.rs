use sea_orm::entity::prelude::*;

struct User {
    id: u32,
    email: String,
    username: String,
    password: String,
    rooms: Vec<Room>,
}

impl User {}

struct Room {
    id: u32,
    messages: Vec<Message>,
}

struct Message {
    id: u32,
    content: String,
}
