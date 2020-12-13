use super::structs::*;
use crate::db::{models, Pool};
use crate::result::{Error, Result};
use std::path::Path;
use std::sync::Arc;
use tg_collector::tg_client::TgClient;
use tokio::sync::RwLock;

pub(super) const TELEGRAM: &str = "TELEGRAM";

pub struct TelegramSourceBuilder {
    api_id: i64,
    api_hash: String,
    phone_number: String,
    log_verbosity_level: i32,
    database_directory: String,
    max_download_queue_size: usize,
    log_download_state_secs_interval: u64,
    files_directory: String,
}

impl TelegramSourceBuilder {
    pub fn new(
        api_id: i64,
        api_hash: &str,
        phone_number: &str,
        max_download_queue_size: usize,
        files_directory: &str,
        log_download_state_secs_interval: u64,
    ) -> Self {
        Self {
            api_id,
            max_download_queue_size,
            log_download_state_secs_interval,
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
            log_verbosity_level: self.log_verbosity_level,
            database_directory: self.database_directory.clone(),
            api_id: self.api_id,
            api_hash: self.api_hash.clone(),
            log_download_state_secs_interval: self.log_download_state_secs_interval,
            phone_number: self.phone_number.clone(),
            max_download_queue_size: self.max_download_queue_size,
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
    pub(super) collector: Arc<RwLock<TgClient>>,
    pub(super) files_directory: String,
}

impl TelegramSource {
    pub fn builder(
        api_id: i64,
        api_hash: &str,
        phone_number: &str,
        max_download_queue_size: usize,
        files_directory: &str,
        log_download_state_secs_interval: u64,
    ) -> TelegramSourceBuilder {
        TelegramSourceBuilder::new(
            api_id,
            api_hash,
            phone_number,
            max_download_queue_size,
            files_directory,
            log_download_state_secs_interval,
        )
    }

    /// Handles new `TelegramFile`.
    ///
    /// Here is `TelegramFile` lifecycle:
    ///     1. new file found during `TelegramUpdate::Message` parse
    ///     2. file downloads with `handle_file_update`
    ///     3. `TelegramUpdate::File` update received when download finished
    ///     4. file moves to `files_directory`

    pub(super) async fn handle_new_files(
        &self,
        db_pool: &Pool,
        files: &[TelegramFileWithMeta],
        record_id: i32,
    ) -> Result<()> {
        let db_files = files
            .iter()
            .map(|file| {
                let meta: Option<String>;
                let type_: String;

                // TODO add posibility to disable particular types from config
                match &file.file_type {
                    FileType::Document => {
                        type_ = "DOCUMENT".to_string();
                        meta = None;
                    }
                    FileType::Animation(animation_meta) => {
                        type_ = "ANIMATION".to_string();
                        meta = serde_json::to_string(animation_meta).ok();
                    }
                    FileType::Image(image_meta) => {
                        type_ = "IMAGE".to_string();
                        meta = serde_json::to_string(image_meta).ok();
                    }
                };
                models::NewFile {
                    kind: TELEGRAM.to_string(),
                    local_path: file.path.local_path.clone(),
                    remote_path: file.path.remote_file.clone(),
                    remote_id: Some(file.path.remote_id.clone()),
                    file_name: file.file_name.clone(),
                    record_id,
                    type_,
                    meta,
                }
            })
            .collect();
        models::NewFile::update_or_create(db_pool, db_files).await?;
        for f in files {
            match self
                .collector
                .write()
                .await
                .download_file(f.path.remote_file.parse().unwrap())
                .await
            {
                Ok(_) => {}
                Err(e) => error!("telegram file download failed: {}", e),
            }
        }
        Ok(())
    }

    pub(super) async fn handle_file_downloaded(
        &self,
        db_pool: &Pool,
        file: &TelegramFile,
    ) -> Result<()> {
        let db_file = models::File::get_file_by_remote_id(db_pool, file.remote_id.clone()).await?;
        match db_file {
            None => warn!("unknown telegram file: {:?}", file),
            Some(mut db_file) => {
                let file_name = Path::new(file.local_path.as_str()).file_name().unwrap();
                let new_path = Path::new(self.files_directory.as_str()).join(&file_name);
                tokio::fs::rename(&file.local_path, &new_path).await?;
                // TODO: cross-platform?
                db_file.local_path = Some(new_path.into_os_string().into_string().unwrap());
                match db_file.file_name {
                    None => {
                        db_file.file_name = Some(file_name.to_os_string().into_string().unwrap())
                    }
                    Some(_) => {}
                }
                db_file.save(db_pool).await?;
            }
        }
        Ok(())
    }

    pub(super) async fn handle_record_inserted(
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
                models::Record::set_external_ink(db_pool, sri.clone(), *si, message_link).await?;
                Ok(1)
            }
            x => {
                warn!("exactly one source must be created, create {}", x);
                Err(Error::SourceCreationError)
            }
        }
    }
}
