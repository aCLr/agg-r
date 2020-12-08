use super::structs::*;
use crate::db::{models, Pool};
use crate::result::{Error, Result};
use std::path::Path;
use std::sync::Arc;
use tg_collector::tg_client::TgClient;
use tg_collector::types::Channel;
use tokio::sync::RwLock;

const TELEGRAM: &'static str = "TELEGRAM";

pub struct TelegramSourceBuilder {
    api_id: i64,
    api_hash: String,
    phone_number: String,
    log_verbosity_level: i32,
    database_directory: String,
    max_download_queue_size: usize,
    files_directory: String,
}

impl TelegramSourceBuilder {
    pub fn new(
        api_id: i64,
        api_hash: &str,
        phone_number: &str,
        max_download_queue_size: usize,
        files_directory: &str,
    ) -> Self {
        Self {
            api_id,
            max_download_queue_size,
            files_directory: files_directory.to_string(),
            phone_number: phone_number.to_string(),
            api_hash: api_hash.to_string(),
            log_verbosity_level: 0,
            database_directory: "tdlib".to_string(),
        }
    }

    pub fn with_log_verbosity_level(mut self, level: i32) -> Self {
        self.log_verbosity_level = level;
        self
    }

    pub fn with_database_directory(mut self, directory: &str) -> Self {
        self.database_directory = directory.to_string();
        self
    }

    pub fn build(&self) -> TelegramSource {
        let tg_conf = tg_collector::config::Config {
            log_verbosity_level: self.log_verbosity_level.clone(),
            database_directory: self.database_directory.clone(),
            api_id: self.api_id.clone(),
            api_hash: self.api_hash.clone(),
            phone_number: self.phone_number.clone(),
            max_download_queue_size: self.max_download_queue_size.clone(),
        };
        TelegramSource {
            collector: Arc::new(RwLock::new(tg_collector::tg_client::TgClient::new(
                &tg_conf,
            ))),
            files_directory: self.files_directory.clone(),
        }
    }
}
pub struct TelegramSource {
    pub(crate) collector: Arc<RwLock<TgClient>>,
    pub(crate) files_directory: String,
}

impl TelegramSource {
    pub fn builder(
        api_id: i64,
        api_hash: &str,
        phone_number: &str,
        max_download_queue_size: usize,
        files_directory: &str,
    ) -> TelegramSourceBuilder {
        TelegramSourceBuilder::new(
            api_id,
            api_hash,
            phone_number,
            max_download_queue_size,
            files_directory,
        )
    }

    pub(crate) fn channel_to_new_source(channel: Channel) -> models::NewSource {
        models::NewSource {
            name: channel.title,
            origin: channel.chat_id.to_string(),
            kind: TELEGRAM.to_string(),
            image: None,
            external_link: channel.username,
        }
    }

    pub(crate) async fn handle_file_update(&self, db_pool: &Pool, file: &TelegramFile) {
        match &file.local_path {
            None => {
                self.collector
                    .write()
                    .await
                    .download_file(file.remote_file.parse().unwrap())
                    .await?;
            }
            Some(path) => {
                let new_path = Path::new(self.files_directory.as_str())
                    .join(Path::new(path).file_name().unwrap());
                tokio::fs::rename(path, new_path).await?;
            }
        };
    }

    pub(crate) async fn handle_record_inserted(
        &self,
        db_pool: &Pool,
        chat_id: i64,
        message_id: i64,
        created: Vec<(String, i32)>,
    ) -> Result<usize, Error> {
        match created.len() {
            0 => Ok(0),
            1 => {
                let message_link = self
                    .collector
                    .read()
                    .await
                    .get_message_link(chat_id, message_id)
                    .await?;
                let (sri, si) = created.first().unwrap();
                models::Record::set_external_ink(db_pool, sri.clone(), si.clone(), message_link)
                    .await?;
                Ok(1)
            }
            x => {
                warn!("exactly one source must be created, create {}", x);
                Err(Error::SourceCreationError)
            }
        }
    }
}
