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

pub mod aggregator;
pub mod config;
pub mod models;
pub mod result;
mod schema;
pub mod storage;
mod tools;
mod updates;
