use crate::result::Result;
use crate::schema::records;
use crate::storage::Pool;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_diesel::*;

#[derive(Queryable, Identifiable, Serialize, Deserialize, Clone, Debug)]
pub struct Record {
    pub id: i32,
    pub title: Option<String>,
    pub source_record_id: String,
    pub source_id: i32,
    pub content: String,
    pub date: NaiveDateTime,
    pub image: Option<String>,
    pub external_link: String,
}

impl Record {
    pub async fn set_external_ink(
        db_pool: &Pool,
        source_record_id: String,
        source_id: i32,
        external_link: String,
    ) -> Result<usize> {
        Ok(diesel::update(
            records::table.filter(
                records::source_record_id
                    .eq(source_record_id)
                    .and(records::source_id.eq(source_id)),
            ),
        )
        .set(records::external_link.eq(external_link.clone()))
        .execute_async(db_pool)
        .await?)
    }

    pub async fn get_all(db_pool: &Pool, limit: i64, offset: i32) -> Result<Vec<Self>> {
        Ok(records::table
            .order(records::date.desc())
            .limit(limit)
            .offset(offset.into())
            .load_async::<Record>(db_pool)
            .await?)
    }

    pub async fn get_filtered(
        db_pool: &Pool,
        source_id: i32,
        limit: i64,
        offset: i32,
    ) -> Result<Vec<Self>> {
        Ok(records::table
            .filter(records::source_id.eq(source_id))
            .order(records::date.desc())
            .limit(limit)
            .offset(offset.into())
            .load_async::<Record>(db_pool)
            .await?)
    }
}
#[derive(Debug, Insertable, Clone)]
#[table_name = "records"]
pub struct NewRecord {
    pub title: Option<String>,
    // TODO: add date, modify date (for app, not fo source)
    pub source_record_id: String,
    pub source_id: i32,
    pub content: String,
    pub date: Option<NaiveDateTime>,
    pub image: Option<String>,
}

impl NewRecord {
    pub async fn update_or_create(
        pool: &Pool,
        records_to_insert: Vec<Self>,
    ) -> Result<Vec<Record>> {
        // TODO: do we need to return updated rows?
        let mut key_to_rec = records_to_insert
            .into_iter()
            .map(|f| ((f.source_record_id.clone(), f.source_id), f))
            .collect::<HashMap<(String, i32), Self>>();
        for record in records::table
            .filter(
                records::source_record_id.eq_any(
                    key_to_rec
                        .keys()
                        .into_iter()
                        .map(|(sri, _)| sri.clone())
                        .collect::<Vec<String>>(),
                ),
            )
            .load_async::<Record>(pool)
            .await?
        {
            let record_id = record.id;
            key_to_rec
                .remove(&(record.source_record_id, record.source_id))
                .map(|r| async move {
                    let update_result =
                        diesel::update(records::table.filter(records::id.eq(record_id)))
                            .set((
                                records::title.eq(r.title.clone()),
                                records::content.eq(r.content.clone()),
                                records::image.eq(r.image.clone()),
                            ))
                            .execute_async(pool)
                            .await;
                    match update_result {
                        Ok(_) => {}
                        Err(e) => error!("{}", e),
                    }
                });
        }

        Ok(diesel::insert_into(records::table)
            .values(key_to_rec.values().cloned().collect::<Vec<NewRecord>>())
            .on_conflict((records::source_record_id, records::source_id))
            .do_nothing()
            .get_results_async(pool)
            .await?)
    }
}
