use async_trait::async_trait;
use mongodb::bson;
use mongodb::{Client, Database};

#[async_trait]
pub trait LogsStore {
    async fn save(&self, type_: String, logs: &str);
}

pub struct MongoLogsStore {
    database: Database,
}

impl MongoLogsStore {
    pub async fn new(connection: &str, database: &str) -> Self {
        let database = Client::with_uri_str(connection)
            .await
            .expect("can't initialize connection to db")
            .database(database);
        Self { database }
    }
}

#[async_trait]
impl LogsStore for MongoLogsStore {
    async fn save(&self, type_: String, logs: &str) {
        let document = match bson::to_bson(&logs) {
            Ok(data) => data.as_document().map(|d| d.clone()),
            Err(err) => {
                warn!("{}", err);
                return;
            }
        };
        match document {
            Some(doc) => {
                match self
                    .database
                    .collection(type_.as_str())
                    .insert_one(doc.clone(), None)
                    .await
                {
                    Ok(_) => {}
                    Err(err) => {
                        warn!("{}", err);
                    }
                };
            }
            None => {}
        }
    }
}
