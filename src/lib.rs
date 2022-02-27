pub mod models;
pub mod schema;

#[macro_use]
extern crate diesel;
use diesel::prelude::*;
use dotenv::dotenv;
use models::{NewStaffMember, StaffMember, WorkEventT};
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
    for staff_member in staff_v.iter() {
        diesel::update(staff_member)
            .set(staff_member)
            .execute(connection)
            .expect(&format!("Error updating staff {}", staff_member.name));
    }
}

pub fn insert_staff(staff_member: NewStaffMember, connection: &SqliteConnection) -> StaffMember {
    // TODO uniqueness checks, so return Result
    use schema::staff::dsl::*;

    diesel::insert_into(staff)
        .values(&staff_member)
        .execute(connection)
        .expect(&format!("Error inserting new staff {}", staff_member.name));

    let mut newly_inserted = staff
        .filter(pin.eq(staff_member.pin))
        .limit(1)
        .load::<StaffMember>(connection)
        .expect(&format!(
            "Error loading newly inserted staff {}",
            staff_member.name
        ));

    let newly_inserted = newly_inserted.remove(0);

    newly_inserted
}

pub fn save_event(event: WorkEventT, connection: &SqliteConnection) {
    use schema::events::dsl::*;

    diesel::insert_into(events)
        .values(&event)
        .execute(connection)
        .expect("Error inserting new event");
}

pub fn print_events(connection: &SqliteConnection) {
    use schema::events::dsl::*;

    let evts = events
        .load::<WorkEventT>(connection)
        .expect("Error reading events");

    println!("Events: {:?}", evts);
}
