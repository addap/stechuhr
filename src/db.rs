use crate::models::{
    LoadStaffMember, NewStaffMember, NewWorkEventT, PasswordHash, StaffMember, WorkEvent,
    WorkEventT, WorkStatus,
};
use crate::schema;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use pbkdf2::{password_hash::PasswordVerifier, Pbkdf2};
use std::env;

pub fn establish_connection() -> SqliteConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

///*************************/
/// Loading
///*************************/

/// Load a staff member from the database.
fn load_staff(connection: &SqliteConnection) -> Vec<LoadStaffMember> {
    use schema::staff::dsl::*;
    staff
        .filter(is_active.eq(true))
        .load::<LoadStaffMember>(connection)
        .expect("Error loading staff from DB")
}

/// Load all events from the database.
fn load_events(connection: &SqliteConnection) -> Vec<WorkEventT> {
    use schema::events::dsl::*;

    let evts = events
        .order_by(created_at.asc())
        .load::<WorkEventT>(connection)
        .expect("Error loading events");

    evts
}

/// Load all events in the specified range from the database.
pub fn load_events_between(
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

pub fn load_state(connection: &SqliteConnection) -> Vec<StaffMember> {
    let loaded_staff = load_staff(connection);
    let events = load_events(connection);
    let staff = staff_compute_status(loaded_staff, &events);

    staff
}

///*************************/
/// Saving
///*************************/

/// Save a single staff member into the database.
pub fn save_staff_member(
    staff_member: &StaffMember,
    connection: &SqliteConnection,
) -> QueryResult<()> {
    let staff_member = LoadStaffMember::from(staff_member);

    diesel::update(&staff_member)
        .set(&staff_member)
        .execute(connection)?;
    Ok(())
}

pub fn save_staff(staff_v: &[StaffMember], connection: &SqliteConnection) -> QueryResult<()> {
    for staff_member in staff_v {
        save_staff_member(staff_member, connection)?;
    }
    Ok(())
}

///*************************/
/// Inserting
///*************************/

pub fn insert_staff(
    staff_member: NewStaffMember,
    connection: &SqliteConnection,
) -> QueryResult<StaffMember> {
    // TODO uniqueness checks, so return Result
    use schema::staff::dsl::*;

    diesel::insert_into(staff)
        .values(&staff_member)
        .execute(connection)?;

    let mut newly_inserted = staff
        .order_by(id.desc())
        .limit(1)
        .load::<LoadStaffMember>(connection)?;

    let newly_inserted = newly_inserted.remove(0);

    Ok(newly_inserted.with_status(WorkStatus::Away))
}

pub fn insert_event(new_event: &NewWorkEventT, connection: &SqliteConnection) -> WorkEventT {
    use schema::events::dsl::*;

    diesel::insert_into(events)
        .values(new_event)
        .execute(connection)
        .expect("Error inserting new event");

    let mut newly_inserted = events
        .order_by(id.desc())
        .limit(1)
        .load::<WorkEventT>(connection)
        .expect("Error loading newly inserted event");

    let newly_inserted = newly_inserted.remove(0);

    newly_inserted
}

pub fn insert_password(new_password: PasswordHash, connection: &SqliteConnection) {
    use schema::passwords::dsl::*;

    diesel::insert_into(passwords)
        .values(&new_password)
        .execute(connection)
        .expect("Error inserting new pasword");
}

///*************************/
/// Other Queries
///*************************/

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

fn staff_compute_status(staff: Vec<LoadStaffMember>, events: &[WorkEventT]) -> Vec<StaffMember> {
    staff
        .into_iter()
        .map(|staff_member| {
            for eventt in events.iter().rev() {
                match eventt.event {
                    WorkEvent::StatusChange(id, _, status) if id == staff_member.uuid() => {
                        return staff_member.with_status(status);
                    }
                    WorkEvent::EventOver => {
                        return staff_member.with_status(WorkStatus::Away);
                    }
                    _ => {}
                }
            }

            return staff_member.with_status(WorkStatus::Away);
        })
        .collect()
}

pub fn delete_staff_member(
    staff_member: StaffMember,
    connection: &SqliteConnection,
) -> QueryResult<()> {
    use schema::staff::dsl::*;

    let staff_member = LoadStaffMember::from(staff_member);

    diesel::update(&staff_member)
        .set((
            is_active.eq(false),
            pin.eq(None::<String>),
            cardid.eq(None::<String>),
        ))
        .execute(connection)?;

    Ok(())
}
