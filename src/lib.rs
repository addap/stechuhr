pub mod date_ext;
pub mod models;
pub mod schema;

#[macro_use]
extern crate diesel;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use dotenv::dotenv;
use models::{NewStaffMember, PasswordHash, StaffMember, WorkEventT};
use pbkdf2::{password_hash::PasswordVerifier, Pbkdf2};
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

pub fn update_staff_member(staff_member: &StaffMember, connection: &SqliteConnection) {
    diesel::update(staff_member)
        .set(staff_member)
        .execute(connection)
        .expect(&format!("Error updating staff {}", staff_member.name));
}

pub fn update_staff(staff_v: &Vec<StaffMember>, connection: &SqliteConnection) {
    for staff_member in staff_v.iter() {
        update_staff_member(staff_member, connection);
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

pub fn save_event(new_event: &WorkEventT, connection: &SqliteConnection) {
    use schema::events::dsl::*;

    diesel::insert_into(events)
        .values(new_event)
        .execute(connection)
        .expect("Error inserting new event");
}

pub fn load_events(
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
    connection: &SqliteConnection,
) -> Vec<WorkEventT> {
    use schema::events::dsl::*;

    let evts = events
        .filter(created_at.ge(start_time))
        .filter(created_at.lt(end_time))
        .order_by(created_at.asc())
        .load::<WorkEventT>(connection)
        .expect("Error loading events");

    evts
}

pub fn save_password(new_password: PasswordHash, connection: &SqliteConnection) {
    use schema::passwords::dsl::*;

    diesel::insert_into(passwords)
        .values(&new_password)
        .execute(connection)
        .expect("Error inserting new pasword");
}

pub fn verify_password(password: &str, connection: &SqliteConnection) -> bool {
    use schema::passwords::dsl::*;

    let pws = passwords
        .load::<PasswordHash>(connection)
        .expect("Error loading passwords");

    for pw in &pws {
        if Pbkdf2
            .verify_password(password.as_ref(), &pw.hash())
            .is_ok()
        {
            return true;
        }
    }

    return false;
}
