use async_trait::async_trait;
use chrono::NaiveDateTime;
use http_collector::models::{Feed, FeedItem, FeedKind};
use http_collector::result::Result as HttpResult;
use std::sync::Arc;

use super::{SourceData, SourceProvider, UpdatesHandler};
use crate::db::{models, Pool};
use crate::result::{Error, Result};

use crate::updates::Source;
use http_collector::collector::{CacheStub, HttpCollector, ResultsHandler};
use serde::Serialize;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::Duration;

// TODO: enum?
const WEB: &'static str = "WEB";

impl From<Feed> for FeedUpdate {
    fn from(feed_update: Feed) -> Self {
        Self {
            link: feed_update.link,
            name: feed_update.name,
            image: feed_update.image,
            kind: feed_update.kind,
            updates: feed_update
                .content
                .iter()
                .map(|f| Update::from(f.clone()))
                .collect(),
        }
    }
}

impl From<FeedItem> for Update {
    fn from(feed_item: FeedItem) -> Self {
        Self {
            title: feed_item.title,
            content: feed_item.content,
            pub_date: feed_item.pub_date,
            guid: feed_item.guid,
            image_link: feed_item.image_link,
        }
    }
}

struct Handler {
    sender: Arc<Mutex<mpsc::Sender<Result<SourceData>>>>,
}

impl Handler {
    pub fn new(sender: Arc<Mutex<mpsc::Sender<Result<SourceData>>>>) -> Self {
        Self { sender }
    }
}

#[async_trait]
impl ResultsHandler for Handler {
    async fn process(&self, result: HttpResult<(&Feed, FeedKind, String)>) {
        let update = match result {
            Ok((updates, _, _)) => Ok(SourceData::WebFeed(FeedUpdate::from(updates.clone()))),
            Err(err) => Err(Error::HttpCollectorError(err)),
        };
        let mut local = self.sender.lock().await;
        if let Err(_) = local.send(update).await {
            error!("updates receiver dropped");
            return;
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Update {
    pub title: Option<String>,
    pub content: String,
    pub pub_date: NaiveDateTime,
    pub guid: String,
    pub image_link: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FeedUpdate {
    pub link: String,
    pub kind: FeedKind,
    pub name: String,
    pub image: Option<String>,
    pub updates: Vec<Update>,
}

pub struct HttpSourceBuilder {
    sleep_secs: u64,
    scrape_source_secs_interval: i32,
}

impl HttpSourceBuilder {
    pub fn new() -> Self {
        Self {
            sleep_secs: 60,
            scrape_source_secs_interval: 60,
        }
    }

    pub fn with_sleep_secs(mut self, sleep_secs: u64) -> Self {
        self.sleep_secs = sleep_secs;
        self
    }

    pub fn with_scrape_source_secs_interval(mut self, scrape_source_secs_interval: i32) -> Self {
        self.scrape_source_secs_interval = scrape_source_secs_interval;
        self
    }

    pub fn build(&self) -> HttpSource {
        HttpSource {
            sleep_secs: self.sleep_secs.clone(),
            scrape_source_secs_interval: self.scrape_source_secs_interval.clone(),
            collector: Arc::new(HttpCollector::new()),
        }
    }
}

pub struct HttpSource {
    sleep_secs: u64,
    scrape_source_secs_interval: i32,
    collector: Arc<HttpCollector<CacheStub>>,
}

impl HttpSource {
    pub fn builder() -> HttpSourceBuilder {
        HttpSourceBuilder::new()
    }
}

#[async_trait]
impl UpdatesHandler<FeedUpdate> for HttpSource {
    async fn create_source(&self, db_pool: &Pool, updates: &FeedUpdate) -> Result<models::Source> {
        let new_source = models::NewSource {
            name: updates.name.clone(),
            origin: updates.link.clone(),
            kind: WEB.to_string(),
            image: updates.image.clone(),
        };

        Ok(new_source.save(db_pool).await?)
    }

    async fn process_updates(&self, db_pool: &Pool, updates: &FeedUpdate) -> Result<usize> {
        let sources = models::Source::get_by_origin(db_pool, updates.link.as_str()).await?;
        let source = match sources.len() {
            0 => self.create_source(db_pool, updates).await?,
            _ => sources.first().unwrap().clone(),
        };
        let affected = models::NewRecord::update_or_create(
            db_pool,
            updates
                .updates
                .iter()
                .map(|u| models::NewRecord {
                    date: Some(u.pub_date.clone()),
                    title: u.title.clone(),
                    guid: u.guid.clone(),
                    source_id: source.id.clone(),
                    content: u.content.clone(),
                    image: u.image_link.clone(),
                })
                .collect::<Vec<models::NewRecord>>(),
        )
        .await?;
        source.set_scraped_now(db_pool).await?;
        Ok(affected)
    }
}

#[async_trait]
impl SourceProvider for HttpSource {
    fn get_source(&self) -> Source {
        Source::Web
    }

    async fn synchronize(&self, db_pool: &Pool, secs_depth: i32) -> Result<()> {
        // nothing to sync with http source
        Ok(())
    }

    async fn run(
        &self,
        db_pool: &Pool,
        updates_sender: Arc<Mutex<mpsc::Sender<Result<SourceData>>>>,
    ) {
        let (sources_sender, sources_receiver) = mpsc::channel(2000);
        let sleep_secs = self.sleep_secs.clone();
        let scrape_source_secs_interval = self.scrape_source_secs_interval.clone();
        let pool = db_pool.clone();
        tokio::spawn(async move {
            sources_gen(
                &pool,
                scrape_source_secs_interval,
                sleep_secs,
                sources_sender,
            )
            .await
        });
        let http_handler = Handler::new(updates_sender);
        let http_runner = self.collector.clone();
        tokio::spawn(async move { http_runner.run(sources_receiver, &http_handler).await });
    }

    async fn search_source(&self, db_pool: &Pool, query: &str) -> Result<Vec<models::Source>> {
        let mut query = query.to_string();
        if !query.starts_with("http://") && !query.starts_with("https://") {
            query = format!("https://{}", query);
        }
        // let feeds: Vec<Feed> = vec![];
        let feeds = self.collector.detect_feeds(query.as_str()).await?;
        let new_sources = models::NewSource::save_bulk(
            db_pool,
            feeds
                .iter()
                .map(|f| models::NewSource {
                    name: f.name.clone(),
                    origin: f.link.clone(),
                    kind: WEB.to_string(),
                    image: f.image.clone(),
                })
                .collect(),
        )
        .await?;
        let feeds: Vec<FeedUpdate> = feeds.iter().map(|f| FeedUpdate::from(f.clone())).collect();
        let mut tasks = vec![];
        feeds
            .iter()
            .for_each(|f| tasks.push(self.process_updates(db_pool, f)));
        futures::future::join_all(tasks).await;
        Ok(new_sources)
    }
}

async fn sources_gen(
    db_pool: &Pool,
    source_check_period: i32,
    sleep_period: u64,
    mut sender: mpsc::Sender<Vec<(Option<FeedKind>, String)>>,
) {
    let sleep_period = Duration::from_secs(sleep_period);
    loop {
        match get_sources(db_pool, &source_check_period).await {
            Ok(sources) => {
                debug!("found sources for scrape: {:?}", sources);
                match sender.send(sources).await {
                    Err(err) => error!("{}", err),
                    Ok(_) => {}
                };
            }
            Err(e) => error!("{}", e),
        };

        debug!("send sources delayed for {:?}", sleep_period);
        tokio::time::delay_for(sleep_period).await;
    }
}

async fn get_sources(
    db_pool: &Pool,
    source_check_period_secs: &i32,
) -> Result<Vec<(Option<FeedKind>, String)>> {
    Ok(
        models::Source::get_by_kind_for_scrape(db_pool, WEB.to_string(), source_check_period_secs)
            .await?
            .iter()
            .map(|r| (None, r.origin.clone()))
            .collect(),
    )
}
