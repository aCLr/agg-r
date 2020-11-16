use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, Pool as _Pool},
};
pub mod models;

pub type Pool = _Pool<ConnectionManager<PgConnection>>;

embed_migrations!();

pub fn migrate(db_pool: &Pool) -> Result<(), diesel_migrations::RunMigrationsError> {
    let connection = db_pool.get().expect("can't get connection from pool");

    // This will run the necessary migrations.
    embedded_migrations::run(&connection)?;
    Ok(())
}
