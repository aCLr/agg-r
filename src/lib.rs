#[macro_use]
extern crate log;
#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

extern crate futures;
extern crate serde;

#[cfg(feature = "logs")]
mod logs;

pub mod aggregator;
pub mod config;
pub mod db;
pub mod error;
mod schema;
mod updates;
