use serde::Deserialize;

#[derive(Deserialize)]
pub struct ApiResponse<T> {
    pub ok: bool,
    pub result: T,
    pub description: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Update {
    pub update_id: i64,
    pub message: Option<Message>,
    pub edited_message: Option<Message>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct Message {
    pub message_id: i64,
    pub text: Option<String>,
    pub chat: Chat,
    pub from: Option<User>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct Chat {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct User {
    pub id: i64,
    pub is_bot: bool,
    pub first_name: String,
    pub username: Option<String>,
}
