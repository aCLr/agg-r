use crate::db::Pool;
use crate::result::{Error, Result};
use crate::schema::files;
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_diesel::*;

#[derive(Queryable, Identifiable, Serialize, Deserialize, Clone, Debug)]
pub struct File {
    pub id: i32,
    pub record_id: i32,
    pub kind: String,
    pub local_path: Option<String>,
    pub remote_path: String,
    pub remote_id: Option<String>,
    pub file_name: Option<String>,
    #[column_name = "type"]
    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub type_: String,
    pub meta: Option<String>,
}

impl File {
    pub async fn save(&self, db_pool: &Pool) -> Result<()> {
        diesel::update(files::table.filter(files::id.eq(self.id)))
            .set((
                files::local_path.eq(self.local_path.clone()),
                files::file_name.eq(self.file_name.clone()),
            ))
            .execute_async(db_pool)
            .await?;
        Ok(())
    }
    pub async fn get_file_by_remote_id(db_pool: &Pool, remote_id: String) -> Result<Option<Self>> {
        match files::table
            .filter(files::remote_id.eq(remote_id.clone()))
            .get_results_async(db_pool)
            .await
        {
            Ok(mut found_files) if found_files.len() == 1 => Ok(found_files.pop()),
            Ok(found_files) if found_files.len() > 1 => Err(Error::DbError(format!(
                "found multiple files for id {}",
                remote_id
            ))),
            Ok(found_files) if found_files.len() == 0 => Ok(None),
            Ok(r) => unreachable!("unexpected result: {:?}", r),
            Err(tokio_diesel::AsyncError::Error(diesel::NotFound)) => Ok(None),
            Err(e) => Err(e)?,
        }
    }
}

#[derive(Debug, Insertable, Clone)]
#[table_name = "files"]
pub struct NewFile {
    pub record_id: i32,
    pub kind: String,
    pub local_path: Option<String>,
    pub remote_path: String,
    pub remote_id: Option<String>,
    pub file_name: Option<String>,
    pub type_: String,
    pub meta: Option<String>,
}

impl NewFile {
    pub async fn update_or_create(db_pool: &Pool, files_to_insert: Vec<Self>) -> Result<()> {
        let mut key_to_files: HashMap<(String, i32), Vec<Self>> = HashMap::new();
        for file in files_to_insert {
            let key = (file.remote_id.clone().unwrap(), file.record_id.clone());
            let existed = key_to_files.get_mut(&key);
            match existed {
                Some(files) => {
                    files.push(file);
                }
                None => {
                    key_to_files.insert(key, vec![file]);
                }
            }
        }
        for file in files::table
            .filter(
                files::remote_id.eq_any(
                    key_to_files
                        .keys()
                        .into_iter()
                        .map(|(ri, _)| ri.clone())
                        .collect::<Vec<String>>(),
                ),
            )
            .load_async::<File>(db_pool)
            .await?
        {
            let file_id = file.id.clone();
            key_to_files
                .remove(&(file.remote_id.unwrap(), file.record_id))
                .map(|files| async move {
                    for file in files {
                        let update_result =
                            diesel::update(files::table.filter(files::id.eq(file_id)))
                                .set((
                                    files::local_path.eq(file.local_path.clone()),
                                    files::file_name.eq(file.file_name.clone()),
                                ))
                                .execute_async(db_pool)
                                .await;
                        match update_result {
                            Ok(_) => {}
                            Err(e) => error!("{}", e),
                        }
                    }
                });
        }
        let mut to_insert = vec![];
        for files in key_to_files.values() {
            to_insert.extend(files.clone())
        }
        diesel::insert_into(files::table)
            .values(to_insert)
            .on_conflict(files::remote_id)
            .do_nothing()
            .execute_async(db_pool)
            .await?;
        Ok(())
    }
}
