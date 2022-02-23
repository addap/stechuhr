pub mod models;
pub mod schema;

#[macro_use]
extern crate diesel;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env;

pub fn establish_connection() -> SqliteConnection {
    // TODO what does this accomplish? any side-effects?
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
