use crate::db::Pool;
use crate::error::Result;
use crate::schema::records;
use chrono::NaiveDateTime;
use diesel::pg::upsert::excluded;
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use tokio_diesel::*;

#[derive(Queryable, Serialize, Deserialize, Clone, Debug)]
pub struct Record {
    pub id: i32,
    pub title: Option<String>,
    pub guid: String,
    pub source_id: i32,
    pub content: String,
    pub date: NaiveDateTime,
    pub image: Option<String>,
}

impl Record {
    pub async fn get_all(db_pool: &Pool, limit: i64, offset: i32) -> Result<Vec<Self>> {
        Ok(records::table
            .order(records::date.desc())
            .limit(limit.into())
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
            .limit(limit.into())
            .offset(offset.into())
            .load_async::<Record>(db_pool)
            .await?)
    }
}
#[derive(Debug, Insertable)]
#[table_name = "records"]
pub struct NewRecord {
    pub title: Option<String>,
    pub guid: String,
    pub source_id: i32,
    pub content: String,
    pub date: NaiveDateTime,
    pub image: Option<String>,
}

impl NewRecord {
    pub async fn update_or_create(pool: &Pool, records: Vec<Self>) -> Result<usize> {
        Ok(diesel::insert_into(records::table)
            .values(records)
            .on_conflict((records::guid, records::source_id))
            .do_update()
            .set(records::content.eq(excluded(records::content)))
            .execute_async(pool)
            .await?)
    }
}
