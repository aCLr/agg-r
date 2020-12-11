use crate::tools;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum TelegramUpdate {
    FileDownloadFinished(TelegramFile),
    Message(TelegramMessage),
}

#[derive(Debug)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub chat_id: i64,
    pub date: Option<i64>,
    pub content: Option<String>,
    pub files: Option<Vec<TelegramFileWithMeta>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImageMeta {
    pub width: i64,
    pub height: i64,
}

#[derive(Debug)]
pub enum FileType {
    Document,
    Image(ImageMeta),
}

#[derive(Debug)]
pub struct TelegramFileWithMeta {
    pub path: FilePath,
    pub file_type: FileType,
    pub file_name: Option<String>,
}

#[derive(Debug)]
pub struct FilePath {
    pub local_path: Option<String>,
    pub remote_file: String,
    pub remote_id: String,
}

impl FilePath {
    pub fn new(file: &tg_collector::File) -> Self {
        Self {
            local_path: tools::empty_string_as_option(file.local().path().as_str()),
            remote_file: file.id().to_string(),
            remote_id: file.remote().unique_id().clone(),
        }
    }
}

#[derive(Debug)]
pub struct TelegramFile {
    pub local_path: String,
    pub remote_file: String,
    pub remote_id: String,
}

#[derive(Debug)]
pub struct TelegramFileForRecord {
    pub file: TelegramFile,
    pub record_id: String,
}
