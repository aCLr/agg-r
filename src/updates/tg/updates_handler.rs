use super::parsers;
use super::TelegramSource;
use super::TelegramUpdate;
use super::UpdatesHandler;
use crate::db::{models, Pool};
use crate::result::{Error, Result};
use async_trait::async_trait;

#[async_trait]
impl UpdatesHandler<TelegramUpdate> for TelegramSource {
    async fn create_source(
        &self,
        db_pool: &Pool,
        updates: &TelegramUpdate,
    ) -> Result<models::Source> {
        self.collector
            .read()
            .await
            .join_chat(&updates.chat_id)
            .await?;
        let chann = self
            .collector
            .read()
            .await
            .get_channel(updates.chat_id)
            .await?;
        if chann.is_none() {
            return Err(Error::SourceNotFound);
        }
        let s = TelegramSource::channel_to_new_source(chann.unwrap());
        Ok(s.save(db_pool).await?)
    }

    async fn process_updates(&self, db_pool: &Pool, updates: &TelegramUpdate) -> Result<usize> {
        let sources = models::Source::search(db_pool, updates.chat_id.to_string().as_str()).await?;
        let source = match sources.len() {
            0 => self.create_source(db_pool, updates).await?,
            _ => sources.first().unwrap().clone(),
        };
        let message_id = updates.message_id.clone();
        let created = models::NewRecord::update_or_create(
            db_pool,
            vec![models::NewRecord {
                title: None,
                image: None,
                date: updates
                    .date
                    .map(|d| chrono::NaiveDateTime::from_timestamp(d.clone(), 0)),
                source_record_id: message_id.to_string(),
                source_id: source.id,
                content: updates.content.clone(),
            }],
        )
        .await?;
        // records inserted
        self.handle_record_inserted(db_pool, updates.chat_id, message_id, created)
            .await
    }
}
