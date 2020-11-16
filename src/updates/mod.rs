use crate::db::{models, Pool};
use crate::error::Result;
use async_trait::async_trait;
use futures::future::join_all;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};

pub mod http;
pub mod tg;

#[derive(Debug)]
pub enum SourceData {
    WebFeed(http::FeedUpdate),
    Telegram(tg::TelegramUpdate),
}

#[async_trait]
pub trait UpdatesHandler<T> {
    async fn create_source(&self, db_pool: &Pool, updates: &T) -> Result<models::Source>;
    async fn process_updates(&self, db_pool: &Pool, updates: &T) -> Result<usize>;
}

#[async_trait]
pub trait SourceProvider {
    async fn run(
        &self,
        db_pool: &Pool,
        updates_sender: Arc<Mutex<mpsc::Sender<Result<SourceData>>>>,
    );
    async fn search_source(&self, db_pool: &Pool, query: &str) -> Result<Vec<models::Source>>;
}

pub struct SourcesAggregator {
    http_source: Option<Arc<http::HttpSource>>,
    tg_source: Option<Arc<tg::TelegramSource>>,
    updates_sender: Arc<Mutex<Sender<Result<SourceData>>>>,
    updates_receiver: Mutex<Receiver<Result<SourceData>>>,
}

impl SourcesAggregator {
    pub fn builder() -> UpdatesHandlerBuilder {
        UpdatesHandlerBuilder::new()
    }

    pub async fn search_source(&self, db_pool: &Pool, query: &str) -> Result<Vec<models::Source>> {
        let source_providers = self.get_enabled_sources();
        let mut tasks = vec![];
        for provider in &source_providers {
            tasks.push(provider.search_source(db_pool, query))
        }
        let mut results = vec![];
        let tasks_results = join_all(tasks).await;
        for task_result in tasks_results {
            match task_result {
                Ok(res) => results.extend(res),
                Err(err) => Err(err)?,
            }
        }
        results.extend(models::Source::get_by_origin(db_pool, query).await?);
        // TODO: check: duplicates appears
        results.dedup_by_key(|s| s.id);
        Ok(results)
    }

    fn get_enabled_sources(&self) -> Vec<Arc<dyn SourceProvider>> {
        let mut enabled: Vec<Arc<dyn SourceProvider>> = vec![];
        macro_rules! push_if_enabled {
            ($source:expr) => {
                match &$source {
                    Some(source) => enabled.push(source.clone()),
                    None => {}
                }
            };
        }
        push_if_enabled!(self.http_source);
        push_if_enabled!(self.tg_source);
        enabled
    }

    pub async fn run(&self, db_pool: &Pool) {
        macro_rules! run_source {
            ($source:expr) => {
                match &$source {
                    None => {}
                    Some(source) => {
                        let s = source.clone();
                        let sender = self.updates_sender.clone();
                        s.run(db_pool, sender).await;
                    }
                }
            };
        }
        run_source!(self.tg_source);
        run_source!(self.http_source);
        self.process_updates(db_pool).await;
    }

    async fn process_updates(&self, db_pool: &Pool) {
        loop {
            while let Some(updates) = self.updates_receiver.lock().await.recv().await {
                debug!("new updates: {:?}", updates);
                let insert_result = match &updates {
                    Ok(update) => match update {
                        SourceData::WebFeed(feed_data) => match &self.http_source {
                            None => {
                                debug!("http source disabled");
                                Ok(0)
                            }
                            Some(source) => source.process_updates(db_pool, feed_data).await,
                        },
                        SourceData::Telegram(telegram_update) => match &self.tg_source {
                            None => {
                                debug!("http source disabled");
                                Ok(0)
                            }
                            Some(source) => source.process_updates(db_pool, telegram_update).await,
                        },
                    },
                    Err(err) => Err(err.clone()),
                };
                match insert_result {
                    Ok(ok_insert) => {
                        debug!("processed updates; affected db rows: {}", ok_insert);
                        debug!("updates: {:?}", updates);
                    }
                    Err(err) => {
                        error!("{}", err);
                    }
                }
            }
        }
    }
}

pub struct UpdatesHandlerBuilder {
    http_source: Option<Arc<http::HttpSource>>,
    tg_source: Option<Arc<tg::TelegramSource>>,
}

impl UpdatesHandlerBuilder {
    pub fn new() -> Self {
        Self {
            http_source: None,
            tg_source: None,
        }
    }

    pub fn with_http_source(mut self, http_source: Arc<http::HttpSource>) -> Self {
        self.http_source = Some(http_source);
        self
    }

    pub fn with_tg_source(mut self, tg_source: Arc<tg::TelegramSource>) -> Self {
        self.tg_source = Some(tg_source);
        self
    }

    pub fn build(self) -> SourcesAggregator {
        let (updates_sender, updates_receiver) = mpsc::channel::<Result<SourceData>>(2000);
        let updates_sender = Arc::new(Mutex::new(updates_sender));
        let updates_receiver = Mutex::new(updates_receiver);
        SourcesAggregator {
            http_source: self.http_source,
            tg_source: self.tg_source,
            updates_sender,
            updates_receiver,
        }
    }
}
