#[derive(Debug)]
pub enum TelegramUpdate {
    File(TelegramFile),
    Message(TelegramMessage),
}

#[derive(Debug)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub chat_id: i64,
    pub date: Option<i64>,
    pub content: String,
    pub file: Option<TelegramFile>,
}

#[derive(Debug)]
pub struct TelegramFile {
    pub local_path: Option<String>,
    pub remote_file: String,
    pub remote_id: String,
    pub file_name: Option<String>,
}
