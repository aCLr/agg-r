use crate::schema::files;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};

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
