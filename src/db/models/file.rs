use crate::db::Pool;
use crate::result::Result;
use crate::schema::records;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_diesel::*;

#[derive(Queryable, Identifiable, Serialize, Deserialize, Clone, Debug)]
pub struct File {
    pub id: i32,
    pub title: Option<String>,
    pub source_record_id: String,
    pub source_id: i32,
    pub content: String,
    pub date: NaiveDateTime,
    pub image: Option<String>,
    pub external_link: String,
}

#[derive(Insertable, Serialize, Deserialize, Clone, Debug)]
pub struct NewFile {
    pub record_id: i32,
    pub kind: String,
    pub local_path: Option<String>,
    pub remote_path: String,
    pub remote_id: Option<String>,
    pub file_name: Option<String>,
}
