use crate::schema::{events, passwords, staff};
use chrono::{Local, NaiveDateTime};
use diesel::deserialize::{self, FromSql, Queryable};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::*;
use lazy_static::lazy_static;
use pbkdf2::password_hash::PasswordHash as PBKDF2Hash;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_lexpr;
use std::str::FromStr;
use std::{error, fmt, io};

#[derive(Debug, Clone)]
pub enum ModelError {
    EmptyName,
    ParsePIN(String),
    ParseCardid(String),
}

impl error::Error for ModelError {}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            ModelError::ParsePIN(pin) => format!("PIN muss aus 4 Ziffern bestehen: \"{}\"", pin),
            ModelError::ParseCardid(cardid) => {
                format!("Dongle-ID muss aus 10 Ziffern bestehen: \"{}\"", cardid)
            }
            ModelError::EmptyName => String::from("Name darf nicht leer sein"),
        };
        f.write_str(&description)
    }
}

#[derive(Debug, PartialEq, Clone, Copy, AsExpression, FromSqlRow, Serialize, Deserialize)]
#[sql_type = "Bool"]
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

    pub fn to_emoji(&self) -> &'static str {
        match self {
            WorkStatus::Away => "resources/cross-mark.png",
            WorkStatus::Working => "resources/check-mark.png",
        }
    }
}

impl fmt::Display for WorkStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            WorkStatus::Away => "Pause",
            WorkStatus::Working => "Arbeit",
        };

        fmt::Display::fmt(str, f)
    }
}

#[derive(Debug, PartialEq, Clone, AsExpression, FromSqlRow, Serialize, Deserialize)]
#[sql_type = "Text"]
pub enum WorkEvent {
    StatusChange(i32, String, WorkStatus),
    EventStart,
    EventOver,
    Info(String),
    Error(String),
}

impl fmt::Display for WorkEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            WorkEvent::StatusChange(_, name, status) => {
                format!("Status von {} wurde auf \"{}\" gesetzt", name, status)
            }
            WorkEvent::EventStart => String::from("Event gestartet"),
            WorkEvent::EventOver => String::from("Event gestoppt"),
            // TODO can we add color with the formatter?
            WorkEvent::Info(msg) => format!("Info: {}", msg),
            WorkEvent::Error(msg) => format!("Error: {}", msg),
        };

        fmt::Display::fmt(&str, f)
    }
}

#[derive(Debug, Clone, AsExpression)]
pub struct WorkEventT {
    #[allow(unused)]
    id: i32,
    pub created_at: NaiveDateTime,
    pub event: WorkEvent,
}

#[derive(Debug, Clone, Insertable, AsExpression)]
#[table_name = "events"]
pub struct NewWorkEventT {
    created_at: NaiveDateTime,
    #[column_name = "event_json"]
    pub event: WorkEvent,
}

impl NewWorkEventT {
    pub fn new(event: WorkEvent) -> Self {
        NewWorkEventT {
            created_at: Local::now().naive_local(),
            event,
        }
    }
}

pub struct PIN;

impl FromStr for PIN {
    type Err = ModelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^\d{4}$").unwrap();
        }
        if RE.is_match(s) {
            Ok(PIN)
        } else {
            Err(ModelError::ParsePIN(s.to_owned()))
        }
    }
}

pub struct Cardid;

impl FromStr for Cardid {
    type Err = ModelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^\d{10}$").unwrap();
        }
        if RE.is_match(s) {
            Ok(Cardid)
        } else {
            Err(ModelError::ParseCardid(s.to_owned()))
        }
    }
}

// a.d. DONE derive aschangeset fails if status is my custom WorkStatus boolean. How to fix?
// using sql_type annotation as described below does not work because it is not found
// https://github.com/diesel-rs/diesel/blob/1.4.x/guide_drafts/trait_derives.md#aschangeset
// https://noyez.gitlab.io/post/2018-08-05-a-small-custom-bool-type-in-diesel/
#[derive(Debug, Clone, AsExpression, AsChangeset, Identifiable)]
#[table_name = "staff"]
#[primary_key(uuid)]
pub struct StaffMember {
    uuid: i32,
    pub name: String,
    pub pin: String,
    pub cardid: String,
    pub status: WorkStatus,
}

#[derive(Debug, Clone, Insertable)]
#[table_name = "staff"]
pub struct NewStaffMember {
    // TODO how to return strig reference from getter? Lifetime annotation on &str did not work
    pub name: String,
    pub pin: String,
    pub cardid: String,
}

impl StaffMember {
    pub fn uuid(&self) -> i32 {
        self.uuid
    }

    pub fn get_by_card_id<'a>(staff: &'a [Self], cardid: &str) -> Option<&'a Self> {
        for staff_member in staff {
            if staff_member.cardid == cardid {
                return Some(staff_member);
            }
        }
        None
    }

    // DONE can I use lifetimes to return a reference to the staffmember?
    // yes, by adding the generic lifetime parameter everywhere
    // TODO is it possible/useful/necessary to "pull out" the lifetime from the Option type?
    // INVARIANT: pins and cardids are disjoint
    pub fn get_by_pin_or_card_id<'a>(staff: &'a [Self], ident: &str) -> Option<&'a Self> {
        for staff_member in staff {
            if staff_member.pin == ident || staff_member.cardid == ident {
                return Some(staff_member);
            }
        }
        None
    }

    pub fn get_by_uuid_mut<'a>(staff: &'a mut Vec<Self>, uuid: i32) -> Option<&'a mut Self> {
        for staff_member in staff.iter_mut() {
            if staff_member.uuid == uuid {
                return Some(staff_member);
            }
        }
        None
    }

    pub fn get_by_uuid<'a>(staff: &'a [Self], uuid: i32) -> Option<&'a Self> {
        for staff_member in staff {
            if staff_member.uuid == uuid {
                return Some(staff_member);
            }
        }
        None
    }
}

impl NewStaffMember {
    pub fn validate(name: &str, pin: &str, cardid: &str) -> Result<(), ModelError> {
        if name.is_empty() {
            return Err(ModelError::EmptyName);
        }
        let _ = pin.parse::<PIN>()?;
        let _ = cardid.parse::<Cardid>()?;

        Ok(())
    }

    pub fn new(name: String, pin: String, cardid: String) -> Result<Self, ModelError> {
        Self::validate(&name, &pin, &cardid)?;

        Ok(Self { name, pin, cardid })
    }
}

/// A pbkdf2 password hash string in PHC format.
/// TODO could already parse PHC string in Queryable
#[derive(Debug, AsExpression, Insertable)]
#[table_name = "passwords"]
pub struct PasswordHash {
    phc: String,
}

impl PasswordHash {
    pub fn new(phc: String) -> Self {
        let parsed_hash = PBKDF2Hash::new(&phc).expect(&format!("Error parsing hash {}", phc));
        match (parsed_hash.salt, parsed_hash.hash) {
            (None, _) | (_, None) => panic!("hash or salt missing {}", phc),
            _ => Self { phc },
        }
    }

    pub fn hash(&self) -> PBKDF2Hash {
        PBKDF2Hash::new(&self.phc).expect(&format!("Error parsing hash {}", self.phc))
    }
}

/* Build my own queryable to parse the WorkStatus of a StaffMember.
 * But since status is now a simple boolean, it could also be derived. */
/* from https://docs.diesel.rs/diesel/deserialize/trait.Queryable.html */
// type DB = diesel::sqlite::Sqlite;
use diesel::backend::Backend;

impl<DB> Queryable<staff::SqlType, DB> for StaffMember
where
    DB: Backend,
    bool: FromSql<Bool, DB>,
    String: FromSql<Text, DB>,
    i32: FromSql<Integer, DB>,
    WorkStatus: FromSql<Bool, DB>,
{
    type Row = (i32, String, String, String, WorkStatus);

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

impl<DB> Queryable<passwords::SqlType, DB> for PasswordHash
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
    String: FromSql<Text, DB>,
{
    type Row = (i32, String);

    fn build(row: Self::Row) -> Self {
        PasswordHash::new(row.1)
    }
}

impl<DB> Queryable<events::SqlType, DB> for WorkEventT
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
    NaiveDateTime: FromSql<Timestamp, DB>,
    WorkEvent: FromSql<Text, DB>,
{
    type Row = (i32, NaiveDateTime, WorkEvent);

    fn build(row: Self::Row) -> Self {
        WorkEventT {
            id: row.0,
            created_at: row.1,
            event: row.2,
        }
    }
}

impl<DB> ToSql<Bool, DB> for WorkStatus
where
    DB: Backend,
    bool: ToSql<Bool, DB>,
{
    fn to_sql<W: std::io::Write>(&self, out: &mut serialize::Output<W, DB>) -> serialize::Result {
        match *self {
            WorkStatus::Away => ToSql::<Bool, DB>::to_sql(&false, out),
            WorkStatus::Working => ToSql::<Bool, DB>::to_sql(&true, out),
        }
    }
}

impl<DB> FromSql<Bool, DB> for WorkStatus
where
    DB: Backend,
    bool: FromSql<Bool, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let value = bool::from_sql(bytes)?;
        Ok(WorkStatus::from_bool(value))
    }
}

impl<DB> ToSql<Text, DB> for WorkEvent
where
    DB: Backend,
{
    fn to_sql<W>(&self, out: &mut Output<W, DB>) -> serialize::Result
    where
        W: io::Write,
    {
        let value = serde_lexpr::to_string(self)?;
        <String as ToSql<Text, DB>>::to_sql(&value, out)
    }
}

impl<DB> FromSql<Text, DB> for WorkEvent
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let value = String::from_sql(bytes)?;
        Ok(serde_lexpr::from_str(&value)?)
    }
}

// impl<DB: Backend> FromSql<SmallInt, DB> for RecordType
// where
//     i16: FromSql<SmallInt, DB>,
// {
//     fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
//         let v = i16::from_sql(bytes)?;
//         Ok(match v {
//             1 => RecordType::A,
//             2 => RecordType::B,
//             _ => return Err("replace me with a real error".into()),
//         })
//     }
// }
