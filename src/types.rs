use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct ApiResponse<T> {
    pub ok: bool,
    pub result: T,
    pub description: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize, Debug)]
pub struct Update {
    pub update_id: i64,
    pub message: Option<Message>,
    pub edited_message: Option<Message>,
    pub callback_query: Option<CallbackQuery>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Message {
    pub message_id: i64,
    pub text: Option<String>,
    pub chat: Chat,
    pub from: Option<User>,
    pub contact: Option<Contact>,
    pub location: Option<Location>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Chat {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct User {
    pub id: i64,
    pub is_bot: bool,
    pub first_name: String,
    pub username: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Contact {
    pub phone_number: String,
    pub first_name: String,
    pub last_name: Option<String>,
    pub user_id: Option<i64>,
    pub vcard: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Location {
    pub longitude: f64,
    pub latitude: f64,
    pub horizontal_accuracy: Option<f64>,
    pub live_period: Option<i64>,
    pub heading: Option<i64>,
    pub proximity_alert_radius: Option<i64>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CallbackQuery {
    pub id: String,
    pub from: User,
    pub message: Option<Message>,
    pub data: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct File {
    pub file_id: String,
    pub file_unique_id: String,
    pub file_size: Option<u64>,
    pub file_path: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PhotoSize {
    pub file_id: String,
    pub file_unique_id: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub file_size: Option<u64>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserProfilePhotos {
    pub total_count: u64,
    pub photos: Vec<Vec<PhotoSize>>, 
}

