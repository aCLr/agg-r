use super::{SourceData, SourceProvider, UpdatesHandler};
use crate::db::{models, Pool};
use crate::error::{Error, Result};
use async_trait::async_trait;
use std::sync::Arc;
use std::thread::JoinHandle;
use tg_collector::tg_client::{TgClient, TgUpdate};
use tg_collector::{Message, MessageContent, MessageText};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::spawn;

macro_rules! not_supported {
    ($data:ident) => {
        Some(Err(Error {
            message: format!("not supported yet: {:?}", $data),
        }))
    };
}

#[derive(Debug)]
pub struct TelegramUpdate {
    pub message_id: i64,
    pub chat_id: i64,
    pub date: i64,
    pub edit_date: i64,
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
        let api = self.clone();
        spawn(async move {
            loop {
                let update = recv.lock().await.recv().await;
                match &update {
                    None => return,
                    Some(update) => {
                        let parsed_update = match api.parse_update(update).await {
                            None => return,
                            Some(parsed_update) => match parsed_update {
                                Ok(update) => Ok(SourceData::Telegram(update)),
                                Err(err) => Err(err),
                            },
                        };
                        let mut sender = sender.lock().await;

                        match sender.send(parsed_update).await {
                            Err(err) => warn!("{}", err),
                            Ok(_) => {}
                        }
                    }
                }
            }
        });
        join_handle
    }

    async fn parse_update(&self, tg_update: &TgUpdate) -> Option<Result<TelegramUpdate>> {
        match tg_update {
            TgUpdate::NewMessage(message_content) => {
                self.parse_message_content(message_content.message()).await
            }
            TgUpdate::MessageContent(not_supported) => not_supported!(not_supported),
            TgUpdate::ChatPhoto(not_supported) => not_supported!(not_supported),
            TgUpdate::ChatTitle(not_supported) => not_supported!(not_supported),
            TgUpdate::Supergroup(not_supported) => not_supported!(not_supported),
            TgUpdate::SupergroupFullInfo(not_supported) => not_supported!(not_supported),
        }
    }

    async fn parse_message_content(&self, message: &Message) -> Option<Result<TelegramUpdate>> {
        let content: Option<String> = match message.content() {
            MessageContent::MessageAnimation(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageAudio(not_supported) => return not_supported!(not_supported),
            MessageContent::MessageChatChangePhoto(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageChatChangeTitle(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageChatDeletePhoto(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageChatJoinByLink(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageChatUpgradeFrom(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageChatUpgradeTo(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageContact(not_supported) => return not_supported!(not_supported),
            MessageContent::MessageContactRegistered(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageCustomServiceAction(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageDice(not_supported) => return not_supported!(not_supported),
            MessageContent::MessageDocument(not_supported) => return not_supported!(not_supported),
            MessageContent::MessageExpiredPhoto(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageExpiredVideo(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageInvoice(not_supported) => return not_supported!(not_supported),
            MessageContent::MessageLocation(not_supported) => return not_supported!(not_supported),
            MessageContent::MessagePassportDataReceived(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessagePhoto(not_supported) => return not_supported!(not_supported),
            MessageContent::MessagePoll(not_supported) => return not_supported!(not_supported),
            MessageContent::MessageScreenshotTaken(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageSticker(not_supported) => return not_supported!(not_supported),
            MessageContent::MessageSupergroupChatCreate(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageText(message_text) => {
                self.parse_message_text(message_text).await
            }
            MessageContent::MessageVenue(not_supported) => return not_supported!(not_supported),
            MessageContent::MessageVideo(not_supported) => return not_supported!(not_supported),
            MessageContent::MessageVideoNote(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageVoiceNote(not_supported) => {
                return not_supported!(not_supported)
            }
            MessageContent::MessageWebsiteConnected(not_supported) => {
                return not_supported!(not_supported)
            }

            _ => {
                debug!("not supported content: {:?}", message.content());
                None
            }
        };
        match content {
            None => None,
            Some(content) => Some(Ok(TelegramUpdate {
                message_id: message.id(),
                chat_id: message.chat_id(),
                date: message.date(),
                edit_date: message.edit_date(),
                content: content,
            })),
        }
    }

    async fn parse_message_text(&self, message_text: &MessageText) -> Option<String> {
        Some(message_text.text().text().clone())
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

    async fn search_source(
        &self,
        db_pool: &Pool,
        query: &str,
    ) -> Result<Vec<models::Source>, Error> {
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
                date: chrono::NaiveDateTime::from_timestamp(updates.date.clone(), 0),
                guid: updates.message_id.to_string(),
                source_id: source.id.clone(),
                content: updates.content.clone(),
            }],
        )
        .await?;
        Ok(affected)
    }
}
