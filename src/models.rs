use crate::schema::staff;
use chrono::{DateTime, Local};
use diesel::{
    deserialize::Queryable,
    serialize::{self, ToSql},
    sql_types,
};
use std::{collections::HashMap, fmt};

#[derive(Debug, Clone, Copy)]
pub enum WorkStatus {
    Away,
    Working,
}

impl WorkStatus {
    pub fn from_bool(b: bool) -> Self {
        if b {
            Self::Working
        } else {
            Self::Away
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            WorkStatus::Away => WorkStatus::Working,
            WorkStatus::Working => WorkStatus::Away,
        }
    }
}

impl fmt::Display for WorkStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match &self {
            WorkStatus::Away => "Pause",
            WorkStatus::Working => "An der Arbeit",
        };

        fmt::Display::fmt(str, f)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WorkEvent {
    StatusChange(i32, bool),
    EventStart,
    EventOver,
}

#[derive(Queryable)]
pub struct WorkEventT {
    pub timestamp: DateTime<Local>,
    pub event: WorkEvent,
}

// a.d. TODO derive aschangeset fails if status is my custom WorkStatus boolean. How to fix?
// using sql_type annotation as described below does not work because it is not found
// https://github.com/diesel-rs/diesel/blob/1.4.x/guide_drafts/trait_derives.md#aschangeset
// https://noyez.gitlab.io/post/2018-08-05-a-small-custom-bool-type-in-diesel/
#[derive(Debug, Clone, AsChangeset, Identifiable)]
#[table_name = "staff"]
#[primary_key(uuid)]
pub struct StaffMember {
    uuid: i32,
    pub name: String,
    pub pin: String,
    pub cardid: String,
    pub status: bool,
}

impl StaffMember {
    // fn get_by_pin_or_card_id(
    //     staff: &HashMap<u32, StaffMember>,
    //     ident: &str,
    // ) -> Option<(u32, StaffMember)> {
    //     for (uuid, staff_member) in staff.iter() {
    //         if staff_member.pin == ident || staff_member.card_id == ident {
    //             return Some((*uuid, staff_member.clone()));
    //         }
    //     }
    //     None
    // }

    pub fn get_uuid_by_pin_or_card_id(
        staff: &HashMap<i32, StaffMember>,
        ident: &str,
    ) -> Option<i32> {
        for (uuid, staff_member) in staff.iter() {
            if staff_member.pin == ident || staff_member.cardid == ident {
                return Some(*uuid);
            }
        }
        None
    }

    pub fn to_hash_map(staff_v: Vec<StaffMember>) -> HashMap<i32, StaffMember> {
        let mut staff = HashMap::new();
        for staff_member in staff_v {
            if let Some(old) = staff.insert(staff_member.uuid, staff_member) {
                // TODO impl Display for StaffMember
                panic!("Previous value for uuid {}", old.uuid);
            }
        }
        staff
    }
}

/* Build my own queryable to parse the WorkStatus of a StaffMember. */
/* from https://docs.diesel.rs/diesel/deserialize/trait.Queryable.html */
type DB = diesel::sqlite::Sqlite;

impl Queryable<staff::SqlType, DB> for StaffMember {
    type Row = (i32, String, String, String, bool);

    fn build(row: Self::Row) -> Self {
        StaffMember {
            uuid: row.0,
            name: row.1,
            pin: row.2,
            cardid: row.3,
            status: row.4,
        }
    }
}

impl ToSql<sql_types::Bool, DB> for WorkStatus {
    fn to_sql<W: std::io::Write>(&self, out: &mut serialize::Output<W, DB>) -> serialize::Result {
        match *self {
            WorkStatus::Away => ToSql::<sql_types::Bool, DB>::to_sql(&false, out),
            WorkStatus::Working => ToSql::<sql_types::Bool, DB>::to_sql(&true, out),
        }
    }
}
