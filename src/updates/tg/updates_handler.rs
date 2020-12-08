use super::TelegramSource;
use super::TelegramUpdate;
use crate::db::{models, Pool};
use crate::result::{Error, Result};
use crate::updates::UpdatesHandler;
use async_trait::async_trait;

#[async_trait]
impl UpdatesHandler<TelegramUpdate> for TelegramSource {
    async fn create_source(
        &self,
        db_pool: &Pool,
        updates: &TelegramUpdate,
    ) -> Result<models::Source> {
        match updates {
            TelegramUpdate::File(f) => Err(Error::UpdateNotSupported),
            TelegramUpdate::Message(message) => {
                self.collector
                    .read()
                    .await
                    .join_chat(&message.chat_id)
                    .await?;
                let chann = self
                    .collector
                    .read()
                    .await
                    .get_channel(message.chat_id)
                    .await?;
                if chann.is_none() {
                    return Err(Error::SourceNotFound);
                }
                let s = TelegramSource::channel_to_new_source(chann.unwrap());
                Ok(s.save(db_pool).await?)
            }
        }
    }

    async fn process_updates(&self, db_pool: &Pool, updates: &TelegramUpdate) -> Result<usize> {
        match updates {
            TelegramUpdate::File(file) => {
                self.handle_file_update(db_pool, file).await;
                Ok(1)
            }
            TelegramUpdate::Message(message) => {
                let sources =
                    models::Source::search(db_pool, message.chat_id.to_string().as_str()).await?;
                let source = match sources.len() {
                    0 => self.create_source(db_pool, updates).await?,
                    _ => sources.first().unwrap().clone(),
                };
                let message_id = message.message_id.clone();
                let created = models::NewRecord::update_or_create(
                    db_pool,
                    vec![models::NewRecord {
                        title: None,
                        image: None,
                        date: message
                            .date
                            .map(|d| chrono::NaiveDateTime::from_timestamp(d.clone(), 0)),
                        source_record_id: message_id.to_string(),
                        source_id: source.id,
                        content: message.content.clone(),
                    }],
                )
                .await?;
                // records inserted
                self.handle_record_inserted(
                    db_pool,
                    message.chat_id,
                    message_id,
                    created
                        .into_iter()
                        .map(|r| (r.source_record_id, r.source_id))
                        .collect(),
                )
                .await
            }
        }
    }
}
