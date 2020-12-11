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
pub mod db;
pub mod result;
mod schema;
mod tools;
mod updates;
