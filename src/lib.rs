struct User {
    id: u32,
    username: String,
    password: String,
    rooms: Vec<Room>,
}

struct Room {
    id: u32,
    messages: Vec<Message>,
}

struct Message {
    id: u32,
    content: String,
}
