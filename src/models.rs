use crate::icons::{self, FONT_EMOJIONE, TEXT_SIZE_EMOJI};
use crate::schema::{events, passwords, staff};
use chrono::{Local, NaiveDateTime};
use diesel::deserialize::{self, FromSql, Queryable};
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::*;
use iced::Color;
use pbkdf2::password_hash::PasswordHash as PBKDF2Hash;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_lexpr;
use std::borrow::Cow;
use std::str::FromStr;
use std::{cmp, error, fmt};

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

#[derive(
    Debug, PartialEq, Eq, PartialOrd, Clone, Copy, AsExpression, FromSqlRow, Serialize, Deserialize,
)]
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

    pub fn to_unicode(&self) -> iced::Text {
        match self {
            WorkStatus::Away => icons::icon(
                icons::emoji::crossmark
                    .with_font(FONT_EMOJIONE)
                    .with_color(Some(Color::from_rgb8(0xFF, 0x00, 0x00)))
                    .with_size(TEXT_SIZE_EMOJI + 4),
            ),
            WorkStatus::Working => icons::icon(
                icons::emoji::checkmark
                    .with_font(FONT_EMOJIONE)
                    .with_color(Some(Color::from_rgb8(0x00, 0xA4, 0x07)))
                    .with_size(TEXT_SIZE_EMOJI + 4),
            ),
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

#[derive(
    Debug, PartialEq, Eq, PartialOrd, Clone, AsExpression, FromSqlRow, Serialize, Deserialize,
)]
#[sql_type = "Text"]
pub enum WorkEvent {
    StatusChange(i32, String, WorkStatus),
    #[deprecated]
    EventStart,
    #[deprecated]
    EventOver,
    _6am,
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
            WorkEvent::_6am => String::from("6 Uhr morgens"),
            WorkEvent::Info(msg) => format!("Info: {}", msg),
            WorkEvent::Error(msg) => format!("Error: {}", msg),
        };

        fmt::Display::fmt(&str, f)
    }
}

// derive AsExpression
#[derive(Debug, Clone, Queryable, PartialEq, Eq, PartialOrd)]
pub struct WorkEventT {
    #[allow(unused)]
    id: i32,
    pub created_at: NaiveDateTime,
    pub event: WorkEvent,
}

impl Ord for WorkEventT {
    // Reverse ordering for timestamp so that the max-heap gives us the earliest events first.
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        other
            .created_at
            .cmp(&self.created_at)
            // Can stop comparing after id since two different items coming from the db will always have different ids.
            .then(self.id.cmp(&other.id))
    }
}

// derive AsExpression
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = events)]
pub struct NewWorkEventT {
    created_at: NaiveDateTime,
    #[column_name = "event_json"]
    pub event: WorkEvent,
}

impl NewWorkEventT {
    pub fn new(created_at: NaiveDateTime, event: WorkEvent) -> Self {
        NewWorkEventT { created_at, event }
    }

    pub fn now(event: WorkEvent) -> Self {
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
        let re = Regex::new(r"^[A-Za-z0-9]{4}$").unwrap();
        if re.is_match(s) {
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
        let re = Regex::new(r"^\d{10}$").unwrap();
        if re.is_match(s) {
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
// derive AsChangeset
#[derive(Debug, Clone, AsChangeset, Identifiable)]
#[table_name = "staff"]
#[primary_key(uuid)]
pub struct DBStaffMember {
    uuid: i32,
    name: String,
    pin: String,
    cardid: String,
    is_visible: bool,
}

impl DBStaffMember {
    pub fn uuid(&self) -> i32 {
        self.uuid
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn with_status(self, status: WorkStatus) -> StaffMember {
        StaffMember {
            uuid: self.uuid,
            name: self.name,
            pin: self.pin,
            cardid: self.cardid,
            is_visible: self.is_visible,
            status,
        }
    }
}

/// The actual staff member that is used in the program.
/// status is computed based on the work events
#[derive(Debug, Clone)]
pub struct StaffMember {
    uuid: i32,
    pub name: String,
    pub pin: String,
    pub cardid: String,
    pub status: WorkStatus,
    pub is_visible: bool,
}

// DONE for save_staff_member I need a DBStaffMember so I have to convert the &StaffMember to an owned value, which is uneccessary.
// How can I implement AsChangeset for StaffMember directly?
// -> Implementing AsChangeset manually is pretty ugly. So I just use another type for it which seems to be the recommended method. (https://github.com/diesel-rs/diesel/blob/master/guide_drafts/trait_derives.md#aschangeset)
impl<'a> From<Cow<'a, StaffMember>> for DBStaffMember {
    fn from(staff_member: Cow<StaffMember>) -> Self {
        let staff_member = staff_member.into_owned();

        Self {
            uuid: staff_member.uuid,
            name: staff_member.name,
            pin: staff_member.pin,
            cardid: staff_member.cardid,
            is_visible: staff_member.is_visible,
        }
    }
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

    /// INVARIANT: pins and cardids are disjoint
    pub fn get_by_pin_or_card_id<'a>(staff: &'a [Self], ident: &str) -> Option<&'a Self> {
        staff
            .iter()
            .find(|staff_member| staff_member.pin == ident || staff_member.cardid == ident)
    }

    pub fn get_by_uuid_mut<'a>(staff: &'a mut [Self], uuid: i32) -> Option<&'a mut Self> {
        staff
            .iter_mut()
            .find(|staff_member| staff_member.uuid == uuid)
    }

    pub fn get_by_uuid<'a>(staff: &'a [Self], uuid: i32) -> Option<&'a Self> {
        staff.iter().find(|staff_member| staff_member.uuid == uuid)
    }
}

#[derive(Debug, Clone, Insertable)]
#[table_name = "staff"]
pub struct NewStaffMember {
    pub name: String,
    pub pin: String,
    pub cardid: String,
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
// derive AsExpression
#[derive(Debug, Insertable)]
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
 * from https://docs.diesel.rs/diesel/deserialize/trait.Queryable.html */
use diesel::backend;

impl<DB> Queryable<staff::SqlType, DB> for DBStaffMember
where
    DB: backend::Backend,
    bool: FromSql<Bool, DB>,
    String: FromSql<Text, DB>,
    i32: FromSql<Integer, DB>,
{
    type Row = (i32, String, Option<String>, Option<String>, bool, bool);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        let pin = row.2.unwrap();
        let cardid = row.3.unwrap();

        Ok(Self {
            uuid: row.0,
            name: row.1,
            pin,
            cardid,
            is_visible: row.4,
        })
    }
}

impl<DB> Queryable<passwords::SqlType, DB> for PasswordHash
where
    DB: backend::Backend,
    i32: FromSql<Integer, DB>,
    String: FromSql<Text, DB>,
{
    type Row = (i32, String);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        Ok(PasswordHash::new(row.1))
    }
}

impl<DB> ToSql<Bool, DB> for WorkStatus
where
    DB: backend::Backend,
    bool: ToSql<Bool, DB>,
{
    fn to_sql(&self, out: &mut serialize::Output<DB>) -> serialize::Result {
        match *self {
            WorkStatus::Away => ToSql::<Bool, DB>::to_sql(&false, out),
            WorkStatus::Working => ToSql::<Bool, DB>::to_sql(&true, out),
        }
    }
}

impl<DB> FromSql<Bool, DB> for WorkStatus
where
    DB: backend::Backend,
    bool: FromSql<Bool, DB>,
{
    fn from_sql(bytes: backend::RawValue<'_, DB>) -> deserialize::Result<Self> {
        let value = bool::from_sql(bytes)?;
        Ok(WorkStatus::from_bool(value))
    }
}

impl ToSql<Text, diesel::sqlite::Sqlite> for WorkEvent
where
    String: ToSql<Text, diesel::sqlite::Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::sqlite::Sqlite>) -> serialize::Result {
        let value = serde_lexpr::to_string(self)?;
        out.set_value(value);
        Ok(IsNull::No)
    }
}

impl<DB> FromSql<Text, DB> for WorkEvent
where
    DB: backend::Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: backend::RawValue<'_, DB>) -> deserialize::Result<Self> {
        let value = String::from_sql(bytes)?;
        Ok(serde_lexpr::from_str(&value)?)
    }
}
