pub mod models;
pub mod schema;

#[macro_use]
extern crate diesel;
use diesel::prelude::*;
use dotenv::dotenv;
use models::StaffMember;
use std::env;

pub fn establish_connection() -> SqliteConnection {
    // TODO what does this accomplish? any side-effects?
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn load_staff(connection: &SqliteConnection) -> Vec<StaffMember> {
    use schema::staff::dsl::*;
    staff
        .load::<StaffMember>(connection)
        .expect("Error loading staff from DB")
}

pub fn save_staff(staff_v: &Vec<StaffMember>, connection: &SqliteConnection) {
    use schema::staff::dsl::*;

    for staff_member in staff_v.iter() {
        diesel::update(staff_member)
            .set(status.eq(staff_member.status))
            .execute(connection)
            .expect(&format!("Error updating staff {}", staff_member.name));
    }
}
