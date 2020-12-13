use super::parsers::channel_to_new_source;
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
            TelegramUpdate::FileDownloadFinished(_) => Err(Error::UpdateNotSupported(
                "FileDownloadFinished".to_string(),
            )),
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
                let s = channel_to_new_source(chann.unwrap());
                Ok(s.save(db_pool).await?)
            }
        }
    }

    async fn process_updates(&self, db_pool: &Pool, updates: &TelegramUpdate) -> Result<usize> {
        match updates {
            TelegramUpdate::FileDownloadFinished(file) => {
                self.handle_file_downloaded(db_pool, file).await?;
                Ok(1)
            }
            TelegramUpdate::Message(message) => {
                let sources =
                    models::Source::search(db_pool, message.chat_id.to_string().as_str()).await?;
                let source = match sources.len() {
                    0 => self.create_source(db_pool, updates).await?,
                    _ => sources.first().unwrap().clone(),
                };
                let message_id = message.message_id;
                let created = models::NewRecord::update_or_create(
                    db_pool,
                    vec![models::NewRecord {
                        title: None,
                        image: None,
                        date: message
                            .date
                            .map(|d| chrono::NaiveDateTime::from_timestamp(d, 0)),
                        source_record_id: message_id.to_string(),
                        source_id: source.id,
                        content: message.content.clone().unwrap_or_default(),
                    }],
                )
                .await?
                .pop();
                match created {
                    None => {
                        if message.files.is_some() {
                            debug!(
                                "skip reaction for a file because record is not new; message: {:?}",
                                message.files
                            );
                        };
                        Ok(0)
                    }
                    Some(rec) if message.files.is_some() => {
                        let files = message.files.as_ref().unwrap();
                        let (handle_file, handle_record) = tokio::join!(
                            self.handle_new_files(db_pool, files, rec.id),
                            self.handle_record_inserted(
                                db_pool,
                                message.chat_id,
                                message_id,
                                vec![(rec.source_record_id, rec.source_id)],
                            )
                        );
                        match handle_file {
                            Ok(_) => {}
                            Err(e) => {
                                error!("{}", e);
                            }
                        };
                        Ok(handle_record?)
                    }
                    Some(rec) if message.files.is_none() => Ok(self
                        .handle_record_inserted(
                            db_pool,
                            message.chat_id,
                            message_id,
                            vec![(rec.source_record_id, rec.source_id)],
                        )
                        .await?),
                    Some(_) => unreachable!("unexpected file: {:?}", message.files),
                }
            }
        }
    }
}
