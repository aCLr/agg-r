use crate::models;
use crate::result::Result;
use async_trait::async_trait;

use crate::models::{File, NewFile, NewRecord, NewSource, Source};
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, Pool as _Pool},
};

pub type Pool = _Pool<ConnectionManager<PgConnection>>;

embed_migrations!();

pub fn migrate(db_pool: &Pool) -> Result<(), diesel_migrations::RunMigrationsError> {
    let connection = db_pool.get().expect("can't get connection from pool");

    // This will run the necessary migrations.
    embedded_migrations::run(&connection)?;
    Ok(())
}

#[async_trait]
pub trait Storage {
    async fn save_file(&self, file: models::File) -> Result<()>;
    async fn get_file_by_remote_id(&self, remote_id: String) -> Result<Option<models::File>>;
    async fn save_files(&self, files: Vec<models::NewFile>) -> Result<()>;

    async fn set_record_external_link(
        &self,
        source_record_id: String,
        source_id: i32,
        external_link: String,
    ) -> Result<usize>;
    async fn save_records(&self, records: Vec<models::NewRecord>) -> Result<Vec<models::Record>>;

    async fn set_source_scraped_now(&self, source: models::Source) -> Result<()>;
    async fn search_source(&self, query: &str) -> Result<()>;
    async fn get_sources_by_kind(&self, kind: String) -> Result<Vec<models::Source>>;
    async fn get_sources_by_kind_for_scrape(
        &self,
        kind: String,
        check_secs_interval: &i32,
    ) -> Result<Vec<models::Source>>;
    async fn save_sources(&self, sources: Vec<models::NewSource>) -> Result<Vec<models::Source>>;
}

pub struct St {}

#[async_trait]
impl Storage for St {
    async fn save_file(&self, file: File) -> Result<()> {
        unimplemented!()
    }

    async fn get_file_by_remote_id(&self, remote_id: String) -> Result<Option<File>> {
        unimplemented!()
    }

    async fn save_files(&self, files: Vec<NewFile>) -> Result<()> {
        unimplemented!()
    }

    async fn set_record_external_link(
        &self,
        source_record_id: String,
        source_id: i32,
        external_link: String,
    ) -> Result<usize> {
        unimplemented!()
    }

    async fn save_records(&self, records: Vec<NewRecord>) -> Result<Vec<models::Record>> {
        unimplemented!()
    }

    async fn set_source_scraped_now(&self, source: Source) -> Result<()> {
        unimplemented!()
    }

    async fn search_source(&self, query: &str) -> Result<()> {
        unimplemented!()
    }

    async fn get_sources_by_kind(&self, kind: String) -> Result<Vec<models::Source>> {
        unimplemented!()
    }

    async fn get_sources_by_kind_for_scrape(
        &self,
        kind: String,
        check_secs_interval: &i32,
    ) -> Result<Vec<models::Source>> {
        unimplemented!()
    }

    async fn save_sources(&self, sources: Vec<NewSource>) -> Result<Vec<models::Source>> {
        unimplemented!()
    }
}
