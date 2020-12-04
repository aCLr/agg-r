mod parsers;
mod source_provider;
mod updates_handler;

pub use source_provider::*;
pub use updates_handler::*;

use super::{SourceData, SourceProvider, UpdatesHandler};
use crate::db::{models, Pool};
use crate::result::{Error, Result};
use parsers::parse_update;
use std::sync::Arc;
use std::thread::JoinHandle;
use tg_collector::tg_client::{TgClient, TgUpdate};
use tg_collector::types::Channel;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::spawn;

// TODO: enum?
const TELEGRAM: &'static str = "TELEGRAM";

#[derive(Debug)]
pub struct TelegramFile {
    pub local_path: Option<String>,
    pub remote_file: String,
    pub remote_id: String,
    pub file_name: Option<String>,
}

#[derive(Debug)]
pub struct TelegramUpdate {
    pub message_id: i64,
    pub chat_id: i64,
    pub date: Option<i64>,
    pub content: String,
    pub file: Option<TelegramFile>,
}

#[derive(Clone)]
pub struct Handler {
    sender: Arc<Mutex<mpsc::Sender<Result<SourceData>>>>,
    tg: Arc<RwLock<TgClient>>,
    orig_sender: mpsc::Sender<TgUpdate>,
    orig_receiver: Arc<Mutex<mpsc::Receiver<TgUpdate>>>,
}

impl Handler {
    pub fn new(
        sender: Arc<Mutex<mpsc::Sender<Result<SourceData>>>>,
        tg: Arc<RwLock<TgClient>>,
    ) -> Self {
        let (orig_sender, orig_receiver) = mpsc::channel::<TgUpdate>(2000);
        Self {
            sender,
            tg,
            orig_sender,
            orig_receiver: Arc::new(Mutex::new(orig_receiver)),
        }
    }

    pub async fn run(&mut self) -> JoinHandle<()> {
        let mut guard = self.tg.write().await;
        guard.start_listen_updates(self.orig_sender.clone());
        // TODO handle join
        let join_handle = guard.start();
        let recv = self.orig_receiver.clone();
        let sender = self.sender.clone();
        spawn(async move {
            loop {
                let update = recv.lock().await.recv().await;
                match &update {
                    None => return,
                    Some(update) => {
                        let parsed_update = match parse_update(update).await {
                            Ok(Some(update)) => Ok(SourceData::Telegram(update)),
                            Err(e) => Err(e),

                            Ok(None) => continue,
                        };
                        let mut local_sender = sender.lock().await;

                        match local_sender.send(parsed_update).await {
                            Err(err) => warn!("{}", err),
                            Ok(_) => {}
                        }
                    }
                }
            }
        });
        join_handle
    }
}

pub struct TelegramSourceBuilder {
    api_id: i64,
    api_hash: String,
    phone_number: String,
    log_verbosity_level: i32,
    database_directory: String,
}

impl TelegramSourceBuilder {
    pub fn new(api_id: i64, api_hash: &str, phone_number: &str) -> Self {
        Self {
            api_id,
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
        };
        TelegramSource {
            collector: Arc::new(RwLock::new(tg_collector::tg_client::TgClient::new(
                &tg_conf,
            ))),
        }
    }
}
pub struct TelegramSource {
    collector: Arc<RwLock<TgClient>>,
}

impl TelegramSource {
    pub fn builder(api_id: i64, api_hash: &str, phone_number: &str) -> TelegramSourceBuilder {
        TelegramSourceBuilder::new(api_id, api_hash, phone_number)
    }

    fn channel_to_new_source(channel: Channel) -> models::NewSource {
        models::NewSource {
            name: channel.title,
            origin: channel.chat_id.to_string(),
            kind: TELEGRAM.to_string(),
            image: None,
            external_link: channel.username,
        }
    }

    async fn handle_record_inserted(
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
