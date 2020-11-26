mod parsers;

use super::{SourceData, SourceProvider, UpdatesHandler};
use crate::db::{models, Pool};
use crate::result::Result;
use async_trait::async_trait;
use parsers::parse_update;
use std::sync::Arc;
use std::thread::JoinHandle;
use tg_collector::tg_client::{TgClient, TgUpdate};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::spawn;

#[derive(Debug)]
pub struct TelegramUpdate {
    pub message_id: i64,
    pub chat_id: i64,
    pub date: Option<i64>,
    pub content: String,
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
}
#[async_trait]
impl SourceProvider for TelegramSource {
    async fn run(
        &self,
        db_pool: &Pool,
        updates_sender: Arc<Mutex<mpsc::Sender<Result<SourceData>>>>,
    ) {
        let mut tg_handler = Handler::new(updates_sender, self.collector.clone());
        tg_handler.run().await;
    }

    async fn search_source(&self, db_pool: &Pool, query: &str) -> Result<Vec<models::Source>> {
        unimplemented!()
    }
}

#[async_trait]
impl UpdatesHandler<TelegramUpdate> for TelegramSource {
    async fn create_source(
        &self,
        db_pool: &Pool,
        updates: &TelegramUpdate,
    ) -> Result<models::Source> {
        unimplemented!()
    }
    async fn process_updates(&self, db_pool: &Pool, updates: &TelegramUpdate) -> Result<usize> {
        let sources =
            models::Source::get_by_origin(db_pool, updates.chat_id.to_string().as_str()).await?;
        let source = match sources.len() {
            0 => self.create_source(db_pool, updates).await?,
            _ => sources.first().unwrap().clone(),
        };

        let affected = models::NewRecord::update_or_create(
            db_pool,
            vec![models::NewRecord {
                title: None,
                image: None,
                date: updates
                    .date
                    .map(|d| chrono::NaiveDateTime::from_timestamp(d.clone(), 0)),
                guid: updates.message_id.to_string(),
                source_id: source.id.clone(),
                content: updates.content.clone(),
            }],
        )
        .await?;
        Ok(affected)
    }
}
