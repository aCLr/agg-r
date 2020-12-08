use crate::db::Pool;
use crate::result::Result;
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
}

#[derive(Insertable, Serialize, Deserialize, Clone, Debug)]
#[table_name = "files"]
pub struct NewFile {
    pub record_id: i32,
    pub kind: String,
    pub local_path: Option<String>,
    pub remote_path: String,
    pub remote_id: Option<String>,
    pub file_name: Option<String>,
}

impl NewFile {
    pub async fn update_or_create(
        db_pool: &Pool,
        files_to_insert: Vec<Self>,
    ) -> Result<Vec<(Option<String>, i32)>> {
        let mut key_to_file = files_to_insert
            .into_iter()
            .filter(|f| f.remote_id.is_some())
            .map(|f| ((f.remote_id.clone().unwrap(), f.record_id.clone()), f))
            .collect::<HashMap<(String, i32), Self>>();
        for file in files::table
            .filter(
                files::remote_id.eq_any(
                    key_to_file
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
            key_to_file
                .remove(&(file.remote_id.unwrap(), file.record_id))
                .map(|f| async move {
                    let update_result = diesel::update(files::table.filter(files::id.eq(file_id)))
                        .set((
                            files::local_path.eq(f.local_path.clone()),
                            files::file_name.eq(f.file_name.clone()),
                        ))
                        .execute_async(db_pool)
                        .await;
                    match update_result {
                        Ok(_) => {}
                        Err(e) => error!("{}", e),
                    }
                });
        }

        Ok(diesel::insert_into(files::table)
            .values(key_to_file.values().cloned().collect::<Vec<NewFile>>())
            .on_conflict((files::remote_id, files::record_id))
            .do_nothing()
            .returning((files::remote_id, files::record_id))
            .get_results_async(db_pool)
            .await?)
    }
}
