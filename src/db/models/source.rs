use crate::db::Pool;
use crate::result::Result;
use crate::schema::sources;
use chrono::NaiveDateTime;
use diesel::dsl::IntervalDsl;
use diesel::expression::functions::date_and_time::now;
use diesel::pg::upsert::excluded;
use diesel::prelude::*;
use diesel::{update, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use tokio_diesel::*;

#[derive(Queryable, Serialize, Deserialize, Clone, Debug)]
pub struct Source {
    pub id: i32,
    pub name: String,
    pub origin: String,
    pub kind: String,
    pub image: Option<String>,
    pub last_scrape_time: NaiveDateTime,
    pub external_link: String,
}

impl Source {
    pub async fn set_scraped_now(&self, pool: &Pool) -> Result<()> {
        update(sources::table.filter(sources::id.eq(self.id)))
            .set(sources::last_scrape_time.eq(now))
            .execute_async(pool)
            .await?;
        Ok(())
    }
    pub async fn get_list(pool: &Pool) -> Result<Vec<Self>> {
        Ok(sources::table.load_async::<Self>(&pool).await?)
    }

    pub async fn search(pool: &Pool, origin: &str) -> Result<Vec<Self>> {
        let like = format!("%{}%", origin);
        let source = sources::table
            .filter(
                sources::origin.like(like.clone()).or(sources::external_link
                    .like(like.clone())
                    .or(sources::name.like(like.clone()))),
            )
            .get_results_async::<Self>(pool)
            .await;
        match source {
            Err(te) => match &te {
                tokio_diesel::AsyncError::Error(de) => match de {
                    diesel::NotFound => Ok(vec![]),
                    _ => Err(te.into()),
                },
                _ => Err(te.into()),
            },
            Ok(s) => Ok(s),
        }
    }

    pub async fn get_by_kind(pool: &Pool, kind: String) -> Result<Vec<Self>> {
        Ok(sources::table
            .filter(sources::kind.eq(kind))
            .load_async::<Self>(pool)
            .await?)
    }

    pub async fn get_by_kind_for_scrape(
        pool: &Pool,
        kind: String,
        check_secs_interval: &i32,
    ) -> Result<Vec<Self>> {
        Ok(sources::table
            .filter(
                sources::kind
                    .eq(kind)
                    .and(sources::last_scrape_time.le(now - check_secs_interval.second())),
            )
            .load_async::<Self>(pool)
            .await?)
    }
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "sources"]
pub struct NewSource {
    pub name: String,
    pub origin: String,
    pub kind: String,
    pub image: Option<String>,
    pub external_link: String,
}

impl NewSource {
    pub async fn save_bulk(pool: &Pool, sources: Vec<Self>) -> Result<Vec<Source>> {
        Ok(diesel::insert_into(sources::table)
            .values(sources)
            .on_conflict((sources::origin, sources::kind))
            .do_update()
            .set(sources::name.eq(excluded(sources::name)))
            .get_results_async::<Source>(pool)
            .await?)
    }

    pub async fn save(&self, pool: &Pool) -> Result<Source> {
        Ok(NewSource::save_bulk(pool, vec![self.clone()])
            .await?
            .first()
            .cloned()
            .unwrap())
    }
}
