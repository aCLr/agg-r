use crate::db::{models, Pool};
use crate::result::Result;
use crate::updates::Source;
use crate::{config, updates};
use std::sync::Arc;

pub struct Aggregator {
    handler: updates::SourcesAggregator,
    db_pool: Pool,
}

impl Aggregator {
    pub fn new(handler: updates::SourcesAggregator, db_pool: Pool) -> Self {
        Self { handler, db_pool }
    }

    pub async fn run(&self) {
        self.handler.run(&self.db_pool).await
    }

    pub async fn search_source(&self, query: &str) -> Result<Vec<models::Source>> {
        self.handler.search_source(&self.db_pool, query).await
    }

    pub async fn synchronize(&self, secs_depth: i32, source: Option<Source>) -> Result<()> {
        self.handler
            .synchronize(&self.db_pool, secs_depth, source)
            .await
    }
}

pub struct AggregatorBuilder<'a> {
    config: &'a config::AggregatorConfig,
    db_pool: Pool,
}

impl<'a> AggregatorBuilder<'a> {
    pub fn new(config: &'a config::AggregatorConfig, db_pool: Pool) -> Self {
        Self { config, db_pool }
    }

    pub fn build(&self) -> Aggregator {
        debug!("config for building: {:?}", self.config);
        let mut updates_builder = updates::SourcesAggregator::builder();

        if *self.config.http().enabled() {
            let http_source = updates::http::HttpSource::builder()
                .with_sleep_secs(*self.config.http().sleep_secs())
                .build();
            let http_source = Arc::new(http_source);
            updates_builder = updates_builder.with_http_source(http_source);
        }

        if *self.config.telegram().enabled() {
            let tg_source = updates::tg::TelegramSource::builder(
                *self.config.telegram().api_id(),
                self.config.telegram().api_hash(),
                self.config.telegram().phone(),
                *self.config.telegram().max_download_queue_size(),
                self.config.telegram().files_directory(),
                *self.config.telegram().log_download_state_secs_interval(),
            )
            .with_database_directory(self.config.telegram().database_directory().as_str())
            .with_log_verbosity_level(*self.config.telegram().log_verbosity_level())
            .build();
            let tg_source = Arc::new(tg_source);
            updates_builder = updates_builder.with_tg_source(tg_source);
        }

        Aggregator::new(updates_builder.build(), self.db_pool.clone())
    }
}
